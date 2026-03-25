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
//! ## DDD 분류: Value Object
//!
//! Relationship은 불변 Value Object다.
//! 상태를 변경하는 메서드는 새 인스턴스를 반환한다.
//! 소유자(owner_id)와 대상(target_id)의 조합이 동일성을 결정한다.
//!
//! 대화 중에는 고정이며, 대화 종료 후 새 인스턴스로 교체된다.

use serde::{Deserialize, Serialize};

use super::emotion::{EmotionState, Situation, SituationFocus};
use super::personality::Score;

// ---------------------------------------------------------------------------
// 갱신 속도 상수
// ---------------------------------------------------------------------------

/// trust 갱신 계수 (대화 후, praiseworthiness 기반)
const TRUST_UPDATE_RATE: f32 = 0.1;
/// closeness 갱신 계수 (대화 후, 전체 감정 valence 기반)
const CLOSENESS_UPDATE_RATE: f32 = 0.05;

// ---------------------------------------------------------------------------
// Relationship (Value Object)
// ---------------------------------------------------------------------------

/// NPC와 상대(NPC 또는 플레이어) 사이의 관계
///
/// 불변 Value Object — 상태 변경 시 새 인스턴스를 반환한다.
/// 3축 모두 Score(-1.0 ~ 1.0) 사용.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// 관계 소유자 ID (누구의 관계인가)
    owner_id: String,
    /// 관계 대상 ID (누구에 대한 관계인가)
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
    pub fn new(
        owner_id: impl Into<String>,
        target_id: impl Into<String>,
        closeness: Score,
        trust: Score,
        power: Score,
    ) -> Self {
        Self {
            owner_id: owner_id.into(),
            target_id: target_id.into(),
            closeness,
            trust,
            power,
        }
    }

    /// 중립 관계 (모든 축 0.0)
    pub fn neutral(owner_id: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self {
            owner_id: owner_id.into(),
            target_id: target_id.into(),
            closeness: Score::neutral(),
            trust: Score::neutral(),
            power: Score::neutral(),
        }
    }

    // --- 접근자 ---

    pub fn owner_id(&self) -> &str { &self.owner_id }
    pub fn target_id(&self) -> &str { &self.target_id }
    pub fn closeness(&self) -> Score { self.closeness }
    pub fn trust(&self) -> Score { self.trust }
    pub fn power(&self) -> Score { self.power }

    // --- 감정 엔진 연동 (읽기 전용) ---

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
        let violation = (self.trust.value() - praiseworthiness).abs();
        violation.min(2.0)
    }

    /// 기대 위반을 감정 배율로 변환
    /// 위반도 1.0(중립) 기준으로 0.5~1.5 범위
    pub fn trust_emotion_modifier(&self, praiseworthiness: f32) -> f32 {
        let violation = self.expectation_violation(praiseworthiness);
        0.5 + violation * 0.5
    }

    // --- 새 인스턴스 반환 (Value Object 패턴) ---

    /// trust를 갱신한 새 Relationship 반환
    /// Action 분기의 praiseworthiness 기반. 점진적 변화.
    pub fn with_updated_trust(&self, praiseworthiness: f32) -> Self {
        let delta = praiseworthiness * TRUST_UPDATE_RATE;
        let new_value = (self.trust.value() + delta).clamp(-1.0, 1.0);
        Self {
            trust: Score::new(new_value, "trust")
                .expect("clamped value is always valid"),
            ..self.clone()
        }
    }

    /// closeness를 갱신한 새 Relationship 반환
    /// 대화의 전체 감정 결과(overall_valence) 기반. 매우 점진적.
    pub fn with_updated_closeness(&self, overall_valence: f32) -> Self {
        let delta = overall_valence * CLOSENESS_UPDATE_RATE;
        let new_value = (self.closeness.value() + delta).clamp(-1.0, 1.0);
        Self {
            closeness: Score::new(new_value, "closeness")
                .expect("clamped value is always valid"),
            ..self.clone()
        }
    }

    /// power를 변경한 새 Relationship 반환
    /// 게임 이벤트(승급, 내공 상실 등)에 의해 직접 설정.
    pub fn with_power(&self, power: Score) -> Self {
        Self {
            power,
            ..self.clone()
        }
    }

    /// 대화 종료 후 갱신된 새 Relationship 반환
    ///
    /// - trust: Action 분기의 praiseworthiness 기반 (점진적)
    /// - closeness: 대화 최종 감정 결과 기반 (매우 점진적)
    /// - power: 변경 없음 (서사 이벤트에서만)
    pub fn after_dialogue(
        &self,
        final_state: &EmotionState,
        situation: &Situation,
    ) -> Self {
        let mut result = self.clone();

        // trust: Action 분기일 때만 갱신
        if let SituationFocus::Action { praiseworthiness, .. } = &situation.focus {
            result = result.with_updated_trust(*praiseworthiness);
        }

        // closeness: 항상 갱신 (전체 감정 결과 기반)
        result = result.with_updated_closeness(final_state.overall_valence());

        result
    }
}

// ---------------------------------------------------------------------------
// Relationship 빌더
// ---------------------------------------------------------------------------

/// 관계를 편리하게 생성하는 빌더
pub struct RelationshipBuilder {
    owner_id: String,
    target_id: String,
    closeness: Score,
    trust: Score,
    power: Score,
}

impl RelationshipBuilder {
    pub fn new(owner_id: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self {
            owner_id: owner_id.into(),
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
        Relationship::new(self.owner_id, self.target_id, self.closeness, self.trust, self.power)
    }
}
