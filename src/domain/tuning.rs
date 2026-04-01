//! 튜닝 상수 — 감정 엔진의 조정 가능한 파라미터
//!
//! 모든 수치 튜닝 대상을 한 곳에 모아 관리.
//! 플레이테스트로 조정할 때 이 파일만 보면 됨.

// ---------------------------------------------------------------------------
// Stimulus (대사 자극)
// ---------------------------------------------------------------------------

/// 한 턴의 감정 변동량 제한 계수
pub const STIMULUS_IMPACT_RATE: f32 = 0.5;

/// 감정 자연 소멸 기준 (이 이하면 제거)
pub const STIMULUS_FADE_THRESHOLD: f32 = 0.05;

/// 감정 관성 최소값 — intensity=1.0이어도 이만큼은 자극에 반응
pub const STIMULUS_MIN_INERTIA: f32 = 0.30;

// ---------------------------------------------------------------------------
// Beat 전환
// ---------------------------------------------------------------------------

/// Beat 합치기 시 이전 감정 소멸 기준 (이 미만이면 제거)
pub const BEAT_MERGE_THRESHOLD: f32 = 0.2;

// ---------------------------------------------------------------------------
// 관계 갱신
// ---------------------------------------------------------------------------

/// trust 갱신 계수 (대화 후, praiseworthiness 기반)
pub const TRUST_UPDATE_RATE: f32 = 0.1;

/// closeness 갱신 계수 (대화 후, 전체 감정 valence 기반)
pub const CLOSENESS_UPDATE_RATE: f32 = 0.05;

/// 상황 중요도(significance)에 의한 최대 배율 증가분.
/// significance=1.0일 때 변동 폭이 (1 + SIGNIFICANCE_SCALE)배 = 4배.
pub const SIGNIFICANCE_SCALE: f32 = 3.0;

// ---------------------------------------------------------------------------
// PAD (Pleasure-Arousal-Dominance)
// ---------------------------------------------------------------------------

/// D축 격차의 스케일러 가중치
pub const PAD_D_SCALE_WEIGHT: f32 = 0.3;

/// PAD 축 점수 데드존 — 이 미만의 |차이|는 0.0 처리
pub const PAD_AXIS_DEAD_ZONE: f32 = 0.02;

/// PAD 축 점수 스케일링 배율 — 데드존 적용 후 곱셈
pub const PAD_AXIS_SCALE: f32 = 3.0;

// ---------------------------------------------------------------------------
// 가이드 (LLM 연기 지시)
// ---------------------------------------------------------------------------

/// 기분(mood) 분기 임계값 — 이 이상이면 긍정/부정 어조 분기
pub const MOOD_THRESHOLD: f32 = 0.3;

/// 정직-겸손 차원이 이 이하면 "거짓말 금지" 제약 해제
pub const HONESTY_RESTRICTION_THRESHOLD: f32 = 0.5;

/// 감정의 유의미 판단 기준 (이 이상이면 연기에 반영)
pub const EMOTION_THRESHOLD: f32 = 0.2;

/// 성격 특성 추출 임계값 (차원 평균이 이 이상이면 두드러진 특성으로 판단)
pub const TRAIT_THRESHOLD: f32 = 0.3;

// ---------------------------------------------------------------------------
// 관계 변조 계수
// ---------------------------------------------------------------------------

/// closeness → 감정 강도 배율 (1.0 + closeness × 이 값)
pub const REL_CLOSENESS_INTENSITY_WEIGHT: f32 = 0.5;

/// trust → 행동 평가 배율 (1.0 + trust × 이 값)
pub const REL_TRUST_EMOTION_WEIGHT: f32 = 0.3;

/// closeness → 공감(HappyFor/Pity) 배율 (1.0 + closeness × 이 값)
pub const REL_CLOSENESS_EMPATHY_WEIGHT: f32 = 0.3;

/// closeness → 적대(Resentment/Gloating) 배율 (1.0 - closeness × 이 값)
pub const REL_CLOSENESS_HOSTILITY_WEIGHT: f32 = 0.3;

// ---------------------------------------------------------------------------
// 레벨 분류 임계값 (RelationshipLevel, PowerLevel)
// ---------------------------------------------------------------------------

/// VeryHigh 분기 기준 (이 초과)
pub const LEVEL_VERY_HIGH_THRESHOLD: f32 = 0.6;

/// High 분기 기준 (이 초과)
pub const LEVEL_HIGH_THRESHOLD: f32 = 0.2;

/// Low 분기 기준 (이 초과)
pub const LEVEL_LOW_THRESHOLD: f32 = -0.2;

/// VeryLow 분기 기준 (이 이하)
pub const LEVEL_VERY_LOW_THRESHOLD: f32 = -0.6;

// ---------------------------------------------------------------------------
// Beat 전환
// ---------------------------------------------------------------------------

/// Beat 전환 시 기본 significance 값
pub const BEAT_DEFAULT_SIGNIFICANCE: f32 = 0.5;
