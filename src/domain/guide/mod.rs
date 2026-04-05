//! LLM 연기 가이드 생성기
//!
//! NPC의 성격(HEXACO) + 현재 감정(OCC EmotionState)을 조합하여
//! LLM이 해당 NPC를 연기할 수 있는 구조화된 가이드를 생성한다.
//!
//! 이 모듈의 출력이 NPC 심리 엔진의 최종 산출물이다.
//! 텍스트/JSON 등 구체적 포맷 변환은 presentation 레이어(GuideFormatter)가 담당한다.

mod directive;
mod enums;
mod snapshot;

pub use directive::*;
pub use enums::*;
pub use snapshot::*;

use serde::{Deserialize, Serialize};

use super::emotion::EmotionState;
use super::personality::Npc;
use super::relationship::Relationship;
use super::tuning::{
    EMOTION_THRESHOLD, HONESTY_RESTRICTION_THRESHOLD, MOOD_THRESHOLD, TRAIT_THRESHOLD,
};

// ---------------------------------------------------------------------------
// LLM 연기 가이드 (최종 산출물)
// ---------------------------------------------------------------------------

/// NPC 심리 엔진의 최종 산출물: LLM이 NPC를 연기하기 위한 구조화된 가이드
///
/// 텍스트/JSON 등 구체적 포맷 변환은 `GuideFormatter` 트레이트 구현체가 담당한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActingGuide {
    /// NPC 이름
    pub npc_name: String,
    /// NPC 설명
    pub npc_description: String,
    /// 성격 스냅샷
    pub personality: PersonalitySnapshot,
    /// 감정 스냅샷
    pub emotion: EmotionSnapshot,
    /// 연기 지시문
    pub directive: ActingDirective,
    /// 상황 설명 (있으면)
    pub situation_description: Option<String>,
    /// 관계 스냅샷 (있으면)
    pub relationship: Option<RelationshipSnapshot>,
}

impl ActingGuide {
    /// NPC + EmotionState + Relationship → ActingGuide 생성
    ///
    /// `partner_name`은 표시용 파트너 NPC 이름. 빈 문자열이면
    /// `Relationship::target_id()`로 fallback된다.
    pub fn build(
        npc: &Npc,
        state: &EmotionState,
        situation_desc: Option<String>,
        relationship: Option<&Relationship>,
        partner_name: &str,
    ) -> Self {
        Self {
            npc_name: npc.name().to_string(),
            npc_description: npc.description().to_string(),
            personality: PersonalitySnapshot::from_profile(npc.personality()),
            emotion: EmotionSnapshot::from_state(state),
            directive: ActingDirective::from_emotion_and_personality(state, npc.personality()),
            situation_description: situation_desc,
            relationship: relationship.map(|r| RelationshipSnapshot::from_relationship(r, partner_name)),
        }
    }
}
