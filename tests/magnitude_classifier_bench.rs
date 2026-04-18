//! 강도 축 분류기 벤치마크 (Phase 4)
//!
//! Listener-perspective 변환의 강도 축(weak/normal/strong)을 k-NN top-k로 분류.
//! 프로토타입 3개 (magnitude_weak.toml, magnitude_normal.toml, magnitude_strong.toml) 로드,
//! 테스트 케이스 26개(sign_benchmark.toml)에 대해 분류 수행.
//!
//! Phase 1 sign_classifier_bench.rs 와 동일 구조, 2-way → 3-way 확장.
//!
//! 실행: `cargo test --features embed --test magnitude_classifier_bench -- --nocapture`
//!
//! 설계: docs/emotion/sign-classifier-design.md §3.3, §3.7.4

#![cfg(feature = "embed")]

use npc_mind::adapter::ort_embedder::OrtEmbedder;
use npc_mind::ports::TextEmbedder;
use serde::Deserialize;
use std::fs;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

const MODEL_PATH: &str = "../models/bge-m3/model_quantized.onnx";
const TOKENIZER_PATH: &str = "../models/bge-m3/tokenizer.json";

const WEAK_PATH: &str = "data/listener_perspective/prototypes/magnitude_weak.toml";
const NORMAL_PATH: &str = "data/listener_perspective/prototypes/magnitude_normal.toml";
const STRONG_PATH: &str = "data/listener_perspective/prototypes/magnitude_strong.toml";
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
    #[allow(dead_code)]
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
// Magnitude
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum Magnitude {
    Weak,
    Normal,
    Strong,
}

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
}

// ============================================================
// 분류 결과
// ============================================================

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PrototypeMatch {
    text: String,
    subtype: String,
    group: Magnitude,
    similarity: f32,
}

#[derive(Debug)]
#[allow(dead_code)]
struct ClassifyResult {
    predicted: Magnitude,
    weak_score: f32,
    normal_score: f32,
    strong_score: f32,
    margin: f32,          // top_score - 2nd_score
    top_matches: Vec<PrototypeMatch>,
}

#[derive(Debug)]
struct CaseResult {
    case: TestCase,
    result: ClassifyResult,
    expected: Magnitude,
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
// 분류기 (k-NN top-k, 3-way)
// ============================================================

fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

struct ProtoSet<'a> {
    group: Magnitude,
    protos: &'a [Prototype],
    embeds: &'a [Vec<f32>],
}

fn classify(utt_emb: &[f32], sets: &[ProtoSet], k: usize) -> ClassifyResult {
    let mut all_matches: Vec<PrototypeMatch> = Vec::new();

    for set in sets {
        for (proto, emb) in set.protos.iter().zip(set.embeds.iter()) {
            all_matches.push(PrototypeMatch {
                text: proto.text.clone(),
                subtype: proto.subtype.clone(),
                group: set.group,
                similarity: cosine_sim(utt_emb, emb),
            });
        }
    }

    // 내림차순 정렬
    all_matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());

    let weak_score = top_k_mean(&all_matches, Magnitude::Weak, k);
    let normal_score = top_k_mean(&all_matches, Magnitude::Normal, k);
    let strong_score = top_k_mean(&all_matches, Magnitude::Strong, k);

    // 최대 점수 카테고리 선택
    let (predicted, top_score, second_score) = pick_top(weak_score, normal_score, strong_score);
    let margin = top_score - second_score;

    let top_matches: Vec<PrototypeMatch> = all_matches.iter().take(5).cloned().collect();

    ClassifyResult {
        predicted,
        weak_score,
        normal_score,
        strong_score,
        margin,
        top_matches,
    }
}

fn top_k_mean(matches: &[PrototypeMatch], target: Magnitude, k: usize) -> f32 {
    let filtered: Vec<f32> = matches
        .iter()
        .filter(|m| m.group == target)
        .take(k)
        .map(|m| m.similarity)
        .collect();
    if filtered.is_empty() {
        return 0.0;
    }
    filtered.iter().sum::<f32>() / filtered.len() as f32
}

/// 3개 점수 중 최대값과 두 번째 값 반환 (margin 계산용)
fn pick_top(w: f32, n: f32, s: f32) -> (Magnitude, f32, f32) {
    let mut arr = [
        (Magnitude::Weak, w),
        (Magnitude::Normal, n),
        (Magnitude::Strong, s),
    ];
    arr.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    (arr[0].0, arr[0].1, arr[1].1)
}

// ============================================================
// 임베더 공유 캐시
// ============================================================

fn shared_embedder() -> &'static Mutex<OrtEmbedder> {
    static EMB: OnceLock<Mutex<OrtEmbedder>> = OnceLock::new();
    EMB.get_or_init(|| Mutex::new(OrtEmbedder::new(MODEL_PATH, TOKENIZER_PATH).unwrap()))
}

// ============================================================
// run_id 생성 (magnitude_knn_YYYY-MM-DD_runNN)
// ============================================================

fn generate_run_id() -> String {
    let today = current_date_string();
    let count = count_files_starting_with(&format!("magnitude_knn_{}", today));
    format!("magnitude_knn_{}_run{:02}", today, count + 1)
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
    println!("\n{}", "=".repeat(135));
    println!("강도 축 분류기 벤치마크 결과 (Phase 4, k-NN top-{})", K);
    println!("{}", "=".repeat(135));
    println!(
        "{:<4} {:<7} {:<11} {:<8} {:<8} {:>6} {:>6} {:>6} {:<8} {}",
        "id", "난이도", "subtype", "기대", "예측", "W", "N", "S", "margin", "발화"
    );
    println!("{}", "-".repeat(135));

    for r in results {
        let mark = if r.passed { "✓" } else { "✗" };
        println!(
            "{:<4} {:<7} {:<11} {:<8} {:<7}{} {:>+6.3} {:>+6.3} {:>+6.3} {:<8.3} {}",
            r.case.id,
            r.case.difficulty,
            r.case.subtype,
            r.expected.as_str(),
            r.result.predicted.as_str(),
            mark,
            r.result.weak_score,
            r.result.normal_score,
            r.result.strong_score,
            r.result.margin,
            r.case.utterance,
        );
    }
    println!("{}", "-".repeat(135));
}

// ============================================================
// Markdown 리포트
// ============================================================

struct RunMeta {
    run_id: String,
    proto_weak_version: String,
    proto_normal_version: String,
    proto_strong_version: String,
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
    out.push_str(&format!("proto_weak_version: \"{}\"\n", meta.proto_weak_version));
    out.push_str(&format!("proto_normal_version: \"{}\"\n", meta.proto_normal_version));
    out.push_str(&format!("proto_strong_version: \"{}\"\n", meta.proto_strong_version));
    out.push_str(&format!("benchmark_version: \"{}\"\n", meta.benchmark_version));
    out.push_str(&format!("classifier: \"{}\"\n", meta.classifier));
    out.push_str(&format!("overall_accuracy: {:.2}\n", acc));
    out.push_str("---\n\n");
    out.push_str(&format!("# 강도 축 분류기 벤치마크 결과 — {}\n\n", meta.run_id));

    write_summary_section(&mut out, results);
    write_confusion_matrix(&mut out, results);
    write_failure_section(&mut out, results);
    write_margin_distribution(&mut out, results);
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
        let (p, t) = count_by_expected(results, mag);
        if t > 0 {
            out.push_str(&format!(
                "| {} | {} | {} | {:.0}% |\n",
                mag, p, t, (p as f32 / t as f32) * 100.0
            ));
        }
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
                    .filter(|r| r.expected.as_str() == exp && r.result.predicted.as_str() == *pred)
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

fn write_failure_section(out: &mut String, results: &[CaseResult]) {
    let failures: Vec<&CaseResult> = results.iter().filter(|r| !r.passed).collect();
    out.push_str("## 실패 케이스 상세\n\n");
    if failures.is_empty() {
        out.push_str("(모든 케이스 통과)\n\n");
        return;
    }
    out.push_str("| id | 난이도 | 발화 | 기대 | 예측 | W | N | S | margin | 노트 |\n");
    out.push_str("|---|---|---|---|---|---|---|---|---|---|\n");
    for r in failures {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.3} | {:.3} | {:.3} | {:.3} | {} |\n",
            r.case.id, r.case.difficulty,
            r.case.utterance.replace('|', "\\|"),
            r.expected.as_str(),
            r.result.predicted.as_str(),
            r.result.weak_score, r.result.normal_score, r.result.strong_score,
            r.result.margin,
            r.case.notes.replace('|', "\\|"),
        ));
    }
    out.push_str("\n");
}

fn write_margin_distribution(out: &mut String, results: &[CaseResult]) {
    out.push_str("## 점수차 분포 (top1 − top2)\n\n");
    out.push_str("| 구간 | 건수 | 통과 | 통과율 |\n|---|---|---|---|\n");
    let buckets = [
        ("0.10 이상 (확신)", 0.10_f32, f32::MAX),
        ("0.05 ~ 0.10 (보통)", 0.05, 0.10),
        ("0.02 ~ 0.05 (약함)", 0.02, 0.05),
        ("0.02 미만 (불확실)", 0.0, 0.02),
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
    let passed = results.iter().filter(|r| r.case.difficulty == diff && r.passed).count();
    (passed, total)
}

fn count_by_expected(results: &[CaseResult], mag: &str) -> (usize, usize) {
    let total = results.iter().filter(|r| r.expected.as_str() == mag).count();
    let passed = results.iter().filter(|r| r.expected.as_str() == mag && r.passed).count();
    (passed, total)
}

// ============================================================
// 메인 테스트
// ============================================================

#[test]
fn 강도축_분류기_벤치마크() {
    // 1. 데이터 로드
    let weak_file = load_prototypes(WEAK_PATH);
    let normal_file = load_prototypes(NORMAL_PATH);
    let strong_file = load_prototypes(STRONG_PATH);
    let bench_file = load_benchmark(BENCH_PATH);

    assert_eq!(weak_file.meta.group, "magnitude_weak");
    assert_eq!(normal_file.meta.group, "magnitude_normal");
    assert_eq!(strong_file.meta.group, "magnitude_strong");

    let weak_protos = &weak_file.prototypes.items;
    let normal_protos = &normal_file.prototypes.items;
    let strong_protos = &strong_file.prototypes.items;
    let cases = &bench_file.cases;

    println!(
        "\n[1] 로드 완료: weak {}, normal {}, strong {}, 테스트 {}",
        weak_protos.len(),
        normal_protos.len(),
        strong_protos.len(),
        cases.len()
    );

    // 2. 프로토타입 임베딩 (사전 계산)
    let mut embedder = shared_embedder().lock().unwrap();

    let weak_texts: Vec<&str> = weak_protos.iter().map(|p| p.text.as_str()).collect();
    let weak_embeds = embedder.embed(&weak_texts).expect("weak 임베딩 실패");

    let normal_texts: Vec<&str> = normal_protos.iter().map(|p| p.text.as_str()).collect();
    let normal_embeds = embedder.embed(&normal_texts).expect("normal 임베딩 실패");

    let strong_texts: Vec<&str> = strong_protos.iter().map(|p| p.text.as_str()).collect();
    let strong_embeds = embedder.embed(&strong_texts).expect("strong 임베딩 실패");

    println!("[2] 프로토타입 임베딩 완료");

    // 3. 테스트 케이스 임베딩
    let case_texts: Vec<&str> = cases.iter().map(|c| c.utterance.as_str()).collect();
    let case_embeds = embedder.embed(&case_texts).expect("테스트 케이스 임베딩 실패");
    drop(embedder);

    println!("[3] 테스트 케이스 임베딩 완료\n");

    // 4. 각 케이스 분류
    let sets = vec![
        ProtoSet { group: Magnitude::Weak, protos: weak_protos, embeds: &weak_embeds },
        ProtoSet { group: Magnitude::Normal, protos: normal_protos, embeds: &normal_embeds },
        ProtoSet { group: Magnitude::Strong, protos: strong_protos, embeds: &strong_embeds },
    ];

    let mut results: Vec<CaseResult> = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        let result = classify(&case_embeds[i], &sets, K);
        let expected = Magnitude::from_str(&case.listener_p_magnitude);
        let passed = result.predicted == expected;
        results.push(CaseResult {
            case: case.clone(),
            result,
            expected,
            passed,
        });
    }

    // 5. 콘솔 출력
    print_console_table(&results);

    // 6. 요약 통계
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
        let (p, t) = count_by_expected(&results, mag);
        if t > 0 {
            println!("  (expect {}): {}/{} ({:.0}%)", mag, p, t, (p as f32 / t as f32) * 100.0);
        }
    }

    // 7. Markdown 리포트 생성
    let run_meta = RunMeta {
        run_id: generate_run_id(),
        proto_weak_version: weak_file.meta.version.clone(),
        proto_normal_version: normal_file.meta.version.clone(),
        proto_strong_version: strong_file.meta.version.clone(),
        benchmark_version: bench_file.meta.version.clone(),
        classifier: format!("knn-top{}", K),
    };
    let report = generate_report(&run_meta, &results);
    let report_path = format!("{}/{}.md", RESULTS_DIR, run_meta.run_id);

    if !std::path::Path::new(RESULTS_DIR).exists() {
        fs::create_dir_all(RESULTS_DIR).expect("결과 디렉토리 생성 실패");
    }
    fs::write(&report_path, report).expect("리포트 쓰기 실패");
    println!("\n리포트 저장됨: {}", report_path);
}
