//! 도메인 이벤트 정의 — Event Sourcing의 핵심 타입
//!
//! 모든 상태 변경은 `DomainEvent`로 기록됩니다.
//! `EventPayload`는 각 비즈니스 동작의 결과를 스칼라 요약값으로 담습니다.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// 이벤트 고유 식별자
pub type EventId = u64;

/// 도메인 이벤트 — 모든 상태 변경의 불변 기록
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    /// 전역 고유 ID
    pub id: EventId,
    /// Unix epoch 밀리초
    pub timestamp_ms: u64,
    /// Aggregate root ID (NPC ID)
    pub aggregate_id: String,
    /// 해당 aggregate 내 순번 (1-based)
    pub sequence: u64,
    /// 추적 메타데이터
    pub metadata: EventMetadata,
    /// 이벤트 내용
    pub payload: EventPayload,
}

/// 이벤트 추적 메타데이터
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventMetadata {
    /// 같은 요청에서 파생된 이벤트 묶음 ID
    pub correlation_id: Option<u64>,
}

/// 도메인 이벤트 페이로드
///
/// Phase 1에서는 스칼라 요약값만 포함합니다.
/// Phase 2에서 감정 스냅샷 등 상세 데이터를 추가합니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPayload {
    /// 초기 상황 평가 완료 (appraise)
    EmotionAppraised {
        npc_id: String,
        partner_id: String,
        situation_description: Option<String>,
        /// 지배 감정: (타입명, 강도)
        dominant: Option<(String, f32)>,
        /// 전체 분위기 (-1.0 ~ 1.0)
        mood: f32,
        /// 전체 감정 스냅샷: (감정 타입명, 강도) 목록
        #[serde(default)]
        emotion_snapshot: Vec<(String, f32)>,
    },

    /// PAD 자극 적용 완료 (apply_stimulus)
    StimulusApplied {
        npc_id: String,
        partner_id: String,
        /// 입력 PAD (pleasure, arousal, dominance)
        pad: (f32, f32, f32),
        mood_before: f32,
        mood_after: f32,
        beat_changed: bool,
        /// 자극 후 감정 스냅샷
        #[serde(default)]
        emotion_snapshot: Vec<(String, f32)>,
    },

    /// Beat 전환 발생
    BeatTransitioned {
        npc_id: String,
        from_focus_id: Option<String>,
        to_focus_id: String,
    },

    /// Scene 시작
    SceneStarted {
        npc_id: String,
        partner_id: String,
        focus_count: usize,
        initial_focus_id: Option<String>,
    },

    /// Scene 종료
    SceneEnded {
        npc_id: String,
        partner_id: String,
    },

    /// 관계 갱신
    RelationshipUpdated {
        owner_id: String,
        target_id: String,
        before_closeness: f32,
        before_trust: f32,
        before_power: f32,
        after_closeness: f32,
        after_trust: f32,
        after_power: f32,
    },

    /// 가이드 생성
    GuideGenerated {
        npc_id: String,
        partner_id: String,
    },

    /// 감정 상태 초기화
    EmotionCleared {
        npc_id: String,
    },
}

impl DomainEvent {
    /// 새 이벤트 생성 (현재 시각 자동 설정)
    pub fn new(
        id: EventId,
        aggregate_id: String,
        sequence: u64,
        payload: EventPayload,
    ) -> Self {
        Self {
            id,
            timestamp_ms: now_ms(),
            aggregate_id,
            sequence,
            metadata: EventMetadata::default(),
            payload,
        }
    }

    /// correlation_id 설정
    pub fn with_correlation(mut self, correlation_id: u64) -> Self {
        self.metadata.correlation_id = Some(correlation_id);
        self
    }

    /// 페이로드 타입명 반환 (로깅/필터링용)
    pub fn payload_type(&self) -> &'static str {
        match &self.payload {
            EventPayload::EmotionAppraised { .. } => "EmotionAppraised",
            EventPayload::StimulusApplied { .. } => "StimulusApplied",
            EventPayload::BeatTransitioned { .. } => "BeatTransitioned",
            EventPayload::SceneStarted { .. } => "SceneStarted",
            EventPayload::SceneEnded { .. } => "SceneEnded",
            EventPayload::RelationshipUpdated { .. } => "RelationshipUpdated",
            EventPayload::GuideGenerated { .. } => "GuideGenerated",
            EventPayload::EmotionCleared { .. } => "EmotionCleared",
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
