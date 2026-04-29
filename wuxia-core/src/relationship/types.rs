// wuxia-core/src/relationship/types.rs
//
// Relationship Aggregate Root — "이 둘은 어떤 사이인가?"
//
// 비유: 강호 인맥첩 (江湖人脈帖)
//   두 사람 사이의 호감, 신뢰를 기록한다.
//   같은 사건을 겪어도 관계에 따라 반응이 달라진다.
//
// 2축 관계 모델:
//   호감도 (affinity)  -100~+100 — "이 사람이 좋은가?" (음수=적대)
//   신뢰도 (trust)       0~100 — "이 사람을 믿을 수 있는가?"
//
// 값 객체 (RelationshipType, RelationshipLevel, TrustLevel)는
// 각각의 전용 모듈로 분리되어 있다.

use serde::{Deserialize, Serialize};

use crate::shared::event::DomainEvent;
use crate::shared::id::{CharacterId, RelationshipId};
use crate::shared::time::GameTime;

use super::description::RelationshipDescriptions;
use super::event::RelationshipEvent;
use super::level::RelationshipLevel;
use super::relationship_type::RelationshipType;
use super::trust_level::TrustLevel;

// ---------------------------------------------------------------------------
// Affinity — 호감도 newtype (-100.0 ~ +100.0)
// ---------------------------------------------------------------------------

/// 호감도 값. -100.0 ~ +100.0 범위가 타입 수준에서 보장된다.
///
/// 생성 시 자동 클램핑되므로 범위 외 값이 존재할 수 없다.
///
/// ```
/// use wuxia_core::relationship::Affinity;
///
/// let a = Affinity::new(150.0);
/// assert_eq!(a.value(), 100.0); // clamped
///
/// let b = Affinity::new(-200.0);
/// assert_eq!(b.value(), -100.0); // clamped
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Affinity(f32);

impl Affinity {
    pub const MIN: f32 = -100.0;
    pub const MAX: f32 = 100.0;

    /// 호감도 값을 생성한다. 범위를 초과하면 클램핑된다.
    pub fn new(value: f32) -> Self {
        Affinity(value.clamp(Self::MIN, Self::MAX))
    }

    /// 내부 f32 값을 반환한다.
    pub fn value(&self) -> f32 {
        self.0
    }

    /// delta를 적용한 새 Affinity를 반환한다.
    pub fn apply_delta(&self, delta: f32) -> Self {
        Affinity::new(self.0 + delta)
    }
}

// ---------------------------------------------------------------------------
// Trust — 신뢰도 newtype (0.0 ~ 100.0)
// ---------------------------------------------------------------------------

/// 신뢰도 값. 0.0 ~ 100.0 범위가 타입 수준에서 보장된다.
///
/// ```
/// use wuxia_core::relationship::Trust;
///
/// let t = Trust::new(120.0);
/// assert_eq!(t.value(), 100.0); // clamped
///
/// let t2 = Trust::new(-10.0);
/// assert_eq!(t2.value(), 0.0); // clamped
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Trust(f32);

impl Trust {
    pub const MIN: f32 = 0.0;
    pub const MAX: f32 = 100.0;

    /// 신뢰도 값을 생성한다. 범위를 초과하면 클램핑된다.
    pub fn new(value: f32) -> Self {
        Trust(value.clamp(Self::MIN, Self::MAX))
    }

    /// 내부 f32 값을 반환한다.
    pub fn value(&self) -> f32 {
        self.0
    }

    /// delta를 적용한 새 Trust를 반환한다.
    pub fn apply_delta(&self, delta: f32) -> Self {
        Trust::new(self.0 + delta)
    }
}

// ---------------------------------------------------------------------------
// Constants (backward compatibility + internal thresholds)
// ---------------------------------------------------------------------------

/// 호감도 최솟값.
pub const AFFINITY_MIN: f32 = Affinity::MIN;
/// 호감도 최댓값.
pub const AFFINITY_MAX: f32 = Affinity::MAX;
/// 신뢰도 최솟값.
pub const TRUST_MIN: f32 = Trust::MIN;
/// 신뢰도 최댓값.
pub const TRUST_MAX: f32 = Trust::MAX;

// RelationshipLevel 판정 임계값
const ACQUAINTANCE_AFFINITY: f32 = 20.0;
const ACQUAINTANCE_TRUST: f32 = 20.0;
const FRIENDLY_AFFINITY: f32 = 50.0;
const FRIENDLY_TRUST: f32 = 30.0;
const CLOSE_AFFINITY: f32 = 70.0;
const CLOSE_TRUST: f32 = 50.0;
const INTIMATE_AFFINITY: f32 = 80.0;
const INTIMATE_TRUST: f32 = 70.0;
const WARY_AFFINITY: f32 = -10.0;
const HOSTILE_AFFINITY: f32 = -40.0;
const ENEMY_AFFINITY: f32 = -80.0;

// ---------------------------------------------------------------------------
// Relationship — 핵심 구조체
// ---------------------------------------------------------------------------

/// 두 캐릭터 사이의 관계.
///
/// `source`가 `target`을 어떻게 느끼는지를 나타낸다.
/// **비대칭**: 소연→플레이어와 플레이어→소연은 별개의 Relationship이다.
///
/// # 2축 모델
/// - `affinity` (호감도): "이 사람이 좋은가?" -100~+100 (음수=적대)
/// - `trust` (신뢰도): "이 사람을 믿을 수 있는가?" 0~100
///
/// # Example
/// ```
/// use wuxia_core::relationship::{Relationship, RelationshipLevel};
/// use wuxia_core::shared::id::{CharacterId, RelationshipId};
///
/// let player = CharacterId::new(1);
/// let soyeon = CharacterId::new(2);
/// let mut rel = Relationship::new(RelationshipId::new(1), player, soyeon);
///
/// assert_eq!(rel.level(), RelationshipLevel::Stranger);
///
/// rel.update_affinity(55.0);
/// rel.update_trust(35.0);
/// assert_eq!(rel.level(), RelationshipLevel::Friendly);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    id: RelationshipId,
    source: CharacterId,
    target: CharacterId,
    relation_type: Option<RelationshipType>,
    affinity: Affinity,
    trust: Trust,
    interaction_count: u32,
    last_interaction: Option<GameTime>,
}

/// 2축 관계 모델의 축 식별자. update_axis() 내부에서 사용.
enum Axis {
    Affinity,
    Trust,
}

// ---------------------------------------------------------------------------
// Impression — 관계의 첫인상 및 재회 인상 (도메인 핵심 로직)
// ---------------------------------------------------------------------------

/// 상대를 대면했을 때 형성되는 지배적인 인상.
/// 상호작용 횟수와 현재 감정 상태(Affinity)를 조합하여 판단한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Impression {
    /// 생면부지: 오늘 처음 보는 사이.
    FirstMeeting,
    /// 호의적 재회: 구면이며 이전 기억이 좋다.
    WarmReunion,
    /// 적대적 재회: 구면이며 이전 기억이 불쾌하다.
    ColdReunion,
    /// 서먹한 재회: 구면이지만 아직 별 감흥이 없다.
    NeutralReunion,
}

impl Relationship {
    /// 현재 수치들을 기반으로 도메인 관점의 '인상'을 판정한다.
    pub fn current_impression(&self) -> Impression {
        if self.interaction_count == 0 {
            return Impression::FirstMeeting;
        }

        if self.affinity.value() > 5.0 {
            Impression::WarmReunion
        } else if self.affinity.value() < -5.0 {
            Impression::ColdReunion
        } else {
            Impression::NeutralReunion
        }
    }

    /// 관계의 현재 상태를 자연어로 설명한다 (도메인 책임).
    pub fn describe(
        &self,
        descs: &RelationshipDescriptions,
        locale: &str,
    ) -> (String, String, String) {
        // 도메인이 스스로 판단한 인상을 기반으로 설명을 가져온다.
        let impression = self.current_impression();
        let interaction_text = descs.describe_impression(impression, self.interaction_count, locale);

        let (level_label, level_desc) = descs
            .lookup_relationship_level(self.level().key(), locale)
            .unwrap_or(("???", ""));

        let (_, trust_desc) = descs
            .lookup_trust_level(self.trust_level().key(), locale)
            .unwrap_or(("???", ""));

        (
            level_label.to_string(),
            format!("{}\n{}", interaction_text, level_desc),
            trust_desc.to_string(),
        )
    }

    // -----------------------------------------------------------------------
    // 생성
    // -----------------------------------------------------------------------

    /// 새 관계를 생성한다. 모든 수치는 0, 유형은 None.
    ///
    /// "뒷골목에서 처음 만난 두 사람. 아직 아무것도 아닌 사이."
    pub fn new(id: RelationshipId, source: CharacterId, target: CharacterId) -> Self {
        Self {
            id,
            source,
            target,
            relation_type: None,
            affinity: Affinity::new(0.0),
            trust: Trust::new(0.0),
            interaction_count: 0,
            last_interaction: None,
        }
    }

    /// 관계 유형을 지정하여 생성한다.
    ///
    /// "소풍자와 소연은 처음부터 사제다."
    pub fn with_type(
        id: RelationshipId,
        source: CharacterId,
        target: CharacterId,
        relation_type: RelationshipType,
    ) -> Self {
        Self {
            relation_type: Some(relation_type),
            ..Self::new(id, source, target)
        }
    }

    // -----------------------------------------------------------------------
    // 읽기 (Getters)
    // -----------------------------------------------------------------------

    pub fn id(&self) -> RelationshipId {
        self.id
    }
    pub fn source(&self) -> CharacterId {
        self.source
    }
    pub fn target(&self) -> CharacterId {
        self.target
    }
    pub fn relation_type(&self) -> Option<RelationshipType> {
        self.relation_type
    }
    pub fn affinity(&self) -> f32 {
        self.affinity.value()
    }
    pub fn trust(&self) -> f32 {
        self.trust.value()
    }
    pub fn interaction_count(&self) -> u32 {
        self.interaction_count
    }
    pub fn last_interaction(&self) -> Option<GameTime> {
        self.last_interaction
    }

    // -----------------------------------------------------------------------
    // 수정 (Mutations) — affinity: -100~+100, trust: 0~100
    // -----------------------------------------------------------------------

    /// 호감도를 delta만큼 변경한다. 양수면 증가, 음수면 감소.
    ///
    /// 변경이 실제로 발생하면 `AffinityChanged` 이벤트를 반환한다.
    /// 레벨 전이가 발생하면 `LevelChanged` 이벤트도 추가된다.
    /// 변화 없으면 빈 Vec (no-op rule).
    ///
    /// "소연이 정보를 무료로 줬다." → `update_affinity(10.0)`
    pub fn update_affinity(&mut self, delta: f32) -> Vec<DomainEvent> {
        self.update_axis(Axis::Affinity, delta)
    }

    /// 신뢰도를 delta만큼 변경한다.
    ///
    /// 변경이 실제로 발생하면 `TrustChanged` 이벤트를 반환한다.
    /// 레벨 전이가 발생하면 `LevelChanged` 이벤트도 추가된다.
    ///
    /// "소연이 개방 소속임을 밝혔다." → `update_trust(15.0)`
    pub fn update_trust(&mut self, delta: f32) -> Vec<DomainEvent> {
        self.update_axis(Axis::Trust, delta)
    }

    /// 2축 공통 업데이트 로직. 값 변경 → 이벤트 발행 → 레벨 전이 감지.
    fn update_axis(&mut self, axis: Axis, delta: f32) -> Vec<DomainEvent> {
        let old_value = match axis {
            Axis::Affinity => self.affinity.value(),
            Axis::Trust => self.trust.value(),
        };
        let old_level = self.level();
        let new_value = match axis {
            Axis::Affinity => {
                self.affinity = self.affinity.apply_delta(delta);
                self.affinity.value()
            }
            Axis::Trust => {
                self.trust = self.trust.apply_delta(delta);
                self.trust.value()
            }
        };
        if old_value == new_value {
            return Vec::new();
        }
        let changed_event = match axis {
            Axis::Affinity => RelationshipEvent::AffinityChanged {
                relationship_id: self.id,
                source: self.source,
                target: self.target,
                old_value,
                new_value,
            },
            Axis::Trust => RelationshipEvent::TrustChanged {
                relationship_id: self.id,
                source: self.source,
                target: self.target,
                old_value,
                new_value,
            },
        };
        let mut events = vec![changed_event.into()];
        let new_level = self.level();
        if old_level != new_level {
            events.push(
                RelationshipEvent::LevelChanged {
                    relationship_id: self.id,
                    source: self.source,
                    target: self.target,
                    old_level,
                    new_level,
                }
                .into(),
            );
        }
        events
    }

    /// 관계 유형을 설정/변경한다.
    ///
    /// 변경이 실제로 발생하면 `TypeChanged` 이벤트를 반환한다.
    /// 동일 유형이면 빈 Vec (no-op rule).
    pub fn set_relation_type(
        &mut self,
        relation_type: Option<RelationshipType>,
    ) -> Vec<DomainEvent> {
        let old_type = self.relation_type;
        self.relation_type = relation_type;
        if old_type == relation_type {
            return Vec::new();
        }
        vec![RelationshipEvent::TypeChanged {
            relationship_id: self.id,
            source: self.source,
            target: self.target,
            old_type,
            new_type: relation_type,
        }
        .into()]
    }

    /// 상호작용을 기록한다. 횟수 +1, 마지막 시간 갱신.
    ///
    /// 항상 `InteractionRecorded` 이벤트를 반환한다.
    pub fn record_interaction(&mut self, game_time: GameTime) -> Vec<DomainEvent> {
        self.interaction_count += 1;
        self.last_interaction = Some(game_time);
        vec![RelationshipEvent::InteractionRecorded {
            relationship_id: self.id,
            source: self.source,
            target: self.target,
            interaction_count: self.interaction_count,
        }
        .into()]
    }

    // -----------------------------------------------------------------------
    // 판정 (Judgment)
    // -----------------------------------------------------------------------

    /// 현재 관계 깊이를 판정한다.
    ///
    /// 음수 호감도가 적대/원수를 결정한다 (별도 적대도 축 없음).
    ///
    /// ```text
    /// 소연 퀘스트 트리거 대응:
    ///   Acquaintance (호감20+) → 거래 파트너
    ///   Friendly     (호감50+신뢰30+) → 개방 언급
    ///   Close        (호감70+신뢰50+) → 사부의 부탁
    ///   Intimate     (호감80+신뢰70+) → 진짜 소연
    ///   Hostile      (호감-40이하) → "다가오지 마"
    ///   Enemy        (호감-80이하) → "개방의 적으로 대하겠어"
    /// ```
    pub fn level(&self) -> RelationshipLevel {
        Self::compute_level(self.affinity.value(), self.trust.value())
    }

    /// 2축 값으로부터 관계 레벨을 계산한다 (내부 헬퍼).
    ///
    /// mutation 메서드에서 변경 전 레벨을 계산할 때도 사용된다.
    fn compute_level(affinity: f32, trust: f32) -> RelationshipLevel {
        // 음수 호감도 → 적대 판정 (낮은 값부터)
        if affinity <= ENEMY_AFFINITY {
            return RelationshipLevel::Enemy;
        }
        if affinity <= HOSTILE_AFFINITY {
            return RelationshipLevel::Hostile;
        }
        if affinity <= WARY_AFFINITY {
            return RelationshipLevel::Wary;
        }
        // 우호 판정 (높은 단계부터)
        if affinity >= INTIMATE_AFFINITY && trust >= INTIMATE_TRUST {
            return RelationshipLevel::Intimate;
        }
        if affinity >= CLOSE_AFFINITY && trust >= CLOSE_TRUST {
            return RelationshipLevel::Close;
        }
        if affinity >= FRIENDLY_AFFINITY && trust >= FRIENDLY_TRUST {
            return RelationshipLevel::Friendly;
        }
        if affinity >= ACQUAINTANCE_AFFINITY || trust >= ACQUAINTANCE_TRUST {
            return RelationshipLevel::Acquaintance;
        }
        RelationshipLevel::Stranger
    }

    /// 적대 상태인가? (affinity <= -40)
    pub fn is_hostile(&self) -> bool {
        self.affinity.value() <= HOSTILE_AFFINITY
    }

    /// 원수 상태인가? (affinity <= -80)
    pub fn is_enemy(&self) -> bool {
        self.affinity.value() <= ENEMY_AFFINITY
    }

    /// 신뢰도 구간을 반환한다.
    ///
    /// 프롬프트 조립 시 숫자 대신 TrustLevel → 설정 파일 → 자연어로 변환.
    pub fn trust_level(&self) -> TrustLevel {
        TrustLevel::from_value(self.trust.value())
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
