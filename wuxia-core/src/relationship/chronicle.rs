// wuxia-core/src/relationship/chronicle.rs
//
// RelationshipChronicle — 관계 변화 연대기 (강호 인연록).
//
// 게임 세계의 객관적 기록. NPC의 주관적 기억(Observation)과 구분된다.
// "1200년 3월 2일, 소연의 호감도가 23에서 21로 떨어졌다" = 사실(Chronicle).
// "그때 그 무례한 질문이 싫었다" = 기억(Observation).
//
// Port & Adapter 패턴:
//   trait ChronicleRepository (Port) ← InMemoryChronicleRepo / JsonlChronicleRepo (Adapter)

use serde::{Deserialize, Serialize};

use crate::shared::id::CharacterId;
use crate::shared::time::GameTime;

// ---------------------------------------------------------------------------
// 값 객체 — ChangeType, CauseSource
// ---------------------------------------------------------------------------

/// 관계 변화의 종류.
///
/// # Example
/// ```
/// use wuxia_core::relationship::ChangeType;
///
/// let ct = ChangeType::Affinity { old: 23.0, new: 21.0 };
/// assert_eq!(ct.variant_name(), "Affinity");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChangeType {
    /// 호감도 변화.
    Affinity { old: f32, new: f32 },
    /// 신뢰도 변화.
    Trust { old: f32, new: f32 },
    /// 관계 레벨 전환 (e.g., Stranger → Acquaintance).
    LevelChanged {
        old_level: String,
        new_level: String,
    },
    /// 관계 유형 변경 (e.g., None → MasterDisciple).
    TypeChanged {
        old_type: Option<String>,
        new_type: Option<String>,
    },
    /// 관계 단절.
    BondBroken { reason: String },
}

impl ChangeType {
    /// 변형의 이름을 문자열로 반환한다. `find_by_change_type` 필터에 사용.
    ///
    /// ```
    /// use wuxia_core::relationship::ChangeType;
    ///
    /// assert_eq!(ChangeType::Affinity { old: 0.0, new: 1.0 }.variant_name(), "Affinity");
    /// assert_eq!(ChangeType::Trust { old: 0.0, new: 1.0 }.variant_name(), "Trust");
    /// assert_eq!(
    ///     ChangeType::LevelChanged { old_level: "A".into(), new_level: "B".into() }.variant_name(),
    ///     "LevelChanged",
    /// );
    /// ```
    pub fn variant_name(&self) -> &'static str {
        match self {
            ChangeType::Affinity { .. } => "Affinity",
            ChangeType::Trust { .. } => "Trust",
            ChangeType::LevelChanged { .. } => "LevelChanged",
            ChangeType::TypeChanged { .. } => "TypeChanged",
            ChangeType::BondBroken { .. } => "BondBroken",
        }
    }
}

/// 변화의 원인 분류.
///
/// ```
/// use wuxia_core::relationship::CauseSource;
///
/// let cs = CauseSource::Conversation;
/// let json = serde_json::to_string(&cs).unwrap();
/// assert_eq!(json, "\"Conversation\"");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CauseSource {
    /// 대화에서 발생.
    Conversation,
    /// 행동에서 발생 (호위, 구출 등).
    Action,
    /// 세계 이벤트 (전쟁, 재해 등).
    Event,
    /// 시간 경과에 의한 자연 변화.
    TimePassage,
    /// 제3자의 영향.
    ThirdParty,
}

// ---------------------------------------------------------------------------
// 도메인 모델 — RelationshipChronicle
// ---------------------------------------------------------------------------

/// 관계 변화 연대기 한 건.
///
/// 비유: 강호 인연록(因緣錄)의 한 줄.
/// "1200년 3월 2일 저녁, 자유도시 주막에서
///  소연(5)이 플레이어(0)에 대한 호감이 23에서 21로 떨어졌다.
///  사유: 사부에 대해 무례하게 물음."
///
/// # Example
/// ```
/// use wuxia_core::relationship::{RelationshipChronicle, ChangeType, CauseSource};
/// use wuxia_core::shared::id::CharacterId;
/// use wuxia_core::shared::time::GameTime;
///
/// let chronicle = RelationshipChronicle {
///     seq: 1,
///     session_id: "s_001".to_string(),
///     schema_ver: 1,
///     source: CharacterId::new(5),
///     target: CharacterId::new(0),
///     game_time: GameTime::new(1200, 3, 15),
///     game_watch: Some("Evening".to_string()),
///     location: None,
///     change_type: ChangeType::Affinity { old: 0.0, new: 2.0 },
///     cause: "인사를 건넴".to_string(),
///     cause_source: CauseSource::Conversation,
///     delta_source: None,
///     event_group: None,
///     witnesses: vec![],
/// };
/// assert_eq!(chronicle.source, CharacterId::new(5));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipChronicle {
    // ── 메타 (시스템 관리) ──
    /// 전역 일련번호 (고유성 + 정렬).
    pub seq: u64,
    /// 대화 세션 묶음.
    pub session_id: String,
    /// 포맷 버전 (마이그레이션용, 초기값 1).
    pub schema_ver: u32,

    // ── 누가 → 누구에게 ──
    /// 관계 주체 (e.g., 소연 5).
    pub source: CharacterId,
    /// 관계 대상 (e.g., 플레이어 0).
    pub target: CharacterId,

    // ── 언제 ──
    /// 게임 시간 (년, 월, 일).
    pub game_time: GameTime,
    /// 시간대 (선택, e.g., "Evening").
    pub game_watch: Option<String>,

    // ── 어디서 ──
    /// 장소 (선택, e.g., "자유도시 주막").
    pub location: Option<String>,

    // ── 무엇이 변했나 ──
    /// 변화 종류.
    pub change_type: ChangeType,

    // ── 왜 ──
    /// 변화 사유 (자연어).
    pub cause: String,
    /// 원인 분류.
    pub cause_source: CauseSource,
    /// delta 산출 출처 (e.g., "LlmTriggeredJudgment"). 판정 기반일 때만.
    pub delta_source: Option<String>,

    // ── 연결 정보 ──
    /// 같은 사건의 첫 seq 참조 (선택).
    pub event_group: Option<u64>,
    /// 목격자 NPC (없으면 빈 배열).
    pub witnesses: Vec<CharacterId>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn soyeon() -> CharacterId {
        CharacterId::new(5)
    }
    fn player() -> CharacterId {
        CharacterId::new(0)
    }
    fn game_time() -> GameTime {
        GameTime::new(1200, 3, 15)
    }

    fn make_chronicle(seq: u64, change_type: ChangeType) -> RelationshipChronicle {
        RelationshipChronicle {
            seq,
            session_id: "s_001".to_string(),
            schema_ver: 1,
            source: soyeon(),
            target: player(),
            game_time: game_time(),
            game_watch: None,
            location: None,
            change_type,
            cause: "테스트".to_string(),
            cause_source: CauseSource::Conversation,
            delta_source: None,
            event_group: None,
            witnesses: vec![],
        }
    }

    #[test]
    fn chronicle_creation() {
        let c = make_chronicle(1, ChangeType::Affinity { old: 0.0, new: 2.0 });
        assert_eq!(c.seq, 1);
        assert_eq!(c.source, soyeon());
        assert_eq!(c.target, player());
        assert_eq!(c.schema_ver, 1);
        assert_eq!(c.session_id, "s_001");
    }

    #[test]
    fn change_type_variant_name() {
        assert_eq!(
            ChangeType::Affinity { old: 0.0, new: 1.0 }.variant_name(),
            "Affinity"
        );
        assert_eq!(
            ChangeType::Trust { old: 0.0, new: 1.0 }.variant_name(),
            "Trust"
        );
        assert_eq!(
            ChangeType::LevelChanged {
                old_level: "Stranger".into(),
                new_level: "Acquaintance".into()
            }
            .variant_name(),
            "LevelChanged"
        );
        assert_eq!(
            ChangeType::TypeChanged {
                old_type: None,
                new_type: Some("Rivals".into())
            }
            .variant_name(),
            "TypeChanged"
        );
        assert_eq!(
            ChangeType::BondBroken {
                reason: "배신".into()
            }
            .variant_name(),
            "BondBroken"
        );
    }

    #[test]
    fn change_type_affinity_serde_roundtrip() {
        let ct = ChangeType::Affinity { old: 23.0, new: 21.0 };
        let json = serde_json::to_string(&ct).unwrap();
        let parsed: ChangeType = serde_json::from_str(&json).unwrap();
        assert_eq!(ct, parsed);
    }

    #[test]
    fn change_type_level_changed_serde_roundtrip() {
        let ct = ChangeType::LevelChanged {
            old_level: "Stranger".to_string(),
            new_level: "Acquaintance".to_string(),
        };
        let json = serde_json::to_string(&ct).unwrap();
        let parsed: ChangeType = serde_json::from_str(&json).unwrap();
        assert_eq!(ct, parsed);
    }

    #[test]
    fn cause_source_all_variants_serde_roundtrip() {
        let variants = [
            CauseSource::Conversation,
            CauseSource::Action,
            CauseSource::Event,
            CauseSource::TimePassage,
            CauseSource::ThirdParty,
        ];
        for cs in &variants {
            let json = serde_json::to_string(cs).unwrap();
            let parsed: CauseSource = serde_json::from_str(&json).unwrap();
            assert_eq!(cs, &parsed);
        }
    }

    #[test]
    fn chronicle_full_serde_roundtrip() {
        let c = RelationshipChronicle {
            seq: 42,
            session_id: "s_003".to_string(),
            schema_ver: 1,
            source: soyeon(),
            target: player(),
            game_time: game_time(),
            game_watch: Some("Evening".to_string()),
            location: Some("자유도시 주막".to_string()),
            change_type: ChangeType::Affinity { old: 23.0, new: 21.0 },
            cause: "사부에 대해 무례하게 물음".to_string(),
            cause_source: CauseSource::Conversation,
            delta_source: Some("LlmTriggeredJudgment".to_string()),
            event_group: Some(41),
            witnesses: vec![CharacterId::new(4)],
        };

        let json = serde_json::to_string(&c).unwrap();
        let parsed: RelationshipChronicle = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.seq, 42);
        assert_eq!(parsed.session_id, "s_003");
        assert_eq!(parsed.source, soyeon());
        assert_eq!(parsed.target, player());
        assert_eq!(parsed.game_watch, Some("Evening".to_string()));
        assert_eq!(parsed.location, Some("자유도시 주막".to_string()));
        assert_eq!(
            parsed.change_type,
            ChangeType::Affinity { old: 23.0, new: 21.0 }
        );
        assert_eq!(parsed.cause, "사부에 대해 무례하게 물음");
        assert_eq!(parsed.cause_source, CauseSource::Conversation);
        assert_eq!(
            parsed.delta_source,
            Some("LlmTriggeredJudgment".to_string())
        );
        assert_eq!(parsed.event_group, Some(41));
        assert_eq!(parsed.witnesses, vec![CharacterId::new(4)]);
    }

    #[test]
    fn chronicle_with_optional_fields_none() {
        // 선택 필드가 모두 None인 경우도 직렬화/역직렬화 성공
        let c = make_chronicle(1, ChangeType::Affinity { old: 0.0, new: 1.0 });
        let json = serde_json::to_string(&c).unwrap();
        let parsed: RelationshipChronicle = serde_json::from_str(&json).unwrap();
        assert!(parsed.game_watch.is_none());
        assert!(parsed.location.is_none());
        assert!(parsed.delta_source.is_none());
        assert!(parsed.event_group.is_none());
        assert!(parsed.witnesses.is_empty());
    }
}
