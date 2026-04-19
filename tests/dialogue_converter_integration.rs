//! DialogueAgent ↔ ListenerPerspectiveConverter 통합 (Phase 7 Step 4)
//!
//! `DialogueAgent.turn()`이 화자 PAD를 청자 PAD로 올바르게 변환하여
//! `Command::ApplyStimulus`로 dispatch하는지 검증한다.
//!
//! Mock 기반이라 임베딩 모델/실제 Converter 없이 변환 경로의 분기만 점검한다.
//! 실제 분류기 정확도는 `tests/listener_perspective_integration_bench.rs`에서 검증.

#![cfg(all(feature = "chat", feature = "listener_perspective"))]

mod common;

use common::TestContext;
use common::mock_chat::MockConversationPort;

use std::sync::Arc;

use npc_mind::application::command::dispatcher::CommandDispatcher;
use npc_mind::application::dto::{
    ActionInput, EventInput, SituationInput,
};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::event::EventPayload;
use npc_mind::domain::listener_perspective::{
    ConvertMeta, ConvertPath, ConvertResult, ListenerPerspectiveConverter,
    ListenerPerspectiveError, Magnitude, Sign,
};
use npc_mind::domain::pad::Pad;
use npc_mind::ports::{EmbedError, GuideFormatter, UtteranceAnalyzer};
use npc_mind::presentation::builtin_toml;
use npc_mind::presentation::formatter::LocaleFormatter;
use npc_mind::{DialogueAgent, EventStore, InMemoryRepository};

// ============================================================
// Mock — 발화당 임베딩 1회 + PAD 반환
// ============================================================

/// 정해진 PAD와 임베딩을 반환하는 분석기.
///
/// `embedding`이 `Some`이면 `analyze_with_embedding`이 그 벡터를 함께 반환.
/// `None`이면 임베딩 없는 분석기(예: 다른 trait 구현체)를 모사.
struct ScriptedAnalyzer {
    pad: Pad,
    embedding: Option<Vec<f32>>,
}

impl UtteranceAnalyzer for ScriptedAnalyzer {
    fn analyze(&mut self, _utterance: &str) -> Result<Pad, EmbedError> {
        Ok(self.pad)
    }

    fn analyze_with_embedding(
        &mut self,
        _utterance: &str,
    ) -> Result<(Pad, Option<Vec<f32>>), EmbedError> {
        Ok((self.pad, self.embedding.clone()))
    }
}

// ============================================================
// Mock — Converter 구현 두 종
// ============================================================

/// 화자 pleasure를 부호 반전. arousal·dominance는 유지.
struct InvertingConverter;

impl ListenerPerspectiveConverter for InvertingConverter {
    fn convert(
        &self,
        _utterance: &str,
        speaker_pad: &Pad,
        _utterance_embedding: &[f32],
    ) -> Result<ConvertResult, ListenerPerspectiveError> {
        Ok(ConvertResult {
            listener_pad: Pad::new(
                -speaker_pad.pleasure,
                speaker_pad.arousal,
                speaker_pad.dominance,
            ),
            meta: ConvertMeta {
                path: ConvertPath::Classifier {
                    sign_margin: 0.5,
                    magnitude_margin: 0.3,
                },
                sign: Sign::Invert,
                magnitude: Magnitude::Normal,
                applied_p_coef: -1.0,
                applied_a_coef: 1.0,
                applied_d_coef: 1.0,
            },
        })
    }
}

/// 항상 실패하는 Converter — fallback 경로 검증용.
struct FailingConverter;

impl ListenerPerspectiveConverter for FailingConverter {
    fn convert(
        &self,
        _utterance: &str,
        _speaker_pad: &Pad,
        _utterance_embedding: &[f32],
    ) -> Result<ConvertResult, ListenerPerspectiveError> {
        Err(ListenerPerspectiveError::Embed(
            "intentional failure for fallback test".to_string(),
        ))
    }
}

// ============================================================
// 셋업 헬퍼
// ============================================================

fn betrayal_situation() -> SituationInput {
    SituationInput {
        description: "배신 상황".into(),
        event: Some(EventInput {
            description: "사건".into(),
            desirability_for_self: -0.6,
            other: None,
            prospect: None,
        }),
        action: Some(ActionInput {
            description: "행위".into(),
            agent_id: Some("gyo_ryong".into()),
            praiseworthiness: -0.7,
        }),
        object: None,
    }
}

fn make_agent_base() -> (
    DialogueAgent<InMemoryRepository, MockConversationPort>,
    Arc<InMemoryEventStore>,
) {
    let ctx = TestContext::new();
    let store: Arc<InMemoryEventStore> = Arc::new(InMemoryEventStore::new());
    let store_dyn: Arc<dyn EventStore> = store.clone();
    let bus = Arc::new(EventBus::new());
    let dispatcher = CommandDispatcher::new(ctx.repo, store_dyn, bus);

    let toml = builtin_toml("ko").expect("ko locale");
    let formatter: Arc<dyn GuideFormatter> =
        Arc::new(LocaleFormatter::from_toml(toml).expect("formatter"));

    let chat = MockConversationPort::new();
    let agent = DialogueAgent::new(dispatcher, chat, formatter);
    (agent, store)
}

/// EventStore에서 첫 StimulusApplied 이벤트의 PAD 튜플 추출
fn first_stimulus_pad(store: &InMemoryEventStore) -> Option<(f32, f32, f32)> {
    store.get_all_events().into_iter().find_map(|e| {
        if let EventPayload::StimulusApplied { pad, .. } = e.payload {
            Some(pad)
        } else {
            None
        }
    })
}

// ============================================================
// 시나리오 (a): Converter 주입 + analyzer 임베딩 → 변환된 listener PAD
// ============================================================

#[tokio::test]
async fn converter_with_analyzer_embedding_produces_listener_pad() {
    let (agent, store) = make_agent_base();
    let mut agent = agent
        .with_analyzer(ScriptedAnalyzer {
            pad: Pad::new(0.6, 0.3, 0.1),
            embedding: Some(vec![1.0, 2.0, 3.0]),
        })
        .with_converter(Arc::new(InvertingConverter));

    agent
        .start_session("s1", "mu_baek", "gyo_ryong", Some(betrayal_situation()))
        .await
        .unwrap();
    agent.turn("s1", "오랜만이군.", None, None).await.unwrap();

    let pad = first_stimulus_pad(&store).expect("StimulusApplied 이벤트 발행");
    // InvertingConverter: pleasure 부호 반전
    assert!(
        (pad.0 - (-0.6)).abs() < 1e-5,
        "pleasure 변환되어야 함 (speaker +0.6 → listener -0.6), 실제={}",
        pad.0
    );
    assert!((pad.1 - 0.3).abs() < 1e-5, "arousal 유지");
    assert!((pad.2 - 0.1).abs() < 1e-5, "dominance 유지");
}

// ============================================================
// 시나리오 (b): Converter 미주입 → speaker PAD 그대로
// ============================================================

#[tokio::test]
async fn no_converter_falls_through_to_speaker_pad() {
    let (agent, store) = make_agent_base();
    let mut agent = agent.with_analyzer(ScriptedAnalyzer {
        pad: Pad::new(0.6, 0.3, 0.1),
        embedding: Some(vec![1.0, 2.0, 3.0]),
    });

    agent
        .start_session("s1", "mu_baek", "gyo_ryong", Some(betrayal_situation()))
        .await
        .unwrap();
    agent.turn("s1", "오랜만이군.", None, None).await.unwrap();

    let pad = first_stimulus_pad(&store).expect("StimulusApplied 이벤트 발행");
    assert!((pad.0 - 0.6).abs() < 1e-5, "pleasure 그대로");
    assert!((pad.1 - 0.3).abs() < 1e-5, "arousal 그대로");
    assert!((pad.2 - 0.1).abs() < 1e-5, "dominance 그대로");
}

// ============================================================
// 시나리오 (c): pad_hint 사용 → 임베딩 부재로 변환 skip
// ============================================================

#[tokio::test]
async fn pad_hint_skips_conversion_due_to_missing_embedding() {
    let (agent, store) = make_agent_base();
    let mut agent = agent
        .with_analyzer(ScriptedAnalyzer {
            pad: Pad::new(0.0, 0.0, 0.0), // analyzer는 호출되지 않아야 함
            embedding: Some(vec![1.0, 2.0, 3.0]),
        })
        .with_converter(Arc::new(InvertingConverter));

    agent
        .start_session("s1", "mu_baek", "gyo_ryong", Some(betrayal_situation()))
        .await
        .unwrap();
    agent
        .turn("s1", "오랜만이군.", Some(Pad::new(0.6, 0.3, 0.1)), None)
        .await
        .unwrap();

    let pad = first_stimulus_pad(&store).expect("StimulusApplied 이벤트 발행");
    // pad_hint는 임베딩 없으므로 InvertingConverter가 호출되지 않아 변환 skip
    assert!(
        (pad.0 - 0.6).abs() < 1e-5,
        "pad_hint 그대로 (변환 미발동), 실제={}",
        pad.0
    );
}

// ============================================================
// 시나리오 (d): Converter 실패 → speaker PAD fallback
// ============================================================

#[tokio::test]
async fn converter_failure_falls_back_to_speaker_pad() {
    let (agent, store) = make_agent_base();
    let mut agent = agent
        .with_analyzer(ScriptedAnalyzer {
            pad: Pad::new(0.6, 0.3, 0.1),
            embedding: Some(vec![1.0, 2.0, 3.0]),
        })
        .with_converter(Arc::new(FailingConverter));

    agent
        .start_session("s1", "mu_baek", "gyo_ryong", Some(betrayal_situation()))
        .await
        .unwrap();
    // 실패해도 turn 자체는 성공해야 함 (fallback)
    agent.turn("s1", "오랜만이군.", None, None).await.unwrap();

    let pad = first_stimulus_pad(&store).expect("StimulusApplied 이벤트 발행");
    assert!(
        (pad.0 - 0.6).abs() < 1e-5,
        "변환 실패 시 speaker PAD fallback, 실제={}",
        pad.0
    );
}
