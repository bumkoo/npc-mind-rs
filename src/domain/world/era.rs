//! Era 애그리거트 — Phase 5b Vertical Slice. **세 번째 인스턴스 도메인**.
//!
//! **장르 중립 원칙**: 이 모듈은 wuxia/판타지/SF 어떤 어휘도 모른다. `kind`는
//! free-form `String`이며, 장르가 채운다 (`genres/wuxia/forms/era.toml` —
//! `founding`/`prosperity`/`turning`/`decline`/`fall`).
//!
//! Phase 5b 외래키:
//! - `Era.key_events` ↔ `Event.id` (활성 — world-load hard-fail, Era→Event 단방향 외래키)
//! - 본 도메인은 Event/Atlas의 `era_id` ↔ `eras.id` 역참조 대상이 된다 (Phase 5a/4 텍스트 → Phase 5b 활성)
//!
//! Boundary 정책: `start_year_relative` inclusive · `end_year_relative` exclusive.
//! 사양 §3.3 참조 — boundary 케이스는 정확히 한 era에만 속하도록 보장.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

use super::event::EventId;

/// Era 식별자 — `era-{slug}` 형식. slug는 ASCII 소문자·숫자·하이픈.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EraId(pub String);

impl EraId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EraId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for EraId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for EraId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Era 시간 메타. 270년차 기준 절대 연도(`year_relative`)로 시작·종료를 정의.
///
/// Boundary 정책 (§3.3):
/// - `start_year_relative` **inclusive** — 해당 연도가 본 era 시작
/// - `end_year_relative` **exclusive** — 해당 연도는 다음 era 시작
///
/// 예: era-fall-of-empire (start=-30, end=0) → -30 ≤ year < 0. -30년차는 본 era 시작,
/// 0년차(현재 270년차)는 어느 era에도 속하지 않음(별도 era 추가 시까지 era_id 비울 것).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct EraTemporal {
    /// 시작 연도 (270년차 기준, inclusive). 예: era-founding=-270.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_year_relative: Option<i32>,
    /// 종료 연도 (270년차 기준, exclusive). 예: era-founding=-220.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_year_relative: Option<i32>,
    /// 자유 메모 (boundary 정책 적용 의도, 시대 종료 트리거 등).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Era 애그리거트 — 세 번째 인스턴스 도메인.
///
/// 핵심 책임:
/// - 정체성: id/name/aliases/kind + summary/tags
/// - 시간 메타: temporal (start/end_year_relative) — boundary 정책 §3.3
/// - **합성 핵심**: key_events (본 era를 대표하는 핵심 사건 외래키)
/// - 자유 본문: body_sections (`## 개요` · `## 핵심 트리거` · `## 결과` 등)
/// - 장르 확장: extras (game_role · player_relevance 등)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Era {
    pub id: EraId,
    /// 장르가 채움 (Phase 5b wuxia: `founding`/`prosperity`/`turning`/`decline`/`fall`).
    pub kind: String,
    pub name: String,
    /// 별호·옛 이름. 예: `["6국 분열기", "240-270년차"]`. FTS5 검색 대상에 포함.
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 장르 자유 확장. wuxia 예: `game_role`(서사적 역할)·`player_relevance`(1-5).
    #[serde(default)]
    pub extras: Map<String, Value>,
    #[serde(default)]
    pub temporal: EraTemporal,
    /// **핵심** — 본 era를 대표하는 사건들. world-load 시 hard-fail (Era→Event 단방향 외래키).
    /// 작성 순서가 곧 시간순 권장 (인과 흐름 시각화 용이).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_events: Vec<EventId>,
    /// H2 섹션 본문. `BTreeMap`이라 알파벳 정렬 순서로 보존되며, 작성 순서는
    /// 보존되지 않는다 (Group/Person/Place/Atlas/Event 동일 정책).
    #[serde(default)]
    pub body_sections: BTreeMap<String, String>,
    /// 마크다운 SoT 경로 (절대 또는 프로젝트 root 기준 상대).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}

impl Era {
    /// 최소 생성자. 테스트·도구용.
    pub fn new(id: impl Into<EraId>, kind: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            name: name.into(),
            aliases: Vec::new(),
            summary: String::new(),
            tags: Vec::new(),
            extras: Map::new(),
            temporal: EraTemporal::default(),
            key_events: Vec::new(),
            body_sections: BTreeMap::new(),
            source_path: None,
        }
    }

    /// 본 era의 시간 범위 안에 year_relative가 속하는가 (start inclusive · end exclusive).
    /// start/end 둘 중 하나라도 None이면 false (정의되지 않은 boundary).
    pub fn contains_year(&self, year_relative: i32) -> bool {
        match (self.temporal.start_year_relative, self.temporal.end_year_relative) {
            (Some(start), Some(end)) => year_relative >= start && year_relative < end,
            _ => false,
        }
    }

    /// 본 era의 길이 (연단위, end - start). start/end 모두 있어야 Some.
    pub fn duration_years(&self) -> Option<u32> {
        match (self.temporal.start_year_relative, self.temporal.end_year_relative) {
            (Some(start), Some(end)) if end >= start => Some((end - start) as u32),
            _ => None,
        }
    }
}

/// 리스트 필터 — `WorldRepository::list_eras`에 전달.
#[derive(Debug, Clone, Default)]
pub struct EraFilter {
    pub kind: Option<String>,
    /// 본 era가 포함하는 year_relative (start_year_relative <= ? AND end_year_relative > ?).
    /// boundary 정책 §3.3 — start inclusive · end exclusive.
    pub contains_year: Option<i32>,
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
    fn era_new_sets_defaults() {
        let e = Era::new("era-x", "founding", "X 시대");
        assert_eq!(e.id.as_str(), "era-x");
        assert_eq!(e.kind, "founding");
        assert_eq!(e.name, "X 시대");
        assert!(e.aliases.is_empty());
        assert!(e.tags.is_empty());
        assert!(e.key_events.is_empty());
        assert!(e.temporal.start_year_relative.is_none());
        assert!(e.temporal.end_year_relative.is_none());
    }

    #[test]
    fn era_temporal_serde_skip_when_none() {
        let t = EraTemporal::default();
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "{}", "모든 필드 None이면 빈 객체");
    }

    #[test]
    fn era_temporal_serializes_negative_years() {
        let t = EraTemporal {
            start_year_relative: Some(-270),
            end_year_relative: Some(-220),
            notes: Some("건국기".into()),
        };
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("\"start_year_relative\":-270"));
        assert!(json.contains("\"end_year_relative\":-220"));
        let back: EraTemporal = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn contains_year_inclusive_start() {
        // boundary 정책 §3.3: start inclusive.
        let mut e = Era::new("era-x", "fall", "X");
        e.temporal.start_year_relative = Some(-30);
        e.temporal.end_year_relative = Some(0);
        assert!(e.contains_year(-30), "start year (-30) inclusive — true");
        assert!(e.contains_year(-29));
        assert!(e.contains_year(-1));
    }

    #[test]
    fn contains_year_exclusive_end() {
        // boundary 정책 §3.3: end exclusive.
        let mut e = Era::new("era-x", "fall", "X");
        e.temporal.start_year_relative = Some(-30);
        e.temporal.end_year_relative = Some(0);
        assert!(!e.contains_year(0), "end year (0) exclusive — false");
        assert!(!e.contains_year(1));
    }

    #[test]
    fn contains_year_outside_range() {
        let mut e = Era::new("era-x", "fall", "X");
        e.temporal.start_year_relative = Some(-30);
        e.temporal.end_year_relative = Some(0);
        assert!(!e.contains_year(-31));
        assert!(!e.contains_year(-100));
        assert!(!e.contains_year(100));
    }

    #[test]
    fn contains_year_undefined_boundary_returns_false() {
        // start 또는 end 둘 중 하나라도 없으면 false (정의되지 않은 era).
        let e = Era::new("era-x", "fall", "X");
        assert!(!e.contains_year(0));
        assert!(!e.contains_year(-50));

        let mut partial = Era::new("era-y", "fall", "Y");
        partial.temporal.start_year_relative = Some(-30);
        // end 없음
        assert!(!partial.contains_year(-20));
    }

    #[test]
    fn duration_years_computes_difference() {
        let mut e = Era::new("era-x", "fall", "X");
        e.temporal.start_year_relative = Some(-30);
        e.temporal.end_year_relative = Some(0);
        assert_eq!(e.duration_years(), Some(30));

        let mut founding = Era::new("era-f", "founding", "F");
        founding.temporal.start_year_relative = Some(-270);
        founding.temporal.end_year_relative = Some(-220);
        assert_eq!(founding.duration_years(), Some(50));
    }

    #[test]
    fn duration_years_none_when_undefined() {
        let e = Era::new("era-x", "fall", "X");
        assert!(e.duration_years().is_none());
    }

    #[test]
    fn duration_years_none_when_end_before_start() {
        // 잘못된 데이터 보호 — end < start면 None (계산 불가).
        let mut e = Era::new("era-x", "fall", "X");
        e.temporal.start_year_relative = Some(0);
        e.temporal.end_year_relative = Some(-30);
        assert!(e.duration_years().is_none());
    }

    #[test]
    fn key_events_skip_empty_serde() {
        let e = Era::new("era-x", "fall", "X");
        let json = serde_json::to_string(&e).unwrap();
        assert!(!json.contains("key_events"), "빈 key_events는 skip");
    }

    #[test]
    fn key_events_preserves_input_order() {
        // key_events 작성 순서가 시간순 권장 — 정렬·재정렬 없음.
        let mut e = Era::new("era-x", "fall", "X");
        e.key_events = vec![
            EventId::new("event-c"),
            EventId::new("event-a"),
            EventId::new("event-b"),
        ];
        let json = serde_json::to_string(&e).unwrap();
        let back: Era = serde_json::from_str(&json).unwrap();
        assert_eq!(
            back.key_events,
            vec![
                EventId::new("event-c"),
                EventId::new("event-a"),
                EventId::new("event-b"),
            ]
        );
    }

    #[test]
    fn full_serde_roundtrip_preserves_all_fields() {
        let mut e = Era::new("era-fall-of-empire", "fall", "붕괴기");
        e.aliases = vec!["6국 분열기".into(), "240-270년차".into()];
        e.summary = "240~270년차의 30년".into();
        e.tags = vec!["wuxia".into(), "era".into(), "historical".into()];
        e.temporal = EraTemporal {
            start_year_relative: Some(-30),
            end_year_relative: Some(0),
            notes: Some("boundary inclusive-exclusive".into()),
        };
        e.key_events = vec![
            EventId::new("event-bloody-night"),
            EventId::new("event-hwasan-fall"),
        ];
        e.body_sections.insert("개요".into(), "산문".into());
        e.body_sections.insert("결과".into(), "칠국 형성".into());
        e.extras
            .insert("game_role".into(), Value::String("게임 시작 시점".into()));

        let json = serde_json::to_string(&e).unwrap();
        let back: Era = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn era_id_from_str_and_string() {
        let a = EraId::from("era-x");
        let b = EraId::from(String::from("era-x"));
        assert_eq!(a, b);
        assert_eq!(a.as_str(), "era-x");
    }

    #[test]
    fn era_id_serde_transparent() {
        let id = EraId::new("era-fall-of-empire");
        let json = serde_json::to_string(&id).unwrap();
        // transparent — 객체가 아니라 문자열로 직렬화.
        assert_eq!(json, "\"era-fall-of-empire\"");
        let back: EraId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn five_canonical_eras_boundary_consistency() {
        // 5 era boundary가 사양 §3.3 정책대로 연속적이고 겹치지 않음을 회귀 가드.
        // start inclusive · end exclusive이라 era_n.end == era_{n+1}.start이어야.
        let founding = era_with_range("era-founding", "founding", -270, -220);
        let prosperity = era_with_range("era-prosperity", "prosperity", -220, -150);
        let turning = era_with_range("era-turning", "turning", -150, -70);
        let decline = era_with_range("era-decline", "decline", -70, -30);
        let fall = era_with_range("era-fall-of-empire", "fall", -30, 0);

        assert_eq!(founding.duration_years(), Some(50));
        assert_eq!(prosperity.duration_years(), Some(70));
        assert_eq!(turning.duration_years(), Some(80));
        assert_eq!(decline.duration_years(), Some(40));
        assert_eq!(fall.duration_years(), Some(30));
        assert_eq!(50 + 70 + 80 + 40 + 30, 270, "총 270년");

        // boundary 케이스 — Phase 5a 6 Event 매핑 검증
        assert!(founding.contains_year(-270));
        assert!(fall.contains_year(-30));
        assert!(!decline.contains_year(-30));
        assert!(fall.contains_year(-12));
        assert!(fall.contains_year(-10));
        assert!(fall.contains_year(-7));
        // 270년차 (=0)는 어느 era에도 속하지 않음
        assert!(!fall.contains_year(0));
    }

    fn era_with_range(id: &str, kind: &str, start: i32, end: i32) -> Era {
        let mut e = Era::new(id, kind, id);
        e.temporal.start_year_relative = Some(start);
        e.temporal.end_year_relative = Some(end);
        e
    }
}
