//! 기억(Memory) 도메인 타입 — RAG 인덱싱 및 검색의 핵심 데이터 구조
//!
//! Step A (Foundation): Scope/Source/Provenance/Layer VO 도입 + MemoryEntry 필드 확장.
//! 기존 struct literal 호환을 위해 `MemoryEntry::personal(...)` 헬퍼 제공.

use serde::{Deserialize, Serialize};

pub mod ranker;

// ---------------------------------------------------------------------------
// MemoryScope — 기억의 소유/접근 범위
// ---------------------------------------------------------------------------

/// 기억의 소유·접근 범위 (Personal/Relationship/Faction/Family/World).
/// Relationship은 대칭적이며 `relationship(a, b)` 생성자로 `a ≤ b` 정규화된다.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MemoryScope {
    Personal { npc_id: String },
    Relationship { a: String, b: String },
    Faction { faction_id: String },
    Family { family_id: String },
    World { world_id: String },
}

impl MemoryScope {
    /// 관계 Scope 정규화 생성자 — `a ≤ b`를 강제해 `relationship(x, y) == relationship(y, x)`.
    pub fn relationship(x: impl Into<String>, y: impl Into<String>) -> Self {
        let (x, y) = (x.into(), y.into());
        let (a, b) = if x <= y { (x, y) } else { (y, x) };
        Self::Relationship { a, b }
    }

    /// SQLite `scope_kind` 컬럼 값.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Personal { .. } => "personal",
            Self::Relationship { .. } => "relationship",
            Self::Faction { .. } => "faction",
            Self::Family { .. } => "family",
            Self::World { .. } => "world",
        }
    }

    /// SQLite `owner_a` 투영값.
    pub fn owner_a(&self) -> &str {
        match self {
            Self::Personal { npc_id } => npc_id,
            Self::Relationship { a, .. } => a,
            Self::Faction { faction_id } => faction_id,
            Self::Family { family_id } => family_id,
            Self::World { world_id } => world_id,
        }
    }

    /// SQLite `owner_b` 투영값. Relationship에서만 Some.
    pub fn owner_b(&self) -> Option<&str> {
        match self {
            Self::Relationship { b, .. } => Some(b),
            _ => None,
        }
    }

    /// vec0 partition key (scope별 결정적 포맷).
    ///
    /// **제약**: NPC/Faction/Family/World ID는 `:` 문자를 포함하지 않아야 한다.
    /// 포함되면 서로 다른 scope 조합이 같은 partition_key를 생성해 벡터 검색 격리가
    /// 깨진다 (예: `relationship:a:b:c` vs `relationship:a:b:c`). 현재 코드에서
    /// 강제하지는 않으며, 시나리오 JSON · NPC 등록 시 호출자가 보장할 책임이 있다.
    pub fn partition_key(&self) -> String {
        match self {
            Self::Personal { npc_id } => format!("personal:{npc_id}"),
            Self::Relationship { a, b } => format!("relationship:{a}:{b}"),
            Self::Faction { faction_id } => format!("faction:{faction_id}"),
            Self::Family { family_id } => format!("family:{family_id}"),
            Self::World { world_id } => format!("world:{world_id}"),
        }
    }
}

// ---------------------------------------------------------------------------
// MemorySource — 기억의 출처 (경험/목격/전해들음/소문)
// ---------------------------------------------------------------------------

/// 기억의 출처 4단계. Ranker 1단계 필터·2단계 점수에서 쓰인다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    Experienced,
    Witnessed,
    Heard,
    Rumor,
}

impl MemorySource {
    /// Ranker 2단계 가중치 (tuning 상수에서 조회).
    pub fn weight(self) -> f32 {
        use crate::domain::tuning::{
            SOURCE_W_EXPERIENCED, SOURCE_W_HEARD, SOURCE_W_RUMOR, SOURCE_W_WITNESSED,
        };
        match self {
            Self::Experienced => SOURCE_W_EXPERIENCED,
            Self::Witnessed => SOURCE_W_WITNESSED,
            Self::Heard => SOURCE_W_HEARD,
            Self::Rumor => SOURCE_W_RUMOR,
        }
    }

    /// 우선순위 (작을수록 상위). Ranker 1단계 source priority filter에서 사용.
    pub fn priority(self) -> u8 {
        match self {
            Self::Experienced => 0,
            Self::Witnessed => 1,
            Self::Heard => 2,
            Self::Rumor => 3,
        }
    }

    /// OriginChain 길이에서 추론되는 기본 Source.
    ///
    /// - 힌트가 `Experienced` 또는 `Witnessed`이면 체인 길이 무시하고 힌트를 반환
    ///   (직접 체험/목격이라는 외부 단서가 명시적으로 주어진 경우).
    /// - 힌트가 `Heard`/`Rumor`이거나 `None`이면 체인 길이 기반 판정:
    ///   - `0` → `Rumor` (출처 불명)
    ///   - `1` → `Heard` (직접 들음)
    ///   - `≥2` → `Rumor` (재전파)
    pub fn from_origin_chain(chain_len: usize, hint: Option<Self>) -> Self {
        if let Some(h @ (Self::Experienced | Self::Witnessed)) = hint {
            return h;
        }
        match chain_len {
            0 => Self::Rumor,
            1 => Self::Heard,
            _ => Self::Rumor,
        }
    }
}

// ---------------------------------------------------------------------------
// Provenance — 시나리오 시드 vs 런타임 파생
// ---------------------------------------------------------------------------

/// 기억의 출처 계보 — 시나리오 작가가 시드했는지 엔진이 런타임에 파생했는지.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// 시나리오 작가가 선언한 초기 기억·Canonical·고아 Rumor 시드
    Seeded,
    /// 엔진이 이벤트 흐름에서 파생
    Runtime,
}

impl Provenance {
    /// Canonical 판정: `Seeded ∧ scope=World` → τ=∞ (영구 사실).
    pub fn is_canonical(self, scope: &MemoryScope) -> bool {
        matches!(self, Self::Seeded) && matches!(scope, MemoryScope::World { .. })
    }
}

// ---------------------------------------------------------------------------
// MemoryLayer — 구체 기억(A) vs 서술적 요약(B)
// ---------------------------------------------------------------------------

/// 기억 계층: A는 turn-level 구체, B는 scene-level 서술 요약 (Consolidation 결과).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MemoryLayer {
    A,
    B,
}

// ---------------------------------------------------------------------------
// MemoryType — 기억 유형
// ---------------------------------------------------------------------------

/// 기억 유형.
///
/// Step A에서 variant 이름을 명시화하되 serde alias로 구 JSON (`Dialogue`, `SceneEnd`, `Relationship`)
/// 역호환을 보장한다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryType {
    /// 대화 턴 (대사). 구 이름: `Dialogue`.
    #[serde(alias = "Dialogue")]
    DialogueTurn,
    /// 관계 변화. 구 이름: `Relationship`.
    #[serde(alias = "Relationship")]
    RelationshipChange,
    /// Beat 전환
    BeatTransition,
    /// Scene 종료 요약 (Layer B). 구 이름: `SceneEnd`.
    #[serde(alias = "SceneEnd")]
    SceneSummary,
    /// 외부 게임 이벤트 (후속 Step에서 WorldEvent로 재편 예정)
    GameEvent,
    /// 세계관 이벤트 (Step D 이후 사용)
    WorldEvent,
    /// 문파·조직 공용 지식 (Step C 이후)
    FactionKnowledge,
    /// 가문·혈연 공용 사실 (Step C 이후)
    FamilyFact,
}

impl MemoryType {
    /// 초기 Layer 매핑 (I-ME-8). SceneSummary만 B, 그 외 A.
    pub fn initial_layer(&self) -> MemoryLayer {
        match self {
            Self::SceneSummary => MemoryLayer::B,
            _ => MemoryLayer::A,
        }
    }

    /// 영속화용 문자열 표현. 저장소 스키마의 일부이므로 Rust 식별자 변경과 무관하게 유지된다.
    pub fn as_persisted(&self) -> &'static str {
        match self {
            MemoryType::DialogueTurn => "DialogueTurn",
            MemoryType::RelationshipChange => "RelationshipChange",
            MemoryType::BeatTransition => "BeatTransition",
            MemoryType::SceneSummary => "SceneSummary",
            MemoryType::GameEvent => "GameEvent",
            MemoryType::WorldEvent => "WorldEvent",
            MemoryType::FactionKnowledge => "FactionKnowledge",
            MemoryType::FamilyFact => "FamilyFact",
        }
    }

    /// 영속화된 문자열 → 변종. 구 이름(`Dialogue`, `SceneEnd`, `Relationship`)도 역호환.
    pub fn from_persisted(s: &str) -> Option<Self> {
        match s {
            "DialogueTurn" | "Dialogue" => Some(MemoryType::DialogueTurn),
            "RelationshipChange" | "Relationship" => Some(MemoryType::RelationshipChange),
            "BeatTransition" => Some(MemoryType::BeatTransition),
            "SceneSummary" | "SceneEnd" => Some(MemoryType::SceneSummary),
            "GameEvent" => Some(MemoryType::GameEvent),
            "WorldEvent" => Some(MemoryType::WorldEvent),
            "FactionKnowledge" => Some(MemoryType::FactionKnowledge),
            "FamilyFact" => Some(MemoryType::FamilyFact),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// MemoryEntry — 기억 항목 (확장)
// ---------------------------------------------------------------------------

/// 기억 항목 — NPC가 과거에 경험한 사건/대화/관계 변화의 기록.
///
/// Step A에서 scope·source·provenance·layer 등 분류 VO와 Event Sourcing 메타(created_seq)가
/// 추가되었다. 신규 필드는 `MemoryEntry::personal(...)` 생성자로 기본값이 채워진다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    // === 식별·Event Sourcing ===
    /// 고유 식별자
    pub id: String,
    /// EventStore append 순번 (I-ME-10). 생성 후 불변.
    #[serde(default)]
    pub created_seq: u64,
    /// 이 기억을 생성한 도메인 이벤트 ID
    pub event_id: u64,

    // === 분류 VO ===
    /// 소유·접근 범위 (생성 후 불변). 구 `npc_id` 필드와 중복 — 신규 코드는 이 필드를 사용.
    #[serde(default = "default_personal_scope")]
    pub scope: MemoryScope,
    /// 출처 (경험/목격/전해들음/소문). 생성 후 불변.
    #[serde(default = "default_source")]
    pub source: MemorySource,
    /// 시드 vs 런타임. 생성 후 불변.
    #[serde(default = "default_provenance")]
    pub provenance: Provenance,
    /// 기억 유형
    pub memory_type: MemoryType,
    /// 계층 (A: 구체, B: 요약). A→B 한 방향 전이.
    #[serde(default = "default_layer")]
    pub layer: MemoryLayer,

    // === 내용 ===
    /// 기억 내용 (검색 대상 텍스트)
    pub content: String,
    /// 논리 Topic 식별자 (시나리오 작가가 선언)
    #[serde(default)]
    pub topic: Option<String>,
    /// 기억 시점의 감정 컨텍스트 (Pleasure, Arousal, Dominance)
    pub emotional_context: Option<(f32, f32, f32)>,

    // === 시간 ===
    /// 기억 시점 타임스탬프 (Unix epoch ms)
    pub timestamp_ms: u64,
    /// 최근 회상 시각 (없으면 생성 시각 사용)
    #[serde(default)]
    pub last_recalled_at: Option<u64>,
    /// 회상 누적 횟수
    #[serde(default)]
    pub recall_count: u32,

    // === Source 메타 ===
    /// 전달 체인 — 체인 길이가 Heard/Rumor 판정 근거
    #[serde(default)]
    pub origin_chain: Vec<String>,
    /// 통합 신뢰도 [0,1] — 생성 시 1회 계산, 이후 불변.
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    /// Faction/Family Scope의 "누가 이 공용 기억을 획득했나" 메타
    #[serde(default)]
    pub acquired_by: Option<String>,

    // === 관계 ===
    /// 이 엔트리를 대체한 다른 엔트리 ID (supersede).
    #[serde(default)]
    pub superseded_by: Option<String>,
    /// 이 엔트리가 흡수된 Layer B 엔트리 ID.
    #[serde(default)]
    pub consolidated_into: Option<String>,

    // === 레거시 grand-fathered ===
    /// Personal Scope 전용 투영값. 비-Personal Scope에서는 `scope.owner_a().to_string()`.
    /// 신규 코드는 `entry.scope`를 직접 사용할 것.
    #[deprecated(note = "Use entry.scope; npc_id is grand-fathered to scope.owner_a() for DB/legacy compat")]
    pub npc_id: String,
}

fn default_personal_scope() -> MemoryScope {
    MemoryScope::Personal {
        npc_id: String::new(),
    }
}
fn default_source() -> MemorySource {
    MemorySource::Experienced
}
fn default_provenance() -> Provenance {
    Provenance::Runtime
}
fn default_layer() -> MemoryLayer {
    MemoryLayer::A
}
fn default_confidence() -> f32 {
    1.0
}

impl MemoryEntry {
    /// Personal Scope용 호환 생성자. 기존 7 필드만으로 신규 필드 기본값을 채워 `MemoryEntry`를 만든다.
    ///
    /// 신규 필드 기본값:
    /// - `scope = Personal { npc_id }`
    /// - `source = Experienced`
    /// - `provenance = Runtime`
    /// - `layer = memory_type.initial_layer()`
    /// - `confidence = 1.0`, `recall_count = 0`, 나머지 None/empty
    /// - `created_seq = event_id` (I-ME-10 근사 — 이벤트 id와 동일 서수 가정)
    pub fn personal(
        id: impl Into<String>,
        npc_id: impl Into<String>,
        content: impl Into<String>,
        emotional_context: Option<(f32, f32, f32)>,
        timestamp_ms: u64,
        event_id: u64,
        memory_type: MemoryType,
    ) -> Self {
        let npc_id = npc_id.into();
        let layer = memory_type.initial_layer();
        #[allow(deprecated)]
        Self {
            id: id.into(),
            created_seq: event_id,
            event_id,
            scope: MemoryScope::Personal {
                npc_id: npc_id.clone(),
            },
            source: MemorySource::Experienced,
            provenance: Provenance::Runtime,
            memory_type,
            layer,
            content: content.into(),
            topic: None,
            emotional_context,
            timestamp_ms,
            last_recalled_at: None,
            recall_count: 0,
            origin_chain: Vec::new(),
            confidence: 1.0,
            acquired_by: None,
            superseded_by: None,
            consolidated_into: None,
            npc_id,
        }
    }

    /// grand-fathered `npc_id` 읽기 전용 접근자 — 비-Personal Scope도 `scope.owner_a()` 반환.
    ///
    /// DB 외래키·레거시 쿼리 호환 목적. 신규 코드는 `entry.scope`를 직접 사용할 것.
    pub fn legacy_npc_id(&self) -> &str {
        self.scope.owner_a()
    }
}

// ---------------------------------------------------------------------------
// MemoryResult — 검색 결과
// ---------------------------------------------------------------------------

/// 기억 검색 결과
#[derive(Debug, Clone)]
pub struct MemoryResult {
    /// 검색된 기억 항목
    pub entry: MemoryEntry,
    /// 관련도 점수 (0.0 ~ 1.0)
    pub relevance_score: f32,
}

// ---------------------------------------------------------------------------
// Tests (Step A — VO semantics)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_scope_relationship_symmetry() {
        let ab = MemoryScope::relationship("a", "b");
        let ba = MemoryScope::relationship("b", "a");
        assert_eq!(ab, ba);
        if let MemoryScope::Relationship { a, b } = &ab {
            assert_eq!(a, "a");
            assert_eq!(b, "b");
        } else {
            panic!("expected Relationship");
        }
    }

    #[test]
    fn memory_scope_partition_key_format() {
        assert_eq!(
            MemoryScope::Personal {
                npc_id: "npc1".into()
            }
            .partition_key(),
            "personal:npc1"
        );
        assert_eq!(
            MemoryScope::relationship("z", "a").partition_key(),
            "relationship:a:z"
        );
        assert_eq!(
            MemoryScope::Faction {
                faction_id: "moorim".into()
            }
            .partition_key(),
            "faction:moorim"
        );
        assert_eq!(
            MemoryScope::Family {
                family_id: "namgoong".into()
            }
            .partition_key(),
            "family:namgoong"
        );
        assert_eq!(
            MemoryScope::World {
                world_id: "jianghu".into()
            }
            .partition_key(),
            "world:jianghu"
        );
    }

    #[test]
    fn memory_scope_owner_projection() {
        let rel = MemoryScope::relationship("z", "a");
        assert_eq!(rel.owner_a(), "a");
        assert_eq!(rel.owner_b(), Some("z"));
        assert_eq!(rel.kind(), "relationship");

        let personal = MemoryScope::Personal {
            npc_id: "npc1".into(),
        };
        assert_eq!(personal.owner_a(), "npc1");
        assert_eq!(personal.owner_b(), None);
    }

    #[test]
    fn memory_source_from_origin_chain() {
        // 체인 길이 0 → Rumor (출처 불명)
        assert_eq!(
            MemorySource::from_origin_chain(0, None),
            MemorySource::Rumor
        );
        // 체인 길이 1 → Heard
        assert_eq!(
            MemorySource::from_origin_chain(1, None),
            MemorySource::Heard
        );
        // 체인 길이 2+ → Rumor
        assert_eq!(
            MemorySource::from_origin_chain(2, None),
            MemorySource::Rumor
        );
        assert_eq!(
            MemorySource::from_origin_chain(5, None),
            MemorySource::Rumor
        );
        // Experienced/Witnessed 힌트는 체인 무시
        assert_eq!(
            MemorySource::from_origin_chain(3, Some(MemorySource::Experienced)),
            MemorySource::Experienced
        );
        assert_eq!(
            MemorySource::from_origin_chain(0, Some(MemorySource::Witnessed)),
            MemorySource::Witnessed
        );
        // Heard/Rumor 힌트는 체인 길이 기반 판정으로 덮어씌움
        assert_eq!(
            MemorySource::from_origin_chain(0, Some(MemorySource::Heard)),
            MemorySource::Rumor
        );
    }

    #[test]
    fn memory_source_priority_ordering() {
        assert!(MemorySource::Experienced.priority() < MemorySource::Witnessed.priority());
        assert!(MemorySource::Witnessed.priority() < MemorySource::Heard.priority());
        assert!(MemorySource::Heard.priority() < MemorySource::Rumor.priority());
    }

    #[test]
    fn provenance_is_canonical() {
        let world = MemoryScope::World {
            world_id: "w".into(),
        };
        let personal = MemoryScope::Personal {
            npc_id: "n".into(),
        };
        assert!(Provenance::Seeded.is_canonical(&world));
        assert!(!Provenance::Seeded.is_canonical(&personal));
        assert!(!Provenance::Runtime.is_canonical(&world));
    }

    #[test]
    fn memory_entry_personal_helper_backward_compat() {
        let e = MemoryEntry::personal(
            "mem-000001",
            "npc1",
            "hello",
            Some((0.1, 0.2, 0.3)),
            1234,
            42,
            MemoryType::DialogueTurn,
        );
        assert_eq!(e.id, "mem-000001");
        #[allow(deprecated)]
        {
            assert_eq!(e.npc_id, "npc1");
        }
        assert_eq!(e.scope, MemoryScope::Personal { npc_id: "npc1".into() });
        assert_eq!(e.source, MemorySource::Experienced);
        assert_eq!(e.provenance, Provenance::Runtime);
        assert_eq!(e.layer, MemoryLayer::A);
        assert_eq!(e.confidence, 1.0);
        assert_eq!(e.recall_count, 0);
        assert_eq!(e.created_seq, 42);
        assert!(e.origin_chain.is_empty());
        assert!(e.topic.is_none());
    }

    #[test]
    fn memory_type_serde_alias_dialogue_to_dialogue_turn() {
        // 구 JSON (`"Dialogue"`) → DialogueTurn 역호환
        let v: MemoryType = serde_json::from_str("\"Dialogue\"").unwrap();
        assert_eq!(v, MemoryType::DialogueTurn);
        // 구 JSON (`"SceneEnd"`) → SceneSummary 역호환
        let v: MemoryType = serde_json::from_str("\"SceneEnd\"").unwrap();
        assert_eq!(v, MemoryType::SceneSummary);
        // 구 JSON (`"Relationship"`) → RelationshipChange 역호환
        let v: MemoryType = serde_json::from_str("\"Relationship\"").unwrap();
        assert_eq!(v, MemoryType::RelationshipChange);

        // from_persisted 호환
        assert_eq!(MemoryType::from_persisted("Dialogue"), Some(MemoryType::DialogueTurn));
        assert_eq!(MemoryType::from_persisted("SceneEnd"), Some(MemoryType::SceneSummary));
        assert_eq!(
            MemoryType::from_persisted("Relationship"),
            Some(MemoryType::RelationshipChange)
        );
        assert_eq!(MemoryType::from_persisted("WorldEvent"), Some(MemoryType::WorldEvent));
        assert_eq!(MemoryType::from_persisted("unknown"), None);
    }

    #[test]
    fn memory_type_initial_layer() {
        assert_eq!(MemoryType::DialogueTurn.initial_layer(), MemoryLayer::A);
        assert_eq!(MemoryType::BeatTransition.initial_layer(), MemoryLayer::A);
        assert_eq!(MemoryType::SceneSummary.initial_layer(), MemoryLayer::B);
        assert_eq!(MemoryType::WorldEvent.initial_layer(), MemoryLayer::A);
    }
}
