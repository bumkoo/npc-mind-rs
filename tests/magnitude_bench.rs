//! Magnitude 벤치마크 (Phase 2)
//!
//! 부호 축 분류 결과(expected_sign, Ground Truth)를 받아
//! P축 크기(magnitude) 변환식 계수를 검증한다.
//!
//! 변환식: P_L = sign_val × magnitude_coef × P_S
//! Bin: |P_L| < 0.3 → weak, < 0.7 → normal, ≥ 0.7 → strong
//!
//! Phase 2.5 Calibration — 실측 PadAnalyzer 기반:
//!   - PadAnalyzer가 추출한 실측 P_S 사용 (수동 speaker_p_value는 참조만)
//!   - expected_sign 사용 (predicted_sign 아님) — Phase 1 부호 오류 전파 차단
//!   - Coef(0.5/1.0/1.5) × Bin(0.15/0.4) 으로 실측 분포(±0.0~0.4)에 교정
//!   - 별도 리포트 (magnitude_YYYY-MM-DD_runNN.md)
//!
//! Run01의 27%를 baseline으로 calibration 후 58% 내외 목표.
//! 부호 반전/dead zone 케이스(6건)는 Phase 1.5 (앵커 보강) 이관 대상.
//!
//! 실행: `cargo test --features embed --test magnitude_bench -- --nocapture`
//!
//! 설계: docs/emotion/sign-classifier-design.md §3.1

#![cfg(feature = "embed")]

use npc_mind::adapter::file_anchor_source::{AnchorFormat, FileAnchorSource};
use npc_mind::adapter::ort_embedder::OrtEmbedder;
use npc_mind::domain::pad::{Pad, PadAnalyzer};
use npc_mind::domain::pad_anchors::builtin_anchor_toml;
use npc_mind::ports::UtteranceAnalyzer;
use serde::Deserialize;
use std::fs;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

const MODEL_PATH: &str = "../models/bge-m3/model_quantized.onnx";
const TOKENIZER_PATH: &str = "../models/bge-m3/tokenizer.json";

const BENCH_PATH: &str = "data/listener_perspective/testcases/sign_benchmark.toml";
const RESULTS_DIR: &str = "data/listener_perspective/results";

// Phase 2.5 Calibration 계수 (실측 P_S 분포 ±0.0~0.4 기반)
const COEF_P_WEAK: f32 = 0.5;
const COEF_P_NORMAL: f32 = 1.0;
const COEF_P_STRONG: f32 = 1.5;

// Bin 경계 (|P_L| 기준, 실측 분포에 맞춰 낮춤)
const BIN_WEAK_MAX: f32 = 0.15;
const BIN_NORMAL_MAX: f32 = 0.4;

// ============================================================
// TOML 스키마
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
    #[allow(dead_code)]
    subtype: String,
    notes: String,
}

// ============================================================
// 변환 로직
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum Sign { Keep, Invert }

impl Sign {
    fn from_str(s: &str) -> Sign {
        match s {
            "keep" => Sign::Keep,
            "invert" => Sign::Invert,
            _ => panic!("알 수 없는 sign: {}", s),
        }
    }
    fn value(&self) -> f32 {
        match self { Sign::Keep => 1.0, Sign::Invert => -1.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Magnitude { Weak, Normal, Strong }

impl Magnitude {
    fn from_str(s: &str) -> Magnitude {
        match s {
            "weak" => Magnitude::Weak,
            "normal" => Magnitude::Normal,
            "strong" => Magnitude::Strong,
            _ => panic!("알 수 없는 magnitude: {}", s),
        }
    }
    fn as_str(&self) -> &'static str {
        match self {
            Magnitude::Weak => "weak",
            Magnitude::Normal => "normal",
            Magnitude::Strong => "strong",
        }
    }
    fn p_coef(&self) -> f32 {
        match self {
            Magnitude::Weak => COEF_P_WEAK,
            Magnitude::Normal => COEF_P_NORMAL,
            Magnitude::Strong => COEF_P_STRONG,
        }
    }
}

/// P_L = sign × magnitude_coef × P_S
fn transform_pleasure(speaker_p: f32, sign: Sign, magnitude: Magnitude) -> f32 {
    speaker_p * sign.value() * magnitude.p_coef()
}

/// |P_L| → magnitude bin
fn bin_magnitude(p_l: f32) -> Magnitude {
    let abs = p_l.abs();
    if abs < BIN_WEAK_MAX { Magnitude::Weak }
    else if abs < BIN_NORMAL_MAX { Magnitude::Normal }
    else { Magnitude::Strong }
}

#[derive(Debug)]
struct CaseResult {
    case: TestCase,
    speaker_p: f32,
    listener_p: f32,
    predicted_magnitude: Magnitude,
    expected_magnitude: Magnitude,
    passed: bool,
}

// ============================================================
// 로더 / 공유 분석기
// ============================================================

fn load_benchmark(path: &str) -> BenchmarkFile {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("벤치 파일 로드 실패 {}: {}", path, e));
    toml::from_str(&content)
        .unwrap_or_else(|e| panic!("벤치 파싱 실패 {}: {}", path, e))
}

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

// ============================================================
// run_id 생성 (magnitude_YYYY-MM-DD_runNN)
// ============================================================

fn generate_run_id() -> String {
    let today = current_date_string();
    let count = count_files_starting_with(&format!("magnitude_{}", today));
    format!("magnitude_{}_run{:02}", today, count + 1)
}

fn current_date_string() -> String {
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
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
// 콘솔 출력
// ============================================================

fn print_console_table(results: &[CaseResult]) {
    println!("\n{}", "=".repeat(125));
    println!("Magnitude 벤치마크 결과 (Phase 2)");
    println!("{}", "=".repeat(125));
    println!(
        "{:<4} {:<7} {:<11} {:<7} {:>7} {:>7} {:<8} {:<8} {}",
        "id", "난이도", "subtype", "sign", "P_S", "P_L", "기대", "예측", "발화"
    );
    println!("{}", "-".repeat(125));

    for r in results {
        let mark = if r.passed { "✓" } else { "✗" };
        println!(
            "{:<4} {:<7} {:<11} {:<7} {:>+7.3} {:>+7.3} {:<8} {:<7}{} {}",
            r.case.id,
            r.case.difficulty,
            r.case.subtype,
            r.case.expected_sign,
            r.speaker_p,
            r.listener_p,
            r.expected_magnitude.as_str(),
            r.predicted_magnitude.as_str(),
            mark,
            r.case.utterance,
        );
    }
    println!("{}", "-".repeat(125));
}

// ============================================================
// Markdown 리포트
// ============================================================

struct RunMeta {
    run_id: String,
    benchmark_version: String,
    coef_weak: f32,
    coef_normal: f32,
    coef_strong: f32,
    bin_weak_max: f32,
    bin_normal_max: f32,
}

fn generate_report(meta: &RunMeta, results: &[CaseResult]) -> String {
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let acc = passed as f32 / total as f32;

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("run_id: \"{}\"\n", meta.run_id));
    out.push_str(&format!("benchmark_version: \"{}\"\n", meta.benchmark_version));
    out.push_str(&format!("coef_weak: {}\n", meta.coef_weak));
    out.push_str(&format!("coef_normal: {}\n", meta.coef_normal));
    out.push_str(&format!("coef_strong: {}\n", meta.coef_strong));
    out.push_str(&format!("bin_weak_max: {}\n", meta.bin_weak_max));
    out.push_str(&format!("bin_normal_max: {}\n", meta.bin_normal_max));
    out.push_str(&format!("overall_accuracy: {:.2}\n", acc));
    out.push_str("---\n\n");
    out.push_str(&format!("# Magnitude 벤치마크 — {}\n\n", meta.run_id));
    write_summary_section(&mut out, results);
    write_failure_section(&mut out, results);
    write_confusion_matrix(&mut out, results);
    out
}

fn write_summary_section(out: &mut String, results: &[CaseResult]) {
    out.push_str("## 요약\n\n");
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    out.push_str("| 분류 | 통과 | 전체 | 정확도 |\n|---|---|---|---|\n");
    out.push_str(&format!(
        "| 전체 | {} | {} | {:.0}% |\n",
        passed, total, (passed as f32 / total as f32) * 100.0
    ));
    for diff in ["easy", "medium", "hard"] {
        let (p, t) = count_by_difficulty(results, diff);
        if t > 0 {
            out.push_str(&format!(
                "| {} | {} | {} | {:.0}% |\n",
                diff, p, t, (p as f32 / t as f32) * 100.0
            ));
        }
    }
    out.push_str("\n### 기대 magnitude별\n\n");
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

fn write_failure_section(out: &mut String, results: &[CaseResult]) {
    let failures: Vec<&CaseResult> = results.iter().filter(|r| !r.passed).collect();
    out.push_str("## 실패 케이스 상세\n\n");
    if failures.is_empty() {
        out.push_str("(모든 케이스 통과)\n\n");
        return;
    }
    out.push_str("| id | 난이도 | 발화 | P_S | P_L | 기대 | 예측 | 노트 |\n");
    out.push_str("|---|---|---|---|---|---|---|---|\n");
    for r in failures {
        out.push_str(&format!(
            "| {} | {} | {} | {:+.3} | {:+.3} | {} | {} | {} |\n",
            r.case.id, r.case.difficulty,
            r.case.utterance.replace('|', "\\|"),
            r.speaker_p, r.listener_p,
            r.expected_magnitude.as_str(),
            r.predicted_magnitude.as_str(),
            r.case.notes.replace('|', "\\|"),
        ));
    }
    out.push_str("\n");
}

fn write_confusion_matrix(out: &mut String, results: &[CaseResult]) {
    out.push_str("## Confusion Matrix (기대 → 예측)\n\n");
    out.push_str("|  | → weak | → normal | → strong |\n|---|---|---|---|\n");
    for exp in ["weak", "normal", "strong"] {
        let row: Vec<usize> = ["weak", "normal", "strong"]
            .iter()
            .map(|pred| {
                results
                    .iter()
                    .filter(|r| {
                        r.expected_magnitude.as_str() == exp
                            && r.predicted_magnitude.as_str() == *pred
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

fn count_by_difficulty(results: &[CaseResult], diff: &str) -> (usize, usize) {
    let total = results.iter().filter(|r| r.case.difficulty == diff).count();
    let passed = results.iter().filter(|r| r.case.difficulty == diff && r.passed).count();
    (passed, total)
}

fn count_by_expected_mag(results: &[CaseResult], mag: &str) -> (usize, usize) {
    let total = results.iter().filter(|r| r.expected_magnitude.as_str() == mag).count();
    let passed = results.iter().filter(|r| r.expected_magnitude.as_str() == mag && r.passed).count();
    (passed, total)
}

// ============================================================
// 메인 테스트
// ============================================================

#[test]
fn magnitude_변환식_벤치마크() {
    // 1. 로드
    let bench = load_benchmark(BENCH_PATH);
    let cases = &bench.cases;
    println!("\n[1] 로드 완료: {} 케이스, bench v{}", cases.len(), bench.meta.version);

    // 2. PadAnalyzer로 화자 P 추출 (26개 한 번만)
    let mut analyzer = shared_analyzer().lock().unwrap();
    let mut speaker_pads: Vec<Pad> = Vec::with_capacity(cases.len());
    for c in cases {
        let pad = analyzer.analyze(&c.utterance).expect("PadAnalyzer 분석 실패");
        speaker_pads.push(pad);
    }
    drop(analyzer);
    println!("[2] 화자 PAD 추출 완료");

    // 3. 변환 + bin + 채점
    let mut results: Vec<CaseResult> = Vec::with_capacity(cases.len());
    for (i, c) in cases.iter().enumerate() {
        let sign = Sign::from_str(&c.expected_sign);
        let expected_mag = Magnitude::from_str(&c.listener_p_magnitude);
        let p_s = speaker_pads[i].pleasure;
        let p_l = transform_pleasure(p_s, sign, expected_mag);
        let predicted_mag = bin_magnitude(p_l);
        let passed = predicted_mag == expected_mag;
        results.push(CaseResult {
            case: c.clone(),
            speaker_p: p_s,
            listener_p: p_l,
            predicted_magnitude: predicted_mag,
            expected_magnitude: expected_mag,
            passed,
        });
    }
    println!("[3] 변환·bin·채점 완료 (Calibration: coef 0.5/1.0/1.5, bin 0.15/0.4)\n");

    // 4. 콘솔 출력
    print_console_table(&results);

    // 5. 요약
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    println!(
        "\n전체 정확도: {}/{} ({:.0}%)",
        passed, total, (passed as f32 / total as f32) * 100.0
    );
    for diff in ["easy", "medium", "hard"] {
        let (p, t) = count_by_difficulty(&results, diff);
        if t > 0 {
            println!("  {}: {}/{} ({:.0}%)", diff, p, t, (p as f32 / t as f32) * 100.0);
        }
    }
    for mag in ["weak", "normal", "strong"] {
        let (p, t) = count_by_expected_mag(&results, mag);
        if t > 0 {
            println!("  (expect {}): {}/{} ({:.0}%)", mag, p, t, (p as f32 / t as f32) * 100.0);
        }
    }

    // 6. Markdown 리포트
    let meta = RunMeta {
        run_id: generate_run_id(),
        benchmark_version: bench.meta.version.clone(),
        coef_weak: COEF_P_WEAK,
        coef_normal: COEF_P_NORMAL,
        coef_strong: COEF_P_STRONG,
        bin_weak_max: BIN_WEAK_MAX,
        bin_normal_max: BIN_NORMAL_MAX,
    };
    let report = generate_report(&meta, &results);
    let report_path = format!("{}/{}.md", RESULTS_DIR, meta.run_id);
    if !std::path::Path::new(RESULTS_DIR).exists() {
        fs::create_dir_all(RESULTS_DIR).expect("결과 디렉토리 생성 실패");
    }
    fs::write(&report_path, report).expect("리포트 쓰기 실패");
    println!("\n리포트 저장됨: {}", report_path);
}
