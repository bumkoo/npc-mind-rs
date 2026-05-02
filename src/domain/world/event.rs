//! Event 애그리거트 — Phase 5a Vertical Slice. **두 번째 인스턴스 도메인**.
//!
//! **장르 중립 원칙**: 이 모듈은 wuxia/판타지/SF 어떤 어휘도 모른다. `kind`는
//! free-form `String`이며, 장르가 채운다 (`genres/wuxia/forms/event.toml` —
//! `betrayal`/`war`/`founding`/`disaster`/`ritual`/`discovery`).
//!
//! Phase 5a 외래키:
//! - `Event.participants.people` ↔ `Person.id` (활성 — world-load hard-fail)
//! - `Event.participants.groups` ↔ `Group.id` (활성)
//! - `Event.participants.places` ↔ `Place.id` (활성)
//! - `Event.related_events` ↔ `Event.id` (자체 도메인, 활성)
//! - `Event.era_id` (있으면) — 텍스트만 보존, Phase 5b Era 도메인 진입 시 외래키 활성

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Event 식별자 — `event-{slug}` 형식. slug는 ASCII 소문자·숫자·하이픈.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(pub String);

impl EventId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for EventId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for EventId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// 이벤트 카테고리 — 사실성·시제 분류.
///
/// - `Historical` — 이미 일어남, 캐논. 시드 자료 28 사건 대부분.
/// - `Scheduled` — Phase 6+ gameplay 다리에서 발생할 예정 사건. Phase 5a 미사용.
/// - `Legendary` — 진위 불확실 (전설·구전).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EventCategory {
    #[default]
    Historical,
    Scheduled,
    Legendary,
}

impl EventCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            EventCategory::Historical => "historical",
            EventCategory::Scheduled => "scheduled",
            EventCategory::Legendary => "legendary",
        }
    }

    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s {
            "historical" => Some(EventCategory::Historical),
            "scheduled" => Some(EventCategory::Scheduled),
            "legendary" => Some(EventCategory::Legendary),
            _ => None,
        }
    }
}

/// Event 시간 메타. Phase 5a엔 자유 텍스트 + relative 정수 캐시.
///
/// `year_relative`는 270년차 기준 절대 연도 (예: 10년 전 = -10). Phase 5b
/// Era 결합 시 정형 시간으로 승급되며 era_id와 함께 절대 연도 계산은 Era
/// 도메인이 책임진다.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct EventTemporal {
    /// 자유 텍스트 — "10년 전 (260년차)", "원년", "수년 후" 등.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    /// 270년차 기준 절대 연도. 정렬·필터에 사용 (Phase 5b Era 정형 전 임시).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year_relative: Option<i32>,
    /// 사건 지속 — "사흘 밤", "수년" 등 자유 텍스트.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    /// 자유 메모 (Phase 5b 마이그레이션 의도 등).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// 사건 관여자 — 세 카테고리 외래키 셋 모음.
///
/// Phase 5a에서 모두 활성. `world-load`가 해당 도메인 id 집합과 대조해
/// 결손 시 hard-fail.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ParticipantsRefs {
    /// PersonId 텍스트 — Phase 2 외래키 검증 활성.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub people: Vec<String>,
    /// GroupId 텍스트 — Phase 1 외래키 검증 활성.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<String>,
    /// PlaceId 텍스트 — Phase 3 외래키 검증 활성.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub places: Vec<String>,
}

impl ParticipantsRefs {
    pub fn is_empty(&self) -> bool {
        self.people.is_empty() && self.groups.is_empty() && self.places.is_empty()
    }
}

/// Event 애그리거트 — 두 번째 인스턴스 도메인.
///
/// 핵심 책임:
/// - 정체성: id/name/aliases/kind/category + summary/tags
/// - 시제 메타: temporal (year 자유 텍스트 + year_relative 정수)
/// - **합성 핵심**: participants (인물·그룹·장소 외래키)
/// - 자체 도메인 외래키: related_events (인과·후속 사건)
/// - 시대 결속: era_id (Phase 5b 활성 — Phase 5a엔 텍스트만)
/// - 자유 본문: body_sections (`## 개요` · `## 발단` · `## 전개` · `## 결과` 등)
/// - 장르 확장: extras (trigger·outcome·player_relevance 등)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub id: EventId,
    /// 장르가 채움 (Phase 5a wuxia: `betrayal`/`war`/`founding`/`disaster`/`ritual`/`discovery`).
    pub kind: String,
    #[serde(default)]
    pub category: EventCategory,
    pub name: String,
    /// 별호·옛 이름. 예: `["붉은 밤", "10년 전 변란"]`. FTS5 검색 대상에 포함.
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 장르 자유 확장. wuxia 예: `trigger`(발단)·`outcome`(결과)·`player_relevance`·`game_role`.
    #[serde(default)]
    pub extras: Map<String, Value>,
    #[serde(default)]
    pub temporal: EventTemporal,
    /// **텍스트만** — Phase 5b Era 도메인 도입 시 외래키 활성.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub era_id: Option<String>,
    /// **핵심** — 본 사건에 관여한 인물·그룹·장소. world-load 시 hard-fail.
    #[serde(default)]
    pub participants: ParticipantsRefs,
    /// H2 섹션 본문. `BTreeMap`이라 알파벳 정렬 순서로 보존되며, 작성 순서는
    /// 보존되지 않는다 (Group/Person/Place/Atlas 동일 정책).
    #[serde(default)]
    pub body_sections: BTreeMap<String, String>,
    /// 같은 도메인 내 외래키 — 인과·후속 사건 (Phase 5a 활성. cycle 검증은
    /// 비활성 — 대부분 사건은 비순환이라 단순 결손만 검증).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_events: Vec<EventId>,
    /// 마크다운 SoT 경로 (절대 또는 프로젝트 root 기준 상대).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}

impl Event {
    /// 최소 생성자. 테스트·도구용.
    pub fn new(
        id: impl Into<EventId>,
        kind: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            category: EventCategory::Historical,
            name: name.into(),
            aliases: Vec::new(),
            summary: String::new(),
            tags: Vec::new(),
            extras: Map::new(),
            temporal: EventTemporal::default(),
            era_id: None,
            participants: ParticipantsRefs::default(),
            body_sections: BTreeMap::new(),
            related_events: Vec::new(),
            source_path: None,
        }
    }

    /// 270년차 기준 절대 연도. Phase 5b Era 결합 전 임시 정렬 키.
    pub fn year_relative(&self) -> Option<i32> {
        self.temporal.year_relative
    }

    /// 본 사건이 특정 인물의 관여를 포함하는가.
    pub fn involves_person(&self, person_id: &str) -> bool {
        self.participants
            .people
            .iter()
            .any(|p| p == person_id)
    }

    /// 본 사건이 특정 그룹의 관여를 포함하는가.
    pub fn involves_group(&self, group_id: &str) -> bool {
        self.participants
            .groups
            .iter()
            .any(|g| g == group_id)
    }

    /// 본 사건이 특정 장소의 관여를 포함하는가.
    pub fn involves_place(&self, place_id: &str) -> bool {
        self.participants
            .places
            .iter()
            .any(|p| p == place_id)
    }
}

/// 리스트 필터 — `WorldRepository::list_events`에 전달.
///
/// year_relative_min/max는 inclusive 범위 (둘 다 None이면 전체).
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    pub category: Option<EventCategory>,
    pub kind: Option<String>,
    /// 특정 인물 관여 — Phase 5a 활성.
    pub participants_person: Option<String>,
    /// 특정 그룹 관여 — Phase 5a 활성.
    pub participants_group: Option<String>,
    /// 특정 장소 관여 — Phase 5a 활성.
    pub participants_place: Option<String>,
    /// year_relative >= min (inclusive). 예: -30 = 30년 전부터.
    pub year_relative_min: Option<i32>,
    /// year_relative <= max (inclusive). 예: 0 = 현재까지.
    pub year_relative_max: Option<i32>,
    /// `tags` 토큰 매칭. `wuxia`/`historical` 등.
    pub genre_tag: Option<String>,
}

// ---------------------------------------------------------------------------
// 단위 테스트
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_new_sets_defaults() {
        let e = Event::new("event-x", "betrayal", "X 사건");
        assert_eq!(e.id.as_str(), "event-x");
        assert_eq!(e.kind, "betrayal");
        assert_eq!(e.name, "X 사건");
        assert_eq!(e.category, EventCategory::Historical);
        assert!(e.aliases.is_empty());
        assert!(e.tags.is_empty());
        assert!(e.related_events.is_empty());
        assert!(e.participants.is_empty());
        assert!(e.temporal.year.is_none());
        assert!(e.era_id.is_none());
    }

    #[test]
    fn category_default_is_historical() {
        let cat: EventCategory = Default::default();
        assert_eq!(cat, EventCategory::Historical);
    }

    #[test]
    fn category_serde_lowercase() {
        let json = serde_json::to_string(&EventCategory::Historical).unwrap();
        assert_eq!(json, "\"historical\"");
        let back: EventCategory = serde_json::from_str("\"scheduled\"").unwrap();
        assert_eq!(back, EventCategory::Scheduled);
    }

    #[test]
    fn category_from_str_loose() {
        assert_eq!(
            EventCategory::from_str_loose("historical"),
            Some(EventCategory::Historical)
        );
        assert_eq!(
            EventCategory::from_str_loose("legendary"),
            Some(EventCategory::Legendary)
        );
        assert!(EventCategory::from_str_loose("invalid").is_none());
    }

    #[test]
    fn event_temporal_serde_skip_when_none() {
        let t = EventTemporal::default();
        let json = serde_json::to_string(&t).unwrap();
        // 모든 필드가 None이면 빈 객체.
        assert_eq!(json, "{}");
    }

    #[test]
    fn event_temporal_year_relative_serializes_negative() {
        let t = EventTemporal {
            year: Some("10년 전".into()),
            year_relative: Some(-10),
            duration: Some("사흘 밤".into()),
            notes: None,
        };
        let json = serde_json::to_string(&t).unwrap();
        // 정수 캐시는 "year_relative":-10 형태로 — 정렬·필터에 직접 사용 가능.
        assert!(json.contains("\"year_relative\":-10"));
        assert!(json.contains("\"year\":\"10년 전\""));
        assert!(json.contains("\"duration\":\"사흘 밤\""));
        assert!(!json.contains("\"notes\""), "notes None은 skip되어야 함");

        let back: EventTemporal = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn participants_refs_default_is_empty() {
        let p = ParticipantsRefs::default();
        assert!(p.is_empty());
        let json = serde_json::to_string(&p).unwrap();
        // 모두 빈 Vec이면 skip → 빈 객체.
        assert_eq!(json, "{}");
    }

    #[test]
    fn participants_refs_skip_empty_arrays() {
        let p = ParticipantsRefs {
            people: vec!["npc-01".into()],
            groups: vec![],
            places: vec!["place-daejin".into()],
        };
        let json = serde_json::to_string(&p).unwrap();
        // 빈 groups는 skip되지만 people·places는 보존.
        assert!(json.contains("\"people\":[\"npc-01\"]"));
        assert!(json.contains("\"places\":[\"place-daejin\"]"));
        assert!(
            !json.contains("\"groups\""),
            "빈 groups는 skip되어야 함"
        );
    }

    #[test]
    fn participants_refs_missing_field_default_is_empty_vec() {
        // YAML/JSON에서 필드 누락 시 빈 Vec.
        let json = "{\"people\":[\"npc-02\"]}";
        let p: ParticipantsRefs = serde_json::from_str(json).unwrap();
        assert_eq!(p.people, vec!["npc-02".to_string()]);
        assert!(p.groups.is_empty());
        assert!(p.places.is_empty());
    }

    #[test]
    fn full_serde_roundtrip_preserves_all_fields() {
        let mut e = Event::new("event-bloody-night", "betrayal", "붉은 밤의 변");
        e.aliases = vec!["붉은 밤".into(), "10년 전 변란".into()];
        e.category = EventCategory::Historical;
        e.summary = "10년 전 통일제국 대진의 영토 와해를 가져온 결정적 사건.".into();
        e.tags = vec!["wuxia".into(), "event".into(), "historical".into()];
        e.temporal = EventTemporal {
            year: Some("10년 전 (260년차)".into()),
            year_relative: Some(-10),
            duration: Some("사흘 밤".into()),
            notes: None,
        };
        e.era_id = Some("era-fall-of-empire".into());
        e.participants = ParticipantsRefs {
            people: vec!["npc-02".into(), "npc-07".into(), "npc-01".into()],
            groups: vec!["group-daejin-court".into(), "group-shipsangsi".into()],
            places: vec!["place-daejin".into(), "place-namgung".into()],
        };
        e.related_events = vec![EventId::new("event-hwasan-fall")];
        e.body_sections.insert(
            "개요".into(),
            "산문 1-2 단락 — 사건 핵심 묘사.".into(),
        );
        e.body_sections
            .insert("결과".into(), "영토 와해 · 6국 독립 시작.".into());
        e.extras.insert(
            "trigger".into(),
            Value::String("천순제 즉위 직후 권력 공백".into()),
        );
        e.extras.insert(
            "player_relevance".into(),
            Value::String("★★★★★".into()),
        );

        let json = serde_json::to_string(&e).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn era_id_skipped_when_none() {
        let e = Event::new("event-x", "war", "X");
        let json = serde_json::to_string(&e).unwrap();
        assert!(
            !json.contains("era_id"),
            "era_id가 None이면 skip되어야 함"
        );
    }

    #[test]
    fn era_id_text_preserved_for_phase5b() {
        // Phase 5a엔 era_id 텍스트만 보존 (검증 비활성). Phase 5b에서 Era 도메인
        // 외래키로 활성화 예정.
        let mut e = Event::new("event-x", "war", "X");
        e.era_id = Some("era-fall-of-empire".into());
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"era_id\":\"era-fall-of-empire\""));
        let back: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(back.era_id.as_deref(), Some("era-fall-of-empire"));
    }

    #[test]
    fn related_events_skip_empty() {
        let e = Event::new("event-x", "war", "X");
        let json = serde_json::to_string(&e).unwrap();
        assert!(!json.contains("related_events"), "빈 related_events는 skip");
    }

    #[test]
    fn involvement_helpers() {
        let mut e = Event::new("event-x", "war", "X");
        e.participants.people = vec!["npc-02".into(), "npc-07".into()];
        e.participants.groups = vec!["group-daejin-court".into()];
        e.participants.places = vec!["place-daejin".into()];

        assert!(e.involves_person("npc-02"));
        assert!(e.involves_person("npc-07"));
        assert!(!e.involves_person("npc-99"));
        assert!(e.involves_group("group-daejin-court"));
        assert!(!e.involves_group("group-other"));
        assert!(e.involves_place("place-daejin"));
        assert!(!e.involves_place("place-namgung"));
    }

    #[test]
    fn year_relative_accessor() {
        let mut e = Event::new("event-x", "war", "X");
        assert!(e.year_relative().is_none());
        e.temporal.year_relative = Some(-10);
        assert_eq!(e.year_relative(), Some(-10));
    }

    #[test]
    fn event_category_legendary_serde() {
        let cat = EventCategory::Legendary;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"legendary\"");
        let back: EventCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cat);
    }
}
