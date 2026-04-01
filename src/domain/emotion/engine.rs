//! 감정 평가 엔진 (Appraisal Engine)
//!
//! 세부 평가는 `appraisal/` 하위 모듈에서 처리한다.

use crate::domain::emotion::situation::{RelationshipModifiers, Situation};
use crate::domain::emotion::types::EmotionState;
use crate::ports::AppraisalWeights;

/// 도메인 서비스 — 성격(AppraisalWeights) × RelationshipModifiers → OCC 감정
///
/// Zero-sized type. `Appraiser` 트레이트를 구현하며,
/// `MindService`에 기본 감정 평가 엔진으로 주입됩니다.
pub struct AppraisalEngine;

impl crate::ports::Appraiser for AppraisalEngine {
    fn appraise<P: AppraisalWeights>(
        &self,
        personality: &P,
        situation: &Situation,
        dialogue_modifiers: &RelationshipModifiers,
    ) -> EmotionState {
        super::appraisal::process(personality, situation, dialogue_modifiers)
    }
}
