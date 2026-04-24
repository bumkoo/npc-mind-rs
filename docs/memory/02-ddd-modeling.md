# NPC 기억 시스템 — 2차: DDD 모델링

> 본 문서는 기억 시스템 설계 3단계 중 **2차 산출물**이다.
> [1차 용어 정의 및 업무 규칙](01-terms-and-rules.md)에서 정리한 도메인 언어를 DDD 모델 요소(Bounded Context · Aggregate · Entity · Value Object · Domain Event · Domain Service · Policy)로 매핑하고, 기존 NPC Mind 컨텍스트와의 통합 지점을 규정한다.
>
> 코드는 타입 스케치 수준에서만 사용한다 (언어 중립 의사 표기). 구현 세부는 3차에서 다룬다.

## 1. 모델링 목표

- **1차 용어·규칙을 모델 요소로 투영**한다. 업무 규칙은 불변식·정책으로 전환된다.
- **Event Sourcing / CQRS 아키텍처와 정합**을 유지한다. 기억 시스템은 기존 `EventBus` + `EventStore` + `CommandDispatcher` 파이프라인 위에 얹힌다.
- **트랜잭션 경계**와 **컨텍스트 경계**를 명시한다.
- **기존 `MemoryProjector` / `SqliteMemoryStore`의 확장 지점**을 도출한다 (구현은 3차).

## 2. Bounded Context 구조

### 2.1 컨텍스트 분할

```
┌────────────────────────────────────────────────────────┐
│  NPC Mind Context  (기존)                              │
│  - Personality, Emotion, Relationship, Scene, Guide    │
│  - Director, DialogueOrchestrator                             │
└──────────┬────────────────────┬────────────────────────┘
           │  events             │  queries
           ▼                     ▲
┌────────────────────────────────────────────────────────┐
│  Memory Context  (신규 확장)                           │
│  - MemoryEntry, Rumor aggregates                       │
│  - MemoryProjector (확장), RetentionService,               │
│    ConsolidationService, RumorSpreadPolicy             │
└────────────────────────────────────────────────────────┘
           │  queries            ▲  events (선택)
           ▼                     │
┌────────────────────────────────────────────────────────┐
│  Shared Kernel                                         │
│  - NpcId, SceneId, FactionId, FamilyId, WorldId       │
│  - PadSnapshot, DomainEventEnvelope                    │
└────────────────────────────────────────────────────────┘
```

### 2.2 관계

- **NPC Mind → Memory**: Mind 컨텍스트의 도메인 이벤트(`DialogueTurnCompleted`, `SceneEnded`, `RelationshipUpdated`, `BeatTransitioned`, `WorldEventOccurred`)를 Memory 컨텍스트가 **구독**한다. Conformist 관계: Memory는 Mind 이벤트 스키마를 그대로 수용.
- **Memory → NPC Mind**: Memory 컨텍스트는 검색 결과를 **쿼리 응답**으로 제공한다 (프롬프트 주입용). 필요시 Memory가 발행하는 이벤트(`MemoryRecalled`, `RumorSpread`)를 Mind가 구독해 감정 점화(규칙 3.7.3) 등에 활용할 수 있다.
- **Shared Kernel**: 양 컨텍스트가 공유하는 최소 타입들. `NpcId`, `SceneId`, PAD 표현 등.

### 2.3 컨텍스트 분할의 의의

현재 단일 프로젝트 안이지만 논리적으로 분리하는 이유는:
- 기억의 불변식(Scope 배타, Source 승격 금지, Supersede 비순환)을 Mind 컨텍스트로부터 보호한다.
- 향후 Memory 컨텍스트를 별도 프로세스/DB로 분리할 여지를 남긴다.
- 기억과 감정의 결합을 **이벤트 경계**로만 허용해 결합도를 낮춘다.

## 3. 유비쿼터스 언어 — 1차 용어의 DDD 매핑

| 1차 용어 | DDD 요소 | 비고 |
|---|---|---|
| 기억 조각 (Memory Entry) | **Aggregate Root** | 독자적 정체성과 생명주기 |
| 주제 (Topic) | Value Object (식별자) | 별도 애그리거트로 승격하지 않음 (§4.3). **시나리오 작가가 선언한 식별자**이며 런타임에 엔진이 자동 생성하지 않는다. 연결은 **문자열 완전 일치**로만 성립 (1차 §2.1). |
| 소유 범위 (Scope) | Value Object (합 타입) | 5 variants |
| 출처 (Source) | Value Object (합 타입) | 4 variants |
| **출처 상태 (Provenance)** | Value Object (enum) | Seeded \| Runtime. Canonical = provenance=Seeded ∧ scope=World (1차 §2.7 R8) |
| 종류 (Type) | Value Object (합 타입) | 7 variants (절차적 지식은 컨텍스트 밖, 1차 §2.4) |
| 계층 (Layer) | Value Object (enum) | A / B. 초기값은 Type × Layer 매핑(1차 §2.5)으로 결정 |
| 잔존도 (Retention) | Value Object | 0~1 불변식 |
| 수명 상수 τ | Value Object (설정) | Type × Source 룩업 테이블 |
| 회상 (Recall) | **Domain Event** | `MemoryRecalled` |
| 통합 (Consolidation) | Domain Service + Event | `MemoryConsolidator` + `MemoryConsolidated` |
| 대체 (Supersede) | 애그리거트 간 참조 | `superseded_by: Option<MemoryEntryId>` |
| 오버레이 (Overlay) | 패턴 (Topic + Supersede의 조합) | 애그리거트 아님 |
| 정보 전달 사건 | **Domain Event** | `InformationTold` — 판정 경로 (a) 명시 발행만 지원. (b)(c)는 향후 확장 (1차 §3.4.1) |
| 소문 (Rumor) | **Aggregate Root** | 기억 조각과 별개의 애그리거트 |
| 소문 원본 (Rumor Canonical) | MemoryEntry 참조 | 같은 Topic의 `MemoryEntry(provenance=Seeded, scope=World)` 존재/부재로 표현. 부재 = 고아 Rumor (1차 §3.4.6) |
| 전달 체인 (Origin Chain) | Value Object | `Vec<NpcId>` |
| 도달 범위 (Reach) | Value Object | 지역·문파·관계망 필터 |
| 왜곡 (Distortion) | Rumor 애그리거트 내부 엔티티 | §4.2 참조 |
| 주입 경로 (Push/Pull) | Domain Service + Application 계층 | `MemoryProjector`, `recall_memory` tool |
| 기억 어투 (Framing) | Domain Service | `MemoryFramer` (출력 표현) |
| 감정 스냅샷 | Value Object | `PadSnapshot` (Shared Kernel; 기존 `domain/pad.rs`의 `EmotionSnapshot`을 노출) |

## 4. 애그리거트 식별

### 4.1 MemoryEntry (기억 조각) — 애그리거트 루트

**정체성**: `MemoryEntryId` (영구 불변 식별자).

**구성 속성 (타입 스케치)**:
```
MemoryEntry {
    id: MemoryEntryId
    scope: MemoryScope           // VO
    source: MemorySource         // VO
    provenance: Provenance       // VO (Seeded | Runtime) — A6
    memory_type: MemoryType      // VO
    layer: MemoryLayer           // VO (A | B) — 초기값은 Type × Layer 매핑으로 결정
    content: MemoryContent       // VO (텍스트)
    topic: Option<TopicId>       // VO
    origin_chain: OriginChain    // VO
    confidence: Confidence       // VO [0,1] — **생성 시 저장되는 통합 신뢰도**
                                 //   Heard/Rumor: stated_confidence × listener_trust
                                 //   Experienced/Witnessed/Seeded: 1.0 또는 작가 지정값
    acquired_by: Option<NpcId>   // Faction/Family Scope 기억에서 "누가 이 공용 기억을 장면에서 획득했는가" 메타 (B3)
    emotional_snapshot: PadSnapshot   // VO
    created_at: Timestamp
    created_seq: EventSequence   // EventStore append sequence — 최신 판정 기준 (A7)
    last_recalled_at: Option<Timestamp>
    recall_count: RecallCount
    superseded_by: Option<MemoryEntryId>
    consolidated_into: Option<MemoryEntryId>
    source_event_id: EventId     // Event Sourcing 추적용
}
```

> **Confidence 의미 고정 (B8)**: `entry.confidence`는 생성 시 단 한 번 계산되어 저장되는 값이며 이후 불변이다. `MemoryRanker`의 `source_confidence(entry)` 런타임 파생은 이 저장값을 입력으로 사용한다.

> **acquired_by 설계 의도 (B3)**: Faction/Family Scope는 공용 기억이지만 "누가 어떤 장면에서 획득했는가"를 메타로 남긴다. 이 필드는 **접근 제어에 쓰이지 않는다** — 같은 Faction 소속 전원이 여전히 회상 가능하다. 서사적 회상("이 비기는 사제가 알려줬지") 연출용 메타이며, Scope 내부 하위 권한 계층(1차 §4.2 범위 외)과 혼동하지 말 것.

**생명주기**:
```
Created ─┬─ Recalled (잔존도 갱신, 반복 가능)
         ├─ Superseded (더 이상 유효 사실 아님)
         ├─ Consolidated (Layer A → Layer B 요약에 흡수)
         └─ Forgotten (논리적; 잔존도 cutoff 이하)
```

**핵심 불변식**:

| 불변식 | 설명 | 1차 규칙 참조 |
|---|---|---|
| I-ME-1 | Scope는 생성 시 정해지고 이후 변경 불가 | 3.2.1 |
| I-ME-2 | Source는 생성 시 정해지고 승격 불가 | 3.2.3 |
| I-ME-3 | `provenance == Seeded`인 엔트리(Canonical)는 `superseded_by`를 통해서만 논리적 무효화 가능, 물리 삭제 불가. τ=∞로 감쇠하지 않는다. | 3.3.1, 3.3.2, 2.7 |
| I-ME-4 | `superseded_by`와 `consolidated_into`는 자기 자신을 가리킬 수 없고, 설정된 후 변경 불가 | 3.3.2, 3.5.5 |
| I-ME-5 | `retention(t)`는 [0,1] 범위 | 2.6 |
| I-ME-6 | `confidence`는 [0,1] 범위, 생성 후 불변 | 2.8, B8 |
| I-ME-7 | `recall_count`는 단조 증가 | 3.5.3 |
| I-ME-8 | 생성 시 Layer는 **Type × Layer 매핑표(1차 §2.5)**로 결정된다. 이후 Layer 변경은 Consolidation(A→B) 한 방향만 허용, B→A 금지 | 2.5, 3.5.5 |
| I-ME-9 | **Faction/Family Scope는 생성 시점의 소속을 기준으로 귀속**되며, 이후 NPC의 소속 변경은 `faction_id`/`family_id` 필드에 반영되지 않는다. 과거 소속 기억의 회상 접근권은 유지된다. | 3.2.4 |
| I-ME-10 | `created_seq`는 EventStore append 시 할당되며 이후 불변. 동일 Topic 내 "최신 유효 엔트리" 판정은 `created_seq` 기준. | 3.3.3 (R9) |

**트랜잭션 경계** (B1 완화):
- **원칙**: 한 커맨드는 논리적 단위 범위 내의 애그리거트들만 변경한다. 단일 엔트리 생성·갱신은 그 범위가 한 애그리거트다.
- **예외 (원자적 다중 변경 허용)**: 다음 두 상황은 같은 BFS cascade 트랜잭션에서 N+1개 애그리거트를 원자적으로 변경할 수 있다.
  - **Supersede**: 새 엔트리 1개 생성 + 기존 엔트리 1개의 `superseded_by` 갱신.
  - **Consolidation**: Layer B 엔트리 1개 생성 + 대상 Layer A 엔트리 N개의 `consolidated_into` 갱신.
- **근거**: 기존 `CommandDispatcher::dispatch_v2`의 BFS → staging → commit 파이프라인이 한 커맨드 내 다수 이벤트를 동일 트랜잭션으로 commit하므로 이 두 케이스는 자연스럽게 원자성을 보장한다.
- **일반 규칙**: 그 외의 관련 애그리거트 변경(예: Rumor 확산에 의한 다수 MemoryEntry 생성)은 단일 BFS cascade 안에서 결정적으로 처리되며 I-RU-5를 트랜잭션 일관성으로 만든다 (B4).

**커맨드**:
- `CreateMemoryEntry` — 신규 생성
- `RecallMemoryEntry` — 검색에 걸렸을 때 발행 (잔존도 기준 시각 갱신)
- `SupersedeMemoryEntry` — 대체 관계 지정
- `ConsolidateMemoryEntry` — Layer A→B 흡수 표시

### 4.2 Rumor (소문) — 애그리거트 루트

**정체성**: `RumorId`.

**존재 이유**: 소문은 여러 NPC에게 `MemoryEntry(Source=Rumor/Heard)`를 **대량 생성**하는 확산 사건이다. 확산 이력·왜곡 계보·도달 범위는 개별 MemoryEntry가 아니라 소문 자체가 관리해야 자연스럽다.

**구성 속성**:
```
Rumor {
    id: RumorId
    topic: Option<TopicId>            // Canonical 참조 키 (A2)
    seed_content: Option<MemoryContent>  // Topic 없는 고아 Rumor만 채움 (1차 §3.4.6 "고아 Rumor" 정책)
    origin: RumorOrigin               // VO: { Seeded, FromWorldEvent(EventId), Authored(NpcId?) }
    reach_policy: ReachPolicy         // VO
    hops: List<RumorHop>              // Entity 목록 (소속 엔티티)
    distortions: List<RumorDistortion> // Entity 목록
    created_at: Timestamp
    status: RumorStatus               // Active | Faded
}

RumorHop {  // Rumor 내부 엔티티
    hop_id: HopId
    hop_index: Count        // 0부터 증가
    content_version: DistortionId?   // 이 홉에서 사용된 내용 버전
    recipients: List<NpcId>
    spread_at: Timestamp
}

RumorDistortion {  // Rumor 내부 엔티티
    distortion_id: DistortionId
    parent_distortion: Option<DistortionId>
    content: MemoryContent
    created_at: Timestamp
}
```

> **Canonical 참조 모델 (A2)**: Rumor는 "원본 서술"을 직접 보유하지 않는다. 대신 `topic`을 통해 같은 Topic의 `MemoryEntry(provenance=Seeded, scope=World)` — 즉 Canonical을 참조한다. Rumor 콘텐츠 해소 규칙:
>
> - `topic`이 있고 해당 Topic에 Canonical 엔트리가 존재하면: 첫 `RumorDistortion`의 `parent_distortion=None`인 content가 Canonical을 복제·각색한 첫 버전이다. 복구 시 Canonical과 Rumor distortion 체인으로 원본 계보가 드러난다.
> - `topic=None` (고아 Rumor): `seed_content`가 반드시 설정되어야 하며, 이것이 소문의 자체 시작점이 된다 (1차 §3.4.6). 향후 `InformationFactualized` 커맨드로 사실화될 때 같은 Topic의 Canonical MemoryEntry를 시딩해 승격 경로를 연다.
> - `topic`은 있지만 Canonical이 아직 없는 경우: "예보된 사실" 상태. `seed_content`를 채워두되 Canonical 시딩 시 링크만 드러난다.

**생명주기**:
```
Seeded | Born ─ Spread (다수 홉) ─ [Mutated] ─ Fade (더 이상 확산 안 함)
```

**핵심 불변식**:

| 불변식 | 설명 | 1차 규칙 참조 |
|---|---|---|
| I-RU-1 | `hops[i].hop_index == i` (단조 증가, 건너뜀 없음) | 3.4.5 |
| I-RU-2 | 기존 hop의 `hop_index`, `recipients`, `spread_at`은 불변 (append-only) | 3.4.6 |
| I-RU-3 | `distortions`는 DAG 구조: `parent_distortion` 체인에 사이클 없음 | 3.4.6 |
| I-RU-4 | Canonical 참조(`topic` + Topic→Canonical 링크)는 불변. `seed_content`는 고아 Rumor에서만 설정되며 설정 후 불변. Canonical 본문 자체는 별도 `MemoryEntry(Seeded, World)`의 `superseded_by`로만 논리 무효화. | 3.4.6, A2 |
| I-RU-5 | 확산(`RumorSpread`)이 일어나면 같은 BFS cascade 트랜잭션 안에서 최소 1개의 `MemoryEntry(Source in {Heard, Rumor})`가 생성된다 (트랜잭션 일관성). | 3.4.4, B1, B4 |
| I-RU-6 | `reach_policy` 바깥 NPC에게는 생성 불가 | 3.4.4 |

**커맨드**:
- `SeedRumor` — 시나리오 시딩 또는 명시 발행
- `SpreadRumor` — N명에게 확산
- `DistortRumor` — 왜곡 버전 파생
- `FadeRumor` — 활성 상태 종료

### 4.3 Topic은 왜 애그리거트가 아닌가

Topic은 여러 MemoryEntry를 묶는 **논리적 키**에 불과하다. 다음 이유로 독립 애그리거트로 승격하지 않는다.

- Topic 자체는 상태가 없다. 속한 엔트리 리스트는 조회 시 계산 가능.
- "같은 Topic 내 최신 유효 엔트리" 불변식은 **append-only + EventStore append sequence (`created_seq`) + `superseded_by`** 로 만족된다. 동일 시각 엔트리가 있어도 append 순서로 선후가 결정되므로 **트랜잭션 일관성이 필요 없는 조회 시 불변식** (read-time invariant).
- Topic을 애그리거트로 만들면 모든 신규 엔트리 생성에 Topic 애그리거트 락을 걸어야 해 확장성을 해친다.

따라서 Topic은 **Value Object 식별자**로 두고, `MemoryEntryRepository`가 Topic 질의 API를 제공한다.

### 4.4 경계 명확화 — 애그리거트가 *아닌* 것들

| 후보 | 왜 아닌가 |
|---|---|
| NpcMemoryBook (NPC별 전체 기억) | 너무 큰 애그리거트. 락 범위가 NPC 전체 기억이 됨. 대신 Repository 질의로 해결. |
| SceneMemorySession (장면별 기억 묶음) | Scene 애그리거트(기존)와 중복. 장면 요약 생성은 **Policy**가 담당하고, 결과물은 `MemoryEntry(Layer=B)` 하나. |
| FactionMemoryBook, FamilyMemoryBook, WorldMemoryBook | 동일 이유. Scope로만 구분하고 애그리거트화하지 않음. |
| InformationTelling | 사건 그 자체. Domain Event로 충분. |

## 5. Value Objects 카탈로그

불변·값 동등성·측정 가능성을 가진 타입들.

### 5.1 분류 값

```
MemoryScope =
  | Personal { npc_id: NpcId }
  | Relationship { a: NpcId, b: NpcId }  // 대칭 — 정렬 규칙: a < b (NpcId lexicographic)
  | Faction { faction_id: FactionId }    // 공용 기억; 귀속 메타는 MemoryEntry.acquired_by (B3)
  | Family { family_id: FamilyId }       // 공용 기억; 귀속 메타는 MemoryEntry.acquired_by (B3)
  | World { world_id: WorldId }

MemorySource = Experienced | Witnessed | Heard | Rumor

Provenance = Seeded | Runtime
// Seeded: 시나리오 작가가 선언한 초기 기억·Canonical·고아 Rumor 시드
// Runtime: 엔진이 이벤트 흐름에서 파생한 기억

MemoryType =
  | DialogueTurn | SceneSummary | BeatTransition
  | RelationshipChange | WorldEvent
  | FactionKnowledge | FamilyFact
  // ProceduralKnowledge는 본 컨텍스트 밖

MemoryLayer = A | B
```

> **MemoryScope::Relationship 대칭성 (A1)**: 두 NPC 사이의 관계 기억은 "관계 그 자체"에 귀속되며 한 쪽 소유자에게 귀속되지 않는다. `{owner, target}` 방향성 구조 대신 `{a, b}` 정렬 쌍으로 모델링해 "A의 B에 대한 기억"과 "B의 A에 대한 기억"이 같은 Scope로 묶인다. 관점 분리(규칙 3.1.4)는 **Scope가 아니라 `source` + `content`가 다른 별개 MemoryEntry 두 개**로 표현한다. 정렬 규칙: `a = min(x, y)`, `b = max(x, y)` (NpcId lexicographic).
>
> **Faction/Family Scope 단순화 (B3)**: 이전안의 `npc_id: Option<NpcId>` 필드는 제거. "누가 이 공용 기억을 장면에서 획득했는가"는 `MemoryEntry.acquired_by` 메타로 이관. Scope는 순수하게 귀속 주체(문파/가문)만 표현한다.

### 5.2 수치·상태 값

```
Retention = Float ∈ [0, 1]
Confidence = Float ∈ [0, 1]
RecallCount = NonNegativeInt
DecayTau = PositiveDuration
```

### 5.3 식별자·내용 값

```
MemoryEntryId = Opaque identifier
RumorId = Opaque identifier
TopicId = Dotted path (예: "화산파.장문인")
MemoryContent = Text (표준화된 한국어 표현)

OriginChain = List<NpcId>  // 빈 리스트 가능 (출처 불명)
```

### 5.4 정책 값

```
ReachPolicy {
    regions: Set<RegionId>
    factions: Set<FactionId>
    npc_ids: Set<NpcId>
    min_significance: Float
}

RumorOrigin =
  | Seeded
  | FromWorldEvent { event_id: EventId }
  | Authored { by: Option<NpcId> }

RumorStatus = Active | Fading | Faded
```

### 5.5 Shared Kernel에서 공유

- `NpcId`, `SceneId`, `FactionId`, `FamilyId`, `WorldId`
- `PadSnapshot { pleasure, arousal, dominance }` — **기존 `domain/pad.rs::EmotionSnapshot`의 공유 형식** (B7). Memory 컨텍스트는 새 타입을 정의하지 않고 Mind 컨텍스트 타입을 Shared Kernel로 승격하여 그대로 재사용한다. PAD 스케일·반올림·NaN 처리 규범은 Mind 컨텍스트 정의를 단일 진실원(single source of truth)으로 삼는다.
- `Timestamp`, `EventId`, `EventSequence`, `SignificanceScore`

## 6. 도메인 이벤트 카탈로그

모든 이벤트는 기존 `EventStore`와 `EventBus`를 통해 흐른다. 이벤트는 과거형 명명.

### 6.1 Memory 컨텍스트 발행 이벤트

| 이벤트 | 발행 시점 | 주요 Payload | 소비자 |
|---|---|---|---|
| `MemoryEntryCreated` | 애그리거트 생성 직후 | entry_id, scope, source, type, layer, topic, confidence, source_event_id | 읽기 프로젝션, (선택) Mind |
| `MemoryEntryRecalled` | 검색에 걸려 실제 주입됨 | entry_id, recalled_at, query_context | RetentionService, 통계 |
| `MemoryEntrySuperseded` | 대체 관계 확정 | old_entry_id, new_entry_id, topic | 세계관 진화 프로젝션 |
| `MemoryEntryConsolidated` | Layer A → Layer B 흡수 | a_entry_ids[], b_entry_id | 통계 |
| `RumorSeeded` | 소문 애그리거트 생성 | rumor_id, topic, origin, seed_content?, reach_policy | 확산 스케줄러 |
| `RumorSpread` | Hop 추가 | rumor_id, hop_index, recipients[], content_version | MemoryEntry 생성 정책 |
| `RumorDistorted` | 왜곡 버전 추가 | rumor_id, distortion_id, parent_distortion, content | (향후) 서사 핸들러 |
| `RumorFaded` | 활성 종료 | rumor_id | 스케줄러 |

### 6.2 Memory 컨텍스트 구독 이벤트 (Mind 컨텍스트 발행)

`InformationTold`·`WorldEventOccurred`는 **Mind 컨텍스트가 발행**하는 이벤트이며(B2), Memory 컨텍스트는 이를 구독해 MemoryEntry를 파생한다. 명령 자체(`Command::TellInformation`, `Command::ApplyWorldEvent`)의 소속은 §6.3 주석 참조.

| 이벤트 | Memory 측 반응 정책 | 산출 |
|---|---|---|
| `DialogueTurnCompleted` | `TurnMemoryEvaluationPolicy` | 조건부 `MemoryEntryCreated` (Layer A) |
| `BeatTransitioned` | `TurnMemoryEvaluationPolicy` | `MemoryEntryCreated` (Layer A, type=BeatTransition) |
| `SceneEnded` | `SceneConsolidationPolicy` | `MemoryEntryCreated` (Layer B, type=SceneSummary) |
| `RelationshipUpdated` | `RelationshipMemoryPolicy` | 조건부 `MemoryEntryCreated` (type=RelationshipChange) |
| `WorldEventOccurred` (신규·Mind 발행) | `WorldOverlayPolicy` | `MemoryEntryCreated` (scope=World) + 필요 시 `MemoryEntrySuperseded` |
| `InformationTold { listener }` (신규·Mind 발행, **청자당 1 이벤트**) | `TellingIngestionPolicy` | `MemoryEntryCreated` (source=Heard 또는 Rumor) |

### 6.3 신규 커맨드

| 커맨드 | 소속 컨텍스트 | 초기 이벤트 | 처리 핸들러 / 경로 |
|---|---|---|---|
| `Command::TellInformation { speaker, listeners, claim, stated_confidence }` | **Mind** (정보 전달은 대사/서사 사건) | `InformationToldRequested` | `InformationPolicy`(Mind) → **청자별로 `InformationTold { listener }` N개 follow-up 발행** (B5). Memory는 §6.2 구독으로 처리. 1차 §3.4.1 **판정 경로 (a)** 명시 발행. |
| `Command::ApplyWorldEvent { topic?, updated_fact, significance }` | **Mind** (세계 사건 자체가 Mind 도메인 이벤트) | `WorldEventRequested` | `WorldOverlayPolicy`(Mind) → `WorldEventOccurred` follow-up. Memory는 §6.2 `WorldOverlayPolicy`로 오버레이+Supersede 처리. 1차 §3.4.1 **판정 경로 (a)** 명시 발행. |
| `Command::SeedRumor { topic?, content, reach, origin }` | **Memory** | `RumorSeedRequested` | `RumorPolicy` → `RumorSeeded` |
| `Command::SpreadRumor { rumor_id, extra_recipients? }` | **Memory** | `RumorSpreadRequested` | `RumorPolicy` → `RumorSpread` follow-up → MemoryEntry 생성 (트랜잭션 일관성, I-RU-5) |

기존 6개 커맨드(`Appraise`, `ApplyStimulus`, `GenerateGuide`, `UpdateRelationship`, `EndDialogue`, `StartScene`)는 변경 없음. 위 4개가 확장분이며 Mind/Memory 소속이 분리된다.

> **`InformationTold` 청자당 1 이벤트 (B5)**: 한 커맨드 `TellInformation`이 N명의 청자를 가지면 InformationPolicy는 N개의 `InformationTold` 이벤트를 발행한다 (각각 `listener: NpcId` 단일). 이렇게 하면:
> - AggregateKey가 `Npc(listener)`로 고정되어 라우팅이 결정적
> - 청자별 trust 계산과 OriginChain 분기가 이벤트 단위로 독립
> - TellingIngestionPolicy가 이벤트 1개당 MemoryEntry 1개의 1:1 매핑을 유지
> - EventStore 재생 시 "누가 누구에게 언제 들었는가"가 이벤트 하나로 그대로 드러난다
>
> `MAX_EVENTS_PER_COMMAND = 20` 한도 내에서 청자 수는 제한된다 (3차에서 확정).
>
> **판정 경로 주석 (A11)**: 1차 §3.4.1의 정보 전달 판정 경로 중 (a)(명시 발행)만 본 확장에서 지원한다. (b)(장면 참여자로부터 유추), (c)(참여자의 회상에서 점화)는 향후 확장. `Command::TellInformation`은 (a)의 구현 경로이고, `Command::ApplyWorldEvent` 역시 (a) 경로의 세계 버전이다.

## 7. 도메인 서비스

상태를 갖지 않는 순수 연산으로, 단일 애그리거트에 자연스럽게 귀속되지 않는 규칙들.

### 7.1 MemoryClassifier

**책임**: `OriginChain` 길이를 근거로 `MemorySource`를 판정한다 (규칙 3.2.2).

**시그니처**: `classify(origin_chain, base_source_hint) -> MemorySource`

### 7.2 RetentionCalculator

**책임**: `MemoryEntry` 상태와 현재 시각으로부터 잔존도를 계산한다.

**시그니처**: `retention(entry, now, tau_table) -> Retention`

**특징**:
- 순수 함수. 읽기 시 계산 (저장값 아님).
- τ 룩업 테이블은 설정 주입 (Type × Source → τ).
- Canonical(World + 시딩분)은 τ=∞ 취급.

### 7.3 MemoryRanker

**책임**: 검색 결과 후보들에 대해 **Source 우선 필터링** → **5요소 가중 점수 산출** 두 단계로 회상 순위를 결정한다 (규칙 1차 §2.6bis + 3.5.1 + 3.6.5 + 3.7.2).

**시그니처**: `rank(candidates, query_context, retention_calc) -> RankedMemories`

**1단계 — Source 우선 필터 (A9, 규칙 1차 §3.6bis)**: 동일 Topic·유사 내용 후보가 여러 Source에 걸쳐 있으면 `Experienced > Witnessed > Heard > Rumor` 순으로 우선 선택. 상위 Source 후보가 있으면 하위 Source 후보를 점수 경합에서 제외한다 ("체험한 사실이 있는데 전해 들은 말을 꺼내지 않는다"). Topic이 다르거나 직접 경쟁이 없는 후보는 2단계에서 함께 점수화된다.

**2단계 — 점수 공식 (A10, 1차 §2.6bis 회상 점수 5요소)**:
```
score(entry) = semantic_similarity(query, entry.content)
             × retention(entry, now)                      // RetentionCalculator
             × source_confidence(entry)                   // = source_weight(entry.source) × entry.confidence
             × emotion_proximity(entry.emotional_snapshot, query.current_pad)
             × temporal_recency(entry.created_at, now)
```

- **semantic_similarity**: 임베딩 코사인 또는 FTS5 점수의 정규화 결합.
- **retention**: 1차 §2.6 감쇠 곡선 기반.
- **source_confidence**: Source 기본 가중치와 생성 시 저장된 `entry.confidence`(B8 불변)의 곱. 런타임 재계산 금지.
- **emotion_proximity**: PAD 공간 거리 기반. 구체 수식은 §12 결정 유보.
- **temporal_recency**: 최근 기억 약한 가산. retention과 **독립 요소**로 구분 — retention은 감쇠곡선, recency는 단기 가산(장면 직후 기억 우선 등).

구체 가중치·정규화·스케일은 3차에서 확정.

### 7.4 MemoryConsolidator

**책임**: Layer A 후보 묶음을 Layer B 요약 `MemoryEntry`로 합치는 절차를 오케스트레이션.

**시그니처**: `consolidate(a_entries[]) -> (b_entry, consolidation_links)`

**특징**:
- 요약 문장 생성은 외부 포트(LLM) 호출 가능.
- 통합 대상 필터는 규칙 3.5.6을 따른다 (관계 변화·세계 사건 제외).

### 7.5 MemoryFramer

**책임**: `MemoryEntry`를 LLM 프롬프트에 삽입할 텍스트로 포맷 (규칙 3.6.4).

**시그니처**: `frame(entry, locale) -> FramedMemoryLine`

**특징**:
- Source에 따라 템플릿 분기.
- Locale별(ko/en) 표현 규범은 3차에서 확정.

### 7.6 RumorDistorter

**책임**: 전달 체인이 길어질 때 내용 변형본을 생성.

**시그니처**: `distort(parent_content, hop_index, context) -> distorted_content`

**특징**:
- 단순 템플릿 치환 또는 LLM 호출.
- 결과는 `RumorDistortion` 엔티티로 Rumor 애그리거트에 귀속.

## 8. 정책 (Policy / Saga)

Mind 컨텍스트의 이벤트를 Memory 컨텍스트의 애그리거트 변경으로 이어주는 반응 규칙들.

### 8.1 TurnMemoryEvaluationPolicy

**트리거**: `DialogueTurnCompleted`, `BeatTransitioned`.

**동작**:
1. 필터 적용 (규칙 3.1.2): 감정 강도·Beat 여부·관계 변동·정보 전달 포함 여부. 필터는 "사건 자체가 기억할 가치가 있는가"를 판정하며, **어느 참여자만 기억하는가**의 선별에는 사용되지 않는다.
2. 필터 통과 시 **해당 장면의 모든 활성 참여자 전원**에 대해 각각 `CreateMemoryEntry` 커맨드를 발행한다 (관점 분리, 규칙 3.1.4 — A3 강화). 참여자별로 `source`·`content`·`emotional_snapshot`이 다른 별개 MemoryEntry가 생성된다.
   - **대상 = 참여자 전원**: 발화자, 청자, 현장 목격자 포함. 선별·샘플링 없음.
   - **제외 = 장면 부재자만**: 해당 장면에 물리적으로 없는 NPC는 본 정책의 대상이 아니다. 이들이 해당 사건을 알게 되는 경로는 `InformationTold` 이벤트 또는 Rumor 확산을 통해서만 열린다.
3. 참여 형태에 따라 Source 자동 분류: 화자/청자 = `Experienced`, 동석자 = `Witnessed`. 필터는 이 단계의 분류에는 관여하지 않는다.

### 8.2 SceneConsolidationPolicy

**트리거**: `SceneEnded`.

**동작**:
1. 해당 `scene_id`의 Layer A 엔트리들을 조회.
2. `MemoryConsolidator.consolidate(...)` 호출.
3. Layer B 엔트리 생성 + 원본 Layer A들에 `consolidated_into` 링크.

### 8.3 RelationshipMemoryPolicy

**트리거**: `RelationshipUpdated`.

**동작**:
1. delta가 유의미(기존 임계 0.05 이상)한가 판정.
2. 유의미하면 **귀속 분기 hook**(A8, 1차 §3.1.2bis) 적용:
   - `RelationshipUpdated.cause`(장면 내 대사/행동 · `InformationTold` · `WorldEventOccurred` · Rumor 등)를 조회.
   - 귀속 원인에 따라 `content`·`source`·`topic`을 분기 생성. 예:
     - 장면 내 사건 → `source=Experienced/Witnessed`, scene_id 링크.
     - 정보 전달 → `source=Heard`, OriginChain에 speaker 포함.
     - 세계 사건 오버레이 → `source=Experienced`, `topic=WorldEvent.topic`.
3. `MemoryEntry(type=RelationshipChange, scope=Relationship{a,b})` 생성. 관점 분리로 당사자 각각 별 엔트리(규칙 3.1.4).
4. 관계 변화는 통합 대상 아님 (규칙 3.5.6).

> **hook 인터페이스 (3차 구체화)**: `RelationshipUpdated` 이벤트 payload에 `cause: RelationshipChangeCause` enum을 추가한다. RelationshipPolicy는 변경 발행 시점에 원인을 이벤트에 각인해야 한다 (A8 보장). 이 enum의 구체 variant는 3차에서 확정.

### 8.4 WorldOverlayPolicy

**트리거**: `WorldEventOccurred`.

**동작**:
1. Topic 지정 시 기존 같은 Topic 최신 엔트리 조회.
2. 새 `MemoryEntry(scope=World, source=Experienced/Witnessed)` 생성.
3. 기존 엔트리가 있으면 `SupersedeMemoryEntry` 커맨드로 대체 링크.
4. 비동기 인지(규칙 3.3.4): 즉시 개인 기억으로 확산하지 않음. 후속 소문 확산은 별도 스케줄링.

### 8.5 TellingIngestionPolicy

**트리거**: `InformationTold`.

**동작**:
1. 청자별로 `OriginChain`을 증가 복제.
2. `MemoryClassifier`로 Source 판정 (체인 길이 ≥2 → Rumor).
3. 신뢰도 산정: `stated_confidence × listener_trust(listener, speaker)` (규칙 3.4.3).
4. `CreateMemoryEntry` 발행.

### 8.6 RumorDistributionPolicy

**트리거**: `RumorSpread`.

**동작**:
1. Hop의 recipients 각각에 대해 Heard/Rumor 기억 엔트리 생성.
2. `content_version`이 있으면 왜곡 버전 적용.
3. 신뢰도는 hop_index에 따라 기하 감소 (규칙 3.4.5).

## 9. 저장소 (Repository) 인터페이스

저장소는 **애그리거트 단위**로만 제공. 조회 편의 메서드는 별도 **Read Model (프로젝션)**으로 제공.

### 9.1 MemoryEntryRepository

- `save(entry)`
- `load(id) -> Option<MemoryEntry>`
- `append_event(domain_event)` — Event Sourcing 정합
- Topic·Scope·Scene 기반 조회 보조 질의 제공

### 9.2 RumorRepository

- `save(rumor)`
- `load(id) -> Option<Rumor>`
- `find_by_topic(topic) -> List<Rumor>`

### 9.3 Read Model (프로젝션)

CQRS 읽기 측. 검색 성능을 위해 별도 인덱스 유지.

- **MemorySearchProjection**: 하이브리드 검색 (FTS5 + vec0). 입력은 질의 텍스트/임베딩, Scope 필터, NPC 컨텍스트. 출력은 `RankedMemories`.
- **TopicLatestProjection**: Topic별 가장 최신 유효 엔트리 포인터 캐시.
- **RumorSpreadProjection**: 활성 소문 + 도달 범위 인덱스.

기존 `SqliteMemoryStore`는 이 중 MemorySearchProjection의 구체 구현이 된다.

## 10. 불변식 체크리스트

애그리거트 경계에서 또는 커맨드 핸들링 시점에 강제되어야 하는 규칙의 통합 목록.

### 10.1 MemoryEntry 불변식

| ID | 규칙 | 강제 시점 |
|---|---|---|
| I-ME-1 | Scope 생성 후 불변 | 커맨드 처리 |
| I-ME-2 | Source 승격 불가 | 커맨드 처리 |
| I-ME-3 | Canonical (`provenance=Seeded ∧ scope=World`) 물리 삭제 금지, `superseded_by`로만 논리 무효화, τ=∞ | Repository + RetentionCalculator |
| I-ME-4 | `superseded_by` / `consolidated_into` 자기 참조 금지, 변경 불가 | 애그리거트 내부 |
| I-ME-5 | `retention ∈ [0,1]` | VO 생성 시 |
| I-ME-6 | `confidence ∈ [0,1]`, 생성 후 불변 | VO 생성 시 |
| I-ME-7 | `recall_count` 단조 증가 | 애그리거트 내부 |
| I-ME-8 | 초기 Layer는 Type×Layer 매핑, 이후 A→B 한 방향만 허용 | 애그리거트 내부 |
| I-ME-9 | Faction/Family 소속 기준은 생성 시점, 이후 NPC 소속 변경은 회상권 영향 없음 | Repository (조회 필터) |
| I-ME-10 | `created_seq` EventStore append 순서로 할당, 동일 Topic 최신 판정 기준 | Repository |

### 10.2 Rumor 불변식

| ID | 규칙 | 강제 시점 |
|---|---|---|
| I-RU-1 | Hop index 단조 연속 | 애그리거트 내부 |
| I-RU-2 | 기존 Hop append-only | 애그리거트 내부 |
| I-RU-3 | Distortion DAG 비순환 | 애그리거트 내부 |
| I-RU-4 | Canonical 참조(`topic` 링크)는 불변, `seed_content`는 설정 후 불변 | 애그리거트 내부 |
| I-RU-5 | Spread는 같은 BFS cascade 트랜잭션에서 최소 1개 MemoryEntry 생성 | 커맨드 처리 (트랜잭션 일관성, B1/B4) |
| I-RU-6 | Reach 바깥 수신자 금지 | 커맨드 처리 |

### 10.3 컨텍스트 간 불변식 (결과적 일관성)

| ID | 규칙 | 보증 메커니즘 |
|---|---|---|
| I-CT-1 | SceneEnded 후 Layer B 요약 생성 | SceneConsolidationPolicy (비동기) |
| I-CT-2 | World 오버레이와 소문 확산의 분리 | WorldOverlayPolicy + 별도 Rumor 스케줄 |
| I-CT-3 | 관점 분리 (한 사건 → N 엔트리) | TurnMemoryEvaluationPolicy |

## 11. Event Sourcing · CQRS와의 정합

### 11.1 쓰기 측 (Write Model)

- 모든 애그리거트 변경은 `CommandDispatcher::dispatch_v2` 파이프라인을 통과한다.
- 기존 아키텍처의 BFS + transactional handler + staging → commit → fanout 패턴을 그대로 따른다.
- 신규 핸들러: `InformationPolicy`, `RumorPolicy`, `WorldOverlayPolicy`, `SceneConsolidationHandler` (Transactional).
- 기존 `MemoryProjector`는 **Inline 핸들러 또는 EventBus 구독자**로 역할을 명확화:
  - Inline: MemoryEntry 생성 직후 임베딩 생성 + Search Projection 갱신.
  - EventBus 구독 유지: 외부 구독자의 at-least-once 복구 경로.

**신규 Policy 우선순위 제안 (B6)**

기존 상수(`application/command/priority.rs`): `SCENE_START=5`, `EMOTION_APPRAISAL=10`, `STIMULUS_APPLICATION=15`, `GUIDE_GENERATION=20`, `RELATIONSHIP_UPDATE=30`. 여기에 Memory 컨텍스트 4개 Transactional 핸들러를 아래와 같이 배치한다.

| Handler | 제안 priority | 위치 근거 |
|---|---|---|
| `WorldOverlayPolicy` | **25** | Guide 직후, Relationship 이전. 세계 오버레이가 장면 프롬프트 guide에는 반영되지 않되, 관계 갱신에는 반영될 수 있도록 RelationshipPolicy보다 앞에 둔다. |
| `InformationPolicy` | **35** | Relationship 갱신 직후. 정보 전달로 관계가 변한 경우 `RelationshipUpdated.cause = InformationTold`가 이미 기록된 상태에서 `InformationTold { listener }`를 발행. |
| `RumorPolicy` | **40** | 정보 전달 이후, 마지막 확산 처리. Rumor 확산은 장면 외부 전파라 장면 흐름 후순위. |
| `SceneConsolidationHandler` | **45** | Scene 종료 지점에 마지막으로 실행. 통합은 다른 모든 관련 Memory 생성이 커밋된 뒤에야 입력이 완결된다. |

priority 상수의 최종 이름·값은 3차에서 확정. 현재 간격(5 단위)은 장래 Policy 삽입 여지를 유지하기 위함.

### 11.2 읽기 측 (Read Model / Projection)

- `MemorySearchProjection`은 Inline projection으로 구현 (commit 직후 갱신).
- `TopicLatestProjection`도 Inline.
- `RumorSpreadProjection`은 Inline.
- 쿼리 API는 `DialogueOrchestrator`·`Director`·Mind Studio가 사용.

### 11.3 복구 (Replay)

- EventStore의 `MemoryEntryCreated/Superseded/Consolidated/Recalled`, `Rumor*` 이벤트만으로 Memory 컨텍스트 전체 상태를 복원 가능해야 한다.
- 임베딩은 결정론적이지 않을 수 있으므로 **임베딩은 복원 시 재계산**하거나 `MemoryEntryCreated` 페이로드에 포함해 재사용.

## 12. 결정 유보 항목 (2차 기준, 3차에서 확정)

- τ 룩업 테이블의 구체 값.
- 감정 근접도 수식 (PAD cosine? Euclidean?).
- `listener_trust` 산정 식 (Relationship.trust와의 변환).
- 통합(Consolidation) 트리거 방식 — 즉시 vs 배치 vs Hybrid.
- 프롬프트 예산 단위와 상한.
- Rumor 확산 스케줄러의 구현 형태 (틱 vs Scene 경계 vs Command 유발).
- Topic 네이밍 규범 (점 표기 계층? 다국어 허용?).
- `MemoryEntryId`·`RumorId`의 물리 형식 (UUID? Content-addressed? Sequence?).
- **B9 — 통합(Consolidation) 배제 Type의 구조적 근거 정리**: 관계 변화·세계 사건을 통합에서 제외하는 근거를 타입 체계로 명문화할지, 규칙 문서로 유지할지.
- **B10 — PadSnapshot 저장 전략**: MemoryEntry가 감정 스냅샷을 저장할 때 전체 벡터를 저장할지, 양자화/앵커 인덱스로 저장할지.
- **B11 — ID 결정성(determinism)**: `MemoryEntryId`·`RumorId`를 결정론적(예: 이벤트 해시 기반)으로 발급할지, 비결정(UUIDv4)로 할지. EventStore replay 재현성과 직결. (위 ID 물리 형식 항목과 교차 참조)

## 13. 다음 단계

**3차 — 구현 설계**

3차에서는 다음을 확정한다.

- 구체 스키마 (SQLite 테이블, sqlite-vec 파티션 키 조정)
- 이벤트 페이로드의 정확한 필드와 버전 관리
- 포트/어댑터 시그니처 (`ports.rs` 확장안)
- 기존 `MemoryProjector` / `SqliteMemoryStore` / `MemoryStore` trait 마이그레이션 전략
- `tuning.rs`에 추가될 상수 기본값
- 테스트 시나리오 (TestContext 확장)
- Mind Studio UI 반영 범위 (표시 전용? 편집 포함?)
- Phase 전개 순서 (Step 1~4, 앞서 대화에서 제안한 로드맵 구체화)

---

**참고**: 본 문서의 모델링은 기존 [아키텍처 v3 (EventBus/CQRS) 문서](../architecture/system-design-eventbus-cqrs.md)의 Phase 3 (RAG) · Phase 7 (WorldKnowledgeStore) · Phase 8 (SummaryAgent) 미래 계획을 통합·구체화한 것이다. 3차 설계에서 해당 문서들과 교차 참조를 갱신한다.
