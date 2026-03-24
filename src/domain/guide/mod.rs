//! LLM 연기 가이드 생성기
//!
//! NPC의 성격(HEXACO) + 현재 감정(OCC EmotionState)을 조합하여
//! LLM이 해당 NPC를 연기할 수 있는 구조화된 가이드를 생성한다.
//!
//! 이 모듈의 출력이 NPC 심리 엔진의 최종 산출물이다.
//! 텍스트/JSON 등 구체적 포맷 변환은 presentation 레이어(GuideFormatter)가 담당한다.

mod enums;
mod snapshot;
mod directive;

pub use enums::*;
pub use snapshot::*;
pub use directive::*;

use serde::{Deserialize, Serialize};

use super::emotion::EmotionState;
use super::personality::Npc;
use super::relationship::Relationship;

/// 감정의 유의미 판단 기준 (이 이상이면 연기에 반영)
pub const EMOTION_THRESHOLD: f32 = 0.2;
/// 성격 특성 추출 임계값 (차원 평균이 이 이상이면 두드러진 특성으로 판단)
pub const TRAIT_THRESHOLD: f32 = 0.3;
/// 연기 지시 분위기 판단 임계값
const MOOD_THRESHOLD: f32 = 0.3;
/// 정직성(H)이 높을 때 거짓말 금지 제약 발동 임계값
const HONESTY_RESTRICTION_THRESHOLD: f32 = 0.5;

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
    pub fn build(
        npc: &Npc,
        state: &EmotionState,
        situation_desc: Option<String>,
        relationship: Option<&Relationship>,
    ) -> Self {
        Self {
            npc_name: npc.name().to_string(),
            npc_description: npc.description().to_string(),
            personality: PersonalitySnapshot::from_profile(npc.personality()),
            emotion: EmotionSnapshot::from_state(state),
            directive: ActingDirective::from_emotion_and_personality(state, npc.personality()),
            situation_description: situation_desc,
            relationship: relationship.map(RelationshipSnapshot::from_relationship),
        }
    }
}
