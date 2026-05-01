//! Worldbuilding ↔ npc-mind 통합 — `Person` → `npc_mind::Npc` 변환.
//!
//! Phase 2 §3.5에서 결정: world-load 후 mind-studio가 `kind in {"active","player"}`
//! 인 Person을 인메모리 `MindRepository`(`InMemoryRepository`)에 자동 등록한다.
//! 같은 NpcId가 이미 있으면 personality·name만 갱신, emotion_state·scene·memory는
//! 보존하는 정책 — Phase 2엔 `add_npc(HashMap insert)`가 이 의미를 만족한다
//! (per CLAUDE.md, mind-studio의 dynamic 상태는 별도 store에 저장).
//!
//! HEXACO 매핑 정책:
//! - `Person.hexaco`(6 dim, Score VO)를 그대로 4 facet에 spread.
//! - 24 facet 정형 보존(`extras.hexaco_facets`)은 **Step 4(체크포인트 2) 이후**의
//!   확장 — 현재 변환은 6 dim → 4 facet copy로 단순.

use crate::domain::personality::{
    Agreeableness, Conscientiousness, Emotionality, Extraversion, HexacoProfile,
    HonestyHumility, Npc, Openness, Score,
};
use crate::domain::world::{HexacoSix, Person};

/// `Person` → `Npc` 변환. `kind`가 `active`/`player`가 아니면 None 반환 — 호출자가
/// 묵시적으로 스킵하도록 함.
///
/// description은 Person.summary를 그대로 사용. 비어 있으면 빈 문자열.
pub fn person_to_npc(person: &Person) -> Option<Npc> {
    if !person.is_mind_eligible() {
        return None;
    }
    let personality = hexaco_six_to_profile(&person.hexaco);
    Some(Npc::new(
        person.id.as_str(),
        person.name.clone(),
        person.summary.clone(),
        personality,
    ))
}

/// HexacoSix(6 dim) → HexacoProfile(6 dim × 4 facet).
///
/// 각 facet에 동일한 dim 값을 복사한다. 24 facet을 따로 명시하고 싶으면
/// `Person.extras["hexaco_facets"]`를 사용한 별도 변환을 도입할 것 (Phase 2+).
pub fn hexaco_six_to_profile(h: &HexacoSix) -> HexacoProfile {
    let spread = |s: Score| s; // alias for clarity below
    HexacoProfile {
        honesty_humility: HonestyHumility {
            sincerity: spread(h.honesty_humility),
            fairness: spread(h.honesty_humility),
            greed_avoidance: spread(h.honesty_humility),
            modesty: spread(h.honesty_humility),
        },
        emotionality: Emotionality {
            fearfulness: spread(h.emotionality),
            anxiety: spread(h.emotionality),
            dependence: spread(h.emotionality),
            sentimentality: spread(h.emotionality),
        },
        extraversion: Extraversion {
            social_self_esteem: spread(h.extraversion),
            social_boldness: spread(h.extraversion),
            sociability: spread(h.extraversion),
            liveliness: spread(h.extraversion),
        },
        agreeableness: Agreeableness {
            forgiveness: spread(h.agreeableness),
            gentleness: spread(h.agreeableness),
            flexibility: spread(h.agreeableness),
            patience: spread(h.agreeableness),
        },
        conscientiousness: Conscientiousness {
            organization: spread(h.conscientiousness),
            diligence: spread(h.conscientiousness),
            perfectionism: spread(h.conscientiousness),
            prudence: spread(h.conscientiousness),
        },
        openness: Openness {
            aesthetic_appreciation: spread(h.openness),
            inquisitiveness: spread(h.openness),
            creativity: spread(h.openness),
            unconventionality: spread(h.openness),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jogo_like() -> Person {
        let mut p = Person::new("npc-02", "active", "조고");
        p.summary = "메인 적대자 — 대진의 그림자.".into();
        p.hexaco = HexacoSix {
            honesty_humility: Score::clamped(-0.8),
            emotionality: Score::clamped(-0.3),
            extraversion: Score::clamped(-0.2),
            agreeableness: Score::clamped(-0.7),
            conscientiousness: Score::clamped(0.7),
            openness: Score::clamped(0.5),
        };
        p
    }

    #[test]
    fn active_person_converts_to_npc() {
        let p = jogo_like();
        let npc = person_to_npc(&p).expect("active kind should produce Npc");
        assert_eq!(npc.id(), "npc-02");
        assert_eq!(npc.name(), "조고");
        assert_eq!(npc.description(), "메인 적대자 — 대진의 그림자.");

        let prof = npc.personality();
        // 6 dim 평균이 입력값과 일치 (모든 facet에 같은 값을 spread했으므로).
        let avg = prof.dimension_averages();
        assert!((avg.h.value() - -0.8).abs() < 1e-6);
        assert!((avg.e.value() - -0.3).abs() < 1e-6);
        assert!((avg.x.value() - -0.2).abs() < 1e-6);
        assert!((avg.a.value() - -0.7).abs() < 1e-6);
        assert!((avg.c.value() - 0.7).abs() < 1e-6);
        assert!((avg.o.value() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn historical_kind_returns_none() {
        let mut p = Person::new("H01", "historical", "진천명");
        p.summary = "270년 전 태조".into();
        assert!(person_to_npc(&p).is_none());
    }

    #[test]
    fn legendary_kind_returns_none() {
        let p = Person::new("L01", "legendary", "독고선");
        assert!(person_to_npc(&p).is_none());
    }

    #[test]
    fn player_kind_converts() {
        let mut p = Person::new("player", "player", "플레이어");
        p.summary = "화산파 유일 생존자".into();
        let npc = person_to_npc(&p).expect("player should convert");
        assert_eq!(npc.id(), "player");
    }

    #[test]
    fn neutral_hexaco_converts_to_neutral_profile() {
        let mut p = Person::new("npc-x", "active", "X");
        p.hexaco = HexacoSix::neutral();
        let npc = person_to_npc(&p).unwrap();
        let avg = npc.personality().dimension_averages();
        assert_eq!(avg.h.value(), 0.0);
        assert_eq!(avg.o.value(), 0.0);
    }

    #[test]
    fn derive_llm_parameters_uses_hexaco() {
        // 조고는 H 매우 낮음 + C 높음 → temperature가 base보다 낮을 것.
        let p = jogo_like();
        let npc = person_to_npc(&p).unwrap();
        let (temp, top_p) = npc.derive_llm_parameters();
        // 범위 체크만 — 정확한 수식은 personality.rs에서.
        assert!(temp.is_finite() && top_p.is_finite());
        assert!(temp > 0.0 && temp < 2.0);
        assert!(top_p > 0.0 && top_p <= 1.0);
    }
}
