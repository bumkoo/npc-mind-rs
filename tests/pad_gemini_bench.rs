//! Gemini 제안 앵커 vs 현재 앵커 벤치마크
//!
//! 축당 7단계 × 10문장 = 210문장 (Gemini) vs 축당 2극 × 6문장 = 36문장 (현재)
//! `cargo test --features embed --test pad_gemini_bench -- --nocapture`

#![cfg(feature = "embed")]

use npc_mind::adapter::ort_embedder::OrtEmbedder;
use npc_mind::adapter::file_anchor_source::{FileAnchorSource, AnchorFormat};
use npc_mind::domain::pad::PadAnalyzer;
use npc_mind::domain::pad_anchors::builtin_anchor_toml;
use npc_mind::ports::TextEmbedder;
use std::sync::{Mutex, OnceLock};

const MODEL_PATH: &str = "../models/bge-m3/model_quantized.onnx";
const TOKENIZER_PATH: &str = "../models/bge-m3/tokenizer.json";

fn shared_embedder() -> &'static Mutex<OrtEmbedder> {
    static EMB: OnceLock<Mutex<OrtEmbedder>> = OnceLock::new();
    EMB.get_or_init(|| Mutex::new(OrtEmbedder::new(MODEL_PATH, TOKENIZER_PATH).unwrap()))
}

// ---- 순수 수학 (PadAnalyzer 내부 로직 재현) ----

fn mean_vector(vectors: &[Vec<f32>]) -> Vec<f32> {
    if vectors.is_empty() {
        return Vec::new();
    }
    let dim = vectors[0].len();
    let n = vectors.len() as f32;
    let mut mean = vec![0.0f32; dim];
    for v in vectors {
        for (i, val) in v.iter().enumerate() {
            mean[i] += val;
        }
    }
    for val in mean.iter_mut() {
        *val /= n;
    }
    mean
}

fn axis_score(emb: &[f32], pos: &[f32], neg: &[f32]) -> f32 {
    let sp = PadAnalyzer::cosine_sim(emb, pos);
    let sn = PadAnalyzer::cosine_sim(emb, neg);
    (sp - sn).clamp(-1.0, 1.0)
}

// ======================================================================
// Gemini 앵커: P축 (쾌/불쾌) — 양극 각 30문장
// ======================================================================

const GEMINI_P_POS: &[&str] = &[
    // P+3 환희 (4개)
    "세상을 다 가진 것처럼 너무나 행복하고 기쁩니다.",
    "벅차오르는 감동과 기쁨에 눈물이 날 지경입니다.",
    "가슴 터질 듯한 환희에 휩싸여 춤이라도 추고 싶습니다.",
    "바라던 모든 꿈이 이루어진 듯한 최고의 기분입니다.",
    // P+2 만족 (3개)
    "일이 아주 잘 풀려서 무척 만족스럽고 뿌듯합니다.",
    "당신의 따뜻한 배려와 친절에 진심으로 감사합니다.",
    "마음이 따뜻해지고 입가에 흐뭇한 미소가 번집니다.",
    // P+1 안도 (3개)
    "큰 문제가 없어서 마음이 꽤 놓이는군요.",
    "작은 여유가 생겨서 꽤나 평온하고 긍정적인 기분입니다.",
    "햇살이 좋아서 그런지 마음이 한결 가볍고 상쾌하군요.",
];

const GEMINI_P_NEG: &[&str] = &[
    // P-1 불만 (3개)
    "일이 조금 꼬이는 것 같아 은근히 신경이 쓰입니다.",
    "기대했던 것과 달라 가벼운 실망감을 느낍니다.",
    "원하던 방향이 아니라서 가벼운 실망감과 아쉬움이 남습니다.",
    // P-2 슬픔/분노 (3개)
    "너무나 슬프고 마음이 아파서 눈물이 흐릅니다.",
    "어처구니없는 상황에 화가 치밀어 오르고 불쾌합니다.",
    "부당한 대우를 받아 속이 끓어오르고 원망스럽습니다.",
    // P-3 절망 (4개)
    "끝없는 절망의 늪에 빠진 것처럼 비참하고 고통스럽습니다.",
    "하늘이 무너지는 듯한 끔찍하고 처절한 비극입니다.",
    "모든 것을 포기하고 싶을 만큼 끔찍하고 최악의 기분입니다.",
    "더 이상 버틸 수 없는 깊은 절망감에 숨조차 쉴 수 없습니다.",
];

// ======================================================================
// Gemini 앵커: A축 (각성/이완) — 양극 각 10문장
// ======================================================================

const GEMINI_A_POS: &[&str] = &[
    // A+3 패닉 (4개)
    "심장이 터질 듯이 쿵쾅거리고 온몸에 소름이 쫙 돋습니다!",
    "극도의 흥분 상태라 도저히 가만히 서 있을 수가 없어요!",
    "피가 거꾸로 솟고 온 신경이 폭발할 것처럼 곤두서 있습니다.",
    "아드레날린이 폭발하며 당장이라도 미친 듯이 뛰어나갈 것 같습니다!",
    // A+2 긴장 (3개)
    "위험을 감지하고 바짝 긴장하여 사방을 경계하고 있습니다.",
    "온 신경을 하나에 집중하여 팽팽한 긴장감을 유지합니다.",
    "조금의 방심도 허용하지 않으려 근육에 팽팽하게 힘을 주고 있습니다.",
    // A+1 활기 (3개)
    "충분히 자고 일어나 정신이 맑아지고 활기가 돕니다.",
    "새로운 흥미거리나 호기심이 생겨 귀가 솔깃해집니다.",
    "발걸음이 빨라지고 눈빛에 생기가 도는 기분입니다.",
];

const GEMINI_A_NEG: &[&str] = &[
    // A-1 나른함 (3개)
    "긴장이 스르르 풀리며 몸이 편안하게 이완됩니다.",
    "마음이 느긋해져서 급하게 서두를 마음이 전혀 없습니다.",
    "온몸의 근육이 이완되며 부드럽고 느긋한 감각이 퍼져나갑니다.",
    // A-2 졸음/피로 (3개)
    "눈꺼풀이 천근만근 무거워지고 강한 졸음이 쏟아집니다.",
    "기운이 쫙 빠져서 손가락 하나 까딱하기 힘들 정도로 피곤합니다.",
    "의식이 점점 흐릿해지며 꿈결 속으로 빠져드는 기분입니다.",
    // A-3 무의식/탈진 (4개)
    "의식이 완전히 끊어지고 깊고 어두운 수면에 빠져듭니다.",
    "숨소리조차 희미해질 만큼 모든 에너지가 완전히 고갈되었습니다.",
    "어떤 외부 자극에도 반응할 수 없는 완전한 무기력 상태입니다.",
    "몸의 감각이 소실되고 아무런 생각이나 움직임도 불가능합니다.",
];

// ======================================================================
// Gemini 앵커: D축 (지배/복종) — 양극 각 10문장
// ======================================================================

const GEMINI_D_POS: &[&str] = &[
    // D+3 절대 지배 (4개)
    "이 모든 상황은 내 완벽한 통제 아래 있으며 내가 지배합니다.",
    "이 세상에 나를 막거나 거역할 수 있는 것은 아무것도 없습니다!",
    "명령 한마디로 상황을 뒤집을 수 있는 절대적인 주도권을 쥐고 있습니다.",
    "누구도 감히 내 결정에 반기를 들 수 없는 무소불위의 주도권입니다.",
    // D+2 주도/자신감 (3개)
    "내가 이 모임을 주도하고 결정을 내릴 수 있다는 확신이 듭니다.",
    "누구에게도 굽히지 않고 내 의견을 당당하게 주장할 수 있습니다.",
    "다른 사람들에게 명확한 지시를 내리고 팀을 이끌어갈 수 있습니다.",
    // D+1 독립/자율 (3개)
    "스스로 문제를 해결하고 책임을 질 수 있는 능력이 충분합니다.",
    "갑작스러운 상황에도 당황하지 않고 침착하게 대처할 수 있습니다.",
    "부당한 요구나 압력에 대해서는 명확하게 거절할 수 있는 자신감이 있습니다.",
];

const GEMINI_D_NEG: &[&str] = &[
    // D-1 눈치/의존 (3개)
    "어떻게 행동해야 할지 확신이 서지 않아 주변의 눈치를 살피게 됩니다.",
    "누군가의 지시나 도움을 기다리며 스스로 나서기를 주저합니다.",
    "내 의견이 받아들여지지 않을까 걱정되어 소극적인 태도를 취합니다.",
    // D-2 무력/위축 (3개)
    "상황이나 상대의 기세에 완전히 압도당해 어찌할 바를 모르겠습니다.",
    "의견을 낼 권리조차 빼앗긴 채 그저 순응하고 복종해야만 합니다.",
    "나의 자존감과 의지가 완전히 꺾여버려 스스로를 하찮게 여기게 됩니다.",
    // D-3 완전 예속 (4개)
    "압도적인 공포에 질려 손가락 하나 까딱할 수 없고 마비된 기분입니다.",
    "내 의지나 자아는 완전히 꺾였고, 철저하게 짓밟혀 굴복했습니다.",
    "숨소리조차 내지 못할 만큼 상대방의 처분만을 기다리는 비참한 상태입니다.",
    "저항하려는 일말의 의지조차 마비되어 그저 처분만을 기다리고 있습니다.",
];

// ======================================================================
// 테스트 대사 (벤치마크와 동일)
// ======================================================================

struct TestCase {
    utterance: &'static str,
    expected_p: f32,
    expected_a: f32,
    expected_d: f32,
    label: &'static str,
}

fn test_cases() -> Vec<TestCase> {
    vec![
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
            expected_a: 0.0,
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
            expected_a: 0.0,
            expected_d: -1.0,
            label: "비탄/슬픔",
        },
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
            expected_d: 0.0,
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
            expected_a: 0.0,
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
            expected_a: 0.0,
            expected_d: 1.0,
            label: "책임감(지배)",
        },
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
            expected_a: 0.0,
            expected_d: 1.0,
            label: "경멸/조롱",
        },
        TestCase {
            utterance: "형님, 같이 술이나 한잔합시다. 오랜만이오.",
            expected_p: 1.0,
            expected_a: 0.0,
            expected_d: 0.0,
            label: "친근함",
        },
        TestCase {
            utterance: "저... 혹시 괜찮으시다면... 함께 가도 될까요?",
            expected_p: 1.0,
            expected_a: 0.0,
            expected_d: -1.0,
            label: "수줍음/소심",
        },
    ]
}

// ======================================================================
// 메인 벤치마크 — Gemini 앵커 vs 현재 앵커 비교
// ======================================================================

#[test]
fn gemini_vs_current_앵커_비교() {
    let mut embedder = shared_embedder().lock().unwrap();

    // 1) Gemini 앵커 임베딩
    println!("\n[1] Gemini 앵커 임베딩 중...");
    let g_p_pos = mean_vector(&embedder.embed(GEMINI_P_POS).unwrap());
    let g_p_neg = mean_vector(&embedder.embed(GEMINI_P_NEG).unwrap());
    let g_a_pos = mean_vector(&embedder.embed(GEMINI_A_POS).unwrap());
    let g_a_neg = mean_vector(&embedder.embed(GEMINI_A_NEG).unwrap());
    let g_d_pos = mean_vector(&embedder.embed(GEMINI_D_POS).unwrap());
    let g_d_neg = mean_vector(&embedder.embed(GEMINI_D_NEG).unwrap());

    // 2) 현재 앵커 (PadAnalyzer 사용)
    println!("[2] 현재 앵커(PadAnalyzer)로 분석기 초기화...");
    drop(embedder);
    let mut embedder2 = OrtEmbedder::new(MODEL_PATH, TOKENIZER_PATH).unwrap();
    let source = FileAnchorSource::from_content(builtin_anchor_toml("ko").unwrap(), AnchorFormat::Toml);
    let analyzer = PadAnalyzer::new(
        Box::new(OrtEmbedder::new(MODEL_PATH, TOKENIZER_PATH).unwrap()),
        &source,
    )
    .unwrap();

    // 3) 테스트 대사 임베딩
    println!("[3] 테스트 대사 임베딩 중...");
    let cases = test_cases();
    let utterances: Vec<&str> = cases.iter().map(|c| c.utterance).collect();
    let embeddings = embedder2.embed(&utterances).unwrap();

    // 4) 결과 비교
    println!("\n{}", "=".repeat(120));
    println!("Gemini 앵커(축당 10+10) vs 현재 앵커(축당 6+6) 비교");
    println!("{}", "=".repeat(120));
    println!("{:<14} {:>22}   {:>22}   {:>22}", "", "P축", "A축", "D축");
    println!(
        "{:<14} {:>10} {:>10}   {:>10} {:>10}   {:>10} {:>10}",
        "라벨", "Gemini", "현재", "Gemini", "현재", "Gemini", "현재"
    );
    println!("{}", "-".repeat(120));

    let mut g_p_ok = 0;
    let mut g_a_ok = 0;
    let mut g_d_ok = 0;
    let mut c_p_ok = 0;
    let mut c_a_ok = 0;
    let mut c_d_ok = 0;
    let mut p_total = 0;
    let mut a_total = 0;
    let mut d_total = 0;

    for (i, case) in cases.iter().enumerate() {
        let emb = &embeddings[i];

        // Gemini 앵커 점수
        let gp = axis_score(emb, &g_p_pos, &g_p_neg);
        let ga = axis_score(emb, &g_a_pos, &g_a_neg);
        let gd = axis_score(emb, &g_d_pos, &g_d_neg);

        // 현재 앵커 점수 (PadAnalyzer.to_pad)
        let cur = analyzer.to_pad(emb);
        let cp = cur.pleasure;
        let ca = cur.arousal;
        let cd = cur.dominance;

        // 방향 정확도 집계
        if case.expected_p != 0.0 {
            p_total += 1;
            if (case.expected_p > 0.0) == (gp > 0.0) {
                g_p_ok += 1;
            }
            if (case.expected_p > 0.0) == (cp > 0.0) {
                c_p_ok += 1;
            }
        }
        if case.expected_a != 0.0 {
            a_total += 1;
            if (case.expected_a > 0.0) == (ga > 0.0) {
                g_a_ok += 1;
            }
            if (case.expected_a > 0.0) == (ca > 0.0) {
                c_a_ok += 1;
            }
        }
        if case.expected_d != 0.0 {
            d_total += 1;
            if (case.expected_d > 0.0) == (gd > 0.0) {
                g_d_ok += 1;
            }
            if (case.expected_d > 0.0) == (cd > 0.0) {
                c_d_ok += 1;
            }
        }

        let gp_m = if case.expected_p != 0.0 {
            if (case.expected_p > 0.0) == (gp > 0.0) {
                "✓"
            } else {
                "✗"
            }
        } else {
            " "
        };
        let cp_m = if case.expected_p != 0.0 {
            if (case.expected_p > 0.0) == (cp > 0.0) {
                "✓"
            } else {
                "✗"
            }
        } else {
            " "
        };
        let ga_m = if case.expected_a != 0.0 {
            if (case.expected_a > 0.0) == (ga > 0.0) {
                "✓"
            } else {
                "✗"
            }
        } else {
            " "
        };
        let ca_m = if case.expected_a != 0.0 {
            if (case.expected_a > 0.0) == (ca > 0.0) {
                "✓"
            } else {
                "✗"
            }
        } else {
            " "
        };
        let gd_m = if case.expected_d != 0.0 {
            if (case.expected_d > 0.0) == (gd > 0.0) {
                "✓"
            } else {
                "✗"
            }
        } else {
            " "
        };
        let cd_m = if case.expected_d != 0.0 {
            if (case.expected_d > 0.0) == (cd > 0.0) {
                "✓"
            } else {
                "✗"
            }
        } else {
            " "
        };

        println!(
            "{:<14} {:>+8.3}{}  {:>+8.3}{}  {:>+8.3}{}  {:>+8.3}{}  {:>+8.3}{}  {:>+8.3}{}",
            case.label, gp, gp_m, cp, cp_m, ga, ga_m, ca, ca_m, gd, gd_m, cd, cd_m,
        );
    }

    println!("{}", "-".repeat(120));
    println!("방향 정확도 요약:");
    println!(
        "  P축: Gemini {}/{} ({:.0}%)  현재 {}/{} ({:.0}%)",
        g_p_ok,
        p_total,
        g_p_ok as f64 / p_total as f64 * 100.0,
        c_p_ok,
        p_total,
        c_p_ok as f64 / p_total as f64 * 100.0
    );
    println!(
        "  A축: Gemini {}/{} ({:.0}%)  현재 {}/{} ({:.0}%)",
        g_a_ok,
        a_total,
        g_a_ok as f64 / a_total as f64 * 100.0,
        c_a_ok,
        a_total,
        c_a_ok as f64 / a_total as f64 * 100.0
    );
    println!(
        "  D축: Gemini {}/{} ({:.0}%)  현재 {}/{} ({:.0}%)",
        g_d_ok,
        d_total,
        g_d_ok as f64 / d_total as f64 * 100.0,
        c_d_ok,
        d_total,
        c_d_ok as f64 / d_total as f64 * 100.0
    );
    println!(
        "  합계: Gemini {}/{} ({:.0}%)  현재 {}/{} ({:.0}%)",
        g_p_ok + g_a_ok + g_d_ok,
        p_total + a_total + d_total,
        (g_p_ok + g_a_ok + g_d_ok) as f64 / (p_total + a_total + d_total) as f64 * 100.0,
        c_p_ok + c_a_ok + c_d_ok,
        p_total + a_total + d_total,
        (c_p_ok + c_a_ok + c_d_ok) as f64 / (p_total + a_total + d_total) as f64 * 100.0
    );
}
