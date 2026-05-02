//! Timeline 애그리거트 — Phase 5b 체크포인트 2 Vertical Slice. **두 번째 관계 도메인**.
//!
//! Atlas와 결이 같은 도메인+뷰 이중성. references는 Era 외래키 시퀀스이며, view 메서드가
//! 두 단계 합성을 수행한다 (timeline → era → events).
//!
//! **Q2 결정 (View trait 보류)**: View trait 일반화는 Phase 6+ 세 번째 관계 도메인 등장
//! 시 재검토. 현재는 Atlas + Timeline 각자 view 메서드 자체 구현.
//!
//! Phase 5b 외래키:
//! - `Timeline.references` ↔ `Era.id` (활성 — world-load hard-fail)
//! - `Timeline.references` 카테고리 내 중복 금지 (timeline_era_refs composite PK 보호)

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use super::era::{Era, EraId};
use super::event::{Event, EventId};
use super::WorldError;
use crate::worldbuilding::WorldRepository;

/// Timeline 식별자 — `timeline-{slug}` 형식.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TimelineId(pub String);

impl TimelineId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TimelineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for TimelineId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for TimelineId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Timeline 애그리거트 — 두 번째 관계 도메인.
///
/// 핵심 책임:
/// - 정체성: id/name/aliases/kind + summary/tags
/// - **합성 핵심**: references (어느 Era들이 본 timeline에 등장)
/// - 자유 본문: body_sections
/// - 장르 확장: extras
///
/// **두 단계 합성** (timeline → era → events):
/// - `eras_in(repo)` — references 직접 합성
/// - `events_in(repo)` — 각 era.key_events 평면화 (era 순서 + 같은 era 내 작성 순서)
/// - `events_during(era_id, repo)` — 특정 era의 key_events만
/// - `causal_chain(seed, repo)` — events_in 안에서 related_events BFS
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Timeline {
    pub id: TimelineId,
    /// 장르가 채움 (Phase 5b wuxia: `history`·`biographical` 등).
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 장르 자유 확장.
    #[serde(default)]
    pub extras: Map<String, Value>,
    /// **핵심** — 본 timeline에 등장하는 Era들. world-load 시 hard-fail (composite PK).
    /// 작성 순서가 곧 view에서 보이는 순서이며, 보통 시간순(과거 → 현재).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<EraId>,
    #[serde(default)]
    pub body_sections: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}

impl Timeline {
    pub fn new(
        id: impl Into<TimelineId>,
        kind: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            name: name.into(),
            aliases: Vec::new(),
            summary: String::new(),
            tags: Vec::new(),
            extras: Map::new(),
            references: Vec::new(),
            body_sections: BTreeMap::new(),
            source_path: None,
        }
    }

    // -----------------------------------------------------------------------
    // 도메인+뷰 이중성 — view 메서드 4종.
    //
    // Atlas의 places_in 패턴과 결이 같지만 timeline은 두 단계 합성:
    // timeline → references(eras) → era.key_events(events). View trait 일반화는 Q2 보류.
    // -----------------------------------------------------------------------

    /// references → 직접 Era 합성 (작성 순서 보존).
    /// 결손된 ID는 silent skip (world-load hard-fail로 결손 0건 보장).
    pub fn eras_in<R: WorldRepository + ?Sized>(
        &self,
        repo: &R,
    ) -> Result<Vec<Era>, WorldError> {
        repo.get_eras_batch(&self.references)
    }

    /// 각 era.key_events를 평면화 (era 순서 + 같은 era 내 작성 순서).
    /// 중복 EventId는 첫 등장만 유지 — 두 era에 동일 사건이 들어 있어도 events_in은 1건.
    pub fn events_in<R: WorldRepository + ?Sized>(
        &self,
        repo: &R,
    ) -> Result<Vec<Event>, WorldError> {
        let eras = self.eras_in(repo)?;
        let mut event_ids: Vec<EventId> = Vec::new();
        let mut seen: HashSet<EventId> = HashSet::new();
        for era in &eras {
            for ke in &era.key_events {
                if seen.insert(ke.clone()) {
                    event_ids.push(ke.clone());
                }
            }
        }
        repo.get_events_batch(&event_ids)
    }

    /// 특정 era에 속하는 본 timeline의 사건들. era.key_events 직접 사용
    /// (event.year_relative 기반 contains_year 체크는 era_id 매핑이 정합이라 불필요 —
    /// era.key_events가 권위).
    ///
    /// `era_id`가 본 timeline의 references에 없으면 빈 Vec (timeline 경계 밖이라 사일런트).
    pub fn events_during<R: WorldRepository + ?Sized>(
        &self,
        era_id: &EraId,
        repo: &R,
    ) -> Result<Vec<Event>, WorldError> {
        if !self.references.iter().any(|r| r == era_id) {
            return Ok(Vec::new());
        }
        let Some(era) = repo.get_era(era_id)? else {
            return Ok(Vec::new());
        };
        repo.get_events_batch(&era.key_events)
    }

    /// 특정 사건의 인과 사슬 — events_in 안에서 related_events 따라 BFS.
    /// timeline 경계 밖의 related_events는 무시 (timeline-국한 transitive closure).
    /// 결과는 BFS 순서이며 seed 자신은 첫 항목으로 포함.
    /// seed가 events_in 안에 없으면 빈 Vec.
    ///
    /// **F1+F2 (review 후 정리)**: events_in으로 한 번에 모든 사건을 로드한 뒤
    /// HashMap으로 사전 인덱싱 — BFS는 O(N) lookup, 결과 합성은 in-memory에서
    /// 직접 (이전 구현은 BFS 후 `get_events_batch(&order)` 재호출로 2× round-trip).
    pub fn causal_chain<R: WorldRepository + ?Sized>(
        &self,
        seed: &EventId,
        repo: &R,
    ) -> Result<Vec<Event>, WorldError> {
        let in_scope: Vec<Event> = self.events_in(repo)?;
        // F2: O(1) lookup용 HashMap (id → Event). BFS의 `iter().find` O(N²) 제거.
        let by_id: HashMap<EventId, Event> = in_scope
            .into_iter()
            .map(|e| (e.id.clone(), e))
            .collect();
        if !by_id.contains_key(seed) {
            return Ok(Vec::new());
        }
        // BFS — seed부터 related_events traversal, timeline 경계 밖은 무시.
        let mut order: Vec<EventId> = Vec::new();
        let mut visited: HashSet<EventId> = HashSet::new();
        let mut queue: VecDeque<EventId> = VecDeque::new();
        queue.push_back(seed.clone());
        visited.insert(seed.clone());
        while let Some(cur) = queue.pop_front() {
            // F2: O(1) lookup으로 related_events traversal.
            if let Some(ev) = by_id.get(&cur) {
                order.push(cur.clone());
                for rel in &ev.related_events {
                    if by_id.contains_key(rel) && visited.insert(rel.clone()) {
                        queue.push_back(rel.clone());
                    }
                }
            }
        }
        // F1: order에 해당하는 Event를 by_id에서 직접 꺼내 합성 — 추가 round-trip X.
        let mut by_id = by_id;
        Ok(order
            .into_iter()
            .filter_map(|id| by_id.remove(&id))
            .collect())
    }
}

/// 리스트 필터 — `WorldRepository::list_timelines`에 전달.
#[derive(Debug, Clone, Default)]
pub struct TimelineFilter {
    pub kind: Option<String>,
    /// 특정 era를 references에 포함하는 timeline만.
    pub references_era: Option<EraId>,
    /// `tags` 토큰 매칭.
    pub genre_tag: Option<String>,
}

// ---------------------------------------------------------------------------
// 단위 테스트
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::world::{
        atlas::{Atlas, AtlasFilter, AtlasId},
        era::EraFilter,
        event::{EventFilter, ParticipantsRefs},
        group::{Group, GroupFilter, GroupId},
        person::{Person, PersonFilter, PersonId},
        place::{Place, PlaceFilter, PlaceId},
    };
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// 최소 in-memory repo — view 메서드 e2e 검증용.
    struct MiniRepo {
        eras: Mutex<HashMap<EraId, Era>>,
        events: Mutex<HashMap<EventId, Event>>,
    }

    impl MiniRepo {
        fn new(eras: Vec<Era>, events: Vec<Event>) -> Self {
            Self {
                eras: Mutex::new(eras.into_iter().map(|e| (e.id.clone(), e)).collect()),
                events: Mutex::new(events.into_iter().map(|e| (e.id.clone(), e)).collect()),
            }
        }
    }

    impl WorldRepository for MiniRepo {
        // 다른 도메인은 사용하지 않음.
        fn list_groups(&self, _: GroupFilter) -> Result<Vec<Group>, WorldError> {
            unimplemented!()
        }
        fn get_group(&self, _: &GroupId) -> Result<Option<Group>, WorldError> {
            unimplemented!()
        }
        fn search_groups(&self, _: &str, _: u32) -> Result<Vec<Group>, WorldError> {
            unimplemented!()
        }
        fn upsert_group(&self, _: &str, _: &Group) -> Result<(), WorldError> {
            unimplemented!()
        }
        fn count_groups(&self, _: Option<&str>) -> Result<u64, WorldError> {
            unimplemented!()
        }
        fn list_persons(&self, _: PersonFilter) -> Result<Vec<Person>, WorldError> {
            unimplemented!()
        }
        fn get_person(&self, _: &PersonId) -> Result<Option<Person>, WorldError> {
            unimplemented!()
        }
        fn search_persons(&self, _: &str, _: u32) -> Result<Vec<Person>, WorldError> {
            unimplemented!()
        }
        fn upsert_person(&self, _: &str, _: &Person) -> Result<(), WorldError> {
            unimplemented!()
        }
        fn count_persons(&self, _: Option<&str>) -> Result<u64, WorldError> {
            unimplemented!()
        }
        fn list_places(&self, _: PlaceFilter) -> Result<Vec<Place>, WorldError> {
            unimplemented!()
        }
        fn get_place(&self, _: &PlaceId) -> Result<Option<Place>, WorldError> {
            unimplemented!()
        }
        fn search_places(&self, _: &str, _: u32) -> Result<Vec<Place>, WorldError> {
            unimplemented!()
        }
        fn upsert_place(&self, _: &str, _: &Place) -> Result<(), WorldError> {
            unimplemented!()
        }
        fn count_places(&self, _: Option<&str>) -> Result<u64, WorldError> {
            unimplemented!()
        }
        fn list_atlases(&self, _: AtlasFilter) -> Result<Vec<Atlas>, WorldError> {
            unimplemented!()
        }
        fn get_atlas(&self, _: &AtlasId) -> Result<Option<Atlas>, WorldError> {
            unimplemented!()
        }
        fn search_atlases(&self, _: &str, _: u32) -> Result<Vec<Atlas>, WorldError> {
            unimplemented!()
        }
        fn upsert_atlas(&self, _: &str, _: &Atlas) -> Result<(), WorldError> {
            unimplemented!()
        }
        fn count_atlases(&self, _: Option<&str>) -> Result<u64, WorldError> {
            unimplemented!()
        }
        fn list_events(&self, _: EventFilter) -> Result<Vec<Event>, WorldError> {
            unimplemented!()
        }
        fn get_event(&self, id: &EventId) -> Result<Option<Event>, WorldError> {
            Ok(self.events.lock().unwrap().get(id).cloned())
        }
        fn search_events(&self, _: &str, _: u32) -> Result<Vec<Event>, WorldError> {
            unimplemented!()
        }
        fn upsert_event(&self, _: &str, _: &Event) -> Result<(), WorldError> {
            unimplemented!()
        }
        fn count_events(&self, _: Option<&str>) -> Result<u64, WorldError> {
            unimplemented!()
        }
        fn list_eras(&self, _: EraFilter) -> Result<Vec<Era>, WorldError> {
            unimplemented!()
        }
        fn get_era(&self, id: &EraId) -> Result<Option<Era>, WorldError> {
            Ok(self.eras.lock().unwrap().get(id).cloned())
        }
        fn search_eras(&self, _: &str, _: u32) -> Result<Vec<Era>, WorldError> {
            unimplemented!()
        }
        fn upsert_era(&self, _: &str, _: &Era) -> Result<(), WorldError> {
            unimplemented!()
        }
        fn count_eras(&self, _: Option<&str>) -> Result<u64, WorldError> {
            unimplemented!()
        }
        fn list_timelines(&self, _: TimelineFilter) -> Result<Vec<Timeline>, WorldError> {
            unimplemented!()
        }
        fn get_timeline(&self, _: &TimelineId) -> Result<Option<Timeline>, WorldError> {
            unimplemented!()
        }
        fn search_timelines(&self, _: &str, _: u32) -> Result<Vec<Timeline>, WorldError> {
            unimplemented!()
        }
        fn upsert_timeline(&self, _: &str, _: &Timeline) -> Result<(), WorldError> {
            unimplemented!()
        }
        fn count_timelines(&self, _: Option<&str>) -> Result<u64, WorldError> {
            unimplemented!()
        }
    }

    fn era_with_events(id: &str, kind: &str, key_events: &[&str]) -> Era {
        let mut e = Era::new(id, kind, id);
        e.key_events = key_events.iter().map(|s| EventId::new(*s)).collect();
        e
    }

    fn event_with_related(id: &str, related: &[&str]) -> Event {
        let mut ev = Event::new(id, "war", id);
        ev.related_events = related.iter().map(|s| EventId::new(*s)).collect();
        ev.participants = ParticipantsRefs::default();
        ev
    }

    fn sample_timeline(refs: &[&str]) -> Timeline {
        let mut t = Timeline::new("timeline-test", "history", "Test");
        t.references = refs.iter().map(|s| EraId::new(*s)).collect();
        t
    }

    #[test]
    fn timeline_new_sets_defaults() {
        let t = Timeline::new("timeline-x", "history", "X");
        assert_eq!(t.id.as_str(), "timeline-x");
        assert_eq!(t.kind, "history");
        assert_eq!(t.name, "X");
        assert!(t.references.is_empty());
        assert!(t.body_sections.is_empty());
    }

    #[test]
    fn timeline_full_serde_roundtrip() {
        let mut t = Timeline::new("timeline-jungwon-history", "history", "270년사");
        t.aliases = vec!["중원사".into(), "main-history".into()];
        t.summary = "원년~현재".into();
        t.tags = vec!["wuxia".into(), "timeline".into()];
        t.references = vec![
            EraId::new("era-founding"),
            EraId::new("era-fall-of-empire"),
        ];
        t.body_sections.insert("개요".into(), "본문".into());
        t.extras
            .insert("game_role".into(), Value::String("메인 시간선".into()));

        let json = serde_json::to_string(&t).unwrap();
        let back: Timeline = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn timeline_id_serde_transparent() {
        let id = TimelineId::new("timeline-x");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"timeline-x\"");
        let back: TimelineId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn eras_in_returns_in_reference_order() {
        let repo = MiniRepo::new(
            vec![
                era_with_events("era-a", "founding", &[]),
                era_with_events("era-b", "fall", &[]),
                era_with_events("era-c", "decline", &[]),
            ],
            vec![],
        );
        // references는 c, a, b 순서 — eras_in도 동일 순서.
        let timeline = sample_timeline(&["era-c", "era-a", "era-b"]);
        let got = timeline.eras_in(&repo).unwrap();
        let ids: Vec<&str> = got.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["era-c", "era-a", "era-b"]);
    }

    #[test]
    fn eras_in_skips_missing_silently() {
        // world-load가 hard-fail로 결손 0건 보장 → view는 silent skip.
        let repo = MiniRepo::new(
            vec![era_with_events("era-a", "founding", &[])],
            vec![],
        );
        let timeline = sample_timeline(&["era-a", "era-missing"]);
        let got = timeline.eras_in(&repo).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id.as_str(), "era-a");
    }

    #[test]
    fn events_in_flattens_era_key_events_in_era_order() {
        // era 순서 + 같은 era 내 작성 순서.
        let repo = MiniRepo::new(
            vec![
                era_with_events("era-a", "founding", &["event-1"]),
                era_with_events("era-b", "fall", &["event-2", "event-3"]),
            ],
            vec![
                event_with_related("event-1", &[]),
                event_with_related("event-2", &[]),
                event_with_related("event-3", &[]),
            ],
        );
        let timeline = sample_timeline(&["era-a", "era-b"]);
        let got = timeline.events_in(&repo).unwrap();
        let ids: Vec<&str> = got.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["event-1", "event-2", "event-3"]);
    }

    #[test]
    fn events_in_dedupes_event_appearing_in_multiple_eras() {
        // 같은 사건이 두 era의 key_events에 있어도 events_in은 1건만 (첫 등장 유지).
        let repo = MiniRepo::new(
            vec![
                era_with_events("era-a", "founding", &["event-shared"]),
                era_with_events("era-b", "fall", &["event-shared", "event-other"]),
            ],
            vec![
                event_with_related("event-shared", &[]),
                event_with_related("event-other", &[]),
            ],
        );
        let timeline = sample_timeline(&["era-a", "era-b"]);
        let got = timeline.events_in(&repo).unwrap();
        let ids: Vec<&str> = got.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["event-shared", "event-other"]);
    }

    #[test]
    fn events_during_returns_only_specified_era_events() {
        let repo = MiniRepo::new(
            vec![
                era_with_events("era-a", "founding", &["event-1"]),
                era_with_events("era-b", "fall", &["event-2", "event-3"]),
            ],
            vec![
                event_with_related("event-1", &[]),
                event_with_related("event-2", &[]),
                event_with_related("event-3", &[]),
            ],
        );
        let timeline = sample_timeline(&["era-a", "era-b"]);
        let got = timeline.events_during(&EraId::new("era-b"), &repo).unwrap();
        let ids: Vec<&str> = got.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["event-2", "event-3"]);
    }

    #[test]
    fn events_during_returns_empty_for_era_outside_timeline() {
        // timeline 경계 밖의 era는 빈 Vec (사일런트).
        let repo = MiniRepo::new(
            vec![
                era_with_events("era-a", "founding", &["event-1"]),
                era_with_events("era-outside", "decline", &["event-x"]),
            ],
            vec![event_with_related("event-1", &[])],
        );
        let timeline = sample_timeline(&["era-a"]); // era-outside는 references X
        let got = timeline
            .events_during(&EraId::new("era-outside"), &repo)
            .unwrap();
        assert!(got.is_empty());
    }

    #[test]
    fn causal_chain_bfs_within_timeline_boundary() {
        // event-A → event-B, event-B → event-C, event-C → event-D
        // timeline은 era-x에 [A, B, C, D]를 포함.
        // causal_chain(A) → [A, B, C, D] BFS 순서.
        let repo = MiniRepo::new(
            vec![era_with_events(
                "era-x",
                "fall",
                &["event-A", "event-B", "event-C", "event-D"],
            )],
            vec![
                event_with_related("event-A", &["event-B"]),
                event_with_related("event-B", &["event-C"]),
                event_with_related("event-C", &["event-D"]),
                event_with_related("event-D", &[]),
            ],
        );
        let timeline = sample_timeline(&["era-x"]);
        let got = timeline
            .causal_chain(&EventId::new("event-A"), &repo)
            .unwrap();
        let ids: Vec<&str> = got.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["event-A", "event-B", "event-C", "event-D"]);
    }

    #[test]
    fn causal_chain_ignores_events_outside_timeline() {
        // event-A → event-out (timeline 밖). causal_chain(A) → [A]만.
        let repo = MiniRepo::new(
            vec![era_with_events("era-x", "fall", &["event-A"])],
            vec![
                event_with_related("event-A", &["event-out"]),
                event_with_related("event-out", &[]),
            ],
        );
        let timeline = sample_timeline(&["era-x"]);
        let got = timeline
            .causal_chain(&EventId::new("event-A"), &repo)
            .unwrap();
        let ids: Vec<&str> = got.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["event-A"], "timeline 경계 밖 event-out은 traversal X");
    }

    #[test]
    fn causal_chain_seed_outside_timeline_returns_empty() {
        let repo = MiniRepo::new(
            vec![era_with_events("era-x", "fall", &["event-A"])],
            vec![event_with_related("event-A", &[])],
        );
        let timeline = sample_timeline(&["era-x"]);
        let got = timeline
            .causal_chain(&EventId::new("event-not-in-timeline"), &repo)
            .unwrap();
        assert!(got.is_empty());
    }

    #[test]
    fn causal_chain_bidirectional_traversal_no_revisit() {
        // bloody-night ↔ hwasan-fall 양방향 시연: BFS는 visited set으로 cycle 방지.
        let repo = MiniRepo::new(
            vec![era_with_events(
                "era-fall",
                "fall",
                &["event-bn", "event-hf", "event-bd"],
            )],
            vec![
                // bloody-night ↔ hwasan-fall + bloody-night → blood-disappearance
                event_with_related("event-bn", &["event-hf", "event-bd"]),
                event_with_related("event-hf", &["event-bn"]),
                event_with_related("event-bd", &[]),
            ],
        );
        let timeline = sample_timeline(&["era-fall"]);
        let got = timeline
            .causal_chain(&EventId::new("event-bn"), &repo)
            .unwrap();
        let ids: Vec<&str> = got.iter().map(|e| e.id.as_str()).collect();
        // BFS: bn → [hf, bd] → ... — visited가 cycle 차단.
        assert_eq!(ids.len(), 3, "3 사건 모두 한 번씩만 (cycle 무한 루프 X)");
        assert_eq!(ids[0], "event-bn", "seed가 첫 항목");
        assert!(ids.contains(&"event-hf"));
        assert!(ids.contains(&"event-bd"));
    }

    #[test]
    fn references_skip_empty_serde() {
        let t = Timeline::new("timeline-x", "history", "X");
        let json = serde_json::to_string(&t).unwrap();
        assert!(!json.contains("references"), "빈 references는 skip");
    }

    #[test]
    fn timeline_filter_default() {
        let f = TimelineFilter::default();
        assert!(f.kind.is_none());
        assert!(f.references_era.is_none());
        assert!(f.genre_tag.is_none());
    }
}
