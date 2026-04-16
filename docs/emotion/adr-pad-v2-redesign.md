# ADR: PAD 자극 시스템 v2 재설계

**상태**: 제안 (Proposed)  
**날짜**: 2026-04-11  
**작성자**: Claude + Bekay  

---

## 1. 배경 및 문제

현재 PAD 시스템(v1)은 bge-m3 임베딩 + 앵커 cosine similarity로 대사의 감정 톤을 추출한다. 운용 과정에서 드러난 근본적 한계:

### 1.1 맥락 무시

임베딩은 대사 텍스트만 보고 PAD를 추출한다. 동일한 "알겠습니다"가 냉랭한 수락인지, 따뜻한 동의인지, 굴복인지 구분 불가. 무협에서 "좋다"라는 한마디가 진심인 칭찬인지 살기 어린 위협인지는 대화 맥락이 결정한다.

### 1.2 조잡한 축 분리

축당 양극 10문장의 평균 벡터는 P/A/D 축을 깨끗하게 분리하지 못한다. "배은망덕한 놈!"은 P- 이면서 A+이고 D+인데, pleasure negative 앵커로만 배치되어 다른 축에 노이즈를 준다. 범용 임베딩 모델(bge-m3)의 벡터 공간에서 P/A/D가 직교한다는 보장이 없다.

### 1.3 새 감정 생성 불가

`apply_stimulus`는 기존 감정의 강도만 변동시키고 새 감정을 만들지 못한다. Joy만 있는 NPC에게 충격적인 배신 소식을 전해도 Fear나 Anger가 생겨나지 않는다 — Joy가 줄어들 뿐이다. 이는 대화 중 감정의 폭을 심하게 제한한다.

### 1.4 단일 턴 독립 처리

연속된 도발, 점진적 압박, 반복되는 위로 등 대화 흐름의 누적 효과가 없다. 3번째 도발이 1번째와 동일한 영향력을 가진다.

---

## 2. 결정

네 가지를 함께 개선한다:

| # | 변경 | 요약 |
|---|------|------|
| A | **LLM 기반 PAD 추출** | 임베딩 파이프라인을 LLM 호출로 대체 |
| B | **새 감정 생성** | 강한 자극 시 기존에 없던 감정이 태어날 수 있도록 |
| C | **자극 모멘텀** | 연속된 비슷한 자극이 점점 강해지는 효과 |
| D | **포트/DTO 확장** | 맥락 전달 + 결과 확장 |

---

## 3. 상세 설계

### 3.1 LLM 기반 PAD 추출 (변경 A)

#### 현재 흐름

```
대사 텍스트 → [bge-m3 임베딩] → [앵커 cosine sim] → Pad {P, A, D}
```

#### 새 흐름

```
대사 텍스트 + 대화 맥락 + NPC 상태
  → [LLM: structured output] → PadAnalysis { pad, reasoning, suggested_emotions }
```

#### UtteranceAnalyzer 포트 확장

```rust
/// 대사 분석 맥락 — LLM이 대사의 감정 톤을 판단하는 데 필요한 정보
pub struct AnalysisContext {
    /// 최근 대화 히스토리 (최대 N턴)
    pub recent_turns: Vec<DialogueTurn>,
    /// NPC의 현재 감정 상태 요약
    pub current_emotions: Vec<(EmotionType, f32)>,
    /// NPC와 화자의 관계 요약
    pub relationship_summary: String,
    /// 현재 장면/상황 설명
    pub scene_context: Option<String>,
}

/// 대사 분석 결과 — PAD + 맥락 기반 추가 정보
pub struct PadAnalysis {
    /// 추출된 PAD 좌표
    pub pad: Pad,
    /// 분석 근거 (디버깅/로깅용)
    pub reasoning: String,
    /// 이 자극이 촉발할 수 있는 새 감정 제안
    /// LLM이 맥락상 강한 감정 전환을 감지했을 때만 포함
    pub suggested_emotions: Vec<SuggestedEmotion>,
}

pub struct SuggestedEmotion {
    pub emotion_type: EmotionType,
    pub initial_intensity: f32,  // 0.2~0.5 범위 제안
    pub reason: String,
}

/// 대사 → PAD 변환 포트 (v2: 맥락 지원)
pub trait UtteranceAnalyzer {
    /// 기존 API (하위 호환) — 맥락 없이 분석
    fn analyze(&mut self, utterance: &str) -> Result<Pad, AnalysisError>;
    
    /// 맥락 포함 분석 — LLM 기반 구현에서 사용
    fn analyze_with_context(
        &mut self,
        utterance: &str,
        context: &AnalysisContext,
    ) -> Result<PadAnalysis, AnalysisError>;
}
```

#### LLM 프롬프트 설계

```
당신은 무협 세계관의 감정 분석 전문가입니다.
상대방의 대사가 NPC에게 주는 감정적 자극을 PAD 좌표로 평가하세요.

## NPC 현재 상태
- 감정: {emotions}
- 관계: {relationship}
- 상황: {scene_context}

## 최근 대화
{recent_turns}

## 분석할 대사
"{utterance}"

## 출력 형식 (JSON)
{
  "pleasure": float,     // -1.0(불쾌) ~ +1.0(쾌적). 이 대사가 NPC에게 주는 쾌/불쾌 느낌
  "arousal": float,      // -1.0(차분) ~ +1.0(흥분). 이 대사가 NPC의 각성을 얼마나 올리는지
  "dominance": float,    // -1.0(위축) ~ +1.0(지배). 이 대사가 얼마나 권위적/강압적인지
  "reasoning": string,   // 판단 근거 1문장
  "new_emotions": [      // 기존에 없던 감정이 새로 생겨야 할 경우만
    { "type": "Fear", "intensity": 0.3, "reason": "..." }
  ]
}
```

#### 기존 임베딩 방식 유지 (fallback)

LLM이 불가능할 때(오프라인, 비용 절감 모드) 기존 `PadAnalyzer`를 fallback으로 유지한다. `UtteranceAnalyzer` 포트를 통해 구현체를 교체할 수 있으므로 아키텍처 변경 없음.

```rust
// 구현체 선택
let analyzer: Box<dyn UtteranceAnalyzer> = if llm_available {
    Box::new(LlmPadAnalyzer::new(llm_client, model_config))
} else {
    Box::new(PadAnalyzer::new(embedder, anchor_source)?)  // 기존 방식
};
```

---

### 3.2 새 감정 생성 (변경 B)

#### 설계 원칙

1. **결정론적 엔진이 최종 판단** — LLM이 제안해도 엔진이 검증/거부할 수 있음
2. **OCC 구조 보존** — 아무 감정이나 생기는 게 아니라, 상황 맥락에 맞는 감정만
3. **낮은 초기 강도** — 새 감정은 0.2~0.4로 시작하여 후속 자극으로 성장

#### 두 경로로 새 감정 생성

**경로 1: LLM 제안 → 엔진 검증**

```rust
// LlmPadAnalyzer가 반환한 suggested_emotions를 엔진이 검증
fn validate_suggested_emotions(
    suggestions: &[SuggestedEmotion],
    current_state: &EmotionState,
    personality: &impl StimulusWeights,
) -> Vec<(EmotionType, f32)> {
    suggestions.iter()
        .filter(|s| !current_state.has(s.emotion_type))  // 이미 있으면 무시
        .filter(|s| s.initial_intensity >= NEW_EMOTION_MIN_INTENSITY)  // 최소 강도
        .filter(|s| s.initial_intensity <= NEW_EMOTION_MAX_INTENSITY)  // 최대 강도 제한
        .map(|s| {
            // 성격에 따른 보정: 예) 용감한 성격은 Fear 초기 강도 감소
            let adjusted = personality.adjust_new_emotion(s.emotion_type, s.initial_intensity);
            (s.emotion_type, adjusted)
        })
        .collect()
}
```

**경로 2: PAD 공명 기반 자동 생성 (임베딩 fallback용)**

LLM 없이도 강한 자극이 새 감정을 만들 수 있도록:

```rust
/// PAD 자극과 강하게 공명하는 감정 후보를 검색
fn discover_new_emotions(
    stimulus: &Pad,
    current_state: &EmotionState,
    personality: &impl StimulusWeights,
) -> Vec<(EmotionType, f32)> {
    ALL_EMOTION_TYPES.iter()
        .filter(|et| !current_state.has(**et))
        .filter_map(|et| {
            let emotion_pad = emotion_to_pad(*et);
            let resonance = pad_dot(&emotion_pad, stimulus);
            // 강한 양의 공명일 때만 새 감정 후보
            if resonance > NEW_EMOTION_RESONANCE_THRESHOLD {  // e.g., 0.6
                let intensity = (resonance * NEW_EMOTION_SCALE)
                    .clamp(NEW_EMOTION_MIN_INTENSITY, NEW_EMOTION_MAX_INTENSITY);
                Some((*et, intensity))
            } else {
                None
            }
        })
        // 최대 2개까지만 (한 턴에 너무 많은 감정이 생기면 부자연스러움)
        .take(MAX_NEW_EMOTIONS_PER_TURN)
        .collect()
}
```

#### 튜닝 상수 (tuning.rs 추가)

```rust
// 새 감정 생성
pub const NEW_EMOTION_RESONANCE_THRESHOLD: f32 = 0.6;  // PAD 공명 이 이상이면 후보
pub const NEW_EMOTION_MIN_INTENSITY: f32 = 0.2;         // 새 감정 최소 강도
pub const NEW_EMOTION_MAX_INTENSITY: f32 = 0.4;         // 새 감정 최대 강도
pub const NEW_EMOTION_SCALE: f32 = 0.5;                  // 공명값 → 초기 강도 스케일
pub const MAX_NEW_EMOTIONS_PER_TURN: usize = 2;          // 턴당 최대 새 감정 수
```

---

### 3.3 자극 모멘텀 (변경 C)

#### 개념

연속된 비슷한 자극이 점점 세지는 효과. "한 번 도발"과 "세 번 연속 도발"은 다르다.

#### 설계

```rust
/// 자극 히스토리 (EmotionStore에 NPC별 저장)
pub struct StimulusHistory {
    /// 최근 N턴의 PAD 자극 기록
    recent: VecDeque<Pad>,
    /// 지수 이동 평균 (exponential moving average)
    momentum: Pad,
}

impl StimulusHistory {
    pub fn new() -> Self {
        Self {
            recent: VecDeque::with_capacity(MOMENTUM_WINDOW),
            momentum: Pad::neutral(),
        }
    }

    /// 새 자극을 기록하고 모멘텀 갱신
    pub fn push(&mut self, pad: &Pad) {
        self.recent.push_back(*pad);
        if self.recent.len() > MOMENTUM_WINDOW {
            self.recent.pop_front();
        }
        // 지수 이동 평균: 최근 자극에 더 높은 가중치
        let decay = MOMENTUM_DECAY;
        self.momentum.pleasure = self.momentum.pleasure * decay + pad.pleasure * (1.0 - decay);
        self.momentum.arousal = self.momentum.arousal * decay + pad.arousal * (1.0 - decay);
        self.momentum.dominance = self.momentum.dominance * decay + pad.dominance * (1.0 - decay);
    }

    /// 모멘텀과 현재 자극의 방향 일치도 → 증폭 계수
    pub fn momentum_multiplier(&self, current: &Pad) -> f32 {
        if self.recent.len() < 2 {
            return 1.0;  // 첫 턴은 모멘텀 없음
        }
        let alignment = pad_dot(&self.momentum, current);
        if alignment > 0.0 {
            // 같은 방향: 누적 증폭 (최대 MOMENTUM_MAX_BOOST)
            1.0 + (alignment * MOMENTUM_BOOST_RATE).min(MOMENTUM_MAX_BOOST - 1.0)
        } else {
            // 반대 방향: 모멘텀 감쇄 (관성 저항)
            // 갑자기 방향이 바뀌면 약간 둔해짐
            (1.0 + alignment * MOMENTUM_RESISTANCE).max(MOMENTUM_MIN_FACTOR)
        }
    }
}
```

#### apply_stimulus에 통합

```rust
fn apply_stimulus(...) -> EmotionState {
    // 모멘텀 계수 조회
    let history = self.repository.get_stimulus_history(&npc_id);
    let momentum = history.momentum_multiplier(&pad);
    
    // 기존 감정 변동 (momentum 적용)
    for emotion in current_state.emotions() {
        let alignment = pad_dot(&emotion_pad, stimulus);
        let inertia = (1.0 - emotion.intensity()).max(STIMULUS_MIN_INERTIA);
        let delta = alignment * absorb * STIMULUS_IMPACT_RATE * inertia * momentum;
        //                                                                ^^^^^^^^
        //                                                        모멘텀 배율 적용
        ...
    }
    
    // 히스토리 갱신
    history.push(&pad);
    self.repository.save_stimulus_history(&npc_id, history);
}
```

#### 튜닝 상수 (tuning.rs 추가)

```rust
// 자극 모멘텀
pub const MOMENTUM_WINDOW: usize = 5;          // 최근 N턴 기억
pub const MOMENTUM_DECAY: f32 = 0.6;           // EMA 감쇠율 (높을수록 과거 영향 큼)
pub const MOMENTUM_BOOST_RATE: f32 = 0.3;      // 방향 일치 시 증폭률
pub const MOMENTUM_MAX_BOOST: f32 = 1.6;       // 최대 증폭 배율 (60% 증가)
pub const MOMENTUM_RESISTANCE: f32 = 0.2;      // 방향 반전 시 저항 계수
pub const MOMENTUM_MIN_FACTOR: f32 = 0.7;      // 최소 배율 (30% 감소까지)
```

#### 무협 시나리오 예시

```
Turn 1: "네놈이 감히!" (도발)
  → momentum: neutral → multiplier: 1.0
  → Anger delta: +0.15

Turn 2: "이 배은망덕한 놈!" (도발 계속)
  → momentum: 도발 방향으로 기움 → multiplier: 1.12
  → Anger delta: +0.17  (12% 증폭)

Turn 3: "사부님의 가르침을 잊었느냐!" (도발 강화)
  → momentum: 도발 강화 → multiplier: 1.25
  → Anger delta: +0.19  (25% 증폭)

Turn 4: "...미안하오." (갑작스런 사과)
  → momentum: 도발 방향 ↔ 사과 방향 충돌 → multiplier: 0.85
  → Anger delta: -0.10  (모멘텀 저항으로 감소 효과 둔화)
  "화가 머리끝까지 올라온 상태에서 갑자기 사과해도 바로 풀리지 않는다"
```

---

### 3.4 DTO 및 포트 확장 (변경 D)

#### StimulusRequest 확장

```rust
pub struct StimulusRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub situation_description: Option<String>,
    // 기존: PAD 직접 입력 (수동/임베딩 결과)
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,
    // 신규: 원문 대사 (LLM 분석용)
    pub utterance: Option<String>,
    // 신규: LLM 제안 새 감정 (analyze_with_context 결과)
    pub suggested_emotions: Vec<SuggestedEmotion>,
}
```

#### StimulusResult 확장

```rust
pub struct StimulusResult {
    // ... 기존 필드 ...
    pub input_pad: Option<PadOutput>,
    // 신규
    pub pad_reasoning: Option<String>,       // LLM의 분석 근거
    pub new_emotions: Vec<NewEmotionInfo>,    // 이번 턴에 새로 생긴 감정
    pub momentum_factor: f32,                 // 적용된 모멘텀 배율
}

pub struct NewEmotionInfo {
    pub emotion_type: EmotionType,
    pub intensity: f32,
    pub source: NewEmotionSource,  // LlmSuggested | ResonanceDiscovered
}
```

#### EmotionStore 포트 확장

```rust
pub trait EmotionStore {
    // ... 기존 ...
    
    /// 자극 히스토리 조회/저장 (모멘텀용)
    fn get_stimulus_history(&self, npc_id: &str) -> StimulusHistory;
    fn save_stimulus_history(&mut self, npc_id: &str, history: StimulusHistory);
}
```

---

## 4. 전체 흐름 (v2)

```
Player says: "배은망덕한 놈! 사부님의 가르침을 잊었느냐!"
                    │
                    ▼
    ┌─────────────────────────────────┐
    │ UtteranceAnalyzer (LLM 구현체)  │
    │                                 │
    │ 입력:                           │
    │  - utterance                    │
    │  - recent_turns (최근 5턴)      │
    │  - current_emotions             │
    │  - relationship (사제, 신뢰 ↓)  │
    │  - scene ("배신 발각 장면")     │
    │                                 │
    │ 출력:                           │
    │  PAD: P:-0.7, A:+0.8, D:+0.6   │
    │  reasoning: "사부-제자 관계에서  │
    │   의리 비난은 극도로 불쾌하고    │
    │   강한 권위로 압박"              │
    │  suggested: [Fear(0.3)]         │
    └──────────────┬──────────────────┘
                   │
                   ▼
    ┌─────────────────────────────────┐
    │ StimulusEngine.apply_stimulus() │
    │                                 │
    │ 1. momentum_multiplier = 1.25   │
    │    (3번째 연속 도발)            │
    │                                 │
    │ 2. 기존 감정 변동:              │
    │    Anger 0.4 → 0.62 (+0.22)    │
    │    Shame 0.3 → 0.45 (+0.15)    │
    │    Joy 0.2 → 0.08 (−0.12)     │
    │                                 │
    │ 3. 새 감정 생성:                │
    │    Fear 0.0 → 0.25 (LLM 제안)  │
    │    (성격 검증: 용감함 보정 적용) │
    │                                 │
    │ 4. 히스토리 갱신                │
    └──────────────┬──────────────────┘
                   │
                   ▼
    ┌─────────────────────────────────┐
    │ generate_guide()                │
    │                                 │
    │ "강한 분노와 수치심에 사로잡혀  │
    │  있으며, 배신에 대한 두려움이   │
    │  싹트기 시작했다..."            │
    └─────────────────────────────────┘
```

---

## 5. 구현 순서

기존 테스트를 깨뜨리지 않으면서 점진적으로 진행한다.

| Phase | 작업 | 영향 범위 | 기존 호환 |
|-------|------|----------|----------|
| **P1** | `StimulusHistory` + 모멘텀 로직 | domain + ports | ✅ 모멘텀 없으면 1.0 |
| **P2** | 새 감정 생성 (PAD 공명 경로) | stimulus.rs | ✅ threshold 높으면 비활성 |
| **P3** | `AnalysisContext` + `PadAnalysis` 타입 | ports, dto | ✅ 기존 `analyze()` 유지 |
| **P4** | `LlmPadAnalyzer` 구현체 | adapter (새 파일) | ✅ 기존 PadAnalyzer 유지 |
| **P5** | MindService 통합 | application | `analyze_with_context` 우선, fallback |
| **P6** | Mind Studio UI 연동 | handlers + 프론트엔드 | 점진적 |

---

## 6. 트레이드오프

### 채택 이유

- **LLM 기반 PAD**: 맥락 이해가 가능한 유일한 방식. 무협의 "말 속에 칼이 있는" 대화를 임베딩으로는 절대 처리 불가
- **새 감정 생성**: 대화 중 감정의 폭이 넓어져 NPC가 더 생동감 있게 반응
- **모멘텀**: 대화 흐름의 자연스러운 누적 효과. 실제 인간 감정과 유사

### 리스크

| 리스크 | 완화 |
|--------|------|
| LLM 레이턴시 증가 (턴당 +1 호출) | 분석 호출은 NPC 응답 생성보다 짧음. 병렬화 가능 |
| LLM 출력 불일치 (같은 대사인데 다른 PAD) | structured output + 값 범위 clamp + 캐싱 |
| 새 감정 남발 → 감정 상태 복잡화 | MAX_NEW_EMOTIONS_PER_TURN=2, 최소 강도 제한 |
| 모멘텀 → 감정 고착 (한번 화나면 계속) | MOMENTUM_DECAY=0.6으로 빠른 감쇠, 방향 전환 시 저항은 약함 |
| 기존 테스트 깨짐 | Phase별 점진 도입, 기존 API 하위 호환 유지 |

### 제거되는 것

- `PAD_AXIS_DEAD_ZONE`, `PAD_AXIS_SCALE` 상수 (LLM 사용 시 불필요, fallback에서만 사용)
- 앵커 TOML 유지보수 부담 (LLM 기본 시 앵커는 fallback 전용)

### 유지되는 것

- `pad_dot` 공식 (D 스케일러 방식) — 여전히 감정-자극 공명 계산의 핵심
- `StimulusEngine`의 inertia 공식 — 모멘텀과 직교하는 개념
- `PadAnalyzer` + 앵커 — LLM 불가 시 fallback
- OCC 감정 체계 전체

---

## 7. 열린 질문

1. **LLM 모델 선택**: 로컬 LLM(llama-server)으로 PAD 분석도 처리할지, 별도 경량 모델을 쓸지?
   - 현재 chat feature의 로컬 LLM과 동일 서버 사용이 가장 단순
   - 대안: 별도 경량 모델(Phi-3 등)로 PAD 전용 추론

2. **분석 비용 최적화**: 매 턴 LLM을 호출하면 비용이 늘어남.
   - 단순한 대사("네", "알겠소")는 규칙 기반으로 처리하고 LLM은 복잡한 대사에만?
   - 아니면 일관성을 위해 항상 LLM?

3. **모멘텀 리셋 시점**: Beat 전환 시 모멘텀을 초기화할지 유지할지?
   - 초기화: 새 장면 새 감정 → 깔끔한 시작
   - 유지: "이전 비트에서 쌓인 감정이 남아있다" → 더 자연스러움

---

## 8. 참고

- v1 설계 기록: [`docs/emotion/pad-stimulus-design-decisions.md`](pad-stimulus-design-decisions.md)
- pad_dot D축 문제 분석: 같은 문서 §3~§6
- HEXACO → 자극 수용도: `src/domain/personality.rs` (`StimulusWeights` 구현)
