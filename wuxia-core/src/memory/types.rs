// wuxia-core/src/memory/types.rs
//
// Memory Stream 데이터 타입 — NPC 기억의 기본 구조.
//
// Stanford Generative Agents 논문의 Memory Stream을 무협 RPG에 적용.
// 이 모듈은 순수 데이터 구조만 포함한다 (로직 없음).
//
// 기억의 세 종류 (MemoryType):
//   Observation (관찰) — NPC가 직접 겪은 사건의 기록
//     예: "화산파 장로가 나를 꾸짖었다"
//     생성: ⑦층 Tier 1 (코드, 매 이벤트)
//
//   Reflection (성찰) — 관찰에서 추출한 고차원 통찰
//     예: "나는 화산파에서 환멸을 느끼고 있다"
//     생성: ⑦층 Tier 2~4 (LLM, 하루 끝/중대 사건/고비)
//
//   Plan (계획) — 성찰에서 도출한 행동 의도
//     예: "내일 밤 몰래 떠나겠다"
//     생성: ⑦층 Tier 2~4 (LLM)
//
// 성찰 트리 구조:
//   Observation(리프) → Reflection(중간) → 고차 Reflection(상위)
//   source_ids로 "이 성찰이 어떤 기억에서 왔는가"를 추적한다.
//
// 도메인 소유 관계:
//   타입 정의: memory/ (이 모듈, llm/과 대칭)
//   비즈니스 규칙: psychology/ (성찰 시 기억 재평가 등)
//   저장소 구현: wuxia-memory/ (InMemory, LanceDB)

use serde::{Deserialize, Serialize};

use crate::shared::id::{CharacterId, MemoryId};
use crate::shared::time::GameTime;

// ---------------------------------------------------------------------------
// MemoryType — 기억의 종류
// ---------------------------------------------------------------------------

/// NPC 기억의 세 가지 종류.
///
/// 각 종류는 심리 아키텍처 ⑦층(성찰)의 서로 다른 단계에서 생성된다.
///
/// # 성찰 트리에서의 위치
/// ```text
///   Observation (리프)  → "사부가 다쳤다"
///        ↓
///   Reflection (중간)   → "나는 사부를 지키지 못했다"
///        ↓
///   Reflection (상위)   → "힘 없는 정의는 허상이다"
///        ↓
///   Plan                → "내일부터 새벽마다 검법을 연마하겠다"
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryType {
    /// 관찰 — NPC가 직접 인지한 사건. 성찰 트리의 리프 노드.
    /// Tier 1(코드)에서 매 이벤트마다 생성된다.
    Observation,

    /// 성찰 — 관찰로부터 추출한 고차원 통찰. 성찰 트리의 중간/상위 노드.
    /// Tier 2~4(LLM)에서 생성되며, source_ids로 근거 기억을 참조한다.
    Reflection,

    /// 계획 — 성찰에서 도출한 행동 의도.
    /// Tier 2~4(LLM)에서 생성되며, ⑥행동 층에 영향을 준다.
    Plan,
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryType::Observation => write!(f, "Observation"),
            MemoryType::Reflection => write!(f, "Reflection"),
            MemoryType::Plan => write!(f, "Plan"),
        }
    }
}

// ---------------------------------------------------------------------------
// MemoryEntry — 기억 하나
// ---------------------------------------------------------------------------

/// NPC 기억 스트림의 한 항목.
///
/// Generative Agents 논문의 Memory Object를 무협 RPG에 맞게 확장.
/// 심리 도메인의 3축 가치관(②층)의 형성기억(formation memories)이
/// 이 MemoryEntry의 ID를 참조한다.
///
/// # Sprint 2 MVP vs 향후 확장
/// - MVP: content, importance, keywords로 기본 저장/검색
/// - Phase B: embedding 벡터는 wuxia-memory(LanceDB)에서 별도 관리
/// - 향후: source_ids, reflection_tier는 심리 도메인 성찰 구현 시 활용
///
/// # Example
/// ```
/// use wuxia_core::memory::{MemoryEntry, MemoryType};
/// use wuxia_core::shared::id::{CharacterId, MemoryId};
/// use wuxia_core::shared::GameTime;
///
/// let entry = MemoryEntry::new(
///     MemoryId::new(1),
///     CharacterId::new(5),  // 소연
///     "자유도시 시장에서 수상한 사내를 보았다".to_string(),
///     7.0,
///     MemoryType::Observation,
///     GameTime::new(1200, 3, 15),
///     vec!["자유도시".to_string(), "수상한 사내".to_string()],
/// );
/// assert_eq!(entry.character_id(), CharacterId::new(5));
/// assert_eq!(entry.importance(), 7.0);
/// assert!(entry.source_ids().is_empty());  // MVP에서는 비어있음
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// 기억의 고유 식별자.
    id: MemoryId,

    /// 이 기억을 소유한 NPC의 ID.
    character_id: CharacterId,

    /// 기억의 내용 (자연어 텍스트).
    /// 예: "화산파 장로가 나를 꾸짖었다"
    content: String,

    /// 중요도 (1.0~10.0). LLM이 평가하거나, 성찰 시 재평가 가능.
    /// f32인 이유: Tier 2 일상 성찰에서 기억 중요도를 재평가할 수 있음.
    /// 예: "방 청소" → 2.0, "사부의 죽음" → 10.0
    importance: f32,

    /// 기억의 종류 (관찰/성찰/계획).
    memory_type: MemoryType,

    /// 이 기억이 생성된 게임 시간.
    game_time: GameTime,

    /// 검색용 키워드. Phase A에서 키워드 매칭 검색에 사용.
    /// Phase B(LanceDB)에서는 벡터 유사도로 대체되지만, 키워드도 보조적으로 유지.
    keywords: Vec<String>,

    /// 이 기억의 근거가 된 기억들의 ID (성찰 트리 지원).
    /// Observation: 비어있음 (리프 노드)
    /// Reflection: 근거가 된 Observation/Reflection의 ID 목록
    /// Plan: 근거가 된 Reflection의 ID 목록
    ///
    /// Sprint 2 MVP에서는 Vec::new()로 비어있음. 심리 도메인 구현 시 활용.
    source_ids: Vec<MemoryId>,

    /// Reflection일 때: 어느 Tier에서 생성되었는가 (2, 3, 4).
    /// Observation/Plan이거나 아직 미설정이면 None.
    ///
    /// Sprint 2 MVP에서는 None. 심리 도메인 구현 시 활용.
    reflection_tier: Option<u8>,

    /// 기억의 언어 코드. "KO" | "EN" | "ZH".
    /// 언어별 검색 threshold를 적용할 때 사용한다.
    /// 게임 주 언어가 한국어이므로 기본값은 "KO".
    ///
    /// serde(default): 기존 직렬화 데이터에 lang 필드가 없으면 "KO"로 역직렬화.
    #[serde(default = "default_lang")]
    lang: String,
}

/// [v4.5] 기억의 중요도 수준. 1.0~10.0 점수를 도메인 단계로 분류한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MemoryImportance {
    /// 사소함 (1.0 ~ 3.9)
    Trivial,
    /// 보통 (4.0 ~ 6.9)
    Common,
    /// 중요 (7.0 ~ 8.9)
    Significant,
    /// 치명적 (9.0 ~ 10.0)
    Critical,
}

impl MemoryImportance {
    pub fn from_score(score: f32) -> Self {
        if score >= 9.0 { MemoryImportance::Critical }
        else if score >= 7.0 { MemoryImportance::Significant }
        else if score >= 4.0 { MemoryImportance::Common }
        else { MemoryImportance::Trivial }
    }
}

/// serde(default) 함수: lang 필드의 기본값 "KO".
fn default_lang() -> String {
    "KO".to_string()
}

impl MemoryEntry {
    /// 기억을 자연어로 설명한다 (도메인 책임).
    ///
    /// # Arguments
    /// * `now` - 현재 게임 시간
    /// * `today_label` - "오늘" 등을 나타내는 로컬라이즈 문자열
    /// * `days_ago_template` - "{n}일 전" 등을 나타내는 템플릿
    /// * `importance_labels` - 중요도 단계별 로컬라이즈 라벨 맵
    pub fn describe(
        &self,
        now: GameTime,
        today_label: &str,
        days_ago_template: &str,
        importance_labels: &std::collections::HashMap<MemoryImportance, String>,
    ) -> String {
        let days_ago = now.days_between(&self.game_time).unsigned_abs();
        let time_desc = if days_ago == 0 {
            today_label.to_string()
        } else {
            days_ago_template.replace("{n}", &days_ago.to_string())
        };

        let imp_level = MemoryImportance::from_score(self.importance);
        let imp_label = importance_labels.get(&imp_level).cloned().unwrap_or_default();

        format!("({}) {}\n  [{}]", time_desc, self.content, imp_label)
    }

    /// 기본 생성자 (MVP용). source_ids와 reflection_tier는 빈 기본값.
    /// lang은 "KO"로 기본 설정된다 (하위 호환).
    pub fn new(
        id: MemoryId,
        character_id: CharacterId,
        content: String,
        importance: f32,
        memory_type: MemoryType,
        game_time: GameTime,
        keywords: Vec<String>,
    ) -> Self {
        Self {
            id,
            character_id,
            content,
            importance: importance.clamp(1.0, 10.0),
            memory_type,
            game_time,
            keywords,
            lang: "KO".to_string(),
            source_ids: Vec::new(),
            reflection_tier: None,
        }
    }

    /// 성찰 트리 정보를 포함한 생성자 (심리 도메인용).
    /// lang은 "KO"로 기본 설정된다. set_lang()으로 변경 가능.
    pub fn with_sources(
        id: MemoryId,
        character_id: CharacterId,
        content: String,
        importance: f32,
        memory_type: MemoryType,
        game_time: GameTime,
        keywords: Vec<String>,
        source_ids: Vec<MemoryId>,
        reflection_tier: Option<u8>,
    ) -> Self {
        Self {
            id,
            character_id,
            content,
            importance: importance.clamp(1.0, 10.0),
            memory_type,
            game_time,
            keywords,
            lang: default_lang(),
            source_ids,
            reflection_tier,
        }
    }

    // --- Getters ---

    pub fn id(&self) -> MemoryId {
        self.id
    }

    pub fn character_id(&self) -> CharacterId {
        self.character_id
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn importance(&self) -> f32 {
        self.importance
    }

    pub fn memory_type(&self) -> MemoryType {
        self.memory_type
    }

    pub fn game_time(&self) -> GameTime {
        self.game_time
    }

    pub fn keywords(&self) -> &[String] {
        &self.keywords
    }

    pub fn lang(&self) -> &str {
        &self.lang
    }

    pub fn source_ids(&self) -> &[MemoryId] {
        &self.source_ids
    }

    pub fn reflection_tier(&self) -> Option<u8> {
        self.reflection_tier
    }

    // --- Mutators (controlled) ---

    /// 성찰 시 기억 중요도를 재평가한다.
    /// Tier 2 일상 성찰에서 사용.
    pub fn update_importance(&mut self, new_importance: f32) {
        self.importance = new_importance.clamp(1.0, 10.0);
    }

    /// 기억의 언어 코드를 설정한다.
    /// LanceDB batch_to_entries() 등에서 DB 컬럼 값을 복원할 때 사용.
    pub fn set_lang(&mut self, lang: &str) {
        self.lang = lang.to_string();
    }
}

// ---------------------------------------------------------------------------
// ScoredMemory — 검색 결과
// ---------------------------------------------------------------------------

/// 검색 결과: 기억 + 관련성 점수.
///
/// MemoryRepository::search()가 반환하는 타입.
/// relevance_score는 구현체에 따라 다르게 계산된다:
///   - InMemoryRepository: 키워드 매칭 (0.0 or 1.0)
///   - LanceDbAdapter: 벡터 코사인 유사도 (0.0~1.0)
///
/// retrieval_score() 순수 함수가 이 값을 recency, importance와 합산하여
/// 최종 순위를 매긴다.
#[derive(Debug, Clone)]
pub struct ScoredMemory {
    /// 검색된 기억.
    pub entry: MemoryEntry,
    /// 관련성 점수 (0.0~1.0). 검색 방식에 따라 의미가 다름.
    pub relevance_score: f32,
}

impl ScoredMemory {
    pub fn new(entry: MemoryEntry, relevance_score: f32) -> Self {
        Self {
            entry,
            relevance_score,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_observation() -> MemoryEntry {
        MemoryEntry::new(
            MemoryId::new(1),
            CharacterId::new(5),
            "자유도시 시장에서 수상한 사내를 보았다".to_string(),
            7.0,
            MemoryType::Observation,
            GameTime::new(1200, 3, 15),
            vec!["자유도시".to_string(), "수상한 사내".to_string()],
        )
    }

    fn sample_reflection() -> MemoryEntry {
        MemoryEntry::with_sources(
            MemoryId::new(10),
            CharacterId::new(5),
            "나는 사부를 지키지 못했다. 더 강해져야 한다.".to_string(),
            9.0,
            MemoryType::Reflection,
            GameTime::new(1200, 3, 20),
            vec!["사부".to_string(), "강해져야".to_string()],
            vec![MemoryId::new(1), MemoryId::new(2), MemoryId::new(3)],
            Some(3), // Tier 3 전환점 성찰
        )
    }

    // -- MemoryType --

    #[test]
    fn memory_type_equality() {
        assert_eq!(MemoryType::Observation, MemoryType::Observation);
        assert_ne!(MemoryType::Observation, MemoryType::Reflection);
        assert_ne!(MemoryType::Reflection, MemoryType::Plan);
    }

    #[test]
    fn memory_type_display() {
        assert_eq!(MemoryType::Observation.to_string(), "Observation");
        assert_eq!(MemoryType::Reflection.to_string(), "Reflection");
        assert_eq!(MemoryType::Plan.to_string(), "Plan");
    }

    #[test]
    fn memory_type_serialization_roundtrip() {
        let types = vec![
            MemoryType::Observation,
            MemoryType::Reflection,
            MemoryType::Plan,
        ];
        for mt in types {
            let json = serde_json::to_string(&mt).unwrap();
            let restored: MemoryType = serde_json::from_str(&json).unwrap();
            assert_eq!(mt, restored);
        }
    }

    // -- MemoryEntry creation --

    #[test]
    fn create_observation() {
        let entry = sample_observation();
        assert_eq!(entry.id(), MemoryId::new(1));
        assert_eq!(entry.character_id(), CharacterId::new(5));
        assert_eq!(entry.content(), "자유도시 시장에서 수상한 사내를 보았다");
        assert_eq!(entry.importance(), 7.0);
        assert_eq!(entry.memory_type(), MemoryType::Observation);
        assert_eq!(entry.game_time(), GameTime::new(1200, 3, 15));
        assert_eq!(entry.keywords().len(), 2);
        assert!(entry.source_ids().is_empty());
        assert_eq!(entry.reflection_tier(), None);
        assert_eq!(entry.lang(), "KO"); // new()는 기본 "KO"
    }

    #[test]
    fn create_reflection_with_sources() {
        let entry = sample_reflection();
        assert_eq!(entry.memory_type(), MemoryType::Reflection);
        assert_eq!(entry.source_ids().len(), 3);
        assert_eq!(entry.source_ids()[0], MemoryId::new(1));
        assert_eq!(entry.reflection_tier(), Some(3));
        assert_eq!(entry.lang(), "KO"); // with_sources에서 "KO" 지정
    }

    #[test]
    fn create_plan() {
        let entry = MemoryEntry::new(
            MemoryId::new(20),
            CharacterId::new(5),
            "내일부터 새벽마다 검법을 연마하겠다".to_string(),
            6.0,
            MemoryType::Plan,
            GameTime::new(1200, 3, 20),
            vec!["검법".to_string(), "수련".to_string()],
        );
        assert_eq!(entry.memory_type(), MemoryType::Plan);
        assert!(entry.source_ids().is_empty());
    }

    // -- importance clamping --

    #[test]
    fn importance_clamped_to_range() {
        let low = MemoryEntry::new(
            MemoryId::new(1),
            CharacterId::new(1),
            "test".to_string(),
            0.0,
            MemoryType::Observation,
            GameTime::new(1200, 1, 1),
            vec![],
        );
        assert_eq!(low.importance(), 1.0);

        let high = MemoryEntry::new(
            MemoryId::new(2),
            CharacterId::new(1),
            "test".to_string(),
            15.0,
            MemoryType::Observation,
            GameTime::new(1200, 1, 1),
            vec![],
        );
        assert_eq!(high.importance(), 10.0);
    }

    // -- update_importance --

    #[test]
    fn update_importance() {
        let mut entry = sample_observation();
        assert_eq!(entry.importance(), 7.0);

        entry.update_importance(9.5);
        assert_eq!(entry.importance(), 9.5);
    }

    #[test]
    fn update_importance_clamped() {
        let mut entry = sample_observation();

        entry.update_importance(0.0);
        assert_eq!(entry.importance(), 1.0);

        entry.update_importance(99.0);
        assert_eq!(entry.importance(), 10.0);
    }

    // -- ScoredMemory --

    #[test]
    fn scored_memory_creation() {
        let entry = sample_observation();
        let scored = ScoredMemory::new(entry.clone(), 0.85);
        assert_eq!(scored.entry.id(), entry.id());
        assert_eq!(scored.relevance_score, 0.85);
    }

    // -- Clone & Eq --

    #[test]
    fn memory_entry_clone_and_eq() {
        let a = sample_observation();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn different_entries_not_equal() {
        let a = sample_observation();
        let b = sample_reflection();
        assert_ne!(a, b);
    }

    // -- Serialization --

    #[test]
    fn memory_entry_serialization_roundtrip() {
        let entry = sample_observation();
        let json = serde_json::to_string(&entry).unwrap();
        let restored: MemoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, restored);
    }

    #[test]
    fn reflection_with_sources_serialization() {
        let entry = sample_reflection();
        let json = serde_json::to_string(&entry).unwrap();
        let restored: MemoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, restored);
        assert_eq!(restored.source_ids().len(), 3);
        assert_eq!(restored.reflection_tier(), Some(3));
    }

    // -- lang 필드 --

    #[test]
    fn new_defaults_to_ko() {
        let entry = sample_observation();
        assert_eq!(entry.lang(), "KO");
    }

    #[test]
    fn with_sources_defaults_to_ko() {
        let entry = sample_reflection();
        assert_eq!(entry.lang(), "KO");
    }

    #[test]
    fn set_lang_changes_value() {
        let mut entry = sample_observation();
        assert_eq!(entry.lang(), "KO");

        entry.set_lang("EN");
        assert_eq!(entry.lang(), "EN");

        entry.set_lang("ZH");
        assert_eq!(entry.lang(), "ZH");
    }

    #[test]
    fn lang_survives_serialization() {
        let mut entry = sample_observation();
        entry.set_lang("EN");

        let json = serde_json::to_string(&entry).unwrap();
        let restored: MemoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.lang(), "EN");
    }

    #[test]
    fn deserialization_without_lang_defaults_to_ko() {
        // 기존 데이터에 lang 필드가 없는 경우 → "KO"로 역직렬화
        let json = r#"{
            "id": 1,
            "character_id": 5,
            "content": "테스트",
            "importance": 5.0,
            "memory_type": "Observation",
            "game_time": {"year": 1200, "month": 3, "day": 15},
            "keywords": [],
            "source_ids": [],
            "reflection_tier": null
        }"#;

        let entry: MemoryEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.lang(), "KO");
    }
}
