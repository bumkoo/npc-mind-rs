//! 앵커 개수별 정확도 비교: 6개 vs 10개
//!
//! 현재 앵커(축당 6+6)에 무협 대사 스타일 4개씩 추가하여 10+10으로 확장 후 비교
//!
//! `cargo test --features embed --test pad_anchor_count_bench -- --nocapture`

#![cfg(feature = "embed")]

use std::sync::{Mutex, OnceLock};
use npc_mind::adapter::ort_embedder::OrtEmbedder;
use npc_mind::adapter::toml_anchor_source::TomlAnchorSource;
use npc_mind::domain::pad::PadAnalyzer;
use npc_mind::domain::pad_anchors::builtin_anchor_toml;
use npc_mind::ports::TextEmbedder;
use npc_mind::PadAnchorSource;

const MODEL_PATH: &str = "../models/bge-m3/model_quantized.onnx";
const TOKENIZER_PATH: &str = "../models/bge-m3/tokenizer.json";

fn shared_embedder() -> &'static Mutex<OrtEmbedder> {
    static EMB: OnceLock<Mutex<OrtEmbedder>> = OnceLock::new();
    EMB.get_or_init(|| {
        Mutex::new(OrtEmbedder::new(MODEL_PATH, TOKENIZER_PATH).unwrap())
    })
}

fn mean_vector(vectors: &[Vec<f32>]) -> Vec<f32> {
    if vectors.is_empty() { return Vec::new(); }
    let dim = vectors[0].len();
    let n = vectors.len() as f32;
    let mut mean = vec![0.0f32; dim];
    for v in vectors { for (i, val) in v.iter().enumerate() { mean[i] += val; } }
    for val in mean.iter_mut() { *val /= n; }
    mean
}

fn axis_score(emb: &[f32], pos: &[f32], neg: &[f32]) -> f32 {
    let sp = PadAnalyzer::cosine_sim(emb, pos);
    let sn = PadAnalyzer::cosine_sim(emb, neg);
    (sp - sn).clamp(-1.0, 1.0)
}

// ====================================================================
// 확장 앵커: 현재 6개 + 추가 4개 = 10개 (대사 톤 통일 유지)
// ====================================================================

const P_POS_10: &[&str] = &[
    // 기존 6개
    "참으로 기쁘고 흐뭇하구려.",
    "마음이 따뜻해지는군. 이런 게 행복이란 것이지.",
    "이토록 훌륭하니 진심으로 만족스럽소.",
    "괜찮소, 걱정하지 마시오. 모든 것이 잘 될 것이오.",
    "은혜를 잊지 않겠소. 정말 고맙소.",
    "오랜만이오, 반갑소! 그간 무고하셨소?",
    // 추가 4개
    "이런 좋은 일이 생기다니, 참으로 다행이오.",
    "그대 덕분이오. 마음 깊이 감사드리오.",
    "하하, 이 맛에 사는 것 아니겠소?",
    "이 기쁨을 함께 나눌 수 있어 더없이 좋소.",
];

const P_NEG_10: &[&str] = &[
    "정말 괴롭고 불쾌하기 짝이 없군.",
    "마음이 아프고 고통스러워 견딜 수가 없구나.",
    "이리 당하다니 참으로 분하고 원통하다!",
    "배은망덕한 놈! 네놈이 어찌 그럴 수 있느냐!",
    "꺼져라. 네 꼴을 보기도 싫다.",
    "모든 것이 끝이다. 아무런 희망이 없다.",
    // 추가 4개
    "이토록 비참한 꼴을 당하다니, 치가 떨린다.",
    "속이 끓어 당장이라도 뒤엎어버리고 싶구나.",
    "내 신세가 이 지경이 되다니, 살아서 무엇하랴.",
    "너 같은 놈은 용서할 수 없다. 절대로.",
];

const A_POS_10: &[&str] = &[
    "피가 끓어오르고 주체할 수 없이 흥분되는군!",
    "헉, 헉... 긴장해서 심장이 터질 것 같다.",
    "도저히 가만히 있을 수가 없다! 몸이 달아오른다!",
    "적이 쳐들어 온다! 모두 무기를 들어라!",
    "당장 칼을 뽑아라! 죽여주마!",
    "어서! 지금 당장 움직여야 한다!",
    // 추가 4개
    "빨리! 한시가 급하다, 지체하면 늦는다!",
    "온몸의 신경이 곤두선다. 한눈팔 수 없다!",
    "비켜라! 길을 막으면 베어버리겠다!",
    "심장이 귓가에서 쿵쿵거린다. 전투가 시작된다!",
];

const A_NEG_10: &[&str] = &[
    "마음이 한없이 차분하고 담담해지는구려.",
    "주변이 참으로 평온하고 고요하군.",
    "몸도 마음도 편안하고 여유롭소.",
    "천천히 하시오. 서두를 것 없소.",
    "편히 쉬시게. 차 한잔 드시오.",
    "강물처럼 흘러가는 대로 두면 되오.",
    // 추가 4개
    "급할 것 없소. 세월이 약이라 했소.",
    "바람결에 몸을 맡기니 마음도 가벼워지는군.",
    "조용히 눈을 감으니 모든 것이 고요하구려.",
    "한숨 돌리시오. 아직 여유가 있소.",
];

const D_POS_10: &[&str] = &[
    "내가 주도한다, 물러서라!",
    "이곳의 모든 상황은 내 통제 아래에 있다.",
    "내가 해내지 못할 일은 천하에 없다.",
    "감히! 무릎 꿇어라! 이것은 명이다!",
    "내 결정에 이의를 달 자가 있느냐?",
    "이 일은 내가 책임지겠소. 뒤로 물러나시오.",
    // 추가 4개
    "내 허락 없이는 아무도 이 문을 나설 수 없다.",
    "입 닥쳐라. 내가 말할 때는 듣기만 해라.",
    "네놈의 운명은 내 손에 달렸다. 잘 생각해라.",
    "내가 나서면 끝이다. 더 이상의 논의는 불필요하다.",
];

const D_NEG_10: &[&str] = &[
    "눈앞이 캄캄하군... 어찌해야 할지 모르겠어.",
    "아무것도 할 수 없다니, 이리도 무력할 수가...",
    "위축되어 숨조차 제대로 쉴 수가 없구나.",
    "소인은 감히 거역할 수 없습니다. 처분을 기다리겠습니다.",
    "살려주십시오... 무엇이든 하겠습니다.",
    "저... 혹시 괜찮으시다면... 따라가도 될까요?",
    // 추가 4개
    "분부만 내려주십시오. 소인이 따르겠습니다.",
    "제가 감히 어찌... 대인의 뜻을 거스를 수 있겠습니까.",
    "어디로 가야 할지, 무엇을 해야 할지 막막합니다.",
    "소인 같은 것이 무슨 수로... 그저 바라볼 뿐입니다.",
];

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
fn 앵커_6개_vs_10개_비교() {
    let mut embedder = shared_embedder().lock().unwrap();

    // 1) 10개 앵커 임베딩
    println!("\n[1] 10개 앵커 임베딩 중...");
    let p10_pos = mean_vector(&embedder.embed(P_POS_10).unwrap());
    let p10_neg = mean_vector(&embedder.embed(P_NEG_10).unwrap());
    let a10_pos = mean_vector(&embedder.embed(A_POS_10).unwrap());
    let a10_neg = mean_vector(&embedder.embed(A_NEG_10).unwrap());
    let d10_pos = mean_vector(&embedder.embed(D_POS_10).unwrap());
    let d10_neg = mean_vector(&embedder.embed(D_NEG_10).unwrap());

    // 2) 프로덕션 앵커 (PadAnalyzer, 외부 TOML 로드)
    println!("[2] 프로덕션 앵커(PadAnalyzer) 초기화...");
    drop(embedder);
    let source = TomlAnchorSource::from_content(builtin_anchor_toml("ko").unwrap());
    let analyzer = PadAnalyzer::new(Box::new(
        OrtEmbedder::new(MODEL_PATH, TOKENIZER_PATH).unwrap()
    ), &source).unwrap();
    let mut embedder2 = OrtEmbedder::new(MODEL_PATH, TOKENIZER_PATH).unwrap();

    // 3) 테스트 대사 임베딩
    let cases = test_cases();
    let utterances: Vec<&str> = cases.iter().map(|c| c.utterance).collect();
    let embeddings = embedder2.embed(&utterances).unwrap();

    println!("[3] {} 대사 분석 중...\n", cases.len());
    println!("{}", "=".repeat(130));
    println!("앵커 6개 vs 10개 PAD 분석 비교");
    println!("{}", "=".repeat(130));
    println!("{:<14} {:>22}   {:>22}   {:>22}",
        "", "P축", "A축", "D축");
    println!("{:<14} {:>10} {:>10}   {:>10} {:>10}   {:>10} {:>10}",
        "라벨", "6개", "10개", "6개", "10개", "6개", "10개");
    println!("{}", "-".repeat(130));

    let mut s6_p = 0; let mut s6_a = 0; let mut s6_d = 0;
    let mut s10_p = 0; let mut s10_a = 0; let mut s10_d = 0;
    let mut pt = 0; let mut at = 0; let mut dt = 0;

    for (i, case) in cases.iter().enumerate() {
        let emb = &embeddings[i];
        // 6개 앵커 (PadAnalyzer)
        let c6 = analyzer.to_pad(emb);
        // 10개 앵커
        let p10 = axis_score(emb, &p10_pos, &p10_neg);
        let a10 = axis_score(emb, &a10_pos, &a10_neg);
        let d10 = axis_score(emb, &d10_pos, &d10_neg);

        if case.expected_p != 0.0 {
            pt += 1;
            if (case.expected_p > 0.0) == (c6.pleasure > 0.0) { s6_p += 1; }
            if (case.expected_p > 0.0) == (p10 > 0.0) { s10_p += 1; }
        }
        if case.expected_a != 0.0 {
            at += 1;
            if (case.expected_a > 0.0) == (c6.arousal > 0.0) { s6_a += 1; }
            if (case.expected_a > 0.0) == (a10 > 0.0) { s10_a += 1; }
        }
        if case.expected_d != 0.0 {
            dt += 1;
            if (case.expected_d > 0.0) == (c6.dominance > 0.0) { s6_d += 1; }
            if (case.expected_d > 0.0) == (d10 > 0.0) { s10_d += 1; }
        }

        let mk = |e: f32, v6: f32, v10: f32| -> (&str, &str) {
            let m6 = if e != 0.0 { if (e > 0.0) == (v6 > 0.0) { "✓" } else { "✗" } } else { " " };
            let m10 = if e != 0.0 { if (e > 0.0) == (v10 > 0.0) { "✓" } else { "✗" } } else { " " };
            (m6, m10)
        };
        let (pm6, pm10) = mk(case.expected_p, c6.pleasure, p10);
        let (am6, am10) = mk(case.expected_a, c6.arousal, a10);
        let (dm6, dm10) = mk(case.expected_d, c6.dominance, d10);

        println!("{:<14} {:>+8.3}{}  {:>+8.3}{}  {:>+8.3}{}  {:>+8.3}{}  {:>+8.3}{}  {:>+8.3}{}",
            case.label,
            c6.pleasure, pm6, p10, pm10,
            c6.arousal, am6, a10, am10,
            c6.dominance, dm6, d10, dm10,
        );
    }

    println!("{}", "-".repeat(130));
    println!("방향 정확도:");
    println!("  P: 6개 {}/{} ({:.0}%)  10개 {}/{} ({:.0}%)",
        s6_p, pt, s6_p as f64/pt as f64*100.0, s10_p, pt, s10_p as f64/pt as f64*100.0);
    println!("  A: 6개 {}/{} ({:.0}%)  10개 {}/{} ({:.0}%)",
        s6_a, at, s6_a as f64/at as f64*100.0, s10_a, at, s10_a as f64/at as f64*100.0);
    println!("  D: 6개 {}/{} ({:.0}%)  10개 {}/{} ({:.0}%)",
        s6_d, dt, s6_d as f64/dt as f64*100.0, s10_d, dt, s10_d as f64/dt as f64*100.0);
    let t6 = s6_p+s6_a+s6_d; let t10 = s10_p+s10_a+s10_d; let tt = pt+at+dt;
    println!("  합계: 6개 {}/{} ({:.0}%)  10개 {}/{} ({:.0}%)",
        t6, tt, t6 as f64/tt as f64*100.0, t10, tt, t10 as f64/tt as f64*100.0);
}
