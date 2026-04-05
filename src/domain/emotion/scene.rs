use super::types::EmotionType;
use super::{EmotionState, Situation, SituationError};
use serde::{Deserialize, Serialize};

/// 장면(Scene) 애그리거트 루트
///
/// 하나의 대화 세션을 관리하며, 비트(Focus) 전환 로직을 캡슐화한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    /// 대화 주체 NPC ID
    npc_id: String,
    /// 대화 상대 ID
    partner_id: String,
    /// 사용 가능한 Focus 옵션 목록
    focuses: Vec<SceneFocus>,
    /// 현재 활성 Focus ID
    active_focus_id: Option<String>,
    /// 상황 중요도 — 게임이 설정하는 값 (0.0~1.0)
    /// 대화 종료 시 관계 갱신 배율에 반영됨: 1 + significance × SIGNIFICANCE_SCALE
    significance: f32,
}

impl Scene {
    pub fn new(npc_id: String, partner_id: String, focuses: Vec<SceneFocus>) -> Self {
        Self {
            npc_id,
            partner_id,
            focuses,
            active_focus_id: None,
            significance: 0.5,
        }
    }

    /// 상황 중요도를 설정하여 Scene을 생성한다.
    pub fn with_significance(
        npc_id: String,
        partner_id: String,
        focuses: Vec<SceneFocus>,
        significance: f32,
    ) -> Self {
        Self {
            npc_id,
            partner_id,
            focuses,
            active_focus_id: None,
            significance: significance.clamp(0.0, 1.0),
        }
    }

    pub fn npc_id(&self) -> &str {
        &self.npc_id
    }
    pub fn partner_id(&self) -> &str {
        &self.partner_id
    }
    pub fn focuses(&self) -> &[SceneFocus] {
        &self.focuses
    }
    pub fn active_focus_id(&self) -> Option<&str> {
        self.active_focus_id.as_deref()
    }
    pub fn significance(&self) -> f32 {
        self.significance
    }

    /// 현재 감정 상태를 기반으로 전환할 Focus를 찾습니다.
    ///
    /// 이미 활성화된 Focus는 재전환 대상에서 제외됩니다 (state latching).
    /// 이는 지배 감정이 임계값을 계속 상회할 때 매 턴 Beat 전환 이벤트가
    /// 중복 발생하는 것을 방지합니다.
    pub fn check_trigger(&self, state: &EmotionState) -> Option<&SceneFocus> {
        self.focuses.iter().find(|f| {
            // 현재 활성 Focus는 제외
            if self.active_focus_id.as_deref() == Some(f.id.as_str()) {
                return false;
            }
            f.trigger.is_met(state)
        })
    }

    /// 활성 Focus를 설정합니다.
    pub fn set_active_focus(&mut self, focus_id: String) {
        self.active_focus_id = Some(focus_id);
    }

    /// Initial Focus를 찾아 반환합니다.
    pub fn initial_focus(&self) -> Option<&SceneFocus> {
        self.focuses
            .iter()
            .find(|f| matches!(f.trigger, FocusTrigger::Initial))
    }
}

// ---------------------------------------------------------------------------
// Scene Focus — Beat 전환을 위한 Focus 옵션 (이관됨)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneFocus {
    pub id: String,
    pub description: String,
    pub trigger: FocusTrigger,
    pub event: Option<super::situation::EventFocus>,
    pub action: Option<super::situation::ActionFocus>,
    pub object: Option<super::situation::ObjectFocus>,
}

impl SceneFocus {
    pub fn to_situation(&self) -> Result<Situation, SituationError> {
        Situation::new(
            &self.description,
            self.event.clone(),
            self.action.clone(),
            self.object.clone(),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FocusTrigger {
    Initial,
    Conditions(Vec<Vec<EmotionCondition>>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionCondition {
    pub emotion: EmotionType,
    pub threshold: ConditionThreshold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionThreshold {
    Below(f32),
    Above(f32),
    Absent,
}

impl EmotionCondition {
    pub fn is_met(&self, state: &EmotionState) -> bool {
        let intensity = state.intensity_of(self.emotion);
        match self.threshold {
            ConditionThreshold::Below(v) => intensity < v,
            ConditionThreshold::Above(v) => intensity > v,
            ConditionThreshold::Absent => intensity == 0.0,
        }
    }
}

impl FocusTrigger {
    pub fn is_met(&self, state: &EmotionState) -> bool {
        match self {
            FocusTrigger::Initial => false,
            FocusTrigger::Conditions(or_groups) => or_groups
                .iter()
                .any(|and_group| and_group.iter().all(|cond| cond.is_met(state))),
        }
    }
}