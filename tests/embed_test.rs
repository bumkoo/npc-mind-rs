//! PAD 앵커 임베딩 파이프라인 통합 테스트
//!
//! 포트 앤드 어댑터 구조 검증:
//! - OrtEmbedder (adapter) → TextEmbedder 포트
//! - PadAnalyzer (domain) → UtteranceAnalyzer 포트
//!
//! 3축(P, A, D) 모두 측정한다.
//! P·A는 pad_dot 내적에, D는 격차 스케일러에 사용된다.
//!
//! `cargo test --features embed --test embed_test -- --nocapture` 으로 실행.
//!
//! ⚠️ Windows: `CFLAGS=/MD CXXFLAGS=/MD` 를 셸에 설정해야 하며, 이전에 다른 CRT로
//! 빌드된 캐시가 있으면 `cargo clean`이 선행되어야 한다 (LNK2038 RuntimeLibrary 불일치).
//!
//! ## TODO — 실 ONNX 기반 significance 측정 추가
//!
//! 현재 이 파일은 *대사 → PAD 변환 품질*(커버리지 ①)만 검증한다.
//! `compute_significance` / `TurnSnapshot` 은 다루지 않는다.
//!
//! `phase1_real_llm_test`에서 significance 단언을 걷어냈고(그쪽은 analyzer 미부착이라
//! 구조적으로 죽은 값이었다), 배선 검증(②)은 mock analyzer 기반
//! `tests/turn_snapshot_pipeline_test.rs`가 담당하게 됐다.
//!
//! 남은 빈 칸은 **실 임베딩으로 ①→②→③을 관통했을 때 significance가 설계 의도대로
//! 나오는가**이며, 실 ONNX가 필요하므로 이 파일이 제자리다. 다만 위 빌드 비용 때문에
//! 보류 중이다. 상세: `docs/tasks/mind-architecture/reflection-test-restructure-handoff.md` §4-(4)

#![cfg(feature = "embed")]

use std::sync::{Mutex, OnceLock};

use npc_mind::adapter::ort_embedder::OrtEmbedder;
use npc_mind::adapter::file_anchor_source::{FileAnchorSource, AnchorFormat};
use npc_mind::domain::pad::PadAnalyzer;
use npc_mind::domain::pad_anchors::builtin_anchor_toml;
use npc_mind::ports::UtteranceAnalyzer;
use npc_mind::{Appraiser, StimulusProcessor};

/// 모델 경로 — 프로젝트 루트 기준 상대 경로
const MODEL_PATH: &str = "../models/bge-m3/model_quantized.onnx";
const TOKENIZER_PATH: &str = "../models/bge-m3/tokenizer.json";

/// PadAnalyzer 싱글턴 (모델 초기화가 무겁기 때문에 테스트 간 공유)
fn shared_analyzer() -> &'static Mutex<PadAnalyzer> {
    static ANALYZER: OnceLock<Mutex<PadAnalyzer>> = OnceLock::new();
    ANALYZER.get_or_init(|| {
        let embedder =
            OrtEmbedder::new(MODEL_PATH, TOKENIZER_PATH).expect("OrtEmbedder 초기화 실패");
        let source = FileAnchorSource::from_content(builtin_anchor_toml("ko").unwrap(), AnchorFormat::Toml);
        let analyzer =
            PadAnalyzer::new(Box::new(embedder), &source).expect("PadAnalyzer 앵커 계산 실패");
        Mutex::new(analyzer)
    })
}

#[test]
fn 도발_대사는_pleasure_음수() {
    let mut analyzer = shared_analyzer().lock().unwrap();
    let pad = analyzer
        .analyze("네 이놈, 죽고 싶으냐!")
        .expect("임베딩 실패");
    println!(
        "도발: P={:.3}, A={:.3}, D={:.3}",
        pad.pleasure, pad.arousal, pad.dominance
    );
    assert!(pad.pleasure < 0.0, "도발 대사의 P는 음수: {}", pad.pleasure);
}

#[test]
fn 감사_대사는_pleasure_양수() {
    let mut analyzer = shared_analyzer().lock().unwrap();
    let pad = analyzer
        .analyze("은혜를 잊지 않겠습니다. 정말 감사합니다.")
        .expect("임베딩 실패");
    println!(
        "감사: P={:.3}, A={:.3}, D={:.3}",
        pad.pleasure, pad.arousal, pad.dominance
    );
    assert!(pad.pleasure > 0.0, "감사 대사의 P는 양수: {}", pad.pleasure);
}

#[test]
fn 위협_대사는_arousal_양수() {
    let mut analyzer = shared_analyzer().lock().unwrap();
    let pad = analyzer
        .analyze("당장 목을 치겠다! 칼을 뽑아라!")
        .expect("임베딩 실패");
    println!(
        "위협: P={:.3}, A={:.3}, D={:.3}",
        pad.pleasure, pad.arousal, pad.dominance
    );
    assert!(pad.arousal > 0.0, "위협 대사의 A는 양수: {}", pad.arousal);
}

#[test]
fn 차분한_대사가_위협보다_arousal_낮음() {
    let mut analyzer = shared_analyzer().lock().unwrap();
    let calm_pad = analyzer
        .analyze("편안히 쉬시게. 차 한잔 올리지.")
        .expect("임베딩 실패");
    let threat_pad = analyzer.analyze("당장 목을 치겠다!").expect("임베딩 실패");
    println!(
        "차분: A={:.3}, 위협: A={:.3}",
        calm_pad.arousal, threat_pad.arousal
    );
    assert!(
        calm_pad.arousal < threat_pad.arousal,
        "차분({:.3}) < 위협({:.3}) arousal",
        calm_pad.arousal,
        threat_pad.arousal
    );
}

#[test]
fn 복종이_명령보다_dominance_낮음() {
    let mut analyzer = shared_analyzer().lock().unwrap();
    let submit = analyzer
        .analyze("제가 잘못했습니다. 어떤 벌이든 달게 받겠습니다.")
        .expect("임베딩 실패");
    let command = analyzer
        .analyze("내가 주도한다. 물러서라, 이것이 명이다!")
        .expect("임베딩 실패");
    println!(
        "복종: D={:.3}, 명령: D={:.3}",
        submit.dominance, command.dominance
    );
    assert!(
        submit.dominance < command.dominance,
        "복종({:.3}) < 명령({:.3}) dominance",
        submit.dominance,
        command.dominance
    );
}

#[test]
fn 전체_흐름_대사분석_후_자극_적용() {
    use npc_mind::domain::emotion::*;
    use npc_mind::domain::relationship::Relationship;

    let mut analyzer = shared_analyzer().lock().unwrap();
    let stimulus = analyzer
        .analyze("배은망덕한 놈! 의리도 없는 것이!")
        .expect("임베딩 실패");
    println!(
        "대사 PAD: P={:.3}, A={:.3}, D={:.3}",
        stimulus.pleasure, stimulus.arousal, stimulus.dominance
    );

    let yu = npc_mind::domain::personality::NpcBuilder::new("gyo", "교룡")
        .agreeableness(|a| {
            a.patience = npc_mind::domain::personality::Score::new(-0.7, "").unwrap();
        })
        .build();
    let rel = Relationship::neutral("gyo", "target");
    let situation = Situation::new(
        "배신",
        Some(EventFocus {
            description: "배신으로 인한 피해".into(),
            desirability_for_self: -0.6,
            desirability_for_other: None,
            prospect: None,
        }),
        Some(ActionFocus {
            description: "배신 행위".into(),
            agent_id: Some("partner".into()),
            praiseworthiness: -0.7,
            modifiers: None,
        }),
        None,
    )
    .unwrap();

    let initial = AppraisalEngine.appraise(yu.personality(), &situation, &rel.modifiers());
    let after = StimulusEngine.apply_stimulus(yu.personality(), &initial, &stimulus);

    let anger_before = initial
        .emotions()
        .iter()
        .find(|e| e.emotion_type() == EmotionType::Anger)
        .map(|e| e.intensity())
        .unwrap_or(0.0);
    let anger_after = after
        .emotions()
        .iter()
        .find(|e| e.emotion_type() == EmotionType::Anger)
        .map(|e| e.intensity())
        .unwrap_or(0.0);

    println!("Anger: {:.3} → {:.3}", anger_before, anger_after);
    assert!(
        anger_after >= anger_before * 0.9,
        "도발 대사 후 Anger 유지/증폭: {} → {}",
        anger_before,
        anger_after
    );
}
