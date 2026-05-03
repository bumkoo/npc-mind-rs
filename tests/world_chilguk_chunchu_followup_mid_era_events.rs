//! Phase 5c.2 follow-up — mid-era 6 event 시드 라운드트립 + era key_events 정합 +
//! related_events 양방향 정합 e2e 테스트.
//!
//! 사양 `task-phase5-followup-mid-era-events.md` v1.0 §3.5·§3.6·§5 자동 검증:
//! - 6 mid-era event (taemuje-enthronement + byeongkwon-recall + mulim-conference-1st +
//!   sapa-formation + jachi-movement + cult-remnant-discovery) 마크다운 라운드트립
//! - 5 신규 kind (founding 재사용 1 + reform-fail · convention · schism · political-movement
//!   · discovery 신규 5)
//! - era 4종 (founding · prosperity · turning · decline) `key_events` 슬롯 채움
//! - related_events 양방향 정합 — task §3.5의 7개 인과 사슬 모두 forward + reverse
//! - 외래키 결손 0건 (5c.1 산출 npc 활성)
//!
//! 실행: `cargo test --features embed --test world_chilguk_chunchu_followup_mid_era_events`
//!
//! 본 follow-up은 데이터 변경만 — Event/Era 도메인 변경 0. 마크다운 SoT 라운드트립과
//! frontmatter·body 라운드트립을 검증.

#![cfg(feature = "embed")]

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use npc_mind::domain::world::{Event, EventId};
use npc_mind::worldbuilding::markdown::{era_from_markdown, event_from_markdown};

/// Phase 5c.2 mid-era 6 event 시간순 (체크포인트 1 + 체크포인트 2):
/// (id, year_relative, era_id, kind)
const MID_ERA_EVENTS: &[(&str, i32, &str, &str)] = &[
    ("event-byeongkwon-recall", -240, "era-founding", "reform-fail"),
    ("event-mulim-conference-1st", -170, "era-prosperity", "convention"),
    ("event-sapa-formation", -140, "era-turning", "schism"),
    ("event-jachi-movement", -110, "era-turning", "political-movement"),
    ("event-cult-remnant-discovery", -80, "era-turning", "discovery"),
    ("event-taemuje-enthronement", -33, "era-decline", "founding"),
];

/// task §3.5의 양방향 인과 사슬 (forward edges — reverse는 자동 검증).
/// `(from, to)`는 from.related_events에 to가 있어야 함.
const BIDIRECTIONAL_LINKS: &[(&str, &str)] = &[
    // 5 mid-era 신규 event 간 인과
    ("event-empire-founding", "event-byeongkwon-recall"),
    ("event-byeongkwon-recall", "event-mulim-conference-1st"),
    ("event-mulim-conference-1st", "event-sapa-formation"),
    ("event-sapa-formation", "event-cult-remnant-discovery"),
    ("event-cult-remnant-discovery", "event-bloody-cult-rebellion-2nd"),
    ("event-jachi-movement", "event-six-states-independence"),
    // 체크포인트 1: taemuje-enthronement 인과 사슬
    ("event-taemuje-enthronement", "event-bloody-cult-rebellion-2nd"),
    ("event-taemuje-enthronement", "event-blood-disappearance"),
    ("event-taemuje-enthronement", "event-bloody-night"),
    // empire-founding ↔ bloody-cult-rebellion-2nd (Phase 5a 기존)
    ("event-empire-founding", "event-bloody-cult-rebellion-2nd"),
];

/// era 4종 (founding · prosperity · turning · decline) — key_events 슬롯 검증.
/// fall-of-empire는 Phase 5b에서 이미 채워졌고 본 follow-up 스코프 외.
const ERA_KEY_EVENTS_EXPECTED: &[(&str, &[&str])] = &[
    (
        "era-founding",
        &["event-empire-founding", "event-byeongkwon-recall"],
    ),
    ("era-prosperity", &["event-mulim-conference-1st"]),
    (
        "era-turning",
        &[
            "event-sapa-formation",
            "event-jachi-movement",
            "event-cult-remnant-discovery",
        ],
    ),
    ("era-decline", &["event-taemuje-enthronement"]),
];

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn event_dir() -> PathBuf {
    project_root().join("projects/chilguk-chunchu/world/event")
}

fn era_dir() -> PathBuf {
    project_root().join("projects/chilguk-chunchu/world/era")
}

fn load_event(id: &str) -> Event {
    let path = event_dir().join(format!("{id}.md"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing event file: {}", path.display()));
    event_from_markdown(&raw).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn load_all_events() -> Vec<Event> {
    let dir = event_dir();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|_| panic!("missing dir: {}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let raw = std::fs::read_to_string(&p).expect("read .md");
            event_from_markdown(&raw).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
        })
        .collect()
}

#[test]
fn six_mid_era_events_parse_with_expected_meta() {
    for &(id, year_relative, era_id, kind) in MID_ERA_EVENTS {
        let event = load_event(id);
        assert_eq!(event.id.as_str(), id, "{id}: id 라운드트립");
        assert_eq!(event.kind, kind, "{id}: kind={kind} 기대");
        assert_eq!(
            event.temporal.year_relative,
            Some(year_relative),
            "{id}: year_relative={year_relative} 기대"
        );
        assert_eq!(
            event.era_id.as_deref(),
            Some(era_id),
            "{id}: era_id={era_id} 기대"
        );
        assert!(!event.summary.is_empty(), "{id}: summary 비어있음");
        assert!(
            !event.body_sections.is_empty(),
            "{id}: body_sections 비어있음 (산문 §개요 등 필요)"
        );
    }
}

#[test]
fn five_new_kinds_introduced_per_q1_policy() {
    // task §3.3·§6.2·디렉터 Q1 정책 — Phase 5c.2가 신규 kind 5종 도입:
    let mut new_kinds = HashSet::new();
    for &(_, _, _, kind) in MID_ERA_EVENTS {
        if kind != "founding" {
            new_kinds.insert(kind);
        }
    }
    let expected: HashSet<&str> = [
        "reform-fail",
        "convention",
        "schism",
        "political-movement",
        "discovery",
    ]
    .into_iter()
    .collect();
    assert_eq!(new_kinds, expected, "Phase 5c.2 신규 kind 5종");

    // founding은 재사용 (taemuje-enthronement) — empire-founding과 같은 카테고리.
    let founding_count = MID_ERA_EVENTS
        .iter()
        .filter(|(_, _, _, k)| *k == "founding")
        .count();
    assert_eq!(founding_count, 1, "founding 재사용은 1건 (taemuje-enthronement)");
}

#[test]
fn era_boundary_consistency() {
    // start inclusive · end exclusive (task §3.2):
    //   era-founding [-270, -220) ∋ -240 ✓
    //   era-prosperity [-220, -150) ∋ -170 ✓
    //   era-turning [-150, -70) ∋ -140, -110, -80 ✓
    //   era-decline [-70, -30) ∋ -33 ✓
    let bounds: HashMap<&str, (i32, i32)> = [
        ("era-founding", (-270, -220)),
        ("era-prosperity", (-220, -150)),
        ("era-turning", (-150, -70)),
        ("era-decline", (-70, -30)),
    ]
    .into_iter()
    .collect();

    for &(id, year_relative, era_id, _) in MID_ERA_EVENTS {
        let (lo, hi) = bounds[era_id];
        assert!(
            year_relative >= lo && year_relative < hi,
            "{id}: year_relative={year_relative} not in [{lo}, {hi}) of {era_id}"
        );
    }
}

#[test]
fn era_key_events_slots_filled() {
    for &(era_id, expected_keys) in ERA_KEY_EVENTS_EXPECTED {
        let path = era_dir().join(format!("{era_id}.md"));
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("missing era file: {}", path.display()));
        let era = era_from_markdown(&raw).unwrap_or_else(|e| panic!("{era_id}: {e}"));
        let actual: Vec<&str> = era.key_events.iter().map(|e| e.as_str()).collect();
        let expected: Vec<&str> = expected_keys.iter().copied().collect();
        assert_eq!(
            actual, expected,
            "{era_id}: key_events 슬롯 정합 (순서 포함)"
        );
    }
}

#[test]
fn related_events_bidirectional_integrity() {
    let events = load_all_events();
    let by_id: HashMap<String, &Event> = events
        .iter()
        .map(|e| (e.id.as_str().to_string(), e))
        .collect();

    for &(from, to) in BIDIRECTIONAL_LINKS {
        // forward
        let from_event = by_id
            .get(from)
            .unwrap_or_else(|| panic!("missing event: {from}"));
        let has_forward = from_event
            .related_events
            .iter()
            .any(|e| e.as_str() == to);
        assert!(
            has_forward,
            "{from}.related_events에 {to} 있어야 함 (forward link)"
        );

        // reverse
        let to_event = by_id
            .get(to)
            .unwrap_or_else(|| panic!("missing event: {to}"));
        let has_reverse = to_event
            .related_events
            .iter()
            .any(|e| e.as_str() == from);
        assert!(
            has_reverse,
            "{to}.related_events에 {from} 있어야 함 (reverse link, 양방향 정합)"
        );
    }
}

#[test]
fn related_events_targets_all_resolve() {
    // 6 신규 + 6 Phase 5a = 12 events. 모든 related_events 항목이 실제 event id로 해소.
    let events = load_all_events();
    let all_ids: HashSet<String> = events.iter().map(|e| e.id.as_str().to_string()).collect();
    assert_eq!(events.len(), 12, "전체 event 수 = 12 (Phase 5a 6 + Phase 5c.2 6)");

    for event in &events {
        for related in &event.related_events {
            assert!(
                all_ids.contains(related.as_str()),
                "{}: related_events에 미해소 id {} 발견",
                event.id,
                related
            );
        }
    }
}

#[test]
fn fk_activations_for_5c1_npcs() {
    // 체크포인트 1: event-taemuje-enthronement participants.people = [npc-danun]
    let enthronement = load_event("event-taemuje-enthronement");
    assert_eq!(
        enthronement.participants.people,
        vec!["npc-danun".to_string()],
        "taemuje-enthronement.participants.people = [npc-danun]"
    );
    assert!(
        enthronement.involves_person("npc-danun"),
        "involves_person helper 정합"
    );
    // 디렉터 Q4 결정 — npc-02(조고) 미포함
    assert!(
        !enthronement.involves_person("npc-02"),
        "Q4: 즉위 시점 조고는 22세 하급 관리, participants 미포함"
    );
    // group-shipsangsi(245년차 결성)도 미포함
    assert!(
        !enthronement.involves_group("group-shipsangsi"),
        "Q4: 즉위 시점 십상시 미존재, participants 미포함"
    );

    // 체크포인트 2 — Q3 정책: 5 mid-era event participants.people 모두 비움
    for id in [
        "event-byeongkwon-recall",
        "event-mulim-conference-1st",
        "event-sapa-formation",
        "event-jachi-movement",
        "event-cult-remnant-discovery",
    ] {
        let event = load_event(id);
        assert!(
            event.participants.people.is_empty(),
            "{id}: Q3 정책 — 본 사건 인물 모두 5c.1 미등록, participants.people 비움"
        );
    }
}

#[test]
fn cult_remnant_discovery_chains_to_taemuje_via_bloody_cult() {
    // task §3.5 인과 사슬 검증 — cult-remnant-discovery(-80) → bloody-cult-rebellion-2nd(-30)
    // → taemuje-enthronement(-33). 본 흐름은 cult-remnant 발견이 80년 후 혈교 부활의 인과
    // 시작점이며, 같은 시기 taemuje-enthronement(-33)이 ③ 수명 연장 거래로 부활을 트리거.
    let cult = load_event("event-cult-remnant-discovery");
    let bloody = load_event("event-bloody-cult-rebellion-2nd");
    let taemuje = load_event("event-taemuje-enthronement");

    // cult-remnant-discovery → bloody-cult-rebellion-2nd
    assert!(
        cult.related_events
            .iter()
            .any(|e| e.as_str() == "event-bloody-cult-rebellion-2nd"),
        "cult-remnant-discovery → bloody-cult-rebellion-2nd 인과 사슬"
    );
    // bloody-cult-rebellion-2nd가 cult-remnant-discovery + taemuje-enthronement 양쪽을
    // 인과 사슬로 가짐 (서로 다른 인과 — cult는 80년 잔당 재건, taemuje는 ③ 거래 직접 원인)
    let has_cult = bloody
        .related_events
        .iter()
        .any(|e| e.as_str() == "event-cult-remnant-discovery");
    let has_taemuje = bloody
        .related_events
        .iter()
        .any(|e| e.as_str() == "event-taemuje-enthronement");
    assert!(
        has_cult && has_taemuje,
        "bloody-cult-rebellion-2nd가 cult-remnant-discovery·taemuje-enthronement 양쪽을 역방향으로 가져야 함"
    );

    // taemuje 시점 -33, cult 시점 -80 — 47년 차이의 인과 사슬은 직접 related_events 아님
    // (사이에 bloody-cult-rebellion-2nd 매개)
    assert!(
        !taemuje.related_events.iter().any(|e| e.as_str() == "event-cult-remnant-discovery"),
        "taemuje-enthronement은 cult-remnant-discovery와 직접 related_events 아님 (간접)"
    );
}

#[test]
fn jachi_movement_directly_chains_to_six_states() {
    // task §3.5: jachi-movement(-110) → six-states-independence(-7) 직접 인과 (110년 누적)
    let jachi = load_event("event-jachi-movement");
    let six_states = load_event("event-six-states-independence");

    assert!(
        jachi
            .related_events
            .iter()
            .any(|e| e.as_str() == "event-six-states-independence"),
        "jachi-movement → six-states-independence forward link"
    );
    assert!(
        six_states
            .related_events
            .iter()
            .any(|e| e.as_str() == "event-jachi-movement"),
        "six-states-independence → jachi-movement reverse link (양방향)"
    );
    // jachi-movement은 동해·남만·서량 3 places 포함
    assert!(jachi.involves_place("place-donghae"));
    assert!(jachi.involves_place("place-namman"));
    assert!(jachi.involves_place("place-seoryang"));
    assert!(jachi.involves_place("place-daejin"));
}

#[test]
fn taemuje_enthronement_summary_corrected() {
    // PR #76 review Finding #1: 화산파 멸문은 +23년 후이지 +30년 후가 아님.
    // 회귀 가드 — summary와 §결과에서 "30년 후 화산파 멸문" 표현이 들어가지 않아야.
    let event = load_event("event-taemuje-enthronement");
    assert!(
        !event.summary.contains("30년 후 화산파 멸문"),
        "summary에 잘못된 '30년 후 화산파 멸문' 표현이 다시 들어가지 않아야 함"
    );
    let result_section = event
        .body_sections
        .get("결과")
        .unwrap_or_else(|| panic!("§결과 body_section 누락"));
    assert!(
        !result_section.contains("30년 후 (267년차"),
        "§결과에 잘못된 '30년 후 (267년차' 표현이 다시 들어가지 않아야 함"
    );
    assert!(
        result_section.contains("23년 후 (260년차, -10): 화산파 멸문"),
        "§결과에 정정된 '23년 후 (260년차, -10): 화산파 멸문' 표현이 있어야 함"
    );
}

#[test]
fn event_id_namespace_consistency() {
    // 6 mid-era event 모두 `event-{slug}` 형식, slug는 ASCII 소문자·숫자·하이픈.
    for &(id, _, _, _) in MID_ERA_EVENTS {
        assert!(id.starts_with("event-"), "{id}: event- 접두 필요");
        let slug = &id["event-".len()..];
        assert!(
            slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "{id}: slug에 ASCII 소문자·숫자·하이픈 외 문자 발견"
        );
    }
}

#[test]
fn participants_groups_use_only_registered_groups() {
    // 6 mid-era event의 participants.groups가 등록된 6 group 중에서만 참조.
    let registered_groups: HashSet<&str> = [
        "group-cheonma-shingyo",
        "group-daejin-court",
        "group-gaebang",
        "group-mulim-mang",
        "group-namgung",
        "group-shipsangsi",
    ]
    .into_iter()
    .collect();

    for &(id, _, _, _) in MID_ERA_EVENTS {
        let event = load_event(id);
        for group in &event.participants.groups {
            assert!(
                registered_groups.contains(group.as_str()),
                "{id}: participants.groups의 {group}이 등록된 group이 아님"
            );
        }
    }
}

#[test]
fn participants_places_use_only_registered_places() {
    // 6 mid-era event의 participants.places가 등록된 11 place 중에서만 참조.
    let registered_places: HashSet<&str> = [
        "place-daejin",
        "place-donghae",
        "place-namman",
        "place-namgung",
        "place-seoryang",
        "place-jiyu-doshi",
        "place-bukwon",
        "place-jungwon",
        "place-hwasan",
        "place-sorim",
        "place-mudang",
    ]
    .into_iter()
    .collect();

    for &(id, _, _, _) in MID_ERA_EVENTS {
        let event = load_event(id);
        for place in &event.participants.places {
            assert!(
                registered_places.contains(place.as_str()),
                "{id}: participants.places의 {place}이 등록된 place가 아님"
            );
        }
    }
}

/// Helper로 EventId 라운드트립 검증.
#[test]
fn event_id_roundtrip_preserved_for_all_events() {
    let events = load_all_events();
    for event in &events {
        // EventId의 String 형태가 유지되는지 확인
        let id_str = event.id.as_str();
        let recreated = EventId::new(id_str);
        assert_eq!(recreated.as_str(), id_str);
    }
}
