use crate::domain::emotion::EmotionState;
use crate::domain::relationship::Relationship;
use super::dto::{AfterDialogueRequest, AfterDialogueResponse, RelationshipValues};

/// 관계 갱신 및 관리를 전담하는 서비스
pub struct RelationshipService;

impl RelationshipService {
    pub fn new() -> Self {
        Self
    }

    /// 대화/Beat 종료 후 관계를 갱신하고 변동 전후 값을 반환합니다.
    pub fn update_relationship(
        &self,
        relationship: &Relationship,
        emotion_state: &EmotionState,
        req: &AfterDialogueRequest,
    ) -> (Relationship, AfterDialogueResponse) {
        let before = RelationshipValues {
            closeness: relationship.closeness().value(),
            trust: relationship.trust().value(),
            power: relationship.power().value(),
        };

        let significance = req.significance.unwrap_or(0.0).clamp(0.0, 1.0);
        let new_rel = relationship.after_dialogue(emotion_state, significance);

        let after = RelationshipValues {
            closeness: new_rel.closeness().value(),
            trust: new_rel.trust().value(),
            power: new_rel.power().value(),
        };

        (new_rel, AfterDialogueResponse { before, after })
    }
}
