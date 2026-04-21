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
    /// 정보 전달 (Step C2, Mind 컨텍스트)
    ///
    /// 화자가 listeners / overhearers에게 정보를 전달한다. Dispatcher가
    /// `TellInformationRequested`를 초기 이벤트로 만들고, `InformationAgent`가
    /// 청자당 1개의 `InformationTold` follow-up을 팬아웃(B5)한다. Inline
    /// `TellingIngestionHandler`가 각 청자의 `MemoryEntry(Heard/Rumor)`를 생성한다.
    TellInformation(TellInformationRequest),
    /// 소문 시딩 (Step C3, Memory 컨텍스트)
    ///
    /// 새 Rumor 애그리거트 생성. `RumorAgent`가 `RumorSeeded` follow-up을 발행하고
    /// `RumorStore`에 저장한다. 실제 확산은 별도 `SpreadRumor` 호출이 필요.
    SeedRumor(SeedRumorRequest),
    /// 소문 확산 (Step C3, Memory 컨텍스트)
    ///
    /// 기존 Rumor에 새 홉 추가. `RumorAgent`가 `RumorSpread` follow-up을 발행하고,
    /// Inline `RumorDistributionHandler`가 각 수신자에게 `MemoryEntry(Rumor)`를 생성한다.
    SpreadRumor(SpreadRumorRequest),
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
            Command::TellInformation(req) => &req.speaker,
            // Rumor 커맨드는 NPC에 묶이지 않음 — 단일 스칼라로 근사값 제공.
            Command::SeedRumor(_) => "",
            Command::SpreadRumor(req) => &req.rumor_id,
        }
    }

    /// 대화 상대 ID.
    ///
    /// **주의**: `TellInformation`·`SeedRumor`·`SpreadRumor`는 "상대" 개념이 없는
    /// Mind/Memory 컨텍스트 커맨드이므로 **빈 문자열을 반환**한다. 이 커맨드들은
    /// `aggregate_key()`가 `Npc(speaker)` 또는 `Rumor(_)`를 돌려주므로
    /// **Director가 Scene 기반으로 라우팅해서는 안 된다**. Director는 `Scene` 키 커맨드만
    /// 자기 SceneTask에 전달하고, 그 외는 dispatcher를 직접 호출한다. 호출자가 빈
    /// partner_id를 `SceneId::new(npc, "")`에 넣는 실수를 하면 유령 Scene과 매칭되므로
    /// 주의.
    pub fn partner_id(&self) -> &str {
        match self {
            Command::Appraise { partner_id, .. }
            | Command::ApplyStimulus { partner_id, .. }
            | Command::GenerateGuide { partner_id, .. }
            | Command::UpdateRelationship { partner_id, .. }
            | Command::EndDialogue { partner_id, .. }
            | Command::StartScene { partner_id, .. } => partner_id,
            Command::TellInformation(_)
            | Command::SeedRumor(_)
            | Command::SpreadRumor(_) => "",
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
            Command::TellInformation(req) => AggregateKey::Npc(req.speaker.clone()),
            // Seed는 아직 rumor_id가 없어 topic 또는 "orphan" 기반 임시 키.
            Command::SeedRumor(req) => {
                AggregateKey::Rumor(req.topic.clone().unwrap_or_else(|| "orphan".into()))
            }
            Command::SpreadRumor(req) => AggregateKey::Rumor(req.rumor_id.clone()),
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
