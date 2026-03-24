//! NPC 관계 모델
//!
//! NPC와 NPC, NPC와 플레이어 사이의 관계를 3축으로 모델링한다.
//! 각 축은 -1.0 ~ 1.0 범위의 Score를 사용한다.
//!
//! 3축:
//! - closeness (친밀도): 감정 반응의 전반적 배율 + Fortune-of-others 방향
//! - trust (신뢰도): 기대 위반/부합에 따른 감정 증폭/완화
//! - power (상하 관계): 대사 톤 결정 (감정 엔진 영향 최소)
//!
//! 대화 중에는 고정이며, 대화 종료 후 또는 게임 이벤트 시 갱신된다.

use serde::{Deserialize, Serialize};

use super::personality::Score;

// ---------------------------------------------------------------------------
// 갱신 속도 상수
// ---------------------------------------------------------------------------

/// trust 갱신 계수 (대화 후, praiseworthiness 기반)
const TRUST_UPDATE_RATE: f32 = 0.1;
/// closeness 갱신 계수 (대화 후, 전체 감정 valence 기반)
const CLOSENESS_UPDATE_RATE: f32 = 0.05;

// ---------------------------------------------------------------------------
// Relationship (NPC 간 관계)
// ---------------------------------------------------------------------------

/// NPC와 상대(NPC 또는 플레이어) 사이의 관계
///
/// 3축 모두 Score(-1.0 ~ 1.0) 사용.
/// 대화 중에는 고정, 대화 종료 후 갱신.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// 관계 대상 ID
    target_id: String,
    /// 친밀도 (-1.0=적대, 0.0=무관, 1.0=절친)
    /// 감정 반응의 전반적 배율 + Fortune-of-others 분기 방향
    closeness: Score,
    /// 신뢰도 (-1.0=불신, 0.0=중립, 1.0=전적 신뢰)
    /// 기대 위반/부합에 따른 감정 증폭/완화 (OCC unexpectedness 역할)
    trust: Score,
    /// 상하 관계 (-1.0=하위, 0.0=대등, 1.0=상위)
    /// 대사 톤 결정 (감정 엔진 영향 최소)
    power: Score,
}

impl Relationship {
    /// 새 관계 생성
    pub fn new(target_id: impl Into<String>, closeness: Score, trust: Score, power: Score) -> Self {
        Self {
            target_id: target_id.into(),
            closeness,
            trust,
            power,
        }
    }

    /// 중립 관계 (모든 축 0.0)
    pub fn neutral(target_id: impl Into<String>) -> Self {
        Self {
            target_id: target_id.into(),
            closeness: Score::neutral(),
            trust: Score::neutral(),
            power: Score::neutral(),
        }
    }

    // --- 접근자 ---

    pub fn target_id(&self) -> &str { &self.target_id }
    pub fn closeness(&self) -> Score { self.closeness }
    pub fn trust(&self) -> Score { self.trust }
    pub fn power(&self) -> Score { self.power }

    // --- 감정 엔진 연동 ---

    /// 감정 반응 배율: closeness 절대값이 클수록 강한 반응
    /// 무관한 사람(0.0)이면 1.0, 가까운/적대적이면 1.0 이상
    pub fn emotion_intensity_multiplier(&self) -> f32 {
        1.0 + self.closeness.intensity() * 0.5
    }

    /// 기대 위반도: trust와 행동의 불일치가 클수록 감정 증폭
    ///
    /// trust 높은데 배신(praiseworthiness 음수) → 높은 위반도 → 감정 증폭
    /// trust 낮은데 배신 → 기대 부합 → 감정 완화
    /// trust 낮은데 도움(praiseworthiness 양수) → 높은 위반도 → 감정 증폭
    ///
    /// 반환: 0.0(기대 부합) ~ 2.0(극도의 기대 위반)
    pub fn expectation_violation(&self, praiseworthiness: f32) -> f32 {
        // trust와 praiseworthiness가 반대 방향이면 위반
        // trust 0.8 + praiseworthiness -0.7 → 차이 1.5
        // trust -0.5 + praiseworthiness -0.7 → 차이 0.2
        let violation = (self.trust.value() - praiseworthiness).abs();
        violation.min(2.0)
    }

    /// 기대 위반을 감정 배율로 변환
    /// 위반도 1.0(중립) 기준으로 0.5~1.5 범위
    pub fn trust_emotion_modifier(&self, praiseworthiness: f32) -> f32 {
        let violation = self.expectation_violation(praiseworthiness);
        // violation 0.0 → 0.5 (기대 부합, 감정 약화)
        // violation 1.0 → 1.0 (중립)
        // violation 2.0 → 1.5 (기대 위반, 감정 증폭)
        0.5 + violation * 0.5
    }

    // --- 대화 후 갱신 ---

    /// 대화 종료 후 trust 갱신
    /// Action 분기의 praiseworthiness 기반. 점진적 변화.
    pub fn update_trust(&mut self, praiseworthiness: f32) {
        let delta = praiseworthiness * TRUST_UPDATE_RATE;
        let new_value = (self.trust.value() + delta).clamp(-1.0, 1.0);
        self.trust = Score::new(new_value, "trust")
            .expect("clamped value is always valid");
    }

    /// 대화 종료 후 closeness 갱신
    /// 대화의 전체 감정 결과(overall_valence) 기반. 매우 점진적.
    pub fn update_closeness(&mut self, overall_valence: f32) {
        let delta = overall_valence * CLOSENESS_UPDATE_RATE;
        let new_value = (self.closeness.value() + delta).clamp(-1.0, 1.0);
        self.closeness = Score::new(new_value, "closeness")
            .expect("clamped value is always valid");
    }

    /// 게임 이벤트에 의한 power 직접 설정
    pub fn set_power(&mut self, power: Score) {
        self.power = power;
    }
}

// ---------------------------------------------------------------------------
// Relationship 빌더
// ---------------------------------------------------------------------------

/// 관계를 편리하게 생성하는 빌더
pub struct RelationshipBuilder {
    target_id: String,
    closeness: Score,
    trust: Score,
    power: Score,
}

impl RelationshipBuilder {
    pub fn new(target_id: impl Into<String>) -> Self {
        Self {
            target_id: target_id.into(),
            closeness: Score::neutral(),
            trust: Score::neutral(),
            power: Score::neutral(),
        }
    }

    pub fn closeness(mut self, value: Score) -> Self {
        self.closeness = value;
        self
    }

    pub fn trust(mut self, value: Score) -> Self {
        self.trust = value;
        self
    }

    pub fn power(mut self, value: Score) -> Self {
        self.power = value;
        self
    }

    pub fn build(self) -> Relationship {
        Relationship::new(self.target_id, self.closeness, self.trust, self.power)
    }
}
