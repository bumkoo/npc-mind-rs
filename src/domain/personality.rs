//! HEXACO 성격 모델
//!
//! 6개 차원(Dimension) × 각 4개 facet = 24개 facet으로
//! NPC의 성격을 정의한다.
//!
//! 각 값은 -1.0 ~ 1.0 범위이며, 0.0이 평균적 성격을 의미한다.
//! - -1.0 ~ -0.4: 해당 특성이 강하게 부정적 (반대 방향)
//! - -0.4 ~  0.4: 보통
//! -  0.4 ~  1.0: 해당 특성이 강하게 긍정적
//!
//! 이 설계의 핵심 이점:
//! 감정 값 × 성격 가중치 = 방향 유지 + 강도 증폭
//! 예: 부정 감정(-0.3) × 까칠함(1.5) = -0.45 (단순 곱셈으로 자연스러운 증폭)

use serde::{Serialize, Deserialize, Deserializer};

// ---------------------------------------------------------------------------
// 성격 → 감정 가중치 상수
// ---------------------------------------------------------------------------

/// 표준 영향력 (E, X, A, C, Mod, Gen, Aes 등 대부분)
const W_STANDARD: f32 = 0.3;
/// 강한 영향력 (empathy H/A/Sent, hostility A, patience)
const W_STRONG: f32 = 0.4;
/// 지배적 영향력 (hostility H — Resentment 유발)
const W_DOMINANT: f32 = 0.7;
/// 약한 영향력 (prudence in prospect/confirmation)
const W_MILD: f32 = 0.2;

/// 기저값: 자기 감정 (표준)
const BASE_SELF: f32 = 1.0;
/// 기저값: 타인 공감 (타인의 운은 자기보다 약함)
const BASE_EMPATHY: f32 = 0.5;
/// 기저값: 적대 (성격이 나빠야 발동)
const BASE_HOSTILITY: f32 = 0.0;

/// 클램프: 표준 범위
const CLAMP_STANDARD: (f32, f32) = (0.5, 1.5);
/// 클램프: 미발동 가능 (empathy, hostility)
const CLAMP_OPTIONAL: (f32, f32) = (0.0, 1.5);
/// 클램프: 자극 수용도 (넓은 범위)
const CLAMP_STIMULUS: (f32, f32) = (0.1, 2.0);

/// HEXACO 성격 점수의 유효 범위
pub const SCORE_MIN: f32 = -1.0;
pub const SCORE_MAX: f32 = 1.0;
pub const SCORE_NEUTRAL: f32 = 0.0;

/// 성격 점수 유효성 검증 에러
#[derive(Debug, Clone, thiserror::Error)]
pub enum PersonalityError {
    #[error("성격 점수 {value}는 유효 범위 [{min}, {max}]를 벗어남 (항목: {field})")]
    ScoreOutOfRange {
        field: String,
        value: f32,
        min: f32,
        max: f32,
    },
}

/// -1.0 ~ 1.0 범위의 성격 점수 (Value Object)
///
/// 0.0 = 중립, 양수 = 해당 특성이 강함, 음수 = 반대 특성이 강함
///
/// 역직렬화 시 범위를 검증한다. 범위 밖 값은 에러를 반환.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Score(f32);

impl<'de> Deserialize<'de> for Score {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f32::deserialize(deserializer)?;
        if !(SCORE_MIN..=SCORE_MAX).contains(&value) {
            return Err(serde::de::Error::custom(format!(
                "Score {value}는 유효 범위 [{SCORE_MIN}, {SCORE_MAX}]를 벗어남"
            )));
        }
        Ok(Self(value))
    }
}

impl Score {
    pub fn new(value: f32, field: &str) -> Result<Self, PersonalityError> {
        if !(SCORE_MIN..=SCORE_MAX).contains(&value) {
            return Err(PersonalityError::ScoreOutOfRange {
                field: field.to_string(),
                value,
                min: SCORE_MIN,
                max: SCORE_MAX,
            });
        }
        Ok(Self(value))
    }

    /// 범위 내로 클램핑하여 Score 생성 (항상 성공)
    ///
    /// 이미 연산 결과로 나온 값을 안전하게 Score로 변환할 때 사용.
    /// 범위 밖 값은 -1.0 또는 1.0으로 클램핑된다.
    pub fn clamped(value: f32) -> Self {
        Self(value.clamp(SCORE_MIN, SCORE_MAX))
    }

    pub fn neutral() -> Self {
        Self(SCORE_NEUTRAL)
    }

    pub fn value(&self) -> f32 {
        self.0
    }

    /// 절대 강도 (방향 무시, 0.0 ~ 1.0)
    pub fn intensity(&self) -> f32 {
        self.0.abs()
    }

    /// 가중치가 적용된 영향력 수치만 계산 (예: 0.5 * 0.3 = 0.15)
    /// 수식의 의도를 명확히 하기 위해 사용합니다.
    pub fn effect(&self, weight: f32) -> f32 {
        self.0 * weight
    }

    // -----------------------------------------------------------------------
    // 감정 강도 변조(Modifier) 계산기
    // -----------------------------------------------------------------------

    /// 기본적인 가중치 계산: 1.0 + (성격 점수 × 가중치 계수)
    /// 성향이 강할수록 감정의 강도를 증폭시키고 싶을 때 사용합니다.
    /// 하한 0.0 보장 — 음수 가중치가 감정 방향을 뒤집지 않도록.
    pub fn modifier(&self, weight: f32) -> f32 {
        (1.0 + self.effect(weight)).max(0.0)
    }
}

/// 가중치 계산 공통 로직 추출
/// 수식: 기저값 + 성격 영향력 합계 -> 지정된 범위로 클램핑
fn finalize_weight(base: f32, effects: f32, range: (f32, f32)) -> f32 {
    (base + effects).clamp(range.0, range.1)
}

// ---------------------------------------------------------------------------
// HEXACO 6개 차원 (Dimension)
// ---------------------------------------------------------------------------

/// H: 정직-겸손성 (Honesty-Humility)
/// +1.0: 진실되고 공정하며 탐욕을 피하고 겸손함
/// -1.0: 교활하고 탐욕적이며 자기과시적
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HonestyHumility {
    pub sincerity: Score,         // 진실성
    pub fairness: Score,          // 공정성
    pub greed_avoidance: Score,   // 탐욕회피
    pub modesty: Score,           // 겸손
}

/// E: 정서성 (Emotionality)
/// +1.0: 두려움이 많고 불안하며 감정적으로 의존적
/// -1.0: 대담하고 독립적이며 감정적 거리감
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emotionality {
    pub fearfulness: Score,       // 두려움
    pub anxiety: Score,           // 불안
    pub dependence: Score,        // 의존성
    pub sentimentality: Score,    // 감상성
}

/// X: 외향성 (Extraversion)
/// +1.0: 자신감 있고 사교적이며 활기참
/// -1.0: 소극적이고 과묵하며 조용함
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extraversion {
    pub social_self_esteem: Score,  // 사회적 자존감
    pub social_boldness: Score,     // 사회적 대담성
    pub sociability: Score,         // 사교성
    pub liveliness: Score,          // 활력
}

/// A: 원만성 (Agreeableness)
/// +1.0: 관용적이고 유순하며 인내심 강함
/// -1.0: 원한을 품고 비판적이며 완고함
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agreeableness {
    pub forgiveness: Score,   // 용서
    pub gentleness: Score,    // 온화함
    pub flexibility: Score,   // 유연성
    pub patience: Score,      // 인내
}

/// C: 성실성 (Conscientiousness)
/// +1.0: 체계적이고 근면하며 신중함
/// -1.0: 충동적이고 게으르며 부주의함
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conscientiousness {
    pub organization: Score,    // 조직력
    pub diligence: Score,       // 근면
    pub perfectionism: Score,   // 완벽주의
    pub prudence: Score,        // 신중함
}

/// O: 경험에 대한 개방성 (Openness to Experience)
/// +1.0: 미적 감각이 뛰어나고 호기심 많고 창의적
/// -1.0: 보수적이고 관습적이며 상상력 부족
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Openness {
    pub aesthetic_appreciation: Score,  // 미적 감상
    pub inquisitiveness: Score,         // 탐구심
    pub creativity: Score,              // 창의성
    pub unconventionality: Score,       // 비관습성
}

// ---------------------------------------------------------------------------
// HEXACO 성격 프로필 (Aggregate Root)
// ---------------------------------------------------------------------------

/// NPC의 완전한 HEXACO 성격 프로필
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexacoProfile {
    pub honesty_humility: HonestyHumility,
    pub emotionality: Emotionality,
    pub extraversion: Extraversion,
    pub agreeableness: Agreeableness,
    pub conscientiousness: Conscientiousness,
    pub openness: Openness,
}

impl HexacoProfile {
    /// 모든 차원이 중립(0.0)인 기본 프로필
    pub fn neutral() -> Self {
        let s = Score::neutral();
        Self {
            honesty_humility: HonestyHumility {
                sincerity: s, fairness: s,
                greed_avoidance: s, modesty: s,
            },
            emotionality: Emotionality {
                fearfulness: s, anxiety: s,
                dependence: s, sentimentality: s,
            },
            extraversion: Extraversion {
                social_self_esteem: s, social_boldness: s,
                sociability: s, liveliness: s,
            },
            agreeableness: Agreeableness {
                forgiveness: s, gentleness: s,
                flexibility: s, patience: s,
            },
            conscientiousness: Conscientiousness {
                organization: s, diligence: s,
                perfectionism: s, prudence: s,
            },
            openness: Openness {
                aesthetic_appreciation: s, inquisitiveness: s,
                creativity: s, unconventionality: s,
            },
        }
    }

    /// 각 차원의 평균 점수를 반환
    pub fn dimension_averages(&self) -> DimensionAverages {
        DimensionAverages {
            h: avg4(self.honesty_humility.sincerity,
                    self.honesty_humility.fairness,
                    self.honesty_humility.greed_avoidance,
                    self.honesty_humility.modesty),
            e: avg4(self.emotionality.fearfulness,
                    self.emotionality.anxiety,
                    self.emotionality.dependence,
                    self.emotionality.sentimentality),
            x: avg4(self.extraversion.social_self_esteem,
                    self.extraversion.social_boldness,
                    self.extraversion.sociability,
                    self.extraversion.liveliness),
            a: avg4(self.agreeableness.forgiveness,
                    self.agreeableness.gentleness,
                    self.agreeableness.flexibility,
                    self.agreeableness.patience),
            c: avg4(self.conscientiousness.organization,
                    self.conscientiousness.diligence,
                    self.conscientiousness.perfectionism,
                    self.conscientiousness.prudence),
            o: avg4(self.openness.aesthetic_appreciation,
                    self.openness.inquisitiveness,
                    self.openness.creativity,
                    self.openness.unconventionality),
        }
    }
}

/// 6개 차원의 평균 점수 요약
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DimensionAverages {
    pub h: Score, pub e: Score, pub x: Score,
    pub a: Score, pub c: Score, pub o: Score,
}

/// 4개 점수의 평균을 계산하여 Score로 반환 (범위 클램핑 포함)
fn avg4(a: Score, b: Score, c: Score, d: Score) -> Score {
    Score::clamped((a.value() + b.value() + c.value() + d.value()) / 4.0)
}

// ---------------------------------------------------------------------------
// NPC 엔티티
// ---------------------------------------------------------------------------

/// NPC 엔티티 — 이름, 설명, 성격 프로필을 가진다
///
/// 생성 후 필드 직접 변경 불가 — NpcBuilder 또는 Npc::new()를 통해 생성한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Npc {
    id: String,
    name: String,
    description: String,
    personality: HexacoProfile,
}

impl Npc {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        personality: HexacoProfile,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            personality,
        }
    }

    pub fn id(&self) -> &str { &self.id }
    pub fn name(&self) -> &str { &self.name }
    pub fn description(&self) -> &str { &self.description }
    pub fn personality(&self) -> &HexacoProfile { &self.personality }
}

// ---------------------------------------------------------------------------
// NPC 빌더 — 무협 캐릭터를 편리하게 생성
// ---------------------------------------------------------------------------

/// 빈 프로필에서 원하는 차원만 설정하는 빌더
pub struct NpcBuilder {
    id: String,
    name: String,
    description: String,
    profile: HexacoProfile,
}

impl NpcBuilder {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            profile: HexacoProfile::neutral(),
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn honesty_humility(mut self, f: impl FnOnce(&mut HonestyHumility)) -> Self {
        f(&mut self.profile.honesty_humility);
        self
    }

    pub fn emotionality(mut self, f: impl FnOnce(&mut Emotionality)) -> Self {
        f(&mut self.profile.emotionality);
        self
    }

    pub fn extraversion(mut self, f: impl FnOnce(&mut Extraversion)) -> Self {
        f(&mut self.profile.extraversion);
        self
    }

    pub fn agreeableness(mut self, f: impl FnOnce(&mut Agreeableness)) -> Self {
        f(&mut self.profile.agreeableness);
        self
    }

    pub fn conscientiousness(mut self, f: impl FnOnce(&mut Conscientiousness)) -> Self {
        f(&mut self.profile.conscientiousness);
        self
    }

    pub fn openness(mut self, f: impl FnOnce(&mut Openness)) -> Self {
        f(&mut self.profile.openness);
        self
    }

    pub fn build(self) -> Npc {
        Npc {
            id: self.id,
            name: self.name,
            description: self.description,
            personality: self.profile,
        }
    }
}


// ---------------------------------------------------------------------------
// PersonalityProfile 구현 — HEXACO → 차원 평균 요약
// ---------------------------------------------------------------------------

impl crate::ports::PersonalityProfile for HexacoProfile {
    fn dimension_averages(&self) -> DimensionAverages {
        self.dimension_averages()
    }
}

// ---------------------------------------------------------------------------
// AppraisalWeights 구현 — HEXACO → OCC 가중치 캡슐화
// ---------------------------------------------------------------------------

impl crate::ports::AppraisalWeights for HexacoProfile {
    /// 사건-자기-현재: Joy, Distress
    ///
    /// d > 0 (좋은 일): E(예민→증폭) + X(사교→기쁨증폭)
    /// d < 0 (나쁜 일): E(예민→증폭) - A(원만→억제) - Pru(신중→억제)
    fn desirability_self_weight(&self, desirability: f32) -> f32 {
        let avg = self.dimension_averages();
        let mut e = avg.e.effect(W_STANDARD);

        e += if desirability >= 0.0 {
            avg.x.effect(W_STANDARD)
        } else {
            -avg.a.effect(W_STANDARD) - self.conscientiousness.prudence.effect(W_STANDARD)
        };

        finalize_weight(BASE_SELF, e, CLAMP_STANDARD)
    }

    /// 사건-자기-전망: Hope, Fear
    ///
    /// d > 0 (희망): E(예민→증폭) + X(낙관→증폭)
    /// d < 0 (공포): E(예민→증폭) + Fear(겁→증폭)
    fn desirability_prospect_weight(&self, desirability: f32) -> f32 {
        let avg = self.dimension_averages();
        let mut e = avg.e.effect(W_STANDARD);

        e += if desirability >= 0.0 {
            avg.x.effect(W_STANDARD) - self.conscientiousness.prudence.effect(W_MILD)
        } else {
            self.emotionality.fearfulness.effect(W_STANDARD)
        };

        finalize_weight(BASE_SELF, e, CLAMP_STANDARD)
    }

    /// 사건-자기-확인: Satisfaction, Disappointment, Relief, FearsConfirmed
    ///
    /// E(예민→크게 반응) - Pru(신중→충격 감소, 이미 마음의 준비)
    fn desirability_confirmation_weight(&self, _desirability: f32) -> f32 {
        let avg = self.dimension_averages();
        let e = avg.e.effect(W_STANDARD) - self.conscientiousness.prudence.effect(W_MILD);

        finalize_weight(BASE_SELF, e, CLAMP_STANDARD)
    }

    /// 사건-타인-공감: HappyFor, Pity
    ///
    /// d > 0 (타인에게 좋은 일 → HappyFor): H(정직→공감) + A(원만→공감)
    /// d < 0 (타인에게 나쁜 일 → Pity): A(원만→연민) + Sent(감상→연민)
    /// 결과가 0 이하이면 해당 감정 미발동
    fn empathy_weight(&self, desirability: f32) -> f32 {
        let avg = self.dimension_averages();

        let e = if desirability >= 0.0 {
            avg.h.effect(W_STRONG) + avg.a.effect(W_STRONG)
        } else {
            avg.a.effect(W_STRONG) + self.emotionality.sentimentality.effect(W_STRONG)
        };

        finalize_weight(BASE_EMPATHY, e, CLAMP_OPTIONAL)
    }

    /// 사건-타인-적대: Resentment, Gloating
    ///
    /// d > 0 (타인에게 좋은 일 → Resentment): -H(정직 낮을수록 시기)
    /// d < 0 (타인에게 나쁜 일 → Gloating): -H(정직 낮음) - A(원만 낮음)
    /// 결과가 0 이하이면 해당 감정 미발동
    fn hostility_weight(&self, desirability: f32) -> f32 {
        let avg = self.dimension_averages();

        let e = if desirability >= 0.0 {
            -avg.h.effect(W_DOMINANT)
        } else {
            -avg.h.effect(W_STRONG) - avg.a.effect(W_STRONG)
        };

        finalize_weight(BASE_HOSTILITY, e, CLAMP_OPTIONAL)
    }

    /// 행동 평가: Pride, Shame, Admiration, Reproach
    ///
    /// 공통: C(성실→기준엄격)
    /// 자기+칭찬(Pride): -Mod(겸손→자긍심억제)
    /// 자기+비난(Shame): +Mod(겸손→수치심증폭, 내 탓이오)
    /// 타인+칭찬(Admiration): +Gen(온화→감탄증폭)
    /// 타인+비난(Reproach): -Gen(온화→비난억제)
    fn praiseworthiness_weight(&self, is_self: bool, praiseworthiness: f32) -> f32 {
        let avg = self.dimension_averages();
        let mut e = avg.c.effect(W_STANDARD);

        e += if is_self {
            if praiseworthiness > 0.0 {
                -self.honesty_humility.modesty.effect(W_STANDARD)
            } else {
                self.honesty_humility.modesty.effect(W_STANDARD)
            }
        } else {
            if praiseworthiness < 0.0 {
                -self.agreeableness.gentleness.effect(W_STANDARD)
            } else {
                self.agreeableness.gentleness.effect(W_STANDARD)
            }
        };

        finalize_weight(BASE_SELF, e, CLAMP_STANDARD)
    }

    /// 대상 호불호: Love, Hate
    ///
    /// Aes(심미안→호불호 반응 강도)
    fn appealingness_weight(&self, _appealingness: f32) -> f32 {
        let e = self.openness.aesthetic_appreciation.effect(W_STANDARD);

        finalize_weight(BASE_SELF, e, CLAMP_STANDARD)
    }
}

// ---------------------------------------------------------------------------
// StimulusWeights 구현 — HEXACO → 자극 수용도 캡슐화
// ---------------------------------------------------------------------------

impl crate::ports::StimulusWeights for HexacoProfile {
    /// E(예민→수용↑) - Pru(신중→급변억제) - patience(부정자극시 완충)
    fn stimulus_absorb_rate(&self, stimulus: &crate::domain::pad::Pad) -> f32 {
        let avg = self.dimension_averages();
        let mut e = avg.e.effect(W_STANDARD) - self.conscientiousness.prudence.effect(W_STANDARD);

        if stimulus.pleasure < 0.0 {
            e -= self.agreeableness.patience.effect(W_STRONG);
        }

        finalize_weight(BASE_SELF, e, CLAMP_STIMULUS)
    }
}
