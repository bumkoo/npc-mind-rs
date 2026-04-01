//! 확정 앵커 60개 × 대사 20개 — 개별 Dense cos_sim 행렬
//!
//! `cargo test --features embed --test pad_individual_scores -- --nocapture`

#![cfg(feature = "embed")]

use bge_m3_onnx_rust::{BgeM3Embedder, cosine_similarity};
use std::sync::{Mutex, OnceLock};

const MODEL_PATH: &str = "../models/bge-m3/model_quantized.onnx";
const TOKENIZER_PATH: &str = "../models/bge-m3/tokenizer.json";

fn shared_embedder() -> &'static Mutex<BgeM3Embedder> {
    static EMB: OnceLock<Mutex<BgeM3Embedder>> = OnceLock::new();
    EMB.get_or_init(|| {
        bge_m3_onnx_rust::init_ort();
        Mutex::new(BgeM3Embedder::new(MODEL_PATH, TOKENIZER_PATH).unwrap())
    })
}

struct AxisAnchors {
    label: &'static str,
    positive: &'static [&'static str],
    negative: &'static [&'static str],
}

const AXES: [AxisAnchors; 3] = [
    AxisAnchors {
        label: "P",
        positive: &[
            "참으로 기쁘고 흐뭇하구려.",
            "마음이 따뜻해지는군. 이런 게 행복이란 것이지.",
            "이토록 훌륭하니 진심으로 만족스럽소.",
            "괜찮소, 걱정하지 마시오. 모든 것이 잘 될 것이오.",
            "은혜를 잊지 않겠소. 정말 고맙소.",
            "오랜만이오, 반갑소! 그간 무고하셨소?",
            "이런 좋은 일이 생기다니, 참으로 다행이오.",
            "그대 덕분이오. 마음 깊이 감사드리오.",
            "하하, 이 맛에 사는 것 아니겠소?",
            "이 기쁨을 함께 나눌 수 있어 더없이 좋소.",
        ],
        negative: &[
            "정말 괴롭고 불쾌하기 짝이 없군.",
            "마음이 아프고 고통스러워 견딜 수가 없구나.",
            "이리 당하다니 참으로 분하고 원통하다!",
            "배은망덕한 놈! 네놈이 어찌 그럴 수 있느냐!",
            "꺼져라. 네 꼴을 보기도 싫다.",
            "모든 것이 끝이다. 아무런 희망이 없다.",
            "이토록 비참한 꼴을 당하다니, 치가 떨린다.",
            "속이 끓어 당장이라도 뒤엎어버리고 싶구나.",
            "내 신세가 이 지경이 되다니, 살아서 무엇하랴.",
            "너 같은 놈은 용서할 수 없다. 절대로.",
        ],
    },
    AxisAnchors {
        label: "A",
        positive: &[
            "피가 끓어오르고 주체할 수 없이 흥분되는군!",
            "헉, 헉... 긴장해서 심장이 터질 것 같다.",
            "도저히 가만히 있을 수가 없다! 몸이 달아오른다!",
            "적이 쳐들어 온다! 모두 무기를 들어라!",
            "검이 부딪히는 굉음에 온몸의 피가 역류하는구나!",
            "어서! 지금 당장 움직여야 한다!",
            "빨리! 한시가 급하다, 지체하면 늦는다!",
            "온몸의 신경이 곤두선다. 한눈팔 수 없다!",
            "사방에서 뿜어져 나오는 살기에 숨이 턱턱 막혀온다!",
            "심장이 귓가에서 쿵쿵거린다. 전투가 시작된다!",
        ],
        negative: &[
            "마음이 한없이 차분하고 담담해지는구려.",
            "주변이 참으로 평온하고 고요하군.",
            "몸도 마음도 편안하고 여유롭소.",
            "천천히 하시오. 서두를 것 없소.",
            "편히 쉬시게. 차 한잔 드시오.",
            "강물처럼 흘러가는 대로 두면 되오.",
            "급할 것 없소. 세월이 약이라 했소.",
            "바람결에 몸을 맡기니 마음도 가벼워지는군.",
            "조용히 눈을 감으니 모든 것이 고요하구려.",
            "한숨 돌리시오. 아직 여유가 있소.",
        ],
    },
    AxisAnchors {
        label: "D",
        positive: &[
            "내가 주도한다, 물러서라!",
            "이곳의 모든 상황은 내 통제 아래에 있다.",
            "내가 해내지 못할 일은 천하에 없다.",
            "감히! 무릎 꿇어라! 이것은 명이다!",
            "내 결정에 이의를 달 자가 있느냐?",
            "이 일은 내가 책임지겠소. 뒤로 물러나시오.",
            "내 허락 없이는 아무도 이 문을 나설 수 없다.",
            "입 닥쳐라. 내가 말할 때는 듣기만 해라.",
            "네놈의 운명은 내 손에 달렸다. 잘 생각해라.",
            "내가 나서면 끝이다. 더 이상의 논의는 불필요하다.",
        ],
        negative: &[
            "눈앞이 캄캄하군... 어찌해야 할지 모르겠어.",
            "아무것도 할 수 없다니, 이리도 무력할 수가...",
            "위축되어 숨조차 제대로 쉴 수가 없구나.",
            "소인은 감히 거역할 수 없습니다. 처분을 기다리겠습니다.",
            "살려주십시오... 무엇이든 하겠습니다.",
            "저... 혹시 괜찮으시다면... 따라가도 될까요?",
            "제가 감히 어찌... 대인의 뜻을 거스를 수 있겠습니까.",
            "이제 발버둥 칠 힘조차 없으니, 네놈 좋을 대로 처분해라.",
            "이리 억울하게 당해야만 하다니, 내 숨통을 끊어놓든 마음대로 하시지요.",
            "어차피 내 뜻대로 되는 것은 하나도 없으니, 네 마음대로 해라.",
        ],
    },
];

struct TestCase {
    utterance: &'static str,
    expected: [f32; 3],
    label: &'static str,
}

fn test_cases() -> Vec<TestCase> {
    vec![
        TestCase {
            utterance: "네 이놈, 죽고 싶으냐!",
            expected: [-1.0, 1.0, 1.0],
            label: "도발",
        },
        TestCase {
            utterance: "은혜를 잊지 않겠습니다. 정말 감사합니다.",
            expected: [1.0, -1.0, -1.0],
            label: "감사",
        },
        TestCase {
            utterance: "배은망덕한 놈! 의리도 없는 것이!",
            expected: [-1.0, 1.0, 1.0],
            label: "배신",
        },
        TestCase {
            utterance: "오늘 날씨가 좋군요.",
            expected: [1.0, 0.0, 0.0],
            label: "중립",
        },
        TestCase {
            utterance: "내 아이가 죽었소... 모든 것이 끝났소.",
            expected: [-1.0, -1.0, -1.0],
            label: "비탄",
        },
        TestCase {
            utterance: "당장 목을 치겠다! 칼을 뽑아라!",
            expected: [-1.0, 1.0, 1.0],
            label: "위협",
        },
        TestCase {
            utterance: "편안히 쉬시게. 차 한잔 올리지.",
            expected: [1.0, -1.0, 1.0],
            label: "환대",
        },
        TestCase {
            utterance: "적이 쳐들어 온다! 모두 무기를 들어라!",
            expected: [-1.0, 1.0, 1.0],
            label: "긴급",
        },
        TestCase {
            utterance: "강물이 고요히 흐르는군. 세상도 이리 평온하면 좋으련만.",
            expected: [1.0, -1.0, 0.0],
            label: "관조",
        },
        TestCase {
            utterance: "내가 주도한다. 물러서라, 이것이 명이다!",
            expected: [0.0, 1.0, 1.0],
            label: "명령",
        },
        TestCase {
            utterance: "제가 잘못했습니다. 어떤 벌이든 달게 받겠습니다.",
            expected: [-1.0, -1.0, -1.0],
            label: "복종",
        },
        TestCase {
            utterance: "감히 내 앞에서! 무릎 꿇어라!",
            expected: [-1.0, 1.0, 1.0],
            label: "위압",
        },
        TestCase {
            utterance: "소인은 아무것도 모릅니다... 살려주십시오.",
            expected: [-1.0, 1.0, -1.0],
            label: "애걸",
        },
        TestCase {
            utterance: "이 일은 내가 책임지겠소. 모두 뒤로 물러나시오.",
            expected: [0.0, 1.0, 1.0],
            label: "책임",
        },
        TestCase {
            utterance: "네가 날 배신하다니... 차라리 죽여라.",
            expected: [-1.0, 1.0, -1.0],
            label: "절망",
        },
        TestCase {
            utterance: "드디어 해냈다! 십 년의 수련이 헛되지 않았구나!",
            expected: [1.0, 1.0, 1.0],
            label: "성취",
        },
        TestCase {
            utterance: "괜찮소, 누구나 실수하오. 다시 일어서면 되지.",
            expected: [1.0, -1.0, 1.0],
            label: "위로",
        },
        TestCase {
            utterance: "흥, 네까짓 것이 나를 이길 수 있다고 생각했느냐?",
            expected: [-1.0, 1.0, 1.0],
            label: "경멸",
        },
        TestCase {
            utterance: "형님, 같이 술이나 한잔합시다. 오랜만이오.",
            expected: [1.0, -1.0, 0.0],
            label: "친근",
        },
        TestCase {
            utterance: "저... 혹시 괜찮으시다면... 함께 가도 될까요?",
            expected: [1.0, 1.0, -1.0],
            label: "소심",
        },
    ]
}

#[test]
fn 개별_앵커_dense_점수() {
    let mut emb = shared_embedder().lock().unwrap();
    let cases = test_cases();

    // 1) 앵커 임베딩
    println!("\n[1] 60개 앵커 Dense 임베딩...");
    struct AxisEmb {
        label: &'static str,
        pos: Vec<Vec<f32>>,
        neg: Vec<Vec<f32>>,
    }
    let mut axes: Vec<AxisEmb> = Vec::new();
    for ax in &AXES {
        let pos: Vec<Vec<f32>> = ax
            .positive
            .iter()
            .map(|t| emb.encode(t).unwrap().dense)
            .collect();
        let neg: Vec<Vec<f32>> = ax
            .negative
            .iter()
            .map(|t| emb.encode(t).unwrap().dense)
            .collect();
        axes.push(AxisEmb {
            label: ax.label,
            pos,
            neg,
        });
    }

    // 2) 대사 임베딩
    println!("[2] 20개 대사 Dense 임베딩...\n");
    let utts: Vec<(&&str, Vec<f32>, [f32; 3])> = cases
        .iter()
        .map(|c| {
            (
                &c.utterance,
                emb.encode(c.utterance).unwrap().dense,
                c.expected,
            )
        })
        .collect();

    // 3) 축별 출력
    for (ax_idx, axis) in axes.iter().enumerate() {
        println!("{}", "=".repeat(170));
        println!(
            "{}축 Dense cos_sim (+ 양극 10개 / - 음극 10개) + 집계",
            axis.label
        );
        println!("{}", "=".repeat(170));
        print!("{:<6}", "대사");
        for i in 1..=10 {
            print!("  {:>5}", format!("+{}", i));
        }
        print!("  |");
        for i in 1..=10 {
            print!("  {:>5}", format!("-{}", i));
        }
        println!("  | {:>6} {:>6} {:>6} | exp", "mean", "top3", "max");
        println!("{}", "-".repeat(170));

        for (i, case) in cases.iter().enumerate() {
            let dense = &utts[i].1;
            let exp = utts[i].2[ax_idx];

            let ps: Vec<f32> = axis
                .pos
                .iter()
                .map(|a| cosine_similarity(dense, a))
                .collect();
            let ns: Vec<f32> = axis
                .neg
                .iter()
                .map(|a| cosine_similarity(dense, a))
                .collect();

            let pm: f32 = ps.iter().sum::<f32>() / 10.0;
            let nm: f32 = ns.iter().sum::<f32>() / 10.0;
            let mean = pm - nm;

            let mut sp = ps.clone();
            sp.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let mut sn = ns.clone();
            sn.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let top3 = sp[..3].iter().sum::<f32>() / 3.0 - sn[..3].iter().sum::<f32>() / 3.0;
            let max_s = sp[0] - sn[0];

            let dir = if exp > 0.0 {
                "+"
            } else if exp < 0.0 {
                "-"
            } else {
                "0"
            };
            let mk = |v: f32| -> &str {
                if exp == 0.0 {
                    " "
                } else if (exp > 0.0) == (v > 0.0) {
                    "✓"
                } else {
                    "✗"
                }
            };

            print!("{:<6}", case.label);
            for s in &ps {
                print!("  {:>.3}", s);
            }
            print!("  |");
            for s in &ns {
                print!("  {:>.3}", s);
            }
            println!(
                "  | {:>+.3}{} {:>+.3}{} {:>+.3}{} | {}",
                mean,
                mk(mean),
                top3,
                mk(top3),
                max_s,
                mk(max_s),
                dir
            );
        }
        println!();
    }

    // 4) 전략별 방향 정확도 요약
    println!("{}", "=".repeat(80));
    println!("전략별 방향 정확도 요약 (확정 앵커 10개)");
    println!("{}", "=".repeat(80));
    println!(
        "{:<6} {:>14} {:>14} {:>14}",
        "축", "D:mean", "D:top3", "D:max"
    );
    println!("{}", "-".repeat(80));

    let mut total = [0u32; 3]; // mean, top3, max
    let mut total_n = 0u32;

    for (ax_idx, axis) in axes.iter().enumerate() {
        let mut ok = [0u32; 3];
        let mut n = 0u32;
        for (i, case) in cases.iter().enumerate() {
            let exp = case.expected[ax_idx];
            if exp == 0.0 {
                continue;
            }
            n += 1;
            let dense = &utts[i].1;
            let ps: Vec<f32> = axis
                .pos
                .iter()
                .map(|a| cosine_similarity(dense, a))
                .collect();
            let ns: Vec<f32> = axis
                .neg
                .iter()
                .map(|a| cosine_similarity(dense, a))
                .collect();
            let pm: f32 = ps.iter().sum::<f32>() / 10.0;
            let nm: f32 = ns.iter().sum::<f32>() / 10.0;
            let mut sp = ps.clone();
            sp.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let mut sn = ns.clone();
            sn.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let mean = pm - nm;
            let top3 = sp[..3].iter().sum::<f32>() / 3.0 - sn[..3].iter().sum::<f32>() / 3.0;
            let max_s = sp[0] - sn[0];
            if (exp > 0.0) == (mean > 0.0) {
                ok[0] += 1;
            }
            if (exp > 0.0) == (top3 > 0.0) {
                ok[1] += 1;
            }
            if (exp > 0.0) == (max_s > 0.0) {
                ok[2] += 1;
            }
        }
        for j in 0..3 {
            total[j] += ok[j];
        }
        total_n += n;
        println!(
            "{:<6} {:>5}/{} ({:>3.0}%) {:>5}/{} ({:>3.0}%) {:>5}/{} ({:>3.0}%)",
            axis.label,
            ok[0],
            n,
            ok[0] as f64 / n as f64 * 100.0,
            ok[1],
            n,
            ok[1] as f64 / n as f64 * 100.0,
            ok[2],
            n,
            ok[2] as f64 / n as f64 * 100.0
        );
    }
    println!("{}", "-".repeat(80));
    println!(
        "{:<6} {:>5}/{} ({:>3.0}%) {:>5}/{} ({:>3.0}%) {:>5}/{} ({:>3.0}%)",
        "합계",
        total[0],
        total_n,
        total[0] as f64 / total_n as f64 * 100.0,
        total[1],
        total_n,
        total[1] as f64 / total_n as f64 * 100.0,
        total[2],
        total_n,
        total[2] as f64 / total_n as f64 * 100.0
    );
}
