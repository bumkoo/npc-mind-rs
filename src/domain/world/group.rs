//! Group 애그리거트 — 첫 인스턴스 도메인 (Phase 1 Vertical Slice).
//!
//! **장르 중립 원칙**: 이 모듈은 wuxia/판타지/SF 어떤 어휘도 모른다. `kind`는
//! free-form `String`이며, 장르가 채운다 (`genres/wuxia/forms/group.toml`).
//!
//! Phase 1 외래키는 텍스트 보존 (Person/Place 도메인 없음). `parent_group` cycle만
//! Phase 1부터 검증 (같은 도메인 내).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// 그룹 식별자 — `group-{slug}` 형식. slug는 ASCII 소문자·숫자·하이픈.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GroupId(pub String);

impl GroupId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for GroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for GroupId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for GroupId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// 그룹 활성 상태. SQLite `groups.status` CHECK와 동기화.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GroupStatus {
    #[default]
    Active,
    Declining,
    Dissolved,
    /// 잠적·잠복 — 활동 중단했으나 해체는 아님.
    Dormant,
}

impl GroupStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Declining => "declining",
            Self::Dissolved => "dissolved",
            Self::Dormant => "dormant",
        }
    }

    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "active" => Some(Self::Active),
            "declining" => Some(Self::Declining),
            "dissolved" => Some(Self::Dissolved),
            "dormant" => Some(Self::Dormant),
            _ => None,
        }
    }
}

/// 시간성 — Phase 1엔 자유 텍스트, Phase 5(Era 결합) 시 정형화 예정.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Temporal {
    /// 형성 시점 — "원년 (270년 전)", "현재 황조 즉위 시" 등 자유 텍스트.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub founded_at: Option<String>,
    /// 해체 시점 (해체된 경우만). dissolved_at이 Some이면 status는 Dissolved 권장.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dissolved_at: Option<String>,
    /// 활성 상태. 기본 Active.
    #[serde(default)]
    pub status: GroupStatus,
    /// 시기별 변동 자유 메모.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// 멤버 참조 — Phase 1엔 텍스트 person_id 보존, Phase 2부터 외래키 활성.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemberRef {
    /// Person ID 텍스트 ("npc-02" 등). Phase 2 외래키 활성 시 검증.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person_id: Option<String>,
    /// person_id가 없을 때 표시명.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// 역할 — "수장", "이인자", "문도", "외부 협력자" 등 자유 텍스트.
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Group 애그리거트.
///
/// 핵심 책임:
/// - 정체성: id/name/aliases/kind
/// - 시간성: temporal (founded/dissolved/status/notes)
/// - 멤버십: members 텍스트 보존
/// - 외래키 텍스트: headquarters/parent_group
/// - 수평 관계: allied_groups/rival_groups
/// - 자유 본문: body_sections (h2 헤더 → 본문)
/// - 장르 확장: extras (장르가 채우는 free-form JSON map; alignment 등)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Group {
    pub id: GroupId,
    /// 장르가 채움 (Phase 1 wuxia: `dynasty-court`/`clan`/`sect-religious`/
    /// `mendicant-order`/`alliance`/`covert-band`/`tribe-confederacy`/`merchants-council`).
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub extras: Map<String, Value>,
    /// H2 섹션 본문 (제목 → 본문). `BTreeMap`이라 **알파벳 정렬** 순서로 보존되며,
    /// 마크다운 작성 순서는 보존되지 않는다 (FTS5 인덱싱·결정적 직렬화에는 문제 없음).
    /// 표시 순서가 중요해지면 (Phase 2+ UI 패널 등) `Vec<(String, String)>` 또는
    /// `IndexMap`으로 교체 검토.
    #[serde(default)]
    pub body_sections: BTreeMap<String, String>,
    #[serde(default)]
    pub temporal: Temporal,
    #[serde(default)]
    pub members: Vec<MemberRef>,
    /// Place ID 텍스트 — Phase 3 외래키 활성.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headquarters: Option<String>,
    /// 수직 포함 — 같은 도메인 내, Phase 1부터 cycle 검증.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_group: Option<GroupId>,
    #[serde(default)]
    pub allied_groups: Vec<GroupId>,
    #[serde(default)]
    pub rival_groups: Vec<GroupId>,
    /// 마크다운 SoT 경로 (절대 또는 프로젝트 root 기준 상대).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}

impl Group {
    /// 최소 생성자. 테스트·도구용.
    pub fn new(id: impl Into<GroupId>, kind: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            name: name.into(),
            aliases: Vec::new(),
            summary: String::new(),
            tags: Vec::new(),
            extras: Map::new(),
            body_sections: BTreeMap::new(),
            temporal: Temporal::default(),
            members: Vec::new(),
            headquarters: None,
            parent_group: None,
            allied_groups: Vec::new(),
            rival_groups: Vec::new(),
            source_path: None,
        }
    }

    /// `extras["alignment"]`를 String으로 추출 (없거나 비문자열이면 None).
    /// SQLite `groups.alignment` 캐시 컬럼 채우는 데 사용.
    pub fn alignment(&self) -> Option<&str> {
        self.extras.get("alignment").and_then(|v| v.as_str())
    }
}

/// 리스트 필터 — `WorldRepository::list_groups`에 전달.
#[derive(Debug, Clone, Default)]
pub struct GroupFilter {
    pub kind: Option<String>,
    pub status: Option<GroupStatus>,
    pub parent_group: Option<GroupId>,
    /// `tags` 또는 `genre_tags` 필터 — body_sections·tags 어느 쪽에든 매칭.
    pub genre_tag: Option<String>,
    /// extras에서 추출된 alignment 캐시 컬럼 매칭.
    pub alignment: Option<String>,
}

/// World 도메인 공용 에러.
#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("저장소 오류: {0}")]
    Storage(String),
    #[error("그룹 '{0}' 없음")]
    NotFound(String),
    #[error("parent_group cycle 감지: {path}")]
    ParentCycle { path: String },
    #[error("부적절한 입력: {0}")]
    Invalid(String),
}

// ---------------------------------------------------------------------------
// parent_group cycle 검증
// ---------------------------------------------------------------------------

/// `groups` 컬렉션 내 `parent_group` 사슬을 DFS로 따라가 자기 자신에 도달하는지 검사.
///
/// **빌드 타임 경고용** — Phase 1엔 에러로 던지지 않고 호출자가 결과를 보고서로 출력.
/// 중복 cycle은 정렬된 노드의 첫 등장만 반환 (deterministic).
pub fn detect_parent_group_cycle(groups: &[Group]) -> Vec<Vec<GroupId>> {
    let by_id: HashMap<&GroupId, &Group> = groups.iter().map(|g| (&g.id, g)).collect();
    let mut seen_cycles: BTreeSet<Vec<GroupId>> = BTreeSet::new();
    let mut sorted: Vec<&GroupId> = by_id.keys().copied().collect();
    sorted.sort();
    for start in sorted {
        let mut visited: HashSet<&GroupId> = HashSet::new();
        let mut path: Vec<GroupId> = Vec::new();
        let mut cur: Option<&GroupId> = Some(start);
        while let Some(id) = cur {
            if visited.contains(&id) {
                if let Some(idx) = path.iter().position(|p| p == id) {
                    let mut cyc: Vec<GroupId> = path[idx..].to_vec();
                    // canonical 형태: cycle 회전 중 가장 작은 시작점으로 정렬
                    rotate_to_min(&mut cyc);
                    seen_cycles.insert(cyc);
                }
                break;
            }
            visited.insert(id);
            path.push(id.clone());
            cur = by_id
                .get(id)
                .and_then(|g| g.parent_group.as_ref())
                .filter(|pid| by_id.contains_key(pid));
        }
    }
    seen_cycles.into_iter().collect()
}

fn rotate_to_min(cycle: &mut [GroupId]) {
    if cycle.is_empty() {
        return;
    }
    let mut min_idx = 0;
    for (i, id) in cycle.iter().enumerate() {
        if id < &cycle[min_idx] {
            min_idx = i;
        }
    }
    cycle.rotate_left(min_idx);
}

// ---------------------------------------------------------------------------
// 단위 테스트
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn g_with_parent(id: &str, parent: Option<&str>) -> Group {
        let mut g = Group::new(id, "alliance", id);
        g.parent_group = parent.map(GroupId::new);
        g
    }

    #[test]
    fn group_new_sets_defaults() {
        let g = Group::new("group-test", "alliance", "테스트");
        assert_eq!(g.id.as_str(), "group-test");
        assert_eq!(g.kind, "alliance");
        assert_eq!(g.name, "테스트");
        assert!(g.aliases.is_empty());
        assert_eq!(g.temporal.status, GroupStatus::Active);
        assert!(g.parent_group.is_none());
    }

    #[test]
    fn temporal_serde_roundtrip() {
        let t = Temporal {
            founded_at: Some("원년 (270년 전)".into()),
            dissolved_at: None,
            status: GroupStatus::Declining,
            notes: Some("v1.0 메모".into()),
        };
        let json = serde_json::to_string(&t).unwrap();
        let back: Temporal = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
        // skip_serializing_if 검증
        assert!(!json.contains("dissolved_at"));
        assert!(json.contains("\"declining\""));
    }

    #[test]
    fn group_status_from_str_loose() {
        assert_eq!(
            GroupStatus::from_str_loose("Active"),
            Some(GroupStatus::Active)
        );
        assert_eq!(
            GroupStatus::from_str_loose("  declining  "),
            Some(GroupStatus::Declining)
        );
        assert_eq!(GroupStatus::from_str_loose("invalid"), None);
    }

    #[test]
    fn cycle_detection_finds_self_loop() {
        let groups = vec![g_with_parent("group-a", Some("group-a"))];
        let cycles = detect_parent_group_cycle(&groups);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0], vec![GroupId::new("group-a")]);
    }

    #[test]
    fn cycle_detection_finds_two_node_cycle() {
        let groups = vec![
            g_with_parent("group-a", Some("group-b")),
            g_with_parent("group-b", Some("group-a")),
        ];
        let cycles = detect_parent_group_cycle(&groups);
        assert_eq!(cycles.len(), 1);
        // canonical rotation: 가장 작은 id에서 시작
        assert_eq!(cycles[0], vec![GroupId::new("group-a"), GroupId::new("group-b")]);
    }

    #[test]
    fn cycle_detection_clean_chain_returns_empty() {
        // shipsangsi → daejin-court (no cycle)
        let groups = vec![
            g_with_parent("group-shipsangsi", Some("group-daejin-court")),
            g_with_parent("group-daejin-court", None),
        ];
        let cycles = detect_parent_group_cycle(&groups);
        assert!(cycles.is_empty());
    }

    #[test]
    fn cycle_detection_finds_three_node_cycle() {
        // a → b → c → a — canonical rotation: 가장 작은 id 'a' 시작.
        let groups = vec![
            g_with_parent("group-a", Some("group-b")),
            g_with_parent("group-b", Some("group-c")),
            g_with_parent("group-c", Some("group-a")),
        ];
        let cycles = detect_parent_group_cycle(&groups);
        assert_eq!(cycles.len(), 1);
        assert_eq!(
            cycles[0],
            vec![
                GroupId::new("group-a"),
                GroupId::new("group-b"),
                GroupId::new("group-c"),
            ]
        );
    }

    #[test]
    fn cycle_detection_finds_two_disjoint_cycles() {
        // (a↔b) + (c↔d) — 두 개의 독립 cycle. 둘 다 검출되며 BTreeSet으로 정렬 보장.
        let groups = vec![
            g_with_parent("group-a", Some("group-b")),
            g_with_parent("group-b", Some("group-a")),
            g_with_parent("group-c", Some("group-d")),
            g_with_parent("group-d", Some("group-c")),
        ];
        let cycles = detect_parent_group_cycle(&groups);
        assert_eq!(cycles.len(), 2);
        assert_eq!(
            cycles[0],
            vec![GroupId::new("group-a"), GroupId::new("group-b")]
        );
        assert_eq!(
            cycles[1],
            vec![GroupId::new("group-c"), GroupId::new("group-d")]
        );
    }

    #[test]
    fn cycle_detection_tail_cycle_excludes_prefix() {
        // a → b → c → b — a는 cycle 외부 prefix. cycle은 {b, c}만 포함되어야 함.
        let groups = vec![
            g_with_parent("group-a", Some("group-b")),
            g_with_parent("group-b", Some("group-c")),
            g_with_parent("group-c", Some("group-b")),
        ];
        let cycles = detect_parent_group_cycle(&groups);
        assert_eq!(cycles.len(), 1);
        // canonical: 가장 작은 'group-b' 시작.
        assert_eq!(
            cycles[0],
            vec![GroupId::new("group-b"), GroupId::new("group-c")]
        );
        assert!(!cycles[0].contains(&GroupId::new("group-a")));
    }

    #[test]
    fn cycle_detection_dangling_parent_is_not_cycle() {
        // 외래키 결손 (Phase 2까지 자연스러운 상태) — cycle 아님.
        let groups = vec![g_with_parent("group-a", Some("group-missing"))];
        let cycles = detect_parent_group_cycle(&groups);
        assert!(cycles.is_empty());
    }

    #[test]
    fn alignment_extracts_from_extras() {
        let mut g = Group::new("group-test", "alliance", "test");
        g.extras
            .insert("alignment".into(), Value::String("orthodox".into()));
        assert_eq!(g.alignment(), Some("orthodox"));

        let g_none = Group::new("group-empty", "alliance", "empty");
        assert_eq!(g_none.alignment(), None);
    }

    #[test]
    fn group_full_serde_roundtrip() {
        let mut g = Group::new("group-x", "clan", "X 가문");
        g.aliases = vec!["X문".into()];
        g.summary = "테스트 그룹".into();
        g.temporal.status = GroupStatus::Active;
        g.parent_group = Some(GroupId::new("group-y"));
        g.allied_groups = vec![GroupId::new("group-z")];
        g.extras
            .insert("alignment".into(), Value::String("orthodox".into()));
        g.body_sections.insert("개요".into(), "산문".into());

        let json = serde_json::to_string(&g).unwrap();
        let back: Group = serde_json::from_str(&json).unwrap();
        assert_eq!(back, g);
    }
}
