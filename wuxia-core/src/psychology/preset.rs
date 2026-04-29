// wuxia-core/src/psychology/preset.rs
//
// NPC 심리 프리셋 (NPC Psychology Presets)
//
// 6명의 핵심 NPC에 대한 HEXACO 성격, 3축가치관, 5가치 초기값.
// 설계 문서(wuxia-npc-psychology-architecture.md §12)에서 추출.
//
// 프로필 요약:
//   명경  H90 E50 X50 A80 C90 O60 — "의로운 스승"
//   조고  H10 E20 X80 A10 C80 O50 — "냉혈 야심가"
//   소연  H50 E60 X60 A40 C70 O70 — "신중한 복수자"
//   야율설화 H40 E30 X70 A50 C40 O80 — "자유로운 유목민"
//   진야림  H60 E40 X30 A60 C30 O50 — "부서진 검객"
//   남궁현  H40 E50 X70 A30 C70 O60 — "야심찬 이인자"

use crate::shared::id::CharacterId;

use super::personality::HexacoPersonality;
use super::three_axis::{ThreeAxisValues, ValueAxis};
use super::values::PracticalValues;

// ---------------------------------------------------------------------------
// HEXACO 성격 프리셋
// ---------------------------------------------------------------------------

/// 명경 (明經, Sai Tai) — 아미파 장문인.
/// H90 E50 X50 A80 C90 O60
pub fn myungkyung_personality(id: CharacterId) -> HexacoPersonality {
    HexacoPersonality::new(id, 90, 50, 50, 80, 90, 60)
}

/// 조고 (趙高) — 황제의 그림자.
/// H10 E20 X80 A10 C80 O50
pub fn jogo_personality(id: CharacterId) -> HexacoPersonality {
    HexacoPersonality::new(id, 10, 20, 80, 10, 80, 50)
}

/// 소연 (素燕) — 개방 정보원.
/// H50 E60 X60 A40 C70 O70
pub fn soyeon_personality(id: CharacterId) -> HexacoPersonality {
    HexacoPersonality::new(id, 50, 60, 60, 40, 70, 70)
}

/// 야율설화 (耶律雪花) — 초원 공주.
/// H40 E30 X70 A50 C40 O80
pub fn yalul_personality(id: CharacterId) -> HexacoPersonality {
    HexacoPersonality::new(id, 40, 30, 70, 50, 40, 80)
}

/// 진야림 (陳夜林) — 부서진 검객.
/// H60 E40 X30 A60 C30 O50
pub fn jinya_personality(id: CharacterId) -> HexacoPersonality {
    HexacoPersonality::new(id, 60, 40, 30, 60, 30, 50)
}

/// 남궁현 (南宮賢) — 야심찬 이인자.
/// H40 E50 X70 A30 C70 O60
pub fn namgung_personality(id: CharacterId) -> HexacoPersonality {
    HexacoPersonality::new(id, 40, 50, 70, 30, 70, 60)
}

// ---------------------------------------------------------------------------
// 3축가치관 프리셋
// ---------------------------------------------------------------------------

/// 명경 3축: 믿음70 옳음90 바람60
pub fn myungkyung_three_axis(id: CharacterId) -> ThreeAxisValues {
    ThreeAxisValues::new(
        id,
        ValueAxis::new(70.0, "제자를 믿는다".to_string()),
        ValueAxis::new(90.0, "도의를 지켜야 한다".to_string()),
        ValueAxis::new(60.0, "제자들을 지키겠다".to_string()),
    )
}

/// 조고 3축: 믿음20 옳음90 바람95
pub fn jogo_three_axis(id: CharacterId) -> ThreeAxisValues {
    ThreeAxisValues::new(
        id,
        ValueAxis::new(20.0, "사람은 도구다".to_string()),
        ValueAxis::new(90.0, "힘이 곧 정의다".to_string()),
        ValueAxis::new(95.0, "천하를 손에 넣겠다".to_string()),
    )
}

/// 소연 3축: 믿음50 옳음60 바람70
pub fn soyeon_three_axis(id: CharacterId) -> ThreeAxisValues {
    ThreeAxisValues::new(
        id,
        ValueAxis::new(50.0, "사부를 믿는다".to_string()),
        ValueAxis::new(60.0, "강호 도리를 지킨다".to_string()),
        ValueAxis::new(70.0, "원수를 갚고 정보망을 세우겠다".to_string()),
    )
}

/// 야율설화 3축: 믿음40 옳음30 바람75
pub fn yalul_three_axis(id: CharacterId) -> ThreeAxisValues {
    ThreeAxisValues::new(
        id,
        ValueAxis::new(40.0, "사람은 행동으로 증명한다".to_string()),
        ValueAxis::new(30.0, "규칙보다 자유가 중요하다".to_string()),
        ValueAxis::new(75.0, "누구의 도구도 아닌 '나'가 되겠다".to_string()),
    )
}

/// 진야림 3축: 믿음30 옳음50 바람20
pub fn jinya_three_axis(id: CharacterId) -> ThreeAxisValues {
    ThreeAxisValues::new(
        id,
        ValueAxis::new(30.0, "다시 믿기 어렵다".to_string()),
        ValueAxis::new(50.0, "옳은 것은 알지만 힘이 없다".to_string()),
        ValueAxis::new(20.0, "조용히 살고 싶을 뿐이다".to_string()),
    )
}

/// 남궁현 3축: 믿음45 옳음60 바람80
pub fn namgung_three_axis(id: CharacterId) -> ThreeAxisValues {
    ThreeAxisValues::new(
        id,
        ValueAxis::new(45.0, "실력이 증명되면 믿는다".to_string()),
        ValueAxis::new(60.0, "정의는 힘 있는 자가 세우는 것".to_string()),
        ValueAxis::new(80.0, "인정받겠다. 왕좌를 차지하겠다".to_string()),
    )
}

// ---------------------------------------------------------------------------
// 5가치 프리셋
// ---------------------------------------------------------------------------

/// 명경 5가치: 충90 의90 효70 복수30 야망20
pub fn myungkyung_values(id: CharacterId) -> PracticalValues {
    PracticalValues::new(id, 90.0, 90.0, 70.0, 30.0, 20.0)
}

/// 조고 5가치: 충30 의10 효10 복수70 야망90
pub fn jogo_values(id: CharacterId) -> PracticalValues {
    PracticalValues::new(id, 30.0, 10.0, 10.0, 70.0, 90.0)
}

/// 소연 5가치: 충70 의60 효40 복수60 야망40
pub fn soyeon_values(id: CharacterId) -> PracticalValues {
    PracticalValues::new(id, 70.0, 60.0, 40.0, 60.0, 40.0)
}

/// 야율설화 5가치: 충20 의30 효30 복수20 야망50
pub fn yalul_values(id: CharacterId) -> PracticalValues {
    PracticalValues::new(id, 20.0, 30.0, 30.0, 20.0, 50.0)
}

/// 진야림 5가치: 충40 의50 효50 복수10 야망10
pub fn jinya_values(id: CharacterId) -> PracticalValues {
    PracticalValues::new(id, 40.0, 50.0, 50.0, 10.0, 10.0)
}

/// 남궁현 5가치: 충50 의40 효30 복수40 야망80
pub fn namgung_values(id: CharacterId) -> PracticalValues {
    PracticalValues::new(id, 50.0, 40.0, 30.0, 40.0, 80.0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(n: u64) -> CharacterId {
        CharacterId::new(n)
    }

    // -- HEXACO 프로필 검증 --

    #[test]
    fn myungkyung_hexaco_profile() {
        let p = myungkyung_personality(cid(1));
        assert_eq!(p.h(), 90);
        assert_eq!(p.e(), 50);
        assert_eq!(p.x(), 50);
        assert_eq!(p.a(), 80);
        assert_eq!(p.c(), 90);
        assert_eq!(p.o(), 60);
    }

    #[test]
    fn jogo_hexaco_profile() {
        let p = jogo_personality(cid(2));
        assert_eq!(p.h(), 10);
        assert_eq!(p.e(), 20);
        assert_eq!(p.x(), 80);
        assert_eq!(p.a(), 10);
        assert_eq!(p.c(), 80);
        assert_eq!(p.o(), 50);
    }

    #[test]
    fn soyeon_hexaco_profile() {
        let p = soyeon_personality(cid(3));
        assert_eq!(p.h(), 50);
        assert_eq!(p.e(), 60);
        assert_eq!(p.x(), 60);
        assert_eq!(p.a(), 40);
        assert_eq!(p.c(), 70);
        assert_eq!(p.o(), 70);
    }

    #[test]
    fn yalul_hexaco_profile() {
        let p = yalul_personality(cid(4));
        assert_eq!(p.h(), 40);
        assert_eq!(p.e(), 30);
        assert_eq!(p.x(), 70);
        assert_eq!(p.a(), 50);
        assert_eq!(p.c(), 40);
        assert_eq!(p.o(), 80);
    }

    #[test]
    fn jinya_hexaco_profile() {
        let p = jinya_personality(cid(5));
        assert_eq!(p.h(), 60);
        assert_eq!(p.e(), 40);
        assert_eq!(p.x(), 30);
        assert_eq!(p.a(), 60);
        assert_eq!(p.c(), 30);
        assert_eq!(p.o(), 50);
    }

    #[test]
    fn namgung_hexaco_profile() {
        let p = namgung_personality(cid(6));
        assert_eq!(p.h(), 40);
        assert_eq!(p.e(), 50);
        assert_eq!(p.x(), 70);
        assert_eq!(p.a(), 30);
        assert_eq!(p.c(), 70);
        assert_eq!(p.o(), 60);
    }

    // -- 3축가치관 프리셋 --

    #[test]
    fn myungkyung_three_axis_profile() {
        let v = myungkyung_three_axis(cid(1));
        assert_eq!(v.trust().intensity(), 70.0);
        assert_eq!(v.rightness().intensity(), 90.0);
        assert_eq!(v.want().intensity(), 60.0);
        assert_eq!(v.rightness().creed(), "도의를 지켜야 한다");
    }

    #[test]
    fn jogo_three_axis_same_rightness_different_creed() {
        // 핵심: 조고와 명경의 옳음(正)이 모두 90이지만 신조가 반대
        let m = myungkyung_three_axis(cid(1));
        let j = jogo_three_axis(cid(2));
        assert_eq!(m.rightness().intensity(), 90.0);
        assert_eq!(j.rightness().intensity(), 90.0);
        assert_ne!(m.rightness().creed(), j.rightness().creed());
    }

    // -- 5가치 프리셋 --

    #[test]
    fn myungkyung_values_profile() {
        let v = myungkyung_values(cid(1));
        assert_eq!(v.loyalty(), 90.0);
        assert_eq!(v.righteousness(), 90.0);
        assert_eq!(v.filial_piety(), 70.0);
        assert_eq!(v.vengeance(), 30.0);
        assert_eq!(v.ambition(), 20.0);
    }

    #[test]
    fn jogo_values_opposite_alignment() {
        let m = myungkyung_values(cid(1));
        let j = jogo_values(cid(2));
        assert!(m.alignment() > 0.0, "명경: 의로운 방향");
        assert!(j.alignment() < 0.0, "조고: 야망/복수 방향");
    }

    #[test]
    fn jogo_high_betrayal_potential() {
        let j = jogo_values(cid(2));
        let m = myungkyung_values(cid(1));
        assert!(j.betrayal_potential() > m.betrayal_potential(),
            "조고의 배신 가능성이 명경보다 높다");
    }

    #[test]
    fn jinya_low_ambition_low_betrayal() {
        let j = jinya_values(cid(5));
        assert!(j.betrayal_potential() < 0.1, "진야림: 야망 낮아 배신 가능성 매우 낮음");
    }
}
