//! Memory Step B — DialogueAgent 기억 주입 통합 테스트
//!
//! `DialogueAgent::with_memory(store, framer)`로 기억 시스템을 주입했을 때:
//! - `start_session`이 시스템 프롬프트에 "떠오르는 기억" 블록을 prepend하는지
//! - `with_memory` 미호출 시 기존 프롬프트 그대로 전달되는지
//! - `BeatTransitioned` 시 `update_system_prompt` 프롬프트에도 기억 블록이 포함되는지

#![cfg(feature = "chat")]

mod common;

use common::mock_chat::{ChatCall, MockConversationPort};
use common::TestContext;

use npc_mind::application::dto::{
    ConditionInput, EventInput, SceneFocusInput, SituationInput,
};
use npc_mind::domain::memory::{MemoryEntry, MemorySource, MemoryType};
use npc_mind::domain::pad::Pad;
use npc_mind::ports::{GuideFormatter, MemoryStore};
use npc_mind::presentation::builtin_toml;
use npc_mind::presentation::formatter::LocaleFormatter;
use npc_mind::presentation::memory_formatter::LocaleMemoryFramer;
use npc_mind::{DialogueAgent, InMemoryRepository};

use std::sync::Arc;

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

fn simple_situation() -> SituationInput {
    SituationInput {
        description: "교룡과 다시 마주쳤다".into(),
        event: Some(EventInput {
            description: "예기치 못한 재회".into(),
            desirability_for_self: 0.1,
            other: None,
            prospect: None,
        }),
        action: None,
        object: None,
    }
}

fn seed_store_with_memories(
    store: &common::in_memory_store::InMemoryMemoryStore,
    npc_id: &str,
) {
    // 서로 다른 Source 두 건을 시드 — Experienced + Heard. Ranker 1단계에서
    // topic이 없으므로 둘 다 살아남고, 2단계 점수에서 vec_similarity=1.0(키워드 경로)로
    // 동률 근방이다.
    let mut e1 = MemoryEntry::personal(
        "mem-seed-001",
        npc_id,
        "예전에 사부님께 검법의 진수를 전수받았다",
        None,
        100,
        1,
        MemoryType::DialogueTurn,
    );
    e1.source = MemorySource::Experienced;
    store.index(e1, None).unwrap();

    let mut e2 = MemoryEntry::personal(
        "mem-seed-002",
        npc_id,
        "교룡이 무림맹에 가담했다는 말을 들었다",
        None,
        200,
        2,
        MemoryType::DialogueTurn,
    );
    e2.source = MemorySource::Heard;
    store.index(e2, None).unwrap();
}

fn setup_with_memory(
    attach_memory: bool,
) -> (
    DialogueAgent<InMemoryRepository, MockConversationPort>,
    Arc<std::sync::Mutex<Vec<ChatCall>>>,
    Arc<common::in_memory_store::InMemoryMemoryStore>,
) {
    let ctx = TestContext::new();
    let (dispatcher, _store, _bus) = common::v2_dispatcher_with_defaults(ctx.repo);

    let toml = builtin_toml("ko").expect("ko locale");
    let formatter: Arc<dyn GuideFormatter> =
        Arc::new(LocaleFormatter::from_toml(toml).expect("formatter"));

    let chat = MockConversationPort::new().with_response("mock", None);
    let calls = chat.calls.clone();

    let memory_store = Arc::new(common::in_memory_store::InMemoryMemoryStore::new());
    seed_store_with_memories(&*memory_store, "mu_baek");

    let mut agent = DialogueAgent::new(dispatcher, chat, formatter);
    if attach_memory {
        let framer: Arc<dyn npc_mind::ports::MemoryFramer> = Arc::new(LocaleMemoryFramer::new());
        agent = agent.with_memory(memory_store.clone(), framer);
    }
    (agent, calls, memory_store)
}

// ---------------------------------------------------------------------------
// 1. start_session prepends memory block
// ---------------------------------------------------------------------------

#[tokio::test]
async fn injection_prepends_memory_block_in_start_session() {
    let (mut agent, calls, _store) = setup_with_memory(true);

    agent
        .start_session(
            "session-mem-1",
            "mu_baek",
            "gyo_ryong",
            Some(simple_situation()),
        )
        .await
        .expect("start_session ok");

    let calls = calls.lock().unwrap();
    let Some(ChatCall::StartSession { prompt, .. }) = calls.first() else {
        panic!("첫 호출은 StartSession이어야 함");
    };

    assert!(
        prompt.contains("떠오르는 기억"),
        "memory block header가 프롬프트에 포함되어야 함\n실제: {prompt}"
    );
    // 시드한 두 기억 중 적어도 하나는 프롬프트에 포함되어야 함.
    let has_any_seed = prompt.contains("사부님")
        || prompt.contains("검법")
        || prompt.contains("교룡이 무림맹")
        || prompt.contains("가담했다");
    assert!(
        has_any_seed,
        "시드한 기억 content 중 하나는 프롬프트에 포함되어야 함\n실제: {prompt}"
    );
    // Source 라벨도 포함 — Experienced 또는 Heard 중 하나.
    let has_any_label = prompt.contains("[겪음]") || prompt.contains("[전해 들음]");
    assert!(has_any_label, "Source 라벨이 포함되어야 함\n실제: {prompt}");
}

// ---------------------------------------------------------------------------
// 2. with_memory 미호출 시 기존 프롬프트 그대로
// ---------------------------------------------------------------------------

#[tokio::test]
async fn injection_without_store_is_noop() {
    let (mut agent, calls, _store) = setup_with_memory(false);

    agent
        .start_session(
            "session-mem-2",
            "mu_baek",
            "gyo_ryong",
            Some(simple_situation()),
        )
        .await
        .expect("start_session ok");

    let calls = calls.lock().unwrap();
    let Some(ChatCall::StartSession { prompt, .. }) = calls.first() else {
        panic!("첫 호출은 StartSession이어야 함");
    };

    assert!(
        !prompt.contains("떠오르는 기억"),
        "memory block header가 포함되면 안 됨 (with_memory 미호출)\n실제: {prompt}"
    );
}

// ---------------------------------------------------------------------------
// 3. Beat 전환 시 update_system_prompt 프롬프트에도 memory block
// ---------------------------------------------------------------------------

fn scene_with_beat_trigger() -> Vec<SceneFocusInput> {
    // 최소한의 2-beat scene:
    //   - calm: Initial (trigger=None) — 약한 Joy 생성 후 쉽게 소멸하도록 desirability=0.05
    //   - angry: Joy가 사라지면(absent) 전환
    vec![
        SceneFocusInput {
            id: "calm".into(),
            description: "평온한 재회".into(),
            trigger: None,
            event: Some(EventInput {
                description: "초기 상황".into(),
                desirability_for_self: 0.05,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
            test_script: vec![],
        },
        SceneFocusInput {
            id: "angry".into(),
            description: "배신 드러남".into(),
            trigger: Some(vec![vec![ConditionInput {
                emotion: "Joy".into(),
                absent: Some(true),
                below: None,
                above: None,
            }]]),
            event: Some(EventInput {
                description: "모욕을 당함".into(),
                desirability_for_self: -0.6,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
            test_script: vec![],
        },
    ]
}

#[tokio::test]
async fn injection_reapplied_on_beat_transition() {
    use npc_mind::application::command::types::Command;

    let (mut agent, calls, _store) = setup_with_memory(true);

    // Scene 시작 (focuses 포함)
    let _ = agent
        .dispatcher()
        .dispatch_v2(Command::StartScene {
            npc_id: "mu_baek".into(),
            partner_id: "gyo_ryong".into(),
            significance: None,
            focuses: scene_with_beat_trigger(),
        })
        .await
        .expect("StartScene ok");

    agent
        .start_session("session-mem-3", "mu_baek", "gyo_ryong", None)
        .await
        .expect("start_session ok");

    // 강한 부정 PAD로 자극 → betrayal focus trigger 충족 예상
    let turn_out = agent
        .turn(
            "session-mem-3",
            "배신자야! 네놈이 무림맹을 배신했다는 건 사실이냐?",
            Some(Pad {
                pleasure: -0.9,
                arousal: 0.8,
                dominance: 0.3,
            }),
            None,
        )
        .await
        .expect("turn ok");

    // 시나리오는 Joy absent + 강한 부정 PAD로 "angry" focus가 반드시 트리거되도록
    // 설계되었다. beat_changed=false면 튜닝 변동 / 엔진 회귀이므로 실패시켜 가시화한다.
    assert!(
        turn_out.beat_changed,
        "테스트 시나리오는 Beat 전환을 보장해야 함 (강한 부정 PAD → Joy absent 트리거). \
         엔진 튜닝 변동으로 더 이상 트리거되지 않으면 시나리오를 조정해야 함."
    );

    // Beat 전환 시 UpdateSystemPrompt가 호출됐는지 확인
    let calls = calls.lock().unwrap();
    let update_prompt = calls.iter().find_map(|c| match c {
        ChatCall::UpdateSystemPrompt { new_prompt, .. } => Some(new_prompt.clone()),
        _ => None,
    });
    let update_prompt = update_prompt.expect("beat 전환 후 UpdateSystemPrompt 호출 없음");
    assert!(
        update_prompt.contains("떠오르는 기억"),
        "Beat 전환 후 시스템 프롬프트에도 memory block이 포함되어야 함\n실제: {update_prompt}"
    );
}
