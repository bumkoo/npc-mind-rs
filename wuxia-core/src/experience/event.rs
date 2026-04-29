// wuxia-core/src/experience/event.rs
//
// Experience Event — 경험 이벤트 정의.
//
// 경험-기억-이벤트 통합 아키텍처의 핵심 타입.
// ExperienceEvent는 큐에 들어가는 유일한 것.
// DomainEvent는 큐에 들어가지 않고 ProcessingContext에 담기는 처리 부산물.
//
// 경험이 발생하면 그것이 곧 기억이 되고,
// 동시에 이벤트로 각 도메인에 전달된다:
//   대화를 했다 → 기억이 생긴다 → 관계가 변하고, 감정이 변한다
//   수련을 했다 → 기억이 생긴다 → 숙련도가 오르고, 피로가 쌓인다
//   전투를 했다 → 기억이 생긴다 → 관계가 변하고, 부상이 생긴다

use serde::{Deserialize, Serialize};

use crate::shared::id::{CharacterId, ExperienceId, ItemId, LocationId, MartialArtId};
use crate::shared::time::GameTime;

// ---------------------------------------------------------------------------
// ExperienceHeader — 공통 헤더
// ---------------------------------------------------------------------------

/// 모든 경험 이벤트의 공통 헤더.
///
/// 경험 ID, 주체, 시간, 장소, 중요도를 담는다.
/// 벡터DB 저장 시 이 헤더 필드가 기본 컬럼이 된다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExperienceHeader {
    /// 경험 고유 식별자 — 벡터DB의 primary key
    pub experience_id: ExperienceId,
    /// 경험의 주체 (이 경험을 "겪은" 캐릭터)
    pub subject: CharacterId,
    /// 경험 발생 시점
    pub time: GameTime,
    /// 경험 발생 장소
    pub location: LocationId,
    /// 경험의 중요도 (1.0~10.0). 기억 검색 가중치에 사용.
    pub importance: f32,
}

impl ExperienceHeader {
    /// 새 ExperienceHeader 생성.
    pub fn new(
        experience_id: ExperienceId,
        subject: CharacterId,
        time: GameTime,
        location: LocationId,
        importance: f32,
    ) -> Self {
        Self {
            experience_id,
            subject,
            time,
            location,
            importance: if importance.is_nan() { 1.0 } else { importance.clamp(1.0, 10.0) },
        }
    }
}

// ---------------------------------------------------------------------------
// CombatResult — 전투 결과
// ---------------------------------------------------------------------------

/// 전투 결과. 전투 도메인이 아직 없으므로 experience 모듈에 임시 배치.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatResult {
    /// 승리
    Victory,
    /// 패배
    Defeat,
    /// 무승부
    Draw,
    /// 도주
    Fled,
}

// ---------------------------------------------------------------------------
// ExperienceEvent — 경험 이벤트 (큐에 들어가는 유일한 타입)
// ---------------------------------------------------------------------------

/// 경험 이벤트 — 큐에 들어가는 유일한 타입.
///
/// 각 variant는 게임 내에서 발생할 수 있는 하나의 "경험"을 나타낸다.
/// 핸들러(EventHandler)가 이 이벤트를 받아 각 도메인의 상태를 갱신한다.
///
/// 경험 = 기억 = 이벤트 (유일한 진실)
///
/// # 비동기 결과도 ExperienceEvent
/// LLM 감정 판정 완료 → `Observation`으로 큐에 넣음.
/// 대화 요약 완료 → `ConversationSummarized`로 큐에 넣음.
/// 패턴이 하나. 예외 없음.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExperienceEvent {
    /// 수련 — 무공을 연마한 경험
    Training {
        header: ExperienceHeader,
        /// 수련한 무공
        skill: MartialArtId,
        /// 수련 방법 (좌선, 대련, 연습 등)
        #[serde(default)]
        method: String,
        /// 사부 (있을 경우)
        mentor: Option<CharacterId>,
        /// 함께 수련한 동료
        companion: Option<CharacterId>,
        /// 수련 시간 (시간대 수, 1~6)
        duration: u32,
        /// 수련 강도 (1~10)
        intensity: u32,
    },

    /// 전투 — 대결한 경험
    Combat {
        header: ExperienceHeader,
        /// 상대방
        opponent: CharacterId,
        /// 전투 결과
        result: CombatResult,
        /// 사용한 무공
        technique_used: Option<MartialArtId>,
        /// 상대가 사용한 무공
        technique_faced: Option<MartialArtId>,
    },

    /// 대화 — NPC와의 대화 (원시 데이터만, 요약은 별도 이벤트)
    Conversation {
        header: ExperienceHeader,
        /// 대화 상대
        counterpart: CharacterId,
        /// 대화 턴 수
        turns: u32,
        /// 원시 대화 내용
        raw_dialogue: String,
    },

    /// 대화 요약 완료 — 비동기 요약 태스크 결과 (Phase 2에서 활용)
    ConversationSummarized {
        header: ExperienceHeader,
        /// 원본 대화의 experience_id
        original_experience_id: ExperienceId,
        /// LLM이 생성한 요약
        summary: String,
        /// 요약 기반 재평가된 중요도
        #[serde(default)]
        revised_importance: Option<f32>,
    },

    /// 관찰 — 직접 목격하거나 감정 판정 결과
    Observation {
        header: ExperienceHeader,
        /// 관찰 대상 (없으면 주변 환경)
        target: Option<CharacterId>,
        /// 관찰 내용
        what: String,
        /// 감정 판정에 의한 호감도 변화량 (극단 트리거 결과)
        sentiment_delta: Option<f32>,
    },

    /// 거래 — 물품을 교환한 경험
    Trade {
        header: ExperienceHeader,
        /// 거래 상대
        counterpart: CharacterId,
        /// 거래 품목
        items: Vec<ItemId>,
        /// 거래 공정성 (-1.0~1.0, 0이 공정)
        fairness: f32,
    },

    /// 이동 — 다른 장소로 이동한 경험
    Travel {
        header: ExperienceHeader,
        /// 목적지
        destination: LocationId,
        /// 동행자
        companion: Option<CharacterId>,
        /// 이동 소요 시간 (시간대 수)
        duration: u32,
    },

    /// 구출 — 누군가를 위험에서 구한 경험
    Rescue {
        header: ExperienceHeader,
        /// 구출한 대상
        saved: CharacterId,
        /// 위험 상황 설명
        danger: String,
        /// 감수한 위험도 (0.0~1.0)
        risk_taken: f32,
    },

    /// 배신 — 배신을 당하거나 목격한 경험
    Betrayal {
        header: ExperienceHeader,
        /// 배신자
        betrayer: CharacterId,
        /// 배신당한 자
        betrayed: CharacterId,
        /// 배신 유형 (정보 누설, 독살 시도 등)
        betrayal_type: String,
    },

    /// 돌봄 — 부상자를 간호한 경험
    Care {
        header: ExperienceHeader,
        /// 환자
        patient: CharacterId,
        /// 간호자
        caregiver: CharacterId,
    },

    /// 선물 — 물건을 주고받은 경험
    Gift {
        header: ExperienceHeader,
        /// 주는 사람
        giver: CharacterId,
        /// 받는 사람
        receiver: CharacterId,
        /// 선물 품목
        item: ItemId,
    },

    /// 휴식 — 쉰 경험
    Rest {
        header: ExperienceHeader,
        /// 휴식 방법 (명상, 수면 등)
        #[serde(default)]
        method: String,
        /// 회복량 (0.0~1.0)
        recovery: f32,
    },

    /// 시간 경과 — 특별한 일 없이 시간이 흐른 경험
    TimePassage {
        header: ExperienceHeader,
        /// 경과 시간 (시간대 수)
        duration: u32,
        /// 교류 없이 보낸 시간인지
        #[serde(default)]
        without_contact: bool,
    },
}

impl ExperienceEvent {
    /// 경험 이벤트의 공통 헤더에 접근한다.
    pub fn header(&self) -> &ExperienceHeader {
        match self {
            Self::Training { header, .. }
            | Self::Combat { header, .. }
            | Self::Conversation { header, .. }
            | Self::ConversationSummarized { header, .. }
            | Self::Observation { header, .. }
            | Self::Trade { header, .. }
            | Self::Travel { header, .. }
            | Self::Rescue { header, .. }
            | Self::Betrayal { header, .. }
            | Self::Care { header, .. }
            | Self::Gift { header, .. }
            | Self::Rest { header, .. }
            | Self::TimePassage { header, .. } => header,
        }
    }

    /// 로깅/디버깅용 이벤트 이름.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Training { .. } => "ExpTraining",
            Self::Combat { .. } => "ExpCombat",
            Self::Conversation { .. } => "ExpConversation",
            Self::ConversationSummarized { .. } => "ExpConversationSummarized",
            Self::Observation { .. } => "ExpObservation",
            Self::Trade { .. } => "ExpTrade",
            Self::Travel { .. } => "ExpTravel",
            Self::Rescue { .. } => "ExpRescue",
            Self::Betrayal { .. } => "ExpBetrayal",
            Self::Care { .. } => "ExpCare",
            Self::Gift { .. } => "ExpGift",
            Self::Rest { .. } => "ExpRest",
            Self::TimePassage { .. } => "ExpTimePassage",
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_header(id: u64) -> ExperienceHeader {
        ExperienceHeader::new(
            ExperienceId::new(id),
            CharacterId::new(1),
            GameTime::new(1200, 3, 15),
            LocationId::new(10),
            5.0,
        )
    }

    // -- ExperienceHeader --

    #[test]
    fn header_creation() {
        let h = make_header(1);
        assert_eq!(h.experience_id, ExperienceId::new(1));
        assert_eq!(h.subject, CharacterId::new(1));
        assert_eq!(h.importance, 5.0);
    }

    #[test]
    fn header_importance_clamped() {
        // 중요도는 1.0~10.0 범위로 클램핑
        let low = ExperienceHeader::new(
            ExperienceId::new(1),
            CharacterId::new(1),
            GameTime::new(1200, 1, 1),
            LocationId::new(1),
            -5.0,
        );
        assert_eq!(low.importance, 1.0);

        let high = ExperienceHeader::new(
            ExperienceId::new(2),
            CharacterId::new(1),
            GameTime::new(1200, 1, 1),
            LocationId::new(1),
            99.0,
        );
        assert_eq!(high.importance, 10.0);
    }

    // -- ExperienceEvent variants --

    #[test]
    fn training_event() {
        let event = ExperienceEvent::Training {
            header: make_header(1),
            skill: MartialArtId::new(5),
            method: "좌선".to_string(),
            mentor: Some(CharacterId::new(2)),
            companion: None,
            duration: 3,
            intensity: 7,
        };
        assert_eq!(event.name(), "ExpTraining");
        assert_eq!(event.header().experience_id, ExperienceId::new(1));
    }

    #[test]
    fn combat_event() {
        let event = ExperienceEvent::Combat {
            header: make_header(2),
            opponent: CharacterId::new(3),
            result: CombatResult::Victory,
            technique_used: Some(MartialArtId::new(1)),
            technique_faced: None,
        };
        assert_eq!(event.name(), "ExpCombat");
    }

    #[test]
    fn conversation_event() {
        let event = ExperienceEvent::Conversation {
            header: make_header(3),
            counterpart: CharacterId::new(5),
            turns: 12,
            raw_dialogue: "소연: 안녕하세요.\n플레이어: 반갑습니다.".to_string(),
        };
        assert_eq!(event.name(), "ExpConversation");
    }

    #[test]
    fn conversation_summarized_event() {
        let event = ExperienceEvent::ConversationSummarized {
            header: make_header(4),
            original_experience_id: ExperienceId::new(3),
            summary: "소연과 인사를 나눔".to_string(),
            revised_importance: Some(3.0),
        };
        assert_eq!(event.name(), "ExpConversationSummarized");
    }

    #[test]
    fn observation_event() {
        let event = ExperienceEvent::Observation {
            header: make_header(5),
            target: Some(CharacterId::new(5)),
            what: "소연이 사부 죽음 언급에 극도로 분노".to_string(),
            sentiment_delta: Some(-9.0),
        };
        assert_eq!(event.name(), "ExpObservation");
        assert_eq!(event.header().subject, CharacterId::new(1));
    }

    #[test]
    fn trade_event() {
        let event = ExperienceEvent::Trade {
            header: make_header(6),
            counterpart: CharacterId::new(7),
            items: vec![ItemId::new(1), ItemId::new(2)],
            fairness: 0.5,
        };
        assert_eq!(event.name(), "ExpTrade");
    }

    #[test]
    fn travel_event() {
        let event = ExperienceEvent::Travel {
            header: make_header(7),
            destination: LocationId::new(20),
            companion: Some(CharacterId::new(3)),
            duration: 4,
        };
        assert_eq!(event.name(), "ExpTravel");
    }

    #[test]
    fn rescue_event() {
        let event = ExperienceEvent::Rescue {
            header: make_header(8),
            saved: CharacterId::new(5),
            danger: "절벽에서 떨어질 위기".to_string(),
            risk_taken: 0.8,
        };
        assert_eq!(event.name(), "ExpRescue");
    }

    #[test]
    fn betrayal_event() {
        let event = ExperienceEvent::Betrayal {
            header: make_header(9),
            betrayer: CharacterId::new(4),
            betrayed: CharacterId::new(1),
            betrayal_type: "정보 누설".to_string(),
        };
        assert_eq!(event.name(), "ExpBetrayal");
    }

    #[test]
    fn care_event() {
        let event = ExperienceEvent::Care {
            header: make_header(10),
            patient: CharacterId::new(1),
            caregiver: CharacterId::new(5),
        };
        assert_eq!(event.name(), "ExpCare");
    }

    #[test]
    fn gift_event() {
        let event = ExperienceEvent::Gift {
            header: make_header(11),
            giver: CharacterId::new(5),
            receiver: CharacterId::new(1),
            item: ItemId::new(42),
        };
        assert_eq!(event.name(), "ExpGift");
    }

    #[test]
    fn rest_event() {
        let event = ExperienceEvent::Rest {
            header: make_header(12),
            method: "명상".to_string(),
            recovery: 0.6,
        };
        assert_eq!(event.name(), "ExpRest");
    }

    #[test]
    fn time_passage_event() {
        let event = ExperienceEvent::TimePassage {
            header: make_header(13),
            duration: 6,
            without_contact: true,
        };
        assert_eq!(event.name(), "ExpTimePassage");
    }

    // -- CombatResult --

    #[test]
    fn combat_result_variants() {
        assert_ne!(CombatResult::Victory, CombatResult::Defeat);
        assert_ne!(CombatResult::Draw, CombatResult::Fled);
    }

    // -- header() accessor --

    #[test]
    fn header_accessor_returns_correct_reference() {
        let event = ExperienceEvent::Rest {
            header: make_header(99),
            method: String::new(),
            recovery: 0.5,
        };
        assert_eq!(event.header().experience_id, ExperienceId::new(99));
        assert_eq!(event.header().location, LocationId::new(10));
    }

    // -- Serialization --

    #[test]
    fn serialization_roundtrip() {
        let event = ExperienceEvent::Training {
            header: make_header(1),
            skill: MartialArtId::new(5),
            method: "대련".to_string(),
            mentor: None,
            companion: None,
            duration: 2,
            intensity: 5,
        };
        let json = serde_json::to_string(&event).unwrap();
        let restored: ExperienceEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, restored);
    }

    #[test]
    fn header_serialization_roundtrip() {
        let header = make_header(42);
        let json = serde_json::to_string(&header).unwrap();
        let restored: ExperienceHeader = serde_json::from_str(&json).unwrap();
        assert_eq!(header, restored);
    }

    #[test]
    fn combat_result_serialization() {
        let result = CombatResult::Fled;
        let json = serde_json::to_string(&result).unwrap();
        let restored: CombatResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, restored);
    }

    // -- Clone --

    #[test]
    fn clone_event() {
        let event = ExperienceEvent::Observation {
            header: make_header(1),
            target: Some(CharacterId::new(5)),
            what: "수상한 움직임".to_string(),
            sentiment_delta: None,
        };
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    // -- name() 전수 검증 --

    #[test]
    fn all_variant_names() {
        let events: Vec<ExperienceEvent> = vec![
            ExperienceEvent::Training {
                header: make_header(1), skill: MartialArtId::new(1), method: String::new(),
                mentor: None, companion: None, duration: 1, intensity: 1,
            },
            ExperienceEvent::Combat {
                header: make_header(2), opponent: CharacterId::new(2),
                result: CombatResult::Draw, technique_used: None, technique_faced: None,
            },
            ExperienceEvent::Conversation {
                header: make_header(3), counterpart: CharacterId::new(3),
                turns: 1, raw_dialogue: String::new(),
            },
            ExperienceEvent::ConversationSummarized {
                header: make_header(4), original_experience_id: ExperienceId::new(3),
                summary: String::new(), revised_importance: None,
            },
            ExperienceEvent::Observation {
                header: make_header(5), target: None, what: String::new(),
                sentiment_delta: None,
            },
            ExperienceEvent::Trade {
                header: make_header(6), counterpart: CharacterId::new(6),
                items: vec![], fairness: 0.0,
            },
            ExperienceEvent::Travel {
                header: make_header(7), destination: LocationId::new(1),
                companion: None, duration: 1,
            },
            ExperienceEvent::Rescue {
                header: make_header(8), saved: CharacterId::new(8),
                danger: String::new(), risk_taken: 0.0,
            },
            ExperienceEvent::Betrayal {
                header: make_header(9), betrayer: CharacterId::new(9),
                betrayed: CharacterId::new(1), betrayal_type: String::new(),
            },
            ExperienceEvent::Care {
                header: make_header(10), patient: CharacterId::new(1),
                caregiver: CharacterId::new(10),
            },
            ExperienceEvent::Gift {
                header: make_header(11), giver: CharacterId::new(11),
                receiver: CharacterId::new(1), item: ItemId::new(1),
            },
            ExperienceEvent::Rest {
                header: make_header(12), method: String::new(), recovery: 0.5,
            },
            ExperienceEvent::TimePassage {
                header: make_header(13), duration: 1, without_contact: false,
            },
        ];

        let expected = vec![
            "ExpTraining", "ExpCombat", "ExpConversation", "ExpConversationSummarized",
            "ExpObservation", "ExpTrade", "ExpTravel", "ExpRescue", "ExpBetrayal",
            "ExpCare", "ExpGift", "ExpRest", "ExpTimePassage",
        ];

        for (event, name) in events.iter().zip(expected.iter()) {
            assert_eq!(event.name(), *name);
        }
    }
}
