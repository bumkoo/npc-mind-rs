//! 기억(Memory) 도메인 타입 — RAG 인덱싱 및 검색의 핵심 데이터 구조

use serde::{Deserialize, Serialize};

/// 기억 항목 — NPC가 과거에 경험한 사건/대화/관계 변화의 기록
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// 고유 식별자
    pub id: String,
    /// 기억의 주체 NPC ID
    pub npc_id: String,
    /// 기억 내용 (검색 대상 텍스트)
    pub content: String,
    /// 기억 시점의 감정 컨텍스트 (Pleasure, Arousal, Dominance)
    pub emotional_context: Option<(f32, f32, f32)>,
    /// 기억 시점 타임스탬프 (Unix epoch ms)
    pub timestamp_ms: u64,
    /// 이 기억을 생성한 도메인 이벤트 ID
    pub event_id: u64,
    /// 기억 유형
    pub memory_type: MemoryType,
}

/// 기억 유형
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryType {
    /// 대화 턴 (대사)
    Dialogue,
    /// 관계 변화
    Relationship,
    /// Beat 전환
    BeatTransition,
    /// Scene 종료
    SceneEnd,
    /// 외부 게임 이벤트
    GameEvent,
}

impl MemoryType {
    /// 영속화용 문자열 표현. 저장소 스키마의 일부이므로 Rust 식별자 변경과 무관하게 유지된다.
    pub fn as_persisted(&self) -> &'static str {
        match self {
            MemoryType::Dialogue => "Dialogue",
            MemoryType::Relationship => "Relationship",
            MemoryType::BeatTransition => "BeatTransition",
            MemoryType::SceneEnd => "SceneEnd",
            MemoryType::GameEvent => "GameEvent",
        }
    }

    /// 영속화된 문자열 → 변종. 알 수 없는 값은 `None`.
    pub fn from_persisted(s: &str) -> Option<Self> {
        match s {
            "Dialogue" => Some(MemoryType::Dialogue),
            "Relationship" => Some(MemoryType::Relationship),
            "BeatTransition" => Some(MemoryType::BeatTransition),
            "SceneEnd" => Some(MemoryType::SceneEnd),
            "GameEvent" => Some(MemoryType::GameEvent),
            _ => None,
        }
    }
}

/// 기억 검색 결과
#[derive(Debug, Clone)]
pub struct MemoryResult {
    /// 검색된 기억 항목
    pub entry: MemoryEntry,
    /// 관련도 점수 (0.0 ~ 1.0)
    pub relevance_score: f32,
}
