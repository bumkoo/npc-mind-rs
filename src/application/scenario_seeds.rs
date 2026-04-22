//! Scenario 시나리오 JSON seeding (Memory Step E3.2).
//!
//! 시나리오 작가가 JSON에 선언하는 `initial_rumors` / `world_knowledge` /
//! `faction_knowledge` / `family_facts` 섹션을 각각 `Rumor` 애그리거트 및
//! `MemoryEntry(provenance=Seeded)`로 변환하는 DTO와 빌더.
//!
//! 본 모듈은 라이브러리 계층에 존재해 테스트·외부 호출자 모두 재사용 가능하다.
//! 실제 store 주입(`MemoryStore.index` / `RumorStore.save`)은 호출자 책임 —
//! Mind Studio의 `load_state` handler가 이 일을 수행한다.
//!
//! 설계 참조: `docs/memory/03-implementation-design.md` §11.2 + §17.3 결정 3.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::application::dto::{RumorOriginInput, RumorReachInput};
use crate::domain::memory::{
    MemoryEntry, MemoryLayer, MemoryScope, MemorySource, MemoryType, Provenance,
};
use crate::domain::rumor::{Rumor, RumorError};

/// 시나리오 JSON에 선언된 기억·소문 시드 묶음.
///
/// 모든 필드 optional — 한 섹션만 써도 됨. 중복 key(Faction/Family의 동일 id에 여러
/// 엔트리)는 Vec에 병합. 정작 store에 어떻게 insert할지는 호출자 정책.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenarioSeeds {
    /// 시나리오 시작 시 존재하는 소문들. `initial_rumors[i].id` 미지정 시 호출자가
    /// 결정적 id를 부여해야 한다 (e.g., `"rumor-seed-{topic-or-idx}"`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initial_rumors: Vec<RumorSeedInput>,

    /// 세계관 Canonical 지식. 각 엔트리는 `scope=World{world_id}`로 주입된다.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub world_knowledge: Vec<WorldKnowledgeSeed>,

    /// 문파 공용 지식. 외부 키 = faction_id. 각 엔트리는 `scope=Faction{faction_id}`.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub faction_knowledge: HashMap<String, Vec<MemoryEntrySeedInput>>,

    /// 가문 공용 사실. 외부 키 = family_id. 각 엔트리는 `scope=Family{family_id}`.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub family_facts: HashMap<String, Vec<MemoryEntrySeedInput>>,
}

impl ScenarioSeeds {
    pub fn is_empty(&self) -> bool {
        self.initial_rumors.is_empty()
            && self.world_knowledge.is_empty()
            && self.faction_knowledge.is_empty()
            && self.family_facts.is_empty()
    }

    /// JSON 문자열에서 파싱. 시나리오 본체 JSON과 동일한 문자열을 받아 seed 섹션만
    /// 추출한다 (다른 필드는 무시).
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// ---------------------------------------------------------------------------
// WorldKnowledgeSeed — world_knowledge 섹션 전용 (world_id 필드 요구)
// ---------------------------------------------------------------------------

/// World scope 엔트리. `scope=World{world_id}`로 지어진다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldKnowledgeSeed {
    /// 세계 식별자 — `scope=World{world_id}`에 사용.
    pub world_id: String,
    #[serde(flatten)]
    pub entry: MemoryEntrySeedInput,
}

impl WorldKnowledgeSeed {
    pub fn into_entry(self, fallback_id: &str) -> MemoryEntry {
        let scope = MemoryScope::World { world_id: self.world_id };
        self.entry.into_entry(scope, fallback_id)
    }
}

// ---------------------------------------------------------------------------
// MemoryEntrySeedInput — scope 무관 공통 필드
// ---------------------------------------------------------------------------

/// Scope를 제외한 공통 MemoryEntry seed 필드. 호출자가 scope를 결정해 `into_entry`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntrySeedInput {
    /// 고유 id. 미지정 시 호출자의 `fallback_id`가 쓰이므로 결정성을 위해 작가가
    /// 직접 지정하는 것이 권장.
    #[serde(default)]
    pub id: Option<String>,
    /// 연결할 Topic (optional). World Canonical은 보통 topic을 갖는다.
    #[serde(default)]
    pub topic: Option<String>,
    /// 엔트리 본문.
    pub content: String,
    /// MemoryType 명시적 지정. 미지정 시 scope로부터 기본값 추론
    /// (World→WorldEvent / Faction→FactionKnowledge / Family→FamilyFact / 그 외 DialogueTurn).
    #[serde(default)]
    pub memory_type: Option<MemoryType>,
    /// 미지정 시 Experienced.
    #[serde(default)]
    pub source: Option<MemorySource>,
    /// 미지정 시 memory_type.initial_layer() (SceneSummary만 B).
    #[serde(default)]
    pub layer: Option<MemoryLayer>,
    /// 미지정 시 1.0.
    #[serde(default)]
    pub confidence: Option<f32>,
    /// Faction/Family Scope의 "누가 획득했나" 메타. 없으면 시나리오 시작 시 해당 소속
    /// 전원이 이미 보유한 것으로 간주.
    #[serde(default)]
    pub acquired_by: Option<String>,
    /// 전달 체인. 미지정 시 빈 Vec (= 직접 경험/목격, 또는 Canonical).
    #[serde(default)]
    pub origin_chain: Vec<String>,
    /// PAD 컨텍스트 (optional).
    #[serde(default)]
    pub emotional_context: Option<(f32, f32, f32)>,
    /// 미지정 시 0 — 시나리오 시작 시점을 나타낸다 (엔진은 절대 timestamp를 비교하지
    /// 않고 created_seq / recall 기반이라 0이 안전).
    #[serde(default)]
    pub timestamp_ms: Option<u64>,
}

impl MemoryEntrySeedInput {
    /// 주어진 scope로 `MemoryEntry`를 빌드. provenance는 강제로 Seeded.
    ///
    /// `fallback_id`는 `self.id`가 없을 때 사용되는 id 일부. 충돌 방지를 위해 호출자가
    /// 유니크한 prefix/index를 조합해 전달해야 한다.
    pub fn into_entry(self, scope: MemoryScope, fallback_id: &str) -> MemoryEntry {
        let id = self.id.unwrap_or_else(|| format!("seed-{fallback_id}"));
        let memory_type = self
            .memory_type
            .unwrap_or_else(|| default_type_for_scope(&scope));
        let layer = self.layer.unwrap_or_else(|| memory_type.initial_layer());
        let source = self.source.unwrap_or(MemorySource::Experienced);
        let confidence = self.confidence.unwrap_or(1.0);
        let timestamp_ms = self.timestamp_ms.unwrap_or(0);
        #[allow(deprecated)]
        let npc_id_legacy = scope.owner_a().to_string();
        #[allow(deprecated)]
        MemoryEntry {
            id,
            created_seq: 0,
            event_id: 0,
            scope,
            source,
            provenance: Provenance::Seeded,
            memory_type,
            layer,
            content: self.content,
            topic: self.topic,
            emotional_context: self.emotional_context,
            timestamp_ms,
            last_recalled_at: None,
            recall_count: 0,
            origin_chain: self.origin_chain,
            confidence,
            acquired_by: self.acquired_by,
            superseded_by: None,
            consolidated_into: None,
            npc_id: npc_id_legacy,
        }
    }
}

fn default_type_for_scope(scope: &MemoryScope) -> MemoryType {
    match scope {
        MemoryScope::World { .. } => MemoryType::WorldEvent,
        MemoryScope::Faction { .. } => MemoryType::FactionKnowledge,
        MemoryScope::Family { .. } => MemoryType::FamilyFact,
        MemoryScope::Relationship { .. } => MemoryType::RelationshipChange,
        MemoryScope::Personal { .. } => MemoryType::DialogueTurn,
    }
}

// ---------------------------------------------------------------------------
// RumorSeedInput — initial_rumors 섹션
// ---------------------------------------------------------------------------

/// `initial_rumors[i]` — `Rumor` 애그리거트 시드.
///
/// 3-tier 콘텐츠 해소 규칙(1차 §3.4.6):
/// - `topic` 있음 + `seed_content` 없음 → 일반 소문 (Canonical에서 해소)
/// - `topic` 있음 + `seed_content` 있음 → 예보된 사실
/// - `topic` 없음 + `seed_content` 있음 → 고아 소문
/// - 둘 다 없음 → 에러 (`OrphanRumorMissingSeed`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RumorSeedInput {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub seed_content: Option<String>,
    #[serde(default)]
    pub reach: RumorReachInput,
    pub origin: RumorOriginInput,
    #[serde(default)]
    pub created_at: Option<u64>,
}

impl RumorSeedInput {
    /// `fallback_id`는 호출자가 index 등으로 유니크하게 구성. `self.id`가 있으면 우선.
    pub fn into_rumor(self, fallback_id: &str) -> Result<Rumor, RumorError> {
        let id = self.id.unwrap_or_else(|| format!("rumor-seed-{fallback_id}"));
        let created_at = self.created_at.unwrap_or(0);
        let reach = (&self.reach).into();
        let origin = (&self.origin).into();
        let rumor = match (self.topic, self.seed_content) {
            (Some(topic), None) => Rumor::new(id, topic, origin, reach, created_at),
            (Some(topic), Some(sc)) => {
                Rumor::with_forecast_content(id, topic, sc, origin, reach, created_at)
            }
            (None, Some(sc)) => Rumor::orphan(id, sc, origin, reach, created_at),
            (None, None) => return Err(RumorError::OrphanRumorMissingSeed),
        };
        rumor.validate()?;
        Ok(rumor)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_world_seed() -> WorldKnowledgeSeed {
        WorldKnowledgeSeed {
            world_id: "jianghu".into(),
            entry: MemoryEntrySeedInput {
                id: Some("world-1".into()),
                topic: Some("sect:leader".into()),
                content: "장문인은 백운이다".into(),
                memory_type: None,
                source: None,
                layer: None,
                confidence: None,
                acquired_by: None,
                origin_chain: vec![],
                emotional_context: None,
                timestamp_ms: None,
            },
        }
    }

    #[test]
    fn world_knowledge_seed_builds_canonical_entry() {
        let seed = sample_world_seed();
        let entry = seed.into_entry("idx-0");
        assert_eq!(entry.id, "world-1");
        assert!(matches!(entry.scope, MemoryScope::World { ref world_id } if world_id == "jianghu"));
        assert_eq!(entry.provenance, Provenance::Seeded);
        // Scope=World + Provenance=Seeded = Canonical (τ=∞).
        assert!(entry.provenance.is_canonical(&entry.scope));
        // 기본 MemoryType = WorldEvent.
        assert_eq!(entry.memory_type, MemoryType::WorldEvent);
        assert_eq!(entry.topic.as_deref(), Some("sect:leader"));
        assert_eq!(entry.confidence, 1.0);
    }

    #[test]
    fn memory_entry_seed_fallback_id_when_missing() {
        let seed = MemoryEntrySeedInput {
            id: None,
            topic: None,
            content: "x".into(),
            memory_type: None,
            source: None,
            layer: None,
            confidence: None,
            acquired_by: None,
            origin_chain: vec![],
            emotional_context: None,
            timestamp_ms: None,
        };
        let entry = seed.into_entry(
            MemoryScope::Faction { faction_id: "sect_yun".into() },
            "sect_yun-0",
        );
        assert_eq!(entry.id, "seed-sect_yun-0");
        assert_eq!(entry.memory_type, MemoryType::FactionKnowledge);
    }

    #[test]
    fn rumor_seed_orphan_requires_content() {
        let seed = RumorSeedInput {
            id: Some("r1".into()),
            topic: None,
            seed_content: None,
            reach: RumorReachInput::default(),
            origin: RumorOriginInput::Seeded,
            created_at: None,
        };
        let err = seed.into_rumor("idx-0").unwrap_err();
        assert!(matches!(err, RumorError::OrphanRumorMissingSeed));
    }

    #[test]
    fn rumor_seed_with_topic_only_builds_canonical_reference() {
        let seed = RumorSeedInput {
            id: Some("r1".into()),
            topic: Some("sect:leader".into()),
            seed_content: None,
            reach: RumorReachInput::default(),
            origin: RumorOriginInput::Seeded,
            created_at: Some(100),
        };
        let rumor = seed.into_rumor("idx-0").unwrap();
        assert_eq!(rumor.id, "r1");
        assert_eq!(rumor.topic.as_deref(), Some("sect:leader"));
        assert!(rumor.seed_content.is_none());
        assert_eq!(rumor.created_at, 100);
    }

    #[test]
    fn rumor_seed_orphan_with_content() {
        let seed = RumorSeedInput {
            id: None,
            topic: None,
            seed_content: Some("강호에 도사가 나타났다".into()),
            reach: RumorReachInput::default(),
            origin: RumorOriginInput::Authored { by: None },
            created_at: None,
        };
        let rumor = seed.into_rumor("idx-7").unwrap();
        assert_eq!(rumor.id, "rumor-seed-idx-7");
        assert!(rumor.topic.is_none());
        assert_eq!(rumor.seed_content.as_deref(), Some("강호에 도사가 나타났다"));
    }

    #[test]
    fn scenario_seeds_parse_empty_sections() {
        let json = r#"{
            "initial_rumors": [],
            "world_knowledge": [],
            "faction_knowledge": {},
            "family_facts": {}
        }"#;
        let seeds = ScenarioSeeds::from_json(json).unwrap();
        assert!(seeds.is_empty());
    }

    #[test]
    fn scenario_seeds_parse_mixed() {
        let json = r#"{
            "initial_rumors": [
                { "id": "r1", "topic": "sect:leader", "seed_content": null,
                  "reach": { "regions": [], "factions": [], "npc_ids": [], "min_significance": 0.0 },
                  "origin": { "kind": "seeded" } }
            ],
            "world_knowledge": [
                { "world_id": "jianghu", "topic": "sect:leader", "content": "장문인은 백운이다" }
            ],
            "faction_knowledge": {
                "sect_yun": [
                    { "content": "문파의 비전은 천뢰검법이다" }
                ]
            }
        }"#;
        let seeds = ScenarioSeeds::from_json(json).unwrap();
        assert_eq!(seeds.initial_rumors.len(), 1);
        assert_eq!(seeds.world_knowledge.len(), 1);
        assert_eq!(seeds.faction_knowledge.get("sect_yun").unwrap().len(), 1);
        assert!(seeds.family_facts.is_empty());
    }

    #[test]
    fn empty_scenario_seeds_serializes_without_fields() {
        let seeds = ScenarioSeeds::default();
        let out = serde_json::to_string(&seeds).unwrap();
        // 빈 섹션은 모두 skip_serializing_if로 제외 → `{}`.
        assert_eq!(out, "{}");
    }

    #[test]
    fn scenario_seeds_parse_ignores_unrelated_fields() {
        // 시나리오 본체 JSON (npcs·relationships 등 포함)에서도 seed만 추출해야 함.
        let json = r#"{
            "npcs": {},
            "relationships": {},
            "world_knowledge": [
                { "world_id": "w1", "content": "hi" }
            ]
        }"#;
        let seeds = ScenarioSeeds::from_json(json).unwrap();
        assert_eq!(seeds.world_knowledge.len(), 1);
        assert_eq!(seeds.world_knowledge[0].world_id, "w1");
    }
}
