//! Command / CommandResult — CQRS Write Side 타입 정의

use crate::domain::aggregate::AggregateKey;

use super::super::dto::*;

/// 상태 변경 요청 (Write Side)
#[derive(Clone)]
pub enum Command {
    /// 상황 평가 → 감정 생성
    Appraise {
        npc_id: String,
        partner_id: String,
        situation: Option<SituationInput>,
    },
    /// PAD 자극 적용 → 감정 변동
    ApplyStimulus {
        npc_id: String,
        partner_id: String,
        pleasure: f32,
        arousal: f32,
        dominance: f32,
        situation_description: Option<String>,
    },
    /// 연기 가이드 재생성
    GenerateGuide {
        npc_id: String,
        partner_id: String,
        situation_description: Option<String>,
    },
    /// 관계 갱신 (Beat 종료)
    UpdateRelationship {
        npc_id: String,
        partner_id: String,
        significance: Option<f32>,
    },
    /// 대화 종료: 관계 갱신 + 감정 초기화 + Scene 정리
    EndDialogue {
        npc_id: String,
        partner_id: String,
        significance: Option<f32>,
    },
    /// Scene 시작: Focus 옵션 등록 + 초기 평가
    StartScene {
        npc_id: String,
        partner_id: String,
        significance: Option<f32>,
        focuses: Vec<SceneFocusInput>,
    },
}

impl Command {
    /// Command의 주체 NPC ID
    pub fn npc_id(&self) -> &str {
        match self {
            Command::Appraise { npc_id, .. }
            | Command::ApplyStimulus { npc_id, .. }
            | Command::GenerateGuide { npc_id, .. }
            | Command::UpdateRelationship { npc_id, .. }
            | Command::EndDialogue { npc_id, .. }
            | Command::StartScene { npc_id, .. } => npc_id,
        }
    }

    /// 대화 상대 ID
    pub fn partner_id(&self) -> &str {
        match self {
            Command::Appraise { partner_id, .. }
            | Command::ApplyStimulus { partner_id, .. }
            | Command::GenerateGuide { partner_id, .. }
            | Command::UpdateRelationship { partner_id, .. }
            | Command::EndDialogue { partner_id, .. }
            | Command::StartScene { partner_id, .. } => partner_id,
        }
    }

    /// 커맨드가 속한 aggregate 식별자 반환
    ///
    /// B안(다중 Scene) 이행 후 Director가 이 키로 적절한 SceneTask에 커맨드를 라우팅한다.
    ///
    /// **B4 Migration Note (plan §9.1):** `Command`에 `scene_id: Option<SceneId>` 필드가 추가되면
    /// `Appraise` · `ApplyStimulus` · `GenerateGuide`를 `scene_id.is_some()`일 때
    /// `Scene` 키로 승격해야 한다. 현재는 Scene 외부에서의 개별 NPC 평가로 간주.
    pub fn aggregate_key(&self) -> AggregateKey {
        match self {
            Command::StartScene {
                npc_id, partner_id, ..
            }
            | Command::EndDialogue {
                npc_id, partner_id, ..
            } => AggregateKey::Scene {
                npc_id: npc_id.clone(),
                partner_id: partner_id.clone(),
            },
            Command::UpdateRelationship {
                npc_id, partner_id, ..
            } => AggregateKey::Relationship {
                owner_id: npc_id.clone(),
                target_id: partner_id.clone(),
            },
            Command::Appraise { npc_id, .. }
            | Command::ApplyStimulus { npc_id, .. }
            | Command::GenerateGuide { npc_id, .. } => AggregateKey::Npc(npc_id.clone()),
        }
    }
}

/// Command 처리 결과
pub enum CommandResult {
    Appraised(AppraiseResult),
    StimulusApplied(StimulusResult),
    GuideGenerated(GuideResult),
    RelationshipUpdated(AfterDialogueResponse),
    DialogueEnded(AfterDialogueResponse),
    SceneStarted(SceneResult),
}
