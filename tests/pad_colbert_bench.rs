//! Dense vs ColBERT PAD 분석 비교 벤치마크
//!
//! Dense: 앵커 평균벡터 vs 대사벡터 코사인유사도
//! ColBERT: 앵커 토큰벡터 vs 대사 토큰벡터 MaxSim
//!
//! `cargo test --features embed --test pad_colbert_bench -- --nocapture`

#![cfg(feature = "embed")]

use std::sync::{Mutex, OnceLock};
use npc_mind::adapter::ort_embedder::OrtEmbedder;
use npc_mind::adapter::toml_anchor_source::TomlAnchorSource;
use npc_mind::domain::pad::PadAnalyzer;
use npc_mind::domain::pad_anchors::builtin_anchor_toml;
use npc_mind::PadAnchorSource;
use bge_m3_onnx_rust::{BgeM3Embedder, max_sim};

const MODEL_PATH: &str = "../models/bge-m3/model_quantized.onnx";
const TOKENIZER_PATH: &str = "../models/bge-m3/tokenizer.json";

fn shared_embedder() -> &'static Mutex<BgeM3Embedder> {
    static EMB: OnceLock<Mutex<BgeM3Embedder>> = OnceLock::new();
    EMB.get_or_init(|| {
        bge_m3_onnx_rust::init_ort();
        Mutex::new(BgeM3Embedder::new(MODEL_PATH, TOKENIZER_PATH).unwrap())
    })
}

/// ColBERT 축 점수: MaxSim(대사, 양극앵커풀) - MaxSim(대사, 음극앵커풀)
fn colbert_axis_score(
    utterance_tokens: &[Vec<f32>],
    pos_pool: &[Vec<f32>],
    neg_pool: &[Vec<f32>],
) -> f32 {
    let pos_score = max_sim(utterance_tokens, pos_pool);
    let neg_score = max_sim(utterance_tokens, neg_pool);
    pos_score - neg_score
}

/// 앵커 텍스트 목록 → ColBERT 토큰 벡터 풀 (모든 앵커의 토큰을 합침)
fn build_colbert_pool(
    embedder: &mut BgeM3Embedder,
    anchors: &[String],
) -> Vec<Vec<f32>> {
    let mut pool = Vec::new();
    for text in anchors {
        let output = embedder.encode(text).expect("ColBERT 인코딩 실패");
        pool.extend(output.colbert);
    }
    pool
}

struct TestCase {
    utterance: &'static str,
    expected_p: f32,
    expected_a: f32,
    expected_d: f32,
    label: &'static str,
}

fn test_cases() -> Vec<TestCase> {
    vec![
        TestCase { utterance: "네 이놈, 죽고 싶으냐!", expected_p: -1.0, expected_a: 1.0, expected_d: 1.0, label: "도발(강)" },
        TestCase { utterance: "은혜를 잊지 않겠습니다. 정말 감사합니다.", expected_p: 1.0, expected_a: -1.0, expected_d: -1.0, label: "감사" },
        TestCase { utterance: "배은망덕한 놈! 의리도 없는 것이!", expected_p: -1.0, expected_a: 1.0, expected_d: 1.0, label: "배신비난" },
        TestCase { utterance: "오늘 날씨가 좋군요.", expected_p: 1.0, expected_a: 0.0, expected_d: 0.0, label: "중립적 긍정" },
        TestCase { utterance: "내 아이가 죽었소... 모든 것이 끝났소.", expected_p: -1.0, expected_a: -1.0, expected_d: -1.0, label: "비탄/슬픔" },
        TestCase { utterance: "당장 목을 치겠다! 칼을 뽑아라!", expected_p: -1.0, expected_a: 1.0, expected_d: 1.0, label: "위협(고각성)" },
        TestCase { utterance: "편안히 쉬시게. 차 한잔 올리지.", expected_p: 1.0, expected_a: -1.0, expected_d: 1.0, label: "차분한 환대" },
        TestCase { utterance: "적이 쳐들어 온다! 모두 무기를 들어라!", expected_p: -1.0, expected_a: 1.0, expected_d: 1.0, label: "긴급/전투" },
        TestCase { utterance: "강물이 고요히 흐르는군. 세상도 이리 평온하면 좋으련만.", expected_p: 1.0, expected_a: -1.0, expected_d: 0.0, label: "명상/관조" },
        TestCase { utterance: "내가 주도한다. 물러서라, 이것이 명이다!", expected_p: 0.0, expected_a: 1.0, expected_d: 1.0, label: "명령(지배)" },
        TestCase { utterance: "제가 잘못했습니다. 어떤 벌이든 달게 받겠습니다.", expected_p: -1.0, expected_a: -1.0, expected_d: -1.0, label: "복종/자책" },
        TestCase { utterance: "감히 내 앞에서! 무릎 꿇어라!", expected_p: -1.0, expected_a: 1.0, expected_d: 1.0, label: "위압(지배)" },
        TestCase { utterance: "소인은 아무것도 모릅니다... 살려주십시오.", expected_p: -1.0, expected_a: 1.0, expected_d: -1.0, label: "애걸(복종)" },
        TestCase { utterance: "이 일은 내가 책임지겠소. 모두 뒤로 물러나시오.", expected_p: 0.0, expected_a: 1.0, expected_d: 1.0, label: "책임감(지배)" },
        TestCase { utterance: "네가 날 배신하다니... 차라리 죽여라.", expected_p: -1.0, expected_a: 1.0, expected_d: -1.0, label: "배신+절망" },
        TestCase { utterance: "드디어 해냈다! 십 년의 수련이 헛되지 않았구나!", expected_p: 1.0, expected_a: 1.0, expected_d: 1.0, label: "성취감" },
        TestCase { utterance: "괜찮소, 누구나 실수하오. 다시 일어서면 되지.", expected_p: 1.0, expected_a: -1.0, expected_d: 1.0, label: "위로/격려" },
        TestCase { utterance: "흥, 네까짓 것이 나를 이길 수 있다고 생각했느냐?", expected_p: -1.0, expected_a: 1.0, expected_d: 1.0, label: "경멸/조롱" },
        TestCase { utterance: "형님, 같이 술이나 한잔합시다. 오랜만이오.", expected_p: 1.0, expected_a: -1.0, expected_d: 0.0, label: "친근함" },
        TestCase { utterance: "저... 혹시 괜찮으시다면... 함께 가도 될까요?", expected_p: 1.0, expected_a: 1.0, expected_d: -1.0, label: "수줍음/소심" },
    ]
}

#[test]
fn dense_vs_colbert_비교() {
    let mut embedder = shared_embedder().lock().unwrap();

    // 0) 앵커 로드
    let source = TomlAnchorSource::from_content(builtin_anchor_toml("ko").unwrap());
    let anchors = source.load_anchors().unwrap();

    // 1) ColBERT 앵커 풀 구축
    println!("\n[1] ColBERT 앵커 풀 구축 중...");
    let p_pos_pool = build_colbert_pool(&mut embedder, &anchors.pleasure.positive);
    let p_neg_pool = build_colbert_pool(&mut embedder, &anchors.pleasure.negative);
    let a_pos_pool = build_colbert_pool(&mut embedder, &anchors.arousal.positive);
    let a_neg_pool = build_colbert_pool(&mut embedder, &anchors.arousal.negative);
    let d_pos_pool = build_colbert_pool(&mut embedder, &anchors.dominance.positive);
    let d_neg_pool = build_colbert_pool(&mut embedder, &anchors.dominance.negative);
    println!("   P pool: +{}/−{} tokens, A pool: +{}/−{}, D pool: +{}/−{}",
        p_pos_pool.len(), p_neg_pool.len(),
        a_pos_pool.len(), a_neg_pool.len(),
        d_pos_pool.len(), d_neg_pool.len());

    // 2) Dense PadAnalyzer
    println!("[2] Dense PadAnalyzer 초기화...");
    drop(embedder);
    let analyzer = PadAnalyzer::new(Box::new(
        OrtEmbedder::new(MODEL_PATH, TOKENIZER_PATH).unwrap()
    ), &source).unwrap();
    let mut embedder2 = shared_embedder().lock().unwrap();

    // 3) 결과 비교
    let cases = test_cases();
    println!("[3] {} 대사 분석 중...\n", cases.len());

    println!("{}", "=".repeat(130));
    println!("Dense vs ColBERT PAD 분석 비교");
    println!("{}", "=".repeat(130));
    println!("{:<14} {:>22}   {:>22}   {:>22}",
        "", "P축", "A축", "D축");
    println!("{:<14} {:>10} {:>10}   {:>10} {:>10}   {:>10} {:>10}",
        "라벨", "Dense", "ColBERT", "Dense", "ColBERT", "Dense", "ColBERT");
    println!("{}", "-".repeat(130));

    let mut d_p_ok = 0; let mut d_a_ok = 0; let mut d_d_ok = 0;
    let mut c_p_ok = 0; let mut c_a_ok = 0; let mut c_d_ok = 0;
    let mut p_total = 0; let mut a_total = 0; let mut d_total = 0;

    for case in &cases {
        // ColBERT
        let output = embedder2.encode(case.utterance).expect("인코딩 실패");
        let cp = colbert_axis_score(&output.colbert, &p_pos_pool, &p_neg_pool);
        let ca = colbert_axis_score(&output.colbert, &a_pos_pool, &a_neg_pool);
        let cd = colbert_axis_score(&output.colbert, &d_pos_pool, &d_neg_pool);

        // Dense
        let dense_pad = analyzer.to_pad(&output.dense);
        let dp = dense_pad.pleasure;
        let da = dense_pad.arousal;
        let dd = dense_pad.dominance;

        // 방향 정확도 집계
        if case.expected_p != 0.0 {
            p_total += 1;
            if (case.expected_p > 0.0) == (dp > 0.0) { d_p_ok += 1; }
            if (case.expected_p > 0.0) == (cp > 0.0) { c_p_ok += 1; }
        }
        if case.expected_a != 0.0 {
            a_total += 1;
            if (case.expected_a > 0.0) == (da > 0.0) { d_a_ok += 1; }
            if (case.expected_a > 0.0) == (ca > 0.0) { c_a_ok += 1; }
        }
        if case.expected_d != 0.0 {
            d_total += 1;
            if (case.expected_d > 0.0) == (dd > 0.0) { d_d_ok += 1; }
            if (case.expected_d > 0.0) == (cd > 0.0) { c_d_ok += 1; }
        }

        let mark = |exp: f32, dense: f32, colbert: f32| -> (&str, &str) {
            let dm = if exp != 0.0 { if (exp > 0.0) == (dense > 0.0) { "✓" } else { "✗" } } else { " " };
            let cm = if exp != 0.0 { if (exp > 0.0) == (colbert > 0.0) { "✓" } else { "✗" } } else { " " };
            (dm, cm)
        };
        let (dpm, cpm) = mark(case.expected_p, dp, cp);
        let (dam, cam) = mark(case.expected_a, da, ca);
        let (ddm, cdm) = mark(case.expected_d, dd, cd);

        println!("{:<14} {:>+8.3}{}  {:>+8.3}{}  {:>+8.3}{}  {:>+8.3}{}  {:>+8.3}{}  {:>+8.3}{}",
            case.label,
            dp, dpm, cp, cpm,
            da, dam, ca, cam,
            dd, ddm, cd, cdm,
        );
    }

    println!("{}", "-".repeat(130));
    println!("방향 정확도 요약:");
    println!("  P축: Dense {}/{} ({:.0}%)  ColBERT {}/{} ({:.0}%)",
        d_p_ok, p_total, d_p_ok as f64 / p_total as f64 * 100.0,
        c_p_ok, p_total, c_p_ok as f64 / p_total as f64 * 100.0);
    println!("  A축: Dense {}/{} ({:.0}%)  ColBERT {}/{} ({:.0}%)",
        d_a_ok, a_total, d_a_ok as f64 / a_total as f64 * 100.0,
        c_a_ok, a_total, c_a_ok as f64 / a_total as f64 * 100.0);
    println!("  D축: Dense {}/{} ({:.0}%)  ColBERT {}/{} ({:.0}%)",
        d_d_ok, d_total, d_d_ok as f64 / d_total as f64 * 100.0,
        c_d_ok, d_total, c_d_ok as f64 / d_total as f64 * 100.0);
    println!("  합계: Dense {}/{} ({:.0}%)  ColBERT {}/{} ({:.0}%)",
        d_p_ok+d_a_ok+d_d_ok, p_total+a_total+d_total,
        (d_p_ok+d_a_ok+d_d_ok) as f64 / (p_total+a_total+d_total) as f64 * 100.0,
        c_p_ok+c_a_ok+c_d_ok, p_total+a_total+d_total,
        (c_p_ok+c_a_ok+c_d_ok) as f64 / (p_total+a_total+d_total) as f64 * 100.0);
}
