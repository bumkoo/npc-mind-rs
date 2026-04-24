//! `listener_perspective` feature OFF + chat 빌드의 회귀 감시 (Phase 7 Step 5)
//!
//! `default = ["listener_perspective"]` 전환 후에도 `--no-default-features --features chat`
//! 빌드가 깨지지 않고 동작 invariant("speaker PAD가 그대로 ApplyStimulus로 dispatch")가
//! 유지되는지 검증한다.
//!
//! 가드: `not(feature = "listener_perspective")` — default 빌드(LP on)에서는 skip.
//!
//! 실행:
//! ```bash
//! cargo test --no-default-features --features chat --test dialogue_no_lp_passthrough
//! ```

#![cfg(all(feature = "chat", not(feature = "listener_perspective")))]

mod common;

use common::TestContext;
use common::mock_chat::MockConversationPort;

use std::sync::Arc;

use npc_mind::application::command::dispatcher::CommandDispatcher;
use npc_mind::application::dto::{ActionInput, EventInput, SituationInput};
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::domain::event::EventPayload;
use npc_mind::domain::pad::Pad;
use npc_mind::ports::{EmbedError, GuideFormatter, UtteranceAnalyzer};
use npc_mind::presentation::builtin_toml;
use npc_mind::presentation::formatter::LocaleFormatter;
use npc_mind::{DialogueOrchestrator, EventStore};

/// 정해진 PAD를 반환하는 mock analyzer.
///
/// `analyze_with_embedding`은 trait default(`(pad, None)`)를 그대로 사용 —
/// LP off에서는 임베딩이 없어도 변환 단계가 우회되어 speaker PAD가 보존된다.
struct ScriptedAnalyzer {
    pad: Pad,
}

impl UtteranceAnalyzer for ScriptedAnalyzer {
    fn analyze(&mut self, _utterance: &str) -> Result<Pad, EmbedError> {
        Ok(self.pad)
    }
}

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

/// LP off 빌드에서 `convert_to_listener_pad`의 not-feature impl이
/// 컴파일 + 동작하여 speaker PAD를 그대로 ApplyStimulus에 dispatch한다.
#[tokio::test]
async fn dialogue_orchestrator_passes_speaker_pad_through_when_lp_off() {
    let ctx = TestContext::new();
    let (dispatcher, store, _bus) = common::v2_dispatcher_with_defaults(ctx.repo);

    let toml = builtin_toml("ko").expect("ko locale");
    let formatter: Arc<dyn GuideFormatter> =
        Arc::new(LocaleFormatter::from_toml(toml).expect("formatter"));

    let chat = MockConversationPort::new();

    // LP off 빌드에서는 `with_converter` 메서드 자체가 컴파일에서 제외 →
    // analyzer만 주입된 상태에서 변환 우회 동작이 invariant.
    let mut agent = DialogueOrchestrator::new(dispatcher, chat, formatter)
        .with_analyzer(ScriptedAnalyzer {
            pad: Pad::new(0.6, 0.3, 0.1),
        });

    agent
        .start_session("s1", "mu_baek", "gyo_ryong", Some(betrayal_situation()))
        .await
        .unwrap();
    agent.turn("s1", "오랜만이군.", None, None).await.unwrap();

    let pad = store
        .get_all_events()
        .into_iter()
        .find_map(|e| {
            if let EventPayload::StimulusApplied { pad, .. } = e.payload {
                Some(pad)
            } else {
                None
            }
        })
        .expect("StimulusApplied 이벤트 발행");

    assert!(
        (pad.0 - 0.6).abs() < 1e-5,
        "LP off에서 speaker pleasure 통과 (실제={})",
        pad.0
    );
    assert!((pad.1 - 0.3).abs() < 1e-5, "LP off에서 speaker arousal 통과");
    assert!(
        (pad.2 - 0.1).abs() < 1e-5,
        "LP off에서 speaker dominance 통과"
    );
}
