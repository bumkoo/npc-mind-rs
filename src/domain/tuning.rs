//! 튜닝 프로파일 — 도메인 정책 파라미터를 런타임 주입 가능한 한 곳에 모아 관리.
//!
//! ## 변경 이력 (헥사고날/DDD 리뷰 #1)
//!
//! 이전엔 모든 파라미터를 `pub const`로 노출했다. 컴파일 시점에 호출처에 인라인 박혀
//! **외부 크레이트가 정책을 조정할 수 없는** 구조였고, 게임 디자인 결정이 도메인 코드에
//! 융합되어 DDD 관점에서도 안티패턴이었다. 이제는 `TuningProfile` 구조체로 추출하고,
//! 프로세스 시작 시 1회 `install()`로 주입할 수 있다 (미설치 시 `Default`).
//!
//! ## 사용
//!
//! ```ignore
//! use npc_mind::domain::tuning::{install, profile, TuningProfile};
//!
//! // 프로세스 시작 시 (선택)
//! install(TuningProfile {
//!     stimulus_impact_rate: 0.7,
//!     ..Default::default()
//! }).expect("once only");
//!
//! // 도메인 코드에서
//! let rate = profile().stimulus_impact_rate;
//! ```
//!
//! `install()`이 호출되지 않으면 첫 `profile()` 호출에서 `Default::default()`로 초기화된다.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 시간 단위 — 진짜 상수 (1일 = 86_400_000 ms). 튜닝 대상 아님.
// ---------------------------------------------------------------------------

/// 하루 (ms) — retention 나이 계산 기준
pub const DAY_MS: u64 = 86_400_000;

// ---------------------------------------------------------------------------
// 기본값 (TuningProfile::default()의 원천)
//
// 별도 모듈에 두고 `const _: () = { assert!(...) }`로 컴파일타임 검증한다.
// 사용자가 install()로 다른 값을 넣으면 `install()` 안에서 debug_assert로 같은 검증을 수행.
// ---------------------------------------------------------------------------

mod defaults {
    // Stimulus
    pub const STIMULUS_IMPACT_RATE: f32 = 0.5;
    pub const STIMULUS_FADE_THRESHOLD: f32 = 0.05;
    pub const STIMULUS_MIN_INERTIA: f32 = 0.30;

    // Beat
    pub const BEAT_MERGE_THRESHOLD: f32 = 0.2;
    pub const BEAT_DEFAULT_SIGNIFICANCE: f32 = 0.5;

    // 관계 갱신
    pub const TRUST_UPDATE_RATE: f32 = 0.1;
    pub const CLOSENESS_UPDATE_RATE: f32 = 0.05;
    pub const SIGNIFICANCE_SCALE: f32 = 3.0;

    // PAD
    pub const PAD_D_SCALE_WEIGHT: f32 = 0.3;
    pub const PAD_AXIS_DEAD_ZONE: f32 = 0.02;
    pub const PAD_AXIS_SCALE: f32 = 3.0;

    // 가이드
    pub const MOOD_THRESHOLD: f32 = 0.3;
    pub const HONESTY_RESTRICTION_THRESHOLD: f32 = 0.5;
    pub const EMOTION_THRESHOLD: f32 = 0.2;
    pub const TRAIT_THRESHOLD: f32 = 0.3;

    // 관계 변조
    pub const REL_CLOSENESS_INTENSITY_WEIGHT: f32 = 0.5;
    pub const REL_TRUST_EMOTION_WEIGHT: f32 = 0.3;
    pub const REL_CLOSENESS_EMPATHY_WEIGHT: f32 = 0.3;
    pub const REL_CLOSENESS_HOSTILITY_WEIGHT: f32 = 0.3;

    // Level 임계값
    pub const LEVEL_VERY_HIGH_THRESHOLD: f32 = 0.6;
    pub const LEVEL_HIGH_THRESHOLD: f32 = 0.2;
    pub const LEVEL_LOW_THRESHOLD: f32 = -0.2;
    pub const LEVEL_VERY_LOW_THRESHOLD: f32 = -0.6;

    // LLM 파라미터
    pub const LLM_BASE_TEMPERATURE: f32 = 0.8;
    pub const LLM_TEMP_OPENNESS_WEIGHT: f32 = 0.1;
    pub const LLM_TEMP_EXTRAVERSION_WEIGHT: f32 = 0.05;
    pub const LLM_TEMP_CONSCIENTIOUSNESS_WEIGHT: f32 = 0.1;
    pub const LLM_TEMP_HONESTY_WEIGHT: f32 = 0.05;
    pub const LLM_BASE_TOP_P: f32 = 0.9;
    pub const LLM_TOP_P_OPENNESS_WEIGHT: f32 = 0.05;
    pub const LLM_TOP_P_CONSCIENTIOUSNESS_WEIGHT: f32 = 0.05;
    pub const LLM_TEMP_MIN: f32 = 0.1;
    pub const LLM_TEMP_MAX: f32 = 1.1;
    pub const LLM_TOP_P_MIN: f32 = 0.8;
    pub const LLM_TOP_P_MAX: f32 = 1.0;

    // SceneTask
    pub const SCENE_TASK_CHANNEL_CAPACITY: usize = 32;

    // Memory (Step A)
    pub const MEMORY_RETENTION_CUTOFF: f32 = 0.10;
    pub const RECALL_BOOST_FACTOR: f32 = 0.15;
    pub const EMOTION_PROXIMITY_BONUS: f32 = 0.30;
    pub const RECENCY_BOOST_TAU_DAYS: f32 = 3.0;
    pub const SIMILARITY_CLUSTER_THRESHOLD: f32 = 0.85;
    pub const DECAY_TAU_DEFAULT_DAYS: f32 = 30.0;

    // Source 가중치
    pub const SOURCE_W_EXPERIENCED: f32 = 1.00;
    pub const SOURCE_W_WITNESSED: f32 = 0.85;
    pub const SOURCE_W_HEARD: f32 = 0.60;
    pub const SOURCE_W_RUMOR: f32 = 0.35;

    // 프롬프트 예산
    pub const MEMORY_PUSH_TOP_K: usize = 5;
    pub const MEMORY_PROMPT_TOKEN_BUDGET: usize = 400;

    // Rumor 감쇠
    pub const RUMOR_HOP_CONFIDENCE_DECAY: f32 = 0.8;
    pub const RUMOR_MIN_CONFIDENCE: f32 = 0.1;

    // Memory 저장 필터
    pub const MEMORY_RELATIONSHIP_DELTA_THRESHOLD: f32 = 0.05;

    // 컴파일타임 invariants — 기본값에 대해 빌드 시점 검증.
    // 사용자 지정 프로파일은 `install()` 안에서 동일한 검증을 debug_assert로 수행.
    const _: () = {
        assert!(STIMULUS_IMPACT_RATE > 0.0 && STIMULUS_IMPACT_RATE <= 1.0);
        assert!(STIMULUS_FADE_THRESHOLD < STIMULUS_MIN_INERTIA);
        assert!(STIMULUS_MIN_INERTIA > 0.0 && STIMULUS_MIN_INERTIA < 1.0);
        assert!(BEAT_MERGE_THRESHOLD > 0.0 && BEAT_MERGE_THRESHOLD < 1.0);

        assert!(CLOSENESS_UPDATE_RATE < TRUST_UPDATE_RATE);
        assert!(1.0 + 1.0 * SIGNIFICANCE_SCALE == 4.0);

        assert!(LEVEL_VERY_HIGH_THRESHOLD > LEVEL_HIGH_THRESHOLD);
        assert!(LEVEL_HIGH_THRESHOLD > LEVEL_LOW_THRESHOLD);
        assert!(LEVEL_LOW_THRESHOLD > LEVEL_VERY_LOW_THRESHOLD);

        assert!(LLM_TEMP_MIN < LLM_BASE_TEMPERATURE);
        assert!(LLM_BASE_TEMPERATURE < LLM_TEMP_MAX);
        assert!(LLM_TOP_P_MIN < LLM_BASE_TOP_P);
        assert!(LLM_BASE_TOP_P <= LLM_TOP_P_MAX);

        assert!(PAD_AXIS_DEAD_ZONE < PAD_AXIS_SCALE);
        assert!(PAD_AXIS_DEAD_ZONE >= 0.0);

        assert!(SOURCE_W_EXPERIENCED > SOURCE_W_WITNESSED);
        assert!(SOURCE_W_WITNESSED > SOURCE_W_HEARD);
        assert!(SOURCE_W_HEARD > SOURCE_W_RUMOR);

        assert!(RUMOR_HOP_CONFIDENCE_DECAY > 0.0 && RUMOR_HOP_CONFIDENCE_DECAY < 1.0);
        assert!(RUMOR_MIN_CONFIDENCE > 0.0 && RUMOR_MIN_CONFIDENCE < 1.0);

        assert!(MEMORY_RELATIONSHIP_DELTA_THRESHOLD >= CLOSENESS_UPDATE_RATE);
        assert!(MEMORY_RETENTION_CUTOFF > 0.0 && MEMORY_RETENTION_CUTOFF < 1.0);
        assert!(MEMORY_PUSH_TOP_K > 0);
        assert!(MEMORY_PROMPT_TOKEN_BUDGET > 0);

        assert!(SCENE_TASK_CHANNEL_CAPACITY > 0);
    };
}

// ---------------------------------------------------------------------------
// TuningProfile — 런타임 주입 가능한 정책 파라미터 묶음
// ---------------------------------------------------------------------------

/// 도메인 정책 파라미터 묶음. 프로세스 1회 `install()`로 주입하거나 `Default::default()` 사용.
///
/// JSON 직렬화를 지원하여 외부 설정 파일·시나리오 메타데이터에서 로드 가능.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TuningProfile {
    // === Stimulus ===
    /// 한 턴의 감정 변동량 제한 계수 (default 0.5)
    pub stimulus_impact_rate: f32,
    /// 감정 자연 소멸 기준 — 이 이하면 제거 (default 0.05)
    pub stimulus_fade_threshold: f32,
    /// 감정 관성 최소값 — intensity=1.0이어도 이만큼은 자극에 반응 (default 0.30)
    pub stimulus_min_inertia: f32,

    // === Beat ===
    /// Beat 합치기 시 이전 감정 소멸 기준 (default 0.2)
    pub beat_merge_threshold: f32,
    /// Beat 전환 시 기본 significance (default 0.5)
    pub beat_default_significance: f32,

    // === 관계 갱신 ===
    pub trust_update_rate: f32,
    pub closeness_update_rate: f32,
    pub significance_scale: f32,

    // === PAD ===
    pub pad_d_scale_weight: f32,
    pub pad_axis_dead_zone: f32,
    pub pad_axis_scale: f32,

    // === 가이드 ===
    pub mood_threshold: f32,
    pub honesty_restriction_threshold: f32,
    pub emotion_threshold: f32,
    pub trait_threshold: f32,

    // === 관계 변조 ===
    pub rel_closeness_intensity_weight: f32,
    pub rel_trust_emotion_weight: f32,
    pub rel_closeness_empathy_weight: f32,
    pub rel_closeness_hostility_weight: f32,

    // === Level 임계값 ===
    pub level_very_high_threshold: f32,
    pub level_high_threshold: f32,
    pub level_low_threshold: f32,
    pub level_very_low_threshold: f32,

    // === LLM 샘플링 ===
    pub llm_base_temperature: f32,
    pub llm_temp_openness_weight: f32,
    pub llm_temp_extraversion_weight: f32,
    pub llm_temp_conscientiousness_weight: f32,
    pub llm_temp_honesty_weight: f32,
    pub llm_base_top_p: f32,
    pub llm_top_p_openness_weight: f32,
    pub llm_top_p_conscientiousness_weight: f32,
    pub llm_temp_min: f32,
    pub llm_temp_max: f32,
    pub llm_top_p_min: f32,
    pub llm_top_p_max: f32,

    // === SceneTask ===
    pub scene_task_channel_capacity: usize,

    // === Memory (Step A) ===
    pub memory_retention_cutoff: f32,
    pub recall_boost_factor: f32,
    pub emotion_proximity_bonus: f32,
    pub recency_boost_tau_days: f32,
    pub similarity_cluster_threshold: f32,
    pub decay_tau_default_days: f32,

    // === Source 가중치 ===
    pub source_w_experienced: f32,
    pub source_w_witnessed: f32,
    pub source_w_heard: f32,
    pub source_w_rumor: f32,

    // === 프롬프트 예산 ===
    pub memory_push_top_k: usize,
    pub memory_prompt_token_budget: usize,

    // === Rumor ===
    pub rumor_hop_confidence_decay: f32,
    pub rumor_min_confidence: f32,

    // === Memory 저장 필터 ===
    pub memory_relationship_delta_threshold: f32,
}

impl Default for TuningProfile {
    fn default() -> Self {
        use defaults::*;
        Self {
            stimulus_impact_rate: STIMULUS_IMPACT_RATE,
            stimulus_fade_threshold: STIMULUS_FADE_THRESHOLD,
            stimulus_min_inertia: STIMULUS_MIN_INERTIA,
            beat_merge_threshold: BEAT_MERGE_THRESHOLD,
            beat_default_significance: BEAT_DEFAULT_SIGNIFICANCE,

            trust_update_rate: TRUST_UPDATE_RATE,
            closeness_update_rate: CLOSENESS_UPDATE_RATE,
            significance_scale: SIGNIFICANCE_SCALE,

            pad_d_scale_weight: PAD_D_SCALE_WEIGHT,
            pad_axis_dead_zone: PAD_AXIS_DEAD_ZONE,
            pad_axis_scale: PAD_AXIS_SCALE,

            mood_threshold: MOOD_THRESHOLD,
            honesty_restriction_threshold: HONESTY_RESTRICTION_THRESHOLD,
            emotion_threshold: EMOTION_THRESHOLD,
            trait_threshold: TRAIT_THRESHOLD,

            rel_closeness_intensity_weight: REL_CLOSENESS_INTENSITY_WEIGHT,
            rel_trust_emotion_weight: REL_TRUST_EMOTION_WEIGHT,
            rel_closeness_empathy_weight: REL_CLOSENESS_EMPATHY_WEIGHT,
            rel_closeness_hostility_weight: REL_CLOSENESS_HOSTILITY_WEIGHT,

            level_very_high_threshold: LEVEL_VERY_HIGH_THRESHOLD,
            level_high_threshold: LEVEL_HIGH_THRESHOLD,
            level_low_threshold: LEVEL_LOW_THRESHOLD,
            level_very_low_threshold: LEVEL_VERY_LOW_THRESHOLD,

            llm_base_temperature: LLM_BASE_TEMPERATURE,
            llm_temp_openness_weight: LLM_TEMP_OPENNESS_WEIGHT,
            llm_temp_extraversion_weight: LLM_TEMP_EXTRAVERSION_WEIGHT,
            llm_temp_conscientiousness_weight: LLM_TEMP_CONSCIENTIOUSNESS_WEIGHT,
            llm_temp_honesty_weight: LLM_TEMP_HONESTY_WEIGHT,
            llm_base_top_p: LLM_BASE_TOP_P,
            llm_top_p_openness_weight: LLM_TOP_P_OPENNESS_WEIGHT,
            llm_top_p_conscientiousness_weight: LLM_TOP_P_CONSCIENTIOUSNESS_WEIGHT,
            llm_temp_min: LLM_TEMP_MIN,
            llm_temp_max: LLM_TEMP_MAX,
            llm_top_p_min: LLM_TOP_P_MIN,
            llm_top_p_max: LLM_TOP_P_MAX,

            scene_task_channel_capacity: SCENE_TASK_CHANNEL_CAPACITY,

            memory_retention_cutoff: MEMORY_RETENTION_CUTOFF,
            recall_boost_factor: RECALL_BOOST_FACTOR,
            emotion_proximity_bonus: EMOTION_PROXIMITY_BONUS,
            recency_boost_tau_days: RECENCY_BOOST_TAU_DAYS,
            similarity_cluster_threshold: SIMILARITY_CLUSTER_THRESHOLD,
            decay_tau_default_days: DECAY_TAU_DEFAULT_DAYS,

            source_w_experienced: SOURCE_W_EXPERIENCED,
            source_w_witnessed: SOURCE_W_WITNESSED,
            source_w_heard: SOURCE_W_HEARD,
            source_w_rumor: SOURCE_W_RUMOR,

            memory_push_top_k: MEMORY_PUSH_TOP_K,
            memory_prompt_token_budget: MEMORY_PROMPT_TOKEN_BUDGET,

            rumor_hop_confidence_decay: RUMOR_HOP_CONFIDENCE_DECAY,
            rumor_min_confidence: RUMOR_MIN_CONFIDENCE,

            memory_relationship_delta_threshold: MEMORY_RELATIONSHIP_DELTA_THRESHOLD,
        }
    }
}

impl TuningProfile {
    /// 프로파일 일관성 검증. `install()` 안에서 호출되며,
    /// 위반 시 debug 빌드에선 panic, release 빌드에선 false 반환.
    ///
    /// 프로덕션에서 fail-fast가 필요한 호출자는 `install()`이 Err를 반환하도록 할 수 있다.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(self.stimulus_impact_rate > 0.0 && self.stimulus_impact_rate <= 1.0) {
            return Err("stimulus_impact_rate must be in (0, 1]");
        }
        if !(self.stimulus_fade_threshold < self.stimulus_min_inertia) {
            return Err("stimulus_fade_threshold must be < stimulus_min_inertia");
        }
        if !(self.stimulus_min_inertia > 0.0 && self.stimulus_min_inertia < 1.0) {
            return Err("stimulus_min_inertia must be in (0, 1)");
        }
        if !(self.beat_merge_threshold > 0.0 && self.beat_merge_threshold < 1.0) {
            return Err("beat_merge_threshold must be in (0, 1)");
        }
        if !(self.closeness_update_rate < self.trust_update_rate) {
            return Err("closeness_update_rate must be < trust_update_rate");
        }
        if !(self.level_very_high_threshold > self.level_high_threshold
            && self.level_high_threshold > self.level_low_threshold
            && self.level_low_threshold > self.level_very_low_threshold)
        {
            return Err("level thresholds must be strictly decreasing");
        }
        if !(self.llm_temp_min < self.llm_base_temperature
            && self.llm_base_temperature < self.llm_temp_max)
        {
            return Err("llm temperature: min < base < max");
        }
        if !(self.llm_top_p_min < self.llm_base_top_p && self.llm_base_top_p <= self.llm_top_p_max)
        {
            return Err("llm top_p: min < base <= max");
        }
        if !(self.source_w_experienced > self.source_w_witnessed
            && self.source_w_witnessed > self.source_w_heard
            && self.source_w_heard > self.source_w_rumor)
        {
            return Err("source weights must be strictly decreasing: Exp > Wit > Heard > Rumor");
        }
        if !(self.rumor_hop_confidence_decay > 0.0 && self.rumor_hop_confidence_decay < 1.0) {
            return Err("rumor_hop_confidence_decay must be in (0, 1)");
        }
        if !(self.memory_retention_cutoff > 0.0 && self.memory_retention_cutoff < 1.0) {
            return Err("memory_retention_cutoff must be in (0, 1)");
        }
        if self.memory_push_top_k == 0 {
            return Err("memory_push_top_k must be > 0");
        }
        if self.scene_task_channel_capacity == 0 {
            return Err("scene_task_channel_capacity must be > 0");
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 글로벌 OnceLock + install/profile API
// ---------------------------------------------------------------------------

static GLOBAL: OnceLock<TuningProfile> = OnceLock::new();

/// 에러: install이 이미 호출되어 프로파일이 고정됨.
#[derive(Debug)]
pub struct AlreadyInstalled(pub TuningProfile);

/// 에러: 프로파일 검증 실패 (validate에서 거부).
#[derive(Debug)]
pub struct InvalidProfile {
    pub profile: TuningProfile,
    pub reason: &'static str,
}

/// 프로세스 시작 시 1회 호출. 두 번째 호출은 `AlreadyInstalled` 에러.
///
/// `validate()` 통과한 프로파일만 설치된다.
pub fn install(profile: TuningProfile) -> Result<(), InstallError> {
    profile
        .validate()
        .map_err(|reason| InstallError::Invalid(InvalidProfile {
            profile: profile.clone(),
            reason,
        }))?;
    GLOBAL
        .set(profile)
        .map_err(|p| InstallError::AlreadyInstalled(AlreadyInstalled(p)))
}

/// install() 실패 사유.
#[derive(Debug)]
pub enum InstallError {
    AlreadyInstalled(AlreadyInstalled),
    Invalid(InvalidProfile),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyInstalled(_) => f.write_str("tuning profile already installed"),
            Self::Invalid(e) => write!(f, "invalid tuning profile: {}", e.reason),
        }
    }
}

impl std::error::Error for InstallError {}

/// 현재 설치된 프로파일 또는 default. 모든 도메인/애플리케이션 코드의 단일 진입점.
pub fn profile() -> &'static TuningProfile {
    GLOBAL.get_or_init(TuningProfile::default)
}

// ---------------------------------------------------------------------------
// 단위 테스트
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_passes_validate() {
        let p = TuningProfile::default();
        p.validate().expect("default must be valid");
    }

    #[test]
    fn level_thresholds_centered_around_zero() {
        let p = TuningProfile::default();
        assert!((p.level_high_threshold + p.level_low_threshold).abs() < 1e-6);
        assert!((p.level_very_high_threshold + p.level_very_low_threshold).abs() < 1e-6);
    }

    #[test]
    fn source_weights_in_unit_range() {
        let p = TuningProfile::default();
        for w in [
            p.source_w_experienced,
            p.source_w_witnessed,
            p.source_w_heard,
            p.source_w_rumor,
        ] {
            assert!(w > 0.0 && w <= 1.0, "source weight {w} out of (0, 1]");
        }
    }

    #[test]
    fn mood_emotion_trait_thresholds_in_unit_range() {
        let p = TuningProfile::default();
        for t in [
            p.mood_threshold,
            p.emotion_threshold,
            p.trait_threshold,
            p.honesty_restriction_threshold,
        ] {
            assert!(t > 0.0 && t < 1.0, "threshold {t} out of (0, 1)");
        }
    }

    #[test]
    fn validate_catches_invalid_stimulus_rate() {
        let mut p = TuningProfile::default();
        p.stimulus_impact_rate = 1.5;
        assert!(p.validate().is_err());
    }

    #[test]
    fn validate_catches_inverted_levels() {
        let mut p = TuningProfile::default();
        p.level_high_threshold = 0.8; // > VERY_HIGH(0.6)
        assert!(p.validate().is_err());
    }

    #[test]
    fn day_ms_is_one_day_in_milliseconds() {
        assert_eq!(DAY_MS, 24 * 60 * 60 * 1000);
    }
}
