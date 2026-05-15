//! OCC 감정 → 4축 매핑.
//!
//! relationships.md v0.7 §4.1~4.3 + Phase 2 Stage 2 결정 (B-D5/D6/D12/D14).
//!
//! - `base_delta(EmotionType) -> AxisDelta` — 12 OCC × 4축 = 48셀 lookup
//! - `hexaco_modifier(EmotionType, &HexacoProfile) -> AxisModifier` — 6 보정 룰
//! - `update_axes_from_emotion(rel, emotion, intensity, hexaco)` — 단일 진입점
//!
//! BondStatus 차단 가드는 본 모듈 진입점에서, B-D12 (Pride/Shame agent_id=None) 가드는
//! *호출 측* (RelationshipPolicy / StimulusPolicy).

use crate::domain::emotion::EmotionType;
use crate::domain::personality::HexacoProfile;
use crate::domain::relationship::axis::{AxisDelta, AxisKind, AxisModifier};
use crate::domain::relationship::Relationship;

// ---------------------------------------------------------------------------
// HEXACO 보정 룰 임계값
// ---------------------------------------------------------------------------

/// HEXACO facet "높음" 임계 (Phase 2 Stage 2 결정 α: 0.5).
/// 중립(0.0)과 최대(±1.0)의 *중간*. Phase 2.3 시뮬레이션 검증 후 미세조정 예정.
const HIGH_THRESHOLD: f32 = 0.5;

/// HEXACO facet "낮음" 임계.
const LOW_THRESHOLD: f32 = -0.5;

// ---------------------------------------------------------------------------
// base_delta — 48셀 lookup
// ---------------------------------------------------------------------------

/// OCC 감정 → 4축 base 변동 (intensity/HEXACO 곱셈 *전*).
///
/// relationships.md v0.7 §4.2 — 12 OCC × 4축 = 48셀.
/// Well-being (Joy/Distress) + Prospect (Hope/Fear/Satisfaction/Disappointment/Relief/FearsConfirmed)
/// + Compound 보조 (Remorse/Gratification) **10 OCC는 default (0)** — B-D14 의도된 누락.
///
/// 매핑 안된 감정 입력 시 `AxisDelta::default()` (모두 0) 반환.
pub(crate) fn base_delta(emotion: EmotionType) -> AxisDelta {
    match emotion {
        // ── 지각·평가 4 (대상 외부) ─────
        EmotionType::Gratitude => AxisDelta {
            trust: 20.0,
            affinity: 10.0,
            respect: 0.0,
            wariness: -10.0,
        },
        EmotionType::Anger => AxisDelta {
            trust: -25.0,
            affinity: -10.0,
            respect: 0.0,
            wariness: 25.0,
        },
        EmotionType::Admiration => AxisDelta {
            trust: 0.0,
            affinity: 0.0,
            respect: 20.0,
            wariness: 0.0,
        },
        EmotionType::Reproach => AxisDelta {
            trust: -10.0,
            affinity: -10.0,
            respect: -25.0,
            wariness: 10.0,
        },

        // ── 공감 4 (Fortune-of-others) ─────
        EmotionType::HappyFor => AxisDelta {
            trust: 5.0,
            affinity: 10.0,
            respect: 0.0,
            wariness: 0.0,
        },
        EmotionType::Resentment => AxisDelta {
            trust: 0.0,
            affinity: -10.0,
            respect: -5.0,
            wariness: 15.0,
        },
        EmotionType::Pity => AxisDelta {
            trust: 0.0,
            affinity: 10.0,
            respect: -5.0,
            wariness: 0.0,
        },
        EmotionType::Gloating => AxisDelta {
            trust: -10.0,
            affinity: -20.0,
            respect: -10.0,
            wariness: 0.0,
        },

        // ── 자기 평가 2 (B-D12: agent_id=None 시 호출 측에서 가드, base 표값은 박힘) ─────
        EmotionType::Pride => AxisDelta {
            trust: 0.0,
            affinity: 5.0,
            respect: 10.0,
            wariness: 0.0,
        },
        EmotionType::Shame => AxisDelta {
            trust: -5.0,
            affinity: -10.0,
            respect: -10.0,
            wariness: 5.0,
        },

        // ── 대상 평가 2 (Object) ─────
        EmotionType::Love => AxisDelta {
            trust: 5.0,
            affinity: 20.0,
            respect: 5.0,
            wariness: -5.0,
        },
        EmotionType::Hate => AxisDelta {
            trust: -10.0,
            affinity: -25.0,
            respect: -5.0,
            wariness: 15.0,
        },

        // ── 매핑 안된 10 OCC (B-D14 의도된 누락) ─────
        // Joy / Distress (Well-being)
        // Hope / Fear / Satisfaction / Disappointment / Relief / FearsConfirmed (Prospect)
        // Remorse / Gratification (Compound — Pride/Shame 합산은 별 OCC로 자동 식별)
        _ => AxisDelta::default(),
    }
}

// ---------------------------------------------------------------------------
// hexaco_modifier — 6 보정 룰
// ---------------------------------------------------------------------------

/// HEXACO 6 facet → 4축 곱셈 배수.
///
/// relationships.md v0.7 §4.3 — 6 보정 룰 (5 활성 + Unconventionality placeholder).
///
/// `emotion` 인자는 *A- Forgiveness 부정감정 한정* 룰에 사용.
pub(crate) fn hexaco_modifier(
    emotion: EmotionType,
    hexaco: &HexacoProfile,
) -> AxisModifier {
    let mut m = AxisModifier::default();

    // ── H+ Sincerity 높음 → trust 변화 ×1.2 ─────
    if hexaco.honesty_humility.sincerity.value() > HIGH_THRESHOLD {
        m = m.scale_axis(AxisKind::Trust, 1.2);
    }

    // ── A+ Patience 높음 → 모든 변화 ×0.7 ─────
    if hexaco.agreeableness.patience.value() > HIGH_THRESHOLD {
        m = m.combine_uniform(0.7);
    }

    // ── A- Forgiveness 낮음 → 부정 감정 변화 ×1.5 ─────
    if hexaco.agreeableness.forgiveness.value() < LOW_THRESHOLD && is_negative_emotion(emotion) {
        m = m.combine_uniform(1.5);
    }

    // ── E+ Anxiety 높음 → wariness 변화 ×1.3 ─────
    if hexaco.emotionality.anxiety.value() > HIGH_THRESHOLD {
        m = m.scale_axis(AxisKind::Wariness, 1.3);
    }

    // ── C+ Prudence 높음 → 모든 변화 ×0.8 (Stage 2 간소화) ─────
    // v0.7 "큰 변화 시 ×0.8, 시간 분산"은 Stage 2 본체에서 *간소 곱셈*.
    // intensity 조건부 + 시간 분산은 Phase 2.3에서 정밀화.
    if hexaco.conscientiousness.prudence.value() > HIGH_THRESHOLD {
        m = m.combine_uniform(0.8);
    }

    // ── O+ Unconventionality 높음 → 양극 도달 더 쉬움 (placeholder, 적용 0) ─────
    // v0.7 "양극 도달 가속"은 clamp 근처에서만 의미. 단순 곱셈으로 표현 어려움.
    // Phase 2.3 또는 3+에서 정밀화.

    m
}

/// 부정 감정 식별 — A− Forgiveness 룰 적용 조건.
///
/// **정의** (relationships.md v0.7 §4.3 + Phase 2 Stage 2 결정):
/// *OCC valence 자체가 부정인 감정* — Distress/Fear/Disappointment/FearsConfirmed (Well-being/Prospect 부정)
/// + Anger/Reproach/Resentment/Gloating/Hate (대상 부정 평가) + Shame/Remorse (자기 부정).
/// 11종.
///
/// **Pity 제외**: Pity는 *공감 4 (Fortune-of-others)* 군의 *연민*이며 OCC valence는 부정이지만
/// base_delta는 `{affinity +10, respect −5, wariness 0}`로 **affinity 긍정**이 주된 효과.
/// "용서 어려운 캐릭터가 *연민*을 더 강하게 느낀다"는 의미가 게임 narrative상 부자연 →
/// 부정 감정 분류에서 제외 (spec §4.3 본문이 모호하므로 본 구현이 기준점).
/// Phase 2.3 narrative 검증에서 *공감 군 4의 ×1.5 적용 여부* 시뮬로 재확인 예정.
fn is_negative_emotion(emotion: EmotionType) -> bool {
    matches!(
        emotion,
        EmotionType::Anger
            | EmotionType::Reproach
            | EmotionType::Resentment
            | EmotionType::Gloating
            | EmotionType::Hate
            | EmotionType::Distress
            | EmotionType::Fear
            | EmotionType::Disappointment
            | EmotionType::FearsConfirmed
            | EmotionType::Shame
            | EmotionType::Remorse
    )
}

// ---------------------------------------------------------------------------
// update_axes_from_emotion — 단일 진입점
// ---------------------------------------------------------------------------

/// OCC 감정 → 4축 변동 통합 적용.
///
/// 흐름 (v0.7 §4.1):
///   1. BondStatus 차단 (`accepts_live_input` false면 즉시 종료)
///   2. `base_delta(emotion)` lookup
///   3. `intensity × hexaco_modifier(emotion, hexaco)` 곱셈
///   4. `rel.apply_delta(&delta)` — 4축 자동 clamp
///
/// **B-D12 (Pride/Shame agent_id=None) 가드는 *호출 측*** (RelationshipPolicy / StimulusPolicy).
/// 본 함수의 책임은 *상대 관계 4축 갱신* 일관.
///
/// ## 호출자 인덱스 (B-D12 가드 *필수* 위치)
///
/// 본 함수를 *새 위치에서* 호출할 때는 반드시 다음 패턴을 함께 박을 것:
/// ```rust,ignore
/// // B-D12 guard: Pride/Shame are self-emotions, no target-relationship semantics.
/// // If this loop is duplicated to a new caller, this guard MUST be copied.
/// if matches!(emotion_type, EmotionType::Pride | EmotionType::Shame) {
///     continue;
/// }
/// update_axes_from_emotion(&mut rel, emotion_type, intensity, hexaco);
/// ```
///
/// 현재 호출자 (4번째 추가 시 본 리스트 갱신 + 호출 측 마커 복사):
/// - `application::command::policies::relationship_policy::handle_relationship_update_with_cause`
/// - `application::command::policies::relationship_policy::handle_dialogue_end`
/// - `application::command::policies::stimulus_policy::process_beat_transition`
///
/// 회고 §W4 + spec §7 참조.
pub fn update_axes_from_emotion(
    rel: &mut Relationship,
    emotion: EmotionType,
    intensity: f32,
    hexaco: &HexacoProfile,
) {
    // ── 가드: BondStatus 차단 ─────
    if !rel.bond_status().accepts_live_input() {
        return;
    }

    // ── base_delta × intensity × hexaco_modifier ─────
    let base = base_delta(emotion);
    let modulator = hexaco_modifier(emotion, hexaco);
    let delta = AxisDelta {
        trust: base.trust * intensity * modulator.trust,
        affinity: base.affinity * intensity * modulator.affinity,
        respect: base.respect * intensity * modulator.respect,
        wariness: base.wariness * intensity * modulator.wariness,
    };

    rel.apply_delta(&delta);
}

// ===========================================================================
// 단위 테스트 — Stage 2.7
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::personality::{HexacoProfile, Score};
    use crate::domain::relationship::axis::{AxisScore, WarinessScore};
    use crate::domain::relationship::bond::BondStatus;
    use crate::domain::relationship::RelationshipBuilder;
    use crate::domain::tuning::profile;

    // -----------------------------------------------------------------------
    // 2.2 base_delta 48셀 — 12 OCC explicit + 10 default
    // -----------------------------------------------------------------------

    #[test]
    fn base_delta_gratitude() {
        assert_eq!(
            base_delta(EmotionType::Gratitude),
            AxisDelta {
                trust: 20.0,
                affinity: 10.0,
                respect: 0.0,
                wariness: -10.0
            }
        );
    }

    #[test]
    fn base_delta_anger() {
        assert_eq!(
            base_delta(EmotionType::Anger),
            AxisDelta {
                trust: -25.0,
                affinity: -10.0,
                respect: 0.0,
                wariness: 25.0
            }
        );
    }

    #[test]
    fn base_delta_admiration() {
        assert_eq!(
            base_delta(EmotionType::Admiration),
            AxisDelta {
                trust: 0.0,
                affinity: 0.0,
                respect: 20.0,
                wariness: 0.0
            }
        );
    }

    #[test]
    fn base_delta_reproach() {
        assert_eq!(
            base_delta(EmotionType::Reproach),
            AxisDelta {
                trust: -10.0,
                affinity: -10.0,
                respect: -25.0,
                wariness: 10.0
            }
        );
    }

    #[test]
    fn base_delta_happy_for() {
        assert_eq!(
            base_delta(EmotionType::HappyFor),
            AxisDelta {
                trust: 5.0,
                affinity: 10.0,
                respect: 0.0,
                wariness: 0.0
            }
        );
    }

    #[test]
    fn base_delta_resentment() {
        assert_eq!(
            base_delta(EmotionType::Resentment),
            AxisDelta {
                trust: 0.0,
                affinity: -10.0,
                respect: -5.0,
                wariness: 15.0
            }
        );
    }

    #[test]
    fn base_delta_pity() {
        assert_eq!(
            base_delta(EmotionType::Pity),
            AxisDelta {
                trust: 0.0,
                affinity: 10.0,
                respect: -5.0,
                wariness: 0.0
            }
        );
    }

    #[test]
    fn base_delta_gloating() {
        assert_eq!(
            base_delta(EmotionType::Gloating),
            AxisDelta {
                trust: -10.0,
                affinity: -20.0,
                respect: -10.0,
                wariness: 0.0
            }
        );
    }

    #[test]
    fn base_delta_pride() {
        // base 표값은 박혀있음 — agent_id=None 가드는 호출 측 책임.
        assert_eq!(
            base_delta(EmotionType::Pride),
            AxisDelta {
                trust: 0.0,
                affinity: 5.0,
                respect: 10.0,
                wariness: 0.0
            }
        );
    }

    #[test]
    fn base_delta_shame() {
        assert_eq!(
            base_delta(EmotionType::Shame),
            AxisDelta {
                trust: -5.0,
                affinity: -10.0,
                respect: -10.0,
                wariness: 5.0
            }
        );
    }

    #[test]
    fn base_delta_love() {
        assert_eq!(
            base_delta(EmotionType::Love),
            AxisDelta {
                trust: 5.0,
                affinity: 20.0,
                respect: 5.0,
                wariness: -5.0
            }
        );
    }

    #[test]
    fn base_delta_hate() {
        assert_eq!(
            base_delta(EmotionType::Hate),
            AxisDelta {
                trust: -10.0,
                affinity: -25.0,
                respect: -5.0,
                wariness: 15.0
            }
        );
    }

    #[test]
    fn base_delta_well_being_and_prospect_default_to_zero() {
        // B-D14 — Well-being 2 + Prospect 6 + Compound 보조 2 = 10 OCC는 default
        for e in [
            EmotionType::Joy,
            EmotionType::Distress,
            EmotionType::Hope,
            EmotionType::Fear,
            EmotionType::Satisfaction,
            EmotionType::Disappointment,
            EmotionType::Relief,
            EmotionType::FearsConfirmed,
            EmotionType::Remorse,
            EmotionType::Gratification,
        ] {
            assert_eq!(base_delta(e), AxisDelta::default(), "{:?} must default", e);
        }
    }

    #[test]
    fn base_delta_sum_anger_hate_reproach_matches_s2() {
        // Stage 0 §3.6 S2 산신묘 케이스 — 합산값 검증
        let sum = base_delta(EmotionType::Anger)
            + base_delta(EmotionType::Hate)
            + base_delta(EmotionType::Reproach);
        assert_eq!(
            sum,
            AxisDelta {
                trust: -45.0,
                affinity: -45.0,
                respect: -30.0,
                wariness: 50.0,
            }
        );
    }

    // -----------------------------------------------------------------------
    // 2.3 hexaco_modifier — 6 보정 룰 + neutral + is_negative_emotion
    // -----------------------------------------------------------------------

    /// 모든 facet을 동일한 값으로 세팅한 프로필 (룰 단독 검증용).
    fn neutral_hexaco() -> HexacoProfile {
        HexacoProfile::neutral()
    }

    fn score(v: f32) -> Score {
        Score::new(v, "").unwrap()
    }

    #[test]
    fn hexaco_modifier_neutral_is_unit() {
        let m = hexaco_modifier(EmotionType::Anger, &neutral_hexaco());
        assert_eq!(m, AxisModifier::default());
    }

    #[test]
    fn hexaco_modifier_sincerity_high_scales_trust() {
        let mut h = neutral_hexaco();
        h.honesty_humility.sincerity = score(0.7);
        let m = hexaco_modifier(EmotionType::Gratitude, &h);
        assert!((m.trust - 1.2).abs() < 1e-6);
        assert!((m.affinity - 1.0).abs() < 1e-6);
        assert!((m.respect - 1.0).abs() < 1e-6);
        assert!((m.wariness - 1.0).abs() < 1e-6);
    }

    #[test]
    fn hexaco_modifier_patience_high_scales_all_by_0_7() {
        let mut h = neutral_hexaco();
        h.agreeableness.patience = score(0.7);
        let m = hexaco_modifier(EmotionType::Gratitude, &h);
        assert!((m.trust - 0.7).abs() < 1e-6);
        assert!((m.affinity - 0.7).abs() < 1e-6);
        assert!((m.respect - 0.7).abs() < 1e-6);
        assert!((m.wariness - 0.7).abs() < 1e-6);
    }

    #[test]
    fn hexaco_modifier_low_forgiveness_amplifies_negative_emotions() {
        let mut h = neutral_hexaco();
        h.agreeableness.forgiveness = score(-0.7);

        // Anger (부정) — 룰 발동
        let m_anger = hexaco_modifier(EmotionType::Anger, &h);
        assert!((m_anger.trust - 1.5).abs() < 1e-6);
        assert!((m_anger.affinity - 1.5).abs() < 1e-6);

        // Gratitude (긍정) — 룰 미발동
        let m_grat = hexaco_modifier(EmotionType::Gratitude, &h);
        assert_eq!(m_grat, AxisModifier::default());
    }

    #[test]
    fn hexaco_modifier_anxiety_high_scales_wariness() {
        let mut h = neutral_hexaco();
        h.emotionality.anxiety = score(0.7);
        let m = hexaco_modifier(EmotionType::Anger, &h);
        assert!((m.wariness - 1.3).abs() < 1e-6);
        assert!((m.trust - 1.0).abs() < 1e-6);
    }

    #[test]
    fn hexaco_modifier_prudence_high_scales_all_by_0_8() {
        let mut h = neutral_hexaco();
        h.conscientiousness.prudence = score(0.7);
        let m = hexaco_modifier(EmotionType::Anger, &h);
        assert!((m.trust - 0.8).abs() < 1e-6);
        assert!((m.affinity - 0.8).abs() < 1e-6);
        assert!((m.respect - 0.8).abs() < 1e-6);
        assert!((m.wariness - 0.8).abs() < 1e-6);
    }

    #[test]
    fn hexaco_modifier_s2_lin_chong_composite() {
        // Stage 0 §3.6 S2 임충 케이스 — Sincerity 0.7 + Forgiveness -0.7 + Prudence 0.8, Anger
        // 단계: × 1.2 (trust) → × 1.5 (전역) → × 0.8 (전역)
        // 최종: { trust: 1.44, affinity: 1.2, respect: 1.2, wariness: 1.2 }
        let mut h = neutral_hexaco();
        h.honesty_humility.sincerity = score(0.7);
        h.agreeableness.forgiveness = score(-0.7);
        h.conscientiousness.prudence = score(0.8);

        let m = hexaco_modifier(EmotionType::Anger, &h);
        assert!((m.trust - 1.44).abs() < 1e-6, "trust = {}", m.trust);
        assert!((m.affinity - 1.2).abs() < 1e-6, "affinity = {}", m.affinity);
        assert!((m.respect - 1.2).abs() < 1e-6, "respect = {}", m.respect);
        assert!((m.wariness - 1.2).abs() < 1e-6, "wariness = {}", m.wariness);
    }

    #[test]
    fn is_negative_emotion_classifies_11_emotions() {
        for e in [
            EmotionType::Anger,
            EmotionType::Reproach,
            EmotionType::Resentment,
            EmotionType::Gloating,
            EmotionType::Hate,
            EmotionType::Distress,
            EmotionType::Fear,
            EmotionType::Disappointment,
            EmotionType::FearsConfirmed,
            EmotionType::Shame,
            EmotionType::Remorse,
        ] {
            assert!(is_negative_emotion(e), "{:?} must be negative", e);
        }

        for e in [
            EmotionType::Joy,
            EmotionType::Gratitude,
            EmotionType::Admiration,
            EmotionType::HappyFor,
            EmotionType::Pity,
            EmotionType::Pride,
            EmotionType::Love,
            EmotionType::Hope,
            EmotionType::Satisfaction,
            EmotionType::Relief,
            EmotionType::Gratification,
        ] {
            assert!(!is_negative_emotion(e), "{:?} must NOT be negative", e);
        }
    }

    // -----------------------------------------------------------------------
    // 2.4 update_axes_from_emotion — 통합 진입점
    // -----------------------------------------------------------------------

    fn rel_lin_chong_pre_shanshenmiao() -> Relationship {
        RelationshipBuilder::new("lin_chong", "lu_qian")
            .trust(AxisScore::new(50.0))
            .affinity(AxisScore::new(40.0))
            .respect(AxisScore::new(30.0))
            .wariness(WarinessScore::new(5.0))
            .build()
    }

    fn hexaco_lin_chong() -> HexacoProfile {
        let mut h = HexacoProfile::neutral();
        h.honesty_humility.sincerity = score(0.7);
        h.agreeableness.forgiveness = score(-0.7);
        h.conscientiousness.prudence = score(0.8);
        h
    }

    #[test]
    fn update_axes_neutral_hexaco_uses_base_delta_and_intensity() {
        let mut r = Relationship::neutral("a", "b");
        update_axes_from_emotion(&mut r, EmotionType::Gratitude, 1.0, &HexacoProfile::neutral());
        // base_delta(Gratitude) × 1.0 × default = { trust: 20, affinity: 10, respect: 0, wariness: -10 → 0 (floor) }
        assert!((r.trust().value() - 20.0).abs() < 1e-4);
        assert!((r.affinity().value() - 10.0).abs() < 1e-4);
        assert!((r.respect().value() - 0.0).abs() < 1e-4);
        assert!((r.wariness().value() - 0.0).abs() < 1e-4); // WarinessScore floors at 0
    }

    #[test]
    fn update_axes_intensity_zero_is_no_op() {
        let mut r = Relationship::new(
            "a",
            "b",
            AxisScore::new(50.0),
            AxisScore::new(40.0),
            AxisScore::new(30.0),
            WarinessScore::new(5.0),
        );
        update_axes_from_emotion(&mut r, EmotionType::Anger, 0.0, &HexacoProfile::neutral());
        assert!((r.trust().value() - 50.0).abs() < 1e-6);
        assert!((r.affinity().value() - 40.0).abs() < 1e-6);
        assert!((r.respect().value() - 30.0).abs() < 1e-6);
        assert!((r.wariness().value() - 5.0).abs() < 1e-6);
    }

    #[test]
    fn update_axes_s2_lin_chong_anger_alone() {
        // Stage 0 §3.6 S2 + Stage 2 종결 게이트 7번
        // 입력: trust 50, affinity 40, respect 30, wariness 5
        // Anger 0.95, 임충 HEXACO (Sincerity 0.7 / Forgiveness -0.7 / Prudence 0.8)
        // modifier: { trust: 1.44, affinity: 1.2, respect: 1.2, wariness: 1.2 }
        // delta:    trust   -25 * 0.95 * 1.44 = -34.2
        //           affinity -10 * 0.95 * 1.2 = -11.4
        //           respect   0 * ... = 0
        //           wariness 25 * 0.95 * 1.2 = 28.5
        // 결과: trust 15.8 / affinity 28.6 / respect 30 / wariness 33.5
        let mut r = rel_lin_chong_pre_shanshenmiao();
        let h = hexaco_lin_chong();
        update_axes_from_emotion(&mut r, EmotionType::Anger, 0.95, &h);

        assert!(
            (r.trust().value() - 15.8).abs() < 0.1,
            "trust = {} (expected ≈ 15.8)",
            r.trust().value()
        );
        assert!(
            (r.affinity().value() - 28.6).abs() < 0.1,
            "affinity = {} (expected ≈ 28.6)",
            r.affinity().value()
        );
        assert!(
            (r.respect().value() - 30.0).abs() < 0.1,
            "respect = {} (expected ≈ 30)",
            r.respect().value()
        );
        assert!(
            (r.wariness().value() - 33.5).abs() < 0.1,
            "wariness = {} (expected ≈ 33.5)",
            r.wariness().value()
        );
    }

    #[test]
    fn update_axes_clamps_to_axis_bounds() {
        let mut r = Relationship::new(
            "a",
            "b",
            AxisScore::new(95.0),
            AxisScore::NEUTRAL,
            AxisScore::NEUTRAL,
            WarinessScore::new(2.0),
        );
        // Gratitude × 1.0 × default = { trust: +20, affinity: +10, ..., wariness: -10 }
        update_axes_from_emotion(&mut r, EmotionType::Gratitude, 1.0, &HexacoProfile::neutral());
        assert_eq!(r.trust().value(), 100.0); // cap at +100
        assert_eq!(r.wariness().value(), 0.0); // floor at 0
    }

    #[test]
    fn update_axes_bond_status_deceased_blocks_update() {
        let mut r = RelationshipBuilder::new("a", "b")
            .trust(AxisScore::new(50.0))
            .affinity(AxisScore::new(40.0))
            .bond_status(BondStatus::Deceased)
            .build();
        update_axes_from_emotion(&mut r, EmotionType::Anger, 0.95, &HexacoProfile::neutral());
        // 차단되어 변동 없음
        assert_eq!(r.trust().value(), 50.0);
        assert_eq!(r.affinity().value(), 40.0);
    }

    #[test]
    fn update_axes_bond_status_resolved_blocks_update() {
        let mut r = RelationshipBuilder::new("a", "b")
            .trust(AxisScore::new(50.0))
            .bond_status(BondStatus::Resolved {
                reason: "혼인 해소".into(),
            })
            .build();
        update_axes_from_emotion(&mut r, EmotionType::Anger, 1.0, &HexacoProfile::neutral());
        assert_eq!(r.trust().value(), 50.0);
    }

    #[test]
    fn update_axes_bond_status_dormant_blocks_update() {
        let mut r = RelationshipBuilder::new("a", "b")
            .trust(AxisScore::new(50.0))
            .bond_status(BondStatus::Dormant)
            .build();
        update_axes_from_emotion(&mut r, EmotionType::Anger, 1.0, &HexacoProfile::neutral());
        assert_eq!(r.trust().value(), 50.0);
    }

    #[test]
    fn update_axes_bond_status_active_passes() {
        let mut r = RelationshipBuilder::new("a", "b")
            .trust(AxisScore::new(50.0))
            .bond_status(BondStatus::Active)
            .build();
        update_axes_from_emotion(&mut r, EmotionType::Gratitude, 1.0, &HexacoProfile::neutral());
        assert!(r.trust().value() > 50.0); // 변동 발생
    }

    #[test]
    fn update_axes_bond_status_reactivating_passes() {
        // accepts_live_input == true (BondStatus 1.4 결정)
        use crate::domain::world::event::EventId;
        let mut r = RelationshipBuilder::new("a", "b")
            .trust(AxisScore::new(50.0))
            .bond_status(BondStatus::Reactivating {
                trigger: EventId("test_trigger".into()),
            })
            .build();
        update_axes_from_emotion(&mut r, EmotionType::Gratitude, 1.0, &HexacoProfile::neutral());
        assert!(r.trust().value() > 50.0);
    }

    #[test]
    fn update_axes_unmapped_emotion_no_change() {
        // base_delta(Joy) = default (0, 0, 0, 0) — 변동 없음
        let mut r = Relationship::new(
            "a",
            "b",
            AxisScore::new(50.0),
            AxisScore::new(40.0),
            AxisScore::new(30.0),
            WarinessScore::new(5.0),
        );
        update_axes_from_emotion(&mut r, EmotionType::Joy, 1.0, &HexacoProfile::neutral());
        assert_eq!(r.trust().value(), 50.0);
        assert_eq!(r.affinity().value(), 40.0);
        assert_eq!(r.respect().value(), 30.0);
        assert_eq!(r.wariness().value(), 5.0);
    }

    // -----------------------------------------------------------------------
    // 2.5 modifiers() 1 hop 회귀 가드 — Stage W1
    //   회고 §W1: 4축 → modifiers() 4 필드의 hop 검증
    //   (스펙 task-rel-phase2-stage2-retrospective-cleanup §4)
    // -----------------------------------------------------------------------

    #[test]
    fn beat_rel_modifiers_affinity_channel_after_anger() {
        let hexaco = hexaco_lin_chong();
        let rel = rel_lin_chong_pre_shanshenmiao();
        let mut beat_rel = rel.clone();

        update_axes_from_emotion(&mut beat_rel, EmotionType::Anger, 0.95, &hexaco);

        let before = rel.modifiers();
        let after = beat_rel.modifiers();

        // (1) 방향 회귀 — affinity 감소 → 친화 채널 감소·적대 채널 증가
        assert!(after.intensity_multiplier < before.intensity_multiplier);
        assert!(after.empathy_modifier < before.empathy_modifier);
        assert!(after.hostility_modifier > before.hostility_modifier);

        // (2) 정량 회귀 — 회고 §S2 affinity 28.6 / 100 = 0.286
        let p = profile();
        let expected = (1.0 + 0.286 * p.rel_closeness_intensity_weight).max(0.0);
        assert!(
            (after.intensity_multiplier - expected).abs() < 1e-3,
            "drift: got {}, expected {}",
            after.intensity_multiplier,
            expected
        );
    }

    #[test]
    fn beat_rel_modifiers_trust_channel_after_anger() {
        let hexaco = hexaco_lin_chong();
        let rel = rel_lin_chong_pre_shanshenmiao();
        let mut beat_rel = rel.clone();

        update_axes_from_emotion(&mut beat_rel, EmotionType::Anger, 0.95, &hexaco);

        let before = rel.modifiers();
        let after = beat_rel.modifiers();

        assert!(after.trust_modifier < before.trust_modifier);

        // 회고 §S2 trust 15.8 / 100 = 0.158
        let p = profile();
        let expected = 1.0 + 0.158 * p.rel_trust_emotion_weight;
        assert!(
            (after.trust_modifier - expected).abs() < 1e-3,
            "drift: got {}, expected {}",
            after.trust_modifier,
            expected
        );
    }

    #[test]
    fn beat_rel_modifiers_admiration_no_leak_until_phase_2_3() {
        // Admiration base_delta = { trust 0, affinity 0, respect +20, wariness 0 }
        // → modifier 4 필드 *완전 불변* 이어야 함.
        // Phase 2.3에서 respect를 modifier에 연결하면 *이 테스트가 깨지는 게 정상* —
        // "Phase 2.3 시작 시 spec 재확인" 신호.
        let hexaco = neutral_hexaco();
        let rel = rel_lin_chong_pre_shanshenmiao();
        let mut beat_rel = rel.clone();

        update_axes_from_emotion(&mut beat_rel, EmotionType::Admiration, 0.7, &hexaco);

        let before = rel.modifiers();
        let after = beat_rel.modifiers();

        assert_eq!(after.intensity_multiplier, before.intensity_multiplier);
        assert_eq!(after.trust_modifier, before.trust_modifier);
        assert_eq!(after.empathy_modifier, before.empathy_modifier);
        assert_eq!(after.hostility_modifier, before.hostility_modifier);
    }

    // -----------------------------------------------------------------------
    // 2.6 B-D12 호출 측 책임 회귀 가드 — Stage W4
    //   회고 §W4: `update_axes_from_emotion` 자체는 Pride/Shame 차단 *안 함*.
    //   B-D12 가드는 *호출 측 책임* (spec §4 결정 — 함수 책임 경계 보존).
    //   (스펙 task-rel-phase2-stage2-retrospective-cleanup §7)
    // -----------------------------------------------------------------------

    #[test]
    fn update_axes_from_emotion_does_not_filter_pride_or_shame_internally() {
        // B-D12 가드는 *호출 측 책임* (spec §4) — 본 함수는 Pride/Shame이 직접
        // 전달되면 *base_delta 그대로 4축을 변동*해야 한다.
        // 누군가 함수 안에 `matches!(Pride|Shame) return;` 박으면 이 테스트가
        // 깨지며 spec §4 + 회고 §W4 재독 후 결정 재확인 강제.
        let hexaco = neutral_hexaco();
        let mut rel = RelationshipBuilder::new("a", "b")
            .trust(AxisScore::new(50.0))
            .affinity(AxisScore::new(40.0))
            .build();
        let affinity_before = rel.affinity();

        update_axes_from_emotion(&mut rel, EmotionType::Pride, 0.8, &hexaco);

        // base_delta(Pride) = { trust 0, affinity +5, respect +10, wariness 0 }
        // → affinity 변동 발생해야 함 (함수 안에서 차단 안 됨)
        assert_ne!(
            rel.affinity(),
            affinity_before,
            "함수 자체는 Pride/Shame 차단하지 *않음* (spec §4 결정)"
        );
    }
}
