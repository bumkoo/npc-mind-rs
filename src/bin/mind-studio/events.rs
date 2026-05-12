//! 실시간 상태 변경 이벤트 — broadcast 채널을 통해 SSE 클라이언트에 전달

/// 상태 변경 이벤트 종류
///
/// `PartialEq`/`Eq`는 Phase 1.6 `event_bridge::map_event` 단위 테스트가 매핑 결정론을
/// 검증하기 위해 필요. variant들은 모두 unit-like(payload 없음)라 derive 가능.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateEvent {
    // 엔티티 CRUD
    NpcChanged,
    RelationshipChanged,
    ObjectChanged,

    // 파이프라인 액션
    Appraised,
    StimulusApplied,
    AfterDialogue,
    GuideGenerated,

    // Scene
    SceneStarted,
    SceneInfoChanged,

    // 시나리오 라이프사이클
    ScenarioLoaded,
    ResultLoaded,
    ScenarioSaved,

    // 개별 필드
    SituationChanged,
    TestReportChanged,

    // 대화
    ChatStarted,
    ChatTurnCompleted,
    ChatEnded,

    // 히스토리 (catch-all)
    HistoryChanged,

    // Step E1 — Memory / Rumor (embed feature 경로에서만 발생)
    MemoryCreated,
    MemorySuperseded,
    /// Memory Step F에서 SceneConsolidationHandler가 발행할 예정(현재는 미발행 placeholder).
    /// SSE 클라이언트 측 핸들러 사전 배선을 위해 미리 선언만 둔다.
    #[allow(dead_code)]
    MemoryConsolidated,
    RumorSeeded,
    RumorSpread,

    // Phase 1 Mind Architecture (relationships.md v0.7 §6) — Reflection 박제 SSE.
    // Phase 1.5 frontend ReflectionPanel 사전 배선용. 현재는 EventBus 구독자가 자동
    // 발행하지 않음 (선언만) — Stage 4 narrative validation 시 프런트 갱신 검토.
    #[allow(dead_code)]
    DialogueReflected,
}

impl StateEvent {
    /// SSE event name (snake_case)
    pub fn name(&self) -> &'static str {
        match self {
            Self::NpcChanged => "npc_changed",
            Self::RelationshipChanged => "relationship_changed",
            Self::ObjectChanged => "object_changed",
            Self::Appraised => "appraised",
            Self::StimulusApplied => "stimulus_applied",
            Self::AfterDialogue => "after_dialogue",
            Self::GuideGenerated => "guide_generated",
            Self::SceneStarted => "scene_started",
            Self::SceneInfoChanged => "scene_info_changed",
            Self::ScenarioLoaded => "scenario_loaded",
            Self::ResultLoaded => "result_loaded",
            Self::ScenarioSaved => "scenario_saved",
            Self::SituationChanged => "situation_changed",
            Self::TestReportChanged => "test_report_changed",
            Self::ChatStarted => "chat_started",
            Self::ChatTurnCompleted => "chat_turn_completed",
            Self::ChatEnded => "chat_ended",
            Self::HistoryChanged => "history_changed",
            Self::MemoryCreated => "memory_created",
            Self::MemorySuperseded => "memory_superseded",
            Self::MemoryConsolidated => "memory_consolidated",
            Self::RumorSeeded => "rumor_seeded",
            Self::RumorSpread => "rumor_spread",
            Self::DialogueReflected => "dialogue_reflected",
        }
    }
}
