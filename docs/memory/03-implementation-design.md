# NPC 기억 시스템 — 3차: 구현 설계

> 본 문서는 기억 시스템 설계 3단계 중 **3차 산출물**이다.
> [1차 용어·규칙](01-terms-and-rules.md)과 [2차 DDD 모델링](02-ddd-modeling.md)을 전제로, 실제 Rust 타입·SQLite 스키마·이벤트 페이로드·포트 확장·튜닝 상수·테스트 전략·롤아웃 순서를 확정한다.
>
> 표기는 Rust 2024 edition을 기준으로 하되 본문은 타입 스케치 수준으로 제한하고 상세 구현은 PR 단위에서 채운다.
>
> **2차 대비 반영 이력 (Option C)**: 본 3차 문서는 2차 `02-ddd-modeling.md`의 전면안(A1~A11 · B1~B11)을 모두 구현 단위까지 투영한다. 구체 대응은 §16 "2차 결정 반영 대응표" 참조.

## 1. 설계 원칙

1. **기존 테스트·시나리오를 깨지 않는다**: `MemoryEntry`·`MemoryStore`·`MemoryAgent`의 공개 API는 호환 가능한 확장으로만 수정한다.
2. **Event Sourcing 정합**: 모든 상태 변경은 `CommandDispatcher::dispatch_v2` 파이프라인을 경유한다. append-only, Replay 복원 가능.
3. **런타임 중립**: 코어 로직은 tokio에 직접 의존하지 않는다. `broadcast`/`Stream` 인프라만 이용.
4. **저장소 단일화**: 별도 `WorldKnowledgeStore`를 만들지 않고 기존 `SqliteMemoryStore`에 Scope로 구분한다. 임베딩 파이프라인 통합 유지.
5. **점진 롤아웃**: Step A~D로 나눠 Step별로 단독 머지·단독 가치 보장.
6. **컨텍스트 경계 준수**: Mind 컨텍스트의 명령/이벤트와 Memory 컨텍스트의 명령/이벤트를 코드상 파일 배치·핸들러 위치로도 분리한다(2차 §6.2/§6.3).

## 2. 도메인 타입 — `src/domain/memory.rs` 확장

### 2.1 `MemoryScope` — 신규 VO (2차 §5.1 반영)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MemoryScope {
    Personal { npc_id: NpcId },
    Relationship { a: NpcId, b: NpcId },   // 대칭: new()에서 a < b 강제 (lexicographic)
    Faction    { faction_id: FactionId },  // npc_id 필드 제거 — 귀속 메타는 MemoryEntry.acquired_by (B3)
    Family     { family_id: FamilyId },    // npc_id 필드 제거 — 귀속 메타는 MemoryEntry.acquired_by (B3)
    World      { world_id: WorldId },
}

impl MemoryScope {
    /// 정규화 생성자 — Relationship의 a/b 정렬을 보장한다.
    pub fn relationship(x: NpcId, y: NpcId) -> Self {
        let (a, b) = if x <= y { (x, y) } else { (y, x) };
        Self::Relationship { a, b }
    }

    /// 종류 태그 (SQLite `scope_kind` 컬럼 값과 일치)
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Personal { .. }     => "personal",
            Self::Relationship { .. } => "relationship",
            Self::Faction { .. }      => "faction",
            Self::Family { .. }       => "family",
            Self::World { .. }        => "world",
        }
    }

    /// SQLite `owner_a` 컬럼 투영값.
    /// - Personal: npc_id
    /// - Relationship: a (정렬 후 작은 쪽)
    /// - Faction/Family: faction_id / family_id
    /// - World: world_id
    pub fn owner_a(&self) -> &str { /* ... */ }

    /// SQLite `owner_b` 컬럼 투영값. Relationship에서만 Some (= b).
    pub fn owner_b(&self) -> Option<&str> { /* ... */ }

    /// 이 Scope가 특정 NPC에게 회상 접근권을 허용하는지.
    /// Faction/Family Scope는 I-ME-9에 따라 **획득 시점의 소속 기준**을 쓰므로
    /// 이 메서드는 단순 정적 판정 — 이후 소속 변경은 반영하지 않는다.
    pub fn is_accessible_to(&self, npc_id: &str, world: &NpcWorld, acquired_by: Option<&str>) -> bool { /* ... */ }
}
```

> **H10 잠정 결정 — `owner_a()`의 대칭 Scope 처리**: Relationship은 "작은 쪽"을 owner_a로 투영한다. 이는 **DB 키 일관성** 목적이며, "관계 기억의 주어 NPC"가 아니다. Faction/Family는 faction_id/family_id를 owner_a로 쓴다(개별 NPC id가 아님). MemoryEntry의 기존 `npc_id: String` 필드가 `Personal Scope 전용`으로 좁혀지는 문제는 §2.5 참조. 최종 승인은 §17 결정 사항 1번 참조.

### 2.2 `MemorySource` — 신규 VO

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MemorySource {
    Experienced,
    Witnessed,
    Heard,
    Rumor,
}

impl MemorySource {
    /// OriginChain 길이에서 추론되는 기본 Source (2차 §7.1 MemoryClassifier).
    /// 힌트 파라미터가 Experienced/Witnessed이면 체인 길이와 무관하게 힌트 사용.
    pub fn from_origin_chain(chain_len: usize, hint: Option<Self>) -> Self {
        if let Some(h @ (Self::Experienced | Self::Witnessed)) = hint {
            return h;
        }
        match chain_len {
            0     => Self::Rumor,   // 출처 불명
            1     => Self::Heard,   // 직접 들음
            _     => Self::Rumor,   // 체인 길이 ≥ 2
        }
    }

    /// MemoryRanker 2단계 공식의 source_weight. 값은 tuning.rs 상수에서 조회.
    pub fn weight(&self) -> f32 {
        match self {
            Self::Experienced => SOURCE_W_EXPERIENCED,
            Self::Witnessed   => SOURCE_W_WITNESSED,
            Self::Heard       => SOURCE_W_HEARD,
            Self::Rumor       => SOURCE_W_RUMOR,
        }
    }

    /// Source 우선순위 — 값이 작을수록 우선. Ranker 1단계 필터에서 사용.
    pub fn priority(&self) -> u8 {
        match self {
            Self::Experienced => 0,
            Self::Witnessed   => 1,
            Self::Heard       => 2,
            Self::Rumor       => 3,
        }
    }
}
```

### 2.3 `MemoryType` · `MemoryLayer` — 기존 확장 (2차 §5.1 정합)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MemoryType {
    DialogueTurn,          // Layer A
    BeatTransition,        // Layer A
    RelationshipChange,    // Layer A
    SceneSummary,          // Layer B
    WorldEvent,            // Layer A (또는 B, WorldOverlayPolicy에 따라)
    FactionKnowledge,      // Layer A
    FamilyFact,            // Layer A
    // ProceduralKnowledge는 컨텍스트 외 (1차 §2.4)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MemoryLayer {
    A, // 구체 기억 (turn-level)
    B, // 서술적 요약 (scene-level)
}
```

**Type × Layer 초기값 매핑표 (I-ME-8 강제)**:

| MemoryType | 초기 Layer | 비고 |
|---|---|---|
| `DialogueTurn` | A | Consolidation 대상 |
| `BeatTransition` | A | Consolidation 대상 |
| `RelationshipChange` | A | Consolidation 제외 (1차 §3.5.6) |
| `SceneSummary` | B | Consolidator만 생성 (직접 Create 금지) |
| `WorldEvent` | A | Consolidation 제외 (1차 §3.5.6) |
| `FactionKnowledge` | A | 공용 — Consolidation 대상 아님(시나리오 시딩) |
| `FamilyFact` | A | 공용 — Consolidation 대상 아님(시나리오 시딩) |

기존 `Dialogue` · `Relationship` · `SceneEnd` variant는 Step A에서 위 이름으로 **rename**한다. 직렬화 하위호환을 위해 `#[serde(alias = "Dialogue")]` 등을 달아 구 JSON도 역호환한다.

> **`SceneEnd` → `SceneSummary` 이행 (2차 §4.1 I-ME-8 · §8.2)**: 구 `MemoryType::SceneEnd`는 `SceneSummary`로 대체되고, 구 시나리오 JSON은 serde alias로 읽힌다. 새 파일은 `SceneSummary`로만 쓴다.

### 2.4 `Provenance` — 신규 VO (2차 §5.1, A6)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Provenance {
    Seeded,   // 시나리오 작가가 선언한 초기 기억·Canonical·고아 Rumor 시드
    Runtime,  // 엔진이 이벤트 흐름에서 파생한 기억
}

impl Provenance {
    /// Canonical 판정: provenance=Seeded ∧ scope=World (1차 §2.7 R8)
    pub fn is_canonical(&self, scope: &MemoryScope) -> bool {
        matches!(self, Self::Seeded) && matches!(scope, MemoryScope::World { .. })
    }
}
```

### 2.5 `MemoryEntry` — 기존 구조체 확장 (호환)

```rust
pub struct MemoryEntry {
    // === 식별·Event Sourcing ===
    pub id: String,
    pub created_seq: u64,                     // EventStore append sequence (A7, I-ME-10) — 불변
    pub event_id: u64,                        // 생성 trigger 이벤트 id (source_event_id)

    // === 분류 VO ===
    pub scope: MemoryScope,                   // 생성 후 불변 (I-ME-1)
    pub source: MemorySource,                 // 생성 후 불변 (I-ME-2)
    pub provenance: Provenance,               // 생성 후 불변 (A6)
    pub memory_type: MemoryType,              // 생성 후 불변
    pub layer: MemoryLayer,                   // A→B 한 방향 (I-ME-8)

    // === 내용 ===
    pub content: String,
    pub topic: Option<String>,
    pub emotional_context: Option<(f32, f32, f32)>,  // PadSnapshot — Shared Kernel (B7)

    // === 시간 ===
    pub timestamp_ms: u64,
    pub last_recalled_at: Option<u64>,
    pub recall_count: u32,

    // === Source 메타 ===
    pub origin_chain: Vec<String>,            // 체인 길이가 Heard/Rumor 판정 근거
    pub confidence: f32,                      // [0,1] — 생성 시 1회 계산, 이후 불변 (B8)
    pub acquired_by: Option<String>,          // Faction/Family Scope의 "누가 이 공용 기억을 획득했나" (B3)

    // === 관계 ===
    pub superseded_by: Option<String>,        // I-ME-4
    pub consolidated_into: Option<String>,    // I-ME-4

    // === 레거시 grand-fathered ===
    /// Personal Scope 전용 투영값. 비-Personal Scope에서는 `scope.owner_a().to_string()` 반환.
    /// Step A에서 `#[deprecated]` 주석. 신규 코드는 `entry.scope`를 직접 사용한다.
    pub npc_id: String,
}
```

**하위호환 원칙**: 기존 코드가 `MemoryEntry { id, npc_id, content, ... }`로 생성할 때 쓸 수 있는 `MemoryEntry::personal(...)` 같은 헬퍼 생성자를 제공한다. 신규 필드는 Default 값(scope=`Personal { npc_id }`, source=`Experienced`, provenance=`Runtime`, layer=`A`, confidence=1.0, recall_count=0, acquired_by=None, 나머지 None·empty Vec)으로 채운다.

> **`npc_id` grand-fathered 동작 (H10 잠정)**: 비-Personal Scope에서 `entry.npc_id`는 `entry.scope.owner_a().to_string()`을 반환한다. 이는 **DB 외래키와 Personal-경로 기존 쿼리 호환** 목적이며, 의미적으로는 "주어 NPC"가 아니다. Relationship 엔트리의 경우 실제 주체는 `scope`의 a 또는 b 중 `content`에 따라 달라지므로, 새 로직은 절대 `entry.npc_id`를 주체로 취급하지 말 것. 최종 결정은 §17 1번 참조.

### 2.6 `Rumor` 애그리거트 — `src/domain/rumor.rs` 신규

```rust
pub struct Rumor {
    pub id: String,
    pub topic: Option<String>,                    // Canonical 참조 키 (A2). 없으면 고아 Rumor
    pub seed_content: Option<MemoryContent>,      // `topic=None` 또는 "예보된 사실"일 때만 설정 (I-RU-4, A2). 1차 §3.4.6
    pub origin: RumorOrigin,
    pub reach_policy: ReachPolicy,
    pub hops: Vec<RumorHop>,
    pub distortions: Vec<RumorDistortion>,
    pub created_at: u64,
    pub status: RumorStatus,
}

pub enum RumorOrigin {
    Seeded,
    FromWorldEvent { event_id: u64 },
    Authored { by: Option<String> },
}

pub struct ReachPolicy {
    pub regions: Vec<String>,
    pub factions: Vec<String>,
    pub npc_ids: Vec<String>,
    pub min_significance: f32,
}

pub struct RumorHop {
    pub hop_index: u32,
    pub content_version: Option<String>,  // DistortionId
    pub recipients: Vec<String>,
    pub spread_at: u64,
}

pub struct RumorDistortion {
    pub id: String,
    pub parent: Option<String>,
    pub content: String,
    pub created_at: u64,
}

pub enum RumorStatus { Active, Fading, Faded }
```

**Canonical 콘텐츠 해소** (1차 §3.4.6 + 2차 §4.2 A2):

| 상태 | `topic` | Canonical `MemoryEntry(Seeded, World)` | `seed_content` | 해소 방법 |
|---|---|---|---|---|
| 일반 Rumor | Some | **있음** | None | MemoryEntryRepository에서 같은 topic의 Seeded+World 엔트리를 조회 |
| 고아 Rumor | None | n/a | **Some** | `seed_content` 사용. 향후 `InformationFactualized`로 Canonical 시딩 시 topic 연결 |
| 예보된 사실 | Some | **없음** | Some | 현재는 `seed_content` 사용. Canonical 시딩 직후 링크 자동 가시화 |

불변식(I-RU-1~6)은 `impl Rumor`의 mutator 메서드에서 강제한다. append-only: `add_hop`, `add_distortion`, `fade`만 제공. `seed_content`는 생성자 1회 설정 후 불변. Canonical 참조(`topic` 링크)는 불변.

## 3. 이벤트 페이로드 확장 — `src/domain/event.rs`

### 3.1 신규 `EventPayload` variants

```rust
pub enum EventPayload {
    // ... 기존 15 variants ...

    // ─────────────────────────────────────────
    // Memory 컨텍스트 발행 이벤트
    // ─────────────────────────────────────────
    MemoryEntryCreated {
        entry_id: String,
        scope: MemoryScope,
        source: MemorySource,
        provenance: Provenance,
        memory_type: MemoryType,
        layer: MemoryLayer,
        topic: Option<String>,
        confidence: f32,
        acquired_by: Option<String>,
        created_seq: u64,
        source_event_id: u64,
    },
    MemoryEntryRecalled {
        entry_id: String,
        recalled_at: u64,
        query_context: Option<String>,
    },
    MemoryEntrySuperseded {
        old_entry_id: String,
        new_entry_id: String,
        topic: Option<String>,
    },
    MemoryEntryConsolidated {
        a_entry_ids: Vec<String>,
        b_entry_id: String,
        scene_id: Option<SceneId>,
    },

    RumorSeeded {
        rumor_id: String,
        topic: Option<String>,
        origin: RumorOrigin,
        seed_content: Option<String>,    // canonical_content → seed_content (A2)
        reach_policy: ReachPolicy,
    },
    RumorSpread {
        rumor_id: String,
        hop_index: u32,
        recipients: Vec<String>,
        content_version: Option<String>,
    },
    RumorDistorted {
        rumor_id: String,
        distortion_id: String,
        parent: Option<String>,
    },
    RumorFaded { rumor_id: String },

    // ─────────────────────────────────────────
    // Mind 컨텍스트 발행 이벤트 (Memory가 구독)
    // ─────────────────────────────────────────
    /// 청자당 1 이벤트 (B5). N명 청자 → N개 follow-up 이벤트.
    InformationTold {
        speaker: String,
        listener: String,                // 이전안의 listeners:Vec 제거
        listener_role: ListenerRole,     // Direct(대화 상대) | Overhearer(동석자)
        claim: String,
        stated_confidence: f32,
        origin_chain_in: Vec<String>,
    },
    WorldEventOccurred {
        topic: Option<String>,
        world_id: String,
        updated_fact: String,
        significance: f32,
        witnesses: Vec<String>,
    },

    // ─────────────────────────────────────────
    // 기존 RelationshipUpdated 필드 확장 (A8 hook)
    // ─────────────────────────────────────────
    // 기존 variant에 `cause: RelationshipChangeCause` 필드 추가
}

pub enum ListenerRole {
    Direct,      // 대화 상대
    Overhearer,  // 동석자 (엿들은 자)
}

/// RelationshipUpdated에 실리는 귀속 원인 (A8, 2차 §8.3 hook).
/// Memory 컨텍스트의 RelationshipMemoryPolicy가 이 값으로 content·source·topic을 분기.
pub enum RelationshipChangeCause {
    SceneInteraction { scene_id: SceneId },        // 장면 내 대사/행동 → source=Experienced/Witnessed
    InformationTold { origin_chain: Vec<String> }, // 정보 전달 → source=Heard
    WorldEventOverlay { topic: Option<String> },   // 세계 사건 → source=Experienced, topic 상속
    Rumor { rumor_id: String },                    // 소문 → source=Rumor
    Unspecified,                                   // 마이그레이션·레거시 호환
}
```

기존 `RelationshipUpdated`에 `cause: RelationshipChangeCause` 필드가 추가된다. 기존 발행 지점(RelationshipAgent)은 Step A 단계에서 일괄 `Unspecified`로, Step B 이후 원인을 식별 가능한 지점부터 정식 variant로 채운다.

### 3.2 신규 `*Requested` variants (v2 초기 이벤트)

```rust
pub enum EventPayload {
    // ...
    /// Mind 컨텍스트 — Command::TellInformation의 초기 이벤트.
    /// InformationAgent가 처리하여 청자별 InformationTold follow-up을 발행.
    TellInformationRequested {
        speaker: String,
        listeners: Vec<String>,       // Direct
        overhearers: Vec<String>,     // Overhearer
        claim: String,
        stated_confidence: f32,
        origin_chain_in: Vec<String>,
    },

    /// Mind 컨텍스트 — Command::ApplyWorldEvent의 초기 이벤트.
    ApplyWorldEventRequested {
        topic: Option<String>,
        world_id: String,
        fact: String,
        significance: f32,
        witnesses: Vec<String>,
    },

    /// Memory 컨텍스트 — Command::SeedRumor.
    SeedRumorRequested {
        topic: Option<String>,
        seed_content: Option<String>, // canonical_content → seed_content
        reach: ReachPolicy,
        origin: RumorOrigin,
    },

    /// Memory 컨텍스트 — Command::SpreadRumor.
    SpreadRumorRequested {
        rumor_id: String,
        extra_recipients: Vec<String>,
    },
}
```

### 3.3 `AggregateKey` 확장

`src/domain/aggregate.rs`:

```rust
pub enum AggregateKey {
    // 기존
    Scene { npc_id: String, partner_id: String },
    Npc(String),
    Relationship { owner_id: String, target_id: String },
    // 신규
    Memory(String),       // MemoryEntryId — 엔트리 단위 라우팅
    Rumor(String),        // RumorId
    World(String),        // WorldId — 세계관 오버레이
}
```

이벤트별 `AggregateKey` 매핑:

| 이벤트 | AggregateKey | 근거 |
|---|---|---|
| `MemoryEntry*` | `Memory(entry_id)` | 엔트리 단위 라우팅 |
| `Rumor*` | `Rumor(rumor_id)` | 소문 애그리거트 단위 |
| `WorldEventOccurred` / `ApplyWorldEventRequested` | `World(world_id)` | 세계관 오버레이 |
| `TellInformationRequested` | `Npc(speaker)` | 발화 커맨드의 주체는 화자 |
| **`InformationTold`** | **`Npc(listener)`** | 청자별 1 이벤트(B5). 라우팅이 청자 기준이어야 trust 계산·OriginChain 분기가 결정적 |
| `SeedRumorRequested` / `SpreadRumorRequested` | `Rumor(rumor_id)` | 소문 범위 |

## 4. Command 확장 — `src/application/command/types.rs`

```rust
pub enum Command {
    // 기존 6개 유지
    Appraise(AppraiseRequest),
    ApplyStimulus(StimulusRequest),
    GenerateGuide(GuideRequest),
    UpdateRelationship(RelationshipUpdateRequest),
    EndDialogue(AfterDialogueRequest),
    StartScene(StartSceneRequest),

    // 신규 4개 — 소속 컨텍스트는 2차 §6.3 준수
    TellInformation(TellInformationRequest),   // Mind 컨텍스트
    ApplyWorldEvent(ApplyWorldEventRequest),   // Mind 컨텍스트
    SeedRumor(SeedRumorRequest),               // Memory 컨텍스트
    SpreadRumor(SpreadRumorRequest),           // Memory 컨텍스트
}
```

DTO는 `application/dto.rs`에 `TellInformationRequest/Response` 등으로 추가. 기존 DTO 네이밍 규약(`*Request` / `*Response`) 준수.

## 5. 포트 트레이트 확장 — `src/ports.rs`

### 5.1 `MemoryStore` 확장 (하위호환)

```rust
pub trait MemoryStore: Send + Sync {
    // ─────────────────────────────────────────
    // 기존 메서드 — 호환 유지 (내부에서 신규 search로 포워드)
    // ─────────────────────────────────────────
    fn index(&self, entry: MemoryEntry, embedding: Option<Vec<f32>>) -> Result<(), MemoryError>;
    fn search_by_meaning(&self, q: &[f32], npc_id: Option<&str>, limit: usize) -> Result<Vec<MemoryResult>, MemoryError>;
    fn search_by_keyword(&self, kw: &str, npc_id: Option<&str>, limit: usize) -> Result<Vec<MemoryResult>, MemoryError>;
    fn get_recent(&self, npc_id: &str, limit: usize) -> Result<Vec<MemoryEntry>, MemoryError>;
    fn count(&self) -> usize;

    // ─────────────────────────────────────────
    // 신규 메서드 — Scope/Provenance 인지
    // ─────────────────────────────────────────
    fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryResult>, MemoryError>;
    fn get_by_id(&self, id: &str) -> Result<Option<MemoryEntry>, MemoryError>;

    /// Topic의 최신 유효 엔트리(superseded되지 않은 것). created_seq 역순으로 고른다(I-ME-10).
    fn get_by_topic_latest(&self, topic: &str) -> Result<Option<MemoryEntry>, MemoryError>;

    /// Topic의 Canonical(Seeded + World scope) 엔트리. Rumor 콘텐츠 해소용.
    fn get_canonical_by_topic(&self, topic: &str) -> Result<Option<MemoryEntry>, MemoryError>;

    fn mark_superseded(&self, old_id: &str, new_id: &str) -> Result<(), MemoryError>;
    fn mark_consolidated(&self, a_ids: &[String], b_id: &str) -> Result<(), MemoryError>;
    fn record_recall(&self, id: &str, now_ms: u64) -> Result<(), MemoryError>;
}

pub struct MemoryQuery {
    pub text: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub scope_filter: Option<MemoryScopeFilter>,
    pub source_filter: Option<Vec<MemorySource>>,
    pub layer_filter: Option<MemoryLayer>,
    pub topic: Option<String>,
    pub exclude_superseded: bool,
    pub exclude_consolidated_source: bool,    // Layer A가 이미 B로 흡수된 경우 제외
    pub min_retention: Option<f32>,
    pub current_pad: Option<(f32, f32, f32)>, // 감정 근접 보너스용
    pub limit: usize,
}

pub enum MemoryScopeFilter {
    Any,
    Exact(MemoryScope),
    /// 이 NPC가 접근 가능한 모든 scope (Personal + 참여 Relationship + 소속 Faction/Family + World).
    /// I-ME-9에 따라 Faction/Family 소속 변경은 기존 엔트리 접근권에 영향 없음.
    NpcAllowed(String),
}
```

> **구 메서드 deprecation 경로 (M3)**: `search_by_meaning` / `search_by_keyword` / `get_recent`는 Step B에서 `#[deprecated(since="0.4.0", note="Use search() with MemoryQuery")]` 주석. 내부 구현은 신규 `search()`로 포워드. 완전 제거는 Step D 이후로 연기.

### 5.2 `RumorStore` — 신규 포트

```rust
pub trait RumorStore: Send + Sync {
    fn save(&self, rumor: &Rumor) -> Result<(), MemoryError>;
    fn load(&self, id: &str) -> Result<Option<Rumor>, MemoryError>;
    fn find_by_topic(&self, topic: &str) -> Result<Vec<Rumor>, MemoryError>;
    fn find_active_in_reach(&self, reach: &ReachPolicy) -> Result<Vec<Rumor>, MemoryError>;
}
```

> **Step C1 구현 결정**: `save` 시그니처를 `&Rumor` 참조로 유지한다. 호출자가 `save → add_hop →
> save` 패턴으로 동일 Rumor를 계속 mutate하는 사용이 자연스럽기 때문. `find_active_in_reach`는
> `Active`와 `Fading` 두 상태를 포함한다 (아직 완전히 죽지 않은 소문은 도달 가능).

### 5.3 `InformationTellingPort` — 신규 포트 (선택)

LLM tool binding이 필요한 경우만. 기본 구현은 `CommandDispatcher`가 직접 처리.

## 6. Agent / Handler 확장

2차 §6.2/§6.3 컨텍스트 분리를 코드 배치로 투영한다.

### 6.1 기존 `MemoryAgent` 역할 재정의 — `src/application/memory_agent.rs`

기존 `MemoryAgent`는 **EventBus 구독자 + 인덱싱 전담자**로 역할을 좁힌다. 정책·판정 로직은 §6.3 Inline 핸들러들로 이전.

유지:
- `DialogueTurnCompleted`, `RelationshipUpdated`, `BeatTransitioned`, `SceneEnded` broadcast 구독.
- 임베딩 생성 + `MemoryStore::index` 호출.
- at-least-once 복구(`subscribe_with_lag` → `EventStore.get_events_after_id`) 경로.

확장:
- `MemoryEntryCreated` 이벤트 수신 시 임베딩만 담당 (MemoryEntry 자체 저장은 Inline 핸들러가 이미 수행).
- 즉, MemoryAgent는 **EventBus 계층**에서 동작하는 비동기 인덱싱 워커이지, Transactional EventHandler 체인의 일원은 아니다.

### 6.2 신규 Transactional EventHandler — `src/application/command/agents/`

v2 `EventHandler` trait 구현체. Transactional 단계에서 실행된다.

| 파일 | Agent | 컨텍스트 | 처리 이벤트 | follow-up 이벤트 |
|---|---|---|---|---|
| `information_agent.rs` | `InformationAgent` | **Mind** | `TellInformationRequested` | 청자별 `InformationTold { listener, listener_role }` N개 (B5) |
| `world_overlay_agent.rs` | `WorldOverlayAgent` | **Mind** | `ApplyWorldEventRequested` | `WorldEventOccurred` |
| `rumor_agent.rs` | `RumorAgent` | **Memory** | `SeedRumorRequested`, `SpreadRumorRequested` | `RumorSeeded`, `RumorSpread`, `RumorDistorted` |

### 6.3 신규 Inline EventHandler (Memory 컨텍스트 정책 투영)

2차 §8의 Policy를 Inline EventHandler로 투영. commit 직후 동기 실행되어 MemoryEntry를 생성하고 프로젝션까지 갱신.

| 파일 | Handler | 처리 이벤트 | 정책 (2차 §8) | 산출 |
|---|---|---|---|---|
| `turn_memory_handler.rs` | `TurnMemoryEvaluationHandler` | `DialogueTurnCompleted`, `BeatTransitioned` | §8.1 | **장면 활성 참여자 전원**에 대해 `MemoryEntry(Layer=A)` 생성. Source는 화자/청자=Experienced, 동석자=Witnessed (A3) |
| `scene_consolidation_handler.rs` | `SceneConsolidationHandler` | `SceneEnded` | §8.2 | `MemoryEntry(Layer=B, type=SceneSummary)` + Layer A에 `consolidated_into` 링크 |
| `relationship_memory_handler.rs` | `RelationshipMemoryHandler` | `RelationshipUpdated` | §8.3 | `cause`로 content·source·topic 분기 (A8). 관점 분리: 당사자 a, b 각각 별 엔트리(3.1.4) |
| `world_overlay_handler.rs` | `WorldOverlayHandler` | `WorldEventOccurred` | §8.4 | `MemoryEntry(scope=World)` + 기존 Topic 최신 엔트리 Supersede |
| `telling_ingestion_handler.rs` | `TellingIngestionHandler` | `InformationTold` | §8.5 | 청자 1명당 `MemoryEntry(Heard/Rumor)` 1개. 신뢰도 = stated × listener_trust |
| `rumor_distribution_handler.rs` | `RumorDistributionHandler` | `RumorSpread` | §8.6 | Hop recipients 각각에 `MemoryEntry(Rumor)` 생성. **Inline best-effort** — `MemoryStore.index` 실패는 로그만 남김. 완전 원자성은 §14 참조. |

### 6.4 신규 Inline Projection Handler

| 파일 | Handler | 갱신 대상 |
|---|---|---|
| `memory_search_projection.rs` | `MemorySearchProjectionHandler` | `SqliteMemoryStore` (FTS5 + vec0). `MemoryEntryCreated/Superseded/Consolidated` 수신 |
| `topic_latest_projection.rs` | `TopicLatestProjectionHandler` | Topic→최신 `created_seq` 엔트리 캐시 |
| `rumor_projection.rs` | `RumorProjectionHandler` | 활성 소문 인덱스. Hop index, status 반영 |

### 6.5 우선순위 상수 (`src/application/command/priority.rs`)

2차 §11.1 B6 제안에 정렬.

**Transactional 축** (follow-up 발행, 에러 시 커맨드 중단):
```rust
pub const SCENE_START: i32 = 5;
pub const EMOTION_APPRAISAL: i32 = 10;
pub const STIMULUS_APPLICATION: i32 = 15;
pub const GUIDE_GENERATION: i32 = 20;
pub const WORLD_OVERLAY: i32 = 25;            // Guide 직후, Relationship 이전 (C11)
pub const RELATIONSHIP_UPDATE: i32 = 30;
pub const INFORMATION_TELLING: i32 = 35;
pub const RUMOR_SPREAD: i32 = 40;
pub const AUDIT: i32 = 90;
```

**Inline 축** (commit 이후 동기 실행, Step D 구현에서 SCENE_CONSOLIDATION은 여기로 이동):
```rust
pub const EMOTION_PROJECTION: i32 = 10;
pub const RELATIONSHIP_PROJECTION: i32 = 20;
pub const SCENE_PROJECTION: i32 = 30;
pub const MEMORY_INGESTION: i32 = 40;          // TellingIngestion, RumorDistribution
pub const WORLD_OVERLAY_INGESTION: i32 = 45;   // Step D: Canonical + supersede
pub const RELATIONSHIP_MEMORY: i32 = 50;       // Step D: cause 분기
pub const SCENE_CONSOLIDATION: i32 = 60;       // Step D: Layer A→B 흡수 (가장 마지막)
```

> **C11 잠정 결정 — WorldOverlay = 25**: 2차 §11.1 B6의 안대로 Guide 직후, Relationship 이전에 배치한다. 근거: "세계 오버레이가 장면 프롬프트 guide에는 반영되지 않되, 관계 갱신에는 반영될 수 있어야 한다".
>
> **SceneConsolidation Inline 이동**: 원 설계는 Transactional 축 `SCENE_CONSOLIDATION=45`였으나, Scene 통합은 `SceneEnded` commit 이후 Layer A 인덱싱이 모두 끝난 상태에서 실행되어야 중복/누락 없이 동작한다. Inline 축으로 재배치하고 값은 `60`(가장 늦게)으로 고정. Invariants: `SCENE_CONSOLIDATION > RELATIONSHIP_MEMORY > WORLD_OVERLAY_INGESTION > MEMORY_INGESTION > SCENE_PROJECTION` (priority.rs 테스트 가드).

## 7. SQLite 스키마 — `src/adapter/sqlite_memory.rs` 마이그레이션

### 7.1 기존 테이블

```sql
-- 기존 (변경 대상)
CREATE TABLE memories (
    id TEXT PRIMARY KEY,
    npc_id TEXT NOT NULL,
    content TEXT NOT NULL,
    memory_type TEXT NOT NULL,
    pad_p REAL, pad_a REAL, pad_d REAL,
    timestamp_ms INTEGER NOT NULL,
    event_id INTEGER NOT NULL
);
CREATE VIRTUAL TABLE memories_fts USING fts5(content, tokenize='trigram');
CREATE VIRTUAL TABLE memories_vec USING vec0(npc_id text partition key, embedding FLOAT[1024]);
```

### 7.2 확장 스키마 (2차 전체 반영)

```sql
-- memories: 칼럼 추가 (기존 컬럼 유지 — npc_id는 grand-fathered, H10)
ALTER TABLE memories ADD COLUMN scope_kind TEXT NOT NULL DEFAULT 'personal';
ALTER TABLE memories ADD COLUMN owner_a TEXT;   -- Personal.npc_id | Relationship.a | Faction.faction_id | ...
ALTER TABLE memories ADD COLUMN owner_b TEXT;   -- Relationship.b 전용 (그 외 NULL)
ALTER TABLE memories ADD COLUMN source TEXT NOT NULL DEFAULT 'experienced';
ALTER TABLE memories ADD COLUMN provenance TEXT NOT NULL DEFAULT 'runtime';  -- (A6) — 'seeded'|'runtime'
ALTER TABLE memories ADD COLUMN layer TEXT NOT NULL DEFAULT 'a';
ALTER TABLE memories ADD COLUMN topic TEXT;
ALTER TABLE memories ADD COLUMN origin_chain TEXT;  -- JSON array
ALTER TABLE memories ADD COLUMN confidence REAL NOT NULL DEFAULT 1.0;
ALTER TABLE memories ADD COLUMN acquired_by TEXT;   -- (B3) — Faction/Family Scope의 획득자 메타
ALTER TABLE memories ADD COLUMN created_seq INTEGER NOT NULL DEFAULT 0;   -- (A7) EventStore append seq
ALTER TABLE memories ADD COLUMN last_recalled_at INTEGER;
ALTER TABLE memories ADD COLUMN recall_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE memories ADD COLUMN superseded_by TEXT REFERENCES memories(id);
ALTER TABLE memories ADD COLUMN consolidated_into TEXT REFERENCES memories(id);

-- created_seq는 초기 데이터 마이그레이션 시 기존 event_id로 seed (I-ME-10 근사)

CREATE INDEX IF NOT EXISTS idx_memories_topic ON memories(topic) WHERE topic IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_memories_topic_latest ON memories(topic, created_seq DESC) WHERE topic IS NOT NULL AND superseded_by IS NULL;
CREATE INDEX IF NOT EXISTS idx_memories_scope ON memories(scope_kind, owner_a, owner_b);
CREATE INDEX IF NOT EXISTS idx_memories_superseded ON memories(superseded_by);
CREATE INDEX IF NOT EXISTS idx_memories_source_layer ON memories(source, layer);
CREATE INDEX IF NOT EXISTS idx_memories_provenance ON memories(provenance, scope_kind);  -- Canonical 빠른 조회
CREATE INDEX IF NOT EXISTS idx_memories_canonical ON memories(topic, provenance, scope_kind) WHERE provenance = 'seeded' AND scope_kind = 'world';
```

### 7.3 vec0 파티션 키 조정

현재 `npc_id`만 partition key. 다양한 scope 지원을 위해 복합 파티션 도입:

```sql
-- 신규: 모든 scope에 공통되는 단일 partition key
CREATE VIRTUAL TABLE memories_vec USING vec0(
    partition_key TEXT partition key,  -- "personal:<npc_id>" | "world:<world_id>" | "relationship:<a>:<b>" | "faction:<faction_id>" | "family:<family_id>"
    embedding FLOAT[1024]
);
```

**파티션 키 포맷** (MemoryScope → partition_key 결정성 보장):
- `Personal { npc_id }` → `"personal:<npc_id>"`
- `Relationship { a, b }` → `"relationship:<a>:<b>"` (a < b로 이미 정규화됨)
- `Faction { faction_id }` → `"faction:<faction_id>"`
- `Family { family_id }` → `"family:<family_id>"`
- `World { world_id }` → `"world:<world_id>"`

**마이그레이션 전략**: 기존 vec0 테이블은 dim 고정이라 재생성 필수. 기존 데이터는 migration script로 새 partition key 포맷으로 재인덱싱. dev/test 환경에서는 sqlite 파일을 버리고 처음부터 재생성해도 무방.

### 7.4 신규 테이블 — Rumor

```sql
CREATE TABLE rumors (
    id TEXT PRIMARY KEY,
    topic TEXT,
    seed_content TEXT,                  -- (A2) canonical_content → seed_content. 고아 Rumor 또는 예보된 사실일 때만 NOT NULL
    origin_kind TEXT NOT NULL,          -- 'seeded' | 'from_world_event' | 'authored'
    origin_ref TEXT,                    -- event_id or npc_id
    reach_regions TEXT,                 -- JSON array
    reach_factions TEXT,                -- JSON array
    reach_npc_ids TEXT,                 -- JSON array
    reach_min_significance REAL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at INTEGER NOT NULL
);

CREATE TABLE rumor_hops (
    rumor_id TEXT NOT NULL REFERENCES rumors(id),
    hop_index INTEGER NOT NULL,
    content_version TEXT,               -- DistortionId
    recipients TEXT NOT NULL,           -- JSON array
    spread_at INTEGER NOT NULL,
    PRIMARY KEY (rumor_id, hop_index)
);

CREATE TABLE rumor_distortions (
    id TEXT NOT NULL,
    rumor_id TEXT NOT NULL REFERENCES rumors(id),
    parent TEXT,                         -- FK는 application-level (add_distortion)에서 검증
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (rumor_id, id)           -- composite — Step C1 사후 리뷰에서 전역 UNIQUE → composite로 전환
);

CREATE INDEX IF NOT EXISTS idx_rumors_topic ON rumors(topic) WHERE topic IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_rumors_status ON rumors(status);
```

> **Step C1 사후 리뷰 결정 (Schema v3)**: 구 설계는 `rumor_distortions.id TEXT PRIMARY KEY`
> (전역 UNIQUE)였는데, 서로 다른 rumor가 각자 `"d1"` distortion을 생성할 때 바로 충돌한다.
> Step C1 초기 구현에서 테스트 헬퍼가 `"{rumor_id}:d1"` prefix로 우회했으나, 발행 지점이
> 생기는 Step C3 전에 PK를 `(rumor_id, id)` composite로 전환한다. 마이그레이션은 `migrate_v3`
> 함수에서 테이블 재생성 + 데이터 복사 + DROP + RENAME을 `unchecked_transaction`으로 감싸
> 처리한다.

### 7.5 마이그레이션 코드 위치

`src/adapter/sqlite_memory.rs`의 `SqliteMemoryStore::init_schema()`에 버전 관리 추가:

```rust
const SCHEMA_VERSION: i64 = 3;
// 1: 기존. 2: Step A Foundation (13 컬럼 ALTER + vec0 재생성 + rumor 테이블 선제 생성).
// 3: Step C1 사후 — rumor_distortions를 (rumor_id, id) composite PK로 전환.

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute("CREATE TABLE IF NOT EXISTS schema_meta (version INTEGER PRIMARY KEY)", [])?;
    let current: i64 = conn.query_row("SELECT COALESCE(MAX(version), 0) FROM schema_meta", [], |r| r.get(0))?;
    if current < 1 { /* 기존 DDL */ }
    if current < 2 { /* Step A ALTER + 신규 테이블 + vec0 재생성 */ }
    if current < 3 { /* rumor_distortions composite PK 전환 */ }
    conn.execute("INSERT OR REPLACE INTO schema_meta(version) VALUES (?)", [SCHEMA_VERSION])?;
}
```

## 8. 검색 점수 공식 — `MemoryRanker`

2차 §7.3 A9 + A10 규정에 따라 **2단계** 방식으로 구현.

`src/domain/memory/ranker.rs` 신규.

### 8.1 1단계 — Source 우선 필터 (A9, 1차 §3.6bis)

동일 Topic·유사 내용 후보가 여러 Source에 걸쳐 있으면 `Experienced > Witnessed > Heard > Rumor` 순으로 우선 선택. 상위 Source 후보가 있으면 하위 Source 후보를 **점수 경합에서 제외**한다.

```rust
/// 후보 목록을 Topic(또는 유사 내용 클러스터) 단위로 그룹핑해 상위 Source만 살린다.
pub fn filter_by_source_priority(candidates: Vec<Candidate>) -> Vec<Candidate> {
    // 1. Topic 가진 후보는 Topic별 그룹.
    // 2. Topic 없는 후보는 content 임베딩 코사인 ≥ SIMILARITY_CLUSTER_THRESHOLD로 근사 클러스터.
    // 3. 각 그룹 내 min(source.priority())만 남기고 드랍.
    // 4. 서로 다른 Topic/클러스터 간에는 필터링하지 않음 — 2단계 점수로 경쟁.
}
```

### 8.2 2단계 — 5요소 가중 점수 (A10, 1차 §2.6bis)

```rust
pub fn final_score(
    entry: &MemoryEntry,
    vec_similarity: f32,             // 0~1 (semantic_similarity)
    query_pad: Option<(f32, f32, f32)>,
    now_ms: u64,
    tau_table: &DecayTauTable,
) -> f32 {
    let retention = retention_curve(entry, now_ms, tau_table);
    let source_confidence = source_weight(entry.source) * entry.confidence;   // (B8) — entry.confidence 불변 사용
    let emotion_proximity = query_pad
        .and_then(|q| entry.emotional_context.map(|e| pad_cosine(e, q)))
        .map(|c| 1.0 + c * EMOTION_PROXIMITY_BONUS)
        .unwrap_or(1.0);
    let temporal_recency = recency_boost(entry.timestamp_ms, now_ms);

    vec_similarity * retention * source_confidence * emotion_proximity * temporal_recency
}

fn retention_curve(e: &MemoryEntry, now_ms: u64, tau: &DecayTauTable) -> f32 {
    let ref_ms = e.last_recalled_at.unwrap_or(e.timestamp_ms);
    let age_days = (now_ms - ref_ms) as f32 / DAY_MS as f32;
    let tau_days = tau.lookup(e.memory_type, e.source, e.provenance);   // Provenance 반영 (2차 §5.1 A6)
    if tau_days.is_infinite() { return 1.0; }                            // Canonical: τ=∞
    let base = (-age_days / tau_days).exp();
    let boost = 1.0 + (e.recall_count as f32).ln_1p() * RECALL_BOOST_FACTOR;
    (base * boost).clamp(0.0, 1.0)
}
```

**요소 독립성**:
- `retention`은 감쇠 곡선 기반.
- `temporal_recency`는 retention과 **독립된 단기 가산** (장면 직후 기억 우선 등).
- `source_confidence`는 Source 기본 가중치 × 생성 시 저장된 `entry.confidence`. 런타임 재계산 금지 (B8 불변).
- `emotion_proximity`는 PAD cosine 기반. 구체 정규화는 §15 결정 유보.

### 8.3 MemoryRanker public API

```rust
pub struct MemoryRanker<'a> {
    pub tau_table: &'a DecayTauTable,
}

impl<'a> MemoryRanker<'a> {
    pub fn rank(
        &self,
        candidates: Vec<Candidate>,
        query: &RankQuery,
        now_ms: u64,
    ) -> Vec<RankedEntry> {
        let filtered = filter_by_source_priority(candidates);
        let mut scored: Vec<_> = filtered.into_iter()
            .map(|c| {
                let s = final_score(&c.entry, c.vec_similarity, query.current_pad, now_ms, self.tau_table);
                RankedEntry { entry: c.entry, score: s }
            })
            .filter(|r| r.score >= query.min_score_cutoff)
            .collect();
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(query.limit);
        scored
    }
}
```

## 9. 튜닝 상수 — `src/domain/tuning.rs` 추가

```rust
// === Memory retention ===
pub const MEMORY_RETENTION_CUTOFF: f32 = 0.10;     // 검색 제외 한계
pub const RECALL_BOOST_FACTOR: f32 = 0.15;         // recall 강화 계수
pub const EMOTION_PROXIMITY_BONUS: f32 = 0.30;     // PAD cosine 기반 가산
pub const RECENCY_BOOST_TAU_DAYS: f32 = 3.0;       // 최근성 부스트 수명
pub const SIMILARITY_CLUSTER_THRESHOLD: f32 = 0.85; // Ranker 1단계 클러스터 기준 (Topic 없을 때)

// === Decay τ (days) === 
// Type × Source × Provenance 3축 룩업 (Provenance 추가는 2차 A6 반영)
// Provenance=Seeded ∧ Scope=World (Canonical) → τ=∞
// Provenance=Seeded ∧ 그 외 (시나리오 시드 개인/관계 기억) → 기본값 사용 (일반 Runtime과 동일)
// Provenance=Runtime → 기본값 사용
pub const DECAY_TAU_DEFAULT_DAYS: f32 = 30.0;

// 구체 테이블 (예시값; 실측/튜닝 후 조정)
// DialogueTurn/Experienced: 15
// BeatTransition/Experienced: 45
// SceneSummary/Experienced: 90
// RelationshipChange/Experienced: 60
// DialogueTurn/Witnessed: 30
// DialogueTurn/Heard: 14
// */Rumor: 7
// WorldEvent/Experienced: 180
// FactionKnowledge or FamilyFact (Seeded): ∞  — 시드 공용 지식
// */Seeded + Scope::World: ∞                   — Canonical

// === Storage filters ===
pub const MEMORY_TURN_INTENSITY_THRESHOLD: f32 = 0.5;
pub const MEMORY_RELATIONSHIP_DELTA_THRESHOLD: f32 = 0.05;  // 기존 유지

// === Source confidence weights === (MemoryRanker 2단계, 2차 §7.3)
pub const SOURCE_W_EXPERIENCED: f32 = 1.00;
pub const SOURCE_W_WITNESSED:   f32 = 0.85;
pub const SOURCE_W_HEARD:       f32 = 0.60;
pub const SOURCE_W_RUMOR:       f32 = 0.35;

// === Prompt budget ===
pub const MEMORY_PUSH_TOP_K: usize = 5;            // 시스템 프롬프트 자동 주입 상한
pub const MEMORY_PROMPT_TOKEN_BUDGET: usize = 400; // 대략 토큰 예산 (가이드)

// === Rumor decay ===
pub const RUMOR_HOP_CONFIDENCE_DECAY: f32 = 0.8;   // 홉마다 신뢰도 배율
pub const RUMOR_MIN_CONFIDENCE: f32 = 0.1;
```

`DecayTauTable`은 별도 struct로 `tuning.rs` 하단에 배치하거나 `domain/memory/tau.rs`에 분리. **lookup 시그니처는 `(MemoryType, MemorySource, Provenance) -> f32` 3축**.

## 10. LLM 주입 통합

### 10.1 `DialogueAgent` 확장 — `src/application/dialogue_agent.rs`

```rust
impl<R, C> DialogueAgent<R, C> where R: MindRepository, C: ConversationPort {
    pub fn with_memory(self, store: Arc<dyn MemoryStore>, framer: Arc<dyn MemoryFramer>) -> Self { ... }

    async fn inject_memory_push(&self, npc: &str, pad: Option<(f32,f32,f32)>, query: &str)
        -> Result<String, MindServiceError>
    {
        // 1) 임베딩 생성 (self.analyzer)
        // 2) MemoryStore.search(MemoryQuery { 
        //       scope_filter: NpcAllowed(npc),
        //       current_pad: pad,
        //       exclude_superseded: true,
        //       min_retention: Some(MEMORY_RETENTION_CUTOFF),
        //       limit: MEMORY_PUSH_TOP_K * 3,
        //    })
        // 3) MemoryRanker (1단계 Source 우선 + 2단계 점수) 적용 → top-K
        // 4) MemoryFramer.frame(entry) 반복 → 한 블록 구성
    }
}
```

**훅 지점**:
- `start_session()` 안, `chat.start_session(prompt)` 호출 **전**: appraise 프롬프트에 기억 블록 prepend.
- `turn()` 안, `BeatTransitioned`가 follow-up에 포함되면 `chat.update_system_prompt()` 호출 시 기억 블록 재구성.
- 옵션: 매 turn마다 동적 컨텍스트 주입(설정값 `MEMORY_INJECT_PER_TURN`).

### 10.2 `MemoryFramer` 구현 — `src/presentation/memory_formatter.rs`

```rust
pub trait MemoryFramer: Send + Sync {
    fn frame(&self, entry: &MemoryEntry, locale: &str) -> String;
    fn frame_block(&self, entries: &[MemoryEntry], locale: &str) -> String;
}

pub struct LocaleMemoryFramer { /* locales 참조 */ }
```

Locale TOML에 섹션 추가:

```toml
# locales/ko.toml
[memory.framing]
experienced = "[겪음] {content}"
witnessed   = "[목격] {content}"
heard       = "[전해 들음] {content}"
rumor       = "[강호에 떠도는 소문] {content}"

[memory.framing.block]
header = "\n# 떠오르는 기억\n"
footer = ""
```

### 10.3 Pull 경로 — `recall_memory` tool (Step B는 off)

rig-core의 function calling으로 `ConversationPort`에 등록.

```rust
// RigChatAdapter 내부
fn register_memory_tool(&mut self, store: Arc<dyn MemoryStore>) {
    self.tools.push(Tool::new("recall_memory")
        .description("과거 기억을 검색한다. 소문까지 포함하려면 sources에 Rumor를 넣는다.")
        .parameter("query", String)
        .parameter("sources", Vec<String>, optional)
        .parameter("limit", u32, default=3)
        .handler(move |args| { /* MemoryQuery 변환 → store.search → Ranker */ }));
}
```

로컬 llama-server의 function calling이 불안정한 경우를 대비해 feature flag `memory_tool`로 가드. **Step B 기본값은 off, Push 경로만 먼저 사용.** Pull 경로 활성화는 Step F (향후).

## 11. Mind Studio 반영 범위

### 11.1 백엔드 REST 엔드포인트 (`src/bin/mind-studio/handlers/memory.rs` 신규)

```
GET  /api/memory/search?npc=<id>&q=<text>&limit=5     — 의미 검색 (MemoryQuery 래핑)
GET  /api/memory/by-npc/<npc_id>?limit=20&layer=b     — NPC별 기억 목록
GET  /api/memory/by-topic/<topic>                     — Topic별 이력 (Canonical 포함)
GET  /api/memory/canonical/<topic>                    — Topic의 Canonical 1건 조회
POST /api/memory/entries                              — 수동 생성 (작가 도구, provenance=Seeded)
POST /api/memory/tell                                 — TellInformation 커맨드
POST /api/world/apply-event                           — ApplyWorldEvent 커맨드

GET  /api/rumors                                      — 활성 소문 목록
POST /api/rumors/seed                                 — SeedRumor (seed_content 필드)
POST /api/rumors/<id>/spread                          — SpreadRumor
```

### 11.2 프론트엔드 — `mind-studio-ui/`

**우선 표시 전용 (Step B까지)**:
- NPC 상세 패널에 "기억" 탭 — Layer A / Layer B 분리 표시, Scope/Source/Provenance/Type 뱃지, 잔존도 바, 회상 횟수.
- 프롬프트 미리보기에서 **주입된 기억 블록을 하이라이트**하여 연출 디버깅 지원. 주입 원인(Top-K Ranker 점수)을 tooltip으로 노출.

**편집 기능 (Step C 이후)**:
- Topic별 오버레이 히스토리 뷰어 (세계관 진화 가시화, Canonical 강조).
- 소문 편집기 — Reach 편집, seed_content 편집, 수동 확산 트리거, 고아 Rumor 표시.
- 시나리오 JSON에 `initial_rumors` / `world_knowledge` 섹션 GUI 편집.

### 11.3 SSE 실시간 갱신

기존 `StateEvent` enum에 항목 추가:

```rust
pub enum StateEvent {
    // ...
    MemoryCreated { npc_id: String, entry_id: String, scope_kind: String },
    MemorySuperseded { topic: Option<String>, old_id: String, new_id: String },
    MemoryConsolidated { scene_id: Option<String>, b_entry_id: String, a_count: usize },
    RumorSeeded { rumor_id: String, topic: Option<String> },
    RumorSpread { rumor_id: String, hop_index: u32 },
}
```

프론트엔드 `useStateSync` 훅에 분기 추가 — 기억 관련 이벤트 수신 시 해당 NPC 기억 탭만 targeted refresh.

## 12. 테스트 전략

### 12.1 단위 테스트

- `domain/memory/*`: `MemoryScope::relationship` 대칭 정규화, `MemorySource::from_origin_chain` 체인 판정, `Provenance::is_canonical`, `Retention` 계산(τ=∞ 포함), `Rumor` mutator 불변식.
- `domain/rumor.rs`: Hop index 단조성, Distortion DAG 비순환, status 전이, `seed_content` 설정 제약.
- `domain/memory/ranker.rs`: 1단계 Source 필터(Experienced가 Heard를 밀어냄), 2단계 5요소 점수.

### 12.2 통합 테스트 — `tests/memory/`

기존 `TestContext` 확장. 신규 테스트 파일:

| 파일 | 시나리오 |
|---|---|
| `memory_scope_test.rs` | Personal/Relationship(대칭)/Faction/Family/World 각 Scope 생성·검색 |
| `memory_provenance_test.rs` | Seeded vs Runtime, Canonical(Seeded+World) τ=∞ 확인 |
| `memory_source_test.rs` | OriginChain 길이에 따른 Heard/Rumor 자동 판정 |
| `memory_retention_test.rs` | τ에 따른 감쇠, 회상 시 재강화, cutoff 필터 |
| `memory_consolidation_test.rs` | SceneEnded → Layer B 요약 생성, Layer A supersede, 관계 변화는 제외 |
| `memory_world_overlay_test.rs` | 오버레이 append + Canonical 시딩 + Supersede + topic latest 질의 |
| `memory_telling_test.rs` | TellInformation → 청자별 InformationTold N개 → Heard 생성, chain 길이 2에서 Rumor |
| `memory_relationship_cause_test.rs` | RelationshipUpdated.cause variant별 분기 (Scene / Told / Overlay / Rumor) |
| `rumor_spread_test.rs` | SpreadRumor → N 수신자 MemoryEntry 생성(I-RU-5 트랜잭션 일관성), hop 증가, 신뢰도 기하 감소 |
| `rumor_canonical_resolution_test.rs` | Topic 있음/없음/Canonical 없음 3가지 경우 콘텐츠 해소 |
| `memory_ranker_source_priority_test.rs` | 같은 Topic에 Experienced + Heard 동시 존재 → Experienced만 살아남음 |
| `memory_injection_test.rs` (chat feature) | DialogueAgent start_session에서 기억 블록 주입 검증 |

### 12.3 회귀 테스트

기존 테스트 전수 통과가 Step A 병합 조건. 특히:
- `dispatch_v2_test` — 기존 6 커맨드 동작 유지
- `dialogue_*` (chat feature) — 기본 대화 흐름 유지
- `listener_perspective` feature on/off 둘 다 green
- `MemoryEntry::personal(...)` 헬퍼로 생성한 기존 테스트 코드 green

### 12.4 시나리오 기반 E2E (data/)

`data/scenarios/` 아래 기억 전용 시나리오 추가:

- `scenarios/memory/hearsay-chain.json` — A→B→C 정보 전달 → Heard → Rumor 전환
- `scenarios/memory/world-overlay-faction-change.json` — 문파 장문인 교체 후 Canonical Supersede + 비동기 인지
- `scenarios/memory/scene-consolidation.json` — 10턴짜리 Scene 후 Layer B 요약 생성 확인
- `scenarios/memory/orphan-rumor.json` — 고아 Rumor(Topic 없음, seed_content 있음) 확산 → Canonical 시딩으로 사실화

## 13. Phase 롤아웃 순서

2차 문서 §13의 3차 항목을 구체화.

### Step A — Foundation (단독 머지 가능) ✅ 완료

**범위**:
- `MemoryScope`(대칭 Relationship, 단순 Faction/Family) / `MemorySource` / `MemoryLayer` / `Provenance` VO 도입.
- `MemoryEntry` 필드 확장 (provenance, created_seq, acquired_by 포함).
- `MemoryType` rename (serde alias로 구 JSON 호환).
- SQLite 스키마 v2 마이그레이션.
- `MemoryStore` 신규 메서드 추가 (기존 메서드 유지, `#[deprecated]`는 Step B).
- `MemoryRanker` 순수 함수 구현 (1단계 + 2단계).
- `tuning.rs` 상수 추가 (Provenance 3축 τ 룩업 포함).
- `RelationshipUpdated.cause` 필드 추가 (기본값 `Unspecified`).

**가치**: 기존 행동 변경 없이 신규 필드·검색 API·Ranker가 준비됨. 기존 테스트 전체 통과.

**DoD**:
- 모든 기존 테스트 green
- 신규 단위 테스트 (scope/source/provenance/retention/ranker) green
- `cargo build --features embed` / `--features chat` 전부 green

**구현 결과** (commit `ebc3a8a` + `f0964f6`):
- 구현 파일: `src/domain/memory.rs`, `src/domain/memory/ranker.rs`(신규), `src/domain/tuning.rs`,
  `src/domain/event.rs`, `src/ports.rs`, `src/adapter/sqlite_memory.rs`, `tests/common/in_memory_store.rs`.
- 사용자 승인 결정 (§17): §17.1 `npc_id` grand-father + `#[deprecated]` · §17.4 `mem-{06d}` 순번 ID 유지.
- 신규 단위/통합 테스트 15개 (scope 대칭, source priority, provenance canonical, serde alias,
  retention curve, v1→v2 마이그레이션 백필, supersede, record_recall, MemoryQuery 필터 등).
- `migrate_v2`는 `unchecked_transaction`으로 원자성 보장 (vec0 DROP/CREATE/RE-INSERT 중단 시 롤백).
- `MemoryScopeFilter::NpcAllowed`는 Personal(해당 NPC) + World + Relationship(참여)을 포함.
  Faction/Family 소속 Join은 Step C에서 `NpcWorld` 도입과 함께 확장 예정.
- `partition_key` 포맷(`"personal:<id>"` 등)은 NPC/Faction/Family/World ID에 `:` 문자가
  없다고 가정 (호출자 책임, 런타임 강제는 Step C에서).
- Step A 범위 **외**: `MemoryRanker` 호출 경로 (Step B에서 `DialogueAgent` 주입 시 연결),
  `RumorStore`·Rumor 애그리거트 (Step C), `Command::TellInformation`·`SeedRumor`·`SpreadRumor`
  (Step C), `SceneConsolidationHandler`·`WorldOverlayAgent` (Step D), 이벤트 신규 variant
  (`MemoryEntryCreated` 등 — Step C/D), Rumor 테이블은 빈 상태로 선제 생성만.

### Step B — Injection & Framing ✅ 완료 (Core only)

**범위**: `MemoryFramer` + locale 섹션 추가, `DialogueAgent.inject_memory_push()`, Push 경로 온. Pull(`recall_memory` tool)은 feature-gated off. 구 `search_by_meaning`/`search_by_keyword` deprecated 마킹.

**가치**: NPC가 자신의 경험·세계관을 프롬프트로 실제로 보게 된다. 연기 품질 체감 상승. Source별 어투 차이 확인.

**DoD**:
- `memory_injection_test` green
- Mind Studio 프롬프트 미리보기에 기억 블록 하이라이트 + Ranker 점수 tooltip
- 샘플 시나리오로 LLM 대화 시 주입 동작 수동 검증

**구현 결과** (commit `43cef24` + `17f0cd7`):
- 구현 파일: `src/ports.rs`(MemoryFramer trait + 구 메서드 deprecation),
  `src/presentation/memory_formatter.rs`(신규 LocaleMemoryFramer),
  `locales/ko.toml` + `locales/en.toml`([memory.framing] 섹션),
  `src/application/dialogue_agent.rs`(with_memory + inject_memory_push + 훅),
  `tests/memory_injection_test.rs`(신규).
- 사용자 승인 결정: 범위는 **Core only** (Mind Studio UI는 Step E로 분리).
  재주입 시점은 `start_session` 1회 + `BeatTransitioned` 발생 시 (매 turn 옵션은 Step F).
- 신규 단위/통합 테스트 10건: LocaleMemoryFramer 7개(ko/en source variants, block
  empty/assemble, 미지원 locale fallback, raw content fallback) + memory_injection_test
  3개(start_session prepend, with_memory 미부착 no-op, BeatTransitioned 재주입).
- `start_session` 쿼리: `situation.description`(없으면 `partner_id`).
- `turn` 쿼리: user utterance + listener-converted PAD.
- `Candidate.embedding`은 `None`으로 전달 — 엔트리 자체 임베딩이 `MemoryResult`에 없으므로
  topic-less 후보가 단독 클러스터가 되어 source-priority 필터의 부당한 드롭을 방지 (리뷰
  대응). 엔트리 임베딩 전달은 `MemoryResult` 스키마 확장(Step C/D)에서 보강 예정.
- Step B 범위 **외**:
  - **Mind Studio 프롬프트 미리보기 UI** → Step E (Mind Studio 편집 기능)
  - **Pull 경로 (`recall_memory` tool)** + 매 turn 재주입 → Step F (Phase 5)
  - **`SqliteMemoryStore::search`의 vec0 통합** (현재 `relevance_score=1.0` 하드코딩,
    semantic 검증은 InMemoryStore에서만) → 후속 작업
  - **`record_recall` 세션 내 dedup** → Step C/D 명시적 Command 경로 도입 시
  - 구 `MemoryStore` 메서드 완전 제거 → Step D 이후

### Step C — Telling & Rumor Seeding ✅ 완료 (C1·C2·C3 3 서브-PR)

**범위**:
- `Command::TellInformation`, `Command::SeedRumor`, `Command::SpreadRumor`.
- `InformationAgent` (Mind), `RumorAgent` (Memory).
- `Rumor` 애그리거트 + `RumorStore`.
- Inline 핸들러: `TellingIngestionHandler`, `RumorDistributionHandler`.
- `InformationTold` 청자당 1 이벤트 패턴 (B5).

**가치**: 무협 분위기의 "강호에 떠도는 소문" 연출 가능. NPC간 정보 격차 게임플레이.

**DoD (달성)**:
- `memory_telling_test` (12) / `rumor_spread_test` (8) / `rumor_canonical_resolution_test` (3) green
- 단위 테스트 포함 총 40+ 테스트 green

**범위 외 (본 Step에서 제외)**:
- 시나리오 JSON `initial_rumors` 섹션 + `hearsay-chain.json`/`orphan-rumor.json` 샘플 → Step E 작가 도구와 묶기
- Mind Studio 활성 소문 UI → Step E
- Rumor.status(Fading/Faded) 전이 + `RumorDistorted`/`RumorFaded` 발행 → Step F
- I-RU-5 크로스-store 완전 원자성(MemoryStore 포함) → Step F 재시도 큐
- `RelationshipChangeCause::InformationTold` 분기 → Step D (`RelationshipMemoryHandler`)

**구현 결과**:

- **Step C1 — Foundation** (커밋 `bcb0581` + 사후 리뷰 `30d7f94`):
  - `src/domain/rumor.rs` 신규 — `Rumor` 애그리거트 + 생성자 3종(`new`/`with_forecast_content`/`orphan`) + 불변식 I-RU-1~6 (hop 단조성·DAG 비순환·status 단방향·고아 seed 필수·content_version 참조 무결성) + `validate` / `from_parts(pub(crate))`.
  - `RumorStore` trait (`ports.rs`) + `SqliteRumorStore` (`adapter/sqlite_rumor.rs`, embed).
  - `AggregateKey::Memory(MemoryEntryId)`/`Rumor(RumorId)`/`World(WorldId)` variant 3종.
  - `EventPayload` 11 신규 variant: `MemoryEntryCreated/Superseded/Consolidated`(3) · `RumorSeeded/Spread/Distorted/Faded`(4) · `TellInformationRequested`/`InformationTold`(2) · `SeedRumorRequested`/`SpreadRumorRequested`(2). `ListenerRole` enum(Direct/Overhearer).
  - **사후 리뷰 수정 7건**: ① `MemoryEntryCreated.memory_type` 누락 필드 추가, ② `reach_overlaps.sig_ok` 수식 재설계 (`query >= rumor`), ③ `load_internal`의 `.ok()` 에러 삼킴 수정, ④ `validate()` DAG 순환 검출 활성화 (회피 분기 제거), ⑤ `add_hop` content_version 참조 무결성 검증, ⑥ `rumor_distortions` PRIMARY KEY를 `(rumor_id, id)` composite로 schema v3 마이그레이션, ⑦ `from_parts` 가시성 `pub(crate)` 축소. 17+ 단위 테스트.

- **Step C2 — TellInformation 커맨드 경로** (커밋 `f410e74` + 사후 `ff3d032`):
  - `TellInformationRequest { speaker, listeners, overhearers, claim, stated_confidence, origin_chain_in, topic }` DTO + `Command::TellInformation` variant.
  - `InformationAgent` (Transactional, `priority::INFORMATION_TELLING = 35`) — `TellInformationRequested` 수신 후 listeners + overhearers 각자에게 `InformationTold` follow-up 발행 (B5 청자당 1 이벤트). Direct/Overhearer role 분기.
  - `TellingIngestionHandler` (Inline, Memory) — `InformationTold` 구독해 각 청자의 `MemoryEntry(Personal + Heard/Rumor)` 생성. `confidence = stated_confidence × normalized_trust` (`normalized_trust = (trust.value()+1)/2`, 관계 부재 시 0.5). `origin_chain = [speaker, ...inherited]` → len=1 → Heard, len ≥ 2 → Rumor (`MemorySource::from_origin_chain`).
  - `CommandDispatcher::with_memory(Arc<dyn MemoryStore>)` 빌더 추가.
  - **사후 리뷰 수정 5건**: ① `commit_staging_buffer`가 커맨드 키로 모든 이벤트 aggregate_id를 덮어쓰던 버그 → 이벤트별 `payload.aggregate_key().npc_id_hint()` 보존 (§3.3 B5 라우팅 정상화), ② listeners ∩ overhearers 중복 제거, ③ MemoryEntry id 결정적 생성 (`mem-{event.id:012}-{listener}`), ④ `topic` 필드 DTO → 이벤트 → MemoryEntry 일관 전달 (Step D Canonical 연결 대비), ⑤ MAX_EVENTS_PER_COMMAND=20 경계 테스트 추가.

- **Step C3 — SeedRumor/SpreadRumor 확산** (커밋 `d088470` + 사후 `8413857` + `5ebf37f`):
  - `SeedRumorRequest { topic, seed_content, reach, origin }` + `SpreadRumorRequest { rumor_id, recipients, content_version }` DTO + 해당 `Command` variant 2종. `RumorReachInput`/`RumorOriginInput` serde tag 패턴.
  - `RumorAgent` (Transactional, `priority::RUMOR_SPREAD = 40`) — Seed는 topic/seed 조합에 따라 `Rumor::new`/`with_forecast_content`/`orphan` 분기해 `RumorStore.save`, Spread는 `Rumor.add_hop` 단조성 강제 + 수신자 dedup 후 `RumorSpread` follow-up. 자체 `AtomicU64` counter로 `rumor-{n:012}` 결정적 id 생성.
  - `RumorDistributionHandler` (Inline) — `RumorSpread` 구독해 각 수신자의 `MemoryEntry(source=Rumor)` 생성. 콘텐츠 해소 3-tier: `content_version`(Distortion) → topic Canonical (`MemoryStore::get_canonical_by_topic`) → `rumor.seed_content` → `"[내용 없음]"`. Confidence = `RUMOR_HOP_CONFIDENCE_DECAY^hop_index` (floor `RUMOR_MIN_CONFIDENCE`).
  - `CommandDispatcher::with_rumor(memory_store, rumor_store)` 빌더 — `RumorAgent` + `RumorDistributionHandler` 일괄 등록.
  - **사후 리뷰 수정 5건 + Step F 명기**: ① `rumor_id`가 `event.id=0`으로 충돌하던 버그 (전 SeedRumor가 같은 id로 귀결) → `RumorAgent` 자체 counter, ② 고아 Rumor들이 `"orphan"` 공용 event_store aggregate 버킷 공유하던 문제 → `SeedRumorRequested.pending_id` 필드 + dispatcher `command_seq: AtomicU64`로 커맨드별 고유 `pending-<id>`, ③ `InMemoryRumorStore` 테스트 헬퍼가 프로덕션 대비 느슨 → status 필터 추가, ④ dead Response export 제거, ⑤ 설계 §14 "원자적 commit" 문구를 "Rumor aggregate + RumorSpread 이벤트까지 원자, MemoryStore 쓰기는 Inline best-effort"로 재정의. `RumorDistorted`/`RumorFaded` 및 Fading/Faded spread 가드에 `TODO(step-f):` 명기.

### Step D — Consolidation & World Overlay ✅ 완료

**범위**:
- `SceneConsolidationHandler` (SceneEnded → Layer B 생성).
- `WorldOverlayAgent` (Mind, Transactional) + `WorldOverlayHandler` (Memory Inline) + `Command::ApplyWorldEvent`.
- `RelationshipMemoryHandler` — `RelationshipUpdated.cause` variant별 분기.
- `RelationshipAgent` BeatTransitioned 경로에서 cause=`SceneInteraction { scene_id }` 설정.

**가치**: 장기 플레이에서 기억 폭증 방지. 세계관이 살아 진화.

**DoD (달성)**:
- `memory_consolidation_test` (3) / `memory_world_overlay_test` (6) / `memory_relationship_cause_test` (6) green
- 다턴 Scene 후 Layer A 엔트리 전수가 `consolidated_into` 마킹됨 + Layer B `SceneSummary` 1건 생성
- 세계관 오버레이: `Command::ApplyWorldEvent` → Canonical `MemoryEntry(World, Seeded)` 생성 + 같은 topic 기존 엔트리 supersede
- `get_canonical_by_topic`이 supersede 후 새 Canonical 반환

**범위 외 (본 Step에서 제외, 후속 Phase로 이관)**:
- `TopicLatestProjection` 독립 구조체 — `SqliteMemoryStore.get_by_topic_latest`/`get_canonical_by_topic`이 이미 인덱스 기반 조회를 제공하므로 **별도 Projection struct는 생성하지 않는다** (2차 §10 프로젝션 최소주의). UI 전용 캐시가 필요해지면 Step E에서 추가.
- LLM 기반 요약 Consolidator — 현재는 휴리스틱(첫·끝 content 조합, 120자 cap). 후속 Phase.
- 목격자(`witnesses`) 개별 Personal MemoryEntry 생성 — Step F 예정. 이벤트 payload에는 필드로 유지.
- Target 관점 Relationship MemoryEntry — `RelationshipMemoryHandler`는 현재 owner 관점 엔트리만 생성한다. Target이 같은 변화를 "느꼈다"는 도메인 판단이 필요하므로 Step F로 연기 (§6.3 line 579 완전 충족은 후속 과제).
- `DialogueEndRequested` → cause=`SceneInteraction` 승격 — payload에 scene_id를 명시 추가하는 스키마 변경이 필요해 Step F에서 처리. 현재는 `Unspecified`.
- `RelationshipAgent`의 나머지 cause variant 자동 채우기 (`InformationTold`/`Rumor`/`WorldEventOverlay` 계열) — 해당 경로가 실제로 관계 갱신을 트리거하게 되는 Step F 이후에 연결. 현재 `RelationshipMemoryHandler`는 cause를 입력만 받으면 올바르게 분기함 (단위 테스트로 검증).

**리뷰 후 수정사항 (2차 파이프라인 통과)**:
- **B1**: `WorldOverlayHandler` supersede 정책 좁힘 — `get_canonical_by_topic` 단건만 supersede. 다른 NPC의 Personal Heard/Rumor는 보존.
- **B3**: `SceneConsolidationHandler` 관점 분리 — 참여 NPC별로 자기 Layer A만 흡수하는 Personal summary를 각각 생성. topic = `"scene:{a}:{b}"`로 정규화.
- **H1/H2**: self-Scene 가드 + per-NPC search 실패 시 해당 NPC만 skip (반쪽 summary 방지).
- **H4**: `RelationshipMemoryHandler`가 주도 축(closeness/trust/power)을 content에 `[axis Δ=0.34]` 형식으로 포함.
- **H5**: `with_memory(store)`를 lean(Step C2 호환)으로 복원하고, Step D 번들은 `with_memory_full(store)`로 분리.
- **M7**: SceneSummary에 `topic=Some("scene:{a}:{b}")` 부여 — 후속 `get_by_topic_latest` 조회 편의.
- **M6**: BeatTransitioned → cause=SceneInteraction 경로 E2E 테스트 추가.

**구현 결과**:

- **도메인·이벤트 확장**:
  - `EventKind::{ApplyWorldEventRequested, WorldEventOccurred}` 2종 추가.
  - `EventPayload::{ApplyWorldEventRequested, WorldEventOccurred}` variant (world_id/topic/fact/significance/witnesses).
  - `AggregateKey::World(world_id)` 라우팅.
  - `Command::ApplyWorldEvent(ApplyWorldEventRequest)` + DTO.
  - `tuning::MEMORY_RELATIONSHIP_DELTA_THRESHOLD = 0.05` (관계 변화 기록 하한).
  - `priority::transactional::WORLD_OVERLAY = 25`, `priority::inline::{WORLD_OVERLAY_INGESTION=45, RELATIONSHIP_MEMORY=50, SCENE_CONSOLIDATION=60}`.

- **Agent / Handler 추가**:
  - `WorldOverlayAgent` (Transactional, `priority::WORLD_OVERLAY`) — `ApplyWorldEventRequested → WorldEventOccurred` 1:1 변환. Inline 핸들러가 실제 영속화 담당.
  - `WorldOverlayHandler` (Inline, `priority::WORLD_OVERLAY_INGESTION`) — Canonical `MemoryEntry(scope=World, provenance=Seeded, type=WorldEvent)` 생성 + `topic` 있을 때 기존 유효 엔트리 **모두** supersede (Canonical 여부 불문 — 새 세계 오버레이가 모든 기존 해석을 덮는 정책).
  - `SceneConsolidationHandler` (Inline, `priority::SCENE_CONSOLIDATION`) — `SceneEnded` 수신 시 NpcAllowed 필터로 두 NPC의 Layer A 엔트리 수집 → `MemoryType::{DialogueTurn, BeatTransition}` 만 흡수 → `SceneSummary` Layer B 엔트리 생성 + `mark_consolidated`. 휴리스틱 요약(첫·끝 content 조합).
  - `RelationshipMemoryHandler` (Inline, `priority::RELATIONSHIP_MEMORY`) — `RelationshipUpdated.cause` variant별 분기:
    - `SceneInteraction { scene_id }` → `source=Experienced, topic=None, content="장면에서 {target}과(와)의 관계 변화"`
    - `InformationTold { origin_chain }` → 체인 길이 기반 Heard/Rumor 분기, `origin_chain` 계승
    - `WorldEventOverlay { topic }` → `Experienced`, `topic` 계승
    - `Rumor { rumor_id }` → `Rumor`, `origin_chain=[rumor:{rumor_id}]`
    - `Unspecified` → `Experienced`, 일반 content
    - `MEMORY_RELATIONSHIP_DELTA_THRESHOLD=0.05` 미만 미세 변동은 skip.
  - `RelationshipAgent.handle_relationship_update_with_cause` 도입 — `BeatTransitioned` 경로에서 cause=`SceneInteraction { scene_id: SceneId::new(npc, partner) }` 설정.

- **Dispatcher 통합**:
  - `with_default_handlers()`에 `WorldOverlayAgent` 추가 (transactional 7종: Scene/Emotion/Stimulus/Guide/Relationship/Information/WorldOverlay).
  - `with_memory(store)` 빌더가 Step D Inline 3종 (`WorldOverlayHandler`/`RelationshipMemoryHandler`/`SceneConsolidationHandler`)을 `TellingIngestionHandler`와 함께 일괄 등록.
  - `Command::ApplyWorldEvent` 초기 이벤트 빌더 + `world_id`/`fact` 비어 있으면 `InvalidSituation` 조기 reject, `significance` [0,1] clamp.

- **우선순위 invariants 테스트 추가**: world overlay가 guide 후, relationship 전; world overlay ingestion이 memory ingestion 후; relationship memory가 world overlay ingestion 후; scene consolidation이 가장 마지막에 실행되는지 회귀 가드.

- **테스트**:
  - `memory_consolidation_test` (3): 다턴 Scene 후 Layer A → Layer B 흡수, no-entries no-op, RelationshipChange 타입 제외 검증.
  - `memory_world_overlay_test` (6): Request/Occurred 이벤트 쌍, Canonical 생성, 기존 supersede, topic=None non-supersede, invalid 입력 reject, significance clamp.
  - `memory_relationship_cause_test` (6): EndDialogue 경로 Unspecified → Experienced, 5개 cause variant별 source/topic/chain 분기.
  - 신규 lib 단위 테스트 14개 (WorldOverlayAgent 2 + WorldOverlayHandler 3 + SceneConsolidationHandler 3 + RelationshipMemoryHandler 6).
  - 기존 `dispatch_v2_test::with_default_handlers_registers_expected_counts` 업데이트 (6→7).

### Step E — Mind Studio 편집 기능 (선택, 병렬 진행 가능)

**범위**: Topic 히스토리 뷰어(Canonical 강조), 소문 편집기(seed_content, 고아 표시), 시나리오 편집 GUI.

**가치**: 작가 워크플로우 편의성.

**DoD**:
- 프론트엔드 Vitest 테스트 추가
- 수동 시나리오 편집 → 저장 → 로드 → 재생 플로우 검증

### Step F — (향후) Pull 경로 활성, 백그라운드 Rumor 확산 틱

Phase 5 StoryAgent와 묶어 진행. 본 문서 범위 외.

## 14. 리스크와 대응

| 리스크 | 영향 | 대응 |
|---|---|---|
| vec0 테이블 스키마 재생성 필요 (dim 고정) | 기존 DB 파일 손실 | dev/test는 재생성 OK. 장기적으로 DB 버전 + migrate script 도입 |
| 모든 NPC에 공통인 세계관 기억이 커서 벡터 검색 시 도배 | RAG 품질 저하 | `MemoryScopeFilter::NpcAllowed`가 기본 동작. World scope는 별도 sub-query 후 병합. Source 우선 필터(A9)로 Canonical이 Personal Experienced보다 앞서 나오지 않게. |
| LLM 기반 Consolidation 비용 | 비동기 배치 처리 필요 | Step D 초기에는 단순 휴리스틱(첫 문장 + 마지막 감정 태그) 요약 → 후속 개선 |
| Heard/Rumor 자동 추출이 어려운 LLM 품질 | Heard 생성 누락 | `Command::TellInformation` 명시 호출 경로를 기본으로(판정 경로 (a), A11). 자동 추출((b)(c))은 향후 확장 |
| 기존 `MemoryEntry::npc_id` 필드와 `MemoryScope` 중복 | 코드 복잡도 | Step A에서 `npc_id`를 `scope.owner_a()`의 Personal-경로 투영으로 grand-father (H10). `#[deprecated]` 주석 + 신규 코드는 `entry.scope` 사용 |
| **Rumor 확산과 MemoryEntry 생성의 부분적 원자성** | 소문 aggregate는 갱신됐는데 일부 수신자 기억이 유실될 가능성 | **Rumor aggregate + RumorSpread 이벤트까지만 원자적**: `SpreadRumor` 커맨드의 Transactional phase에서 `RumorAgent`가 `Rumor.add_hop` → `RumorStore.save` → `RumorSpread` commit까지 한 단위로 롤백 가능. **수신자 `MemoryEntry` 쓰기는 Inline best-effort**: Inline phase가 commit 이후에 돌기 때문에 `MemoryStore.index` 실패는 `tracing::warn!`만 남기고 커맨드 전체는 성공으로 마무리된다. 완전한 cross-store 원자성(MemoryStore 포함)은 분산 트랜잭션 없이 불가능하므로 Step F 이후 별도 재시도 큐/ sidecar로 해소 예정. I-RU-5는 "aggregate 일관성" 수준으로 재정의됨. |
| InformationTold N개 이벤트가 `MAX_EVENTS_PER_COMMAND=20` 초과 | 커맨드 실패 | N명 청자에 대해 이벤트 1개씩 발행되므로 청자 수를 감시. 초기 한도 N≤15로 가이드, 필요 시 한도 상향 또는 청자 일괄 이벤트 분리 검토 |
| `RelationshipUpdated.cause` = `Unspecified` 잔존 | RelationshipMemoryHandler 분기 불능 → 기본 branch로만 기억 | Step A에서 기존 RelationshipAgent 발행 지점을 `Unspecified`로 두고, Step B/C에서 원인 소스 추가 시점마다 variant 채우기. 테스트로 감시 |

## 15. 결정 유보 (3차 이후)

아래는 3차 문서 범위를 넘어 후속 Phase에서 확정한다.

- Topic 네이밍 규범의 정식 스펙 (계층 구조, 다국어, 특수문자 허용 범위) — 2차 §12 공통 이슈
- `MemoryEntryId` / `RumorId` 포맷 — 현재 `mem-{06d}` 유지 vs UUID v7 vs Content-hash (결정론 여부, 2차 B11)
- 감정 근접도(`emotion_proximity`) 정규화 수식 — cosine 말고 Euclidean/Mahalanobis 채택 여부
- 감정 점화 효과(1차 §3.7.3) 수식 — 본 Step에서는 구현 안 함, 측정 후 결정
- 백그라운드 Rumor 확산 틱 (Step F)의 스케줄러 형태 — Director tick vs 별도 WorldClock
- LLM 기반 Consolidator의 정확한 프롬프트 템플릿
- 통합(Consolidation) 배제 Type의 구조적 명문화 여부 (2차 B9)
- PadSnapshot 저장 전략 — 벡터 직접 vs 양자화/앵커 인덱스 (2차 B10)
- `listener_trust` 산정 식 — `Relationship.trust`를 어떻게 변환할지
- 프롬프트 예산 상한 단위 (토큰 정확 계수 vs 바이트 근사)

## 16. 2차 결정 반영 대응표

| 2차 항목 | 본 3차 반영 위치 |
|---|---|
| A1 — MemoryScope::Relationship 대칭 `{a, b}` | §2.1 (MemoryScope + `relationship` 생성자), §7.3 partition key 포맷 |
| A2 — Rumor Canonical 참조 모델(seed_content, topic) | §2.6 (Rumor 구조 + Canonical 해소 표), §3.1 RumorSeeded payload, §7.4 `rumors.seed_content` |
| A3 — TurnMemoryEvaluationPolicy 참여자 전원 | §6.3 `TurnMemoryEvaluationHandler` 설명 |
| A6 — Provenance VO | §2.4 Provenance, §2.5 MemoryEntry 필드, §7.2 `provenance` 컬럼, §9 τ 3축 룩업 |
| A7/I-ME-10 — created_seq | §2.5 MemoryEntry, §7.2 `created_seq` 컬럼, §7.2 `idx_memories_topic_latest` |
| A8 — RelationshipChangeCause hook | §3.1 `RelationshipChangeCause` enum, §6.3 `RelationshipMemoryHandler` |
| A9 — Source 우선 필터 | §8.1, §12.2 `memory_ranker_source_priority_test` |
| A10 — 5요소 점수 | §8.2 `final_score` |
| A11 — 판정 경로 (a) 한정 | §4 Command 설명, §14 리스크 |
| B1 — 트랜잭션 경계 완화(Supersede/Consolidation 원자성) | §12.2 `memory_consolidation_test`, §14 리스크 행 |
| B3 — Faction/Family Scope 단순화 + acquired_by | §2.1 (npc_id 제거), §2.5 `acquired_by`, §7.2 `acquired_by` 컬럼 |
| B4 — I-RU-5 aggregate 일관성(수신자 MemoryEntry는 best-effort로 축소) | §14 리스크 행(Rumor 원자성), §12.2 `rumor_spread_test` |
| B5 — InformationTold 청자당 1 이벤트 | §3.1 InformationTold(listener 단일), §3.3 AggregateKey `Npc(listener)`, §6.2 InformationAgent follow-up 설명 |
| B6 — Agent 우선순위 | §6.5 priority 상수 (C11 잠정 결정) |
| B7 — PadSnapshot Shared Kernel | §2.5 `emotional_context` 주석 |
| B8 — Confidence 불변 | §2.5, §8.2 `source_confidence`, §9 튜닝 상수 |
| B9/B10/B11 — 결정 유보 | §15에 각 항목 명시 |

## 17. 추가 결정 필요 사항 (구현 착수 전)

아래 항목은 **본 문서에 임시 결정을 적용**했으나, 착수 전 명시적 승인이 필요하다. 승인 이후 임시 결정과 다르면 해당 절과 튜닝 상수 값을 수정한다.

### 17.1 결정 1 — `MemoryEntry.npc_id` grand-fathered 동작 (H10)

**질문**: 비-Personal Scope(특히 `Relationship{a,b}`)의 `MemoryEntry.npc_id` 필드는 어떤 값을 반환해야 하는가?

**현재 임시 결정**: `entry.scope.owner_a()` 투영(= Relationship의 경우 작은 쪽, Faction/Family는 그룹 ID). 목적은 **DB 외래키·레거시 쿼리 호환**만.

**대안들**:

| 대안 | 설명 | 장점 | 단점 |
|---|---|---|---|
| **A (현재 임시)** | `scope.owner_a()` 투영 | 결정적, DB 단순 | Relationship에서 "의미 없는 값" — 새 코드가 오해할 위험 |
| B | 비-Personal Scope에서 `npc_id = ""`(빈 문자열) | 의미 없음을 명시 | 기존 npc_id NOT NULL 컬럼·외래키와 충돌 |
| C | `npc_id` 필드를 완전 제거, `scope`만 사용 | 깔끔 | 전 레포 대규모 수정(Step A 범위 초과), 외부 API 호환 깨짐 |
| D | Personal Scope 전용으로 강제, 비-Personal은 Option<String> | 타입 안전 | MemoryEntry 구조 변경 크고, FFI 영향 |

**권장**: A를 Step A에서 유지하되 `#[deprecated(note="Use entry.scope")]` 명시. Step D 완료 후 C로 전환 여부 재평가.

### 17.2 결정 2 — `WORLD_OVERLAY` priority 위치 (C11)

**질문**: `WORLD_OVERLAY` 우선순위를 Guide(20) ~ Relationship(30) 사이(= 25)에 둘지, Rumor(40) 이후(= 50)에 둘지?

**현재 임시 결정**: **25** (2차 §11.1 B6 제안대로). 근거: "세계 오버레이가 관계 갱신에 반영될 수 있어야 한다".

**대안들**:

| 대안 | 값 | 시나리오 효과 |
|---|---|---|
| **A (현재 임시)** | 25 | 같은 커맨드 안에서 WorldEvent → Relationship 영향 가능. Guide에는 반영 안 됨(이미 생성됨). |
| B | 50 | Rumor 확산까지 끝난 뒤 오버레이. 관계 갱신과 독립. 같은 커맨드 안 연쇄 단순화. |

**권장**: A를 유지. 같은 장면 안에서 "장문인 교체가 발표됨 → 그 자리 관계 신뢰 급감"과 같은 흐름이 자연스럽다. 만약 `ApplyWorldEvent` 같은 Mind 커맨드가 Relationship 갱신을 동반하지 않는 것이 공식이라면 B로 바꿔도 영향 없다(단, 향후 `cause=WorldEventOverlay` branch는 별도 커맨드에서 처리).

### 17.3 결정 3 — 시나리오 JSON 스펙 (M8)

**질문**: `initial_rumors` / `world_knowledge` / `faction_knowledge` / `family_facts` 필드를 JSON 스키마로 언제 확정할지?

**현재 임시 결정**: Step A는 `MemoryEntry` 구조만, 시나리오 JSON 섹션은 Step C 착수 시 함께 확정 (편집 GUI Step E).

**대안들**:

- A (현재 임시): Step C에 포함. 시나리오 테스트는 코드에서 직접 생성.
- B: Step A에 JSON 스펙 동반 → 테스트 재현성 ↑, 범위 ↑.

**권장**: A. 단, Step A 단위 테스트에서 `MemoryEntry` 시드 헬퍼만 제공하여 로컬 fixture로 대체.

### 17.4 결정 4 — `MemoryEntryId` / `RumorId` 결정성 (B11, §15 연계)

**질문**: ID를 결정론적(이벤트 해시 기반)으로 할지, 랜덤(UUID v7)으로 할지?

**현재 임시 결정**: **결정 유보** (§15에 명시). Step A에서는 기존 `mem-{06d}` 순번 포맷 유지.

**대안들**:

| 대안 | 장점 | 단점 |
|---|---|---|
| A (현재 임시, 순번) | EventStore replay로 같은 ID 재생성 가능 | 분산 환경 충돌 |
| B (UUID v7) | 분산 안전, 시간 정렬 유지 | Replay 시 동일 ID 불가 → projection 재구성 문제 |
| C (Content-hash) | 같은 내용 → 같은 ID (결정성 최대) | 내용이 같은 서로 다른 사건에 대한 구분 어려움 |

**권장**: 현재 테스트 편의로 A 유지. Phase 5+ 분산 고려 시 B로 이행 (projection ID 매핑 테이블 추가 비용 감수).

### 17.5 결정 5 — `MAX_EVENTS_PER_COMMAND` 상한과 청자 수 한도 (B5 연계)

**질문**: `InformationTold` 청자당 1 이벤트이므로 청자 N명이면 N+α개 이벤트가 발행된다. 현재 상한 20에 맞춰 청자 한도를 얼마로 둘지?

**현재 임시 결정**: 청자 ≤ 15 (+α = TellInformationRequested 1 + RelationshipMemoryHandler 산출 1~3). 한도 초과 시 `DispatchV2Error::EventBudgetExceeded`.

**대안들**:

- A (현재 임시): 15 상한.
- B: `MAX_EVENTS_PER_COMMAND`를 50으로 상향.
- C: 청자 배치 분할 → 1 커맨드당 최대 N=10 청자, 초과 시 `TellInformation` 내부에서 분할 커맨드 체인.

**권장**: A를 Step C까지 유지. 실제 시나리오에서 청자 >15인 경우는 "공회 연설" 수준이고 그때는 `Command::ApplyWorldEvent` 경로를 쓰는 게 더 맞다.

---

## 18. 관련 문서 갱신 사항

본 Step을 진행하면서 다음 문서도 함께 갱신한다.

- `docs/architecture/system-design-eventbus-cqrs.md` §9~11 (Phase 3/7/8) → 본 설계로 대체되는 부분 업데이트
- `docs/architecture/b-plan-implementation.md` → B5 이후 Phase 확장으로 **B6 — Memory Expansion** 섹션 추가
- `docs/api/api-reference.md` → MemoryStore / RumorStore 포트 시그니처 반영
- `CLAUDE.md` → 주요 진입점 설명에 Memory 컨텍스트 확장 한 줄 추가 + 신규 Command 4종 표 추가

---

**승인 후 작업 순서 제안**:
1. 본 문서 §17 결정 사항 1~5 승인 (권장안 A로 우선 제안)
2. Step A PR 준비 (스키마 + VO + Provenance + Ranker + 기존 테스트 호환성)
3. Step A 머지 후 Step B 착수
