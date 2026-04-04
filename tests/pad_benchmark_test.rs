//! PAD 분석기 벤치마크 — 다양한 무협 대사의 PAD 결과를 수집하여 품질 평가
//!
//! `cargo test --features embed --test pad_benchmark_test -- --nocapture`

#![cfg(feature = "embed")]

use npc_mind::adapter::ort_embedder::OrtEmbedder;
use npc_mind::adapter::file_anchor_source::{FileAnchorSource, AnchorFormat};
use npc_mind::domain::pad::PadAnalyzer;
use npc_mind::domain::pad_anchors::builtin_anchor_toml;
use npc_mind::ports::UtteranceAnalyzer;
use std::sync::{Mutex, OnceLock};

const MODEL_PATH: &str = "../models/bge-m3/model_quantized.onnx";
const TOKENIZER_PATH: &str = "../models/bge-m3/tokenizer.json";

fn shared_analyzer() -> &'static Mutex<PadAnalyzer> {
    static ANALYZER: OnceLock<Mutex<PadAnalyzer>> = OnceLock::new();
    ANALYZER.get_or_init(|| {
        let embedder = OrtEmbedder::new(MODEL_PATH, TOKENIZER_PATH).unwrap();
        let source = FileAnchorSource::from_content(builtin_anchor_toml("ko").unwrap(), AnchorFormat::Toml);
        Mutex::new(PadAnalyzer::new(Box::new(embedder), &source).unwrap())
    })
}

/// (대사, 기대P방향, 기대A방향, 기대D방향, 설명)
struct TestCase {
    utterance: &'static str,
    expected_p: f32, // >0 양수기대, <0 음수기대, 0 무관
    expected_a: f32,
    expected_d: f32,
    label: &'static str,
}

#[test]
fn pad_분석기_벤치마크() {
    let cases = vec![
        // === P축: 쾌/불쾌 ===
        TestCase {
            utterance: "네 이놈, 죽고 싶으냐!",
            expected_p: -1.0,
            expected_a: 1.0,
            expected_d: 1.0,
            label: "도발(강)",
        },
        TestCase {
            utterance: "은혜를 잊지 않겠습니다. 정말 감사합니다.",
            expected_p: 1.0,
            expected_a: -1.0,
            expected_d: -1.0,
            label: "감사",
        },
        TestCase {
            utterance: "배은망덕한 놈! 의리도 없는 것이!",
            expected_p: -1.0,
            expected_a: 1.0,
            expected_d: 1.0,
            label: "배신비난",
        },
        TestCase {
            utterance: "오늘 날씨가 좋군요.",
            expected_p: 1.0,
            expected_a: 0.0,
            expected_d: 0.0,
            label: "중립적 긍정",
        },
        TestCase {
            utterance: "내 아이가 죽었소... 모든 것이 끝났소.",
            expected_p: -1.0,
            expected_a: -1.0,
            expected_d: -1.0,
            label: "비탄/슬픔",
        },
        // === A축: 각성/이완 ===
        TestCase {
            utterance: "당장 목을 치겠다! 칼을 뽑아라!",
            expected_p: -1.0,
            expected_a: 1.0,
            expected_d: 1.0,
            label: "위협(고각성)",
        },
        TestCase {
            utterance: "편안히 쉬시게. 차 한잔 올리지.",
            expected_p: 1.0,
            expected_a: -1.0,
            expected_d: 1.0,
            label: "차분한 환대",
        },
        TestCase {
            utterance: "적이 쳐들어 온다! 모두 무기를 들어라!",
            expected_p: -1.0,
            expected_a: 1.0,
            expected_d: 1.0,
            label: "긴급/전투",
        },
        TestCase {
            utterance: "강물이 고요히 흐르는군. 세상도 이리 평온하면 좋으련만.",
            expected_p: 1.0,
            expected_a: -1.0,
            expected_d: 0.0,
            label: "명상/관조",
        },
        // === D축: 지배/복종 ===
        TestCase {
            utterance: "내가 주도한다. 물러서라, 이것이 명이다!",
            expected_p: 0.0,
            expected_a: 1.0,
            expected_d: 1.0,
            label: "명령(지배)",
        },
        TestCase {
            utterance: "제가 잘못했습니다. 어떤 벌이든 달게 받겠습니다.",
            expected_p: -1.0,
            expected_a: -1.0,
            expected_d: -1.0,
            label: "복종/자책",
        },
        TestCase {
            utterance: "감히 내 앞에서! 무릎 꿇어라!",
            expected_p: -1.0,
            expected_a: 1.0,
            expected_d: 1.0,
            label: "위압(지배)",
        },
        TestCase {
            utterance: "소인은 아무것도 모릅니다... 살려주십시오.",
            expected_p: -1.0,
            expected_a: 1.0,
            expected_d: -1.0,
            label: "애걸(복종)",
        },
        TestCase {
            utterance: "이 일은 내가 책임지겠소. 모두 뒤로 물러나시오.",
            expected_p: 0.0,
            expected_a: 1.0,
            expected_d: 1.0,
            label: "책임감(지배)",
        },
        // === 복합 감정 ===
        TestCase {
            utterance: "네가 날 배신하다니... 차라리 죽여라.",
            expected_p: -1.0,
            expected_a: 1.0,
            expected_d: -1.0,
            label: "배신+절망",
        },
        TestCase {
            utterance: "드디어 해냈다! 십 년의 수련이 헛되지 않았구나!",
            expected_p: 1.0,
            expected_a: 1.0,
            expected_d: 1.0,
            label: "성취감",
        },
        TestCase {
            utterance: "괜찮소, 누구나 실수하오. 다시 일어서면 되지.",
            expected_p: 1.0,
            expected_a: -1.0,
            expected_d: 1.0,
            label: "위로/격려",
        },
        TestCase {
            utterance: "흥, 네까짓 것이 나를 이길 수 있다고 생각했느냐?",
            expected_p: -1.0,
            expected_a: 1.0,
            expected_d: 1.0,
            label: "경멸/조롱",
        },
        TestCase {
            utterance: "형님, 같이 술이나 한잔합시다. 오랜만이오.",
            expected_p: 1.0,
            expected_a: -1.0,
            expected_d: 0.0,
            label: "친근함",
        },
        TestCase {
            utterance: "저... 혹시 괜찮으시다면... 함께 가도 될까요?",
            expected_p: 1.0,
            expected_a: 1.0,
            expected_d: -1.0,
            label: "수줍음/소심",
        },
    ];

    let mut analyzer = shared_analyzer().lock().unwrap();

    println!("\n{}", "=".repeat(90));
    println!("PAD 분석기 벤치마크 결과");
    println!("{}", "=".repeat(90));
    println!(
        "{:<16} {:>8} {:>8} {:>8}  {}",
        "라벨", "P", "A", "D", "대사"
    );
    println!("{}", "-".repeat(90));

    let mut p_correct = 0;
    let mut a_correct = 0;
    let mut d_correct = 0;
    let mut p_total = 0;
    let mut a_total = 0;
    let mut d_total = 0;

    for case in &cases {
        let pad = analyzer.analyze(case.utterance).expect("분석 실패");

        let p_ok = if case.expected_p > 0.0 {
            p_total += 1;
            pad.pleasure > 0.0
        } else if case.expected_p < 0.0 {
            p_total += 1;
            pad.pleasure < 0.0
        } else {
            true
        };
        let a_ok = if case.expected_a > 0.0 {
            a_total += 1;
            pad.arousal > 0.0
        } else if case.expected_a < 0.0 {
            a_total += 1;
            pad.arousal < 0.0
        } else {
            true
        };
        let d_ok = if case.expected_d > 0.0 {
            d_total += 1;
            pad.dominance > 0.0
        } else if case.expected_d < 0.0 {
            d_total += 1;
            pad.dominance < 0.0
        } else {
            true
        };

        if p_ok {
            p_correct += 1;
        }
        if a_ok {
            a_correct += 1;
        }
        if d_ok {
            d_correct += 1;
        }

        let p_mark = if case.expected_p != 0.0 {
            if p_ok { "✓" } else { "✗" }
        } else {
            " "
        };
        let a_mark = if case.expected_a != 0.0 {
            if a_ok { "✓" } else { "✗" }
        } else {
            " "
        };
        let d_mark = if case.expected_d != 0.0 {
            if d_ok { "✓" } else { "✗" }
        } else {
            " "
        };

        println!(
            "{:<16} {:>+7.3}{} {:>+7.3}{} {:>+7.3}{}  {}",
            case.label,
            pad.pleasure,
            p_mark,
            pad.arousal,
            a_mark,
            pad.dominance,
            d_mark,
            case.utterance
        );
    }

    println!("{}", "-".repeat(90));
    println!(
        "방향 정확도:  P={}/{} ({:.0}%)  A={}/{} ({:.0}%)  D={}/{} ({:.0}%)",
        p_correct,
        p_total,
        p_correct as f64 / p_total as f64 * 100.0,
        a_correct,
        a_total,
        a_correct as f64 / a_total as f64 * 100.0,
        d_correct,
        d_total,
        d_correct as f64 / d_total as f64 * 100.0,
    );
}
