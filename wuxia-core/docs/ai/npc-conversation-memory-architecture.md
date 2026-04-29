# NPC 대화 기억 아키텍처

**버전:** v1.2
**수정일:** 2026-03-01

> **역할**: NPC 대화 기억 시스템의 구현 아키텍처 단일 원본 (Single Source of Truth)  
> 📎 NPC 심리 아키텍처: [npc-psychology-architecture.md](../../psychology/npc-psychology-architecture.md) §7 성찰
> 📎 기억 중요도 라벨: [memory-importance-label-design.md](../../design/memory-importance-label-design.md)
> 📎 임베딩 모델 선정: [step3.1-embedding-benchmark-report.md](../../plans/step3.1-embedding-benchmark-report.md)
> 📎 임베딩 threshold: [step3.3-threshold-analyzer-report.md](../../plans/step3.3-threshold-analyzer-report.md)
> 📎 Sprint 3 진행: [sprint3-progress.md](../../plans/sprint3-progress.md)
> 📎 감정 판정 벤치마크: [step4.2-sentiment-benchmark-report.md](../benchmarks/step4.2-sentiment-benchmark-report.md)

---

## 1. 개요 — "소연이 영원히 기억한다"

이 문서는 NPC가 플레이어와의 대화를 기억하고, 그 기억이 이후 대화에 영향을 미치는
전체 시스템의 구현 아키텍처를 설명한다.

### 1.1 핵심 목표

```
  1회차: 플레이어가 "혈교가 뭐야?" 질문 → 소연이 대답 → 프로그램 종료
  2회차: 플레이어가 "저번에 사파 얘기 했잖아" → 소연이 혈교 맥락을 기억하고 대답
```

이를 위해 세 가지가 필요하다:

```
  ① 기억 저장 — 대화가 끝나면 요약+중요도를 벡터 DB에 영구 저장
  ② 기억 검색 — 매 턴마다 플레이어 입력과 의미적으로 관련된 기억을 찾음
  ③ 기억 반영 — 찾은 기억을 프롬프트에 삽입하여 LLM 응답에 반영
```

### 1.2 전체 흐름 (한눈에 보기)

```
  ┌─────────────────────────────────────────────────────────────────┐
  │                    NPC 대화 기억 전체 흐름                        │
  │                                                                  │
  │  ┌─────────┐     ┌──────────────┐     ┌──────────┐              │
  │  │ 플레이어 │────►│ ChatSession  │────►│ LLM(4B)  │              │
  │  │  입력    │     │ (오케스트라) │◄────│  응답    │              │
  │  └─────────┘     └──────┬───────┘     └──────────┘              │
  │                         │                                        │
  │              ┌──────────┼──────────┐                             │
  │              ▼          ▼          ▼                             │
  │  ┌───────────────┐ ┌────────┐ ┌──────────────┐                  │
  │  │ContextProvider│ │ Parser │ │Conversation   │                  │
  │  │(기억+관계)     │ │(태그)  │ │Manager(압축)  │                  │
  │  └───────┬───────┘ └────────┘ └──────────────┘                  │
  │          │                                                       │
  │    ┌─────┴──────┐                                                │
  │    ▼            ▼                                                │
  │  ┌──────┐  ┌───────────┐                                        │
  │  │벡터  │  │ 관계 상태  │                                        │
  │  │검색  │  │ (2축)      │                                        │
  │  └──┬───┘  └───────────┘                                        │
  │     │                                                            │
  │     ▼                                                            │
  │  ┌───────────────┐  ┌───────────┐                                │
  │  │ Embedding      │──►│  LanceDB  │  ← 디스크 영속               │
  │  │ KO: BGE-M3     │  │ (벡터 DB) │                               │
  │  │    (1024차원)   │  └───────────┘                               │
  │  │ EN: Gemma       │                                              │
  │  │    (768차원)    │                                              │
  │  └───────────────┘                                               │
  └─────────────────────────────────────────────────────────────────┘
```

### 1.3 대화 한 턴의 시간 흐름

플레이어가 한 마디 입력했을 때, 각 컴포넌트가 어떤 순서로 동작하는지 보여준다.

```
  Player     ChatSession  ContextProvider  Embedding   LanceDB   Ranker    LLM(4B)   Parser
    │             │              │              │          │         │         │         │
    │ "사파 얘기  │              │              │          │         │         │         │
    │  했잖아"   │              │              │          │         │         │         │
    ├────────────►│              │              │          │         │         │         │
    │             │              │              │          │         │         │         │
    │             │  기억 검색 요청              │          │         │         │         │
    │             ├─────────────►│              │          │         │         │         │
    │             │              │              │          │         │         │         │
    │             │              │  텍스트→벡터 변환        │         │         │         │
    │             │              ├─────────────►│          │         │         │         │
    │             │              │  벡터 (KO:1024d/EN:768d)│         │         │         │
    │             │              │◄─────────────┤          │         │         │         │
    │             │              │              │          │         │         │         │
    │             │              │  벡터 유사도 검색 (후보 10개)       │         │         │
    │             │              ├─────────────────────────►│         │         │         │
    │             │              │  유사 기억 후보들        │         │         │         │
    │             │              │◄─────────────────────────┤         │         │         │
    │             │              │              │          │         │         │         │
    │             │              │  4축 랭킹 (최신성+중요도+관련도+감정)         │         │
    │             │              ├──────────────────────────────────►│         │         │
    │             │              │  상위 5개 + 행동 지시 라벨         │         │         │
    │             │              │◄──────────────────────────────────┤         │         │
    │             │              │              │          │         │         │         │
    │             │  기억 목록   │              │          │         │         │         │
    │             │  + 관계 설명 │              │          │         │         │         │
    │             │◄─────────────┤              │          │         │         │         │
    │             │              │              │          │         │         │         │
    │             │  프롬프트 조립                │          │         │         │         │
    │             │  (캐릭터+기억+관계+대화이력+지시)         │         │         │         │
    │             │              │              │          │         │         │         │
    │             │  NPC 응답 생성               │          │         │         │         │
    │             ├──────────────────────────────────────────────────►│         │
    │             │              │              │          │         │ 캐릭터   │         │
    │             │              │              │          │         │ 응답     │         │
    │             │◄──────────────────────────────────────────────────┤         │
    │             │              │              │          │         │         │         │
    │             │  태그 추출 + 정리             │          │         │         │         │
    │             ├────────────────────────────────────────────────────────────►│
    │             │  대사 텍스트 + 호감도 변화    │          │         │         │         │
    │             │◄────────────────────────────────────────────────────────────┤
    │             │              │              │          │         │         │         │
    │  소연 응답  │              │              │          │         │         │         │
    │◄────────────┤              │              │          │         │         │         │
```

### 1.4 대화 종료 시 기억 저장 흐름

대화가 끝나면(`/quit`), 이번 대화 전체를 하나의 기억으로 요약하여 영구 저장한다.

```
  Player     ChatSession       LLM(4B)        Parser       Embedding    LanceDB
    │             │               │              │              │          │
    │ /quit       │               │              │              │          │
    ├────────────►│               │              │              │          │
    │             │               │              │              │          │
    │             │  대화 요약 + 중요도 평가 요청 │              │          │
    │             ├──────────────►│              │              │          │
    │             │               │              │              │          │
    │             │  "혈교에 대해 │              │              │          │
    │             │   물었고..."  │              │              │          │
    │             │  [importance: │              │              │          │
    │             │   7.5]        │              │              │          │
    │             │◄──────────────┤              │              │          │
    │             │               │              │              │          │
    │             │  요약에서 중요도 숫자 추출    │              │          │
    │             ├──────────────────────────────►│              │          │
    │             │  요약 텍스트 + 중요도 7.5     │              │          │
    │             │◄──────────────────────────────┤              │          │
    │             │               │              │              │          │
    │             │  기억 저장 (요약→벡터 변환 후 저장)          │          │
    │             ├─────────────────────────────────────────────►│          │
    │             │               │              │  벡터 (1024d/768d) │     │
    │             │               │              │◄─────────────┤          │
    │             ├────────────────────────────────────────────────────────►│
    │             │               │              │              │  💾 영속  │
    │             │◄────────────────────────────────────────────────────────┤
    │             │               │              │              │          │
    │  "소연의    │               │              │              │          │
    │   기억 저장 │               │              │              │          │
    │   완료"    │               │              │              │          │
    │◄────────────┤               │              │              │          │
    │             │               │              │              │          │
    │  (프로그램 종료)             │              │              │          │
    │             │               │              │              │          │
    │  ─── 다음날 다시 실행 ───   │              │              │          │
    │             │               │              │              │          │
    │ "어제 무슨  │               │              │              │          │
    │  얘기했지?"│               │              │              │          │
    │  ─► (§1.3 흐름 반복, 어제 저장한 기억이 검색됨) ──►       │          │
```

---

## 2. 계층 아키텍처 — 4개 crate의 역할

기억 시스템은 헥사고날 아키텍처 원칙에 따라 4개 crate에 분산된다.

```
  의존성 방향 (화살표 = "~를 안다"):

  wuxia-core (도메인 모델, port trait 정의)
       ▲           ▲           ▲
       │           │           │
  wuxia-data   wuxia-llm   wuxia-memory
  (TOML 로딩)  (LLM 어댑터) (저장소 어댑터)
       ▲           ▲           ▲
       └───────────┼───────────┘
                   │
              wuxia-app
              (조립 계층 — 타입 변환, 크로스 어댑터 오케스트레이션)

  ※ wuxia-llm과 wuxia-memory는 서로 독립 (헥사고날 원칙).
     조합은 wuxia-app이 담당 (예: LiveContextProvider).
```

### 2.1 각 crate의 기억 관련 책임

| crate | 파일 | 책임 |
|-------|------|------|
| **wuxia-core** | `memory/types.rs` | MemoryEntry, MemoryType, ScoredMemory 데이터 구조 |
| | `memory/port.rs` | MemoryRepository trait (헥사고날 출력 포트) |
| | `memory/embedding.rs` | EmbeddingPort trait (텍스트→벡터 변환 포트) |
| | `memory/retrieval.rs` | retrieval_score(), rank_memories() 순수 함수 |
| | `memory/event.rs` | MemoryEvent 도메인 이벤트 |
| | `relationship/types.rs` | Relationship 2축 (호감 -100~+100 / 신뢰 0~100) |
| | `shared/prompt_config.rs` | PromptConfig, MemoryLabelsConfig 타입 정의 |
| **wuxia-memory** | `in_memory.rs` | InMemoryRepository (테스트용) |
| | `lancedb.rs` | LanceDbRepository (영속 저장소, 벡터 검색) |
| | `embedding/mock.rs` | MockEmbedding (테스트용 결정론적 벡터) |
| | `embedding/llamacpp_adapter.rs` | LlamaCppEmbedding (실제 임베딩 모델) |
| | `config.rs` | EmbeddingConfig (TOML 설정 파서) |
| **wuxia-data** | `prompt_config.rs` | prompt_config.toml 로딩 |
| | `relationship_desc.rs` | descriptions.toml 로딩 (관계 자연어 설명) |
| **wuxia-llm** | `conversation/session.rs` | ChatSession (대화 오케스트레이터) |
| | `conversation/context.rs` | ContextProvider trait + Null/Static/Live 구현체 |
| | `parser.rs` | 응답 파싱 ([affinity: N] 태그 추출) |
| | `prompt/template.rs` | build_system_prompt(), 프롬프트 조립 |
| | `sentiment/` | 2단계 하이브리드 감정 판정 파이프라인 |
| **wuxia-app** | `context.rs` | LiveContextProvider (도메인 랭킹 + 어댑터 포맷팅 조립) |

---

## 3. 데이터 모델 — 기억의 구조

### 3.1 MemoryEntry (하나의 기억)

```rust
pub struct MemoryEntry {
    id: MemoryId,              // 고유 식별자
    character_id: CharacterId, // 이 기억의 주인 NPC
    content: String,           // 기억 내용 ("플레이어가 혈교에 대해 물었다")
    importance: f32,           // 중요도 1.0~10.0
    memory_type: MemoryType,   // Observation / Reflection / Plan
    game_time: GameTime,       // 게임 내 시간 (년/월/일)
    keywords: Vec<String>,     // 키워드 ["혈교", "사파"]
    source_ids: Vec<MemoryId>, // 이 기억의 원천 (성찰→관찰 추적)
    reflection_tier: u8,       // 성찰 깊이 (0=관찰, 1=1차성찰, ...)
    lang: String,              // 언어 코드 "KO" / "EN"
}
```

### 3.2 기억의 세 종류 (MemoryType)

```
  Observation (관찰)  ← 대화 종료 시 자동 생성
  ──────────────────
  "플레이어가 혈교의 배후에 대해 물었다.
   소연은 혈교 습격의 기억을 떠올리며 감정적으로 반응했다."
  importance: 7.5 (LLM이 산정)
  
  Reflection (성찰)  ← 향후 구현 (하루 끝, Tier 2~4)
  ──────────────────
  "플레이어는 사파에 관심이 많다. 혹시 혈교와 관련이 있는 건 아닐까?"
  source_ids: [관찰 기억 3개]
  
  Plan (계획)  ← 향후 구현
  ────────────
  "플레이어의 사파 관심을 좀 더 관찰하겠다."
  source_ids: [위 성찰]
```

### 3.3 LanceDB 저장 스키마

```
  npc_memories 테이블:
  
  ┌────────────────┬────────────┬─────────────────────────────┐
  │ 컬럼           │ Arrow 타입  │ 설명                        │
  ├────────────────┼────────────┼─────────────────────────────┤
  │ id             │ UInt64     │ MemoryId                    │
  │ character_id   │ UInt64     │ NPC 식별자                   │
  │ content        │ Utf8       │ 기억 내용                    │
  │ importance     │ Float32    │ 1.0~10.0                    │
  │ memory_type    │ Utf8       │ "Observation"/"Reflection"  │
  │ game_year      │ Int32      │ 게임 연도                    │
  │ game_month     │ Int32      │ 게임 월                      │
  │ game_day       │ Int32      │ 게임 일                      │
  │ keywords       │ Utf8       │ JSON 배열 문자열             │
  │ source_ids     │ Utf8       │ JSON 배열 문자열             │
  │ reflection_tier│ UInt8      │ 성찰 깊이                    │
  │ lang           │ Utf8       │ "KO" / "EN" / "ZH"         │
  │ vector         │ Vector(*)  │ 임베딩 벡터 (KO:1024d, EN:768d) │
  └────────────────┴────────────┴─────────────────────────────┘
  
  GameTime을 3개 컬럼으로 분리하는 이유:
  → LanceDB에서 범위 쿼리 가능 (game_year > 1200)
```

---

## 4. 대화 중 기억 검색 — send() 흐름

플레이어가 한 마디 할 때마다 아래 파이프라인이 실행된다.

### 4.1 전체 파이프라인 (6단계)

```
  플레이어: "저번에 사파 얘기 했잖아"
                │
                ▼
  ┌──────────────────────────────────────────────────────────────┐
  │  Stage 1: 벡터 임베딩                                         │
  │  KO: "사파 얘기" → BGE-M3 → [0.12, -0.34, ...] (1024d)      │
  │  EN: "Sapa story" → EmbeddingGemma → [...] (768d)            │
  │  소요: ~7ms (BGE-M3, CPU) / ~28ms (Gemma, CPU)               │
  └──────────────────────┬─────────────────────────────────────┘
                         ▼
  ┌──────────────────────────────────────────────────────────┐
  │  Stage 2: 벡터 유사도 검색 (LanceDB)                       │
  │  cosine similarity → 후보 10개 반환 (search_top_k=10)     │
  │  2-Stage 필터:                                             │
  │    1차: cosine_sim >= threshold(0.4656) → 통과             │
  │    2차: sim >= boost_threshold? → keyword 없어도 OK        │
  │         sim <  boost_threshold? → keyword overlap 필요     │
  └──────────────────────┬───────────────────────────────────┘
                         ▼
  ┌──────────────────────────────────────────────────────────┐
  │  Stage 3: 4축 랭킹 (rank_memories)                        │
  │  score = α₁×recency + α₂×importance                      │
  │        + α₃×relevance + α₄×emotional_match               │
  │                                                            │
  │  기본 가중치:                                               │
  │    recency=1.0, importance=1.0, relevance=2.0             │
  │    emotional_bias=0.0 (OCC 미구현)                         │
  │                                                            │
  │  상위 5개 선택 (rank_top_k=5)                              │
  └──────────────────────┬───────────────────────────────────┘
                         ▼
  ┌──────────────────────────────────────────────────────────┐
  │  Stage 4: 행동 지시 라벨 포맷팅                             │
  │                                                            │
  │  importance → 5단계 라벨 변환:                              │
  │    9.0+ → "말을 끊거나 침묵해라"                            │
  │    7.0+ → "말투가 흔들려라"                                 │
  │    5.0+ → "회상하듯 말해라"                                 │
  │    3.0+ → "사실만 담담히 말해라"                             │
  │    1.0+ → "얼버무려라"                                      │
  │                                                            │
  │  시간 포맷 + 라벨 결합:                                     │
  │  "(3일 전) 혈교 무인이 수상한 움직임을 보였다.              │
  │    [사실만 담담히 말해라]"                                   │
  └──────────────────────┬───────────────────────────────────┘
                         ▼
  ┌──────────────────────────────────────────────────────────┐
  │  Stage 5: 프롬프트 조립 (build_system_prompt)              │
  │  시스템 프롬프트에 [관련 기억] 섹션으로 삽입               │
  │  (§5에서 상세 설명)                                        │
  └──────────────────────┬───────────────────────────────────┘
                         ▼
  ┌──────────────────────────────────────────────────────────┐
  │  Stage 6: LLM 생성 + 파싱                                 │
  │  gemma3:4b가 기억을 참조하여 캐릭터 응답 생성              │
  │  [affinity: N] 태그 추출 → 관계 변화 추적                 │
  └──────────────────────────────────────────────────────────┘
```

### 4.2 ContextProvider — 3계층 전략 패턴

ChatSession은 컨텍스트 공급원을 trait으로 추상화한다.
개발 단계에 따라 구현체를 교체하며, 코드 변경 없이 기능이 확장된다.

```
  ContextProvider trait
  ─────────────────────
  search_memories(query: &str) → Vec<String>   // 포맷팅된 기억 목록
  relationship_summary() → Option<String>       // 관계 자연어 설명


  구현체 3종 (점진적 복잡도):
  ┌─────────────────────────────────────────────────────────┐
  │                                                          │
  │  NullContextProvider (테스트)                             │
  │  ─────────────────────                                   │
  │  search_memories() → vec![]                              │
  │  relationship_summary() → None                           │
  │  용도: 기억/관계 없이 순수 대화 로직 테스트              │
  │                                                          │
  │  StaticContextProvider (Sprint 2 호환)                    │
  │  ────────────────────────────                             │
  │  search_memories() → 고정 Vec<String> 반환               │
  │  relationship_summary() → 고정 Option<String> 반환       │
  │  용도: 기억을 외부에서 미리 넣어두는 간단한 데모         │
  │                                                          │
  │  LiveContextProvider<R: MemoryRepository> (Sprint 3)     │
  │  ──────────────────────────────────────────               │
  │  search_memories() → 벡터 검색 → 4축 랭킹 → 라벨 포맷팅 │
  │  relationship_summary() → 2축 관계 자연어 설명           │
  │  용도: 실제 게임 대화 (LanceDB + 실시간 검색)            │
  │  위치: wuxia-app (조립 계층 — 도메인 함수 사용으로 분리) │
  │                                                          │
  └─────────────────────────────────────────────────────────┘
```

### 4.3 LiveContextProvider 내부 구조

```rust
pub struct LiveContextProvider<R: MemoryRepository> {
    repo: R,                    // 벡터 DB (LanceDb or InMemory)
    character_id: CharacterId,  // NPC 식별자
    current_time: GameTime,     // 현재 게임 시간
    weights: RetrievalWeights,  // 4축 가중치 (NPC 성격별 조절 가능)
    search_top_k: usize,       // 벡터 검색 후보 수 (기본 10)
    rank_top_k: usize,         // 최종 선택 수 (기본 5)
    relationship: Relationship, // 현재 관계 스냅샷 (owned, 세션 중 불변)
    descs: RelationshipDescriptions, // 관계 수준별 자연어 설명
    locale: Locale,             // 언어 (Ko/En)
    prompt_config: PromptConfig,// 프롬프트 설정 (라벨, 포맷)
}
```

설계 결정:
- **제네릭 `R: MemoryRepository`** — 정적 디스패치, InMemory/LanceDb 교체 자유
- **relationship owned** — 세션 중 불변 스냅샷, 참조 수명 복잡도 회피
- **top_k 분리** — search_top_k=10 (후보 넓게), rank_top_k=5 (최종 좁게)

---

## 5. 프롬프트 구성 — build_system_prompt()

### 5.1 프롬프트 11개 섹션 구조

LLM에게 전달되는 system prompt는 11개 섹션으로 구성된다.
모든 섹션 헤더와 템플릿은 prompt_config.toml에서 로딩한다 (코드 하드코딩 없음).

```
  ┌─────────────────────────────────────────────────┐
  │            LLM System Prompt 구조               │
  │                                                  │
  │  ① 정체성                                        │
  │     "너는 소연(素燕)이다.                        │
  │      별호는 천이(千耳)."                         │
  │                                                  │
  │  ② [기본 정보]                                   │
  │     "23세 여. 개방 소속.                         │
  │      자유도시에서 천이방을 운영하는 정보상."      │
  │                                                  │
  │  ③ [성격]                                        │
  │     "외향적이고 에너지가 넘친다..."              │
  │                                                  │
  │  ④ [말투 규칙]                                   │
  │     "해요체 + 반말 혼용..."                      │
  │     금기어: AI, 모델, 프롬프트, ...              │
  │                                                  │
  │  ⑤ [대사 예시]                                   │
  │     "「어머, 손님! 뭘 찾으세요?」"               │
  │                                                  │
  │  ⑥ [가치관]                                      │
  │     "복수심이 강하다..."                         │
  │                                                  │
  │  ⑦ [배경]                                        │
  │     "11세에 혈교 습격으로 개방 형제를 잃었다..." │
  │                                                  │
  │  ⑧ [관계 상태]  ← ContextProvider 공급           │
  │     "상대와의 관계: 친근                         │
  │      소연은 상대에게 호감을 느끼고 있으며,       │
  │      기본적으로 신뢰하는 편이다."                │
  │                                                  │
  │  ⑨ [관련 기억]  ← ContextProvider 공급           │
  │     "(어제) 플레이어가 혈교에 대해 물었다.       │
  │       [말투가 흔들려라]                           │
  │     (3일 전) 서문에서 수상한 사내를 보았다.      │
  │       [사실만 담담히 말해라]"                     │
  │                                                  │
  │  ⑩ [이번 대화 요약]                              │
  │     "(ConversationManager 미시요약 결과)"        │
  │                                                  │
  │  ⑪ [지시]  ← 항상 마지막 (LLM이 가장 잘 기억)   │
  │     "소연으로서 대화한다.                        │
  │      절대 AI임을 밝히지 않는다.                  │
  │      플레이어의 말에 소연의 성격대로 반응한다.   │
  │      한국어로 대답한다."                         │
  │                                                  │
  └─────────────────────────────────────────────────┘
```

### 5.2 4계층 데이터 소스

프롬프트 빌더는 4계층의 데이터를 순수 함수로 조립한다:

```
  ┌─────────────────────────────────────────────────────────┐
  │  1계층: CharacterPromptData (캐릭터 정적 데이터)          │
  │  ───────────────────────────────────────                  │
  │  이름, 나이, 성격, 가치관, 배경 ...                       │
  │  출처: NPC 열전 (npc-*.md)               │
  │  빈도: 게임 시작 시 1회 로딩                              │
  │                                                          │
  │  2계층: SpeechRules (말투 규칙)                            │
  │  ────────────────────────────                              │
  │  대사 예시, 금기어 목록                                    │
  │  출처: 언어별 창작 (번역 불가능 영역)                      │
  │  빈도: 캐릭터별 1벌                                        │
  │                                                          │
  │  3계층: PromptContext (동적 컨텍스트)  ← 매 턴 변경       │
  │  ────────────────────────────────────                      │
  │  memories: Vec<String>         ← ContextProvider가 공급   │
  │  conversation_summary: Option  ← ConversationManager 압축 │
  │  relationship_summary: Option  ← ContextProvider가 공급   │
  │  system_reminder: Option       ← 드리프트 방지 규칙 상기  │
  │                                                          │
  │  4계층: LanguageDirective (언어 지시)                      │
  │  ──────────────────────────────                            │
  │  "한국어로 대답한다."                                      │
  │  출처: prompt_config.toml                                 │
  └─────────────────────────────────────────────────────────┘
```

### 5.3 관계 상태 프롬프트 생성

2축 관계값 → 자연어 설명 변환 과정:

```
  Relationship { affinity: 55.0, trust: 35.0 }
       │
       ▼
  RelationshipLevel 판정 (affinity + trust 기반)
  → affinity>=50 AND trust>=30 → Friendly
       │
       ▼
  descriptions.toml 조회
  → level.Friendly.ko = "소연은 상대에게 호감을 느끼고 있으며, ..."
  → trust.cautious.ko = "기본적으로 신뢰하는 편이다."
       │
       ▼
  format_relationship_for_prompt() 조립 (wuxia-llm)
  → RelationshipView { level_label, level_desc, trust_desc }
  → "상대와의 관계: 친근
     소연은 상대에게 호감을 느끼고 있으며, ...
     기본적으로 신뢰하는 편이다."

  ※ 적대 상태는 affinity 음수값(-100~0)으로 표현.
     별도 hostility 축 없이, affinity < -30 → Wary/Hostile/Enemy 판정.
```

### 5.4 기억 프롬프트 생성

RankedMemory → 포맷팅된 문자열 변환 과정:

```
  RankedMemory {
      entry: { content: "혈교 습격으로 형제를 잃었다",
               importance: 9.2,
               game_time: (1188, 5, 3) },
      final_score: 4.2
  }
       │
       ▼
  시간 포맷팅 (현재 1200년 3월 15일 기준)
  → days_between = 약 4330일 → "12년 전"
       │
       ▼
  중요도 라벨 변환 (importance 9.2 → level_5)
  → "말을 끊거나 침묵해라"
       │
       ▼
  entry 포맷 적용 (prompt_config.toml [memory_format.ko])
  → "({time}) {content}\n  [{label}]"
  → "(12년 전) 혈교 습격으로 형제를 잃었다.
       [말을 끊거나 침묵해라]"
```

### 5.5 행동 지시형 라벨의 설계 의도

라벨은 LLM이 읽는 **행동 명령**이다. 인간 독자용이 아니다.

```
  왜 "각인된 기억"이 아니라 "말을 끊거나 침묵해라"인가?

  12B 모델:
    입력: "각인된 기억" → 추론: "이건 아주 중요하니까 감정적으로 반응해야지"
    → 성공 (추상→행동 추론 가능)

  4B 모델:
    입력: "각인된 기억" → 추론: "???" → 무시하거나 메타 태그를 대사에 노출
    입력: "말을 끊거나 침묵해라" → 즉시 적용: "............그 얘긴 하지 마."
    → 성공 (추론 단계 0, 직접 실행)

  결론: 라벨에서 추론 부담을 제거하면 4B에서도 동작한다.
```

### 5.6 토큰 예산

```
  RTX 2070S + gemma3:4b 기준 (ctx 8192):

  ┌──────────────────────┬────────┬────────┐
  │ 섹션                  │ 토큰   │ 비율   │
  ├──────────────────────┼────────┼────────┤
  │ ①~⑦ 정적 캐릭터 정보 │ ~800   │ ~10%  │
  │ ⑧ 관계 상태           │ ~80    │ ~1%   │
  │ ⑨ 관련 기억 (5개)     │ ~250   │ ~3%   │
  │ ⑩ 대화 요약           │ ~200   │ ~2%   │
  │ ⑪ 지시                │ ~60    │ ~1%   │
  │ 대화 이력             │ ~4000  │ ~49%  │
  │ LLM 응답 여유         │ ~2800  │ ~34%  │
  ├──────────────────────┼────────┼────────┤
  │ 합계                  │ 8192   │ 100%  │
  └──────────────────────┴────────┴────────┘

  기억 5개 × 라벨 ≈ 250 토큰 (ctx의 ~3%)
  → 비용 대비 효과 매우 높음
```

---

## 6. 대화 종료 시 기억 저장 — end() 흐름

### 6.1 ObservationDraft 생성

```
  session.end() 호출
       │
       ▼
  build_final_summary_request()
  ── LLM에게 요약+중요도 동시 요청 (1회 호출):
  │
  │  프롬프트:
  │  "너는 대화 요약 도우미다.
  │   1. 핵심 내용을 2~3문장으로 요약해라.
  │   2. 마지막 줄에 중요도를 1~10으로 평가해라.
  │   중요도 기준:
  │     1~3: 일상 잡담
  │     4~6: 정보 교환
  │     7~8: 관계 변화
  │     9~10: 극적 사건
  │   형식:
  │   [요약 내용]
  │   [importance: N]"
  │
       ▼
  parse_summary_with_importance()
  ── 응답에서 [importance: N] 파싱:
  │  성공 → (요약 텍스트, N)
  │  실패 → (전체 텍스트, 5.0)  // 안전 기본값
  │
       ▼
  ObservationDraft {
      summary: "플레이어가 혈교에 대해 물었다. 소연은 ...",
      importance: 7.5,
      turn_count: 8,
      had_compression: true,
  }
```

### 6.2 ObservationDraft → MemoryEntry → LanceDB

```
  ObservationDraft
       │
       ▼
  MemoryEntry::new(
      next_id(),
      npc_character_id,
      draft.summary,         // 요약 텍스트
      draft.importance,      // LLM이 평가한 중요도
      MemoryType::Observation,
      current_game_time,
      extract_keywords(&draft.summary),  // 키워드 자동 추출
  )
       │
       ▼
  LanceDbRepository::save(entry)
  │  1. embedder.embed(content)  → 벡터 (KO:1024d / EN:768d)
  │  2. entry + 벡터 → Arrow RecordBatch
  │  3. table.add(batch)         → 디스크에 영속
       │
       ▼
  💾 LanceDB 파일 (./data/memory.lance)
  → 프로그램을 종료해도 기억이 남는다
```

---

## 7. 관계 반영 — affinity_delta 흐름

### 7.1 매 턴 호감도 변화 추적

LLM 응답에 `[affinity: N]` 태그를 삽입하도록 프롬프트에서 지시한다.

```
  LLM 응답 원문:
  "어머, 그 얘기를 아는 거야? 꽤 잘 알고 있네! [affinity: +2]"
       │
       ▼
  parse_response_with_tags()
  │  text: "어머, 그 얘기를 아는 거야? 꽤 잘 알고 있네!"  ← 태그 제거
  │  affinity_delta: +2                                     ← 추출
       │
       ▼
  ChatReply {
      text: "어머, 그 얘기를 아는 거야? 꽤 잘 알고 있네!",
      affinity_delta: 2,  // i8, -5~+5 범위 clamp
      turn_index: 3,
      compressed: false,
  }
```

### 7.2 태그 규칙

```
  형식:     [affinity: N] 또는 [Affinity: N]  (대소문자 무관)
  범위:     -5 ~ +5 (초과 시 clamp)
  태그 없음: affinity_delta = 0
  태그 2개:  마지막 값만 사용
  위치:     응답 어디에나 (모두 제거)
  
  플레이어에게는 태그가 보이지 않는다 (항상 제거 후 반환)
```

### 7.3 관계 업데이트

```
  ChatReply.affinity_delta = +2
       │
       ▼
  relationship.update_affinity(+2.0)
  │  affinity: 55.0 → 57.0  (-100~+100 clamp)
       │
       ▼
  대화 종료 시:
  │  총 누적 delta → relationship 영구 반영
  │  RelationshipEvent::AffinityChanged 발행
  │  RelationshipRepository (JSON) + ChronicleRepository (JSONL)에 저장
```

### 7.4 2단계 하이브리드 감정 판정 파이프라인 [v4.2~v4.4]

§7.1의 `[affinity: N]` 태그 방식은 레거시이며, 현재는 2단계 하이브리드 감정 판정으로 대체되었다.
(`PromptContext.skip_affinity_directive = true` 시 태그 지시 억제)

```
  NPC 대사 생성
       │
       ├── Stage 1: 극단 앵커 임베딩 (~7ms/턴, 매 턴)
       │   ├── NPC 대사 → BGE-M3 임베딩
       │   ├── 극단 warmth/coldness 앵커 각 20개와 cosine similarity
       │   ├── ≥ 0.60 → 트리거! → 즉시 Stage 2 LLM 판정
       │   └── < 0.60 → 통과 (다음 턴 대기)
       │
       └── Stage 2: LLM 정기 감정 판정 (~300ms, 12턴마다 또는 트리거 시)
           ├── 누적 대화 컨텍스트 → gemma3-12b
           ├── JSON 출력: {"sentiment", "score", "reason"}
           ├── score → judgment_to_delta() → affinity 변화량
           └── Relationship 업데이트

  코드 위치:
    wuxia-core  — ExtremeAnchorSet, SentimentJudgment, TurnCounter, DeltaSource
    wuxia-llm   — SentimentJudge trait, SentimentPipeline, LlmSentimentJudge
    wuxia-data  — extreme-anchors.toml, sentiment-judge.toml 로딩
```

상세 벤치마크: [step4.2-sentiment-benchmark-report.md](../benchmarks/step4.2-sentiment-benchmark-report.md)

---

## 8. 벡터 검색 상세 — 2-Stage Search

### 8.1 임베딩 모델 (언어별 분리)

```
  ┌──────────────────────┬────────────────────┬────────────────────┐
  │                      │ KO/ZH (한국어/중국어)│ EN (영어)          │
  ├──────────────────────┼────────────────────┼────────────────────┤
  │ 모델                 │ Bge-M3-567M-Q8_0   │ embeddinggemma     │
  │                      │                    │ -300m-qat-Q8_0     │
  │ 차원                 │ 1024               │ 768                │
  │ 유형                 │ symmetric          │ asymmetric         │
  │ task_prompt          │ (없음)             │ "task: search      │
  │                      │                    │  result | query: " │
  │ 극단 앵커 threshold  │ 0.60               │ 0.70               │
  │ 선정 근거            │ 안전 마진 넓음      │ 추후 벤치마크 확정  │
  └──────────────────────┴────────────────────┴────────────────────┘

  설정: assets/ai/embedding.toml (profile-based, active_profile 전환)

  VRAM 예산 (RTX 2070S 8GB):
  ┌──────────────────────┬────────┐
  │ gemma3:4b (메인 LLM) │ ~3.9GB │
  │ KV Cache (8192 ctx)  │ ~0.6GB │
  │ 임베딩 모델          │ ~0.3GB │
  │ CUDA 오버헤드        │ ~0.3GB │
  │ 남은 여유            │ ~2.9GB │
  └──────────────────────┴────────┘
```

### 8.2 2-Stage 필터링

```
  Stage 1: Cosine Similarity Threshold
  ─────────────────────────────────────
  cosine_sim >= threshold → 통과
  cosine_sim <  threshold → 탈락

  언어별 threshold (L2_min 기준, 관련 기억 0% 손실 보장):
  ┌──────┬───────────┬────────┬─────────┐
  │ 언어 │ threshold │ GAP    │ L4 역전 │
  ├──────┼───────────┼────────┼─────────┤
  │ KO   │ 0.4656    │ +0.066 │ 0/8     │
  │ EN   │ 0.4580    │ +0.065 │ 0/8     │
  │ ZH   │ 0.4668    │ +0.063 │ 0/8     │
  └──────┴───────────┴────────┴─────────┘

  Stage 2: Keyword Overlap Boost
  ──────────────────────────────
  boost_threshold = threshold × boost_ratio(1.1)
  
  sim >= boost_threshold → keyword 없어도 통과
  sim <  boost_threshold → keyword overlap > 0 필요 (아니면 탈락)
  
  예시 (KO, threshold=0.4656, boost=0.5122):
    sim=0.55 + keyword=0 → 통과  (0.55 >= 0.5122)
    sim=0.48 + keyword=0 → 탈락  (0.48 < 0.5122, keyword=0)
    sim=0.48 + keyword=1 → 통과  (keyword > 0)
```

### 8.3 4축 랭킹 공식

```
  score = α₁ × recency
        + α₂ × importance
        + α₃ × relevance
        + α₄ × emotional_match

  recency       = decay_factor ^ (경과 일수)
                  0.995^3 ≈ 0.985,  0.995^30 ≈ 0.860,  0.995^360 ≈ 0.164

  importance    = entry.importance / 10.0
                  "만두를 먹었다"(2) → 0.2,  "사부 배신"(9) → 0.9

  relevance     = 벡터 유사도 (LanceDB search에서 이미 계산됨)

  emotional_match = PAD P축 × 기억 감정가 (OCC_TODO②: 향후 활성화)
                    현재: 항상 0.0 (emotional_bias_weight = 0.0)
```

---

## 9. 컨텍스트 압축 — ConversationManager

대화가 길어지면 ctx를 넘길 수 있다. ConversationManager가 자동으로 오래된 대화를 요약한다.

```
  대화 이력 축적:
  ┌──────────────────────────────────────┐
  │ Turn 1: user/assistant               │
  │ Turn 2: user/assistant               │
  │ Turn 3: user/assistant               │
  │ ... (토큰 누적 → 임계치 도달)        │
  └──────────────────────────────────────┘
       │
       ▼
  ConversationManager 판정: "압축 필요"
       │
       ▼
  LLM 1회 호출: 오래된 턴들을 1~2문장으로 요약
       │
       ▼
  미시요약을 PromptContext.conversation_summary에 누적
  ┌──────────────────────────────────────┐
  │ [이번 대화 요약]                      │
  │ "플레이어가 혈교에 대해 물었고..."    │
  │                                       │
  │ Turn 5: user/assistant  ← 최근만 유지 │
  │ Turn 6: user/assistant               │
  └──────────────────────────────────────┘

  주의: 미시요약은 기억 저장 대상이 아니다.
        영구 기억은 session.end()의 최종 요약에서만 생성된다.
```

---

## 10. 설정 파일 구조

### 10.1 prompt_config.toml (프롬프트 전체)

```
  assets/data/prompt/prompt_config.toml
  │
  ├── [language_directive]     — 언어 지시문 (ko/en)
  ├── [headers.ko/en]          — 10개 섹션 헤더
  ├── [templates.ko/en]        — 문장 템플릿 (플레이스홀더)
  ├── [memory_format.ko/en]    — 기억 포맷 (시간+내용+라벨)
  └── [memory_labels]          — 중요도 라벨
       ├── [thresholds]        — 5단계 경계값
       ├── [ko]                — 행동 지시 라벨 (한국어)
       └── [en]                — 행동 지시 라벨 (영어)
```

### 10.2 embedding.toml (임베딩 설정)

```
  assets/ai/embedding.toml
  │
  ├── [model]                  — 모델 정보 (이름, 파일, 차원, task_prompt)
  ├── [threshold]              — 언어별 cosine similarity 경계값
  └── [search]                 — boost_ratio, candidate_multiplier
```

### 10.3 descriptions.toml (관계 설명)

```
  assets/data/relationship/descriptions.toml
  │
  ├── [level.Stranger/Acquaintance/.../Enemy]  — 8단계 관계 설명
  └── [trust.none/wary/cautious/considerable/deep]  — 5단계 신뢰 설명
  ※ 적대 상태는 별도 축 없이, affinity 음수값에 따른
     RelationshipLevel (Wary/Hostile/Enemy)로 표현
```

---

## 11. 코드 호출 흐름 — send() 한 턴 상세

```rust
// ChatSession::send() 의사코드 (실제 코드 기반 단순화)

pub fn send(&mut self, user_input: &str) -> Result<ChatReply, LlmError> {
    // 1. 기억 검색 + 관계 요약 (ContextProvider 경유)
    let memories = self.context_provider.search_memories(user_input);
    let rel_summary = self.context_provider.relationship_summary();

    // 2. 동적 컨텍스트 구성
    let ctx = PromptContext {
        memories,
        conversation_summary: self.manager.current_summary(),
        relationship_summary: rel_summary,
        system_reminder: self.character.system_reminder.clone(),
    };

    // 3. 시스템 프롬프트 빌드 (순수 함수)
    let system_prompt = build_system_prompt(
        &self.character, &self.speech, &ctx, self.locale, &self.prompt_config
    );

    // 4. 대화 이력에 사용자 메시지 추가
    self.manager.push_user(user_input);

    // 5. 압축 판정 + 실행
    if self.manager.should_compress() {
        let summary = self.llm.generate(compress_request)?;
        self.manager.apply_compression(summary);
    }

    // 6. LLM 호출
    let messages = self.manager.build_messages(&system_prompt);
    let raw = self.llm.generate(LlmRequest { messages, .. })?;

    // 7. 응답 파싱 ([affinity: N] 태그 추출 + 제거)
    let parsed = parse_response_with_tags(&raw);

    // 8. 대화 이력에 어시스턴트 메시지 추가
    self.manager.push_assistant(&parsed.text);

    Ok(ChatReply {
        text: parsed.text,
        affinity_delta: parsed.affinity_delta,
        turn_index: self.turn_count,
        compressed: had_compression,
    })
}
```

---

## 12. 향후 확장 (OCC 감정 연동)

현재 시스템에 4개의 OCC 연동 포인트가 예약되어 있다:

```
  OCC_TODO①: importance 자동 산정
  ────────────────────────────────
  현재: LLM이 평가 (end() 시)
  향후: OCC 감정 강도 → importance 자동 계산
    공식: importance = base(3.0) + Σ(emotion.intensity × relevant_value)
    예: 분노(0.55) × 의(0.8) = 0.44 → importance += 4.4 → 7.4

  OCC_TODO②: PAD 기분 일치 기억 편향
  ─────────────────────────────────
  현재: emotional_bias_weight = 0.0 (무효)
  향후: 분노 상태 → 부정적 기억이 더 잘 떠오름 (mood-congruent memory)
    코드 위치: LiveContextProvider에서 EmotionalBias를 주입

  OCC_TODO③: 5가치 관련도 증폭
  ─────────────────────────────
  현재: relevance = 순수 벡터 유사도
  향후: NPC의 5가치(충/의/효/복수/야망)와 기억 키워드 일치 시 relevance 증폭
    예: 소연(의=0.8) + "배신" 키워드 → relevance × 1.8

  OCC_TODO④: MemoryEntry 감정가(valence) 필드
  ─────────────────────────────────────────────
  현재: 없음
  향후: 기억 저장 시 OCC 감정 평가 결과를 기록
    긍정 감정 → valence > 0, 부정 감정 → valence < 0
    PAD 기분 일치 편향(②)에서 사용
```

---

## 13. 테스트 전략

| 계층 | 테스트 방식 | 구현체 |
|------|------------|--------|
| 4축 랭킹 (retrieval.rs) | 단위 테스트 (순수 함수) | — |
| InMemoryRepository | 단위 테스트 | MockEmbedding |
| LanceDbRepository | 통합 테스트 (tmpdir) | MockEmbedding |
| ContextProvider | 단위 테스트 | InMemoryRepository |
| ChatSession | 단위 테스트 | MockLlm + NullContextProvider |
| 프롬프트 빌더 | 단위 테스트 (순수 함수) | — |
| 전체 통합 | 수동 시나리오 (soyeon_chat) | LanceDb + LlamaCpp |

---

## 14. 변경 이력

| 버전 | 변경일시 | 변경 내역 |
|:---:|----------|-----------|
| v1.0 | 2026-02-21 07:30 | 초기 작성. Sprint 3 Iter3 작업③ 완료 시점 기준. 전체 아키텍처(4 crate 역할), 데이터 모델(MemoryEntry+LanceDB 스키마), 대화 중 6단계 검색 파이프라인, 프롬프트 11섹션 구조와 4계층 데이터 소스, 행동 지시형 라벨 설계 의도, 관계 반영(affinity_delta), 2-Stage 벡터 검색, 컨텍스트 압축, 설정 파일 구조, send() 호출 흐름, OCC 확장 포인트 4개, 테스트 전략 정리. |
| v1.1 | 2026-02-21 08:34 | §1.3 "대화 한 턴의 시간 흐름" 시퀀스 다이어그램 추가 (Player→ChatSession→ContextProvider→Embedding→LanceDB→Ranker→LLM→Parser 8개 참여자). §1.4 "대화 종료 시 기억 저장 흐름" 시퀀스 다이어그램 추가 (요약+중요도→벡터→영속→재실행 시 검색). 아키텍처 레벨 동적 흐름 가시화. |
| v1.2 | 2026-03-01 | 코드베이스 정합성 갱신: 관계 모델 3축→2축(호감/신뢰), 임베딩 차원 언어별 분리(KO:BGE-M3 1024d, EN:EmbeddingGemma 768d), crate 의존성 다이어그램 갱신(wuxia-app 조립 계층 추가), 깨진 크로스 레퍼런스 5개 수정, descriptions.toml 구조 갱신(hostility 축 제거), LiveContextProvider 위치 명시(wuxia-app), 2단계 하이브리드 감정 판정 파이프라인(§7.4) 추가, 버전 태깅 통일. |
