# Step 4: 감정 판정 시스템 — 임베딩 트리거 + LLM 판정 하이브리드

**버전:** v2.0.0  
**작성일:** 2026-03-01T01:30:00+09:00  
**관련 문서:**
- `step4.2-sentiment-benchmark-report.md` — 벤치마크 결과 상세
- `step3_3-threshold-analyzer-report.md` — 임베딩 threshold 분석
- `step4-benchmark-report.md` — Step 4 모델 비교

---

## 1. 목표

NPC 대사에서 플레이어에 대한 감정(호의/적대/중립)을 판정하여 관계 시스템에 반영한다.

---

## 2. 벤치마크 결과 요약

Step 4.1~4.2에서 3가지 방식을 실험하여 최종 전략을 도출했다.

### 2.1 임베딩 단독 감정 분류 — 실패

```
모델: BGE-M3 (1024d), 방식B(개별앵커 max), 앵커 20개
전체 정확도: 71.7% (기준 80% 미달)

  warmth:   100%
  coldness:  70%  ← 존대말 바이어스 (무협체 "~하오" → warmth 오분류)
  neutral:   45%  ← 정보 전달을 warmth로 과대 분류

근본 원인: 임베딩은 표면 문체를 감정으로 오인함
결론: 임베딩 단독으로 3분류 감정 판정 불가
```

### 2.2 LLM 직접 판정 — 성공

```
모델: gemma3-12b (Q3_K_M, 5.6GB)

Phase A (단일 대사 60개): 100.0% ★
Phase B (멀티턴 시나리오 10개): 76.7% (시나리오 대사 보정 시 ~90% 예상)

핵심 교훈:
  - 맥락 맞춤 질문으로 70%→100% (동일 모델)
  - LLM은 대화 논리까지 평가하므로 테스트 설계 시 맥락 일치 필수
  - 응답 시간: 3턴~9턴 누적해도 ~300ms 차이 → 12턴 판정 가능
```

### 2.3 극단 앵커 임베딩 트리거 — 성공

```
목적: "누가 봐도 명확한" 극단 감정만 저비용으로 즉시 감지

BGE-M3 + 극단 앵커(생사 수준 20+20개):
  진양성: 100% (30/30)
  거짓양성: 0% @ t=0.60~0.65
  안전 마진(갭): 0.067

핵심 발견: 앵커를 극단 수준으로 설계하면 극단 어휘와만 가까워지는 효과
  "네놈을 죽여주마" vs "별로 마음에 들지 않소" → 0.58 (분리됨)
  "목숨 바쳐 모시겠소" vs "다음에 또 들르시오" → 0.50 (분리됨)

※ 욕설 키워드 사전은 "죽","베어","목을" 등 일상 맥락 거짓양성 과다로 제외
```

### 2.4 언어별 모델 전략 (잠정)

```
한국어/중국어: BGE-M3 — 극단 트리거 안전 마진 넓음
영어:          EmbeddingGemma — 잠정, 프롬프트 작성 후 재검증 필요
```

---

## 3. 확정 아키텍처: 2단계 하이브리드

```
매 턴 (~7ms):
  극단 앵커 임베딩 (BGE-M3, t=0.60)
  극단 warmth/coldness 앵커 각 20개
  → 트리거 시 → 즉시 LLM 감정 판정

12턴마다 (정기):
  LLM 감정 판정 (~300ms)
  누적 대화 컨텍스트 → JSON 판정

LLM 판정 결과:
  {"sentiment":"warmth|coldness|neutral", "score":-3~+3, "reason":"..."}
  → score를 affinity delta로 변환 → Relationship 반영
```

### 3.1 트리거 흐름도

```
  NPC 대사 입력
      │
      ├─ 극단 앵커 임베딩 ─ ≥0.60 ─┐
      │                            │
      │    ┌───────────────────────┘
      │    ▼
      │  즉시 LLM 감정 판정 (300ms)
      │    │
      │    ▼
      │  score → delta → Relationship 업데이트
      │
      └─ 트리거 없음 ─── 12턴 경과? ─── Yes ─→ 정기 LLM 판정
                              │
                              No → 다음 턴 대기
```

### 3.2 왜 이 조합인가

```
계층              비용     역할                      한계
───────────────────────────────────────────────────────────
극단 앵커 임베딩   ~7ms    극단적 호의/적대 즉시 감지   약한 감정 분리 불가
LLM 정기 판정     ~300ms   전체 감정 상태 정밀 판정    매 턴 불가 (비용)

두 계층이 상호 보완: 임베딩은 빠르지만 극단만, LLM은 정밀하지만 비싸다.
```

---

## 4. 전체 파이프라인 아키텍처

```
[인프라]                    [도메인 (wuxia-core)]           [어플리케이션 (wuxia-llm)]
                                                           
EmbeddingPort (BGE-M3)      ExtremeAnchorSet               ChatSession.send()
  │ embed(대사)              check_extreme() → bool           │
  │ embed(극단앵커)                                           │ ① LLM 호출 → 대사
  │                         SentimentJudgeConfig              │ ② 극단 앵커 체크 (매 턴)
LlmPort (gemma3-12b)        score_to_delta()                 │ ③ 12턴 카운터 체크
  │ judge(대화누적)                                           │ ④ ②or③ 트리거 시 → LLM 판정
  │ → JSON 파싱             ConversationEffect                │ ⑤ score → delta (도메인)
  │                         apply_conversation_effect()       │ ⑥ 관계 반영 (도메인)
  │                         Relationship.update_affinity()    │
  └─────────────────────────────┼─────────────────────────────┘
                                │
                          Vec<DomainEvent>
                          [AffinityChanged, LevelChanged?]
```

---

## 5. 선행 조건

- ✅ Relationship aggregate (2축: 호감도 -100~+100, 신뢰도 0~100)
- ✅ ConversationEffect + apply_conversation_effect() 도메인 서비스
- ✅ EmbeddingPort trait + cosine_similarity() 유틸
- ✅ ChatSession + ContextProvider (기억 검색 파이프라인)
- ✅ RelationshipView → 프롬프트 주입 경로
- ✅ ADR: Lightweight CQRS 결정
- ✅ LLM 판정 프롬프트 검증 완료 (Phase A 100%)
- ✅ 극단 앵커 BGE-M3 벤치마크 완료 (진양성 100%, 거짓양성 0%)
- ✅ gemma3-12b 모델 파일 (`models/google_gemma-3-12b-it-Q3_K_M.gguf`)
- ✅ BGE-M3 모델 파일 (`models/Bge-M3-567M-Q8_0.gguf`)

---

## 6. Iteration 계획

### Iteration 4.1 — 벤치마크 (완료 ✅)

임베딩 감정 분류 한계 확인 (71.7%), LLM 판정 검증 (100%), 극단 앵커 트리거 검증.

산출물:
- ✅ `step4.2-sentiment-benchmark-report.md` (v1.3)
- ✅ `crates/wuxia-memory/examples/sentiment_benchmark.rs`
- ✅ `crates/wuxia-memory/examples/sentiment_extreme_benchmark.rs`
- ✅ `crates/wuxia-llm/examples/sentiment_llm_benchmark.rs`
- ✅ `crates/wuxia-llm/examples/sentiment_llm_benchmark_b.rs`

---

### Iteration 4.2 — wuxia-core 도메인 로직 (완료 ✅)

**목표**: 감정 판정의 순수 비즈니스 규칙을 도메인 모델로 구현한다.

**원칙**:
- wuxia-core에만 코드 추가 (인프라 의존 없음)
- `cargo test -p wuxia-core`로 검증
- 기존 Relationship, ConversationEffect 재사용

산출물:
- ✅ `crates/wuxia-core/src/relationship/sentiment.rs` — ExtremeAnchorSet, SentimentJudgment, TurnCounter, SentimentDirection, SentimentJudgeConfig, judgment_to_delta
- ✅ `crates/wuxia-core/src/relationship/effect.rs` — DeltaSource enum 확장, ConversationEffect::with_source()
- ✅ 21 unit tests + 3 doc tests in sentiment.rs

#### 4.2.1 ExtremeAnchorSet (Value Object)

```rust
// wuxia-core/src/relationship/sentiment.rs

/// 극단 앵커와 대사 벡터의 유사도로 극단 감정 트리거 여부 판정
pub struct ExtremeAnchorSet {
    warmth_vectors: Vec<Vec<f32>>,     // 극단 호의 앵커 20개 벡터
    coldness_vectors: Vec<Vec<f32>>,   // 극단 적대 앵커 20개 벡터
    threshold: f32,                     // 0.60 (BGE-M3 기준)
    dimension: usize,
}

impl ExtremeAnchorSet {
    /// 대사 벡터가 극단 앵커에 threshold 이상 유사한지 체크
    /// 방식: 개별앵커 max (벤치마크에서 검증된 방식B)
    pub fn check_extreme(&self, utterance_vector: &[f32]) -> ExtremeCheckResult
}

pub struct ExtremeCheckResult {
    pub triggered: bool,           // threshold 이상인가
    pub direction: SentimentDirection,  // Warmth / Coldness / None
    pub max_similarity: f32,       // 최고 유사도 값
}

pub enum SentimentDirection {
    Warmth,
    Coldness,
    None,
}
```

#### 4.2.2 SentimentJudgeConfig (Value Object)

```rust
/// LLM 판정 결과(score)를 affinity delta로 변환하는 설정
pub struct SentimentJudgeConfig {
    pub periodic_turns: u32,    // 12 (정기 판정 주기)
}

/// LLM 판정 결과
pub struct SentimentJudgment {
    pub sentiment: SentimentDirection,  // warmth / coldness / neutral
    pub score: i8,                      // -3 ~ +3
    pub reason: String,
}

/// score를 그대로 delta로 사용 (LLM이 -3~+3 반환)
pub fn judgment_to_delta(judgment: &SentimentJudgment) -> i8 {
    judgment.score  // LLM score = affinity delta
}
```

#### 4.2.3 TurnCounter (Value Object)

```rust
/// 정기 판정 턴 카운터
pub struct TurnCounter {
    count: u32,
    period: u32,  // 12
}

impl TurnCounter {
    pub fn tick(&mut self) -> bool {
        self.count += 1;
        if self.count >= self.period {
            self.count = 0;
            true  // 판정 트리거
        } else {
            false
        }
    }

    /// 즉시 트리거 발생 시 카운터 리셋
    pub fn reset(&mut self) {
        self.count = 0;
    }
}
```

#### 4.2.4 ConversationEffect 확장

```rust
pub enum DeltaSource {
    LlmPeriodicJudgment,     // 12턴 정기 LLM 판정
    LlmTriggeredJudgment,    // 극단 임베딩 트리거 → LLM 판정
    LegacyTag,               // 기존 [affinity: N] 태그 (하위호환)
}

pub struct ConversationEffect {
    affinity_delta: i8,
    source: DeltaSource,
}
```

#### 4.2.5 테스트 계획 (~20개)

```
ExtremeAnchorSet:
  ① 극단 호의 벡터 → triggered=true, direction=Warmth
  ② 극단 적대 벡터 → triggered=true, direction=Coldness
  ③ 중립 벡터 → triggered=false, direction=None
  ④ threshold 경계값 (0.599 → false, 0.601 → true)
  ⑤ warmth와 coldness 둘 다 높은 경우 → 높은 쪽 채택

SentimentJudgment + judgment_to_delta:
  ⑥ score +3 → delta +3
  ⑦ score -3 → delta -3
  ⑧ score 0 → delta 0
  ⑨ warmth/score 양수 정합성
  ⑩ coldness/score 음수 정합성

TurnCounter:
  ⑪ 12턴 미만 → false
  ⑫ 12턴 도달 → true + 리셋
  ⑬ reset() 후 다시 12턴 필요
  ⑭ tick() 연속 호출 패턴

ConversationEffect 확장:
  ⑮ DeltaSource 필드 접근 + 기존 테스트 호환
  ⑯ LlmPeriodicJudgment source 검증
  ⑰ LlmTriggeredJudgment source 검증

통합 시나리오:
  ⑱ 극단 트리거 → judgment → delta → ConversationEffect 흐름
  ⑲ 비트리거 → 12턴 경과 → judgment → delta 흐름
  ⑳ 극단 트리거 발생 시 카운터 리셋 검증
```

모든 테스트는 **하드코딩 벡터** 사용 (Mock 임베딩 불필요, 순수 수학).

---

### Iteration 4.3 — LLM 판정 모듈 (wuxia-llm) (완료 ✅)

**목표**: LLM에 대화 컨텍스트를 전달하고 JSON 판정을 받는 모듈 구현.

산출물:
- ✅ `crates/wuxia-llm/src/sentiment/judge.rs` — SentimentJudge trait, LlmSentimentJudge, MockSentimentJudge
- ✅ `crates/wuxia-llm/src/sentiment/parser.rs` — JSON 판정 파서 (score 이중 파싱)
- ✅ `crates/wuxia-llm/src/sentiment/mod.rs` — 모듈 재수출

#### 4.3.1 SentimentJudge trait

```rust
// wuxia-llm/src/sentiment/judge.rs

/// LLM 감정 판정 인터페이스
pub trait SentimentJudge {
    /// 대화 컨텍스트를 보고 NPC 감정 판정
    fn judge(&self, conversation: &[DialogueTurn]) -> Result<SentimentJudgment>;
}

pub struct DialogueTurn {
    pub speaker: Speaker,  // Player / Npc
    pub text: String,
}
```

#### 4.3.2 LlmSentimentJudge 구현

```rust
/// gemma3-12b 기반 감정 판정
pub struct LlmSentimentJudge<L: LlmPort> {
    llm: L,
    system_prompt: String,   // 벤치마크 검증된 프롬프트
    sampling: SamplingConfig, // temp=0.1, max_tokens=150, seed=42
}

impl<L: LlmPort> SentimentJudge for LlmSentimentJudge<L> {
    fn judge(&self, conversation: &[DialogueTurn]) -> Result<SentimentJudgment> {
        // ① 대화 포맷팅 (턴 번호 + 역할 + 대사)
        // ② LLM 호출
        // ③ JSON 파싱 (score: i64 또는 str 이중 파싱)
        // ④ SentimentJudgment 반환
    }
}
```

#### 4.3.3 검증된 프롬프트 (벤치마크 Phase A에서 100% 달성)

```
시스템 프롬프트:
  "너는 무협 소설의 대화를 분석하는 감정 평가관이다.
   주어진 대화에서 NPC가 플레이어에게 보이는 **현재** 감정을 판정하라.
   ...
   중요: 무협체 존대말("~하오", "~하겠소")은 문체일 뿐 감정이 아니다."

유저 메시지:
  "다음은 NPC와 플레이어의 연속 대화이다. 대화 전체를 읽고,
   NPC의 **현재** 감정을 판정하라.
   턴1: 플레이어: "..." NPC: "..."
   턴2: ...
   위 대화에서 NPC의 현재 감정을 JSON으로 판정하라."

샘플링: temperature=0.1, max_tokens=150, top_k=40, seed=42
```

상세: `step4.2-sentiment-benchmark-report.md` 5장 참조.

#### 4.3.4 JSON 파서

```rust
/// LLM 응답 JSON 파싱 (score가 i64 또는 str일 수 있음)
pub fn parse_judgment_json(json_str: &str) -> Result<SentimentJudgment> {
    // ① serde_json 시도
    // ② score 필드: i64 먼저 → 실패 시 str → i64 파싱
    // ③ sentiment 필드: "warmth"|"coldness"|"neutral"
    // ④ reason 필드: String
}
```

#### 4.3.5 테스트 계획 (~10개)

```
LlmSentimentJudge (MockLlm):
  ① warmth JSON 응답 → SentimentJudgment 정상 파싱
  ② coldness JSON 응답 → 정상 파싱
  ③ neutral JSON 응답 → 정상 파싱
  ④ score가 문자열("-2") → 정상 파싱 (이중 파싱)
  ⑤ 잘못된 JSON → Error 반환

DialogueTurn 포맷팅:
  ⑥ 3턴 대화 → 올바른 포맷 문자열
  ⑦ 12턴 대화 → 올바른 포맷 문자열

프롬프트 조립:
  ⑧ 시스템 프롬프트에 무협체 존대말 주의사항 포함 확인
  ⑨ 유저 메시지에 턴 번호 순서 확인
  ⑩ 빈 대화 → Error 반환
```

---

### Iteration 4.4 — ChatSession 파이프라인 통합 (완료 ✅)

**목표**: ChatSession.send()에 2단계 하이브리드 판정을 연결한다.

산출물:
- ✅ `crates/wuxia-llm/src/sentiment/pipeline.rs` — SentimentPipeline (극단 앵커 트리거 + 정기 LLM 판정)
- ✅ `crates/wuxia-llm/src/conversation/session.rs` — ChatSession에 Option<SentimentPipeline> 통합
- ✅ `crates/wuxia-llm/src/prompt/template.rs` — skip_affinity_directive 지원
- ✅ `crates/wuxia-llm/src/prompt/types.rs` — PromptContext.skip_affinity_directive 필드

#### 4.4.1 변경 후 send() 흐름

```
ChatSession.send(player_input):
  ① 기억 검색 (ContextProvider)
  ② 프롬프트 조립 (build_system_prompt — 태그 지시 제거)
  ③ LLM 호출 → npc_text
  ④ 턴 카운터 tick()
  ⑤ embed(npc_text) → utterance_vector                    ← 🆕
  ⑥ extreme_anchors.check_extreme(utterance_vector)        ← 🆕
  ⑦ if ⑥.triggered OR ④==true:                            ← 🆕
       judge.judge(conversation_history) → judgment
       delta = judgment_to_delta(judgment)
       if ⑥.triggered: turn_counter.reset()
     else:
       delta = 0  (이번 턴 감정 변동 없음)
  ⑧ ConversationEffect::new(delta, source)
  ⑨ apply_conversation_effect(&mut rel, &effect)
  ⑩ ctx 압축 판정
  ⑪ ChatReply 반환
```

#### 4.4.2 ChatSession 의존성

```rust
pub struct ChatSession<L: LlmPort, C: ContextProvider, E: EmbeddingPort> {
    llm: L,
    context_provider: C,
    embedder: E,                           // 🆕 극단 앵커 체크용
    judge: LlmSentimentJudge<L>,           // 🆕 LLM 판정용 (같은 LLM 재사용)
    extreme_anchors: ExtremeAnchorSet,     // 🆕
    turn_counter: TurnCounter,             // 🆕
    relationship: Relationship,
    // ... 기존 필드
}
```

#### 4.4.3 앵커 초기화 (세션 생성 시 1회)

```
① 극단 앵커 문장 로딩 (toml)
② embedder.embed_batch(warmth_extreme_texts) → warmth_vectors (20개)
③ embedder.embed_batch(coldness_extreme_texts) → coldness_vectors (20개)
④ ExtremeAnchorSet::new(warmth_vecs, coldness_vecs, threshold=0.60)
⑤ 세션 내부에 캐시 (세션 수명 동안 불변)

비용: 40문장 embed → BGE-M3 기준 ~600ms (1회만)
```

#### 4.4.4 프롬프트 변경

```
prompt_config.toml 변경:
  제거: "[affinity: N] 태그를 붙여라" 지시
  유지: 캐릭터 지시/대화 규칙 그대로

이유: LLM이 감정 판정 전용으로 별도 호출되므로, 대사 생성 LLM에게 태그 요구 불필요
```

#### 4.4.5 대화 이력 관리

```
LLM 판정에 전달할 대화 이력:
  - 12턴 이하: 전체 이력 전달
  - 12턴 초과: 최근 12턴만 전달 (n_ctx=4096 충분)
  - 30턴 초과 장기 대화: "이전 요약 + 최근 12턴" (향후 구현)
```

#### 4.4.6 테스트 계획 (~12개)

```
ChatSession 단위 (MockLlm + MockEmbedding):
  ① 극단 호의 대사 → 즉시 트리거 → LLM 판정 → 양수 delta
  ② 극단 적대 대사 → 즉시 트리거 → LLM 판정 → 음수 delta
  ③ 중립 대사 → 트리거 없음 → delta 0
  ④ 12턴 중립 후 정기 판정 트리거 → LLM 판정 호출
  ⑤ 극단 트리거 후 카운터 리셋 → 다시 12턴 필요
  ⑥ 임베딩 실패 → graceful fallback (delta 0)
  ⑦ LLM 판정 실패 → graceful fallback (delta 0)

통합 시나리오 (MockLlm + MockEmbedding + 실제 Relationship):
  ⑧ 3턴 극단 호의 → affinity 증가 + 레벨 변화 확인
  ⑨ 혼합 시나리오: 중립 11턴 + 극단 1턴 → 트리거 확인
  ⑩ 12턴 정기 판정 → RelationshipView 반영 확인
  ⑪ 레벨 전이: Stranger → Acquaintance 이벤트 확인
  ⑫ ChatReply에 affinity_delta 올바른 값 검증
```

---

### Iteration 4.5 — 호감도의 대화 컨텍스트 주입 (완료 ✅)

**목표**: 갱신된 호감도가 다음 턴 프롬프트에 즉시 반영되는 것을 확인한다.

#### 4.5.1 기존 경로 (변경 없음)

```
Relationship.update_affinity(delta)
    │
    ▼ format_relationship_for_prompt()
RelationshipView { level, trust, hostility }
    │
    ▼ build_system_prompt()
[관계 상태] 관계: 아는 사이 / 신뢰: 경계
    │
    ▼ 다음 턴 LLM 프롬프트에 포함
```

delta 산출 방식(태그→LLM판정)이 바뀌어도 이 경로는 동일하게 동작한다.

#### 4.5.2 확인 사항

```
① ChatSession이 Relationship을 직접 소유 (v3에서 확립)
② update_affinity() 호출 후 다음 턴 send()에서 level() 재계산
③ 1턴 지연 (기존과 동일, 문제 없음)
④ CQRS: Write(update_affinity) → Read(RelationshipView) 동기적
```

#### 4.5.3 통합 테스트 (~4개)

```
① 극단 호의 트리거 → 다음 턴 프롬프트에 관계 변화 반영
② 12턴 정기 판정 → 다음 턴 프롬프트에 반영
③ 레벨 전이 → [관계 상태] 텍스트 변경 확인
④ end-to-end: 전체 파이프라인 동작 확인
```

---

## 7. 데이터 파일

### 7.1 극단 앵커 설정 (신규)

```toml
# assets/ai/extreme-anchors.toml

[config]
model = "bge-m3"
threshold = 0.60
dimension = 1024

[warmth]
anchors = [
    "목숨 바쳐 모시겠소",
    "천 번이라도 죽겠소",
    "은혜를 목숨으로 갚겠소",
    # ... 20개 (벤치마크에서 사용한 세트B)
]

[coldness]
anchors = [
    "네놈을 죽여주마",
    "사지를 찢어주마",
    "목숨을 내놓으시오",
    # ... 20개 (벤치마크에서 사용한 세트B)
]
```

### 7.2 LLM 판정 프롬프트 설정 (신규)

```toml
# assets/ai/sentiment-judge.toml

[sampling]
temperature = 0.1
max_tokens = 150
top_k = 40
top_p = 0.95
min_p = 0.05
seed = 42
repeat_penalty = 1.0
n_ctx = 4096

[prompt]
system = """
너는 무협 소설의 대화를 분석하는 감정 평가관이다.
... (벤치마크 검증된 프롬프트 — report 5장 참조)
"""

user_template = """
다음은 NPC와 플레이어의 연속 대화이다. 대화 전체를 읽고,
NPC의 **현재** 감정을 판정하라.
{turns}
위 대화에서 NPC의 현재 감정을 JSON으로 판정하라.
"""
```

---

## 8. 파일 변경 요약

### 신규 파일

| 파일 | crate | 내용 |
|------|-------|------|
| `relationship/sentiment.rs` | wuxia-core | ExtremeAnchorSet, SentimentJudgment, TurnCounter |
| `sentiment/judge.rs` | wuxia-llm | SentimentJudge trait, LlmSentimentJudge |
| `sentiment/parser.rs` | wuxia-llm | JSON 판정 파서 (score 이중 파싱) |
| `assets/ai/extreme-anchors.toml` | 데이터 | 극단 앵커 40개 |
| `assets/ai/sentiment-judge.toml` | 데이터 | LLM 판정 프롬프트 + 샘플링 설정 |

### 수정 파일

| 파일 | crate | 변경 내용 |
|------|-------|----------|
| `relationship/effect.rs` | wuxia-core | DeltaSource enum 확장 |
| `relationship/mod.rs` | wuxia-core | `pub mod sentiment;` 추가 |
| `conversation/session.rs` | wuxia-llm | EmbeddingPort 추가, 2단계 판정 파이프라인 |
| `prompt_config.toml` | wuxia-data | 태그 지시 제거 |
| `prompt/template.rs` | wuxia-llm | AFFINITY_TAG_PREFIX 제거 |

### 유지 (재사용)

| 파일 | 이유 |
|------|------|
| `effect.rs` apply_conversation_effect | delta 적용 로직 동일 |
| `types.rs` Relationship.update_affinity | delta 받아서 반영 (변경 없음) |
| `event.rs` RelationshipEvent 7종 | 이벤트 패턴 그대로 |
| `parser.rs` extract_affinity_tag | 하위호환/향후 재사용 가능 |

---

## 9. 테스트 전략

```
wuxia-core (Iter 4.2):
  Mock 없음 — 순수 함수, 하드코딩 벡터
  cargo test -p wuxia-core

wuxia-llm (Iter 4.3~4.4):
  MockLlm — LLM 응답 고정 JSON
  MockEmbedding — 결정론적 벡터 반환
  cargo test -p wuxia-llm

통합 (Iter 4.5):
  MockLlm + MockEmbedding + 실제 Relationship
  cargo test -p wuxia-llm --features integration
```

---

## 10. 일정 (추정)

| Iteration | 상태 | 산출물 |
|-----------|------|--------|
| **4.1** 벤치마크 | ✅ 완료 | 리포트 + 벤치마크 코드 |
| **4.2** wuxia-core 도메인 | ✅ 완료 | sentiment.rs + effect 확장 (21 unit + 3 doc tests) |
| **4.3** LLM 판정 모듈 | ✅ 완료 | judge.rs + parser.rs + mod.rs (29 tests in sentiment/) |
| **4.4** ChatSession 통합 | ✅ 완료 | pipeline.rs + session.rs 통합 |
| **4.5** 컨텍스트 주입 확인 | ✅ 완료 | skip_affinity_directive + 통합 테스트 |

**전체 완료**: `c5e14c3` feat(sentiment): 2단계 하이브리드 감정 판정 시스템 구현 [step4.2-4.5]

---

## 11. 결정 유보 사항 (Step 4 이후)

```
① 언어별 임베딩 모델 전략 확정
   → 프롬프트 작성 완료 후 KO/ZH=BGE, EN=Gemma 가설 검증
   → 기억 검색 + 극단 트리거 양쪽 재벤치마크

② Phase B 시나리오 보정
   → 대사/기대값 조정 후 재실행 (85%+ 예상)
   → 멀티턴 판정 정확도 최종 확인

③ 30턴+ 장기 대화 대응
   → "이전 요약 + 최근 12턴" 하이브리드 방식 설계

④ ChatSession wuxia-app 이동 (기술 부채)
   → 제네릭이 3개가 되면 복잡도 증가
   → 4.4 완료 후 재평가

⑤ EventLog append 연결
   → ADR Step 2 (Phase 4 후반) 시점에 진행
   → 현재는 이벤트 반환만 하고 소비 후 폐기
```

---

## 변경 이력

| 버전 | 변경일 | 변경 내역 |
|------|--------|----------|
| v2.0.0 | 2026-03-01T01:30:00+09:00 | 전면 재작성. 임베딩 단독→2단계 하이브리드(극단 앵커 트리거+LLM 정기 판정). 벤치마크 결과 반영. 욕설 키워드 사전 제외. Iteration 4.1 완료 처리. 4.2~4.5 재설계. |
| v1.2.0 | 2026-02-28T16:30:00+09:00 | 선행조건 2축 모델 반영. 유사도 계산 방식A vs 방식B 벤치마크 설계. |
| v1.1.0 | 2026-02-28T16:00:00+09:00 | 벤치마크 Python→Rust. 앵커 문장 수 5~30개 범위 확대. |
| v1.0.0 | 2026-02-28T15:30:00+09:00 | 초안. 임베딩 단독 감정 분석 계획 (4개 Iteration). |
