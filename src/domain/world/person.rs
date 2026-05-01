//! Person 애그리거트 — Phase 2 Vertical Slice (인스턴스 도메인 #2).
//!
//! **장르 중립 원칙**: 이 모듈은 wuxia/판타지/SF 어떤 어휘도 모른다. `kind`는
//! free-form `String`이며, 장르가 채운다 (`genres/wuxia/forms/person.toml`).
//!
//! HEXACO 6 dim은 `npc_mind::domain::personality::Score` VO를 직접 재사용한다
//! (-1.0 ~ +1.0 범위 강제, 역직렬화 시 자동 검증). 24 facet은 `extras.hexaco_facets`
//! 정형 JSON으로 선택 보존. mind 시스템 업서트 시 6 dim 값을 4 facet에 spread —
//! 자세한 정책은 `worldbuilding::mind_sync` 참조.
//!
//! Phase 2 외래키 활성:
//! - `affiliation: Vec<GroupId>` — `groups` 테이블 존재 검증 (결손 시 에러).
//! - `Group.members.person_id` ↔ `persons.id` — 결손 시 에러.
//!
//! Phase 3 도메인(Place) 도입 전까지 `birthplace`/`current_location`은 텍스트 보존
//! (경고만).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::domain::personality::Score;
use crate::domain::world::group::GroupId;

/// 인물 식별자 — `npc-{nn}` 또는 `person-{slug}` 형식. wuxia 패키지는 `npc-NN`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PersonId(pub String);

impl PersonId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PersonId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for PersonId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for PersonId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// 생사 상태. SQLite `persons.status` CHECK와 동기화.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PersonStatus {
    /// 게임 시작 시 생존 — 직접 만남 가능.
    #[default]
    Alive,
    /// 사망. 역사 인물 또는 게임 도중 사망.
    Dead,
    /// 생사 불명 — 퀘스트 대상.
    Missing,
    /// 정보 없음.
    Unknown,
}

impl PersonStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Alive => "alive",
            Self::Dead => "dead",
            Self::Missing => "missing",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "alive" => Some(Self::Alive),
            "dead" => Some(Self::Dead),
            "missing" => Some(Self::Missing),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

/// HEXACO 6 dim — `npc_mind` `Score` VO 재사용 (-1.0 ~ +1.0, 역직렬화 시 검증).
///
/// 6 dim은 frontmatter 일급 필드이며 mind 시스템 업서트의 입력. 24 facet 상세는
/// `extras.hexaco_facets` 정형 JSON 또는 본문 산문(`## HEXACO 분석`)에 보존한다.
///
/// `Default`은 모든 차원 0.0(중립).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct HexacoSix {
    #[serde(default = "Score::neutral")]
    pub honesty_humility: Score,
    #[serde(default = "Score::neutral")]
    pub emotionality: Score,
    #[serde(default = "Score::neutral")]
    pub extraversion: Score,
    #[serde(default = "Score::neutral")]
    pub agreeableness: Score,
    #[serde(default = "Score::neutral")]
    pub conscientiousness: Score,
    #[serde(default = "Score::neutral")]
    pub openness: Score,
}

impl Default for HexacoSix {
    fn default() -> Self {
        Self::neutral()
    }
}

impl HexacoSix {
    /// 모든 차원 0.0(중립).
    pub fn neutral() -> Self {
        let s = Score::neutral();
        Self {
            honesty_humility: s,
            emotionality: s,
            extraversion: s,
            agreeableness: s,
            conscientiousness: s,
            openness: s,
        }
    }
}

/// 시간성 — Phase 2엔 자유 텍스트.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PersonTemporal {
    /// 출생 시점 — "215년차 즈음", "원년 직전" 등 자유 텍스트.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birth_year: Option<String>,
    /// 사망 시점 (사망/추정 사망인 경우만).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death_year: Option<String>,
    /// 게임 시작 시점 나이. character-roster의 "나이" 컬럼 매핑.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age_at_game_start: Option<u32>,
    /// 자유 메모.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Person 애그리거트.
///
/// 핵심 책임:
/// - 정체성: id/name/aliases/kind
/// - 생사: status
/// - 성격: hexaco (6 dim 일급)
/// - 시간성: temporal
/// - 소속: affiliation (Phase 2 외래키 활성)
/// - 장소: birthplace/current_location (Phase 3 외래키 텍스트)
/// - 자유 본문: body_sections
/// - 장르 확장: extras (hexaco_facets · signature_skill 등)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Person {
    pub id: PersonId,
    /// 장르가 채움 (wuxia: `active`/`historical`/`legendary`/`player`).
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub status: PersonStatus,
    #[serde(default)]
    pub hexaco: HexacoSix,
    #[serde(default)]
    pub temporal: PersonTemporal,
    /// Phase 1 Group ID — Phase 2부터 정식 외래키 (결손 시 world-load 에러).
    #[serde(default)]
    pub affiliation: Vec<GroupId>,
    /// Place ID 텍스트 — Phase 3 외래키 활성.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birthplace: Option<String>,
    /// Place ID 텍스트 — Phase 3 외래키 활성.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_location: Option<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 장르 확장 free-form. hexaco_facets·signature_skill·biography_short·priority 등.
    #[serde(default)]
    pub extras: Map<String, Value>,
    /// H2 섹션 본문 (제목 → 본문). `BTreeMap`이라 알파벳 정렬 — 마크다운 작성 순서는
    /// 보존되지 않는다. Group과 동일 정책.
    #[serde(default)]
    pub body_sections: BTreeMap<String, String>,
    /// 마크다운 SoT 경로.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}

impl Person {
    /// 최소 생성자. 테스트·도구용. HEXACO는 neutral, status는 Alive.
    pub fn new(id: impl Into<PersonId>, kind: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            name: name.into(),
            aliases: Vec::new(),
            status: PersonStatus::default(),
            hexaco: HexacoSix::neutral(),
            temporal: PersonTemporal::default(),
            affiliation: Vec::new(),
            birthplace: None,
            current_location: None,
            summary: String::new(),
            tags: Vec::new(),
            extras: Map::new(),
            body_sections: BTreeMap::new(),
            source_path: None,
        }
    }

    /// kind가 mind 시스템 업서트 대상인지 판정.
    /// `active` · `player`만 등록 — `historical` · `legendary`는 대화 없음.
    pub fn is_mind_eligible(&self) -> bool {
        matches!(self.kind.as_str(), "active" | "player")
    }
}

/// 리스트 필터 — `WorldRepository::list_persons`에 전달.
#[derive(Debug, Clone, Default)]
pub struct PersonFilter {
    pub kind: Option<String>,
    pub status: Option<PersonStatus>,
    /// 특정 Group의 멤버 필터 — affiliation 배열에 포함된 Person만.
    pub affiliation: Option<GroupId>,
    /// `tags` 배열 매칭.
    pub genre_tag: Option<String>,
}

// ---------------------------------------------------------------------------
// 단위 테스트
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn person_new_sets_defaults() {
        let p = Person::new("npc-test", "active", "테스트");
        assert_eq!(p.id.as_str(), "npc-test");
        assert_eq!(p.kind, "active");
        assert_eq!(p.name, "테스트");
        assert!(p.aliases.is_empty());
        assert_eq!(p.status, PersonStatus::Alive);
        assert_eq!(p.hexaco, HexacoSix::neutral());
        assert!(p.affiliation.is_empty());
        assert!(p.is_mind_eligible());
    }

    #[test]
    fn person_status_from_str_loose() {
        assert_eq!(
            PersonStatus::from_str_loose("Alive"),
            Some(PersonStatus::Alive)
        );
        assert_eq!(
            PersonStatus::from_str_loose(" missing "),
            Some(PersonStatus::Missing)
        );
        assert_eq!(PersonStatus::from_str_loose("dead"), Some(PersonStatus::Dead));
        assert_eq!(PersonStatus::from_str_loose("invalid"), None);
    }

    #[test]
    fn hexaco_six_neutral_all_zero() {
        let h = HexacoSix::neutral();
        assert_eq!(h.honesty_humility.value(), 0.0);
        assert_eq!(h.emotionality.value(), 0.0);
        assert_eq!(h.extraversion.value(), 0.0);
        assert_eq!(h.agreeableness.value(), 0.0);
        assert_eq!(h.conscientiousness.value(), 0.0);
        assert_eq!(h.openness.value(), 0.0);
    }

    #[test]
    fn hexaco_six_serde_roundtrip() {
        let h = HexacoSix {
            honesty_humility: Score::clamped(-0.8),
            emotionality: Score::clamped(-0.3),
            extraversion: Score::clamped(-0.2),
            agreeableness: Score::clamped(-0.7),
            conscientiousness: Score::clamped(0.7),
            openness: Score::clamped(0.5),
        };
        let json = serde_json::to_string(&h).unwrap();
        let back: HexacoSix = serde_json::from_str(&json).unwrap();
        assert_eq!(back, h);
    }

    #[test]
    fn hexaco_six_out_of_range_deserialize_fails() {
        // Score VO는 (-1, 1) 범위 밖 값을 역직렬화 거부 — frontmatter 작성 오류 방어.
        let bad = r#"{"honesty_humility": -1.5, "emotionality": 0.0, "extraversion": 0.0, "agreeableness": 0.0, "conscientiousness": 0.0, "openness": 0.0}"#;
        assert!(serde_json::from_str::<HexacoSix>(bad).is_err());
    }

    #[test]
    fn person_temporal_serde_skips_missing_fields() {
        let t = PersonTemporal {
            birth_year: Some("215년차 즈음".into()),
            death_year: None,
            age_at_game_start: Some(55),
            notes: None,
        };
        let json = serde_json::to_string(&t).unwrap();
        // skip_serializing_if 검증
        assert!(!json.contains("death_year"));
        assert!(!json.contains("notes"));
        assert!(json.contains("215"));
        assert!(json.contains("55"));

        let back: PersonTemporal = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn person_full_serde_roundtrip() {
        let mut p = Person::new("npc-02", "active", "조고(曹高)");
        p.aliases = vec!["대진의 그림자".into(), "십상시의 주인".into()];
        p.status = PersonStatus::Alive;
        p.hexaco = HexacoSix {
            honesty_humility: Score::clamped(-0.8),
            emotionality: Score::clamped(-0.3),
            extraversion: Score::clamped(-0.2),
            agreeableness: Score::clamped(-0.7),
            conscientiousness: Score::clamped(0.7),
            openness: Score::clamped(0.5),
        };
        p.temporal = PersonTemporal {
            age_at_game_start: Some(55),
            birth_year: Some("215년차 즈음".into()),
            ..Default::default()
        };
        p.affiliation = vec![
            GroupId::new("group-daejin-court"),
            GroupId::new("group-shipsangsi"),
        ];
        p.summary = "메인 적대자".into();
        p.tags = vec!["wuxia".into(), "antagonist".into()];
        p.extras
            .insert("priority".into(), json!("★★★"));
        p.body_sections.insert("개요".into(), "본문".into());

        let json = serde_json::to_string(&p).unwrap();
        let back: Person = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn is_mind_eligible_only_active_or_player() {
        let mut p = Person::new("npc-x", "active", "X");
        assert!(p.is_mind_eligible());
        p.kind = "player".into();
        assert!(p.is_mind_eligible());
        p.kind = "historical".into();
        assert!(!p.is_mind_eligible());
        p.kind = "legendary".into();
        assert!(!p.is_mind_eligible());
        p.kind = "unknown".into();
        assert!(!p.is_mind_eligible());
    }
}
