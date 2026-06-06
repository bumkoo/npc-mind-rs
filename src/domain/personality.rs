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

use serde::{Deserialize, Deserializer, Serialize};

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
    pub sincerity: Score,       // 진실성
    pub fairness: Score,        // 공정성
    pub greed_avoidance: Score, // 탐욕회피
    pub modesty: Score,         // 겸손
}

/// E: 정서성 (Emotionality)
/// +1.0: 두려움이 많고 불안하며 감정적으로 의존적
/// -1.0: 대담하고 독립적이며 감정적 거리감
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emotionality {
    pub fearfulness: Score,    // 두려움
    pub anxiety: Score,        // 불안
    pub dependence: Score,     // 의존성
    pub sentimentality: Score, // 감상성
}

/// X: 외향성 (Extraversion)
/// +1.0: 자신감 있고 사교적이며 활기참
/// -1.0: 소극적이고 과묵하며 조용함
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extraversion {
    pub social_self_esteem: Score, // 사회적 자존감
    pub social_boldness: Score,    // 사회적 대담성
    pub sociability: Score,        // 사교성
    pub liveliness: Score,         // 활력
}

/// A: 원만성 (Agreeableness)
/// +1.0: 관용적이고 유순하며 인내심 강함
/// -1.0: 원한을 품고 비판적이며 완고함
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agreeableness {
    pub forgiveness: Score, // 용서
    pub gentleness: Score,  // 온화함
    pub flexibility: Score, // 유연성
    pub patience: Score,    // 인내
}

/// C: 성실성 (Conscientiousness)
/// +1.0: 체계적이고 근면하며 신중함
/// -1.0: 충동적이고 게으르며 부주의함
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conscientiousness {
    pub organization: Score,  // 조직력
    pub diligence: Score,     // 근면
    pub perfectionism: Score, // 완벽주의
    pub prudence: Score,      // 신중함
}

/// O: 경험에 대한 개방성 (Openness to Experience)
/// +1.0: 미적 감각이 뛰어나고 호기심 많고 창의적
/// -1.0: 보수적이고 관습적이며 상상력 부족
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Openness {
    pub aesthetic_appreciation: Score, // 미적 감상
    pub inquisitiveness: Score,        // 탐구심
    pub creativity: Score,             // 창의성
    pub unconventionality: Score,      // 비관습성
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
                sincerity: s,
                fairness: s,
                greed_avoidance: s,
                modesty: s,
            },
            emotionality: Emotionality {
                fearfulness: s,
                anxiety: s,
                dependence: s,
                sentimentality: s,
            },
            extraversion: Extraversion {
                social_self_esteem: s,
                social_boldness: s,
                sociability: s,
                liveliness: s,
            },
            agreeableness: Agreeableness {
                forgiveness: s,
                gentleness: s,
                flexibility: s,
                patience: s,
            },
            conscientiousness: Conscientiousness {
                organization: s,
                diligence: s,
                perfectionism: s,
                prudence: s,
            },
            openness: Openness {
                aesthetic_appreciation: s,
                inquisitiveness: s,
                creativity: s,
                unconventionality: s,
            },
        }
    }

    /// 각 차원의 평균 점수를 반환
    pub fn dimension_averages(&self) -> DimensionAverages {
        DimensionAverages {
            h: avg4(
                self.honesty_humility.sincerity,
                self.honesty_humility.fairness,
                self.honesty_humility.greed_avoidance,
                self.honesty_humility.modesty,
            ),
            e: avg4(
                self.emotionality.fearfulness,
                self.emotionality.anxiety,
                self.emotionality.dependence,
                self.emotionality.sentimentality,
            ),
            x: avg4(
                self.extraversion.social_self_esteem,
                self.extraversion.social_boldness,
                self.extraversion.sociability,
                self.extraversion.liveliness,
            ),
            a: avg4(
                self.agreeableness.forgiveness,
                self.agreeableness.gentleness,
                self.agreeableness.flexibility,
                self.agreeableness.patience,
            ),
            c: avg4(
                self.conscientiousness.organization,
                self.conscientiousness.diligence,
                self.conscientiousness.perfectionism,
                self.conscientiousness.prudence,
            ),
            o: avg4(
                self.openness.aesthetic_appreciation,
                self.openness.inquisitiveness,
                self.openness.creativity,
                self.openness.unconventionality,
            ),
        }
    }
}

/// 6개 차원의 평균 점수 요약
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DimensionAverages {
    pub h: Score,
    pub e: Score,
    pub x: Score,
    pub a: Score,
    pub c: Score,
    pub o: Score,
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
///
/// `inner_compass`는 Phase 1 Mind Architecture A-min 도입 — 캐릭터의 *움직이게 하는*
/// 가치 한 줄 (예: "협지대자 위국위민"). Reflection prompt builder에서
/// `compass_short_label()`로 회수해 LLM에 NPC 컨텍스트로 주입한다.
/// taboo / life_question은 Phase 3c (`InnerCompass` struct 승격) 시 추가 예정.
/// 디자인 참조: `docs/game-design/2-characters/_schema.md` Layer 2 §inner_compass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Npc {
    id: String,
    name: String,
    description: String,
    personality: HexacoProfile,
    #[serde(default)]
    inner_compass: Option<String>,
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
            inner_compass: None,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn personality(&self) -> &HexacoProfile {
        &self.personality
    }

    /// 캐릭터의 *움직이게 하는* 가치 한 줄. 미설정 시 None.
    pub fn inner_compass(&self) -> Option<&str> {
        self.inner_compass.as_deref()
    }

    /// Reflection prompt builder용 짧은 라벨. Phase 1 = `inner_compass()`와 동일
    /// (cut 없음). 후속 phase에서 첫 N자 cut 또는 `short_form` 필드로 분리 가능.
    pub fn compass_short_label(&self) -> Option<&str> {
        self.inner_compass()
    }

    /// 성격 지표를 기반으로 LLM 생성 파라미터를 도출한다 (Gemma 3 12B 최적화)
    /// 반환값: (temperature, top_p)
    pub fn derive_llm_parameters(&self) -> (f32, f32) {
        self.personality.derive_llm_parameters()
    }
}

impl HexacoProfile {
    /// 성격 지표를 기반으로 LLM 생성 파라미터를 유도 (Gemma 3 12B 최적화)
    /// 반환: (temperature, top_p)
    pub fn derive_llm_parameters(&self) -> (f32, f32) {
        let p = crate::domain::tuning::profile();

        let avg = self.dimension_averages();
        let h = avg.h.value();
        let _e = avg.e.value();
        let x = avg.x.value();
        let _a = avg.a.value();
        let c = avg.c.value();
        let o = avg.o.value();

        // Temperature = Base + (O * Wo) + (X * Wx) - (C * Wc) - (H * Wh)
        let temperature = p.llm_base_temperature
            + (o * p.llm_temp_openness_weight)
            + (x * p.llm_temp_extraversion_weight)
            - (c * p.llm_temp_conscientiousness_weight)
            - (h * p.llm_temp_honesty_weight);

        // Top P = Base + (O * Wo) - (C * Wc)
        let top_p = p.llm_base_top_p
            + (o * p.llm_top_p_openness_weight)
            - (c * p.llm_top_p_conscientiousness_weight);

        (
            temperature.clamp(p.llm_temp_min, p.llm_temp_max),
            top_p.clamp(p.llm_top_p_min, p.llm_top_p_max),
        )
    }
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
    inner_compass: Option<String>,
}

impl NpcBuilder {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            profile: HexacoProfile::neutral(),
            inner_compass: None,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Phase 1 Mind Architecture A-min: 캐릭터의 가치 한 줄.
    /// 미설정 시 `Npc::compass_short_label()` → None (Reflection prompt에서 제외).
    pub fn with_inner_compass(mut self, compass: impl Into<String>) -> Self {
        self.inner_compass = Some(compass.into());
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
            inner_compass: self.inner_compass,
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
    /// d >= 0 (좋은 일 → Joy): X(사교→기쁨증폭) — 긍정 정서는 외향성 주도
    /// d < 0 (나쁜 일 → Distress): E(예민→증폭) - A(원만→억제) - Pru(신중→억제)
    fn desirability_self_weight(&self, desirability: f32) -> f32 {
        let avg = self.dimension_averages();

        // 정서성(E)은 부정 분기(Distress)에만 — 긍정 정서는 X가 주도 (Phase 2.4.1)
        let e = if desirability >= 0.0 {
            avg.x.effect(W_STANDARD)
        } else {
            avg.e.effect(W_STANDARD)
                - avg.a.effect(W_STANDARD)
                - self.conscientiousness.prudence.effect(W_STANDARD)
        };

        finalize_weight(BASE_SELF, e, CLAMP_STANDARD)
    }

    /// 사건-자기-전망: Hope, Fear
    ///
    /// d >= 0 (희망 → Hope): X(낙관→증폭) - Pru(신중→기대억제) — 기대는 외향성 주도
    /// d < 0 (공포 → Fear): E(예민→증폭) + Fear(겁→증폭)
    fn desirability_prospect_weight(&self, desirability: f32) -> f32 {
        let avg = self.dimension_averages();

        // 정서성(E)은 부정 분기(Fear)에만 — 기대(Hope)는 X가 주도 (Phase 2.4.1)
        let e = if desirability >= 0.0 {
            avg.x.effect(W_STANDARD) - self.conscientiousness.prudence.effect(W_MILD)
        } else {
            avg.e.effect(W_STANDARD) + self.emotionality.fearfulness.effect(W_STANDARD)
        };

        finalize_weight(BASE_SELF, e, CLAMP_STANDARD)
    }

    /// 사건-자기-확인: Satisfaction, Disappointment, Relief, FearsConfirmed
    ///
    /// fear축(Relief/FearsConfirmed): E(예민→크게 반응) - Pru(신중→충격 감소) — 불변
    /// hope축(Satisfaction/Disappointment): X(낙관→크게 반응) - Pru
    fn desirability_confirmation_weight(&self, is_fear_axis: bool) -> f32 {
        let avg = self.dimension_averages();

        // fear-lifecycle은 E, hope-lifecycle은 X가 확인 강도를 주도 (Phase 2.4.1)
        let driver = if is_fear_axis {
            avg.e.effect(W_STANDARD)
        } else {
            avg.x.effect(W_STANDARD)
        };
        let e = driver - self.conscientiousness.prudence.effect(W_MILD);

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
    /// 성실성 기여 (성실성 평균 경유 prudence 오염 제거 — org·prud 제외, Phase 2.4.1):
    ///   diligence 균일(0.10) + perfectionism 비대칭
    ///   (Pride −0.10 / Shame +0.20 / Admiration +0.15 / Reproach +0.20)
    /// 분기항(기존 유지):
    ///   자기+칭찬(Pride) -Mod / 자기+비난(Shame) +Mod
    ///   타인+칭찬(Admiration) +Gen / 타인+비난(Reproach) -Gen
    fn praiseworthiness_weight(&self, is_self: bool, praiseworthiness: f32) -> f32 {
        let c = &self.conscientiousness;

        // 성실성 기여 — diligence 균일(0.10)
        let dil = c.diligence.effect(0.10);

        // perfectionism 비대칭 — sign은 effect 밖 적용(modesty/gentleness 관례 일치)
        let perf = if is_self {
            if praiseworthiness > 0.0 {
                -c.perfectionism.effect(0.10) // Pride −0.10
            } else {
                c.perfectionism.effect(0.20) // Shame +0.20
            }
        } else {
            if praiseworthiness > 0.0 {
                c.perfectionism.effect(0.15) // Admiration +0.15
            } else {
                c.perfectionism.effect(0.20) // Reproach +0.20
            }
        };

        // 분기항(기존 유지) — 자기=modesty, 타인=gentleness
        let branch = if is_self {
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

        finalize_weight(BASE_SELF, dil + perf + branch, CLAMP_STANDARD)
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

// ---------------------------------------------------------------------------
// 단위 테스트 — 내부 헬퍼 / 경계값 / 분기 커버리지
// 통합 시나리오는 tests/personality_test.rs 참조.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{AppraisalWeights, StimulusWeights};

    fn s(v: f32) -> Score {
        Score::new(v, "test").expect("범위 내 값")
    }

    #[test]
    fn score_clamped_caps_value_above_max() {
        assert_eq!(Score::clamped(2.5).value(), SCORE_MAX);
    }

    #[test]
    fn score_clamped_caps_value_below_min() {
        assert_eq!(Score::clamped(-3.0).value(), SCORE_MIN);
    }

    #[test]
    fn score_modifier_floor_at_zero_when_negative_combined() {
        // -1.0 * 2.0 = -2.0 → 1 + (-2) = -1 → max(0) = 0
        let m = Score::new(-1.0, "f").unwrap().modifier(2.0);
        assert_eq!(m, 0.0);
    }

    #[test]
    fn score_deserialize_rejects_above_max() {
        let result: Result<Score, _> = serde_json::from_str("1.5");
        assert!(result.is_err());
    }

    #[test]
    fn score_deserialize_rejects_below_min() {
        let result: Result<Score, _> = serde_json::from_str("-1.5");
        assert!(result.is_err());
    }

    #[test]
    fn score_deserialize_accepts_boundary_values() {
        assert_eq!(serde_json::from_str::<Score>("1.0").unwrap().value(), 1.0);
        assert_eq!(serde_json::from_str::<Score>("-1.0").unwrap().value(), -1.0);
    }

    #[test]
    fn avg4_clamps_to_max_when_all_inputs_max() {
        let result = avg4(s(1.0), s(1.0), s(1.0), s(1.0));
        assert_eq!(result.value(), SCORE_MAX);
    }

    #[test]
    fn avg4_returns_arithmetic_mean_for_mixed_values() {
        let result = avg4(s(0.4), s(0.0), s(-0.4), s(0.0));
        assert!((result.value() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn desirability_self_weight_uses_different_branches_by_sign() {
        // d>=0: X(+)만 사용. d<0: E(+) - A(-) - Pru(-) 사용. (Phase 2.4.1: E를 음수 분기로 이동)
        // 같은 프로필에서 분기에 따라 결과가 달라야 함.
        let mut p = HexacoProfile::neutral();
        p.extraversion.social_self_esteem = s(1.0);
        p.extraversion.social_boldness = s(1.0);
        p.extraversion.sociability = s(1.0);
        p.extraversion.liveliness = s(1.0);

        let pos = p.desirability_self_weight(0.5);
        let neg = p.desirability_self_weight(-0.5);
        // X+1: pos에서 effect 추가, neg에선 미사용 → 다른 값
        assert!(pos > neg);
    }

    #[test]
    fn empathy_weight_neutral_profile_returns_base() {
        let p = HexacoProfile::neutral();
        let w = p.empathy_weight(0.5);
        assert_eq!(w, BASE_EMPATHY);
    }

    #[test]
    fn hostility_weight_zero_for_high_honesty() {
        // H=+1 → -avg.h*W_DOMINANT = -0.7 → BASE(0) + (-0.7) → CLAMP_OPTIONAL(min=0) → 0
        let mut p = HexacoProfile::neutral();
        p.honesty_humility.sincerity = s(1.0);
        p.honesty_humility.fairness = s(1.0);
        p.honesty_humility.greed_avoidance = s(1.0);
        p.honesty_humility.modesty = s(1.0);

        let w = p.hostility_weight(0.5);
        assert_eq!(w, 0.0);
    }

    #[test]
    fn praiseworthiness_weight_branches_differ_by_self_and_sign() {
        // is_self=true: praise>0 → -modesty effect, praise<0 → +modesty effect
        // 같은 modesty(+1)에 대해 두 결과가 달라야 함.
        let mut p = HexacoProfile::neutral();
        p.honesty_humility.modesty = s(1.0);

        let self_praise = p.praiseworthiness_weight(true, 0.5);
        let self_blame = p.praiseworthiness_weight(true, -0.5);
        assert!(self_praise < self_blame, "겸손은 자기칭찬은 억제하고 자기비난은 증폭한다");
    }

    #[test]
    fn praiseworthiness_weight_other_branch_uses_gentleness() {
        // is_self=false: praise>0 → +gentleness, praise<0 → -gentleness
        let mut p = HexacoProfile::neutral();
        p.agreeableness.gentleness = s(1.0);

        let other_admire = p.praiseworthiness_weight(false, 0.5);
        let other_reproach = p.praiseworthiness_weight(false, -0.5);
        assert!(other_admire > other_reproach, "온화함은 감탄을 증폭하고 비난을 억제한다");
    }

    #[test]
    fn appealingness_weight_neutral_profile_returns_base_self() {
        let p = HexacoProfile::neutral();
        assert_eq!(p.appealingness_weight(0.5), BASE_SELF);
    }

    #[test]
    fn stimulus_absorb_rate_subtracts_patience_for_negative_pleasure() {
        use crate::domain::pad::Pad;
        let mut p = HexacoProfile::neutral();
        p.agreeableness.patience = s(1.0);

        let neg_pad = Pad { pleasure: -0.5, arousal: 0.0, dominance: 0.0 };
        let pos_pad = Pad { pleasure: 0.5, arousal: 0.0, dominance: 0.0 };
        // patience=+1: 부정 자극에서만 차감
        assert!(p.stimulus_absorb_rate(&neg_pad) < p.stimulus_absorb_rate(&pos_pad));
    }

    #[test]
    fn finalize_weight_clamps_to_provided_range() {
        assert_eq!(finalize_weight(1.0, 5.0, CLAMP_STANDARD), CLAMP_STANDARD.1);
        assert_eq!(finalize_weight(0.0, -5.0, CLAMP_OPTIONAL), CLAMP_OPTIONAL.0);
    }

    // -----------------------------------------------------------------------
    // Phase 1 Mind Architecture A-min — inner_compass + compass_short_label
    // -----------------------------------------------------------------------

    #[test]
    fn npc_new_defaults_inner_compass_to_none() {
        let npc = Npc::new("a", "Alice", "desc", HexacoProfile::neutral());
        assert_eq!(npc.inner_compass(), None);
        assert_eq!(npc.compass_short_label(), None);
    }

    #[test]
    fn npc_builder_with_inner_compass_sets_compass() {
        let npc = NpcBuilder::new("lin", "임충")
            .with_inner_compass("협지대자 위국위민")
            .build();
        assert_eq!(npc.inner_compass(), Some("협지대자 위국위민"));
        assert_eq!(npc.compass_short_label(), Some("협지대자 위국위민"));
    }

    #[test]
    fn npc_compass_short_label_is_alias_for_inner_compass_in_phase1() {
        // Phase 1: cut 없음. 후속 phase에서 첫 N자 cut 또는 short_form 분리 가능.
        let npc = NpcBuilder::new("g", "곽정")
            .with_inner_compass("매우 길고 긴 가치 한 줄로 prompt token 부풀릴 수 있는 케이스")
            .build();
        assert_eq!(npc.inner_compass(), npc.compass_short_label());
    }

    #[test]
    fn npc_inner_compass_serde_roundtrip_preserves_value() {
        let npc = NpcBuilder::new("y", "수련")
            .with_inner_compass("공성명수신퇴")
            .build();
        let json = serde_json::to_string(&npc).unwrap();
        let back: Npc = serde_json::from_str(&json).unwrap();
        assert_eq!(back.inner_compass(), Some("공성명수신퇴"));
    }

    #[test]
    fn npc_legacy_json_without_inner_compass_field_deserializes_as_none() {
        // forward-compat: 기존 시나리오 JSON (inner_compass 키 부재)도 호환
        let legacy_json = r#"{
            "id": "old",
            "name": "Legacy",
            "description": "before A-min",
            "personality": {
                "honesty_humility": {"sincerity": 0.0, "fairness": 0.0, "greed_avoidance": 0.0, "modesty": 0.0},
                "emotionality": {"fearfulness": 0.0, "anxiety": 0.0, "dependence": 0.0, "sentimentality": 0.0},
                "extraversion": {"social_self_esteem": 0.0, "social_boldness": 0.0, "sociability": 0.0, "liveliness": 0.0},
                "agreeableness": {"forgiveness": 0.0, "gentleness": 0.0, "flexibility": 0.0, "patience": 0.0},
                "conscientiousness": {"organization": 0.0, "diligence": 0.0, "perfectionism": 0.0, "prudence": 0.0},
                "openness": {"aesthetic_appreciation": 0.0, "inquisitiveness": 0.0, "creativity": 0.0, "unconventionality": 0.0}
            }
        }"#;
        let npc: Npc = serde_json::from_str(legacy_json).expect("legacy JSON 호환 deserialize");
        assert_eq!(npc.inner_compass(), None);
    }
}
