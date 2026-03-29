use crate::domain::emotion::{AppraisalEngine, StimulusEngine, EmotionState};
use crate::domain::guide::ActingGuide;
use crate::domain::pad::Pad;
use crate::domain::personality::Npc;
use crate::domain::relationship::Relationship;
use crate::ports::GuideFormatter;
use crate::presentation::korean::KoreanFormatter;

use super::dto::*;

/// 오류 타입: 서비스 계층에서 발생하는 도메인 오류
#[derive(Debug, thiserror::Error)]
pub enum MindServiceError {
    #[error("NPC '{0}'를 찾을 수 없습니다.")]
    NpcNotFound(String),
    #[error("관계 '{0}↔{1}'를 찾을 수 없습니다.")]
    RelationshipNotFound(String, String),
    #[error("상황 정보가 잘못되었습니다: {0}")]
    InvalidSituation(String),
    #[error("현재 감정 상태를 찾을 수 없습니다. (먼저 평가를 실행하세요)")]
    EmotionStateNotFound,
}

/// 라이브러리 사용자가 제공해야 할 저장소 트레이트
/// 
/// 웹 서버의 StateInner나 데이터베이스를 통해 NPC/관계/감정을 조회하고 저장하는 역할을 합니다.
pub trait MindRepository {
    fn get_npc(&self, id: &str) -> Option<Npc>;
    fn get_relationship(&self, owner_id: &str, target_id: &str) -> Option<Relationship>;
    fn get_object_description(&self, object_id: &str) -> Option<String>;
    
    // 상태 관리 (현재 턴의 감정 상태)
    fn get_emotion_state(&self, npc_id: &str) -> Option<EmotionState>;
    fn save_emotion_state(&mut self, npc_id: &str, state: EmotionState);
    fn clear_emotion_state(&mut self, npc_id: &str);

    // 관계 갱신
    fn save_relationship(&mut self, owner_id: &str, target_id: &str, rel: Relationship);
}

/// Mind 엔진의 핵심 진입점 (Application Service)
pub struct MindService<R: MindRepository> {
    repository: R,
}

impl<R: MindRepository> MindService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    /// 상황을 평가하여 감정을 생성하고 가이드를 반환합니다.
    ///
    /// * `trace_collector`: (웹 환경 등에서) 로그를 수집하기 위해 실행 전후에 호출할 콜백 (선택적)
    pub fn appraise(
        &mut self,
        req: AppraiseRequest,
        mut before_eval: impl FnMut(),
        mut after_eval: impl FnMut() -> Vec<String>,
    ) -> Result<AppraiseResponse, MindServiceError> {
        let npc = self.repository.get_npc(&req.npc_id)
            .ok_or_else(|| MindServiceError::NpcNotFound(req.npc_id.clone()))?;

        let relationship = self.repository.get_relationship(&req.npc_id, &req.partner_id)
            .ok_or_else(|| MindServiceError::RelationshipNotFound(req.npc_id.clone(), req.partner_id.clone()))?;

        let situation = req.situation.to_domain(&self.repository, &req.npc_id, &req.partner_id)?;

        before_eval();
        let emotion_state = AppraisalEngine::appraise(npc.personality(), &situation, &relationship);
        let trace = after_eval();

        let guide = ActingGuide::build(&npc, &emotion_state, Some(situation.description.clone()), Some(&relationship));
        let formatter = KoreanFormatter::new();
        let prompt = formatter.format_prompt(&guide);

        let emotions: Vec<EmotionOutput> = emotion_state.emotions().iter()
            .map(|e| EmotionOutput {
                emotion_type: format!("{:?}", e.emotion_type()),
                intensity: e.intensity(),
                context: e.context().map(|s| s.to_string()),
            })
            .collect();

        let dominant = emotion_state.dominant().map(|e| EmotionOutput {
            emotion_type: format!("{:?}", e.emotion_type()),
            intensity: e.intensity(),
            context: e.context().map(|s| s.to_string()),
        });

        let mood = emotion_state.overall_valence();

        self.repository.save_emotion_state(&req.npc_id, emotion_state);

        Ok(AppraiseResponse {
            emotions,
            dominant,
            mood,
            prompt,
            trace,
        })
    }

    /// 대화 턴 중 발생한 PAD 자극을 적용하여 감정을 갱신합니다.
    pub fn apply_stimulus(&mut self, req: StimulusRequest) -> Result<AppraiseResponse, MindServiceError> {
        let npc = self.repository.get_npc(&req.npc_id)
            .ok_or_else(|| MindServiceError::NpcNotFound(req.npc_id.clone()))?;

        let relationship = self.repository.get_relationship(&req.npc_id, &req.partner_id)
            .ok_or_else(|| MindServiceError::RelationshipNotFound(req.npc_id.clone(), req.partner_id.clone()))?;

        let current = self.repository.get_emotion_state(&req.npc_id)
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        let pad = Pad { pleasure: req.pleasure, arousal: req.arousal, dominance: req.dominance };
        let new_state = StimulusEngine::apply_stimulus(npc.personality(), &current, &pad);

        let guide = ActingGuide::build(&npc, &new_state, req.situation_description.clone(), Some(&relationship));
        let formatter = KoreanFormatter::new();
        let prompt = formatter.format_prompt(&guide);

        let emotions: Vec<EmotionOutput> = new_state.emotions().iter()
            .map(|e| EmotionOutput {
                emotion_type: format!("{:?}", e.emotion_type()),
                intensity: e.intensity(),
                context: e.context().map(|s| s.to_string()),
            })
            .collect();

        let dominant = new_state.dominant().map(|e| EmotionOutput {
            emotion_type: format!("{:?}", e.emotion_type()),
            intensity: e.intensity(),
            context: e.context().map(|s| s.to_string()),
        });

        let mood = new_state.overall_valence();

        self.repository.save_emotion_state(&req.npc_id, new_state);

        Ok(AppraiseResponse {
            emotions,
            dominant,
            mood,
            prompt,
            trace: vec![],
        })
    }

    /// 현재 감정 상태 기반으로 가이드를 재생성합니다.
    pub fn generate_guide(&self, req: GuideRequest) -> Result<GuideResponse, MindServiceError> {
        let npc = self.repository.get_npc(&req.npc_id)
            .ok_or_else(|| MindServiceError::NpcNotFound(req.npc_id.clone()))?;

        let relationship = self.repository.get_relationship(&req.npc_id, &req.partner_id)
            .ok_or_else(|| MindServiceError::RelationshipNotFound(req.npc_id.clone(), req.partner_id.clone()))?;

        let emotion_state = self.repository.get_emotion_state(&req.npc_id)
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        let guide = ActingGuide::build(&npc, &emotion_state, req.situation_description.clone(), Some(&relationship));
        let formatter = KoreanFormatter::new();
        let prompt = formatter.format_prompt(&guide);
        let json = formatter.format_json(&guide).unwrap_or_default();

        Ok(GuideResponse { prompt, json })
    }

    /// 대화가 종료된 후, 현재 감정 상태를 바탕으로 관계를 갱신합니다.
    pub fn after_dialogue(&mut self, req: AfterDialogueRequest) -> Result<AfterDialogueResponse, MindServiceError> {
        let relationship = self.repository.get_relationship(&req.npc_id, &req.partner_id)
            // 양방향 관계 조회 (A->B 가 없으면 B->A 시도) -> 웹 환경의 로직과 유사하게
            .or_else(|| self.repository.get_relationship(&req.partner_id, &req.npc_id))
            .ok_or_else(|| MindServiceError::RelationshipNotFound(req.npc_id.clone(), req.partner_id.clone()))?;

        let emotion_state = self.repository.get_emotion_state(&req.npc_id)
            .ok_or(MindServiceError::EmotionStateNotFound)?;

        let before = RelationshipValues {
            closeness: relationship.closeness().value(),
            trust: relationship.trust().value(),
            power: relationship.power().value(),
        };

        let significance = req.significance.unwrap_or(0.0).clamp(0.0, 1.0);
        let new_rel = relationship.after_dialogue(&emotion_state, req.praiseworthiness, significance);

        let after = RelationshipValues {
            closeness: new_rel.closeness().value(),
            trust: new_rel.trust().value(),
            power: new_rel.power().value(),
        };

        // 관계 갱신 및 상태 초기화
        self.repository.save_relationship(&req.npc_id, &req.partner_id, new_rel);
        self.repository.clear_emotion_state(&req.npc_id);

        Ok(AfterDialogueResponse { before, after })
    }
}
