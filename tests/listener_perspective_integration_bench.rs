//! Listener-perspective 통합 벤치 (Phase 7 Step 3)
//!
//! `EmbeddedConverter` 엔드투엔드 검증.
//! Prefilter → SignClassifier → MagnitudeClassifier → 변환식을 하나의 API로 호출.
//!
//! ## 판정 기준 (A안)
//!
//! 주 판정: `result.meta.magnitude == expected_magnitude`
//! 병기 지표: `bin(|result.listener_pad.pleasure|)` 도 출력 — 디버깅용
//!
//! ## 회귀 감시 목표
//!
//! - Prefilter 경로: 100% (변경 없음 기대)
//! - Classifier 경로: ~73-77% (sign+magnitude k-NN)
//! - 전체: ~85% 내외
//!
//! 실행: `cargo test --features "embed listener_perspective" --test listener_perspective_integration_bench -- --nocapture`
//!
//! 설계: `docs/emotion/phase7-converter-integration.md`

#![cfg(all(feature = "embed", feature = "listener_perspective"))]

use npc_mind::adapter::file_anchor_source::{AnchorFormat, FileAnchorSource};
use npc_mind::adapter::ort_embedder::OrtEmbedder;
use npc_mind::domain::listener_perspective::{
    EmbeddedConverter, ListenerPerspectiveConverter, Magnitude, MagnitudeBinThresholds,
};
use npc_mind::domain::pad::{Pad, PadAnalyzer};
use npc_mind::domain::pad_anchors::builtin_anchor_toml;
use npc_mind::ports::{TextEmbedder, UtteranceAnalyzer};
use serde::Deserialize;
use std::fs;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

// ============================================================
// 경로 상수
// ============================================================

const MODEL_PATH: &str = "../models/bge-m3/model_quantized.onnx";
const TOKENIZER_PATH: &str = "../models/bge-m3/tokenizer.json";

const BENCH_PATH: &str = "data/listener_perspective/testcases/sign_benchmark.toml";
const RESULTS_DIR: &str = "data/listener_perspective/results";
const PATTERNS_PATH: &str = "data/listener_perspective/prefilter/patterns.toml";
const KEEP_PATH: &str = "data/listener_perspective/prototypes/sign_keep.toml";
const INVERT_PATH: &str = "data/listener_perspective/prototypes/sign_invert.toml";
const MAG_WEAK_PATH: &str = "data/listener_perspective/prototypes/magnitude_weak.toml";
const MAG_NORMAL_PATH: &str = "data/listener_perspective/prototypes/magnitude_normal.toml";
const MAG_STRONG_PATH: &str = "data/listener_perspective/prototypes/magnitude_strong.toml";

// ============================================================
// TOML 스키마 (테스트 케이스)
// ============================================================

#[derive(Debug, Deserialize)]
struct BenchmarkFile {
    meta: BenchmarkMeta,
    #[serde(rename = "case")]
    cases: Vec<TestCase>,
}

#[derive(Debug, Deserialize)]
struct BenchmarkMeta {
    version: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct TestCase {
    id: String,
    utterance: String,
    #[allow(dead_code)]
    label: String,
    expected_sign: String,
    #[allow(dead_code)]
    speaker_p_sign: String,
    #[allow(dead_code)]
    speaker_p_value: f32,
    #[allow(dead_code)]
    listener_p_sign: String,
    listener_p_magnitude: String,
    difficulty: String,
    subtype: String,
    notes: String,
}

// ============================================================
// 케이스 결과
// ============================================================

#[derive(Debug)]
struct CaseResult {
    case: TestCase,
    expected_mag: Magnitude,
    /// Converter 가 반환한 meta.magnitude (A안 주 판정 기준)
    predicted_mag_meta: Magnitude,
    /// bin(|listener_pad.pleasure|) — C안 병기 지표
    predicted_mag_bin: Magnitude,
    speaker_p: f32,
    listener_p: f32,
    path_label: String, // "prefilter:<cat>" | "classifier"
    /// A안 주 판정
    passed_meta: bool,
    /// C안 병기
    passed_bin: bool,
}

// ============================================================
// 로더 / 공유 인스턴스
// ============================================================

fn load_benchmark(path: &str) -> BenchmarkFile {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("벤치 파일 로드 실패 {}: {}", path, e));
    toml::from_str(&content)
        .unwrap_or_else(|e| panic!("벤치 파싱 실패 {}: {}", path, e))
}

/// 공유 PadAnalyzer (화자 PAD 추출)
fn shared_analyzer() -> &'static Mutex<PadAnalyzer> {
    static ANALYZER: OnceLock<Mutex<PadAnalyzer>> = OnceLock::new();
    ANALYZER.get_or_init(|| {
        let embedder = OrtEmbedder::new(MODEL_PATH, TOKENIZER_PATH).unwrap();
        let source = FileAnchorSource::from_content(
            builtin_anchor_toml("ko").unwrap(),
            AnchorFormat::Toml,
        );
        Mutex::new(PadAnalyzer::new(Box::new(embedder), &source).unwrap())
    })
}

/// 공유 Converter (초기화 시 프로토타입 임베딩 내장)
fn shared_converter() -> &'static Mutex<EmbeddedConverter> {
    static CONV: OnceLock<Mutex<EmbeddedConverter>> = OnceLock::new();
    CONV.get_or_init(|| {
        let mut embedder = OrtEmbedder::new(MODEL_PATH, TOKENIZER_PATH).unwrap();
        let conv = EmbeddedConverter::from_paths(
            &mut embedder,
            PATTERNS_PATH,
            KEEP_PATH,
            INVERT_PATH,
            MAG_WEAK_PATH,
            MAG_NORMAL_PATH,
            MAG_STRONG_PATH,
        )
        .expect("EmbeddedConverter 초기화 실패");
        Mutex::new(conv)
    })
}

/// 발화 임베딩 전용 embedder (PadAnalyzer 내부 embedder 와 분리)
fn shared_utt_embedder() -> &'static Mutex<OrtEmbedder> {
    static EMB: OnceLock<Mutex<OrtEmbedder>> = OnceLock::new();
    EMB.get_or_init(|| Mutex::new(OrtEmbedder::new(MODEL_PATH, TOKENIZER_PATH).unwrap()))
}

// ============================================================
// run_id 생성 (converter_YYYY-MM-DD_runNN)
// ============================================================

fn generate_run_id() -> String {
    let today = current_date_string();
    let count = count_files_starting_with(&format!("converter_{}", today));
    format!("converter_{}_run{:02}", today, count + 1)
}

fn current_date_string() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let kst_secs = now.as_secs() as i64 + 9 * 3600;
    let days = kst_secs / 86400;
    let (y, m, d) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    let mut year: i32 = 1970;
    let mut days = days;
    loop {
        let year_days = if is_leap(year) { 366 } else { 365 };
        if days < year_days { break; }
        days -= year_days;
        year += 1;
    }
    let mdays = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month: u32 = 1;
    for &md in mdays.iter() {
        if days < md { break; }
        days -= md;
        month += 1;
    }
    (year, month, (days + 1) as u32)
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn count_files_starting_with(prefix: &str) -> usize {
    let Ok(entries) = fs::read_dir(RESULTS_DIR) else { return 0; };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name().to_str()
                .map(|n| n.starts_with(prefix) && n.ends_with(".md"))
                .unwrap_or(false)
        })
        .count()
}

// ============================================================
// 파싱 헬퍼
// ============================================================

fn parse_magnitude(s: &str) -> Magnitude {
    match s {
        "weak" => Magnitude::Weak,
        "normal" => Magnitude::Normal,
        "strong" => Magnitude::Strong,
        other => panic!("알 수 없는 magnitude: {}", other),
    }
}

fn path_label(path: &npc_mind::domain::listener_perspective::ConvertPath) -> String {
    use npc_mind::domain::listener_perspective::ConvertPath;
    match path {
        ConvertPath::Prefilter { category, .. } => format!("prefilter:{}", category),
        ConvertPath::Classifier { .. } => "classifier".to_string(),
    }
}

// ============================================================
// 콘솔 출력
// ============================================================

fn print_console_table(results: &[CaseResult]) {
    println!("\n{}", "=".repeat(160));
    println!("Listener-perspective 통합 벤치 (Phase 7 Step 3 — EmbeddedConverter 엔드투엔드)");
    println!("{}", "=".repeat(160));
    println!(
        "{:<4} {:<7} {:<11} {:<32} {:>7} {:>7} {:<6} {:<8} {:<8} {}",
        "id", "난이도", "subtype", "path", "P_S", "P_L", "기대", "meta", "bin", "발화"
    );
    println!("{}", "-".repeat(160));

    for r in results {
        let mark_meta = if r.passed_meta { "✓" } else { "✗" };
        let mark_bin = if r.passed_bin { "✓" } else { "✗" };
        println!(
            "{:<4} {:<7} {:<11} {:<32} {:>+7.3} {:>+7.3} {:<6} {:<7}{} {:<7}{} {}",
            r.case.id,
            r.case.difficulty,
            r.case.subtype,
            r.path_label,
            r.speaker_p,
            r.listener_p,
            r.expected_mag.as_str(),
            r.predicted_mag_meta.as_str(),
            mark_meta,
            r.predicted_mag_bin.as_str(),
            mark_bin,
            r.case.utterance,
        );
    }
    println!("{}", "-".repeat(160));
}

// ============================================================
// Markdown 리포트 (A안 주 지표 + C안 병기)
// ============================================================

struct RunMeta {
    run_id: String,
    benchmark_version: String,
    comparison_mode: String, // "meta_magnitude"
    acc_meta: f32,           // A안 정확도
    acc_bin: f32,            // C안 병기 정확도
}

fn generate_report(meta: &RunMeta, results: &[CaseResult]) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("run_id: \"{}\"\n", meta.run_id));
    out.push_str(&format!("benchmark_version: \"{}\"\n", meta.benchmark_version));
    out.push_str(&format!("comparison_mode: \"{}\"\n", meta.comparison_mode));
    out.push_str(&format!("overall_accuracy_meta: {:.2}\n", meta.acc_meta));
    out.push_str(&format!("overall_accuracy_bin: {:.2}\n", meta.acc_bin));
    out.push_str("---\n\n");

    out.push_str(&format!(
        "# Listener-perspective 통합 벤치 — {}\n\n",
        meta.run_id
    ));
    out.push_str(&format!(
        "**주 판정**: `result.meta.magnitude == expected` (A안)\n\n\
         **병기 지표**: `bin(|result.listener_pad.pleasure|)` (C안)\n\n"
    ));

    write_summary_section(&mut out, results);
    write_path_breakdown(&mut out, results);
    write_failure_section(&mut out, results);
    write_confusion_matrix(&mut out, results);
    write_bin_vs_meta_diff(&mut out, results);

    out
}

fn write_summary_section(out: &mut String, results: &[CaseResult]) {
    out.push_str("## 요약\n\n");
    let total = results.len();
    let passed_meta = results.iter().filter(|r| r.passed_meta).count();
    let passed_bin = results.iter().filter(|r| r.passed_bin).count();

    out.push_str("| 지표 | 통과 | 전체 | 정확도 |\n|---|---|---|---|\n");
    out.push_str(&format!(
        "| **meta.magnitude** (주 판정) | {} | {} | **{:.0}%** |\n",
        passed_meta, total, (passed_meta as f32 / total as f32) * 100.0
    ));
    out.push_str(&format!(
        "| bin(\\|P_L\\|) (병기) | {} | {} | {:.0}% |\n",
        passed_bin, total, (passed_bin as f32 / total as f32) * 100.0
    ));
    out.push_str("\n### 난이도별 (meta 기준)\n\n");
    out.push_str("| 난이도 | 통과 | 전체 | 정확도 |\n|---|---|---|---|\n");
    for diff in ["easy", "medium", "hard"] {
        let (p, t) = count_by_difficulty(results, diff);
        if t > 0 {
            out.push_str(&format!(
                "| {} | {} | {} | {:.0}% |\n",
                diff, p, t, (p as f32 / t as f32) * 100.0
            ));
        }
    }
    out.push_str("\n### 기대 magnitude별 (meta 기준)\n\n");
    out.push_str("| magnitude | 통과 | 전체 | 정확도 |\n|---|---|---|---|\n");
    for mag in ["weak", "normal", "strong"] {
        let (p, t) = count_by_expected_mag(results, mag);
        if t > 0 {
            out.push_str(&format!(
                "| {} | {} | {} | {:.0}% |\n",
                mag, p, t, (p as f32 / t as f32) * 100.0
            ));
        }
    }
    out.push_str("\n");
}

fn write_path_breakdown(out: &mut String, results: &[CaseResult]) {
    out.push_str("## 경로별 통계 (meta 기준)\n\n");
    use std::collections::BTreeMap;
    let mut by_path: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for r in results {
        let entry = by_path.entry(r.path_label.as_str()).or_insert((0, 0));
        entry.1 += 1;
        if r.passed_meta {
            entry.0 += 1;
        }
    }
    out.push_str("| 경로 | 통과 | 전체 | 정확도 |\n|---|---|---|---|\n");
    for (path, (p, t)) in by_path {
        out.push_str(&format!(
            "| {} | {} | {} | {:.0}% |\n",
            path, p, t, (p as f32 / t as f32) * 100.0
        ));
    }
    out.push_str("\n");
}

fn write_failure_section(out: &mut String, results: &[CaseResult]) {
    let failures: Vec<&CaseResult> = results.iter().filter(|r| !r.passed_meta).collect();
    out.push_str("## 실패 케이스 상세 (meta 기준)\n\n");
    if failures.is_empty() {
        out.push_str("(모든 케이스 통과)\n\n");
        return;
    }
    out.push_str("| id | 난이도 | path | 발화 | P_S | P_L | 기대 | meta | bin | 노트 |\n");
    out.push_str("|---|---|---|---|---|---|---|---|---|---|\n");
    for r in failures {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {:+.3} | {:+.3} | {} | {} | {} | {} |\n",
            r.case.id, r.case.difficulty, r.path_label,
            r.case.utterance.replace('|', "\\|"),
            r.speaker_p, r.listener_p,
            r.expected_mag.as_str(),
            r.predicted_mag_meta.as_str(),
            r.predicted_mag_bin.as_str(),
            r.case.notes.replace('|', "\\|"),
        ));
    }
    out.push_str("\n");
}

fn write_confusion_matrix(out: &mut String, results: &[CaseResult]) {
    out.push_str("## Confusion Matrix (기대 → meta 예측)\n\n");
    out.push_str("|  | → weak | → normal | → strong |\n|---|---|---|---|\n");
    for exp in ["weak", "normal", "strong"] {
        let row: Vec<usize> = ["weak", "normal", "strong"]
            .iter()
            .map(|pred| {
                results
                    .iter()
                    .filter(|r| {
                        r.expected_mag.as_str() == exp
                            && r.predicted_mag_meta.as_str() == *pred
                    })
                    .count()
            })
            .collect();
        out.push_str(&format!(
            "| **{}** | {} | {} | {} |\n",
            exp, row[0], row[1], row[2]
        ));
    }
    out.push_str("\n");
}

fn write_bin_vs_meta_diff(out: &mut String, results: &[CaseResult]) {
    out.push_str("## meta vs bin 불일치 케이스 (이중 판정 구조 분석)\n\n");
    let mismatches: Vec<&CaseResult> = results
        .iter()
        .filter(|r| r.predicted_mag_meta != r.predicted_mag_bin)
        .collect();

    if mismatches.is_empty() {
        out.push_str("(meta 와 bin 모든 케이스 일치)\n\n");
        return;
    }

    out.push_str(
        "Converter 가 분류기로 결정한 magnitude (`meta`) 와 \
         변환식 결과의 bin 판정 (`bin`) 이 다른 케이스.\n\
         classifier 가 정한 magnitude 의 coef 가 적용된 후 P_L 의 크기가 다른 bin 에 떨어지는 경우.\n\n",
    );
    out.push_str("| id | path | P_S | P_L | meta | bin | 기대 |\n");
    out.push_str("|---|---|---|---|---|---|---|\n");
    for r in mismatches {
        out.push_str(&format!(
            "| {} | {} | {:+.3} | {:+.3} | {} | {} | {} |\n",
            r.case.id,
            r.path_label,
            r.speaker_p,
            r.listener_p,
            r.predicted_mag_meta.as_str(),
            r.predicted_mag_bin.as_str(),
            r.expected_mag.as_str(),
        ));
    }
    out.push_str("\n");
}

// ============================================================
// 카운트 헬퍼
// ============================================================

fn count_by_difficulty(results: &[CaseResult], diff: &str) -> (usize, usize) {
    let total = results.iter().filter(|r| r.case.difficulty == diff).count();
    let passed = results
        .iter()
        .filter(|r| r.case.difficulty == diff && r.passed_meta)
        .count();
    (passed, total)
}

fn count_by_expected_mag(results: &[CaseResult], mag: &str) -> (usize, usize) {
    let total = results
        .iter()
        .filter(|r| r.expected_mag.as_str() == mag)
        .count();
    let passed = results
        .iter()
        .filter(|r| r.expected_mag.as_str() == mag && r.passed_meta)
        .count();
    (passed, total)
}

// Magnitude::as_str / Sign::as_str 는 도메인 inherent method 사용.

// ============================================================
// 메인 테스트
// ============================================================

#[test]
fn listener_perspective_converter_integration() {
    // 1. 벤치 케이스 로드
    let bench = load_benchmark(BENCH_PATH);
    let cases = &bench.cases;
    println!(
        "\n[1] 벤치 로드: {} 케이스, version v{}",
        cases.len(),
        bench.meta.version
    );

    // 2. PadAnalyzer 로 화자 PAD 추출 (모두 한 번에)
    let mut analyzer = shared_analyzer().lock().unwrap();
    let mut speaker_pads: Vec<Pad> = Vec::with_capacity(cases.len());
    for c in cases {
        let pad = analyzer.analyze(&c.utterance).expect("PadAnalyzer 분석 실패");
        speaker_pads.push(pad);
    }
    drop(analyzer);
    println!("[2] 화자 PAD 추출 완료");

    // 3. 발화 임베딩 일괄 계산 (분류기 경로에 주입)
    let mut utt_embedder = shared_utt_embedder().lock().unwrap();
    let texts: Vec<&str> = cases.iter().map(|c| c.utterance.as_str()).collect();
    let utt_embeddings: Vec<Vec<f32>> = utt_embedder
        .embed(&texts)
        .expect("발화 임베딩 실패");
    drop(utt_embedder);
    println!("[3] 발화 임베딩 완료");

    // 4. Converter 호출
    let converter = shared_converter().lock().unwrap();
    let bin_thresholds = MagnitudeBinThresholds::default();
    let mut results: Vec<CaseResult> = Vec::with_capacity(cases.len());
    let mut prefilter_hits = 0;

    for (i, c) in cases.iter().enumerate() {
        let expected_mag = parse_magnitude(&c.listener_p_magnitude);

        let result = converter
            .convert(&c.utterance, &speaker_pads[i], &utt_embeddings[i])
            .expect("Converter.convert 실패");

        let p_label = path_label(&result.meta.path);
        if p_label.starts_with("prefilter:") {
            prefilter_hits += 1;
        }

        let predicted_mag_meta = result.meta.magnitude;
        let predicted_mag_bin = bin_thresholds.bin_of(result.listener_pad.pleasure.abs());

        let passed_meta = predicted_mag_meta == expected_mag;
        let passed_bin = predicted_mag_bin == expected_mag;

        // speaker P: prefilter hit 시 override 값 기록, 아니면 화자 PAD
        let speaker_p_display = match &result.meta.path {
            npc_mind::domain::listener_perspective::ConvertPath::Prefilter { .. } => {
                // prefilter 경로: p_s_default 를 표기 (override 값). 역산:
                // P_L = sign × coef × p_s_default  →  p_s_default = P_L / (sign × coef)
                let coef = result.meta.applied_p_coef;
                if coef.abs() > f32::EPSILON {
                    result.listener_pad.pleasure / coef
                } else {
                    speaker_pads[i].pleasure
                }
            }
            npc_mind::domain::listener_perspective::ConvertPath::Classifier { .. } => {
                speaker_pads[i].pleasure
            }
        };

        results.push(CaseResult {
            case: c.clone(),
            expected_mag,
            predicted_mag_meta,
            predicted_mag_bin,
            speaker_p: speaker_p_display,
            listener_p: result.listener_pad.pleasure,
            path_label: p_label,
            passed_meta,
            passed_bin,
        });
    }
    drop(converter);
    println!(
        "[4] Converter 실행 완료 (prefilter hit {}/{}, classifier {}/{})\n",
        prefilter_hits,
        cases.len(),
        cases.len() - prefilter_hits,
        cases.len()
    );

    // 5. 콘솔 출력
    print_console_table(&results);

    // 6. 요약
    let total = results.len();
    let passed_meta = results.iter().filter(|r| r.passed_meta).count();
    let passed_bin = results.iter().filter(|r| r.passed_bin).count();
    let acc_meta = passed_meta as f32 / total as f32;
    let acc_bin = passed_bin as f32 / total as f32;

    println!(
        "\n전체 정확도 (meta 주 판정): {}/{} ({:.0}%)",
        passed_meta, total, acc_meta * 100.0
    );
    println!(
        "전체 정확도 (bin 병기):      {}/{} ({:.0}%)",
        passed_bin, total, acc_bin * 100.0
    );
    for diff in ["easy", "medium", "hard"] {
        let (p, t) = count_by_difficulty(&results, diff);
        if t > 0 {
            println!(
                "  {}: {}/{} ({:.0}%)",
                diff, p, t, (p as f32 / t as f32) * 100.0
            );
        }
    }
    for mag in ["weak", "normal", "strong"] {
        let (p, t) = count_by_expected_mag(&results, mag);
        if t > 0 {
            println!(
                "  (expect {}): {}/{} ({:.0}%)",
                mag, p, t, (p as f32 / t as f32) * 100.0
            );
        }
    }

    // 7. Markdown 리포트
    let meta = RunMeta {
        run_id: generate_run_id(),
        benchmark_version: bench.meta.version.clone(),
        comparison_mode: "meta_magnitude".to_string(),
        acc_meta,
        acc_bin,
    };
    let report = generate_report(&meta, &results);
    let report_path = format!("{}/{}.md", RESULTS_DIR, meta.run_id);
    if !std::path::Path::new(RESULTS_DIR).exists() {
        fs::create_dir_all(RESULTS_DIR).expect("결과 디렉토리 생성 실패");
    }
    fs::write(&report_path, report).expect("리포트 쓰기 실패");
    println!("\n리포트 저장됨: {}", report_path);

    // 8. 회귀 감시 WARNING (fail 아님)
    if acc_meta < 0.80 {
        eprintln!(
            "WARNING: meta 정확도 {:.0}% < 80% baseline (이전 기록 참조)",
            acc_meta * 100.0
        );
    }
}
