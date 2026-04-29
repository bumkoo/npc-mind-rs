// wuxia-core/src/test_fixtures.rs
//
// Shared Test Fixtures — 테스트용 공통 헬퍼 모음.
//
// 여러 도메인 테스트에서 반복되는 make_character(), make_rel() 등을
// 한 곳에 모아 중복을 제거한다.
//
// `#[cfg(test)]`이므로 프로덕션 빌드에는 포함되지 않는다.

use crate::character::{Character, CharacterRole, Gender};
use crate::psychology::{
    HexacoPersonality, PadState, PracticalValues, ThreeAxisValues, ValueAxis,
};
use crate::relationship::Relationship;
use crate::shared::id::{CharacterId, RelationshipId};

/// 테스트용 캐릭터 생성 (전체 파라미터).
///
/// `birth_year`는 `1200 - age`로 자동 계산한다.
pub fn make_character(id: u64, name: &str, age: u32, role: CharacterRole) -> Character {
    Character::new(
        CharacterId::new(id),
        name.to_string(),
        None,
        Gender::Male,
        1200 - age,
        age,
        role,
    )
}

/// 테스트용 캐릭터 생성 (간략 버전).
///
/// 이름 "테스트", 나이 25, NPC 역할로 생성한다.
pub fn make_default_character(id: u64) -> Character {
    make_character(id, "테스트", 25, CharacterRole::Npc)
}

/// 테스트용 관계 생성.
pub fn make_relationship(id: u64, source_id: u64, target_id: u64) -> Relationship {
    Relationship::new(
        RelationshipId::new(id),
        CharacterId::new(source_id),
        CharacterId::new(target_id),
    )
}

// ---------------------------------------------------------------------------
// Psychology test fixtures
// ---------------------------------------------------------------------------

/// 테스트용 HEXACO 성격 생성 (전체 파라미터).
pub fn make_hexaco(id: u64, h: u32, e: u32, x: u32, a: u32, c: u32, o: u32) -> HexacoPersonality {
    HexacoPersonality::new(CharacterId::new(id), h, e, x, a, c, o)
}

/// 테스트용 기본 심리 프로필 (중간값).
///
/// HEXACO: 모두 50, 3축: 모두 50.0, 5가치: 모두 50.0
pub fn make_default_psyche(
    id: u64,
) -> (
    HexacoPersonality,
    ThreeAxisValues,
    PracticalValues,
    PadState,
) {
    let personality = HexacoPersonality::new(CharacterId::new(id), 50, 50, 50, 50, 50, 50);
    let three_axis = ThreeAxisValues::new(
        CharacterId::new(id),
        ValueAxis::new(50.0, "기본 믿음".to_string()),
        ValueAxis::new(50.0, "기본 옳음".to_string()),
        ValueAxis::new(50.0, "기본 바람".to_string()),
    );
    let values = PracticalValues::new(CharacterId::new(id), 50.0, 50.0, 50.0, 50.0, 50.0);
    let mood = PadState::neutral();
    (personality, three_axis, values, mood)
}

/// 테스트용 명경 심리 프로필.
///
/// H90 E50 X50 A80 C90 O60 — 도덕적이고 성실한 성격.
pub fn make_myungkyung_psyche(
    id: u64,
) -> (
    HexacoPersonality,
    ThreeAxisValues,
    PracticalValues,
    PadState,
) {
    let personality = HexacoPersonality::new(CharacterId::new(id), 90, 50, 50, 80, 90, 60);
    let three_axis = ThreeAxisValues::new(
        CharacterId::new(id),
        ValueAxis::new(80.0, "사람을 믿는다".to_string()),
        ValueAxis::new(90.0, "도의를 지켜야 한다".to_string()),
        ValueAxis::new(50.0, "제자들을 지키겠다".to_string()),
    );
    let values = PracticalValues::new(CharacterId::new(id), 90.0, 90.0, 70.0, 30.0, 20.0);
    let mood = PadState::neutral();
    (personality, three_axis, values, mood)
}
