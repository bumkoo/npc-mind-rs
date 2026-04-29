// wuxia-core/src/relationship/level.rs
//
// RelationshipLevel — 두 캐릭터 사이의 관계 깊이 (복합 판정).

use serde::{Deserialize, Serialize};

/// 두 캐릭터 사이의 관계 깊이.
///
/// 음수 호감도가 적대를 결정한다.
/// "소연이 플레이어를 적으로 인식하면, 과거 호감은 의미 없다."
///
/// ```text
///   affinity <= -80                     → Enemy
///   affinity <= -40                     → Hostile
///   affinity <= -10                     → Wary
///   affinity >= 80 AND trust >= 70      → Intimate   (진짜 소연)
///   affinity >= 70 AND trust >= 50      → Close      (사부의 부탁)
///   affinity >= 50 AND trust >= 30      → Friendly   (개방 언급)
///   affinity >= 20 OR  trust >= 20      → Acquaintance (거래 파트너)
///   else                                → Stranger
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationshipLevel {
    /// 원수. 개방의 적. 호감도 -80 이하.
    Enemy,
    /// 적대. 경계 대상. 호감도 -40 이하.
    Hostile,
    /// 경계. 약간의 불편. 호감도 -10 이하.
    Wary,
    /// 모르는 사이. 첫 만남 이전.
    Stranger,
    /// 아는 사이. 이름 정도. 소연 호감도 20+.
    Acquaintance,
    /// 친근한 사이. 개인사를 나눔. 소연 호감도 50+ 신뢰 30+.
    Friendly,
    /// 가까운 사이. 비밀을 공유. 소연 호감도 70+ 신뢰 50+.
    Close,
    /// 깊은 유대. 목숨을 맡길 수 있음. 소연 호감도 80+ 신뢰 70+.
    Intimate,
}

impl RelationshipLevel {
    /// 한글 이름 반환 (UI/로깅용).
    pub fn name(&self) -> &'static str {
        match self {
            Self::Enemy => "원수",
            Self::Hostile => "적대",
            Self::Wary => "경계",
            Self::Stranger => "모르는 사이",
            Self::Acquaintance => "아는 사이",
            Self::Friendly => "친근",
            Self::Close => "가까운 사이",
            Self::Intimate => "깊은 유대",
        }
    }

    /// 설정 파일 조회용 키. descriptions.toml의 [relationship_level.X]와 매칭.
    pub fn key(&self) -> &'static str {
        match self {
            Self::Enemy => "Enemy",
            Self::Hostile => "Hostile",
            Self::Wary => "Wary",
            Self::Stranger => "Stranger",
            Self::Acquaintance => "Acquaintance",
            Self::Friendly => "Friendly",
            Self::Close => "Close",
            Self::Intimate => "Intimate",
        }
    }
}
