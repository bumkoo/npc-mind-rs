//! 실시간 상태 변경 이벤트 — broadcast 채널을 통해 SSE 클라이언트에 전달

/// 상태 변경 이벤트 종류
#[derive(Clone, Debug)]
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
        }
    }
}
