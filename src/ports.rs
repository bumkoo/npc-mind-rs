//! 포트 정의 — 헥사고날 아키텍처의 확장 포인트
//!
//! 도메인 핵심 로직의 추상화 경계를 정의한다.
//! 외부 어댑터는 이 트레이트를 구현하여 도메인과 연결된다.

use crate::domain::emotion::{EmotionState, RelationshipModifiers, Scene, Situation};
use crate::domain::guide::ActingGuide;
use crate::domain::pad::{CachedPadEmbeddings, Pad, PadAnchorSet, UtteranceEmbedding};
use crate::domain::personality::{DimensionAverages, Npc};
use crate::domain::relationship::Relationship;
use crate::domain::scene_id::SceneId;

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

/// PAD 앵커 로딩 포트 — 포맷 무관 앵커 소스
///
/// TOML, JSON, DB 등 어디서든 앵커 텍스트와 캐싱된 임베딩을 로드.
/// 도메인(PadAnalyzer)은 이 트레이트에만 의존하고 파일 포맷을 모른다.
/// 차후 다른 설정 로딩에도 이 패턴을 재사용할 수 있다.
pub trait PadAnchorSource {
    /// 3축 앵커 텍스트 로드
    fn load_anchors(&self) -> Result<PadAnchorSet, AnchorLoadError>;

    /// 캐싱된 임베딩 로드 (없으면 None → 재계산 필요)
    fn load_cached_embeddings(&self) -> Result<Option<CachedPadEmbeddings>, AnchorLoadError>;

    /// 계산된 임베딩 저장 (캐시 경로 없으면 no-op)
    fn save_cached_embeddings(
        &self,
        embeddings: &CachedPadEmbeddings,
    ) -> Result<(), AnchorLoadError>;
}

/// 앵커 로딩 오류
#[derive(Debug, thiserror::Error)]
pub enum AnchorLoadError {
    #[error("앵커 파싱 실패: {0}")]
    ParseError(String),
    #[error("앵커 I/O 실패: {0}")]
    IoError(String),
    #[error("앵커 데이터 검증 실패: {0}")]
    ValidationError(String),
}

/// 도메인 포트: 대사 → PAD 변환
///
/// PadAnalyzer가 이 트레이트를 구현.
/// TextEmbedder로 벡터를 얻고, 앵커 비교로 PAD를 계산.
pub trait UtteranceAnalyzer {
    /// 대사 텍스트 → PAD (Pleasure, Arousal, Dominance)
    fn analyze(&mut self, utterance: &str) -> Result<Pad, EmbedError>;

    /// 대사 텍스트 → (PAD, 발화 임베딩) — 후속 단계와 임베딩 공유용
    ///
    /// 임베딩을 가진 분석기(예: PadAnalyzer)는 이 메서드를 override하여
    /// `Some(UtteranceEmbedding)`을 반환할 수 있다. 기본 구현은 `analyze()`만 호출하고
    /// 임베딩은 `None`으로 반환하므로 trait 호환성이 유지된다.
    ///
    /// `UtteranceEmbedding` newtype은 임의 `Vec<f32>`와 구분되며, `Deref<[f32]>` /
    /// `AsRef<[f32]>` 구현으로 분류기 호출 시 `&[f32]`로 강제 변환된다.
    ///
    /// 사용 예: `DialogueAgent`가 PAD 분석 결과의 임베딩을
    /// `ListenerPerspectiveConverter`에 재사용하여 발화당 임베딩 1회를 보장.
    fn analyze_with_embedding(
        &mut self,
        utterance: &str,
    ) -> Result<(Pad, Option<UtteranceEmbedding>), EmbedError> {
        self.analyze(utterance).map(|pad| (pad, None))
    }
}

// ---------------------------------------------------------------------------
// 저장소 포트 — ISP 분리
// ---------------------------------------------------------------------------

/// NPC/관계/오브젝트 월드 — 게임 세계 데이터 조회 및 관계 갱신
///
/// 라이브러리 사용자가 게임 엔티티 저장소에 맞게 구현합니다.
pub trait NpcWorld {
    fn get_npc(&self, id: &str) -> Option<Npc>;
    fn get_relationship(&self, owner_id: &str, target_id: &str) -> Option<Relationship>;
    fn get_object_description(&self, object_id: &str) -> Option<String>;
    fn save_relationship(&mut self, owner_id: &str, target_id: &str, rel: Relationship);
}

/// 감정 상태 저장소 — NPC별 감정 상태 CRUD
///
/// 인메모리, 파일, DB 등 구체적 저장 방식은 어댑터가 결정합니다.
pub trait EmotionStore {
    fn get_emotion_state(&self, npc_id: &str) -> Option<EmotionState>;
    fn save_emotion_state(&mut self, npc_id: &str, state: EmotionState);
    fn clear_emotion_state(&mut self, npc_id: &str);
}

/// Scene 상태 저장소 — Scene/Focus/Beat 관리
///
/// Scene 시작 시 Focus 목록을 등록하고, 대화 진행 중 활성 Focus를 관리합니다.
pub trait SceneStore {
    /// 현재 활성 Scene 정보를 조회합니다 (단일 Scene 레거시 경로).
    fn get_scene(&self) -> Option<Scene>;
    /// Scene 정보를 저장합니다 (생성 또는 갱신).
    fn save_scene(&mut self, scene: Scene);
    /// Scene 정보를 삭제합니다 (대화 종료 시).
    fn clear_scene(&mut self);

    /// B4 Session 3: 다중 Scene 환경에서 특정 Scene 조회.
    ///
    /// 기본 구현은 `get_scene()`이 일치하는 scene_id를 반환할 때만 Some. 단일 Scene
    /// 저장소는 이 기본 구현으로 충분. 다중 Scene 저장소(`InMemoryRepository`)는
    /// 오버라이드하여 HashMap 조회를 수행 — multi-scene 환경에서 **올바른** Scene을
    /// 반환함을 보장하도록 `StimulusAgent` 등이 이 메서드를 사용.
    fn get_scene_by_id(&self, scene_id: &SceneId) -> Option<Scene> {
        self.get_scene().filter(|s| {
            s.npc_id() == scene_id.npc_id && s.partner_id() == scene_id.partner_id
        })
    }
}

/// 편의 super-trait — 3개 포트를 모두 구현하면 자동으로 MindRepository
///
/// `MindService`는 이 트레이트를 바운드로 사용합니다.
/// 개별 포트만 필요한 곳(예: DTO 변환)에서는 `NpcWorld`만 요구합니다.
pub trait MindRepository: NpcWorld + EmotionStore + SceneStore {}

/// 3개 포트를 모두 구현한 타입은 자동으로 MindRepository
impl<T: NpcWorld + EmotionStore + SceneStore> MindRepository for T {}

// ---------------------------------------------------------------------------
// 대화 에이전트 포트 (chat feature)
// ---------------------------------------------------------------------------

/// 대화 턴 — 세션 내 한 턴의 발화
#[cfg(feature = "chat")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DialogueTurn {
    pub role: DialogueRole,
    pub content: String,
}

/// 발화 역할
#[cfg(feature = "chat")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DialogueRole {
    /// 시스템 프롬프트 (ActingGuide)
    System,
    /// 대화 상대 (Player 또는 상대 NPC)
    User,
    /// 이 NPC의 응답 (LLM 출력)
    Assistant,
}

/// llama-server가 `/v1/chat/completions` 응답에 포함하는 성능 메트릭
///
/// prompt(프롬프트 처리)와 predicted(토큰 생성) 두 단계의 속도 정보를 담는다.
/// llama-server 이외의 서버에서는 `None`으로 전달된다.
#[cfg(feature = "chat")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlamaTimings {
    /// 프롬프트 토큰 수
    pub prompt_n: u32,
    /// 프롬프트 처리 총 시간 (ms)
    pub prompt_ms: f64,
    /// 프롬프트 토큰당 처리 시간 (ms)
    pub prompt_per_token_ms: f64,
    /// 프롬프트 처리 속도 (tokens/sec)
    pub prompt_per_second: f64,
    /// 생성된 토큰 수
    pub predicted_n: u32,
    /// 토큰 생성 총 시간 (ms)
    pub predicted_ms: f64,
    /// 토큰당 생성 시간 (ms)
    pub predicted_per_token_ms: f64,
    /// 토큰 생성 속도 (tokens/sec)
    pub predicted_per_second: f64,
}

/// LLM 응답 + 선택적 성능 메트릭
///
/// `ConversationPort::send_message()` 및 `send_message_stream()`의 반환 타입.
/// llama-server에서는 `timings`가 채워지고, 그 외 서버에서는 `None`.
#[cfg(feature = "chat")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatResponse {
    /// LLM이 생성한 응답 텍스트
    pub text: String,
    /// llama-server 성능 메트릭 (없으면 None)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub timings: Option<LlamaTimings>,
}

/// 대화 에이전트 오류
#[cfg(feature = "chat")]
#[derive(Debug, thiserror::Error)]
pub enum ConversationError {
    #[error("LLM 연결 실패: {0}")]
    ConnectionError(String),
    #[error("세션을 찾을 수 없습니다: {0}")]
    SessionNotFound(String),
    #[error("LLM 추론 오류: {0}")]
    InferenceError(String),
}

/// 대화 에이전트 포트 — LLM과의 다턴 대화 세션을 추상화
///
/// Mind Engine이 생성한 프롬프트를 system prompt로 사용하여
/// LLM과 다턴 대화를 수행한다.
///
/// `rig`, `reqwest`, 또는 목(mock) 구현 등 구체적 LLM 클라이언트를
/// 교체할 수 있도록 포트로 추상화한다.
///
/// # 대화 흐름
///
/// ```rust,ignore
/// // 1. appraise()로 생성한 프롬프트로 세션 시작
/// port.start_session("s1", &prompt).await?;
///
/// // 2. 상대 대사 → NPC 응답
/// let npc_reply = port.send_message("s1", "오랜만이군.").await?;
///
/// // 3. Beat 전환 시 프롬프트 갱신
/// port.update_system_prompt("s1", &new_prompt).await?;
///
/// // 4. 대화 종료 → 이력 반환
/// let history = port.end_session("s1").await?;
/// ```
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct LlmModelInfo {
    pub provider_url: String,
    pub model_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub seed: Option<u64>,
}

impl Default for LlmModelInfo {
    fn default() -> Self {
        Self {
            provider_url: "unknown".into(),
            model_name: "unknown".into(),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: None,
            seed: None,
        }
    }
}

impl LlmModelInfo {
    /// NPC의 성격을 바탕으로 파라미터를 덮어씁니다.
    pub fn apply_npc_personality(&mut self, npc: &crate::domain::personality::Npc) {
        let (temp, top_p) = npc.derive_llm_parameters();
        self.temperature = Some(temp);
        self.top_p = Some(top_p);
    }
}

/// LLM 모델의 특성 및 메타데이터를 제공하는 포트
pub trait LlmInfoProvider: Send + Sync {
    fn get_model_info(&self) -> LlmModelInfo;
}

/// LLM 서버에서 모델 정보를 런타임에 재감지하는 포트 (chat feature 전용)
///
/// `dialogue_start` 시점에 호출하여, 서버 기동 이후 모델이 교체된 경우에도
/// 정확한 모델명을 반환한다.
#[cfg(feature = "chat")]
#[async_trait::async_trait]
pub trait LlmModelDetector: Send + Sync {
    async fn refresh_model_info(&self) -> Result<LlmModelInfo, String>;
}

// ---------------------------------------------------------------------------
// llama-server 모니터링 포트
// ---------------------------------------------------------------------------

/// llama-server 헬스 상태
///
/// `/health` 응답. `status`는 `"ok"`, `"no slot available"`, `"loading model"` 등.
#[cfg(feature = "chat")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlamaHealth {
    pub status: String,
}

/// llama-server 슬롯 정보
///
/// `/slots` 응답의 개별 슬롯. llama-server는 동시 요청 처리를 위해
/// 여러 슬롯을 사용하며, 각 슬롯의 상태(idle/processing)를 보고한다.
#[cfg(feature = "chat")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlamaSlotInfo {
    pub id: u32,
    /// 0=idle, 1=processing
    #[serde(default)]
    pub state: u32,
    /// 현재 슬롯의 프롬프트 토큰 수
    #[serde(default)]
    pub n_past: u32,
    /// 생성된 토큰 수
    #[serde(default)]
    pub n_predicted: u32,
    /// 처리 중 여부
    #[serde(default)]
    pub is_processing: bool,
    /// 파싱되지 않은 추가 필드 보존
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// llama-server Prometheus 메트릭 (파싱 결과)
///
/// `/metrics` 응답(Prometheus 텍스트)에서 주요 메트릭을 추출한다.
/// `raw` 필드에 원본 텍스트를 보존하여 UI에서 전체 메트릭을 표시할 수 있다.
#[cfg(feature = "chat")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlamaMetrics {
    /// 원본 Prometheus 텍스트
    pub raw: String,
    /// 프롬프트 토큰 총 수
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_total: Option<f64>,
    /// 생성 토큰 총 수
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_predicted_total: Option<f64>,
    /// 프롬프트 처리 총 시간 (초)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_seconds_total: Option<f64>,
    /// 토큰 생성 총 시간 (초)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_predicted_seconds_total: Option<f64>,
    /// 디코드 총 횟수
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_decode_total: Option<f64>,
    /// 디코드당 사용 중인 슬롯 수
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_busy_slots_per_decode: Option<f64>,
    /// 프롬프트 처리 속도 (tokens/sec)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_seconds: Option<f64>,
    /// 토큰 생성 속도 (tokens/sec)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predicted_tokens_seconds: Option<f64>,
    /// KV 캐시 사용률 (0.0~1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kv_cache_usage_ratio: Option<f64>,
    /// KV 캐시 토큰 수
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kv_cache_tokens: Option<f64>,
    /// 처리 중인 요청 수
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requests_processing: Option<f64>,
    /// 대기 중인 요청 수
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requests_deferred: Option<f64>,
}

#[cfg(feature = "chat")]
impl LlamaMetrics {
    /// Prometheus 텍스트에서 주요 메트릭을 파싱한다.
    pub fn parse(raw: &str) -> Self {
        fn extract(text: &str, key: &str) -> Option<f64> {
            text.lines()
                .find(|l| l.starts_with(key) && !l.starts_with('#'))
                .and_then(|l| l.split_whitespace().last()?.parse().ok())
        }
        Self {
            prompt_tokens_total: extract(raw, "llamacpp:prompt_tokens_total"),
            tokens_predicted_total: extract(raw, "llamacpp:tokens_predicted_total"),
            prompt_seconds_total: extract(raw, "llamacpp:prompt_seconds_total"),
            tokens_predicted_seconds_total: extract(raw, "llamacpp:tokens_predicted_seconds_total"),
            n_decode_total: extract(raw, "llamacpp:n_decode_total"),
            n_busy_slots_per_decode: extract(raw, "llamacpp:n_busy_slots_per_decode"),
            prompt_tokens_seconds: extract(raw, "llamacpp:prompt_tokens_seconds"),
            predicted_tokens_seconds: extract(raw, "llamacpp:predicted_tokens_seconds"),
            kv_cache_usage_ratio: extract(raw, "llamacpp:kv_cache_usage_ratio"),
            kv_cache_tokens: extract(raw, "llamacpp:kv_cache_tokens"),
            requests_processing: extract(raw, "llamacpp:requests_processing"),
            requests_deferred: extract(raw, "llamacpp:requests_deferred"),
            raw: raw.to_string(),
        }
    }
}

/// llama-server 고유 모니터링 API 포트
///
/// `/slots`, `/metrics`, `/health` 등 llama.cpp 서버 전용 엔드포인트를 추상화한다.
/// llama-server 이외의 OpenAI 호환 서버에서는 구현하지 않아도 된다.
#[cfg(feature = "chat")]
#[async_trait::async_trait]
pub trait LlamaServerMonitor: Send + Sync {
    /// 서버 헬스 체크 (`GET /health`)
    async fn health(&self) -> Result<LlamaHealth, String>;

    /// 슬롯 상태 조회 (`GET /slots`)
    async fn slots(&self) -> Result<Vec<LlamaSlotInfo>, String>;

    /// Prometheus 메트릭 조회 (`GET /metrics`)
    async fn metrics(&self) -> Result<LlamaMetrics, String>;
}

#[cfg(feature = "chat")]
#[async_trait::async_trait]
pub trait ConversationPort: Send + Sync {
    /// 새 대화 세션을 시작한다.
    ///
    /// `system_prompt`: MindEngine이 생성한 ActingGuide 프롬프트.
    /// `generation_config`: NPC 성격 등에 기반한 고정 생성 파라미터 (temp, top_p 등)
    async fn start_session(
        &self,
        session_id: &str,
        system_prompt: &str,
        generation_config: Option<LlmModelInfo>,
    ) -> Result<(), ConversationError>;

    /// 상대의 대사를 전달하고 NPC(LLM)의 응답을 받는다.
    ///
    /// 대화 이력 및 생성 파라미터는 세션 내부에서 관리된다.
    /// llama-server인 경우 `ChatResponse.timings`에 성능 메트릭이 포함된다.
    async fn send_message(
        &self,
        session_id: &str,
        user_message: &str,
    ) -> Result<ChatResponse, ConversationError>;

    /// 상대의 대사를 전달하고 NPC(LLM)의 응답을 스트리밍으로 받는다.
    ///
    /// 토큰은 `token_tx`로 실시간 전송되고, 완성된 응답 + timings가 반환된다.
    async fn send_message_stream(
        &self,
        session_id: &str,
        user_message: &str,
        token_tx: tokio::sync::mpsc::Sender<String>,
    ) -> Result<ChatResponse, ConversationError>;

    /// system_prompt를 갱신한다 (Beat 전환 시).
    ///
    /// 대화 이력은 유지하면서 LLM Agent의 system prompt만 교체한다.
    /// 감정 변화가 즉시 연기에 반영되도록 한다.
    async fn update_system_prompt(
        &self,
        session_id: &str,
        new_prompt: &str,
    ) -> Result<(), ConversationError>;

    /// 세션을 종료하고 전체 대화 이력을 반환한다.
    async fn end_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<DialogueTurn>, ConversationError>;
}

// ---------------------------------------------------------------------------
// 가이드 포맷터 포트
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// 기억 저장소 포트 (RAG)
// ---------------------------------------------------------------------------

use crate::domain::memory::{MemoryEntry, MemoryLayer, MemoryResult, MemoryScope, MemorySource};

/// Scope 기반 검색 필터.
///
/// `NpcAllowed(npc_id)`은 Step A에서는 "Personal Scope with matching npc_id | World"로
/// 근사 구현한다. Faction/Family 소속 Join은 Step C 이후 `NpcWorld` 조회를 받아 확장한다.
#[derive(Debug, Clone)]
pub enum MemoryScopeFilter {
    Any,
    Exact(MemoryScope),
    /// 이 NPC가 접근 가능한 모든 scope.
    NpcAllowed(String),
}

/// Ranker 이전 단계에서 `MemoryStore`에 넘길 질의 DTO.
///
/// Step A에서는 필드만 정의하고, `SqliteMemoryStore::search`가 SQL WHERE로 변환한다.
/// Ranker 호출은 Step B `DialogueAgent.inject_memory_push`에서 연결한다.
#[derive(Debug, Clone, Default)]
pub struct MemoryQuery {
    pub text: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub scope_filter: Option<MemoryScopeFilter>,
    pub source_filter: Option<Vec<MemorySource>>,
    pub layer_filter: Option<MemoryLayer>,
    pub topic: Option<String>,
    pub exclude_superseded: bool,
    pub exclude_consolidated_source: bool,
    pub min_retention: Option<f32>,
    pub current_pad: Option<(f32, f32, f32)>,
    pub limit: usize,
}

/// 기억 저장/검색 포트 — RAG 인덱스 추상화
///
/// `&self`로 호출하여 `Arc<dyn MemoryStore>` 공유가 가능합니다.
/// 내부 가변성(interior mutability)으로 동시성을 처리합니다.
///
/// **Step A 마이그레이션**: 기존 5개 메서드는 유지되며 Step B에서 `#[deprecated]` 처리 예정.
/// 신규 7개 메서드(`search` 이하)는 기본 구현 없이 모든 구현체가 제공한다.
pub trait MemoryStore: Send + Sync {
    // ---- 기존 메서드 (호환 유지) ----

    /// 기억 인덱싱 (메타데이터 + 선택적 임베딩 벡터)
    fn index(&self, entry: MemoryEntry, embedding: Option<Vec<f32>>) -> Result<(), MemoryError>;

    /// 의미 기반 검색 (벡터 유사도)
    #[deprecated(since = "0.4.0", note = "Use MemoryStore::search(MemoryQuery { embedding: Some(..), .. })")]
    fn search_by_meaning(
        &self,
        query_embedding: &[f32],
        npc_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>, MemoryError>;

    /// 키워드 기반 검색 (텍스트 매칭)
    #[deprecated(since = "0.4.0", note = "Use MemoryStore::search(MemoryQuery { text: Some(..), .. })")]
    fn search_by_keyword(
        &self,
        keyword: &str,
        npc_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>, MemoryError>;

    /// 최근 기억 조회 (시간순 내림차순)
    #[deprecated(since = "0.4.0", note = "Use MemoryStore::search(MemoryQuery { scope_filter: Some(NpcAllowed(..)), .. })")]
    fn get_recent(
        &self,
        npc_id: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError>;

    /// 저장된 기억 수
    fn count(&self) -> usize;

    // ---- Step A 신규 메서드 ----

    /// Scope/Source/Layer/topic 등 다축 필터 기반 검색.
    fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryResult>, MemoryError>;

    /// ID로 단일 엔트리 조회.
    fn get_by_id(&self, id: &str) -> Result<Option<MemoryEntry>, MemoryError>;

    /// Topic의 최신 유효 엔트리(superseded 되지 않은 것). `created_seq DESC` 기준.
    fn get_by_topic_latest(&self, topic: &str) -> Result<Option<MemoryEntry>, MemoryError>;

    /// Topic의 Canonical(`Seeded + World scope`) 엔트리. Rumor 콘텐츠 해소용.
    fn get_canonical_by_topic(&self, topic: &str) -> Result<Option<MemoryEntry>, MemoryError>;

    /// `old_id`에 `superseded_by = new_id` 마킹.
    fn mark_superseded(&self, old_id: &str, new_id: &str) -> Result<(), MemoryError>;

    /// `a_ids` 각각에 `consolidated_into = b_id` 마킹 (Layer A → B 흡수).
    fn mark_consolidated(&self, a_ids: &[String], b_id: &str) -> Result<(), MemoryError>;

    /// 회상 발생 기록 — `last_recalled_at` / `recall_count` 갱신.
    fn record_recall(&self, id: &str, now_ms: u64) -> Result<(), MemoryError>;

    /// 저장된 모든 MemoryEntry 삭제 (Step E3.2 — 시나리오 로드 시 fresh start).
    ///
    /// dev tool(Mind Studio)의 `load_state` 경로에서 이전 시나리오 시드/런타임 엔트리를
    /// 제거하기 위해 사용. 영구 저장소 사용자는 호출 여부를 결정해야 한다.
    /// 벡터 인덱스(SQLite vec0)가 있다면 함께 비운다.
    fn clear_all(&self) -> Result<(), MemoryError>;
}

/// 기억 저장소 오류
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("기억 저장소 오류: {0}")]
    StorageError(String),
    #[error("임베딩 오류: {0}")]
    EmbeddingError(String),
}

// ---------------------------------------------------------------------------
// 소문 저장소 포트 (Step C1 foundation)
// ---------------------------------------------------------------------------

use crate::domain::rumor::{ReachPolicy, Rumor};

/// 소문 애그리거트 저장/검색 포트.
///
/// Step C1에서는 trait 시그니처만 정의하고 `SqliteRumorStore` 어댑터가 구현한다.
/// 실제 호출 경로(`RumorAgent`, `RumorDistributionHandler`)는 Step C3에서 연결된다.
pub trait RumorStore: Send + Sync {
    /// 신규 또는 기존 rumor upsert. `Rumor.validate()`로 이미 검증된 값을 받는다.
    fn save(&self, rumor: &Rumor) -> Result<(), MemoryError>;

    /// ID로 단일 rumor 조회. Hop·distortion 목록을 모두 포함해 복원한다.
    fn load(&self, id: &str) -> Result<Option<Rumor>, MemoryError>;

    /// Topic에 묶인 모든 rumor 조회. Canonical 해소 대상 탐색 등에 사용.
    fn find_by_topic(&self, topic: &str) -> Result<Vec<Rumor>, MemoryError>;

    /// 주어진 `reach`에 도달 가능한 활성 rumor 목록.
    ///
    /// reach 규칙: reach.regions/factions/npc_ids는 "이 축 중 하나라도 겹치면 도달"로
    /// 해석한다(설계 §2.6). min_significance는 하한 필터.
    /// Step C1에서는 후보 필터만 제공하고 최종 정책은 Step C3 `RumorAgent`가 결정.
    fn find_active_in_reach(&self, reach: &ReachPolicy) -> Result<Vec<Rumor>, MemoryError>;

    /// 저장된 모든 rumor 목록 (status 필터 없음 — Active/Fading/Faded 전부 포함).
    ///
    /// UI 조회용 헬퍼. 정렬은 구현체 재량이지만 Sqlite 구현은 `created_at DESC`.
    fn list_all(&self) -> Result<Vec<Rumor>, MemoryError>;

    /// 저장된 모든 Rumor (+ hops, distortions) 삭제 (Step E3.2 — 시나리오 로드 시 fresh start).
    ///
    /// Mind Studio의 `load_state`가 `MemoryStore::clear_all`과 쌍으로 호출한다.
    fn clear_all(&self) -> Result<(), MemoryError>;
}

/// 기억 프레이밍 포트 (Step B — LLM 프롬프트 주입용).
///
/// `MemoryEntry`를 Source별 라벨(예: `[겪음]`/`[목격]`/`[전해 들음]`/`[강호에 떠도는 소문]`)로
/// 포맷해 "떠오르는 기억" 블록을 구성한다. `DialogueAgent.inject_memory_push`가
/// `MemoryRanker` 결과를 이 포트로 프레이밍한다.
pub trait MemoryFramer: Send + Sync {
    /// 단일 엔트리를 source별 라벨로 포맷 (예: `"[겪음] content"`).
    /// 미지원 locale 또는 섹션 누락 시 raw content를 그대로 반환한다.
    fn frame(&self, entry: &MemoryEntry, locale: &str) -> String;

    /// 엔트리 목록을 header/footer로 감싼 하나의 프롬프트 블록으로 포맷.
    /// 빈 slice면 빈 문자열을 반환해 caller가 `format!("{block}{prompt}")` 형태로
    /// prepend만 하면 no-op이 되도록 한다.
    fn frame_block(&self, entries: &[MemoryEntry], locale: &str) -> String;
}
