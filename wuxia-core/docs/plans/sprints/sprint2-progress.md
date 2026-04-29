# Sprint 2 — "소연이 기억한다" 진행 상황

**버전:** v1.0.0  
**수정일:** 2026-02-21 04:30:00

---

## 목표

소연이 이전 대화를 기억하고, 맥락에 맞는 응답을 생성한다.  
대화가 길어지면 ctx를 자동 압축하고, 대화 종료 시 기억으로 저장한다.

```
Sprint 2 — "소연이 기억한다" 전체 흐름

  플레이어               ChatSession            MemoryRepository       LLM
  ─────────             ────────────           ────────────────      ─────
       │                      │                      │                │
       │ "혈교가 뭐야?"       │                      │                │
       ├─────────────────────►│                      │                │
       │                      │  기억 검색: "혈교"    │                │
       │                      │  (rank_memories)     │                │
       │                      ├─────────────────────►│                │
       │                      │  [관련 기억 반환]     │                │
       │                      │◄─────────────────────┤                │
       │                      │                      │                │
       │                      │  프롬프트 조립        │                │
       │                      │  시스템+[기억]+[요약]+대화              │
       │                      ├────────────────────────────────────►  │
       │                      │  "혈교는 사파 중에서..."               │
       │                      │◄────────────────────────────────────  │
       │                      │                      │                │
       │                      │  ctx 60% 초과?       │                │
       │                      │  → 미시 요약 생성    │                │
       │                      │  → 오래된 턴 삭제    │                │
       │                      │                      │                │
       │ 소연: "혈교는..."     │                      │                │
       │◄─────────────────────┤                      │                │
       │                      │                      │                │
       │ (대화 종료)           │                      │                │
       │                      │  💾 최종 Observation  │                │
       │                      ├─────────────────────►│                │
```

---

## 선행 조건

- ✅ Sprint 1 완료 (LlmPort, LlamaCppAdapter, KV cache, ModelArch)
- ✅ wuxia-core: LlmPort trait, LlmRequest/Response, Message
- ✅ wuxia-llm: CharacterPromptData, build_system_prompt(), MockLlm, parse_response()
- ✅ wuxia-memory: 빈 crate (Cargo.toml + lib.rs placeholder)

---

## 스텝 진행표

| Step | 이름 | crate | 상태 | 테스트 | 날짜 |
|------|------|-------|------|--------|------|
| 2.1 | MemoryEntry + MemoryRepository trait | wuxia-core | ✅ 완료 | 59개 (기존+신규) | 2026-02-20 |
| 2.2 | InMemoryRepository 구현 | wuxia-memory | ✅ 완료 | 28 unit + 2 doc | 2026-02-20 |
| 2.3 | retrieval_score() + OCC 감정 훅 | wuxia-core | ✅ 완료 | 16 신규 | 2026-02-20 |
| 2.4 | PromptContext → 시스템 프롬프트 기억 삽입 | wuxia-llm | ✅ 완료 | 11 신규 + 17 수정 | 2026-02-20 |
| 2.5 | ConversationManager (ctx 압축) | wuxia-llm | ✅ 완료 | 28 신규 | 2026-02-20 |
| 2.6 | ChatSession (대화 루프 통합) | wuxia-llm | ✅ 완료 | 15 신규 | 2026-02-21 |

**총 테스트:** ~170개 (wuxia-core ~100, wuxia-llm ~100, wuxia-memory ~30)

> **✅ 현행화 (2026-03-03):** Sprint 2 Phase A 기능 전체 **개발완료 유지**. 테스트 ~170개 → 현재 ~1,463개 (wuxia-core 1,002 + wuxia-llm 340 + wuxia-memory 97 + wuxia-data 16 + wuxia-app 8). Phase B(LanceDB)도 Sprint 3에서 **개발완료**됨.

---

## Phase A: InMemory 기반 (Step 2.1~2.6) — ✅ 완료

### Step 2.1 — memory/ 모듈 + MemoryRepository trait [wuxia-core] ✅

기억의 구조와 저장소 규약을 정의했다.

**생성된 파일:**
- `wuxia-core/src/memory/types.rs` — MemoryEntry, MemoryType, MemoryId, Reflection 트리
- `wuxia-core/src/memory/port.rs` — MemoryRepository trait, ScoredMemory
- `wuxia-core/src/memory/event.rs` — MemoryStored / MemoryRecalled 이벤트

> **🔄 현행화 (2026-03-03):** memory/ 모듈이 8개 파일로 확장됨. 추가된 파일: `recall.rs` (recall_memories 도메인 서비스), `service.rs` + `service_tests.rs` (store_memory, recall_and_emit, update_importance 서비스 함수). `types.rs`에 `lang` 필드 추가 (다국어 threshold 분기). `port.rs`의 `ScoredMemory`는 별도 `retrieval.rs`에서 `RankedMemory`와 함께 관리. MemoryRepository 에러 타입: `String` → `PortError`로 변경됨.

**핵심 설계:**
- MemoryType: Observation(관찰) / Reflection(반성) / Plan(계획) — Generative Agents 논문 기반
- MemoryEntry: 불변 구조체. importance는 f32 (0.0~10.0), keywords는 Vec<String>
- Reflection 트리: `reflection_sources: Vec<MemoryId>` — 어떤 기억에서 성찰이 나왔는지 추적
- MemoryRepository: save / find_by_id / find_recent / search / count + 부분 구현 허용
- ScoredMemory: MemoryEntry + relevance_score (검색 결과용)

### Step 2.2 — InMemoryRepository 구현 [wuxia-memory] ✅

HashMap 기반 메모리 저장소를 구현했다.

**생성된 파일:**
- `wuxia-memory/src/in_memory.rs` — InMemoryRepository

**핵심 설계:**
- HashMap<CharacterId, Vec<MemoryEntry>> + AtomicU64 ID 자동생성
- search(): 키워드 매칭 + relevance_score 계산 (정확 일치 1.0, 부분 일치 0.5)
- Send + Sync: RwLock으로 래핑
- 28 unit tests + 2 doctests 통과

### Step 2.3 — retrieval_score() + OCC 감정 훅 [wuxia-core] ✅

기억 검색 스코어링 시스템을 구현했다.

**생성된 파일:**
- `wuxia-core/src/memory/retrieval.rs` — retrieval_score(), rank_memories()

**핵심 설계:**

```
  점수 = (α × recency) + (β × importance) + (γ × relevance) + (δ × emotional_bias)

  recency     = 0.995^일수  (시간 감쇠)
  importance  = entry.importance / 10.0  (정규화)
  relevance   = ScoredMemory.relevance_score  (키워드 매칭 → 향후 벡터)
  emotional_bias = OCC_TODO⑤ 예약 (현재 0.0)
```

- RetrievalWeights: NPC별 가중치 튜닝 가능 (α, β, γ, δ)
- EmotionalBias: PAD(Pleasure-Arousal-Dominance) 기반 기분 일치 편향 — OCC 통합 시 활성화
- RankedMemory: MemoryEntry + final_score (정렬 결과)
- rank_memories(): weights + emotional_bias + top_k 파라미터
- 16개 테스트 통과 (감쇠, 정규화, 복합 랭킹 등)

> **🔄 현행화 (2026-03-03):** OCC 감정 시스템이 `wuxia-core/src/psychology/` 도메인으로 완전 구현됨 (Phase 4.5, 207 테스트). 7층 NPC 심리: HEXACO 성격(①층), 3축 가치관(②층), 실천적 가치(③층), OCC 감정 22종(④층), PAD 기분(⑤층), 인지평가, NPC 프리셋 6종. `EmotionalBias`와 `RetrievalWeights` 구조는 유지되며, 심리 도메인과의 프롬프트 통합은 Phase 5에서 진행 예정.

### Step 2.4 — PromptContext → 시스템 프롬프트 기억 삽입 [wuxia-llm] ✅

프롬프트에 기억과 대화 요약 섹션을 추가했다.

**변경된 파일:**
- `wuxia-llm/src/prompt/template.rs` — PromptContext struct, build_system_prompt() 확장

> **🔄 현행화 (2026-03-03):** `PromptContext`가 `template.rs` → `prompt/types.rs`로 분리됨. 필드 확장: `memories`, `conversation_summaries` 외에 `relationship_summary`, `game_time`, `relationships`, `skip_affinity_directive` 등 추가. `format_memories_for_prompt()`는 `prompt/format.rs`로 분리됨. 프롬프트 구조가 XML 2계층 그룹핑으로 변경됨 (Persona → Current_Context → Directives). XML 태그 상수 14개 (`TAG_PERSONA` ~ `TAG_DIRECTIVES`) 사용.

**핵심 설계:**

```
  프롬프트 섹션 순서 (10개):
  1. 정체성 → 2. 기본정보 → 3. 성격 → 4. 말투 → 5. 대사예시
  → 6. 가치관 → 7. 배경 → 8. 관련기억 → 9. 이번대화요약 → 10. 지시
```

- PromptContext: memories(Vec<String>) + conversation_summaries(Vec<String>)
- format_memories_for_prompt(): RankedMemory → "(3일 전) 내용 [중요도: 7]" 변환
- 비어있는 섹션은 자동 생략 (기억 0건 → [관련 기억] 없음)
- [지시]는 항상 마지막 — LLM이 프롬프트 끝부분을 가장 잘 기억하므로

### Step 2.5 — ConversationManager (ctx 압축) [wuxia-llm] ✅

대화가 길어질 때 자동으로 오래된 턴을 요약하여 토큰을 절약한다.

**생성된 파일:**
- `wuxia-llm/src/conversation/manager.rs` (conversation.rs에서 이동)

**핵심 설계 — 단계별 압축:**

```
  0%           60%       75%       90%  95%  100%
  ├────────────┼─────────┼─────────┼────┼────┤
  │   None     │Compress │Compress │Comp│End │

  Compress: 가장 오래된 8턴 → LLM 요약 → 6턴 삭제 + 2턴 겹침 유지
  ForceEnd: NPC가 "다음에 또 보자~" 식으로 자연스럽게 마무리
```

- ConversationManager: LLM을 직접 호출하지 않음 (판정/추출만)
- estimate_token_count(): 문자 수 기반 토큰 추정 (한국어 1.5자≈1토큰, ASCII 4자≈1토큰)
- CompressAction: None / Compress / ForceEnd
- SummaryRequest: 요약 대상 텍스트 + 턴 수
- 3계층 기억 구조: 장기(과거 Observation) + 중기(미시 요약) + 단기(최근 원문)
- 28개 테스트 통과

### Step 2.6 — ChatSession (대화 루프 통합) [wuxia-llm] ✅

Sprint 2의 모든 조각을 하나로 연결하는 오케스트레이터.

**생성된 파일:**
- `wuxia-llm/src/conversation/mod.rs` — 모듈 선언 + re-export
- `wuxia-llm/src/conversation/session.rs` — ChatSession

**핵심 설계:**

```rust
// 게임 루프에서의 사용법
let mut session = ChatSession::new(llm, data, speech, locale, memories, ...);

loop {
    let reply = session.send("플레이어 입력")?;
    println!("NPC: {}", reply.npc_text);
    if reply.should_end { break; }
}

let observation = session.end()?;  // → MemoryRepository에 저장
```

- ChatSession<L: LlmPort> 제네릭 → MockLlm, LlamaCppAdapter 모두 사용 가능
- send() 내부: 압축 판정 → 요약 → 프롬프트 조립 → LLM 호출 → 파싱 → 턴 기록
- build_summary_request(): 요약 전용 프롬프트 (temperature 0.3, max_tokens 150)
- build_force_end_reminder(): NPC가 자연스럽게 대화 마무리하도록 지시
- ChatReply: npc_text + action + usage_ratio + turn_count + should_end
- end(): 짧은 대화(≤3턴) → 원문 반환, 긴 대화 → LLM 요약 호출
- 15개 테스트 통과 (기본 대화, 압축, ForceEnd, 세션 종료, 시나리오)

> **🔄 현행화 (2026-03-03):** `ChatSession<L: LlmPort>` → `ChatSession<L: LlmPort, C: ContextProvider>`로 확장됨. 추가 필드: `relationship: Option<Relationship>`, `descs: Option<RelationshipDescriptions>`, `sentiment_pipeline` (감정 판정), `memory_trace`. `ChatReply`에 `affinity_delta: i8`, `sentiment_detail` 필드 추가. `end()` 반환 타입: `String` → `SessionEndResult` (ObservationDraft + Relationship). `ObservationDraft` 구조체 추가 (대화 기억 후보). 짧은 대화(≤3턴)도 LLM 요약 수행하도록 변경됨. `ContextProvider` trait으로 기억 공급 책임 분리 (Null/Static/Live 구현체 3종).

---

## Phase B: LanceDB 교체 (Step 2.7~2.9) — ✅ 개발완료 (Sprint 3에서 구현)

| Step | 이름 | 상태 |
|------|------|------|
| 2.7 | LanceDB 연결 + 스키마 정의 | ✅ 개발완료 (Sprint 3 Step 3.2) |
| 2.8 | 임베딩 생성 + 벡터 검색 | ✅ 개발완료 (Sprint 3 Step 3.1 + 3.3) |
| 2.9 | soyeon_chat LanceDB 모드 | ✅ 개발완료 (Sprint 3 Step 3.6 Iter 2) |

> **✅ 현행화 (2026-03-03):** Phase B 전체 **개발완료**. Sprint 3에서 구현됨:
> - **2.7 LanceDB 연결:** `wuxia-memory/src/lancedb/` (mod.rs + arrow_convert.rs), lancedb 0.26.2, Arrow 57
> - **2.8 임베딩 + 벡터 검색:** `EmbeddingPort` trait (`wuxia-core/src/shared/embedding.rs`), BGE-M3 1024차원 (active default), 2-stage search (threshold + keyword overlap), `EmbeddingConfig` TOML 프로파일 기반
> - **2.9 soyeon_chat LanceDB:** `wuxia-app/examples/soyeon_chat_v2.rs` (LiveContextProvider + LanceDB 기억 영속)

---

## 최종 모듈 구조

```
  wuxia-core/src/
  ├── memory/
  │   ├── mod.rs           — re-exports
  │   ├── types.rs         — MemoryEntry, MemoryType, Reflection 트리
  │   ├── port.rs          — MemoryRepository trait, ScoredMemory
  │   ├── event.rs         — MemoryStored, MemoryRecalled 이벤트
  │   └── retrieval.rs     — retrieval_score(), rank_memories(), RetrievalWeights
  └── llm/                 — LlmPort trait (Sprint 1)

  wuxia-llm/src/
  ├── prompt/
  │   ├── mod.rs           — re-exports
  │   └── template.rs      — CharacterPromptData, PromptContext, build_system_prompt()
  ├── conversation/
  │   ├── mod.rs           — re-exports
  │   ├── manager.rs       — ConversationManager (ctx 압축 판정)
  │   └── session.rs       — ChatSession (대화 루프 오케스트레이터)
  ├── parser.rs            — parse_response()
  ├── mock.rs              — MockLlm (Fixed/Echo/Scripted)
  └── adapter/             — LlamaCppAdapter (feature "live-llm")

  wuxia-memory/src/
  └── in_memory.rs         — InMemoryRepository (HashMap 기반)
```

> **🔄 현행화 (2026-03-03):** 현재 모듈 구조 대폭 확장. 주요 추가:
> ```
>   wuxia-core/src/
>   ├── memory/             — +recall.rs, service.rs, service_tests.rs (8파일)
>   ├── relationship/       — 신규 13파일 (types, level, trust_level, relationship_type,
>   │                         event, port, effect, chronicle, description, sentiment 등)
>   ├── psychology/         — 신규 18파일 (HEXACO, 3축 가치관, OCC 감정, PAD 기분, 프리셋)
>   └── shared/             — 12파일 (+embedding.rs, port_error.rs, sentiment.rs)
>
>   wuxia-llm/src/
>   ├── prompt/             — 5파일 (template, types, format, fixtures, error)
>   ├── conversation/       — +context.rs (ContextProvider trait)
>   ├── sentiment/          — 신규 5파일 (2단계 하이브리드 감정 판정 파이프라인)
>   ├── quality/            — 신규 12파일 (시나리오 러너, 6 지표, LLM 채점기, 리포트)
>   └── text_utils.rs       — 신규 (한국어 텍스트 처리)
>
>   wuxia-memory/src/
>   ├── lancedb/            — 2파일 (mod.rs + arrow_convert.rs)
>   ├── embedding/          — 3파일 (mock, llamacpp_adapter, archived/)
>   ├── chronicle/          — 신규 3파일 (InMemory + JSONL 연대기 저장)
>   ├── relationship_store/ — 신규 3파일 (InMemory + JSON 관계 상태 저장)
>   └── config.rs, error.rs — 신규
> ```

---

## crate 간 연동 구조

```
  wuxia-core (도메인)              wuxia-llm (LLM)             wuxia-memory (저장소)
  ══════════════════              ════════════                 ═════════════════════

  MemoryEntry struct ◄───── ChatSession이 Observation 생성
  MemoryRepository trait ◄─ ChatSession이 사용 (향후) ─────► InMemoryRepository
  retrieval_score() ◄────── rank_memories() 호출               (향후 LanceDb)
  RankedMemory ◄─────────── format_memories_for_prompt()
  RetrievalWeights ◄─────── NPC별 가중치 튜닝
  EmotionalBias ◄────────── OCC_TODO⑤ 예약

  LlmPort trait ◄────────── MockLlm / LlamaCppAdapter
  LlmRequest ◄──────────── ChatSession이 조립
  PromptContext ◄────────── memories + conversation_summaries

  의존성 방향:
    wuxia-core ← wuxia-llm (LlmPort 구현)
    wuxia-core ← wuxia-memory (MemoryRepository 구현)
    wuxia-llm은 wuxia-memory를 모른다 (게임 루프에서 조립)
```

---

## OCC 감정 통합 예약 (OCC_TODO⑤)

Sprint 2에서 구조적 훅을 미리 심어두었다:

| 위치 | 예약 내용 | 현행화 상태 |
|------|----------|------------|
| `RetrievalWeights.emotional_bias_weight` | 감정 편향 가중치 (δ) — 현재 0.0 | ✅ 구조 유지, 값은 미활성 |
| `EmotionalBias` struct | PAD 기반 기분 일치 편향 — 구조 정의만 | ✅ 구조 유지, 심리 도메인에서 PAD 완전 구현 |
| `rank_memories()` emotional_bias 파라미터 | Option<EmotionalBias> — 현재 None 전달 | ✅ 구조 유지 |
| `PromptContext` | OCC_TODO: current_emotion 필드 추가 예정 | 🔄 미추가 (프롬프트 통합은 Phase 5) |
| `format_memories_for_prompt()` | OCC_TODO: 감정 태그 추가 예정 | 🔄 미추가 (Phase 5) |

> **🔄 현행화 (2026-03-03):** OCC 감정 시스템의 **도메인 로직은 완전 구현됨** (`wuxia-core/src/psychology/` — Phase 4.5, 207 테스트). 22종 감정, PAD 기분, 인지평가(`appraise_to_emotions`), 감정 감쇠(`decay_emotion`), HEXACO 필터(`hexaco_emotion_filter`), NPC 프리셋 6종. 다만 기억 검색과 프롬프트에 감정을 **연결하는 통합 작업**(EmotionalBias 활성화, PromptContext current_emotion)은 Phase 5(Bevy 통합)에서 진행 예정.

---

## Sprint 1과의 관계

Sprint 1에서 확인한 Gap:
- 🔴 기억/감정이 프롬프트에 전혀 없음
- 🔴 대화가 길어지면 ctx 오버플로

Sprint 2에서 메운 Gap:
- ✅ 과거 기억을 프롬프트에 삽입 (PromptContext.memories)
- ✅ 대화 중 미시 요약으로 ctx 자동 압축 (ConversationManager)
- ✅ 대화 종료 시 Observation 생성 (ChatSession.end())
- ✅ NPC별 기억 검색 가중치 튜닝 (RetrievalWeights)

남은 Gap (Sprint 3 이후):
- ⬜ Character/Growth → 프롬프트 자동 변환
- 🔄 OCC 감정 상태 → 프롬프트 + 기억 검색 반영
- ✅ LanceDB 벡터 검색 (의미 기반 기억 검색)
- ✅ soyeon_chat 예제에 기억 시스템 실전 연결

> **✅ 현행화 (2026-03-03):**
> - **LanceDB 벡터 검색** → ✅ 개발완료 (Sprint 3 Step 3.2~3.3, BGE-M3 1024차원, 2-stage search)
> - **soyeon_chat 기억 연결** → ✅ 개발완료 (Sprint 3 Step 3.6, LiveContextProvider + LanceDB 영속)
> - **OCC 감정 상태** → 🔄 도메인 로직 완전 구현 (Phase 4.5 심리 도메인 207 테스트), 프롬프트/기억 통합은 Phase 5
> - **Character/Growth → 프롬프트** → ⬜ 미구현 (Phase 5 예정)

---

## 변경 이력

| 버전 | 변경일시 | 변경 내역 |
|------|----------|-----------|
| v0.1.0 | 2026-02-20 04:00:00 | 초기 작성. 9 Step 계획 (Phase A 6 Step + Phase B 3 Step) |
| v1.0.0 | 2026-02-21 04:30:00 | Phase A 전체 완료 반영. Step 2.1~2.6 상세 기록. 모듈 구조 최종 정리. OCC 예약 현황 추가. 테스트 약 170개 |
| v1.0.1 | 2026-03-03 | **현행화.** Phase B(2.7~2.9) `✅ 개발완료` (Sprint 3에서 구현). OCC 감정 → 심리 도메인 구현 완료(Phase 4.5), 프롬프트 통합 Phase 5. memory/ 8파일 확장 주석. PromptContext types.rs 분리, XML 프롬프트 구조 변경. ChatSession 2축 제네릭(+ContextProvider), SessionEndResult, 감정 판정. 모듈 구조 대폭 확장 (relationship 13파일, psychology 18파일, sentiment 5파일, quality 12파일, chronicle/relationship_store 신규). 테스트 ~170→~1,463. |
