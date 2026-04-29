// wuxia-core/src/experience/handlers/bond_handler.rs
//
// ③ Bond 핸들러 — 관계 갱신.
//
// Person↔Person 관계만 처리 (MVP).
//
// 구독하는 경험:
//   Conversation → interaction 기록
//   Observation (sentiment_delta) → 호감도 변경
//   Combat → 결과별 호감도 변경
//   Rescue → 구출자에 대한 신뢰/호감 증가
//   Betrayal → 배신자에 대한 신뢰/호감 대폭 감소
//   Care → 간호자에 대한 호감/신뢰 증가
//   Gift → 호감 증가
//   Trade → 공정성에 따라 호감 변경

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::relationship::Relationship;
use crate::shared::event::DomainEvent;
use crate::shared::id::{CharacterId, RelationshipId};

use super::super::event::{CombatResult, ExperienceEvent};
use super::super::handler::{EventHandler, HandlerResult, ProcessingContext};

// ---------------------------------------------------------------------------
// RelationshipKey — 방향 있는 관계 키
// ---------------------------------------------------------------------------

/// 방향 있는 관계 키 — (source, target) 쌍.
///
/// A→B와 B→A는 별개의 관계.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationshipKey(pub CharacterId, pub CharacterId);

// ---------------------------------------------------------------------------
// BondHandler
// ---------------------------------------------------------------------------

/// ③ Bond 핸들러 — 관계 갱신.
///
/// `HashMap<RelationshipKey, Relationship>`를 소유하고,
/// ExperienceEvent에 따라 `update_affinity()` / `update_trust()` /
/// `record_interaction()`을 호출한다.
pub struct BondHandler {
    relationships: HashMap<RelationshipKey, Relationship>,
}

impl BondHandler {
    /// 관계 맵으로 핸들러 생성.
    pub fn new(relationships: HashMap<RelationshipKey, Relationship>) -> Self {
        Self { relationships }
    }

    /// 빈 핸들러 생성 (lazy creation 테스트용).
    pub fn empty() -> Self {
        Self {
            relationships: HashMap::new(),
        }
    }

    /// 특정 관계 참조 (읽기).
    pub fn get(&self, source: CharacterId, target: CharacterId) -> Option<&Relationship> {
        self.relationships.get(&RelationshipKey(source, target))
    }

    /// 모든 관계를 소비하여 반환.
    pub fn into_relationships(self) -> HashMap<RelationshipKey, Relationship> {
        self.relationships
    }

    /// 관계가 없으면 생성 (lazy initialization, MVP용).
    ///
    /// ID는 deterministic: source × 10000 + target.
    /// 프로덕션에서는 ID generator 주입 필요.
    fn get_or_create(&mut self, source: CharacterId, target: CharacterId) -> &mut Relationship {
        let key = RelationshipKey(source, target);
        self.relationships.entry(key).or_insert_with(|| {
            let id = RelationshipId::new(source.value() * 10000 + target.value());
            Relationship::new(id, source, target)
        })
    }
}

impl EventHandler for BondHandler {
    fn handle_event(
        &mut self,
        event: &ExperienceEvent,
        _ctx: &ProcessingContext,
    ) -> HandlerResult {
        let subject = event.header().subject;

        let side_effects: Vec<DomainEvent> = match event {
            // 대화 → 교류 기록
            ExperienceEvent::Conversation { counterpart, .. } => {
                let rel = self.get_or_create(subject, *counterpart);
                rel.record_interaction(event.header().time)
            }

            // 관찰 (감정 판정 결과) → 호감도 변경
            ExperienceEvent::Observation {
                target: Some(target),
                sentiment_delta: Some(delta),
                ..
            } => {
                let rel = self.get_or_create(subject, *target);
                rel.update_affinity(*delta)
            }

            // 전투 → 결과별 호감도 변경
            ExperienceEvent::Combat { opponent, result, .. } => {
                let rel = self.get_or_create(subject, *opponent);
                let delta = match result {
                    CombatResult::Victory => -5.0,  // 패배한 상대의 적대감
                    CombatResult::Defeat => -3.0,   // 패배 → 원한
                    CombatResult::Draw => 2.0,      // 호각 → 약간의 존경
                    CombatResult::Fled => -1.0,     // 도주 → 경멸
                };
                rel.update_affinity(delta)
            }

            // 구출 → 구출받은 사람의 구출자에 대한 신뢰/호감 증가
            ExperienceEvent::Rescue { saved, risk_taken, .. } => {
                let rel = self.get_or_create(*saved, subject);
                let mut effects = rel.update_trust(20.0 * risk_taken);
                effects.extend(rel.update_affinity(15.0 * risk_taken));
                effects
            }

            // 배신 → 배신당한 사람의 배신자에 대한 신뢰/호감 대폭 감소
            ExperienceEvent::Betrayal { betrayer, betrayed, .. } => {
                let rel = self.get_or_create(*betrayed, *betrayer);
                let mut effects = rel.update_trust(-40.0);
                effects.extend(rel.update_affinity(-30.0));
                effects
            }

            // 돌봄 → 환자의 간호자에 대한 호감/신뢰 증가
            ExperienceEvent::Care { patient, caregiver, .. } => {
                let rel = self.get_or_create(*patient, *caregiver);
                let mut effects = rel.update_affinity(5.0);
                effects.extend(rel.update_trust(3.0));
                effects
            }

            // 선물 → 받는 사람의 주는 사람에 대한 호감 증가
            ExperienceEvent::Gift { giver, receiver, .. } => {
                let rel = self.get_or_create(*receiver, *giver);
                rel.update_affinity(3.0)
            }

            // 거래 → 공정성에 따라 호감 변경
            ExperienceEvent::Trade { counterpart, fairness, .. } => {
                let rel = self.get_or_create(subject, *counterpart);
                let delta = *fairness * 5.0;
                rel.update_affinity(delta)
            }

            // 나머지는 관계에 영향 없음
            _ => Vec::new(),
        };

        HandlerResult::with_effects(side_effects)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experience::event::ExperienceHeader;
    use crate::relationship::event::RelationshipEvent;
    use crate::shared::id::{ExperienceId, ItemId, LocationId};
    use crate::shared::time::GameTime;

    fn make_header(subject_id: u64) -> ExperienceHeader {
        ExperienceHeader::new(
            ExperienceId::new(1),
            CharacterId::new(subject_id),
            GameTime::new(1200, 3, 15),
            LocationId::new(10),
            5.0,
        )
    }

    fn ctx() -> ProcessingContext {
        ProcessingContext::new()
    }

    // --- 대화 ---

    #[test]
    fn conversation_records_interaction() {
        let mut handler = BondHandler::empty();
        let event = ExperienceEvent::Conversation {
            header: make_header(1),
            counterpart: CharacterId::new(5),
            turns: 10,
            raw_dialogue: "대화".to_string(),
        };

        let result = handler.handle_event(&event, &ctx());

        assert_eq!(result.side_effects.len(), 1);
        assert!(result.side_effects.iter().any(|e| {
            matches!(e, DomainEvent::Relationship(RelationshipEvent::InteractionRecorded { .. }))
        }));
        let rel = handler.get(CharacterId::new(1), CharacterId::new(5)).unwrap();
        assert_eq!(rel.interaction_count(), 1);
    }

    // --- 관찰 (감정 판정) ---

    #[test]
    fn observation_with_delta_changes_affinity() {
        let mut handler = BondHandler::empty();
        let event = ExperienceEvent::Observation {
            header: make_header(1),
            target: Some(CharacterId::new(5)),
            what: "소연이 극도의 분노".to_string(),
            sentiment_delta: Some(-8.0),
        };

        let result = handler.handle_event(&event, &ctx());

        assert!(!result.side_effects.is_empty());
        let rel = handler.get(CharacterId::new(1), CharacterId::new(5)).unwrap();
        assert!(rel.affinity() < 0.0);
    }

    #[test]
    fn observation_without_delta_no_op() {
        let mut handler = BondHandler::empty();
        let event = ExperienceEvent::Observation {
            header: make_header(1),
            target: Some(CharacterId::new(5)),
            what: "수상한 움직임".to_string(),
            sentiment_delta: None,
        };

        let result = handler.handle_event(&event, &ctx());
        assert!(result.side_effects.is_empty());
    }

    #[test]
    fn observation_without_target_no_op() {
        let mut handler = BondHandler::empty();
        let event = ExperienceEvent::Observation {
            header: make_header(1),
            target: None,
            what: "바람이 분다".to_string(),
            sentiment_delta: Some(3.0),
        };

        let result = handler.handle_event(&event, &ctx());
        assert!(result.side_effects.is_empty());
    }

    // --- 전투 ---

    #[test]
    fn combat_victory_decreases_opponent_affinity() {
        let mut handler = BondHandler::empty();
        let event = ExperienceEvent::Combat {
            header: make_header(1),
            opponent: CharacterId::new(3),
            result: CombatResult::Victory,
            technique_used: None,
            technique_faced: None,
        };

        let result = handler.handle_event(&event, &ctx());

        assert!(!result.side_effects.is_empty());
        let rel = handler.get(CharacterId::new(1), CharacterId::new(3)).unwrap();
        assert!(rel.affinity() < 0.0); // 승리 → 상대 적대감
    }

    #[test]
    fn combat_draw_increases_respect() {
        let mut handler = BondHandler::empty();
        let event = ExperienceEvent::Combat {
            header: make_header(1),
            opponent: CharacterId::new(3),
            result: CombatResult::Draw,
            technique_used: None,
            technique_faced: None,
        };

        handler.handle_event(&event, &ctx());

        let rel = handler.get(CharacterId::new(1), CharacterId::new(3)).unwrap();
        assert!(rel.affinity() > 0.0); // 무승부 → 존경
    }

    // --- 구출 ---

    #[test]
    fn rescue_increases_trust_and_affinity() {
        let mut handler = BondHandler::empty();
        let event = ExperienceEvent::Rescue {
            header: make_header(1), // subject=1이 구출함
            saved: CharacterId::new(5),
            danger: "절벽 추락 위기".to_string(),
            risk_taken: 0.8,
        };

        let result = handler.handle_event(&event, &ctx());

        assert!(!result.side_effects.is_empty());
        // saved(5)의 subject(1)에 대한 관계
        let rel = handler.get(CharacterId::new(5), CharacterId::new(1)).unwrap();
        assert!(rel.trust() > 0.0);
        assert!(rel.affinity() > 0.0);
    }

    #[test]
    fn rescue_risk_scales_delta() {
        // 낮은 위험도 → 작은 변화
        let mut handler_low = BondHandler::empty();
        let event_low = ExperienceEvent::Rescue {
            header: make_header(1),
            saved: CharacterId::new(5),
            danger: "작은 위험".to_string(),
            risk_taken: 0.1,
        };
        handler_low.handle_event(&event_low, &ctx());

        // 높은 위험도 → 큰 변화
        let mut handler_high = BondHandler::empty();
        let event_high = ExperienceEvent::Rescue {
            header: make_header(1),
            saved: CharacterId::new(5),
            danger: "큰 위험".to_string(),
            risk_taken: 0.9,
        };
        handler_high.handle_event(&event_high, &ctx());

        let rel_low = handler_low.get(CharacterId::new(5), CharacterId::new(1)).unwrap();
        let rel_high = handler_high.get(CharacterId::new(5), CharacterId::new(1)).unwrap();

        assert!(rel_high.trust() > rel_low.trust());
        assert!(rel_high.affinity() > rel_low.affinity());
    }

    // --- 배신 ---

    #[test]
    fn betrayal_decreases_trust_and_affinity() {
        let mut handler = BondHandler::empty();
        let event = ExperienceEvent::Betrayal {
            header: make_header(1),
            betrayer: CharacterId::new(3),
            betrayed: CharacterId::new(5),
            betrayal_type: "정보 누설".to_string(),
        };

        let result = handler.handle_event(&event, &ctx());

        assert!(!result.side_effects.is_empty());
        // betrayed(5)의 betrayer(3)에 대한 관계
        let rel = handler.get(CharacterId::new(5), CharacterId::new(3)).unwrap();
        // Trust는 0.0~100.0 범위이므로 0 이하로는 내려가지 않음
        assert_eq!(rel.trust(), 0.0);
        assert!(rel.affinity() < 0.0);
    }

    // --- 돌봄 ---

    #[test]
    fn care_increases_affinity_and_trust() {
        let mut handler = BondHandler::empty();
        let event = ExperienceEvent::Care {
            header: make_header(1),
            patient: CharacterId::new(1),
            caregiver: CharacterId::new(5),
        };

        handler.handle_event(&event, &ctx());

        let rel = handler.get(CharacterId::new(1), CharacterId::new(5)).unwrap();
        assert!(rel.affinity() > 0.0);
        assert!(rel.trust() > 0.0);
    }

    // --- 선물 ---

    #[test]
    fn gift_increases_affinity() {
        let mut handler = BondHandler::empty();
        let event = ExperienceEvent::Gift {
            header: make_header(1),
            giver: CharacterId::new(5),
            receiver: CharacterId::new(1),
            item: ItemId::new(42),
        };

        handler.handle_event(&event, &ctx());

        let rel = handler.get(CharacterId::new(1), CharacterId::new(5)).unwrap();
        assert!(rel.affinity() > 0.0);
    }

    // --- 거래 ---

    #[test]
    fn trade_fairness_scales_delta() {
        // 공정한 거래 (fairness=0) → 호감 변화 없음
        let mut handler = BondHandler::empty();
        let event = ExperienceEvent::Trade {
            header: make_header(1),
            counterpart: CharacterId::new(7),
            items: vec![ItemId::new(1)],
            fairness: 0.0,
        };

        let result = handler.handle_event(&event, &ctx());
        assert!(result.side_effects.is_empty()); // delta=0 → no-op

        // 유리한 거래 (fairness=0.5) → 호감 증가
        let event_good = ExperienceEvent::Trade {
            header: make_header(1),
            counterpart: CharacterId::new(8),
            items: vec![ItemId::new(1)],
            fairness: 0.5,
        };
        handler.handle_event(&event_good, &ctx());
        let rel = handler.get(CharacterId::new(1), CharacterId::new(8)).unwrap();
        assert!(rel.affinity() > 0.0);
    }

    // --- 무관한 이벤트 ---

    #[test]
    fn training_no_op_for_bond() {
        let mut handler = BondHandler::empty();
        let event = ExperienceEvent::Training {
            header: make_header(1),
            skill: crate::shared::id::MartialArtId::new(1),
            method: String::new(),
            mentor: None,
            companion: None,
            duration: 3,
            intensity: 5,
        };

        let result = handler.handle_event(&event, &ctx());
        assert!(result.side_effects.is_empty());
    }

    // --- Lazy creation ---

    #[test]
    fn lazy_relationship_creation() {
        let mut handler = BondHandler::empty();

        // 처음에 관계 없음
        assert!(handler.get(CharacterId::new(1), CharacterId::new(5)).is_none());

        let event = ExperienceEvent::Conversation {
            header: make_header(1),
            counterpart: CharacterId::new(5),
            turns: 1,
            raw_dialogue: String::new(),
        };
        handler.handle_event(&event, &ctx());

        // 대화 후 관계가 생성됨
        let rel = handler.get(CharacterId::new(1), CharacterId::new(5));
        assert!(rel.is_some());
        assert_eq!(rel.unwrap().interaction_count(), 1);
    }

    // --- 소유권 ---

    #[test]
    fn into_relationships_returns_map() {
        let mut handler = BondHandler::empty();
        let event = ExperienceEvent::Conversation {
            header: make_header(1),
            counterpart: CharacterId::new(5),
            turns: 1,
            raw_dialogue: String::new(),
        };
        handler.handle_event(&event, &ctx());

        let rels = handler.into_relationships();
        assert_eq!(rels.len(), 1);
    }
}
