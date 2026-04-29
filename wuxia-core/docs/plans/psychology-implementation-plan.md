# Psychology Domain Implementation Plan (Iterations 2.4~2.7)

> **Date:** 2026-03-01
> **Status:** ✅ 전체 완료 (2026-03-03 확인)
> **Tests:** 207 tests (wuxia-core psychology 모듈 — personality, three_axis, values, emotion, mood, filter, appraisal, decay, preset)
> **Scope:** wuxia-core psychology module — 코드 우선, 문서는 최소(CLAUDE.md)만 업데이트

## 개요

설계 문서(`docs/psychology/` 10개 파일)를 기반으로 `wuxia-core/src/psychology/` 모듈을 구현한다.
7층 심리 아키텍처 중 ①~⑤층을 Rust 타입과 순수 함수로 구현하고,
⑥행동, ⑦성찰은 타입 정의만 포함한다 (LLM 연동은 Phase 5).

## 모듈 구조

```
crates/wuxia-core/src/psychology/
├── mod.rs                 # 모듈 루트 + re-exports + PsychologyError
├── personality.rs         # ①층: HexacoPersonality (6요소, 0~100)
├── personality_tests.rs   # HEXACO 테스트 (경계값, 프리셋, 변경규칙)
├── three_axis.rs          # ②층: ThreeAxisValues + ValueAxis + CreedCandidate
├── three_axis_tests.rs    # 3축가치관 테스트
├── values.rs              # ③층: PracticalValues (5가치, 0.0~100.0)
├── values_tests.rs        # 5가치 테스트
├── emotion.rs             # ④층: EmotionType (22종) + ActiveEmotion
├── emotion_tests.rs       # 감정 테스트 (생성, 감쇠, 필터)
├── mood.rs                # ⑤층: PadState (-1.0~+1.0)
├── mood_tests.rs          # PAD 테스트
├── filter.rs              # HEXACO → 감정 필터 순수 함수
├── filter_tests.rs        # 필터 공식 테스트 (명경, 조고 시나리오)
├── appraisal.rs           # OCC 인지 평가: OccStimulus + OccAppraisal + appraise()
├── appraisal_tests.rs     # 인지 평가 테스트
├── decay.rs               # 감정 감쇠 순수 함수 + 반감기 상수
├── event.rs               # PsychologyEvent enum
├── preset.rs              # 6 NPC 프리셋 (HEXACO + 3축 + 5가치)
└── preset_tests.rs        # 프리셋 검증 테스트
```

## Iteration 2.4 — 3축가치관 + 5가치 + 모듈 골격

### Step 1: 모듈 골격 생성
- `psychology/mod.rs` 생성 (re-exports, PsychologyError)
- `lib.rs`에 `pub mod psychology;` 추가
- `shared/event.rs`에 `Psychology(PsychologyEvent)` variant 추가
- `shared/event.rs`에 `From<PsychologyEvent>` 구현
- `psychology/event.rs` 생성 (PsychologyEvent enum — 초기 variant)

### Step 2: ②층 — ThreeAxisValues
**파일:** `three_axis.rs`

```rust
pub struct ValueAxis {
    intensity: f32,                         // 0.0~100.0
    creed: String,                          // 현재 신조
    creed_candidates: Vec<CreedCandidate>,  // 대안 후보
    formation_memories: Vec<MemoryId>,      // 형성 기억
}

pub struct CreedCandidate {
    text: String,
    source: String,
    exposure_count: u32,
    resonance: f32,  // 0.0~100.0
}

pub struct ThreeAxisValues {
    character_id: CharacterId,
    trust: ValueAxis,       // 믿음(信)
    rightness: ValueAxis,   // 옳음(正)
    want: ValueAxis,        // 바람(願)
}
```

**주요 메서드:**
- `new(character_id, trust, rightness, want)` → Self
- `adjust_intensity(axis, delta, tier)` → Vec<PsychologyEvent>
  - Tier별 범위 클램핑: Tier1: ±5, Tier2: ±10, Tier3: ±20, Tier4: ±30
- `update_creed(axis, new_creed, reason)` → Vec<PsychologyEvent>
- `add_creed_candidate(axis, candidate)` → Vec<PsychologyEvent>
- `increment_exposure(axis, candidate_idx)` → 접촉 횟수 증가
- `add_formation_memory(axis, memory_id)` → 형성기억 추가
- Getters: `trust()`, `rightness()`, `want()`, `axis(AxisType)`

### Step 3: ③층 — PracticalValues
**파일:** `values.rs`

```rust
pub struct PracticalValues {
    character_id: CharacterId,
    loyalty: f32,        // 충(忠) 0.0~100.0
    righteousness: f32,  // 의(義) 0.0~100.0
    filial_piety: f32,   // 효(孝) 0.0~100.0
    vengeance: f32,      // 복수(復) 0.0~100.0
    ambition: f32,       // 야망(野) 0.0~100.0
}

pub enum PracticalValueType {
    Loyalty,
    Righteousness,
    FilialPiety,
    Vengeance,
    Ambition,
}
```

**주요 메서드:**
- `new(character_id, loyalty, righteousness, filial_piety, vengeance, ambition)` → Self
- `adjust(value_type, delta, tier)` → Vec<PsychologyEvent>
  - Tier별 범위: Tier1: ±5, Tier2: ±10, Tier3: ±20, Tier4: ±20
- `get(value_type)` → f32
- `alignment()` → f32 (충+의+효 vs 복수+야망 균형)
- `betrayal_potential()` → f32 (야망 × (1 - 충/100) × (1 - 의/100))

### Step 4: PsychologyEvent (초기)
```rust
pub enum PsychologyEvent {
    AxisIntensityChanged { character_id, axis: AxisType, old: f32, new: f32, tier: ReflectionTier },
    CreedChanged { character_id, axis: AxisType, old_creed: String, new_creed: String },
    PracticalValueChanged { character_id, value_type: PracticalValueType, old: f32, new: f32, tier },
}
```

### 테스트 (~60개 예상)
- 경계값: intensity 0, 100, 음수→0 클램프, 100초과→100 클램프
- Tier별 범위 제한: Tier1 ±5 초과 시 클램프
- 신조 변경 이벤트 발생 확인
- 대안 후보 접촉 횟수 증가
- NPC 시나리오: 소연 복수 가치 상승 → alignment 변화

---

## Iteration 2.5 — HEXACO 성격

### Step 1: ①층 — HexacoPersonality
**파일:** `personality.rs`

```rust
pub struct HexacoPersonality {
    character_id: CharacterId,
    honesty_humility: u32,   // H: 0~100
    emotionality: u32,       // E: 0~100
    extraversion: u32,       // X: 0~100
    agreeableness: u32,      // A: 0~100
    conscientiousness: u32,  // C: 0~100
    openness: u32,           // O: 0~100
}

pub enum HexacoFactor {
    HonestyHumility,
    Emotionality,
    Extraversion,
    Agreeableness,
    Conscientiousness,
    Openness,
}
```

**주요 메서드:**
- `new(character_id, h, e, x, a, c, o)` → Self (각 값 0~100 클램프)
- `apply_tier4_change(changes: &[(HexacoFactor, i32)])` → Result<Vec<PsychologyEvent>, PsychologyError>
  - 최대 2개 factor만 변경 가능, 각 ±5 제한
  - 위반 시 PsychologyError::PersonalityChangeExceeded
- `get(factor)` → u32
- `emotional_reactivity()` → f32 (E 기반)
- `anger_suppression()` → f32 (A 기반)
- `moral_sensitivity()` → f32 (H 기반)

**변경 규칙 (엄격):**
- Tier 1~3: 성격 변경 불가 (오직 Tier 4)
- Tier 4: 최대 2개 factor, 각 ±5
- 나이 드리프트 없음

### Step 2: HEXACO 필터 함수
**파일:** `filter.rs`

```rust
// H 필터 — 도덕 감정
pub fn h_guilt_filter(h: u32) -> f32    // × (1.0 + H × 0.005)
pub fn h_shame_filter(h: u32) -> f32    // × (1.0 + H × 0.005)
pub fn h_gloating_filter(h: u32) -> f32 // × (1.0 - H × 0.004)
pub fn h_reproach_filter(h: u32) -> f32 // × (1.0 + H × 0.003)

// E 필터 — 공포/공감
pub fn e_fear_filter(e: u32) -> f32     // × (1.0 + E × 0.005)
pub fn e_pity_filter(e: u32) -> f32     // × (1.0 + E × 0.004)

// A 필터 — 분노/원한 조절
pub fn a_anger_filter(a: u32) -> f32    // × (1.0 - A × 0.004)
pub fn a_resentment_filter(a: u32) -> f32 // × (1.0 - A × 0.003)

// 통합: 감정 타입 + HEXACO → 필터 계수
pub fn hexaco_emotion_filter(emotion: &EmotionType, personality: &HexacoPersonality) -> f32
```

### Step 3: NPC 프리셋
**파일:** `preset.rs`

```rust
pub fn myungkyung_personality() → HexacoPersonality  // H90 E50 X50 A80 C90 O60
pub fn jogo_personality() → HexacoPersonality         // H10 E20 X80 A10 C80 O50
pub fn soyeon_personality() → HexacoPersonality       // H50 E60 X60 A40 C70 O70
pub fn yalul_personality() → HexacoPersonality        // H40 E30 X70 A50 C40 O80
pub fn jinya_personality() → HexacoPersonality        // H60 E40 X30 A60 C30 O50
pub fn namgung_personality() → HexacoPersonality      // H40 E50 X70 A30 C70 O60

// 3축 + 5가치 프리셋도 함께
pub fn myungkyung_three_axis() → ThreeAxisValues
pub fn myungkyung_values() → PracticalValues
// ... 6 NPC 각각
```

### PsychologyEvent 추가
```rust
PersonalityChanged { character_id, factor: HexacoFactor, old: u32, new: u32 },
```

### 테스트 (~80개 예상)
- 생성: 각 factor 0~100 범위, 초과 시 클램프
- Tier4 변경: 2개 이하 factor, ±5 제한
- Tier4 위반: 3개 factor → Error, ±6 → 클램프/Error
- 필터 함수: H=90→Guilt×1.45, H=10→Guilt×1.05 (문서 예시 검증)
- 명경 시나리오: H90×A80 → 분노 극도로 억제
- 조고 시나리오: H10×A10 → 분노 거의 억제 안됨
- 프리셋 검증: 6 NPC 각각 값 확인

---

## Iteration 2.6 — OCC 감정 + PAD 기분

### Step 1: ④층 — EmotionType + ActiveEmotion
**파일:** `emotion.rs`

```rust
pub enum EmotionType {
    // Event Consequence — Well-being
    Joy,         // 희열
    Distress,    // 고뇌
    // Event Consequence — Prospect-based
    Hope,        // 기대
    Fear,        // 두려움
    Satisfaction,// 흡족
    FearsConfirmed, // 절망
    Relief,      // 안도
    Disappointment, // 실망
    // Event Consequence — Fortunes-of-others
    HappyFor,    // 축하
    Pity,        // 측은
    Gloating,    // 통쾌
    Resentment,  // 시기
    // Agent Action
    Pride,       // 자부
    Shame,       // 수치
    Admiration,  // 감탄
    Reproach,    // 비난
    // Compound
    Gratification, // 뿌듯함 (Pride + Joy)
    Remorse,     // 회한 (Shame + Distress)
    Gratitude,   // 감은 (Admiration + Joy)
    Anger,       // 분노 (Reproach + Distress)
    // Object Aspect
    Love,        // 애착
    Hate,        // 혐오
}

pub enum EmotionCategory {
    EventConsequence,
    AgentAction,
    ObjectAspect,
    Compound,
}

pub struct ActiveEmotion {
    emotion_type: EmotionType,
    intensity: f32,               // 0.0~100.0
    source_event_description: String,
    source_agent: Option<CharacterId>,
    created_at: GameTime,
}
```

**주요 함수/메서드:**
- `EmotionType::category()` → EmotionCategory
- `EmotionType::valence()` → Valence (Positive/Negative)
- `EmotionType::half_life_hours()` → f32 (상수 테이블)
- `EmotionType::pad_delta()` → (f32, f32, f32) (ΔP, ΔA, ΔD 최대값)
- `ActiveEmotion::is_expired(threshold)` → bool (intensity < threshold)

### Step 2: 감정 감쇠
**파일:** `decay.rs`

```rust
/// 감정 감쇠 — 반감기 기반 지수 감쇠
/// intensity(t) = intensity₀ × e^(-λ × Δt), λ = ln(2) / half_life
pub fn decay_emotion(intensity: f32, half_life_hours: f32, elapsed_hours: f32) -> f32

/// 감정 목록에서 만료된 감정 제거 (threshold 미만)
pub fn cleanup_expired(emotions: &mut Vec<ActiveEmotion>, threshold: f32)
```

**반감기 상수 (설계 문서 기준):**
| 감정 | 반감기(시간) |
|------|-------------|
| Relief, Satisfaction, Gloating, HappyFor | 2~3 |
| Joy, Pride, Admiration | 4~6 |
| Distress, Gratification, Disappointment | 8 |
| Hope, Fear | 12 |
| Anger, Shame, Resentment, Reproach | 24 |
| FearsConfirmed | 36 |
| Remorse | 48 |
| Love, Hate | ∞ (no decay) |

### Step 3: ⑤층 — PadState
**파일:** `mood.rs`

```rust
pub struct PadState {
    pleasure: f32,    // P: -1.0 ~ +1.0
    arousal: f32,     // A: -1.0 ~ +1.0
    dominance: f32,   // D: -1.0 ~ +1.0
}
```

**주요 메서드:**
- `new(p, a, d)` → Self (클램프)
- `neutral()` → Self (0, 0, 0)
- `apply_emotion(emotion_type, intensity)` → PadState
  - ΔP_actual = ΔP_max × (intensity / 100)
  - 각 축 클램프 -1.0~+1.0
- `decay_toward_neutral(rate)` → PadState (자연 감쇠)
- `mood_bias()` → f32 (P 기반 감정 편향 계수)
- `is_extreme()` → bool (|P|>0.8 or |A|>0.8 → Tier3 트리거)

### PsychologyEvent 추가
```rust
EmotionGenerated { character_id, emotion_type: EmotionType, intensity: f32 },
EmotionDecayed { character_id, emotion_type: EmotionType, old_intensity: f32, new_intensity: f32 },
EmotionExpired { character_id, emotion_type: EmotionType },
MoodChanged { character_id, old: PadState, new: PadState },
```

### 테스트 (~100개 예상)
- 22 감정 타입: category(), valence(), half_life(), pad_delta() 각각 검증
- 감쇠: 반감기 경과 → 강도 50%, 2반감기 → 25%
- Love/Hate: 감쇠 없음 검증
- PAD: 감정 적용 후 클램프 확인
- PAD extreme 판정: |P|>0.8 → Tier3 트리거
- 시나리오: 명경이 제자 납치 소식 → Anger(81) + Fear(72) → PAD 변화

---

## Iteration 2.7 — OCC 인지 평가

### Step 1: 평가 입력 타입
**파일:** `appraisal.rs`

```rust
pub enum OccStimulus {
    /// 사건의 결과 (목표 관련성으로 평가)
    EventConsequence {
        description: String,
        is_prospective: bool,               // 전망 vs 확정
        concerns_other: Option<CharacterId>, // 타인운 vs 자기
    },
    /// 행위자의 행동 (기준 부합으로 평가)
    AgentAction {
        agent_id: CharacterId,
        is_self: bool,
    },
    /// 대상의 속성 (호감으로 평가)
    ObjectAspect {
        description: String,
        familiarity: f32,  // 0.0~100.0
    },
}

pub struct OccAppraisal {
    pub stimulus: OccStimulus,
    pub desirability: f32,       // -1.0 ~ +1.0
    pub praiseworthiness: f32,   // -1.0 ~ +1.0
    pub appealingness: f32,      // -1.0 ~ +1.0
    pub relevant_values: Vec<(PracticalValueType, f32)>,  // (가치, 관여도 0~1)
}
```

### Step 2: 평가 → 감정 변환 순수 함수
```rust
/// OCC 인지 평가 결과를 감정으로 변환
/// 핵심 공식: emotion_intensity = |appraisal| × value_weight × mood_bias × hexaco_filter
pub fn appraise_to_emotions(
    appraisal: &OccAppraisal,
    values: &PracticalValues,
    personality: &HexacoPersonality,
    mood: &PadState,
) -> Vec<(EmotionType, f32)>
```

**변환 규칙 (설계 문서 기준):**

1. **EventConsequence (사건):**
   - desirability > 0 + 확정: Joy
   - desirability < 0 + 확정: Distress
   - desirability > 0 + 전망: Hope
   - desirability < 0 + 전망: Fear
   - concerns_other + desirability > 0: HappyFor
   - concerns_other + desirability < 0: Pity (또는 Gloating if hostile)

2. **AgentAction (행동):**
   - praiseworthiness > 0 + is_self: Pride
   - praiseworthiness < 0 + is_self: Shame
   - praiseworthiness > 0 + !is_self: Admiration
   - praiseworthiness < 0 + !is_self: Reproach

3. **ObjectAspect (대상):**
   - appealingness > 0: Love
   - appealingness < 0: Hate

4. **Compound 감정:**
   - Reproach + Distress → Anger (분노)
   - Admiration + Joy → Gratitude (감은)
   - Pride + Joy → Gratification (뿌듯함)
   - Shame + Distress → Remorse (회한)

**강도 계산 공식:**
```
base_intensity = |appraisal_value| × 100  // 0~100으로 스케일링
value_weight = Σ(value × relevance) / 100  // 관련 가치 가중
mood_bias = 1.0 + mood.pleasure() × 0.3    // P가 긍정이면 긍정감정↑
personality_filter = hexaco_emotion_filter(emotion_type, personality)

final_intensity = (base_intensity × value_weight × mood_bias × personality_filter).clamp(0.0, 100.0)
```

### Step 3: Tier/ReflectionTier 타입
```rust
pub enum ReflectionTier {
    Instant,      // Tier 1: 순간 반응 (코드, <1ms)
    Daily,        // Tier 2: 일상 성찰 (LLM, 하루 1회)
    TurningPoint, // Tier 3: 전환점 성찰 (LLM, 중대 사건)
    Life,         // Tier 4: 인생 성찰 (LLM, 매우 드묾)
}
```

### PsychologyEvent 추가
```rust
AppraisalCompleted { character_id, stimulus_desc: String, emotions: Vec<(EmotionType, f32)> },
```

### 테스트 (~70개 예상)
- 사건 평가: desirability +0.8 → Joy, -0.8 → Distress
- 행동 평가: 칭찬할 자기 행동 → Pride, 비난할 타인 행동 → Reproach
- 복합 감정: Reproach + Distress → Anger 생성 확인
- 가치 가중: 의(90) × 위반(0.95) → 분노 강도 85+
- HEXACO 필터 적용: 명경(A80) → 분노 ×0.68
- 무협 시나리오: 명경 - 제자 납치 → Anger(~55 after A filter) + Fear
- 무협 시나리오: 조고 - 모욕 → Anger(~96, A10 filter minimal)

---

## 공통 인프라 변경

### shared/event.rs
```rust
// 추가:
Psychology(PsychologyEvent),

// name() match arm 추가
// From<PsychologyEvent> 구현
```

### shared/id.rs
- 새 ID 불필요 (CharacterId 사용)

### lib.rs
```rust
pub mod psychology;  // 추가
```

### test_fixtures.rs
```rust
pub fn make_hexaco(id: u64, h: u32, e: u32, x: u32, a: u32, c: u32, o: u32) -> HexacoPersonality
pub fn make_default_psyche(id: u64) -> (HexacoPersonality, ThreeAxisValues, PracticalValues)
pub fn make_myungkyung_psyche() -> (HexacoPersonality, ThreeAxisValues, PracticalValues)
```

## 테스트 목표

총 ~310개 테스트 예상:
- Iteration 2.4: ~60 (3축 + 5가치)
- Iteration 2.5: ~80 (HEXACO + 필터 + 프리셋)
- Iteration 2.6: ~100 (22감정 + 감쇠 + PAD)
- Iteration 2.7: ~70 (인지 평가 + 복합 감정)

## 구현 순서

1. `mod.rs` + `event.rs` (골격)
2. `values.rs` + 테스트 (③층 — 가장 단순)
3. `three_axis.rs` + 테스트 (②층)
4. `personality.rs` + 테스트 (①층)
5. `filter.rs` + 테스트 (①→④ 연결)
6. `preset.rs` + 테스트 (6 NPC)
7. `emotion.rs` + `decay.rs` + 테스트 (④층)
8. `mood.rs` + 테스트 (⑤층)
9. `appraisal.rs` + 테스트 (④+③+①+⑤ 통합)
10. DomainEvent 연결 + lib.rs + test_fixtures.rs
11. CLAUDE.md 업데이트
12. `cargo test -p wuxia-core` 전체 통과 확인
