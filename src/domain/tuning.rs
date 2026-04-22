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

// ---------------------------------------------------------------------------
// LLM 파라미터 유도 (Gemma 3 최적화)
// ---------------------------------------------------------------------------

/// 기본 Temperature 값
pub const LLM_BASE_TEMPERATURE: f32 = 0.8;

/// 개방성(O)에 의한 Temperature 변조 계수
pub const LLM_TEMP_OPENNESS_WEIGHT: f32 = 0.1;

/// 외향성(X)에 의한 Temperature 변조 계수
pub const LLM_TEMP_EXTRAVERSION_WEIGHT: f32 = 0.05;

/// 성실성(C)에 의한 Temperature 변조 계수 (차감)
pub const LLM_TEMP_CONSCIENTIOUSNESS_WEIGHT: f32 = 0.1;

/// 정직-겸손성(H)에 의한 Temperature 변조 계수 (차감)
pub const LLM_TEMP_HONESTY_WEIGHT: f32 = 0.05;

/// 기본 Top P 값
pub const LLM_BASE_TOP_P: f32 = 0.9;

/// 개방성(O)에 의한 Top P 변조 계수
pub const LLM_TOP_P_OPENNESS_WEIGHT: f32 = 0.05;

/// 성실성(C)에 의한 Top P 변조 계수 (차감)
pub const LLM_TOP_P_CONSCIENTIOUSNESS_WEIGHT: f32 = 0.05;

/// Temperature 최소값
pub const LLM_TEMP_MIN: f32 = 0.1;

/// Temperature 최대값
pub const LLM_TEMP_MAX: f32 = 1.1;

/// Top P 최소값
pub const LLM_TOP_P_MIN: f32 = 0.8;

/// Top P 최대값
pub const LLM_TOP_P_MAX: f32 = 1.0;

// ============================================================================
// Director / SceneTask (B4 Session 4)
// ============================================================================

/// SceneTask mpsc 채널 capacity.
///
/// Scene 당 한 task가 커맨드를 순차 소비한다. caller가 `dispatch_to`로 송신한 커맨드는
/// 이 버퍼에 적재된다. 버퍼가 꽉 차면 `dispatch_to`의 `send().await`가 backpressure로
/// 대기한다.
///
/// 현재 값(32)은 "플레이어가 연속으로 커맨드를 폭주 입력해도 짧은 시간 동안 SceneTask가
/// 처리해 공간이 생기는" 경험값. 원격 LLM 호출이 수초간 지연되는 상황에서 backpressure가
/// 발생할 수 있으며, 이 경우 caller가 backpressure를 포용하거나 용량을 늘려 조정한다.
pub const SCENE_TASK_CHANNEL_CAPACITY: usize = 32;

// ============================================================================
// Memory (RAG · Ranker · Retention) — Step A
// ============================================================================
//
// 3차 설계 §9. 값은 초기 제안값이며 실측·튜닝 후 조정한다.

/// 하루 (ms) — retention 나이 계산 기준
pub const DAY_MS: u64 = 86_400_000;

/// Ranker 검색 제외 한계 — retention이 이 미만이면 탈락
pub const MEMORY_RETENTION_CUTOFF: f32 = 0.10;

/// recall_count 기반 retention 강화 계수 — `1 + ln1p(recall_count) × factor`
pub const RECALL_BOOST_FACTOR: f32 = 0.15;

/// 감정 근접(PAD cosine) 보너스 상한 — `1 + cos × bonus`
pub const EMOTION_PROXIMITY_BONUS: f32 = 0.30;

/// 최근성 부스트 수명 (days) — `exp(-age_days / τ_recency)`
pub const RECENCY_BOOST_TAU_DAYS: f32 = 3.0;

/// Ranker 1단계 Topic-없는 클러스터링 기준 (코사인 유사도)
pub const SIMILARITY_CLUSTER_THRESHOLD: f32 = 0.85;

/// τ 룩업 기본값 (days) — 미매핑 조합에 적용
pub const DECAY_TAU_DEFAULT_DAYS: f32 = 30.0;

// === Source 가중치 (MemoryRanker 2단계) ===
pub const SOURCE_W_EXPERIENCED: f32 = 1.00;
pub const SOURCE_W_WITNESSED: f32 = 0.85;
pub const SOURCE_W_HEARD: f32 = 0.60;
pub const SOURCE_W_RUMOR: f32 = 0.35;

// === 프롬프트 예산 (Step B 주입용 · Step A에서는 상수만 정의) ===
pub const MEMORY_PUSH_TOP_K: usize = 5;
pub const MEMORY_PROMPT_TOKEN_BUDGET: usize = 400;

// === Rumor 감쇠 (Step C 이후 사용) ===
pub const RUMOR_HOP_CONFIDENCE_DECAY: f32 = 0.8;
pub const RUMOR_MIN_CONFIDENCE: f32 = 0.1;

// === Memory 저장 필터 (Step D 이후 사용) ===
/// 관계 변화 기록 하한 — closeness/trust/power 중 최대 Δ가 이 값 미만이면
/// `RelationshipMemoryHandler`는 MemoryEntry를 남기지 않는다. 미세 변동 폭증 방지.
pub const MEMORY_RELATIONSHIP_DELTA_THRESHOLD: f32 = 0.05;
