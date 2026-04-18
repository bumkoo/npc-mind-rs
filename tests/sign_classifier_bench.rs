//! 부호 축 분류기 벤치마크
//!
//! Listener-perspective 변환의 부호 축(keep/invert)을 k-NN top-k로 분류.
//! 프로토타입 2개 (sign_keep.toml, sign_invert.toml) 로드,
//! 테스트 케이스 26개(sign_benchmark.toml)에 대해 분류 수행.
//!
//! 결과: 콘솔 출력 + Markdown 리포트 자동 생성.
//!
//! 실행: `cargo test --features embed --test sign_classifier_bench -- --nocapture`
//!
//! 설계: docs/emotion/sign-classifier-design.md

#![cfg(feature = "embed")]

use npc_mind::adapter::ort_embedder::OrtEmbedder;
use npc_mind::ports::TextEmbedder;
use serde::Deserialize;
use std::fs;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

const MODEL_PATH: &str = "../models/bge-m3/model_quantized.onnx";
const TOKENIZER_PATH: &str = "../models/bge-m3/tokenizer.json";

const KEEP_PATH: &str = "data/listener_perspective/prototypes/sign_keep.toml";
const INVERT_PATH: &str = "data/listener_perspective/prototypes/sign_invert.toml";
const BENCH_PATH: &str = "data/listener_perspective/testcases/sign_benchmark.toml";
const RESULTS_DIR: &str = "data/listener_perspective/results";

const K: usize = 3;

// ============================================================
// TOML 스키마
// ============================================================

#[derive(Debug, Deserialize)]
struct PrototypeFile {
    meta: PrototypeMeta,
    prototypes: PrototypeSection,
}

#[derive(Debug, Deserialize)]
struct PrototypeMeta {
    version: String,
    group: String,
}

#[derive(Debug, Deserialize)]
struct PrototypeSection {
    items: Vec<Prototype>,
}

#[derive(Debug, Deserialize, Clone)]
struct Prototype {
    text: String,
    subtype: String,
    #[allow(dead_code)]
    source: Option<String>,
}

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
    listener_p_sign: String,
    difficulty: String,
    #[allow(dead_code)]
    subtype: String,
    notes: String,
}

// ============================================================
// 분류 결과
// ============================================================

#[derive(Debug, Clone, Copy)]
enum Sign {
    Keep,
    Invert,
}

impl Sign {
    fn as_str(&self) -> &'static str {
        match self {
            Sign::Keep => "keep",
            Sign::Invert => "invert",
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct ClassifyResult {
    predicted_sign: Sign,
    keep_score: f32,
    invert_score: f32,
    margin: f32,
    top_matches: Vec<PrototypeMatch>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PrototypeMatch {
    text: String,
    subtype: String,
    group: Sign,
    similarity: f32,
}

#[derive(Debug)]
struct CaseResult {
    case: TestCase,
    result: ClassifyResult,
    passed: bool,
}

// ============================================================
// 로더
// ============================================================

fn load_prototypes(path: &str) -> PrototypeFile {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("프로토타입 파일 로드 실패 {}: {}", path, e));
    toml::from_str(&content)
        .unwrap_or_else(|e| panic!("프로토타입 파싱 실패 {}: {}", path, e))
}

fn load_benchmark(path: &str) -> BenchmarkFile {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("벤치 파일 로드 실패 {}: {}", path, e));
    toml::from_str(&content)
        .unwrap_or_else(|e| panic!("벤치 파싱 실패 {}: {}", path, e))
}

// ============================================================
// 분류기 (k-NN top-k)
// ============================================================

/// 코사인 유사도
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

/// 분류 실행: 발화 임베딩 vs keep/invert 프로토타입 임베딩
fn classify(
    utt_emb: &[f32],
    keep_protos: &[Prototype],
    keep_embeds: &[Vec<f32>],
    invert_protos: &[Prototype],
    invert_embeds: &[Vec<f32>],
    k: usize,
) -> ClassifyResult {
    let mut all_matches: Vec<PrototypeMatch> = Vec::new();

    for (proto, emb) in keep_protos.iter().zip(keep_embeds.iter()) {
        all_matches.push(PrototypeMatch {
            text: proto.text.clone(),
            subtype: proto.subtype.clone(),
            group: Sign::Keep,
            similarity: cosine_sim(utt_emb, emb),
        });
    }
    for (proto, emb) in invert_protos.iter().zip(invert_embeds.iter()) {
        all_matches.push(PrototypeMatch {
            text: proto.text.clone(),
            subtype: proto.subtype.clone(),
            group: Sign::Invert,
            similarity: cosine_sim(utt_emb, emb),
        });
    }

    // 내림차순 정렬
    all_matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());

    let keep_score = top_k_mean(&all_matches, Sign::Keep, k);
    let invert_score = top_k_mean(&all_matches, Sign::Invert, k);

    let predicted_sign = if keep_score >= invert_score {
        Sign::Keep
    } else {
        Sign::Invert
    };
    let margin = (keep_score - invert_score).abs();

    let top_matches: Vec<PrototypeMatch> = all_matches.iter().take(5).cloned().collect();

    ClassifyResult {
        predicted_sign,
        keep_score,
        invert_score,
        margin,
        top_matches,
    }
}

fn top_k_mean(matches: &[PrototypeMatch], target: Sign, k: usize) -> f32 {
    let target_str = target.as_str();
    let filtered: Vec<f32> = matches
        .iter()
        .filter(|m| m.group.as_str() == target_str)
        .take(k)
        .map(|m| m.similarity)
        .collect();
    if filtered.is_empty() {
        return 0.0;
    }
    filtered.iter().sum::<f32>() / filtered.len() as f32
}

// ============================================================
// 임베더 공유 캐시
// ============================================================

fn shared_embedder() -> &'static Mutex<OrtEmbedder> {
    static EMB: OnceLock<Mutex<OrtEmbedder>> = OnceLock::new();
    EMB.get_or_init(|| Mutex::new(OrtEmbedder::new(MODEL_PATH, TOKENIZER_PATH).unwrap()))
}

// ============================================================
// run_id 생성 (YYYY-MM-DD_runNN)
// ============================================================

fn generate_run_id() -> String {
    let today = current_date_string();
    let count = count_files_starting_with(&today);
    format!("{}_run{:02}", today, count + 1)
}

fn current_date_string() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let secs = now.as_secs() as i64;
    // KST +9h
    let kst_secs = secs + 9 * 3600;
    let days = kst_secs / 86400;
    let (y, m, d) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    let mut year: i32 = 1970;
    let mut days = days;
    loop {
        let year_days = if is_leap(year) { 366 } else { 365 };
        if days < year_days {
            break;
        }
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
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, (days + 1) as u32)
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn count_files_starting_with(prefix: &str) -> usize {
    let Ok(entries) = fs::read_dir(RESULTS_DIR) else {
        return 0;
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|name| name.starts_with(prefix) && name.ends_with(".md"))
                .unwrap_or(false)
        })
        .count()
}

// ============================================================
// 콘솔 출력
// ============================================================

fn print_console_table(results: &[CaseResult]) {
    println!("\n{}", "=".repeat(110));
    println!("부호 축 분류기 벤치마크 결과");
    println!("{}", "=".repeat(110));
    println!(
        "{:<4} {:<7} {:<11} {:<7} {:<7} {:<8} {}",
        "id", "난이도", "subtype", "기대", "예측", "margin", "발화"
    );
    println!("{}", "-".repeat(110));

    for r in results {
        let mark = if r.passed { "✓" } else { "✗" };
        println!(
            "{:<4} {:<7} {:<11} {:<7} {:<7} {:<7.3}{} {}",
            r.case.id,
            r.case.difficulty,
            r.case.subtype,
            r.case.expected_sign,
            r.result.predicted_sign.as_str(),
            r.result.margin,
            mark,
            r.case.utterance,
        );
    }
    println!("{}", "-".repeat(110));
}

// ============================================================
// Markdown 리포트
// ============================================================

struct RunMeta {
    run_id: String,
    prototype_keep_version: String,
    prototype_invert_version: String,
    benchmark_version: String,
    classifier: String,
}

fn generate_report(meta: &RunMeta, results: &[CaseResult]) -> String {
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let acc = passed as f32 / total as f32;

    let mut out = String::new();

    out.push_str("---\n");
    out.push_str(&format!("run_id: \"{}\"\n", meta.run_id));
    out.push_str(&format!("prototype_keep_version: \"{}\"\n", meta.prototype_keep_version));
    out.push_str(&format!("prototype_invert_version: \"{}\"\n", meta.prototype_invert_version));
    out.push_str(&format!("benchmark_version: \"{}\"\n", meta.benchmark_version));
    out.push_str(&format!("classifier: \"{}\"\n", meta.classifier));
    out.push_str(&format!("overall_accuracy: {:.2}\n", acc));
    out.push_str("---\n\n");

    out.push_str(&format!("# 부호 축 분류기 벤치마크 결과 — {}\n\n", meta.run_id));

    write_summary_section(&mut out, results);
    write_failure_section(&mut out, results);
    write_margin_distribution(&mut out, results);

    out
}

fn write_summary_section(out: &mut String, results: &[CaseResult]) {
    out.push_str("## 요약\n\n");

    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    out.push_str("| 분류 | 통과 | 전체 | 정확도 |\n");
    out.push_str("|---|---|---|---|\n");
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
    out.push_str("\n");

    out.push_str("### 부호별\n\n");
    out.push_str("| 부호 | 통과 | 전체 | 정확도 |\n");
    out.push_str("|---|---|---|---|\n");
    for sign in ["keep", "invert"] {
        let (p, t) = count_by_sign(results, sign);
        if t > 0 {
            out.push_str(&format!(
                "| {} | {} | {} | {:.0}% |\n",
                sign, p, t, (p as f32 / t as f32) * 100.0
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

    out.push_str("| id | 난이도 | 발화 | 기대 | 예측 | 점수차 | 노트 |\n");
    out.push_str("|---|---|---|---|---|---|---|\n");
    for r in failures {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.3} | {} |\n",
            r.case.id,
            r.case.difficulty,
            r.case.utterance.replace('|', "\\|"),
            r.case.expected_sign,
            r.result.predicted_sign.as_str(),
            r.result.margin,
            r.case.notes.replace('|', "\\|"),
        ));
    }
    out.push_str("\n");
}

fn write_margin_distribution(out: &mut String, results: &[CaseResult]) {
    out.push_str("## 점수차 분포\n\n");
    out.push_str("| 구간 | 건수 | 통과 | 통과율 |\n");
    out.push_str("|---|---|---|---|\n");

    let buckets = [
        ("0.20 이상 (확신)", 0.20_f32, f32::MAX),
        ("0.10 ~ 0.20 (보통)", 0.10, 0.20),
        ("0.05 ~ 0.10 (약함)", 0.05, 0.10),
        ("0.05 미만 (불확실)", 0.0, 0.05),
    ];
    for (label, lo, hi) in buckets {
        let in_bucket: Vec<&CaseResult> = results
            .iter()
            .filter(|r| r.result.margin >= lo && r.result.margin < hi)
            .collect();
        let total = in_bucket.len();
        let passed = in_bucket.iter().filter(|r| r.passed).count();
        let rate = if total > 0 {
            format!("{:.0}%", (passed as f32 / total as f32) * 100.0)
        } else {
            "-".to_string()
        };
        out.push_str(&format!("| {} | {} | {} | {} |\n", label, total, passed, rate));
    }
    out.push_str("\n");
}

fn count_by_difficulty(results: &[CaseResult], diff: &str) -> (usize, usize) {
    let total = results.iter().filter(|r| r.case.difficulty == diff).count();
    let passed = results
        .iter()
        .filter(|r| r.case.difficulty == diff && r.passed)
        .count();
    (passed, total)
}

fn count_by_sign(results: &[CaseResult], sign: &str) -> (usize, usize) {
    let total = results.iter().filter(|r| r.case.expected_sign == sign).count();
    let passed = results
        .iter()
        .filter(|r| r.case.expected_sign == sign && r.passed)
        .count();
    (passed, total)
}

// ============================================================
// 메인 테스트
// ============================================================

#[test]
fn 부호축_분류기_벤치마크() {
    // 1. 데이터 로드
    let keep_file = load_prototypes(KEEP_PATH);
    let invert_file = load_prototypes(INVERT_PATH);
    let bench_file = load_benchmark(BENCH_PATH);

    assert_eq!(keep_file.meta.group, "sign_keep");
    assert_eq!(invert_file.meta.group, "sign_invert");

    let keep_protos = &keep_file.prototypes.items;
    let invert_protos = &invert_file.prototypes.items;
    let cases = &bench_file.cases;

    println!(
        "\n[1] 로드 완료: keep {}, invert {}, 테스트 {}",
        keep_protos.len(),
        invert_protos.len(),
        cases.len()
    );

    // 2. 프로토타입 임베딩 (사전 계산)
    let mut embedder = shared_embedder().lock().unwrap();

    let keep_texts: Vec<&str> = keep_protos.iter().map(|p| p.text.as_str()).collect();
    let keep_embeds = embedder.embed(&keep_texts).expect("keep 프로토타입 임베딩 실패");

    let invert_texts: Vec<&str> = invert_protos.iter().map(|p| p.text.as_str()).collect();
    let invert_embeds = embedder.embed(&invert_texts).expect("invert 프로토타입 임베딩 실패");

    println!("[2] 프로토타입 임베딩 완료");

    // 3. 테스트 케이스 임베딩 (배치)
    let case_texts: Vec<&str> = cases.iter().map(|c| c.utterance.as_str()).collect();
    let case_embeds = embedder.embed(&case_texts).expect("테스트 케이스 임베딩 실패");
    drop(embedder);

    println!("[3] 테스트 케이스 임베딩 완료\n");

    // 4. 각 케이스 분류
    let mut results: Vec<CaseResult> = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        let result = classify(
            &case_embeds[i],
            keep_protos,
            &keep_embeds,
            invert_protos,
            &invert_embeds,
            K,
        );
        let passed = result.predicted_sign.as_str() == case.expected_sign;
        results.push(CaseResult {
            case: case.clone(),
            result,
            passed,
        });
    }

    // 5. 콘솔 출력
    print_console_table(&results);

    // 6. 요약 통계 (콘솔)
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

    // 7. Markdown 리포트 생성
    let run_meta = RunMeta {
        run_id: generate_run_id(),
        prototype_keep_version: keep_file.meta.version.clone(),
        prototype_invert_version: invert_file.meta.version.clone(),
        benchmark_version: bench_file.meta.version.clone(),
        classifier: format!("knn-top{}", K),
    };
    let report = generate_report(&run_meta, &results);

    let report_path = format!("{}/{}.md", RESULTS_DIR, run_meta.run_id);

    // 디렉토리 없으면 생성 (최초 실행 대비)
    if !std::path::Path::new(RESULTS_DIR).exists() {
        fs::create_dir_all(RESULTS_DIR).expect("결과 디렉토리 생성 실패");
    }
    fs::write(&report_path, report).expect("리포트 쓰기 실패");

    println!("\n리포트 저장됨: {}", report_path);

    // 8. 회귀 방지 WARNING (fail 아님)
    // Phase 1 목표: easy 95%
    let easy = count_by_difficulty(&results, "easy");
    if easy.1 > 0 {
        let easy_rate = easy.0 as f32 / easy.1 as f32;
        if easy_rate < 0.95 {
            eprintln!("WARNING: easy 정확도 {:.0}% < 95% 목표", easy_rate * 100.0);
        }
    }
}
