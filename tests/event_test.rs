//! EventAwareMindService 통합 테스트
//!
//! 기존 TestContext를 재사용하여 이벤트 발행 · 저장 · Projection을 검증합니다.

mod common;

use common::TestContext;
use npc_mind::application::dto::*;
use npc_mind::application::event_service::EventAwareMindService;
use npc_mind::application::event_store::InMemoryEventStore;
use npc_mind::application::event_bus::EventBus;
use npc_mind::application::mind_service::MindService;
use npc_mind::application::projection::{EmotionProjection, Projection, RelationshipProjection};
use npc_mind::domain::event::{DomainEvent, EventPayload};
use npc_mind::{EventStore, InMemoryRepository};

use std::sync::{Arc, RwLock};

/// 테스트용 공유 Projection 래퍼 — registry에 등록하면서 외부에서도 읽도록 함
struct Shared<P: Projection + Send + Sync>(Arc<RwLock<P>>);

impl<P: Projection + Send + Sync> Projection for Shared<P> {
    fn apply(&mut self, event: &DomainEvent) {
        self.0.write().unwrap().apply(event);
    }
}

fn make_service(
    repo: &mut InMemoryRepository,
) -> (
    EventAwareMindService<&mut InMemoryRepository, npc_mind::domain::emotion::AppraisalEngine, npc_mind::domain::emotion::StimulusEngine>,
    Arc<InMemoryEventStore>,
) {
    let inner = MindService::new(repo);
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());
    let service = EventAwareMindService::new(inner, store.clone(), bus);
    (service, store)
}

fn appraise_req() -> AppraiseRequest {
    AppraiseRequest {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        situation: Some(SituationInput {
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
        }),
    }
}

fn stimulus_req() -> StimulusRequest {
    StimulusRequest {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        situation_description: None,
        pleasure: 0.5,
        arousal: 0.3,
        dominance: 0.2,
    }
}

// ---------------------------------------------------------------------------
// 테스트
// ---------------------------------------------------------------------------

#[test]
fn appraise_emits_emotion_appraised() {
    let mut ctx = TestContext::new();
    let (mut service, store) = make_service(&mut ctx.repo);

    service.appraise(appraise_req(), || {}, Vec::new).unwrap();

    let events = store.get_all_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].payload, EventPayload::EmotionAppraised { .. }));
    assert_eq!(events[0].aggregate_id, "mu_baek");
    assert_eq!(events[0].sequence, 1);
}

#[test]
fn stimulus_emits_stimulus_applied() {
    let mut ctx = TestContext::new();
    let (mut service, store) = make_service(&mut ctx.repo);

    // appraise 먼저 (감정 상태 필요)
    service.appraise(appraise_req(), || {}, Vec::new).unwrap();

    // stimulus
    service.apply_stimulus(stimulus_req(), || {}, Vec::new).unwrap();

    let events = store.get_all_events();
    assert_eq!(events.len(), 2); // EmotionAppraised + StimulusApplied
    assert!(matches!(events[1].payload, EventPayload::StimulusApplied { .. }));

    if let EventPayload::StimulusApplied { beat_changed, .. } = &events[1].payload {
        assert!(!beat_changed, "Beat 전환 없이 StimulusApplied");
    }
}

#[test]
fn after_dialogue_emits_three_events() {
    let mut ctx = TestContext::new();
    let (mut service, store) = make_service(&mut ctx.repo);

    // appraise (감정 상태 생성)
    service.appraise(appraise_req(), || {}, Vec::new).unwrap();

    // after_dialogue
    let req = AfterDialogueRequest {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        significance: Some(0.5),
    };
    service.after_dialogue(req).unwrap();

    let events = store.get_all_events();
    // EmotionAppraised + RelationshipUpdated + EmotionCleared + SceneEnded
    assert_eq!(events.len(), 4);
    assert!(matches!(events[1].payload, EventPayload::RelationshipUpdated { .. }));
    assert!(matches!(events[2].payload, EventPayload::EmotionCleared { .. }));
    assert!(matches!(events[3].payload, EventPayload::SceneEnded { .. }));
}

#[test]
fn start_scene_emits_scene_started_and_appraised() {
    let mut ctx = TestContext::new();
    let (mut service, store) = make_service(&mut ctx.repo);

    let req = SceneRequest {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        description: "테스트 씬".into(),
        significance: Some(0.5),
        focuses: vec![SceneFocusInput {
            id: "focus_initial".into(),
            description: "초기 상황".into(),
            trigger: None, // Initial
            event: Some(EventInput {
                description: "사건".into(),
                desirability_for_self: -0.3,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
            test_script: vec![],
        }],
    };

    service.start_scene(req, || {}, Vec::new).unwrap();

    let events = store.get_all_events();
    assert!(events.len() >= 2, "SceneStarted + EmotionAppraised 최소 2개");
    assert!(matches!(events[0].payload, EventPayload::SceneStarted { .. }));
    assert!(matches!(events[1].payload, EventPayload::EmotionAppraised { .. }));
}

#[test]
fn generate_guide_emits_guide_generated() {
    let mut ctx = TestContext::new();
    let (mut service, store) = make_service(&mut ctx.repo);

    // appraise 먼저
    service.appraise(appraise_req(), || {}, Vec::new).unwrap();

    let req = GuideRequest {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        situation_description: None,
    };
    service.generate_guide(req).unwrap();

    let events = store.get_all_events();
    assert_eq!(events.len(), 2); // EmotionAppraised + GuideGenerated
    assert!(matches!(events[1].payload, EventPayload::GuideGenerated { .. }));
}

#[test]
fn event_store_filters_by_aggregate() {
    let mut ctx = TestContext::new();
    // 두 번째 NPC 추가
    ctx.repo.add_npc(common::make_수련());
    ctx.repo.add_relationship(
        npc_mind::domain::relationship::Relationship::neutral("shu_lien", "gyo_ryong"),
    );

    let (mut service, store) = make_service(&mut ctx.repo);

    // mu_baek appraise
    service.appraise(appraise_req(), || {}, Vec::new).unwrap();

    // shu_lien appraise
    let req2 = AppraiseRequest {
        npc_id: "shu_lien".into(),
        partner_id: "gyo_ryong".into(),
        situation: Some(SituationInput {
            description: "다른 상황".into(),
            event: Some(EventInput {
                description: "좋은 사건".into(),
                desirability_for_self: 0.5,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        }),
    };
    service.appraise(req2, || {}, Vec::new).unwrap();

    // 전체 2건
    assert_eq!(store.get_all_events().len(), 2);
    // mu_baek만 1건
    assert_eq!(store.get_events("mu_baek").len(), 1);
    // shu_lien만 1건
    assert_eq!(store.get_events("shu_lien").len(), 1);
}

#[test]
fn projection_updates_on_events() {
    let mut ctx = TestContext::new();
    let inner = MindService::new(&mut ctx.repo);
    let store: Arc<dyn npc_mind::EventStore> = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new());

    // Projection 준비 (공유 핸들)
    let emotion_proj = Arc::new(RwLock::new(EmotionProjection::new()));
    let rel_proj = Arc::new(RwLock::new(RelationshipProjection::new()));

    let service = EventAwareMindService::new(inner, store, bus);
    // L1 Projection 등록 — emit 경로에서 동기 호출됨
    service.register_projection(Shared(emotion_proj.clone()));
    service.register_projection(Shared(rel_proj.clone()));

    let mut service = service;

    // appraise
    service.appraise(appraise_req(), || {}, Vec::new).unwrap();

    // EmotionProjection에 mood가 반영되었는지 — dispatch 직후에도 최신 (쿼리 일관성)
    let ep = emotion_proj.read().unwrap();
    assert!(ep.get_mood("mu_baek").is_some());

    // after_dialogue
    drop(ep);
    let after_req = AfterDialogueRequest {
        npc_id: "mu_baek".into(),
        partner_id: "gyo_ryong".into(),
        significance: Some(0.5),
    };
    service.after_dialogue(after_req).unwrap();

    // RelationshipProjection에 값이 반영되었는지
    let rp = rel_proj.read().unwrap();
    assert!(rp.get_values("mu_baek", "gyo_ryong").is_some());

    // EmotionProjection에서 cleared
    let ep = emotion_proj.read().unwrap();
    assert!(ep.get_mood("mu_baek").is_none(), "EmotionCleared 후 mood 제거");
}

#[test]
fn wrapper_returns_identical_results() {
    // 같은 시나리오를 MindService와 EventAwareMindService로 실행하여 결과 비교
    let mut ctx1 = TestContext::new();
    let mut ctx2 = TestContext::new();

    // 직접 MindService
    let mut direct = MindService::new(&mut ctx1.repo);
    let direct_result = direct.appraise(appraise_req(), || {}, Vec::new).unwrap();

    // EventAwareMindService
    let (mut wrapped, _store) = make_service(&mut ctx2.repo);
    let wrapped_result = wrapped.appraise(appraise_req(), || {}, Vec::new).unwrap();

    // 핵심 필드 비교
    assert_eq!(direct_result.mood, wrapped_result.mood);
    assert_eq!(direct_result.emotions.len(), wrapped_result.emotions.len());
    assert_eq!(
        direct_result.dominant.as_ref().map(|d| &d.emotion_type),
        wrapped_result.dominant.as_ref().map(|d| &d.emotion_type),
    );
}
