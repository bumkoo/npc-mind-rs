//! 포트 정의 — 헥사고날 아키텍처의 확장 포인트
//!
//! 도메인 핵심 로직의 추상화 경계를 정의한다.
//! 외부 어댑터는 이 트레이트를 구현하여 도메인과 연결된다.

use crate::domain::emotion::{EmotionState, Situation, SceneFocus, RelationshipModifiers};
use crate::domain::guide::ActingGuide;
use crate::domain::pad::Pad;
use crate::domain::personality::{Npc, DimensionAverages};
use crate::domain::relationship::Relationship;

// ---------------------------------------------------------------------------
// 성격 프로필 포트
// ---------------------------------------------------------------------------

/// 성격 프로필 포트 — 가이드 생성 시 성격 차원 요약을 제공
///
/// HEXACO, Big Five 등 구체적 성격 모델이 이 트레이트를 구현하여
/// 가이드 도메인이 성격 모델의 내부 facet 구조를 모른 채
/// 차원 평균만 받아 연기 지시/스냅샷을 생성할 수 있다.
pub trait PersonalityProfile {
    /// 성격 차원별 평균 점수를 반환
    fn dimension_averages(&self) -> DimensionAverages;
}

// ---------------------------------------------------------------------------
// OCC 감정 평가 가중치 포트
// ---------------------------------------------------------------------------

/// OCC 감정 평가 가중치 포트 — 성격 모델이 자극의 해석 강도를 반환
///
/// 성격 모델(HEXACO, Big Five 등)이 이 트레이트를 구현하여
/// "이 자극을 내 성격으로 얼마나 크게 느끼냐"를 캡슐화한다.
/// 엔진은 성격 모델의 내부 facet을 모른 채 가중치만 받아 사용한다.
///
/// 모든 weight는 가산 모델(base + facets)로 계산하며,
/// clamp(0.5, 1.5) 범위로 극단값을 방지한다.
pub trait AppraisalWeights {
    /// 사건-자기-현재: Joy, Distress 가중치
    /// d > 0 → 기쁨 증폭 계수, d < 0 → 고통 증폭 계수
    fn desirability_self_weight(&self, desirability: f32) -> f32;

    /// 사건-자기-전망: Hope, Fear 가중치
    /// d > 0 → 희망 증폭, d < 0 → 공포 증폭
    fn desirability_prospect_weight(&self, desirability: f32) -> f32;

    /// 사건-자기-확인: Satisfaction, Disappointment, Relief, FearsConfirmed 가중치
    fn desirability_confirmation_weight(&self, desirability: f32) -> f32;

    /// 사건-타인-공감: HappyFor, Pity 가중치
    /// 0이면 미발동, >0이면 강도. d > 0 → 대리기쁨, d < 0 → 연민
    fn empathy_weight(&self, desirability: f32) -> f32;

    /// 사건-타인-적대: Resentment, Gloating 가중치
    /// 0이면 미발동, >0이면 강도. d > 0 → 시기, d < 0 → 쾌재
    fn hostility_weight(&self, desirability: f32) -> f32;

    /// 행동 평가: Pride, Shame, Admiration, Reproach 가중치
    /// is_self: 자기 행동 여부, pw 부호로 칭찬/비난 분기
    fn praiseworthiness_weight(&self, is_self: bool, praiseworthiness: f32) -> f32;

    /// 대상 호불호: Love, Hate 가중치
    fn appealingness_weight(&self, appealingness: f32) -> f32;
}

// ---------------------------------------------------------------------------
// 감정 평가 엔진 포트
// ---------------------------------------------------------------------------

/// 감정 평가 포트 — 성격 × 상황 × 관계 modifier 기반 OCC 감정 생성
///
/// 상황 진입 시 1회 평가. 대화 중 감정 변동은 StimulusProcessor가 담당.
pub trait Appraiser {
    /// 성격(가중치) + 상황 + 관계 modifier → 감정 상태 (상황 진입 시 1회)
    fn appraise<P: AppraisalWeights>(
        &self,
        personality: &P,
        situation: &Situation,
        dialogue_modifiers: &RelationshipModifiers,
    ) -> EmotionState;
}

/// 대사 자극 수용도 포트 — 성격이 자극을 얼마나 크게 수용하는가
///
/// AppraisalWeights가 "상황을 얼마나 크게 느끼냐"라면,
/// StimulusWeights는 "대화 중 자극에 얼마나 흔들리냐"를 캡슐화한다.
pub trait StimulusWeights {
    /// 자극 수용도 (0.1 ~ 2.0)
    /// 높을수록 대사에 크게 반응, 낮을수록 덤덤
    fn stimulus_absorb_rate(&self, stimulus: &Pad) -> f32;
}

/// 대사 자극 처리 포트 — 대화 매 턴 감정 변동
///
/// 기존 감정의 강도만 변동. 새 감정 생성 없음.
pub trait StimulusProcessor {
    /// 성격(수용도) + 현재 감정 + PAD 자극 → 갱신된 감정 상태
    fn apply_stimulus<P: StimulusWeights>(
        &self,
        personality: &P,
        current_state: &EmotionState,
        stimulus: &Pad,
    ) -> EmotionState;
}

/// 인프라 포트: 텍스트 → 벡터 변환 (임베딩)
///
/// 임베딩 모델(fastembed, ort, Python 서버 등)이 이 트레이트를 구현.
/// 도메인(PadAnalyzer)은 이 트레이트에만 의존하고
/// 구체적 임베딩 구현을 알지 못한다.
pub trait TextEmbedder {
    /// 텍스트 목록 → 임베딩 벡터 목록
    fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError>;
}

/// 임베딩 오류
#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("임베딩 모델 초기화 실패: {0}")]
    InitError(String),
    #[error("임베딩 추론 실패: {0}")]
    InferenceError(String),
}

/// 도메인 포트: 대사 → PAD 변환
///
/// PadAnalyzer가 이 트레이트를 구현.
/// TextEmbedder로 벡터를 얻고, 앵커 비교로 PAD를 계산.
pub trait UtteranceAnalyzer {
    /// 대사 텍스트 → PAD (Pleasure, Arousal, Dominance)
    fn analyze(&mut self, utterance: &str) -> Result<Pad, EmbedError>;
}

// ---------------------------------------------------------------------------
// 저장소 포트
// ---------------------------------------------------------------------------

/// 라이브러리 사용자가 제공해야 할 저장소 트레이트
///
/// NPC/관계/감정/Scene 상태를 조회하고 저장하는 역할을 합니다.
/// 인메모리, 파일, DB 등 구체적 저장 방식은 어댑터가 결정합니다.
pub trait MindRepository {
    fn get_npc(&self, id: &str) -> Option<Npc>;
    fn get_relationship(&self, owner_id: &str, target_id: &str) -> Option<Relationship>;
    fn get_object_description(&self, object_id: &str) -> Option<String>;

    // 감정 상태 관리
    fn get_emotion_state(&self, npc_id: &str) -> Option<EmotionState>;
    fn save_emotion_state(&mut self, npc_id: &str, state: EmotionState);
    fn clear_emotion_state(&mut self, npc_id: &str);

    // 관계 갱신
    fn save_relationship(&mut self, owner_id: &str, target_id: &str, rel: Relationship);

    // Scene 상태 관리
    fn get_scene_focuses(&self) -> &[SceneFocus];
    fn set_scene_focuses(&mut self, focuses: Vec<SceneFocus>);
    fn get_active_focus_id(&self) -> Option<&str>;
    fn set_active_focus_id(&mut self, id: Option<String>);
    fn get_scene_npc_id(&self) -> Option<&str>;
    fn get_scene_partner_id(&self) -> Option<&str>;
    fn set_scene_ids(&mut self, npc_id: String, partner_id: String);
}

/// 연기 가이드 포맷터 포트 — 가이드를 특정 형식으로 변환
///
/// 다국어 지원, 다른 LLM 포맷 등 다양한 출력 형식을 제공할 수 있다.
/// `Send + Sync`를 요구하여 `Arc<dyn GuideFormatter>`로 공유 가능.
pub trait GuideFormatter: Send + Sync {
    /// 프롬프트 텍스트 생성
    fn format_prompt(&self, guide: &ActingGuide) -> String;

    /// JSON 출력 생성
    fn format_json(&self, guide: &ActingGuide) -> Result<String, serde_json::Error>;
}
