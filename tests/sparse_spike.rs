//! Sparse 조회 스파이크 (Phase 3 대안 탐색)
//!
//! 목적: BGE-M3의 sparse(lexical) 임베딩이 정규식 prefilter를 대체/보완할 수 있는지 검증.
//! 스파이크 원칙: 기존 코드 0 변경. BgeM3Embedder 직접 사용.
//!
//! 측정:
//!   - 26 테스트 케이스 × 4 카테고리 × 3 프로토타입 = 312 sparse_dot_product
//!   - 카테고리별 top-1 프로토타입 점수 → threshold(0.3) 초과 시 hit
//!   - 정규식 prefilter 결과와 교차 비교
//!   - 기여 토큰 로깅 — patterns.toml 대체 근거 자료
//!
//! 실행: `cargo test --features embed --test sparse_spike -- --nocapture`

#![cfg(all(feature = "embed", feature = "listener_perspective"))]

use bge_m3_onnx_rust::{BgeM3Embedder, sparse_dot_product};
use npc_mind::domain::listener_perspective::Prefilter;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use tokenizers::Tokenizer;

const MODEL_PATH: &str = "../models/bge-m3/model_quantized.onnx";
const TOKENIZER_PATH: &str = "../models/bge-m3/tokenizer.json";
const BENCH_PATH: &str = "data/listener_perspective/testcases/sign_benchmark.toml";
const PATTERNS_PATH: &str = "data/listener_perspective/prefilter/patterns.toml";

const THRESHOLD: f32 = 0.3;

// ============================================================
// TOML 스키마
// ============================================================

#[derive(Debug, Deserialize)]
struct BenchmarkFile {
    #[allow(dead_code)]
    meta: BenchmarkMeta,
    #[serde(rename = "case")]
    cases: Vec<TestCase>,
}

#[derive(Debug, Deserialize)]
struct BenchmarkMeta {
    #[allow(dead_code)]
    version: String,
}

#[derive(Debug, Deserialize, Clone)]
struct TestCase {
    id: String,
    utterance: String,
    #[allow(dead_code)]
    expected_sign: String,
    #[allow(dead_code)]
    listener_p_magnitude: String,
    #[allow(dead_code)]
    difficulty: String,
    #[allow(dead_code)]
    subtype: String,
}

// ============================================================
// 프로토타입 세트 (카테고리당 3, 테스트 어휘 회피)
// ============================================================

struct CategoryPrototypes {
    name: &'static str,
    examples: &'static [&'static str],
}

const PROTOTYPES: &[CategoryPrototypes] = &[
    CategoryPrototypes {
        name: "counterfactual_gratitude",
        examples: &[
            "자네 덕분에 황천길을 면하였소",
            "당신 없었으면 시신이나 되었을 뻔",
            "구원해 주시지 않았다면 어찌 살아남았겠소",
        ],
    },
    CategoryPrototypes {
        name: "negation_praise",
        examples: &[
            "전무후무한 기재요",
            "세상에 두 번 없을 명장이시오",
            "비견할 자 없는 고수라 하겠소",
        ],
    },
    CategoryPrototypes {
        name: "wuxia_criticism",
        examples: &[
            "강호의 도를 저버린 망나니요",
            "협객의 자격이 없는 자요",
            "천인공노할 흉적이오",
        ],
    },
    CategoryPrototypes {
        name: "sarcasm_interjection",
        examples: &[
            "허어, 대단하신 분이구려",
            "어이구, 훌륭하신 말씀이시오",
            "참으로 기특한 생각이로다",
        ],
    },
];

// ============================================================
// 유틸
// ============================================================

fn load_benchmark() -> BenchmarkFile {
    let content = fs::read_to_string(BENCH_PATH).expect("벤치 로드 실패");
    toml::from_str(&content).expect("벤치 파싱 실패")
}

fn load_tokenizer() -> Tokenizer {
    Tokenizer::from_file(TOKENIZER_PATH).expect("토크나이저 로드 실패")
}

/// token_id → 텍스트 복원 (디버깅용)
fn token_text(tok: &Tokenizer, id: u32) -> String {
    tok.decode(&[id], true).unwrap_or_else(|_| format!("<{}>", id))
}

/// 상위 N개 기여 토큰을 "token:weight" 문자열로
fn format_contributors(
    utterance_sparse: &HashMap<u32, f32>,
    proto_sparse: &HashMap<u32, f32>,
    tok: &Tokenizer,
    top_n: usize,
) -> String {
    let mut contribs: Vec<(u32, f32)> = utterance_sparse
        .iter()
        .filter_map(|(tid, w_u)| proto_sparse.get(tid).map(|w_p| (*tid, w_u * w_p)))
        .collect();
    contribs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    contribs.truncate(top_n);
    contribs
        .iter()
        .map(|(tid, score)| format!("{}:{:.3}", token_text(tok, *tid).trim(), score))
        .collect::<Vec<_>>()
        .join(" + ")
}

// ============================================================
// 분류 로직
// ============================================================

/// 카테고리 내 프로토타입 중 top-1 점수와 해당 프로토타입 인덱스
struct CategoryScore {
    category: &'static str,
    top_score: f32,
    top_proto_idx: usize,
}

fn score_categories(
    utterance_sparse: &HashMap<u32, f32>,
    proto_sparses: &[Vec<HashMap<u32, f32>>],
) -> Vec<CategoryScore> {
    proto_sparses
        .iter()
        .zip(PROTOTYPES.iter())
        .map(|(protos, cat)| {
            let (top_idx, top_score) = protos
                .iter()
                .enumerate()
                .map(|(i, p)| (i, sparse_dot_product(utterance_sparse, p)))
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .unwrap_or((0, 0.0));
            CategoryScore {
                category: cat.name,
                top_score,
                top_proto_idx: top_idx,
            }
        })
        .collect()
}

/// threshold 초과하는 최고 점수 카테고리 반환
fn classify(scores: &[CategoryScore], threshold: f32) -> Option<&CategoryScore> {
    scores
        .iter()
        .filter(|s| s.top_score >= threshold)
        .max_by(|a, b| a.top_score.partial_cmp(&b.top_score).unwrap())
}

// ============================================================
// 메인 테스트
// ============================================================

#[test]
fn sparse_조회_스파이크() {
    // 1. 초기화
    bge_m3_onnx_rust::init_ort();
    let mut embedder = BgeM3Embedder::new(MODEL_PATH, TOKENIZER_PATH)
        .expect("BgeM3Embedder 초기화 실패");
    let tokenizer = load_tokenizer();
    let bench = load_benchmark();
    let prefilter = Prefilter::from_path(PATTERNS_PATH).expect("Prefilter 로드 실패");
    println!(
        "\n[1] 초기화 완료: {} 케이스, {} 프로토타입 카테고리, threshold={}\n",
        bench.cases.len(),
        PROTOTYPES.len(),
        THRESHOLD
    );

    // 2. 프로토타입 sparse 사전 계산 (4 × 3 = 12회)
    let proto_sparses: Vec<Vec<HashMap<u32, f32>>> = PROTOTYPES
        .iter()
        .map(|cat| {
            cat.examples
                .iter()
                .map(|ex| {
                    let out = embedder.encode(ex).expect("프로토타입 인코딩 실패");
                    out.sparse
                })
                .collect()
        })
        .collect();
    println!("[2] 프로토타입 sparse 임베딩 완료 ({} 벡터)\n",
        proto_sparses.iter().map(|v| v.len()).sum::<usize>());

    // 3. 각 케이스 처리
    println!("{}", "=".repeat(165));
    println!("Sparse 조회 스파이크 결과 (vs 정규식 Prefilter)");
    println!("{}", "=".repeat(165));
    println!(
        "{:<4} {:<32} {:<28} {:<28} {:>6}  {}",
        "id", "발화", "정규식 결과", "sparse 결과 (top1)", "점수", "기여 토큰"
    );
    println!("{}", "-".repeat(165));

    let mut agreement = 0;       // 정규식 hit + sparse hit + 카테고리 동일
    let mut sparse_only = 0;     // 정규식 miss + sparse hit
    let mut regex_only = 0;      // 정규식 hit + sparse miss
    let mut both_miss = 0;

    for case in &bench.cases {
        let out = embedder.encode(&case.utterance).expect("발화 인코딩 실패");
        let scores = score_categories(&out.sparse, &proto_sparses);
        let sparse_hit = classify(&scores, THRESHOLD);

        let regex_hit = prefilter.classify(&case.utterance);

        // 집계
        match (&regex_hit, sparse_hit) {
            (Some(r), Some(s)) => {
                if r.matched_category == s.category {
                    agreement += 1;
                } else {
                    regex_only += 1;
                }
            }
            (Some(_), None) => regex_only += 1,
            (None, Some(_)) => sparse_only += 1,
            (None, None) => both_miss += 1,
        }

        // 출력
        let regex_str = match &regex_hit {
            Some(r) => format!("{}", r.matched_category),
            None => "(miss)".to_string(),
        };
        let (sparse_str, score, contribs) = match sparse_hit {
            Some(s) => {
                let proto = &proto_sparses[PROTOTYPES.iter().position(|c| c.name == s.category).unwrap()][s.top_proto_idx];
                let c = format_contributors(&out.sparse, proto, &tokenizer, 4);
                (s.category.to_string(), s.top_score, c)
            }
            None => {
                // threshold 미달이어도 top 카테고리 점수는 보여줌
                let top = scores.iter().max_by(|a, b| a.top_score.partial_cmp(&b.top_score).unwrap()).unwrap();
                ("(miss)".to_string(), top.top_score, String::new())
            }
        };

        let utt_short = if case.utterance.chars().count() > 30 {
            format!("{}...", case.utterance.chars().take(28).collect::<String>())
        } else {
            case.utterance.clone()
        };

        println!(
            "{:<4} {:<32} {:<28} {:<28} {:>6.3}  {}",
            case.id, utt_short, regex_str, sparse_str, score, contribs
        );
    }
    println!("{}", "-".repeat(165));

    // 4. 요약
    let total = bench.cases.len();
    println!("\n=== 교차 비교 요약 ===");
    println!("정규식 + sparse 동일 카테고리 : {}/{}", agreement, total);
    println!("sparse만 hit                  : {}/{}", sparse_only, total);
    println!("정규식만 hit                  : {}/{}", regex_only, total);
    println!("둘 다 miss                     : {}/{}", both_miss, total);
}
