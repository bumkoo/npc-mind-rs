// wuxia-core/src/relationship/relationship_type.rs
//
// RelationshipType — 두 캐릭터 사이의 관계 유형.

use serde::{Deserialize, Serialize};

/// 두 캐릭터 사이의 관계 유형.
///
/// MVP에서는 `Option<RelationshipType>`으로 사용하며,
/// 소연↔플레이어는 `None`(미정)으로 시작한다.
/// 게임 진행에 따라 유형이 설정/변경된다.
///
/// # Example
/// ```
/// use wuxia_core::relationship::RelationshipType;
///
/// let rel = RelationshipType::MasterDisciple;
/// assert_eq!(rel.name(), "사제");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationshipType {
    /// 사제 (師弟) — 스승과 제자. 소풍자↔소연.
    MasterDisciple,
    /// 동문 (同門) — 같은 문파의 동료.
    FellowDisciple,
    /// 연인 — 호감80+ 신뢰70+ 조건 충족 시 가능.
    Lover,
    /// 적 — 적대도80+ 시 자동 전환 가능.
    Enemy,
    /// 친구 — 일반적 우호 관계.
    Friend,
    /// 동맹 — 공동 목표를 가진 협력 관계.
    Ally,
    /// 후원자/상관 — 진대인↔소연 같은 거래/상하 관계.
    Patron,
    /// 경쟁자 — 적대는 아니지만 경쟁하는 관계.
    Rival,
}

impl RelationshipType {
    /// 한글 이름 반환 (UI/로깅용).
    pub fn name(&self) -> &'static str {
        match self {
            Self::MasterDisciple => "사제",
            Self::FellowDisciple => "동문",
            Self::Lover => "연인",
            Self::Enemy => "적",
            Self::Friend => "친구",
            Self::Ally => "동맹",
            Self::Patron => "후원자",
            Self::Rival => "경쟁자",
        }
    }
}
