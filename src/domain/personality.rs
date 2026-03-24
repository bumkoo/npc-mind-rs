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

use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Score(f32);

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

    pub fn neutral() -> Self {
        Self(SCORE_NEUTRAL)
    }

    pub fn value(&self) -> f32 {
        self.0
    }

    /// 이 점수가 "높은" 수준인지 (0.4 이상)
    pub fn is_high(&self) -> bool {
        self.0 >= 0.4
    }

    /// 이 점수가 "낮은" 수준인지 (-0.4 이하)
    pub fn is_low(&self) -> bool {
        self.0 <= -0.4
    }

    /// 양수 방향인지 (해당 특성이 강한 쪽)
    pub fn is_positive(&self) -> bool {
        self.0 > 0.0
    }

    /// 음수 방향인지 (반대 특성이 강한 쪽)
    pub fn is_negative(&self) -> bool {
        self.0 < 0.0
    }

    /// 절대 강도 (방향 무시, 0.0 ~ 1.0)
    pub fn intensity(&self) -> f32 {
        self.0.abs()
    }

    /// 두 점수 간의 차이 (절대값)
    pub fn distance(&self, other: &Score) -> f32 {
        (self.0 - other.0).abs()
    }

    /// 성격 가중치로 값을 증폭 (방향 유지, 강도 조절)
    /// 감정값 × 성격점수 형태로 사용
    pub fn amplify(&self, factor: f32) -> f32 {
        (self.0 * factor).clamp(SCORE_MIN, SCORE_MAX)
    }
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
#[derive(Debug, Clone, Copy)]
pub struct DimensionAverages {
    pub h: f32, pub e: f32, pub x: f32,
    pub a: f32, pub c: f32, pub o: f32,
}

fn avg4(a: Score, b: Score, c: Score, d: Score) -> f32 {
    (a.value() + b.value() + c.value() + d.value()) / 4.0
}

// ---------------------------------------------------------------------------
// NPC 엔티티
// ---------------------------------------------------------------------------

/// NPC 식별자
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NpcId(pub String);

/// NPC 엔티티 — 이름, 설명, 성격 프로필을 가진다
///
/// 생성 후 필드 직접 변경 불가 — NpcBuilder 또는 Npc::new()를 통해 생성한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Npc {
    id: NpcId,
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
            id: NpcId(id.into()),
            name: name.into(),
            description: description.into(),
            personality,
        }
    }

    pub fn id(&self) -> &NpcId { &self.id }
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
            id: NpcId(self.id),
            name: self.name,
            description: self.description,
            personality: self.profile,
        }
    }
}
