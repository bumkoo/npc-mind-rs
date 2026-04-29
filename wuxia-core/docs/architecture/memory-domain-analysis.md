# 기억 도메인 상세 분석 (Memory Domain Analysis)

> **버전**: v1.0 | **최종 수정**: 2026-02-28T21:35:00Z  
> **역할**: 기억 도메인의 경계, 소유 데이터, 업무 규칙, 이벤트, 포트, 검토 사항 정의  
> 📎 도메인 전체 구조: [domain-analysis.md](domain-analysis.md) §1.1 (12번 기억)  
> 📎 구현 아키텍처: [npc-conversation-memory-architecture.md](../ai/npc-conversation-memory-architecture.md)  
> 📎 NPC 심리 아키텍처: [npc-psychology-architecture.md](npc-psychology-architecture.md) §7 성찰  
> 📎 임베딩 벤치마크: [step3.1-embedding-benchmark-report.md](../step3.1-embedding-benchmark-report.md)  
> 📎 임베딩 threshold: [step3.3-threshold-analyzer-report.md](../step3.3-threshold-analyzer-report.md)  
> 📎 Generative Agents 참조: [generative-agents.md](../reference/generative-agents.md)

---

## 1. 개요

### 1.1 도메인 목적

> "이 NPC는 무엇을 기억하나?"  
> 비유: 기억 서고(記憶書庫) — NPC가 겪고, 생각하고, 계획한 것의 저장소

기억 도메인은 NPC의 경험을 구조화하여 저장하고, 상황에 맞는 기억을 검색·순위화하여
다른 도메인(심리, 서사, 관계)에 제공하는 역할을 한다.

Stanford Generative Agents 논문의 세 가지 기억 유형(관찰·성찰·계획)과
4축 검색 알고리즘(최신성·중요도·관련도·감정일치)을 핵심 업무 규칙으로 채택한다.

### 1.2 독립 배경

기존에는 심리 도메인(§2.2)의 하위 항목("기억 스트림 — 향후 구현")이었다.
그러나 구현 결과 다음과 같은 독립 도메인의 특징을 모두 갖추고 있어 12번째 도메인으로 승격하였다:

| 독립 도메인 기준 | 기억 도메인 충족 여부 |
|-----------------|---------------------|
| 독자적인 소유 데이터 | ✅ MemoryEntry, ScoredMemory, RankedMemory |
| 독자적인 이벤트 | ✅ MemoryEvent 3종 (MemoryStored, MemoryRecalled, ImportanceUpdated) |
| 독자적인 포트(Output Port) | ✅ MemoryRepository, EmbeddingPort |
| 독자적인 업무 규칙 | ✅ 4축 검색 점수 계산, 감쇠 함수, 정규화 |
| 독립된 모듈 구조 | ✅ wuxia-core/src/memory/ (7개 파일) |
| 다른 도메인과 ID 참조 관계 | ✅ CharacterId로 심리/관계 도메인 연결 |

### 1.3 핵심 질문과 비유

```
  기억 도메인이 답하는 질문:
  ─────────────────────────────────────
  "소연은 플레이어와의 첫 만남을 기억하는가?"     → 저장 여부
  "지금 상황에서 소연이 떠올리는 기억은 무엇인가?" → 검색·순위화
  "이 기억이 소연에게 얼마나 중요한가?"           → 중요도 판정
  "소연이 과거 경험을 되돌아보며 새 깨달음을 얻었나?" → 성찰 기억 생성
```

---

## 2. 소유 데이터

### 2.1 기억 유형 (MemoryType) — 3종

Stanford Generative Agents 논문에 근거한 세 가지 기억 유형:

| 유형 | 코드명 | 업무 규칙 | 무협 예시 |
|------|--------|----------|----------|
| 관찰 (Observation) | Observation | NPC가 직접 겪거나 목격한 사건을 기록한다 | "화산파 장로가 나를 꾸짖었다" |
| 성찰 (Reflection) | Reflection | 여러 관찰을 종합하여 의미를 부여한 해석이다. 반드시 원본 기억(source_ids)을 참조한다 | "나는 화산파에서 환멸을 느끼고 있다" |
| 계획 (Plan) | Plan | 성찰에 기반하여 NPC가 스스로 세운 행동 의도이다 | "내일 밤 몰래 떠나겠다" |

**기억 유형 간 관계 (성찰 트리)**:

```
  관찰①: "장로가 나를 꾸짖었다"  ──┐
  관찰②: "사형이 내 편을 안 든다" ──┼──► 성찰A: "환멸을 느낀다" (source_ids: [①, ②])
  관찰③: "강호에 다른 길이 있다"  ──┘         │
                                             └──► 계획X: "내일 밤 떠나겠다" (source_ids: [A])
```

### 2.2 기억 항목 (MemoryEntry) — 필드 정의

기억 도메인이 소유하는 핵심 데이터 단위이다.

| 필드 | 설명 | 업무 규칙 |
|------|------|----------|
| id | 기억 고유 식별자 | 생성 시 자동 부여, 불변 |
| character_id | 이 기억의 주인 NPC | CharacterId 참조 (캐릭터 도메인) |
| memory_type | 관찰 / 성찰 / 계획 | §2.1 세 가지 유형 중 하나 |
| content | 기억 내용 (자연어 텍스트) | 한국어/중국어 등 다국어 지원 |
| importance | 중요도 (1~10) | 1=일상 잡담, 5=의미 있는 대화, 10=인생을 바꿀 사건 |
| keywords | 핵심 키워드 목록 | 검색 시 키워드 매칭에 활용 |
| source_ids | 원본 기억 ID 목록 | 성찰/계획 유형만 사용. 관찰은 빈 목록 |
| created_at | 생성 시점 (게임 시간) | 최신성(recency) 점수 계산에 사용 |
| lang | 언어 코드 | 다국어(i18n) 지원을 위한 언어 식별 |

### 2.3 검색 결과 데이터

| 데이터 | 설명 |
|--------|------|
| ScoredMemory | MemoryEntry + 벡터 유사도 점수(score). 벡터DB 검색 직후의 원시 결과 |
| RankedMemory | MemoryEntry + 4축 종합 점수(final_score) + 개별 축 점수. 최종 순위화된 결과 |

---

## 3. 포트 (Output Port) — Hexagonal Architecture

### 3.1 설계 원칙

기억 도메인은 Hexagonal Architecture의 원칙에 따라, 도메인 로직(wuxia-core)이
외부 기술(벡터DB, 임베딩 모델)에 직접 의존하지 않는다.
도메인은 Port(trait)만 정의하고, 구체적인 기술 구현은 Adapter(wuxia-memory)가 담당한다.

```
  ┌─────────────────────────────────────────────────────────┐
  │              wuxia-core (도메인 순수 로직)                │
  │                                                          │
  │   MemoryEntry, MemoryType, ScoredMemory, RankedMemory   │
  │   retrieval_score(), rank_memories(), recall_memories()  │
  │                                                          │
  │   ┌──────────────────┐    ┌────────────────┐            │
  │   │ MemoryRepository │    │ EmbeddingPort  │  ← Port    │
  │   │    (trait)       │    │   (trait)      │            │
  │   └────────┬─────────┘    └───────┬────────┘            │
  └────────────┼──────────────────────┼─────────────────────┘
               │                      │
  ─ ─ ─ ─ ─ ─ ┼ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┼ ─ ─ ─ ─ (경계)
               │                      │
  ┌────────────┼──────────────────────┼─────────────────────┐
  │            ▼                      ▼                      │
  │   ┌──────────────────┐    ┌────────────────┐            │
  │   │ InMemoryRepo     │    │ MockEmbedding  │  ← 테스트  │
  │   │ LanceDbRepo      │    │ LlamaCppEmbed  │  ← 프로덕션│
  │   └──────────────────┘    └────────────────┘            │
  │              wuxia-memory (어댑터 계층)                   │
  └─────────────────────────────────────────────────────────┘
```

### 3.2 MemoryRepository — 기억 저장소 포트

NPC의 기억을 영속 저장하고 검색하기 위한 추상 인터페이스이다.

| 메서드 | 업무 규칙 |
|--------|----------|
| save | 기억 항목 하나를 저장한다. 동일 ID가 존재하면 오류를 반환한다 |
| find_recent | 특정 NPC의 최근 기억을 N개 반환한다 (시간 역순). 관찰/성찰/계획 필터 가능 |
| search | 쿼리 벡터와 유사한 기억을 벡터 유사도 순으로 반환한다. threshold 이상만 포함 |
| count | 특정 NPC의 총 기억 수를 반환한다 |
| update_importance | 기존 기억의 중요도를 변경한다. 성찰 결과로 중요도가 재평가될 때 사용 |

**어댑터 구현 현황**:

| 어댑터 | 용도 | 검색 방식 |
|--------|------|----------|
| InMemoryRepository | 단위 테스트, 개발 | 키워드 매칭 (HashMap 기반) |
| LanceDbRepository | 프로덕션 | 벡터 유사도 검색 (코사인 유사도) |

### 3.3 EmbeddingPort — 임베딩 변환 포트

자연어 텍스트를 벡터(부동소수점 배열)로 변환하기 위한 추상 인터페이스이다.

| 메서드 | 업무 규칙 |
|--------|----------|
| embed | 쿼리 텍스트를 벡터로 변환한다 (검색용) |
| embed_document | 문서 텍스트를 벡터로 변환한다 (저장용). 비대칭 모델에서 쿼리와 문서 임베딩을 구분 |
| dimensions | 벡터 차원 수를 반환한다 (예: 768) |
| model_name | 사용 중인 모델명을 반환한다 |

**비대칭 모델 업무 규칙**: 검색 시에는 embed()로 변환한 쿼리 벡터를, 저장 시에는 embed_document()로 변환한 문서 벡터를 사용한다. 모델에 따라 두 메서드가 동일할 수 있다.

**어댑터 구현 현황**:

| 어댑터 | 용도 |
|--------|------|
| MockEmbedding | 단위 테스트 (고정 벡터 반환) |
| LlamaCppEmbedding | 프로덕션 (로컬 LLM 임베딩 모델 사용) |

**보조 유틸리티 (순수 함수)**:

| 함수명 | 업무 규칙 |
|--------|----------|
| cosine_similarity | 두 벡터 간 코사인 유사도를 계산한다 (-1.0 ~ 1.0) |
| l2_normalize | 벡터를 L2 정규화한다 (단위 벡터로 변환) |

---

## 4. 이벤트 (MemoryEvent)

### 4.1 이벤트 정의

도메인 이벤트는 "이미 일어난 사실"을 나타낸다.
기억 도메인은 세 가지 이벤트를 정의하며, DomainEvent 래퍼를 통해 다른 도메인에 전파된다.

| 이벤트 | 의미 | 포함 데이터 | 발생 시점 |
|--------|------|-----------|----------|
| MemoryStored | 새 기억이 저장되었다 | memory_id, character_id, memory_type, importance | 기억 저장 직후 |
| MemoryRecalled | 기억이 회상되었다 | character_id, 회상된 기억 수, 최고 점수 기억 ID | 기억 검색·순위화 완료 후 |
| ImportanceUpdated | 기억의 중요도가 변경되었다 | memory_id, character_id, old_importance, new_importance | 중요도 갱신 직후 |

### 4.2 DomainEvent 통합 현황

기억 이벤트는 프로젝트의 공유 이벤트 래퍼(DomainEvent)에 Memory 변형(variant)으로 통합되어 있다. From 트레이트를 통해 MemoryEvent → DomainEvent 자동 변환이 가능하다.

### 4.3 이벤트 구독자 설계 (향후)

| 이벤트 | 예상 구독자 | 구독 이유 |
|--------|-----------|----------|
| MemoryStored | 심리 도메인 | 중요도 높은 기억 축적 시 성찰 트리거 판단 |
| MemoryStored | 서사 도메인 | 스토리 분기 조건 확인 (예: "배반 목격 기억 3개 이상") |
| MemoryRecalled | 심리 도메인 | 자주 회상되는 기억 → 중요도 상향 조정 후보 |
| ImportanceUpdated | 관계 도메인 | 상대방 관련 기억 중요도 변화 → 관계 재평가 |

---

## 5. 도메인 서비스와 업무 규칙

### 5.1 구현 완료 — 검색·순위화 로직

기억 도메인의 핵심 업무 규칙은 "어떤 기억을 떠올릴 것인가"를 결정하는 검색·순위화이다.

#### 4축 검색 점수 공식

```
  종합 점수 = w₁ × 최신성 + w₂ × 중요도 + w₃ × 관련도 + w₄ × 감정일치도
```

| 축 | 계산 방법 | 가중치 객체 |
|----|----------|-----------|
| 최신성 (recency) | decay_factor ^ (경과 일수). 최근 기억일수록 높음 | RetrievalWeights |
| 중요도 (importance) | importance / 10. 중요한 기억일수록 높음 | RetrievalWeights |
| 관련도 (relevance) | 벡터 유사도 점수 (ScoredMemory.score). 쿼리와 의미적으로 가까울수록 높음 | RetrievalWeights |
| 감정일치도 (emotional_match) | 현재 PAD 기분 상태와 기억 당시 감정의 일치도. 기분 일치 편향(mood congruence) 반영 | EmotionalBias |

#### 구현된 함수 목록

| 함수명 | 역할 | 비고 |
|--------|------|------|
| retrieval_score() | 단일 기억에 대한 4축 종합 점수를 계산한다 | 순수 함수, I/O 없음 |
| rank_memories() | 여러 기억을 종합 점수 내림차순으로 정렬한다 | 순수 함수, I/O 없음 |
| recall_memories() | Repository에서 검색 → rank_memories()로 순위화 → 결과 반환 | 도메인 서비스, Repository 의존 |

#### 감정일치도 (EmotionalBias) 업무 규칙

NPC의 현재 기분(PAD 3차원)이 기억 검색에 영향을 미친다.
기분이 좋을 때는 긍정적 기억을, 나쁠 때는 부정적 기억을 더 잘 떠올린다 (기분 일치 편향).

```
  기분이 좋은 소연 → "플레이어가 도와준 기억" 떠올림 확률↑
  기분이 나쁜 소연 → "배신당한 기억" 떠올림 확률↑
```

> **OCC 통합 마커**: 현재 코드에 OCC_TODO 마커 4개가 존재하며, 심리 도메인의 OCC 감정 시스템 구현 후 연결 예정.

### 5.2 검토 사항 — store_memory 서비스 도입

**현재 상태**: 기억 저장은 호출자(wuxia-llm 등)가 직접 Repository.save()를 호출한다.
저장 후 MemoryStored 이벤트가 생성되지 않는다.

**문제**: DDD 관점에서, 도메인 상태 변경(기억 저장)은 반드시 도메인 이벤트를 수반해야 한다.
이벤트 없이는 다른 도메인(심리, 서사)이 새 기억 발생을 알 수 없다.

**필요한 업무 규칙**:

```
  store_memory 서비스의 업무 흐름:
  ──────────────────────────────────────────────────
  ① 입력 검증
     - 기억 내용이 비어있지 않은가?
     - 중요도가 1~10 범위인가?
     - 성찰/계획 유형이면 source_ids가 비어있지 않은가?
  
  ② 저장 실행
     - MemoryRepository.save()를 통해 영속화

  ③ 이벤트 생성
     - MemoryStored 이벤트를 생성하여 반환
     - 반환값: (저장 결과, Vec<MemoryEvent>)

  ④ 이벤트 전파 (Application Layer 책임)
     - 심리 도메인: 중요도 높은 기억 축적 감지 → 성찰 트리거
     - 서사 도메인: 스토리 조건 확인
```

**설계 원칙 — Hexagonal Architecture 관점**:
- store_memory는 도메인 서비스이다 (wuxia-core에 위치)
- Repository trait만 의존하며, 구체적 DB 기술을 모른다
- 이벤트 생성까지가 도메인 서비스의 책임이고, 이벤트 전파는 Application Layer(wuxia-game)의 책임이다

### 5.3 검토 사항 — recall 이벤트 통합

**현재 상태**: recall_memories()는 순위화된 기억 목록(Vec\<RankedMemory\>)만 반환한다.
회상 행위 자체가 이벤트로 기록되지 않는다.

**문제**: 어떤 기억이 자주 회상되는지 추적할 수 없다. 이는 다음 업무 규칙과 연결된다:
- "자주 떠올리는 기억은 시간이 지나도 잊히지 않는다" (중요도 자동 상향)
- "반복 회상은 감정 강화를 유발한다" (심리 도메인 연동)

**필요한 업무 규칙**:

```
  recall 이벤트 통합의 업무 흐름:
  ──────────────────────────────────────────────────
  ① 기존 recall_memories() 실행
     - Repository.search() → rank_memories() → 결과 반환
  
  ② MemoryRecalled 이벤트 생성 (추가)
     - 회상을 요청한 NPC의 character_id
     - 회상된 기억의 수
     - 최고 점수 기억의 ID (가장 강하게 떠오른 기억)
  
  ③ 반환값 변경
     - 현재: Vec<RankedMemory>
     - 변경: (Vec<RankedMemory>, Vec<MemoryEvent>)
```

**설계 선택지**:

| 방안 | 설명 | 장단점 |
|------|------|--------|
| A. 기존 함수 시그니처 변경 | recall_memories()의 반환값에 이벤트 포함 | 단순하지만 기존 호출자 수정 필요 |
| B. 래퍼 함수 추가 | recall_and_emit()를 별도로 만들고 기존 함수 유지 | 하위 호환성 유지, 점진적 전환 |

> **권장**: 방안 B. 기존 recall_memories()를 순수 함수로 유지하면서, 이벤트가 필요한 곳에서만 recall_and_emit()를 사용한다. 다른 도메인(성장, 전투 등)도 동일한 패턴(서비스_and_emit)을 따르므로 일관성 있다.

### 5.4 검토 사항 — update_importance 서비스 도입

**현재 상태**: Repository.update_importance()는 포트에 정의되어 있지만, 이를 호출하면서 ImportanceUpdated 이벤트를 생성하는 도메인 서비스가 없다.

**필요한 업무 규칙**:

```
  update_importance 서비스의 업무 흐름:
  ──────────────────────────────────────────────────
  ① 변경 타당성 검증
     - 새 중요도가 1~10 범위인가?
     - 기존 중요도와 동일하면 변경하지 않는다 (불필요한 이벤트 방지)

  ② 중요도 갱신 실행
     - MemoryRepository.update_importance() 호출

  ③ ImportanceUpdated 이벤트 생성
     - old_importance와 new_importance를 모두 포함하여
       구독자가 변화 방향(상향/하향)을 판단할 수 있게 한다
```

**중요도 변경이 발생하는 업무 상황**:

| 상황 | 변경 방향 | 트리거 |
|------|----------|--------|
| 성찰을 통해 과거 기억의 의미를 재해석 | 상향 | 심리 도메인의 성찰(⑦층) 결과 |
| 동일 기억이 반복 회상됨 | 상향 | MemoryRecalled 이벤트 누적 |
| 시간 경과로 일상적 기억이 희미해짐 | 하향 | 시간 도메인 YearPassed 이벤트 |
| 관계 단절로 상대방 관련 기억이 덜 중요해짐 | 하향 | 관계 도메인 이벤트 |

---

## 6. 다른 도메인과의 관계

### 6.1 관계 다이어그램

```
              ┌──────────┐
              │ 시간 도메인│
              │ (Time)   │
              └────┬─────┘
                   │ YearPassed → 장기 미회상 기억 중요도 하향 검토
                   ▼
  ┌──────────┐   ┌────────────┐   ┌──────────┐
  │ 심리 도메인│◄─►│ 기억 도메인 │◄─►│ 서사 도메인│
  │(Psychology)│   │ (Memory)   │   │(Narrative)│
  └──────────┘   └─────┬──────┘   └──────────┘
                       │
                       ▼
              ┌──────────────┐
              │ 관계 도메인   │
              │(Relationship)│
              └──────────────┘
```

### 6.2 심리 ↔ 기억

| 방향 | 흐름 | 설명 |
|------|------|------|
| 심리 → 기억 | 성찰 트리거 → 관련 기억 검색 요청 | 심리 도메인이 "성찰이 필요하다"고 판단하면 기억 도메인에서 재료가 되는 기억을 검색 |
| 심리 → 기억 | 성찰 결과 → 새 기억 저장 | LLM이 생성한 성찰 텍스트를 성찰(Reflection) 유형 기억으로 저장 |
| 심리 → 기억 | OCC 감정 상태 → 감정일치도 가중치 | 현재 PAD 기분이 기억 검색의 EmotionalBias로 작용 |
| 기억 → 심리 | MemoryStored 이벤트 → 성찰 조건 누적 | 중요도 높은 기억이 일정 수 쌓이면 성찰 트리거 |

**경계 규칙**: 심리 도메인은 기억의 "내용"을 직접 수정하지 않는다. 항상 기억 도메인의 Port를 통해 저장/검색한다.

### 6.3 서사 ↔ 기억

| 방향 | 흐름 | 설명 |
|------|------|------|
| 서사 → 기억 | 스토리 이벤트 → 관찰 기억 생성 | "정사대전이 발발했다" 같은 세계 사건을 NPC별 관찰 기억으로 저장 |
| 기억 → 서사 | MemoryStored 이벤트 → 분기 조건 확인 | "배반 목격 기억 3개 이상이면 퀘스트 활성화" 같은 스토리 트리거 |

### 6.4 관계 ↔ 기억

| 방향 | 흐름 | 설명 |
|------|------|------|
| 관계 → 기억 | 관계 변화 → 관련 기억 중요도 재평가 | 사제 관계 파기 시 스승 관련 기억의 중요도 재산정 |
| 기억 → 관계 | 기억 검색 결과 → 관계 맥락 제공 | 대화 시 과거 상호작용 기억을 프롬프트에 제공하여 관계 반영 응답 생성 |

### 6.5 시간 ↔ 기억

| 방향 | 흐름 | 설명 |
|------|------|------|
| 시간 → 기억 | YearPassed/DayPassed → 최신성 점수 자동 감쇠 | 시간이 흐르면 기억의 recency 점수가 자연 하락 (decay_factor 적용) |
| 시간 → 기억 | 장기 미회상 기억 → 중요도 하향 후보 | 일정 기간 회상되지 않은 일상(importance ≤ 3) 기억 정리 검토 |

---

## 7. 어댑터 구현 현황 (wuxia-memory crate)

### 7.1 모듈 구조

```
  wuxia-memory/
  ├── in_memory.rs       ← InMemoryRepository (테스트용)
  ├── lancedb/           ← LanceDbRepository (프로덕션용)
  ├── embedding/
  │   ├── mock.rs        ← MockEmbedding (테스트용)
  │   └── llama_cpp.rs   ← LlamaCppEmbedding (프로덕션용)
  ├── config.rs          ← EmbeddingConfig 로더
  └── examples/
      ├── embedding_benchmark.rs   ← 임베딩 성능 벤치마크
      └── threshold_analyzer.rs    ← 유사도 임계값 분석
```

### 7.2 어댑터별 역할

| 어댑터 | 구현하는 포트 | 기술 의존성 | 용도 |
|--------|-------------|-----------|------|
| InMemoryRepository | MemoryRepository | 없음 (HashMap) | 단위 테스트, 개발 |
| LanceDbRepository | MemoryRepository | LanceDB | 프로덕션 벡터 검색 |
| MockEmbedding | EmbeddingPort | 없음 (고정값) | 단위 테스트 |
| LlamaCppEmbedding | EmbeddingPort | llama-cpp-2 | 프로덕션 임베딩 |

### 7.3 wuxia-core 도메인 로직 모듈 구조

```
  wuxia-core/src/memory/
  ├── types.rs       ← MemoryEntry, MemoryType, ScoredMemory (소유 데이터)
  ├── port.rs        ← MemoryRepository trait (Output Port)
  ├── embedding.rs   ← EmbeddingPort trait, cosine_similarity, l2_normalize
  ├── retrieval.rs   ← retrieval_score(), rank_memories() (순수 함수)
  ├── recall.rs      ← recall_memories() (도메인 서비스)
  ├── event.rs       ← MemoryEvent enum (3종)
  └── mod.rs         ← 공개 re-export
```

---

## 8. 변경 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| **v1.0** | **2026-02-28** | **최초 작성. 심리 도메인 하위 항목에서 12번째 독립 도메인으로 승격. §1 개요 (독립 배경, 핵심 질문). §2 소유 데이터 (MemoryType 3종, MemoryEntry 필드, ScoredMemory/RankedMemory). §3 포트 (MemoryRepository 5메서드, EmbeddingPort 4메서드, 어댑터 현황). §4 이벤트 (MemoryEvent 3종, DomainEvent 통합, 구독자 설계). §5 도메인 서비스 (4축 검색 점수, 구현 완료 함수 3개, 검토 사항: store_memory/recall 이벤트 통합/update_importance 서비스). §6 도메인 간 관계 (심리↔기억, 서사↔기억, 관계↔기억, 시간↔기억). §7 어댑터 구현 현황.** |

---
