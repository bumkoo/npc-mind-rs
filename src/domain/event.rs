//! 도메인 이벤트 정의 — Event Sourcing의 핵심 타입
//!
//! 모든 상태 변경은 `DomainEvent`로 기록됩니다.
//! `EventPayload`는 각 비즈니스 동작의 결과를 스칼라 요약값으로 담습니다.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::aggregate::AggregateKey;
use super::emotion::{Scene, Situation};
use super::memory::{MemoryLayer, MemoryScope, MemorySource, MemoryType, Provenance};
use super::rumor::{ReachPolicy, RumorOrigin};
use super::scene_id::SceneId;

/// 이벤트 고유 식별자
pub type EventId = u64;

/// 이벤트 종류 태그 — `EventPayload`의 각 variant와 1:1 대응
///
/// `HandlerInterest::Kinds`에서 타입 안전 필터링용.
/// `payload_type()`은 문자열 기반 로깅 호환을 위해 별도로 유지한다.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum EventKind {
    /// B1: Appraise 커맨드의 초기 이벤트 — EmotionAgent의 `EventHandler` 진입 트리거
    AppraiseRequested,
    EmotionAppraised,
    /// B1: ApplyStimulus 커맨드의 초기 이벤트 — StimulusAgent의 `EventHandler` 진입 트리거
    StimulusApplyRequested,
    StimulusApplied,
    BeatTransitioned,
    /// B4.1: StartScene 커맨드의 초기 이벤트 — SceneAgent 진입 트리거
    SceneStartRequested,
    SceneStarted,
    SceneEnded,
    RelationshipUpdated,
    /// B4.1: UpdateRelationship 커맨드의 초기 이벤트 — RelationshipAgent 진입
    RelationshipUpdateRequested,
    /// B4.1: EndDialogue 커맨드의 초기 이벤트 — RelationshipAgent 진입 (3 follow-ups 발행)
    DialogueEndRequested,
    /// B4.1: GenerateGuide 커맨드의 초기 이벤트 — GuideAgent 진입
    GuideRequested,
    GuideGenerated,
    DialogueTurnCompleted,
    EmotionCleared,

    // ─── Memory 컨텍스트 (Step C1 foundation, §3.1) ─────────────────────────
    /// Inline 핸들러가 새 `MemoryEntry`를 생성했음을 알림
    MemoryEntryCreated,
    /// `mark_superseded` 경로로 엔트리가 대체됨
    MemoryEntrySuperseded,
    /// Scene 요약 Consolidation으로 Layer A→B 흡수됨 (Step D 사용)
    MemoryEntryConsolidated,

    // ─── Rumor 컨텍스트 (Step C1 foundation, §3.1) ──────────────────────────
    /// Memory 컨텍스트 — `Command::SeedRumor`의 초기 이벤트 (Step C2/C3 사용)
    SeedRumorRequested,
    /// Memory 컨텍스트 — `Command::SpreadRumor`의 초기 이벤트 (Step C2/C3 사용)
    SpreadRumorRequested,
    RumorSeeded,
    RumorSpread,
    RumorDistorted,
    RumorFaded,

    // ─── Mind 컨텍스트 — Memory가 구독 (Step C1 foundation, §3.1) ───────────
    /// Mind 컨텍스트 — `Command::TellInformation`의 초기 이벤트 (Step C2 사용)
    TellInformationRequested,
    /// 청자당 1 이벤트 (B5)
    InformationTold,
}

/// 청자 역할 — `InformationTold` 이벤트에 실림 (B5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListenerRole {
    /// 화자가 직접 말을 건 대상
    Direct,
    /// 같은 공간에서 엿들은 자
    Overhearer,
}

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

/// `RelationshipUpdated` 이벤트에 실리는 **귀속 원인** (Memory 시스템 §8.3 hook, A8).
///
/// Step A에서는 모든 발행 지점이 `Unspecified`로 고정된다. Step C/D에서 InformationAgent/
/// WorldOverlayAgent 추가 시 정식 variant로 채워진다. Memory 컨텍스트의
/// `RelationshipMemoryPolicy`가 이 값으로 content·source·topic을 분기한다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RelationshipChangeCause {
    /// 장면 내 대사/행동 (Step A default가 이쪽으로 이행하려면 scene_id가 필요하므로
    /// Step A는 Unspecified 유지)
    SceneInteraction { scene_id: SceneId },
    /// 정보 전달 (Step C `InformationTold`에서 설정)
    InformationTold { origin_chain: Vec<String> },
    /// 세계 사건 오버레이 (Step D `WorldEventOccurred`에서 설정)
    WorldEventOverlay { topic: Option<String> },
    /// 소문 확산 (Step C `RumorSpread`에서 설정)
    Rumor { rumor_id: String },
    /// Step A 기본값 — 마이그레이션·레거시 호환. Memory 정책은 이 값을 일반 분기로 처리한다.
    Unspecified,
}

impl Default for RelationshipChangeCause {
    fn default() -> Self {
        Self::Unspecified
    }
}

/// 도메인 이벤트 페이로드
///
/// Phase 1에서는 스칼라 요약값만 포함합니다.
/// Phase 2에서 감정 스냅샷 등 상세 데이터를 추가합니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPayload {
    /// B1: Appraise 요청 (Command → 초기 이벤트)
    ///
    /// B3 `dispatch_v2()`에서 `Command::Appraise`가 이 이벤트로 변환되어
    /// EmotionAgent의 `EventHandler::handle()` 진입점 역할을 한다.
    /// B1 단계에서는 변환 경로가 없으므로 L1 단위 테스트에서만 수동 생성.
    AppraiseRequested {
        npc_id: String,
        partner_id: String,
        /// 이미 해석된 도메인 상황 (SituationService 등이 SituationInput → Situation 변환 후 주입)
        situation: Situation,
    },

    /// B1: ApplyStimulus 요청 (Command → 초기 이벤트)
    ///
    /// B3 `dispatch_v2()`에서 `Command::ApplyStimulus`가 이 이벤트로 변환되어
    /// StimulusAgent의 `EventHandler::handle()` 진입점 역할을 한다.
    StimulusApplyRequested {
        npc_id: String,
        partner_id: String,
        pad: (f32, f32, f32),
        situation_description: Option<String>,
    },

    /// B4.1: GenerateGuide 커맨드 → GuideAgent 진입
    GuideRequested {
        npc_id: String,
        partner_id: String,
        situation_description: Option<String>,
    },

    /// B4.1: UpdateRelationship 커맨드 → RelationshipAgent 진입
    RelationshipUpdateRequested {
        npc_id: String,
        partner_id: String,
        significance: Option<f32>,
    },

    /// B4.1: EndDialogue 커맨드 → RelationshipAgent 진입 (3 follow-ups: RelationshipUpdated
    /// + EmotionCleared + SceneEnded). v1 `RelationshipAgent::handle_end_dialogue` 등가.
    DialogueEndRequested {
        npc_id: String,
        partner_id: String,
        significance: Option<f32>,
    },

    /// B4.1: StartScene 커맨드 → SceneAgent 진입
    ///
    /// Dispatcher가 `SituationService`로 `SceneFocusInput` DTO를 resolved `SceneFocus` 도메인
    /// 객체로 변환한 뒤 `Scene::with_significance`로 빌드된 Scene을 `prebuilt_scene`에 담아
    /// 전달. SceneAgent가 Scene 등록 + 초기 Focus appraise를 수행하고 `SceneStarted` +
    /// (옵션) `EmotionAppraised`를 follow-up으로 발행. focuses는 `prebuilt_scene.focuses()`
    /// 로 접근하며 별도 필드 중복 제거(B4.1 리뷰 M5).
    SceneStartRequested {
        npc_id: String,
        partner_id: String,
        significance: Option<f32>,
        /// 이미 확정된 초기 focus (내부적으로 `scene.initial_focus()`와 동일 — 탐색 단축용)
        initial_focus_id: Option<String>,
        /// 사전 구성된 Scene. focuses는 여기의 `.focuses()`로 접근.
        prebuilt_scene: Scene,
    },

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
    ///
    /// B4 Session 3 (Option A): `partner_id` 필드 추가. 다중 Scene 환경에서
    /// `RelationshipAgent`가 이 이벤트에 반응할 때 올바른 Scene의 관계를 갱신할 수 있도록
    /// payload에서 직접 읽는다. 기존에는 `ctx.repo.get_scene()` fallback으로 추론했는데,
    /// `InMemoryRepository.last_scene_id`가 다른 Scene을 가리킬 때 **잘못된 관계를 갱신**
    /// 하는 multi-scene 오동작이 있었음.
    BeatTransitioned {
        npc_id: String,
        partner_id: String,
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
        /// 갱신 원인 (Memory 시스템 §8.3 policy branching용, A8 hook).
        /// Step A에서는 모든 발행 지점이 `Unspecified`로 고정. Step C/D에서 InformationAgent/
        /// WorldOverlayAgent 등이 정식 variant를 채운다. serde default로 구 JSON 역호환.
        #[serde(default)]
        cause: RelationshipChangeCause,
    },

    /// 가이드 생성
    GuideGenerated {
        npc_id: String,
        partner_id: String,
    },

    /// 대화 턴 완료
    DialogueTurnCompleted {
        npc_id: String,
        partner_id: String,
        /// 화자: "user" 또는 "assistant"
        speaker: String,
        /// 대사 내용
        utterance: String,
        /// 대사 시점 감정 스냅샷
        #[serde(default)]
        emotion_snapshot: Vec<(String, f32)>,
    },

    /// 감정 상태 초기화
    EmotionCleared {
        npc_id: String,
    },

    // ─────────────────────────────────────────────────────────────────────
    // Memory 컨텍스트 — Step C1 foundation (§3.1)
    //
    // Step C1에서는 variant 타입만 추가되며, 발행 handler는 Step C2/C3/D에서 연결된다.
    // `MemoryAgent`·`TurnMemoryEvaluationHandler` 등이 Step D까지 완료되면 실제 흐름에 편입.
    // ─────────────────────────────────────────────────────────────────────

    /// 새 `MemoryEntry` 생성됨 (Inline 핸들러가 발행).
    MemoryEntryCreated {
        entry_id: String,
        scope: MemoryScope,
        source: MemorySource,
        provenance: Provenance,
        /// 엔트리 종류 (DialogueTurn/BeatTransition/SceneSummary 등 — 설계 §3.1 요구)
        memory_type: MemoryType,
        layer: MemoryLayer,
        topic: Option<String>,
        confidence: f32,
        acquired_by: Option<String>,
        /// EventStore append sequence — entry의 `created_seq`와 일치 (A7, I-ME-10)
        created_seq: u64,
        /// 이 엔트리를 파생시킨 원본 트리거 이벤트 id
        source_event_id: u64,
    },

    /// 기존 엔트리가 새 엔트리로 대체됨 (supersede).
    MemoryEntrySuperseded {
        old_entry_id: String,
        new_entry_id: String,
        topic: Option<String>,
    },

    /// Scene 요약 Consolidation — Layer A 엔트리들이 하나의 Layer B로 흡수됨.
    /// Step D `SceneConsolidationHandler`가 발행.
    MemoryEntryConsolidated {
        a_entry_ids: Vec<String>,
        b_entry_id: String,
        scene_id: Option<SceneId>,
    },

    // ─────────────────────────────────────────────────────────────────────
    // Rumor 컨텍스트 — Step C1 foundation (§3.1/§3.2)
    // ─────────────────────────────────────────────────────────────────────

    /// Memory 컨텍스트 — `Command::SeedRumor`의 초기 이벤트.
    /// `RumorAgent`가 소비해 `RumorSeeded` follow-up을 발행한다 (Step C3).
    SeedRumorRequested {
        topic: Option<String>,
        seed_content: Option<String>,
        reach: ReachPolicy,
        origin: RumorOrigin,
    },

    /// Memory 컨텍스트 — `Command::SpreadRumor`의 초기 이벤트.
    SpreadRumorRequested {
        rumor_id: String,
        extra_recipients: Vec<String>,
    },

    /// 새 소문 시딩됨.
    RumorSeeded {
        rumor_id: String,
        topic: Option<String>,
        origin: RumorOrigin,
        /// 고아 Rumor 또는 예보된 사실일 때만 Some (A2).
        seed_content: Option<String>,
        reach_policy: ReachPolicy,
    },

    /// 소문 한 홉 확산 (I-RU-1 단조 hop_index 강제).
    RumorSpread {
        rumor_id: String,
        hop_index: u32,
        recipients: Vec<String>,
        /// DistortionId. None이면 원본 그대로.
        content_version: Option<String>,
    },

    /// 소문 콘텐츠가 새 변형으로 파생됨 (DAG 노드 추가).
    RumorDistorted {
        rumor_id: String,
        distortion_id: String,
        parent: Option<String>,
    },

    /// 소문이 `Faded` 상태로 종결됨.
    RumorFaded {
        rumor_id: String,
    },

    // ─────────────────────────────────────────────────────────────────────
    // Mind 컨텍스트 (Memory가 구독) — Step C1 foundation (§3.1/§3.2)
    // ─────────────────────────────────────────────────────────────────────

    /// Mind 컨텍스트 — `Command::TellInformation`의 초기 이벤트.
    /// `InformationAgent`가 청자별로 `InformationTold` follow-up을 발행 (Step C2).
    TellInformationRequested {
        speaker: String,
        listeners: Vec<String>,
        overhearers: Vec<String>,
        claim: String,
        stated_confidence: f32,
        origin_chain_in: Vec<String>,
    },

    /// 청자 1명당 1 이벤트 (B5). N명 청자 → N개 follow-up.
    /// `AggregateKey::Npc(listener)`로 라우팅된다.
    InformationTold {
        speaker: String,
        listener: String,
        listener_role: ListenerRole,
        claim: String,
        stated_confidence: f32,
        origin_chain_in: Vec<String>,
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
            EventPayload::AppraiseRequested { .. } => "AppraiseRequested",
            EventPayload::EmotionAppraised { .. } => "EmotionAppraised",
            EventPayload::StimulusApplyRequested { .. } => "StimulusApplyRequested",
            EventPayload::StimulusApplied { .. } => "StimulusApplied",
            EventPayload::BeatTransitioned { .. } => "BeatTransitioned",
            EventPayload::SceneStartRequested { .. } => "SceneStartRequested",
            EventPayload::SceneStarted { .. } => "SceneStarted",
            EventPayload::SceneEnded { .. } => "SceneEnded",
            EventPayload::RelationshipUpdated { .. } => "RelationshipUpdated",
            EventPayload::RelationshipUpdateRequested { .. } => "RelationshipUpdateRequested",
            EventPayload::DialogueEndRequested { .. } => "DialogueEndRequested",
            EventPayload::GuideRequested { .. } => "GuideRequested",
            EventPayload::GuideGenerated { .. } => "GuideGenerated",
            EventPayload::DialogueTurnCompleted { .. } => "DialogueTurnCompleted",
            EventPayload::EmotionCleared { .. } => "EmotionCleared",
            EventPayload::MemoryEntryCreated { .. } => "MemoryEntryCreated",
            EventPayload::MemoryEntrySuperseded { .. } => "MemoryEntrySuperseded",
            EventPayload::MemoryEntryConsolidated { .. } => "MemoryEntryConsolidated",
            EventPayload::SeedRumorRequested { .. } => "SeedRumorRequested",
            EventPayload::SpreadRumorRequested { .. } => "SpreadRumorRequested",
            EventPayload::RumorSeeded { .. } => "RumorSeeded",
            EventPayload::RumorSpread { .. } => "RumorSpread",
            EventPayload::RumorDistorted { .. } => "RumorDistorted",
            EventPayload::RumorFaded { .. } => "RumorFaded",
            EventPayload::TellInformationRequested { .. } => "TellInformationRequested",
            EventPayload::InformationTold { .. } => "InformationTold",
        }
    }

    /// 페이로드 종류 태그 반환 (타입 안전 필터링용)
    pub fn kind(&self) -> EventKind {
        match &self.payload {
            EventPayload::AppraiseRequested { .. } => EventKind::AppraiseRequested,
            EventPayload::EmotionAppraised { .. } => EventKind::EmotionAppraised,
            EventPayload::StimulusApplyRequested { .. } => EventKind::StimulusApplyRequested,
            EventPayload::StimulusApplied { .. } => EventKind::StimulusApplied,
            EventPayload::BeatTransitioned { .. } => EventKind::BeatTransitioned,
            EventPayload::SceneStartRequested { .. } => EventKind::SceneStartRequested,
            EventPayload::SceneStarted { .. } => EventKind::SceneStarted,
            EventPayload::SceneEnded { .. } => EventKind::SceneEnded,
            EventPayload::RelationshipUpdated { .. } => EventKind::RelationshipUpdated,
            EventPayload::RelationshipUpdateRequested { .. } => {
                EventKind::RelationshipUpdateRequested
            }
            EventPayload::DialogueEndRequested { .. } => EventKind::DialogueEndRequested,
            EventPayload::GuideRequested { .. } => EventKind::GuideRequested,
            EventPayload::GuideGenerated { .. } => EventKind::GuideGenerated,
            EventPayload::DialogueTurnCompleted { .. } => EventKind::DialogueTurnCompleted,
            EventPayload::EmotionCleared { .. } => EventKind::EmotionCleared,
            EventPayload::MemoryEntryCreated { .. } => EventKind::MemoryEntryCreated,
            EventPayload::MemoryEntrySuperseded { .. } => EventKind::MemoryEntrySuperseded,
            EventPayload::MemoryEntryConsolidated { .. } => EventKind::MemoryEntryConsolidated,
            EventPayload::SeedRumorRequested { .. } => EventKind::SeedRumorRequested,
            EventPayload::SpreadRumorRequested { .. } => EventKind::SpreadRumorRequested,
            EventPayload::RumorSeeded { .. } => EventKind::RumorSeeded,
            EventPayload::RumorSpread { .. } => EventKind::RumorSpread,
            EventPayload::RumorDistorted { .. } => EventKind::RumorDistorted,
            EventPayload::RumorFaded { .. } => EventKind::RumorFaded,
            EventPayload::TellInformationRequested { .. } => EventKind::TellInformationRequested,
            EventPayload::InformationTold { .. } => EventKind::InformationTold,
        }
    }

    /// 이벤트가 속한 aggregate 식별자 반환
    ///
    /// B안(다중 Scene) 이행 후 SceneTask가 자기 aggregate의 이벤트만 순차 처리할 때 사용.
    ///
    /// **B4 Migration Note (plan §9.1):** `EventPayload`에 `scene_id` 필드가 추가되면
    /// `EmotionAppraised` · `StimulusApplied` · `BeatTransitioned` · `GuideGenerated` ·
    /// `DialogueTurnCompleted` 계열을 `AggregateKey::Npc` → `AggregateKey::Scene`로
    /// 승격해야 한다. 현재는 `(npc_id, partner_id)`로 Scene을 식별할 수 없어
    /// `SceneStarted` / `SceneEnded`만 `Scene` 키를 반환한다.
    pub fn aggregate_key(&self) -> AggregateKey {
        match &self.payload {
            EventPayload::SceneStartRequested {
                npc_id, partner_id, ..
            }
            | EventPayload::SceneStarted {
                npc_id, partner_id, ..
            }
            | EventPayload::SceneEnded { npc_id, partner_id }
            | EventPayload::DialogueEndRequested {
                npc_id, partner_id, ..
            }
            | EventPayload::BeatTransitioned {
                npc_id, partner_id, ..
            } => AggregateKey::Scene {
                npc_id: npc_id.clone(),
                partner_id: partner_id.clone(),
            },
            EventPayload::RelationshipUpdated {
                owner_id,
                target_id,
                ..
            } => AggregateKey::Relationship {
                owner_id: owner_id.clone(),
                target_id: target_id.clone(),
            },
            EventPayload::RelationshipUpdateRequested {
                npc_id, partner_id, ..
            } => AggregateKey::Relationship {
                owner_id: npc_id.clone(),
                target_id: partner_id.clone(),
            },
            EventPayload::AppraiseRequested { npc_id, .. }
            | EventPayload::EmotionAppraised { npc_id, .. }
            | EventPayload::StimulusApplyRequested { npc_id, .. }
            | EventPayload::StimulusApplied { npc_id, .. }
            | EventPayload::GuideRequested { npc_id, .. }
            | EventPayload::GuideGenerated { npc_id, .. }
            | EventPayload::DialogueTurnCompleted { npc_id, .. }
            | EventPayload::EmotionCleared { npc_id } => AggregateKey::Npc(npc_id.clone()),

            // Memory 컨텍스트 — `Memory(entry_id)`.
            EventPayload::MemoryEntryCreated { entry_id, .. }
            | EventPayload::MemoryEntrySuperseded {
                old_entry_id: entry_id,
                ..
            } => AggregateKey::Memory(entry_id.clone()),
            // Consolidation은 결과(b) 쪽을 주체로 라우팅.
            EventPayload::MemoryEntryConsolidated { b_entry_id, .. } => {
                AggregateKey::Memory(b_entry_id.clone())
            }

            // Rumor 컨텍스트 — `Rumor(rumor_id)`.
            EventPayload::SeedRumorRequested { topic, .. } => {
                // 시딩 요청 시에는 아직 rumor_id가 없으므로 topic 기반 임시 키.
                // 고아 Rumor(topic=None)면 "orphan"을 사용.
                AggregateKey::Rumor(topic.clone().unwrap_or_else(|| "orphan".into()))
            }
            EventPayload::SpreadRumorRequested { rumor_id, .. }
            | EventPayload::RumorSeeded { rumor_id, .. }
            | EventPayload::RumorSpread { rumor_id, .. }
            | EventPayload::RumorDistorted { rumor_id, .. }
            | EventPayload::RumorFaded { rumor_id } => AggregateKey::Rumor(rumor_id.clone()),

            // Mind 컨텍스트 — TellInformationRequested는 화자(Npc), InformationTold는 청자(Npc).
            EventPayload::TellInformationRequested { speaker, .. } => {
                AggregateKey::Npc(speaker.clone())
            }
            EventPayload::InformationTold { listener, .. } => {
                AggregateKey::Npc(listener.clone())
            }
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::emotion::{EventFocus, Situation};

    fn make_event(payload: EventPayload) -> DomainEvent {
        DomainEvent::new(1, "test".into(), 1, payload)
    }

    fn trivial_situation() -> Situation {
        Situation::new(
            "test situation",
            Some(EventFocus {
                description: "test event".into(),
                desirability_for_self: 0.0,
                desirability_for_other: None,
                prospect: None,
            }),
            None,
            None,
        )
        .expect("Situation with event focus must be valid")
    }

    #[test]
    fn kind_matches_payload_type_for_all_variants() {
        let cases: Vec<(EventPayload, EventKind, &'static str)> = vec![
            (
                EventPayload::AppraiseRequested {
                    npc_id: "a".into(),
                    partner_id: "b".into(),
                    situation: trivial_situation(),
                },
                EventKind::AppraiseRequested,
                "AppraiseRequested",
            ),
            (
                EventPayload::StimulusApplyRequested {
                    npc_id: "a".into(),
                    partner_id: "b".into(),
                    pad: (0.0, 0.0, 0.0),
                    situation_description: None,
                },
                EventKind::StimulusApplyRequested,
                "StimulusApplyRequested",
            ),
            (
                EventPayload::EmotionAppraised {
                    npc_id: "a".into(),
                    partner_id: "b".into(),
                    situation_description: None,
                    dominant: None,
                    mood: 0.0,
                    emotion_snapshot: vec![],
                },
                EventKind::EmotionAppraised,
                "EmotionAppraised",
            ),
            (
                EventPayload::StimulusApplied {
                    npc_id: "a".into(),
                    partner_id: "b".into(),
                    pad: (0.0, 0.0, 0.0),
                    mood_before: 0.0,
                    mood_after: 0.0,
                    beat_changed: false,
                    emotion_snapshot: vec![],
                },
                EventKind::StimulusApplied,
                "StimulusApplied",
            ),
            (
                EventPayload::BeatTransitioned {
                    npc_id: "a".into(),
                    partner_id: "b".into(),
                    from_focus_id: None,
                    to_focus_id: "f".into(),
                },
                EventKind::BeatTransitioned,
                "BeatTransitioned",
            ),
            (
                EventPayload::SceneStarted {
                    npc_id: "a".into(),
                    partner_id: "b".into(),
                    focus_count: 0,
                    initial_focus_id: None,
                },
                EventKind::SceneStarted,
                "SceneStarted",
            ),
            (
                EventPayload::SceneEnded {
                    npc_id: "a".into(),
                    partner_id: "b".into(),
                },
                EventKind::SceneEnded,
                "SceneEnded",
            ),
            (
                EventPayload::RelationshipUpdated {
                    owner_id: "a".into(),
                    target_id: "b".into(),
                    before_closeness: 0.0,
                    before_trust: 0.0,
                    before_power: 0.0,
                    after_closeness: 0.0,
                    after_trust: 0.0,
                    after_power: 0.0,
                    cause: RelationshipChangeCause::Unspecified,
                },
                EventKind::RelationshipUpdated,
                "RelationshipUpdated",
            ),
            (
                EventPayload::GuideGenerated {
                    npc_id: "a".into(),
                    partner_id: "b".into(),
                },
                EventKind::GuideGenerated,
                "GuideGenerated",
            ),
            (
                EventPayload::DialogueTurnCompleted {
                    npc_id: "a".into(),
                    partner_id: "b".into(),
                    speaker: "user".into(),
                    utterance: "hi".into(),
                    emotion_snapshot: vec![],
                },
                EventKind::DialogueTurnCompleted,
                "DialogueTurnCompleted",
            ),
            (
                EventPayload::EmotionCleared { npc_id: "a".into() },
                EventKind::EmotionCleared,
                "EmotionCleared",
            ),
            (
                EventPayload::MemoryEntryCreated {
                    entry_id: "mem-1".into(),
                    scope: MemoryScope::Personal {
                        npc_id: "a".into(),
                    },
                    source: MemorySource::Experienced,
                    provenance: Provenance::Runtime,
                    memory_type: MemoryType::DialogueTurn,
                    layer: MemoryLayer::A,
                    topic: None,
                    confidence: 1.0,
                    acquired_by: None,
                    created_seq: 0,
                    source_event_id: 1,
                },
                EventKind::MemoryEntryCreated,
                "MemoryEntryCreated",
            ),
            (
                EventPayload::MemoryEntrySuperseded {
                    old_entry_id: "mem-1".into(),
                    new_entry_id: "mem-2".into(),
                    topic: Some("t".into()),
                },
                EventKind::MemoryEntrySuperseded,
                "MemoryEntrySuperseded",
            ),
            (
                EventPayload::MemoryEntryConsolidated {
                    a_entry_ids: vec!["a1".into(), "a2".into()],
                    b_entry_id: "b1".into(),
                    scene_id: None,
                },
                EventKind::MemoryEntryConsolidated,
                "MemoryEntryConsolidated",
            ),
            (
                EventPayload::SeedRumorRequested {
                    topic: Some("t".into()),
                    seed_content: None,
                    reach: ReachPolicy::default(),
                    origin: RumorOrigin::Seeded,
                },
                EventKind::SeedRumorRequested,
                "SeedRumorRequested",
            ),
            (
                EventPayload::SpreadRumorRequested {
                    rumor_id: "r1".into(),
                    extra_recipients: vec![],
                },
                EventKind::SpreadRumorRequested,
                "SpreadRumorRequested",
            ),
            (
                EventPayload::RumorSeeded {
                    rumor_id: "r1".into(),
                    topic: Some("t".into()),
                    origin: RumorOrigin::Seeded,
                    seed_content: None,
                    reach_policy: ReachPolicy::default(),
                },
                EventKind::RumorSeeded,
                "RumorSeeded",
            ),
            (
                EventPayload::RumorSpread {
                    rumor_id: "r1".into(),
                    hop_index: 0,
                    recipients: vec!["a".into()],
                    content_version: None,
                },
                EventKind::RumorSpread,
                "RumorSpread",
            ),
            (
                EventPayload::RumorDistorted {
                    rumor_id: "r1".into(),
                    distortion_id: "d1".into(),
                    parent: None,
                },
                EventKind::RumorDistorted,
                "RumorDistorted",
            ),
            (
                EventPayload::RumorFaded {
                    rumor_id: "r1".into(),
                },
                EventKind::RumorFaded,
                "RumorFaded",
            ),
            (
                EventPayload::TellInformationRequested {
                    speaker: "a".into(),
                    listeners: vec!["b".into()],
                    overhearers: vec![],
                    claim: "truth".into(),
                    stated_confidence: 1.0,
                    origin_chain_in: vec![],
                },
                EventKind::TellInformationRequested,
                "TellInformationRequested",
            ),
            (
                EventPayload::InformationTold {
                    speaker: "a".into(),
                    listener: "b".into(),
                    listener_role: ListenerRole::Direct,
                    claim: "truth".into(),
                    stated_confidence: 1.0,
                    origin_chain_in: vec![],
                },
                EventKind::InformationTold,
                "InformationTold",
            ),
        ];

        for (payload, expected_kind, expected_name) in cases {
            let ev = make_event(payload);
            assert_eq!(ev.kind(), expected_kind, "kind mismatch for {expected_name}");
            assert_eq!(
                ev.payload_type(),
                expected_name,
                "payload_type mismatch for {expected_name}"
            );
        }
    }

    #[test]
    fn aggregate_key_routes_scene_lifecycle_to_scene() {
        let started = make_event(EventPayload::SceneStarted {
            npc_id: "muback".into(),
            partner_id: "gyoryong".into(),
            focus_count: 0,
            initial_focus_id: None,
        });
        let ended = make_event(EventPayload::SceneEnded {
            npc_id: "muback".into(),
            partner_id: "gyoryong".into(),
        });
        let expected = AggregateKey::Scene {
            npc_id: "muback".into(),
            partner_id: "gyoryong".into(),
        };
        assert_eq!(started.aggregate_key(), expected);
        assert_eq!(ended.aggregate_key(), expected);
    }

    #[test]
    fn aggregate_key_routes_relationship_to_relationship() {
        let ev = make_event(EventPayload::RelationshipUpdated {
            owner_id: "a".into(),
            target_id: "b".into(),
            before_closeness: 0.0,
            before_trust: 0.0,
            before_power: 0.0,
            after_closeness: 0.0,
            after_trust: 0.0,
            after_power: 0.0,
            cause: RelationshipChangeCause::Unspecified,
        });
        assert_eq!(
            ev.aggregate_key(),
            AggregateKey::Relationship {
                owner_id: "a".into(),
                target_id: "b".into(),
            }
        );
    }

    #[test]
    fn aggregate_key_routes_emotion_like_to_npc() {
        let ev = make_event(EventPayload::EmotionAppraised {
            npc_id: "muback".into(),
            partner_id: "gyoryong".into(),
            situation_description: None,
            dominant: None,
            mood: 0.0,
            emotion_snapshot: vec![],
        });
        assert_eq!(ev.aggregate_key(), AggregateKey::Npc("muback".into()));
    }

    #[test]
    fn aggregate_key_routes_memory_events_to_memory() {
        let created = make_event(EventPayload::MemoryEntryCreated {
            entry_id: "mem-7".into(),
            scope: MemoryScope::Personal {
                npc_id: "a".into(),
            },
            source: MemorySource::Experienced,
            provenance: Provenance::Runtime,
            memory_type: MemoryType::DialogueTurn,
            layer: MemoryLayer::A,
            topic: None,
            confidence: 1.0,
            acquired_by: None,
            created_seq: 0,
            source_event_id: 1,
        });
        assert_eq!(created.aggregate_key(), AggregateKey::Memory("mem-7".into()));

        let superseded = make_event(EventPayload::MemoryEntrySuperseded {
            old_entry_id: "mem-5".into(),
            new_entry_id: "mem-6".into(),
            topic: None,
        });
        assert_eq!(
            superseded.aggregate_key(),
            AggregateKey::Memory("mem-5".into()),
            "supersede는 old_entry_id 기준"
        );

        let consolidated = make_event(EventPayload::MemoryEntryConsolidated {
            a_entry_ids: vec!["a1".into()],
            b_entry_id: "b1".into(),
            scene_id: None,
        });
        assert_eq!(
            consolidated.aggregate_key(),
            AggregateKey::Memory("b1".into()),
            "consolidation은 b_entry_id 기준"
        );
    }

    #[test]
    fn aggregate_key_routes_rumor_events_to_rumor() {
        let seeded = make_event(EventPayload::RumorSeeded {
            rumor_id: "r1".into(),
            topic: Some("t".into()),
            origin: RumorOrigin::Seeded,
            seed_content: None,
            reach_policy: ReachPolicy::default(),
        });
        assert_eq!(seeded.aggregate_key(), AggregateKey::Rumor("r1".into()));

        let spread = make_event(EventPayload::RumorSpread {
            rumor_id: "r1".into(),
            hop_index: 0,
            recipients: vec![],
            content_version: None,
        });
        assert_eq!(spread.aggregate_key(), AggregateKey::Rumor("r1".into()));
    }

    #[test]
    fn aggregate_key_routes_seed_request_by_topic_with_orphan_fallback() {
        let with_topic = make_event(EventPayload::SeedRumorRequested {
            topic: Some("moorim-leader-change".into()),
            seed_content: None,
            reach: ReachPolicy::default(),
            origin: RumorOrigin::Seeded,
        });
        assert_eq!(
            with_topic.aggregate_key(),
            AggregateKey::Rumor("moorim-leader-change".into())
        );

        let orphan = make_event(EventPayload::SeedRumorRequested {
            topic: None,
            seed_content: Some("떠도는 흉흉한 얘기".into()),
            reach: ReachPolicy::default(),
            origin: RumorOrigin::Authored { by: None },
        });
        assert_eq!(orphan.aggregate_key(), AggregateKey::Rumor("orphan".into()));
    }

    #[test]
    fn aggregate_key_routes_information_events_correctly() {
        // TellInformationRequested → 화자
        let req = make_event(EventPayload::TellInformationRequested {
            speaker: "sage".into(),
            listeners: vec!["pupil".into(), "wanderer".into()],
            overhearers: vec![],
            claim: "맹주가 바뀐다".into(),
            stated_confidence: 0.8,
            origin_chain_in: vec!["sage".into()],
        });
        assert_eq!(req.aggregate_key(), AggregateKey::Npc("sage".into()));

        // InformationTold → 청자 (B5 라우팅 기준)
        let told = make_event(EventPayload::InformationTold {
            speaker: "sage".into(),
            listener: "pupil".into(),
            listener_role: ListenerRole::Direct,
            claim: "맹주가 바뀐다".into(),
            stated_confidence: 0.8,
            origin_chain_in: vec!["sage".into()],
        });
        assert_eq!(told.aggregate_key(), AggregateKey::Npc("pupil".into()));
    }
}
