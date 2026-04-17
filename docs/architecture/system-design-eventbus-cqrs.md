# NPC Mind Engine v3 — EventBus · CQRS · Event Sourcing · Multi-Agent 시스템 디자인

> **Status**: In Progress (Phase 1-3 + Pipeline + EventBus v2 완료)  
> **Date**: 2026-04-16 (최종 업데이트: 2026-04-17)  
> **Scope**: 엔진 전체 리팩토링 — 현재 헥사고날 아키텍처를 이벤트 기반으로 전환  
> **Key Decisions**: EventBus 중심 통신, CQRS 분리, Event Sourcing 도입, 기능별 에이전트, 게임 히스토리 RAG
>
> ### 구현 현황
>
> | 단계 | 상태 | 구현 내용 |
> |------|------|----------|
> | **Phase 1** | ✅ 완료 | `DomainEvent`(9 variants, emotion_snapshot 포함), `InMemoryEventStore`, `EventBus`, `EventAwareMindService`(Strangler Fig), `Projection` 3종 |
> | **Phase 2** | ✅ 완료 | `Command`(6 variants), `CommandDispatcher`(Orchestrator), `EmotionAgent`, `GuideAgent`, `RelationshipAgent`, `HandlerContext`/`HandlerOutput` |
> | **Phase 3** | ✅ 완료 | `MemoryStore` 포트, `InMemoryMemoryStore`, `SqliteMemoryStore`(FTS5+벡터BLOB) [embed], `MemoryAgent`(broadcast subscriber) [embed], `DialogueTurnCompleted` 이벤트 |
> | **Pipeline** | ✅ 완료 | `Pipeline`(순차 에이전트 체인, `PipelineState` 컨텍스트 전파) |
> | **EventBus v2** | ✅ 완료 | `tokio::sync::broadcast` 기반 단일화. `subscribe()` → `impl Stream<Arc<DomainEvent>>` (runtime-agnostic). L1 `ProjectionRegistry`(Dispatcher 내부, 동기 쓰기 경로)로 쿼리 일관성 보장. `TieredEventBus`/`StdThreadSink`/`TokioSink` 삭제. |
> | **Phase 4+** | 미구현 | DialogueAgent, StoryAgent, SummaryAgent, Tool 시스템, WorldKnowledgeStore |
>
> ### 설계 문서와 구현의 차이
>
> - **EventBus**: 문서 원안(`tokio::broadcast`) 채택. `EventBus v2`에서 `tokio::sync::broadcast::Sender` 내부 구현 + `tokio_stream::wrappers::BroadcastStream`으로 공개 API를 `futures::Stream`으로 감쌈. 호출자는 tokio deps 불요.
> - **Projection 위치**: 문서는 "모든 소비자가 EventBus 구독" 전제 → 구현은 **B-lite**: Projection은 bus 밖 L1, Agent/SSE는 broadcast 구독 L2. Projection을 broadcast 구독자로 두면 race·lag 시 상태 손상 위험이 있어 쿼리 일관성을 위해 분리.
> - **Agent 통신**: Command 처리는 Orchestrator 패턴(`CommandDispatcher`가 직접 호출, 순서 보장), 이벤트 후속 소비는 broadcast Stream 기반.
> - **RAG 저장소**: 문서는 SQLite + LanceDB 하이브리드 → 구현은 SQLite-Primary (LanceDB async-only 제약). 벡터는 BLOB으로 저장.
> - **Pipeline**: 문서에 없던 개념. Tier 1(커맨드 내부 순차 동기) 역할 담당. EventBus v2 이후 "Tier"라는 분류 체계는 Pipeline(동기) vs EventBus(비동기)로 자연 정렬됨.
> - **Lag 복구**: broadcast capacity 초과 시 `subscribe_with_lag()`의 `Lagged(n)` 통지를 받고 `EventStore::get_events_after_id()`로 replay하여 at-least-once 유지.

---

## 1. 현재 아키텍처 진단

### 1.1 현재 구조 (v2)

```
MindService<R, A, S>  ← God Object (appraise + stimulus + scene + guide + relationship 모두 담당)
     │
     ├── AppraisalEngine / StimulusEngine  (도메인 서비스, 순수 함수)
     ├── MindRepository (NpcWorld + EmotionStore + SceneStore)
     ├── GuideFormatter
     └── FormattedMindService (포맷팅 래퍼)
```

### 1.2 핵심 문제

| 문제 | 영향 | 심각도 |
|------|------|--------|
| **MindService God Object** | appraise → stimulus → beat transition → guide → relationship 전부 한 곳에서 처리. 570+ 라인 | 높음 |
| **동기적 직렬 처리** | 모든 호출이 `&mut self`로 순차 실행. 병렬 불가 | 높음 |
| **상태 변경 추적 불가** | save_emotion_state()는 최종 상태만 덮어씀. 변화 과정 유실 | 높음 |
| **히스토리 부재** | 과거 대화, 감정 변화, 관계 변동 이력 없음 | 높음 |
| **단일 NPC 관점** | A가 B와 대화할 때, C의 감정은 처리 불가 | 중간 |
| **확장 어려움** | 새 기능(감정 decay, 기억, 소문 등)은 MindService 수정 필요 | 높음 |

---

## 2. 목표 아키텍처 개요

```
┌─────────────────────────────────────────────────────────────┐
│                      Game Runtime                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │ Emotion  │  │ Dialogue │  │  Story   │  │ Memory   │    │
│  │  Agent   │  │  Agent   │  │  Agent   │  │  Agent   │    │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘    │
│       │              │              │              │         │
│  ═════╪══════════════╪══════════════╪══════════════╪═════    │
│       │         EventBus (tokio::broadcast)        │         │
│  ═════╪══════════════╪══════════════╪══════════════╪═════    │
│       │              │              │              │         │
│  ┌────┴─────────────────────────────┴────────────────┐      │
│  │              Command Handlers (Write)              │      │
│  │  AppraiseCmd · StimulusCmd · EndDialogueCmd · ...  │      │
│  └────┬───────────────────────────────────────────────┘      │
│       │                                                      │
│  ┌────┴──────────┐    ┌──────────────────┐                   │
│  │  Event Store  │───▶│  Read Projections │                  │
│  │  (Append-only)│    │  (Query Side)     │                  │
│  └───────────────┘    └──────────────────┘                   │
│                              │                               │
│                        ┌─────┴──────┐                        │
│                        │  RAG Index │                        │
│                        │ (History)  │                        │
│                        └────────────┘                        │
└─────────────────────────────────────────────────────────────┘
```

### 핵심 원칙

1. **모든 상태 변경은 Command → Event → Store** 경로만 허용
2. **에이전트는 EventBus로만 소통** — 직접 메서드 호출 없음
3. **Query는 Projection에서만 읽음** — Event Store 직접 읽기 금지
4. **Event Store는 append-only** — 삭제/수정 없음, 스냅샷으로 최적화

---

## 3. Event 설계

### 3.1 도메인 이벤트 카탈로그

```rust
/// 모든 도메인 이벤트의 루트 enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub id: EventId,            // UUID
    pub timestamp: Timestamp,   // 논리적 시간 (턴 번호 + wall clock)
    pub aggregate_id: String,   // NPC ID (주체)
    pub sequence: u64,          // 해당 aggregate 내 순번
    pub payload: EventPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPayload {
    // ── Scene 라이프사이클 ──
    SceneStarted {
        npc_id: String,
        partner_id: String,
        scene_description: String,
        focuses: Vec<SceneFocus>,
        significance: f32,
    },
    SceneEnded {
        npc_id: String,
        partner_id: String,
        total_turns: u32,
        final_mood: f32,
    },

    // ── 감정 (Appraisal) ──
    EmotionAppraised {
        npc_id: String,
        situation: Situation,
        emotions: Vec<EmotionSnapshot>,  // (type, intensity, context)
        dominant: Option<EmotionType>,
        mood: f32,
    },
    
    // ── 감정 (Stimulus) ──
    StimulusApplied {
        npc_id: String,
        utterance: String,              // 원문
        pad: Pad,                       // 분석 결과
        before_emotions: Vec<EmotionSnapshot>,
        after_emotions: Vec<EmotionSnapshot>,
        mood_delta: f32,
    },

    // ── Beat 전환 ──
    BeatTransitioned {
        npc_id: String,
        from_focus_id: String,
        to_focus_id: String,
        trigger_conditions: Vec<EmotionCondition>,
        merged_emotions: Vec<EmotionSnapshot>,
    },

    // ── 가이드 생성 ──
    GuideGenerated {
        npc_id: String,
        directive: ActingDirective,
        prompt_hash: String,            // 프롬프트 변경 감지용
    },

    // ── 대화 ──
    DialogueTurnCompleted {
        npc_id: String,
        partner_id: String,
        turn_number: u32,
        speaker: DialogueRole,
        utterance: String,
        pad: Option<Pad>,               // 분석된 PAD (있으면)
        response: Option<String>,       // NPC 응답 (있으면)
        timings: Option<LlamaTimings>,
    },

    // ── 관계 ──
    RelationshipUpdated {
        owner_id: String,
        target_id: String,
        before: RelationshipValues,
        after: RelationshipValues,
        cause: RelationshipCause,       // Dialogue / BeatTransition / GameEvent
    },

    // ── NPC 월드 ──
    // ── 컨텍스트 관리 ──
    ContextSummarized {
        session_id: String,
        from_turn: u32,
        to_turn: u32,
        summary: String,
        key_emotions: Vec<EmotionSnapshot>,
    },

    // ── NPC 월드 ──
    NpcCreated { npc: Npc },
    NpcRemoved { npc_id: String },
    ObjectInteracted { npc_id: String, object_id: String, appealingness: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipCause {
    DialogueEnd { mood: f32, significance: f32 },
    BeatTransition { focus_id: String },
    GameEvent { description: String },
}
```

### 3.2 이벤트 순서 규약

하나의 대화 턴에서 발생하는 이벤트 순서:

```
1. DialogueTurnCompleted (상대 대사 기록)
2. StimulusApplied       (감정 변동 적용)
3. [조건부] BeatTransitioned   (Focus 조건 충족 시)
4. [조건부] EmotionAppraised   (새 Beat appraisal)
5. GuideGenerated        (갱신된 가이드)
6. DialogueTurnCompleted (NPC 응답 기록)
```

### 3.3 이벤트 메타데이터

```rust
pub struct EventMetadata {
    pub causation_id: Option<EventId>,  // 이 이벤트를 발생시킨 Command ID
    pub correlation_id: CorrelationId,  // 같은 요청에서 파생된 이벤트 묶음
    pub agent_id: Option<String>,       // 어떤 에이전트가 발생시켰는가
}
```

---

## 4. CQRS 설계

### 4.1 Command Side (Write)

```rust
/// Command 정의 — 모든 상태 변경 요청
#[derive(Debug, Clone)]
pub enum Command {
    StartScene(StartSceneCmd),
    Appraise(AppraiseCmd),
    ApplyStimulus(ApplyStimulusCmd),
    TransitionBeat(TransitionBeatCmd),
    GenerateGuide(GenerateGuideCmd),
    EndDialogue(EndDialogueCmd),
    UpdateRelationship(UpdateRelationshipCmd),
}

/// Command → Events 변환 핸들러
#[async_trait]
pub trait CommandHandler<C> {
    type Error;
    /// Command를 처리하고 0개 이상의 이벤트를 반환
    async fn handle(&self, cmd: C) -> Result<Vec<DomainEvent>, Self::Error>;
}
```

**핸들러 구현 예시** (`ApplyStimulusHandler`):

```rust
pub struct ApplyStimulusHandler {
    stimulus_engine: Arc<StimulusEngine>,
    event_store: Arc<dyn EventStore>,
}

#[async_trait]
impl CommandHandler<ApplyStimulusCmd> for ApplyStimulusHandler {
    async fn handle(&self, cmd: ApplyStimulusCmd) -> Result<Vec<DomainEvent>> {
        // 1. Projection에서 현재 감정 상태 재구성
        let current_state = self.event_store
            .rebuild_emotion_state(&cmd.npc_id).await?;
        
        // 2. 순수 도메인 로직 실행
        let npc = self.event_store.rebuild_npc(&cmd.npc_id).await?;
        let new_state = self.stimulus_engine
            .apply_stimulus(&npc.profile, &current_state, &cmd.pad);
        
        // 3. 이벤트 생성 (저장은 호출자가)
        let mut events = vec![DomainEvent::stimulus_applied(
            &cmd.npc_id, &cmd.utterance, &cmd.pad,
            &current_state, &new_state,
        )];
        
        // 4. Beat 전환 체크
        if let Some(transition) = self.check_beat_trigger(&new_state).await? {
            events.extend(transition);
        }
        
        Ok(events)
    }
}
```

### 4.2 Query Side (Read Projections)

```rust
/// Read Model — 이벤트 스트림에서 파생된 뷰
pub trait Projection: Send + Sync {
    /// 새 이벤트 수신 시 뷰 갱신
    fn apply(&mut self, event: &DomainEvent);
}

// ── 주요 Projection들 ──

/// NPC별 현재 감정 상태 (최신 스냅샷)
pub struct EmotionProjection {
    states: HashMap<String, EmotionState>,  // npc_id → 현재 감정
}

/// NPC별 관계 현황
pub struct RelationshipProjection {
    relationships: HashMap<(String, String), Relationship>,
}

/// 대화 히스토리 (RAG 인덱싱 대상)
pub struct DialogueHistoryProjection {
    turns: Vec<DialogueTurnRecord>,
    // 감정 변화 타임라인도 포함
    emotion_timeline: HashMap<String, Vec<EmotionTimepoint>>,
}

/// Scene 상태 (활성 Focus, Beat 진행)
pub struct SceneProjection {
    active_scenes: HashMap<String, SceneState>,
}

/// 가이드 캐시 (최신 프롬프트)
pub struct GuideProjection {
    guides: HashMap<String, (ActingGuide, String)>,  // npc_id → (guide, prompt)
}
```

### 4.3 CQRS 데이터 흐름

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│  Command    │────▶│  Command     │────▶│ Event Store │
│  (Write)    │     │  Handler     │     │ (append)    │
└─────────────┘     └──────────────┘     └──────┬──────┘
                                                │
                                    ┌───────────┤  (publish)
                                    │           │
                              ┌─────▼─────┐  ┌──▼──────────┐
                              │ Projection│  │  EventBus   │
                              │ Updater   │  │ (broadcast) │
                              └─────┬─────┘  └──┬──────────┘
                                    │            │
                              ┌─────▼─────┐  ┌──▼──────────┐
                              │  Read DB  │  │  Agents     │
                              │ (Query)   │  │ (subscribe) │
                              └───────────┘  └─────────────┘
```

---

## 5. Event Store 설계

### 5.1 저장 구조

```rust
#[async_trait]
pub trait EventStore: Send + Sync {
    /// 이벤트 추가 (append-only)
    async fn append(&self, events: Vec<DomainEvent>) -> Result<(), StoreError>;
    
    /// 특정 aggregate의 이벤트 스트림 조회
    async fn get_events(
        &self, 
        aggregate_id: &str, 
        after_sequence: Option<u64>,
    ) -> Result<Vec<DomainEvent>, StoreError>;
    
    /// 특정 타입의 이벤트만 필터
    async fn get_events_by_type(
        &self,
        event_type: &str,
        after: Option<Timestamp>,
    ) -> Result<Vec<DomainEvent>, StoreError>;
    
    /// 스냅샷 저장 (N턴마다)
    async fn save_snapshot(
        &self,
        aggregate_id: &str,
        sequence: u64,
        state: &[u8],  // serialized aggregate state
    ) -> Result<(), StoreError>;
    
    /// 스냅샷 로드 (최신)
    async fn load_snapshot(
        &self,
        aggregate_id: &str,
    ) -> Result<Option<(u64, Vec<u8>)>, StoreError>;
}
```

### 5.2 구현 전략 — 단계별

| 단계 | 저장소 | 용도 |
|------|--------|------|
| **Phase 1** | `InMemoryEventStore` | 개발/테스트. `Vec<DomainEvent>` + `HashMap<String, Vec<usize>>` |
| **Phase 2** | `FileEventStore` | 싱글 프로세스 게임. Append-only JSON Lines 파일 |
| **Phase 3** | `SqliteEventStore` | 프로덕션. WAL 모드, aggregate_id 인덱스 |

### 5.3 스냅샷 정책

```
매 SNAPSHOT_INTERVAL(20) 이벤트마다 또는
Scene 종료(SceneEnded) 시 자동 스냅샷 생성

스냅샷 = {
    npc_states: HashMap<String, Npc>,
    emotion_states: HashMap<String, EmotionState>,
    relationships: HashMap<(String,String), Relationship>,
    active_scene: Option<Scene>,
}

재구성: latest_snapshot + 이후 이벤트 replay
```

---

## 6. EventBus 설계

### 6.1 버스 구조

```rust
use tokio::sync::broadcast;

pub struct EventBus {
    sender: broadcast::Sender<Arc<DomainEvent>>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }
    
    /// 이벤트 발행 — Event Store 저장 후 호출
    pub fn publish(&self, event: Arc<DomainEvent>) {
        // lagged receivers는 최신 이벤트부터 재수신
        let _ = self.sender.send(event);
    }
    
    /// 구독자 생성
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<DomainEvent>> {
        self.sender.subscribe()
    }
}
```

### 6.2 이벤트 흐름 보장

```
Command 수신
  → CommandHandler.handle() → Vec<DomainEvent>
  → EventStore.append(events)    ← 영속화 먼저
  → EventBus.publish(events)     ← 성공 후 발행
  → Projections.apply(events)    ← 비동기 갱신
  → Agents receive via subscribe ← 비동기 반응
```

**At-least-once 보장**: EventStore에 저장된 이벤트는 반드시 발행. 재시작 시 마지막 발행 sequence 이후 이벤트를 재발행.

### 6.3 이벤트 필터링

에이전트별로 관심 이벤트만 수신:

```rust
pub struct EventFilter {
    pub event_types: Option<HashSet<String>>,   // None = 전부
    pub aggregate_ids: Option<HashSet<String>>,  // None = 전부
}

impl EventFilter {
    pub fn matches(&self, event: &DomainEvent) -> bool {
        // type + aggregate 모두 일치해야 통과
        self.type_matches(event) && self.aggregate_matches(event)
    }
}
```

---

## 7. Multi-Agent 아키텍처

### 7.1 에이전트 설계 원칙

1. **단일 목표(Single Goal)**: 각 에이전트는 하나의 도메인 책임만 담당
2. **이벤트 구동(Event-Driven)**: EventBus 구독으로 작동, 직접 호출 없음
3. **Command 발행**: 상태 변경이 필요하면 Command를 발행 (직접 저장 금지)
4. **Tool 사용**: 외부 리소스(LLM, 임베딩, RAG) 접근은 Tool 인터페이스로 추상화
5. **상태 없음(Stateless)**: 에이전트 자체는 상태를 보유하지 않음. Projection에서 읽기

### 7.2 에이전트 카탈로그

```
┌─────────────────────────────────────────────────────────┐
│                    Agent Registry                        │
├──────────────┬──────────────────┬────────────────────────┤
│   Agent      │   Single Goal   │   Subscribes To        │
├──────────────┼──────────────────┼────────────────────────┤
│ EmotionAgent │ 감정 평가/변동   │ SceneStarted,          │
│              │                  │ DialogueTurnCompleted   │
├──────────────┼──────────────────┼────────────────────────┤
│ DialogueAgent│ NPC 대사 생성    │ GuideGenerated,        │
│              │                  │ DialogueTurnCompleted   │
├──────────────┼──────────────────┼────────────────────────┤
│ StoryAgent   │ 서사 진행 판단   │ BeatTransitioned,      │
│              │                  │ RelationshipUpdated,    │
│              │                  │ SceneEnded             │
├──────────────┼──────────────────┼────────────────────────┤
│ MemoryAgent  │ 기억 저장/검색   │ DialogueTurnCompleted, │
│              │                  │ RelationshipUpdated,    │
│              │                  │ SceneEnded             │
├──────────────┼──────────────────┼────────────────────────┤
│ GuideAgent   │ 연기 가이드 생성 │ EmotionAppraised,      │
│              │                  │ StimulusApplied,        │
│              │                  │ BeatTransitioned       │
├──────────────┼──────────────────┼────────────────────────┤
│ RelAgent     │ 관계 갱신        │ SceneEnded,            │
│              │                  │ BeatTransitioned       │
├──────────────┼──────────────────┼────────────────────────┤
│ SummaryAgent │ 대화 컨텍스트    │ DialogueTurnCompleted  │
│              │ 압축/요약        │                        │
└──────────────┴──────────────────┴────────────────────────┘
```

### 7.3 에이전트 트레이트

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    /// 에이전트 이름 (로깅/디버그)
    fn name(&self) -> &str;
    
    /// 관심 이벤트 필터
    fn event_filter(&self) -> EventFilter;
    
    /// 이벤트 수신 시 처리
    /// 반환: 발행할 Command 목록 (없으면 빈 Vec)
    async fn on_event(
        &self, 
        event: &DomainEvent,
        ctx: &AgentContext,
    ) -> Result<Vec<Command>, AgentError>;
}

/// 에이전트가 사용할 수 있는 컨텍스트
pub struct AgentContext {
    pub projections: Arc<ProjectionStore>,   // Read-only 뷰
    pub tools: Arc<ToolRegistry>,            // LLM, 임베딩 등
    pub event_store: Arc<dyn EventStore>,    // 히스토리 조회
}
```

### 7.4 에이전트 상세 설계

#### EmotionAgent — 감정 평가 전담

```rust
pub struct EmotionAgent {
    appraisal_engine: AppraisalEngine,
    stimulus_engine: StimulusEngine,
}

impl Agent for EmotionAgent {
    fn name(&self) -> &str { "EmotionAgent" }
    
    fn event_filter(&self) -> EventFilter {
        EventFilter::types(&["SceneStarted", "DialogueTurnCompleted"])
    }
    
    async fn on_event(&self, event: &DomainEvent, ctx: &AgentContext) -> Result<Vec<Command>> {
        match &event.payload {
            // Scene 시작 → 초기 감정 평가
            EventPayload::SceneStarted { npc_id, .. } => {
                let situation = ctx.projections.scene.get_initial_situation(npc_id)?;
                Ok(vec![Command::Appraise(AppraiseCmd {
                    npc_id: npc_id.clone(),
                    situation,
                })])
            }
            
            // 상대 대사 → PAD 분석 → stimulus 적용
            EventPayload::DialogueTurnCompleted { 
                npc_id, speaker: DialogueRole::User, utterance, ..
            } => {
                // Tool로 PAD 분석
                let pad = ctx.tools.utterance_analyzer()
                    .analyze(utterance).await?;
                
                Ok(vec![Command::ApplyStimulus(ApplyStimulusCmd {
                    npc_id: npc_id.clone(),
                    utterance: utterance.clone(),
                    pad,
                })])
            }
            
            _ => Ok(vec![])
        }
    }
}
```

#### DialogueAgent — NPC 대사 생성 전담

```rust
pub struct DialogueAgent;

impl Agent for DialogueAgent {
    fn name(&self) -> &str { "DialogueAgent" }
    
    fn event_filter(&self) -> EventFilter {
        // GuideGenerated를 받으면 → LLM으로 대사 생성
        EventFilter::types(&["GuideGenerated"])
    }
    
    async fn on_event(&self, event: &DomainEvent, ctx: &AgentContext) -> Result<Vec<Command>> {
        if let EventPayload::GuideGenerated { npc_id, .. } = &event.payload {
            // 1. Projection에서 최신 가이드 가져옴
            let (guide, prompt) = ctx.projections.guide.get(npc_id)?;
            
            // 2. RAG로 관련 기억 검색
            let memories = ctx.tools.rag()
                .search_relevant(npc_id, &guide.situation_description, 5).await?;
            
            // 3. LLM Tool로 대사 생성
            let enriched_prompt = format!("{}\n\n[기억]\n{}", prompt, memories.format());
            let response = ctx.tools.llm()
                .generate(npc_id, &enriched_prompt).await?;
            
            // 4. 대화 턴 완료 커맨드 발행
            Ok(vec![Command::CompleteTurn(CompleteTurnCmd {
                npc_id: npc_id.clone(),
                speaker: DialogueRole::Assistant,
                utterance: response.text,
                timings: response.timings,
            })])
        } else {
            Ok(vec![])
        }
    }
}
```

#### StoryAgent — 서사 진행 판단

```rust
pub struct StoryAgent;

impl Agent for StoryAgent {
    fn name(&self) -> &str { "StoryAgent" }
    
    fn event_filter(&self) -> EventFilter {
        EventFilter::types(&[
            "BeatTransitioned", "RelationshipUpdated", "SceneEnded"
        ])
    }
    
    async fn on_event(&self, event: &DomainEvent, ctx: &AgentContext) -> Result<Vec<Command>> {
        match &event.payload {
            EventPayload::SceneEnded { npc_id, partner_id, final_mood, .. } => {
                // 1. 히스토리에서 이 두 캐릭터의 관계 추이 조회
                let rel_history = ctx.event_store
                    .get_events_by_type("RelationshipUpdated", None).await?
                    .into_iter()
                    .filter(|e| matches_pair(e, npc_id, partner_id))
                    .collect::<Vec<_>>();
                
                // 2. LLM에게 서사 방향 판단 요청 (Tool)
                let narrative_analysis = ctx.tools.llm()
                    .analyze_narrative(&rel_history, *final_mood).await?;
                
                // 3. 다음 Scene 제안 (이벤트로 기록만, 실행은 게임 측)
                // StoryAgent는 제안만 하고 결정은 게임 로직에 위임
                Ok(vec![])
            }
            _ => Ok(vec![])
        }
    }
}
```

#### MemoryAgent — 기억 관리 (RAG 인덱싱)

```rust
pub struct MemoryAgent;

impl Agent for MemoryAgent {
    fn name(&self) -> &str { "MemoryAgent" }
    
    fn event_filter(&self) -> EventFilter {
        EventFilter::types(&[
            "DialogueTurnCompleted", "RelationshipUpdated", 
            "BeatTransitioned", "SceneEnded"
        ])
    }
    
    async fn on_event(&self, event: &DomainEvent, ctx: &AgentContext) -> Result<Vec<Command>> {
        match &event.payload {
            // 모든 대화 턴을 RAG 인덱스에 추가
            EventPayload::DialogueTurnCompleted { 
                npc_id, utterance, pad, ..
            } => {
                let memory_entry = MemoryEntry {
                    npc_id: npc_id.clone(),
                    content: utterance.clone(),
                    emotional_context: pad.clone(),
                    timestamp: event.timestamp,
                    event_id: event.id,
                };
                ctx.tools.rag().index(memory_entry).await?;
                Ok(vec![])
            }
            
            // 관계 변화를 의미 있는 기억으로 저장
            EventPayload::RelationshipUpdated { 
                owner_id, target_id, before, after, cause, ..
            } => {
                let significance = relationship_change_significance(before, after);
                if significance > 0.1 {
                    let summary = format_relationship_memory(
                        owner_id, target_id, before, after, cause
                    );
                    ctx.tools.rag().index(MemoryEntry {
                        npc_id: owner_id.clone(),
                        content: summary,
                        emotional_context: None,
                        timestamp: event.timestamp,
                        event_id: event.id,
                    }).await?;
                }
                Ok(vec![])
            }
            
            _ => Ok(vec![])
        }
    }
}
```

### 7.5 에이전트 순차 도입 계획

```
Phase 1: EmotionAgent + GuideAgent
  - 현재 MindService.appraise() + apply_stimulus() + generate_guide() 분리
  - InMemoryEventStore + 기본 Projections
  - 기존 테스트 통과 확인

Phase 2: + MemoryAgent + RAG
  - DialogueHistoryProjection → RAG 인덱스 연동
  - 에이전트 간 이벤트 흐름 검증

Phase 3: + DialogueAgent
  - LLM 대화를 이벤트 기반으로 전환
  - ConversationPort를 Tool로 래핑

Phase 4: + StoryAgent
  - 서사 진행 분석
  - 다중 NPC 관점 지원
```

---

## 8. Tool 시스템

### 8.1 Tool 트레이트

```rust
/// 에이전트가 사용하는 외부 도구 추상화
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    
    async fn execute(
        &self, 
        input: ToolInput,
    ) -> Result<ToolOutput, ToolError>;
}

/// 도구 레지스트리 — 에이전트에게 주입
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn utterance_analyzer(&self) -> &dyn UtteranceAnalyzerTool { .. }
    pub fn llm(&self) -> &dyn LlmTool { .. }
    pub fn rag(&self) -> &dyn RagTool { .. }
    pub fn world(&self) -> &dyn WorldKnowledgeTool { .. }
    pub fn formatter(&self) -> &dyn FormatterTool { .. }
}
```

### 8.2 핵심 Tool 목록

| Tool | 용도 | 현재 대응 | 래핑 대상 |
|------|------|----------|----------|
| `UtteranceAnalyzerTool` | 대사→PAD 변환 | `PadAnalyzer` + `OrtEmbedder` | embed feature |
| `LlmTool` | LLM 대화/분석 | `ConversationPort` + `RigChatAdapter` | chat feature |
| `RagTool` | 게임 내 기억 검색 | 없음 (신규) | embed feature 확장 |
| `WorldKnowledgeTool` | 세계관 지식 검색 | 없음 (신규) | embed feature 확장 |
| `FormatterTool` | 가이드→프롬프트 포맷팅 | `LocaleFormatter` | presentation |
| `EventQueryTool` | 이벤트 스토어 조회 | 없음 (신규) | EventStore |

---

## 9. RAG 설계 (게임 내 히스토리)

### 9.1 인덱싱 대상

```
┌──────────────────────────────────────────┐
│              RAG Index                    │
├──────────────┬───────────────────────────┤
│  소스        │ 내용                      │
├──────────────┼───────────────────────────┤
│ 대화 기록    │ 턴별 대사 + PAD + 감정    │
│ 관계 변화    │ closeness/trust 변동 사유  │
│ Beat 전환    │ 감정 조건 + 새 상황       │
│ Scene 요약   │ LLM이 생성한 Scene 요약   │
│ 주요 사건    │ 게임 이벤트 (외부 주입)   │
└──────────────┴───────────────────────────┘
```

### 9.2 RAG 포트

```rust
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// 기억 항목 인덱싱 (임베딩 + 저장)
    async fn index(&self, entry: MemoryEntry) -> Result<(), MemoryError>;
    
    /// 유사 기억 검색
    async fn search(
        &self,
        query: &str,
        npc_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>, MemoryError>;
    
    /// 시간 범위 필터링
    async fn search_temporal(
        &self,
        query: &str,
        npc_id: &str,
        after: Timestamp,
        before: Timestamp,
        limit: usize,
    ) -> Result<Vec<MemoryResult>, MemoryError>;
}

#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub npc_id: String,
    pub content: String,
    pub emotional_context: Option<Pad>,
    pub timestamp: Timestamp,
    pub event_id: EventId,
    pub memory_type: MemoryType,  // Dialogue, Relationship, Event, Summary
}

#[derive(Debug, Clone)]
pub struct MemoryResult {
    pub entry: MemoryEntry,
    pub relevance_score: f32,
    pub emotional_distance: Option<f32>,  // PAD 거리 (감정 유사도)
}
```

### 9.3 하이브리드 저장 구조 (SQLite + LanceDB)

텍스트 검색과 벡터 검색은 최적 구조가 다르므로, 각자 강한 DB에 나눠 저장하고 **같은 id로 연결**한다.

```
저장 시:
  MemoryEntry { id: "mem-042", content: "약속을 어긴 자가...", npc_id, timestamp, ... }
    │
    ├─ SQLite: 원문 텍스트 + 메타데이터 + FTS5 인덱스
    │   id, npc_id, content, timestamp, emotion, event_id, memory_type
    │
    └─ LanceDB: 임베딩 벡터만
        id, vector[768]
    
  양쪽이 같은 id를 공유 → 이것이 포인터 역할
```

**검색 경로 A — 키워드 검색 (SQLite 시작)**:
```
"무림맹주" 검색
  → SQLite FTS5 매칭: mem-042, mem-088 찾음
  → 이 id들로 LanceDB에서 벡터 조회 (필요 시)
  → 벡터로 유사 기억 확장 검색 가능
```

**검색 경로 B — 의미 검색 (LanceDB 시작)**:
```
"배신당한 기분" 검색
  → bge-m3로 임베딩 → LanceDB에서 유사 벡터 top 5
  → 돌아온 id들로 SQLite에서 원문 + 메타데이터 읽기
  → npc_id, timestamp, emotion 등 즉시 활용
```

**왜 분리하는가**: SQLite는 텍스트 검색(FTS5)과 메타데이터 필터링에 최적화되어 있고, LanceDB는 벡터 ANN 검색에 최적화되어 있다. 한 DB에 둘 다 넣으면 어느 쪽도 최적이 아니다. 각자 잘하는 걸 하게 두고 id로 연결하는 것이 성능과 유연성 모두에서 유리하다.

```rust
pub struct HybridMemoryStore {
    sqlite: SqliteConnection,   // 텍스트 + 메타데이터 + FTS5
    lance: LanceDB,             // 벡터만
    embedder: Arc<dyn TextEmbedder>,
}

impl MemoryStore for HybridMemoryStore {
    async fn index(&self, entry: MemoryEntry) -> Result<()> {
        let embedding = self.embedder.embed(&[&entry.content])?;
        let id = entry.id.clone();
        
        // 같은 id로 양쪽에 저장
        self.sqlite.insert(&entry).await?;           // 텍스트 + 메타데이터
        self.lance.insert(&id, &embedding[0]).await?; // 벡터
        Ok(())
    }
    
    async fn search_by_keyword(
        &self, keyword: &str, npc_id: Option<&str>, limit: usize,
    ) -> Result<Vec<MemoryResult>> {
        // SQLite FTS5에서 시작 → 필요 시 LanceDB로 건너감
        let hits = self.sqlite.fts_search(keyword, npc_id, limit).await?;
        Ok(hits)
    }
    
    async fn search_by_meaning(
        &self, query: &str, npc_id: Option<&str>, limit: usize,
    ) -> Result<Vec<MemoryResult>> {
        // LanceDB에서 시작 → SQLite에서 메타데이터 보강
        let query_vec = self.embedder.embed(&[query])?;
        let vec_hits = self.lance.search(&query_vec[0], limit).await?;
        let ids: Vec<&str> = vec_hits.iter().map(|h| h.id.as_str()).collect();
        let full_results = self.sqlite.get_by_ids(&ids, npc_id).await?;
        Ok(full_results)
    }
}
```

### 9.4 검색 전략

```
NPC가 대사를 생성할 때:
  1. 현재 상황 description → search_by_meaning (LanceDB → SQLite) (top 3)
  2. 현재 대화 상대 이름 → search_by_keyword (SQLite → LanceDB) (top 2)
  3. 현재 감정과 유사한 과거 기억 → search_by_meaning + emotion 필터 (top 2)
  4. 결과 합산 후 중복 제거 → 최대 5개 기억을 프롬프트에 주입
```

### 9.5 구현 전략

| 단계 | 구현 | 비고 |
|------|------|------|
| Phase 1 | In-memory Vec + brute-force cosine | 개발/테스트용. 벡터 분리 없음 |
| Phase 2 | **SQLite(FTS5) + LanceDB(벡터)** | 하이브리드. 서버 없이 파일 기반. id로 연결 |
| Phase 3 | 필요 시 LanceDB → 외부 벡터DB 교체 | MemoryStore 포트 뒤에 숨어있으므로 교체 용이 |

---

## 10. 세계관 지식 저장소 (WorldKnowledgeStore)

### 10.1 동적 데이터 vs 정적 데이터

```
동적 데이터 (Event Store + HybridMemoryStore):
  "턴 15에서 무림맹주가 배신 암시에 Fear가 올랐다"
  → 게임 중에 생김. 이벤트로 기록. MemoryAgent가 인덱싱.

정적 데이터 (WorldKnowledgeStore):
  "무림맹주는 중원 무림을 총괄하는 최고 직위이다"
  "화산파는 검법으로 유명하며, 본산은 화산에 있다"
  → 게임 전에 이미 정해져 있음. 플레이 중에 바뀌지 않음.
```

세계관 지식은 이벤트(일어난 일)가 아니라 **설정(존재하는 사실)**이므로 Event Store에 넣지 않는다. 별도의 읽기 전용 저장소를 둔다.

### 10.2 세계관 데이터 카테고리

```
WorldKnowledge
  ├─ 문파/조직    "화산파: 검법 중심, 본산 화산, 주요 인물..."
  ├─ 인물         "장무기: 명교 교주, 구양신공 보유, 성격..."
  ├─ 무공/기술    "구양신공: 양의 내공, 습득 조건, 위력..."
  ├─ 장소         "광명정: 명교 본거지, 지리적 특징..."
  └─ 규칙/관습    "무림맹 소집 절차, 사제 서열의 의미..."
```

### 10.3 저장 구조 (하이브리드, 읽기 전용)

게임 내 기억과 같은 SQLite + LanceDB 하이브리드 구조를 사용하되, **읽기 전용**이다.

```
WorldKnowledgeStore:
  SQLite (FTS5)                        LanceDB
  ┌───────────────────────────┐       ┌──────────────────┐
  │ id: "wk-042"              │──────▶│ id: "wk-042"     │
  │ category: "문파"           │       │ vector: [0.3,...] │
  │ name: "화산파"             │◀──────│                  │
  │ content: "검법 중심의..."  │       └──────────────────┘
  │ tags: ["검법","중원"]      │
  │ related: ["wk-015","wk-088"]  ← 관련 항목 링크
  └───────────────────────────┘
```

게임 내 기억과 다른 점:
- `category`, `tags`, `related` 등 구조화된 메타데이터 (사전 정리 가능)
- 불변 — MemoryAgent가 인덱싱하지 않음. 게임 개발 시 사전 구축
- 관련 항목 링크로 그래프 탐색 가능

### 10.4 WorldKnowledgeTool

```rust
pub struct WorldKnowledgeTool {
    store: Arc<WorldKnowledgeStore>,
}

impl WorldKnowledgeTool {
    /// 의미 검색: "장무기가 배운 무공이 뭐지?"
    async fn search(&self, query: &str, limit: usize) -> Vec<KnowledgeResult>;
    
    /// 카테고리 필터: 문파 정보만
    async fn search_by_category(
        &self, query: &str, category: &str, limit: usize,
    ) -> Vec<KnowledgeResult>;
    
    /// 특정 항목 직접 조회: "화산파"
    async fn get_by_name(&self, name: &str) -> Option<KnowledgeEntry>;
    
    /// 관련 항목 탐색: "화산파" → 관련 인물, 무공 목록
    async fn get_related(&self, id: &str) -> Vec<KnowledgeEntry>;
}
```

### 10.5 관계 정보의 3계층 구조

인물 간의 관계는 세 가지 성격이 섞여 있으며, 각각 다른 저장소와 도메인이 관리한다.

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 1: 세계관 관계 (WorldKnowledgeStore)                 │
│  불변. 개발 시 설정. 아무도 안 건드림.                       │
│  "장무기-주지약: 연인", "화산파-소림파: 동맹"                │
│  구조적 관계 타입: 혈연, 사제, 연인, 동맹, 적대, 은인...    │
│  검색: WorldKnowledgeTool.get_relationships("장무기")       │
├─────────────────────────────────────────────────────────────┤
│  Layer 2: 수치적 관계 (RelationshipProjection)              │
│  게임 중 매 Scene마다 갱신. RelAgent가 관리.                │
│  closeness, trust, power 3축 수치.                          │
│  현재 Relationship 도메인 모델이 그대로 유지됨.              │
│  검색: Projection 직접 읽기                                 │
├─────────────────────────────────────────────────────────────┤
│  Layer 3: 관계 기억 (HybridMemoryStore)                     │
│  "왜" 관계가 변했는지의 맥락. MemoryAgent가 인덱싱.         │
│  "3일 전 약속을 어겨서 trust가 떨어졌다"                    │
│  검색: RagTool.search("신뢰")                               │
└─────────────────────────────────────────────────────────────┘
```

**Layer 1**은 현재 엔진에 없는 개념이다. WorldKnowledgeStore에 관계 카테고리를 추가해야 한다.

```
WorldKnowledge — 관계 카테고리:
  { id: "wr-001", category: "관계",
    from: "장무기", to: "주지약",
    relation_type: "연인",
    content: "명교 교주와 의녀. 여러 위기를 함께 넘김.",
    tags: ["명교","연인"], related: ["wk-003","wk-015"] }
```

**Layer 2**는 현재 `Relationship` 도메인 모델(closeness, trust, power)이다. v3에서 도메인 로직은 그대로 유지. 저장소만 `InMemoryRepository`에서 `Event Store + RelationshipProjection`으로 전환.

**Layer 3**는 MemoryAgent가 `RelationshipUpdated` 이벤트에서 의미 있는 변화(변동량 > 0.1)만 골라서 인덱싱한 것이다.

DialogueAgent가 대사를 만들 때 세 층을 합친다:
```
"장무기"와 대화 중인 NPC:
  Layer 1 (세계관): "이 사람은 내 사제(師弟)다"
  Layer 2 (수치):   closeness: 0.5, trust: 0.1 (낮음)
  Layer 3 (기억):   "약속을 어겼다", "하지만 위기에서 구해줬다"
```

### 10.6 에이전트 활용

DialogueAgent가 대사를 만들 때 **네 가지 소스**에서 맥락을 조합한다.

```
DialogueAgent 대사 생성 시:
  1. RAG (게임 내 기억): "이 상대와 과거에 무슨 일이 있었지?"
     → "3일 전 약속을 어긴 적 있다"

  2. World (세계관 지식): "대화에서 언급된 '구양신공'이 뭐지?"
     → "구양신공: 양의 내공으로, 특징은..."

  3. World (세계관 관계): "이 상대와 나는 어떤 사이지?"
     → "사제 관계, 같은 문파"

  4. ContextProjection (현재 대화): 요약본 + 최근 턴 원문

  → 합쳐서 LLM에 전달:
     [가이드] tone: 경계하는, attitude: 방어적
     [세계관] 구양신공은 양의 내공...
     [관계-설정] 사제 관계 / [관계-수치] trust 낮음
     [기억] 3일 전 약속을 어겼음
     [요약] 이전 대화: 재회 후 사부의 죽음에 대해 논의...
     [최근 대화] 턴 12~15 원문
```

StoryAgent도 서사 방향 판단 시 세계관을 참조한다.
"이 두 문파는 원래 적대 관계인데 지금 협력 중이다 → 갈등 발생 가능성" 같은 판단에 문파 간 관계 정보가 필요하다.

### 10.7 빌드 파이프라인

세계관 데이터는 개발자가 별도 가공한 설정 자료(세계관 문서, 캐릭터 시트 등)를 입력으로 받아 게임 개발 시 1회 구축한다. 원작 소설 원문은 저작권 문제가 있으므로 직접 포함하지 않으며, 개발자가 오리지널 세계관을 설계하거나 허락된 범위에서 가공한 자료를 사용한다.

```
개발 시 (1회):
  세계관 설정 자료 (개발자가 준비)
    → 구조화 도구로 카테고리별 분류 + 관련 항목 링크
    → bge-m3로 임베딩 생성
    → WorldKnowledgeStore (SQLite + LanceDB 파일)
    → 게임 데이터에 포함

게임 실행 시:
  WorldKnowledgeStore → 읽기 전용으로 로드
  에이전트들이 WorldKnowledgeTool로 조회
```

입력 자료의 위치와 형식은 프로젝트 진행 시 별도 결정한다.

---

## 11. 컨텍스트 윈도우 관리 (SummaryAgent)

### 11.1 문제

LLM 대화가 길어지면 컨텍스트 윈도우가 찬다. 오래된 대사를 그냥 잘라내면 NPC가 앞에서 한 말을 까먹는다.

### 11.2 SummaryAgent

`DialogueTurnCompleted` 이벤트를 구독하고, 컨텍스트 크기가 예산을 넘으면 오래된 턴들을 LLM으로 요약한다.

```
SummaryAgent 동작 조건:
  1. DialogueTurnCompleted 수신
  2. ContextProjection에서 현재 컨텍스트 토큰 수 추정
  3. CONTEXT_BUDGET(예: 6K 토큰)의 70%를 넘으면:
     → 오래된 턴들을 LLM Tool로 요약
     → SummarizeContext 커맨드 발행
     → ContextSummarized 이벤트 생성 (Event Store에 기록)
  4. 넘지 않으면: 아무것도 안 함
```

### 11.3 요약 적용

**Event Store는 건드리지 않는다.** 원본 대화 이벤트는 그대로 보존. 요약 사실 자체가 새 이벤트로 추가된다.

```
Event Store (append-only, 원본 유지):
  #301: DialogueTurnCompleted { turn: 1, "오랜만이군" }
  #302: DialogueTurnCompleted { turn: 2, "그래, 3년이나..." }
  ...
  #340: DialogueTurnCompleted { turn: 15, "너를 믿어도 되겠냐?" }
  #341: ContextSummarized { turns: 1..11, summary: "3년 만에 재회..." }  ← 요약 이벤트
```

요약이 적용되는 곳은 **ContextProjection** — DialogueAgent가 LLM에게 보낼 컨텍스트를 조립할 때 사용한다.

```
LLM 컨텍스트 조립 (DialogueAgent):
  ┌──────────────────────────────────────┐
  │ System Prompt (ActingGuide)   ~1.5K  │
  │ 세계관 (WorldKnowledge)       ~0.5K  │
  │ 기억 (RAG)                    ~0.5K  │
  │ [요약] 3년 만에 재회...       ~0.3K  │ ← 턴 1~11 압축
  │ 턴 12~15 원문                 ~1.5K  │ ← 최근은 원문 유지
  │ [여유 공간]                   ~3.7K  │
  └──────────────────────────────────────┘
```

### 11.4 ContextProjection

```rust
pub struct ContextProjection {
    /// 요약본 (세션별)
    summaries: HashMap<String, Vec<ContextSummary>>,
    /// 요약 이후의 원문 턴들
    recent_turns: HashMap<String, Vec<DialogueTurn>>,
    /// 요약 경계 (이 턴 번호까지 요약됨)
    summary_boundary: HashMap<String, u32>,
}

pub struct ContextSummary {
    pub from_turn: u32,
    pub to_turn: u32,
    pub summary_text: String,
    pub key_emotions: Vec<EmotionSnapshot>,
}
```

### 11.5 요약된 대사의 기억 접근

컨텍스트에서 잘려나간 대사도 RAG에는 남아 있다. "턴 3: 사부가 돌아가셨다"가 요약으로 압축되어도, HybridMemoryStore에서 검색하면 원문이 나온다.

```
컨텍스트에서 빠진 대사를 NPC가 언급해야 할 때:
  ContextProjection: [요약] "사부의 죽음을 전했고..."  ← 상세 없음
  HybridMemoryStore: "사부가 돌아가셨다" (원문, PAD, 감정 포함)  ← 검색 가능
  Event Store: #305 DialogueTurnCompleted { "사부가 돌아가셨다" }  ← 원본 보존
```

---

## 12. 전체 데이터 흐름 예시

### Scene 시작 → 첫 대사까지

```
[Game] StartScene 커맨드 발행
  │
  ├─ CommandHandler → SceneStarted 이벤트 저장 + 발행
  │
  ├─ EmotionAgent (SceneStarted 수신)
  │    → Appraise 커맨드 발행
  │    → CommandHandler → EmotionAppraised 이벤트
  │
  ├─ GuideAgent (EmotionAppraised 수신)
  │    → GenerateGuide 커맨드 발행
  │    → CommandHandler → GuideGenerated 이벤트
  │
  ├─ DialogueAgent (GuideGenerated 수신)
  │    → RAG로 관련 기억 검색
  │    → LLM으로 NPC 첫 대사 생성
  │    → CompleteTurn 커맨드 발행
  │    → CommandHandler → DialogueTurnCompleted 이벤트
  │
  └─ MemoryAgent (DialogueTurnCompleted 수신)
       → RAG 인덱스에 대사 기록
```

### 대화 턴 (Player 대사 → NPC 응답)

```
[Game] Player 대사 "너를 믿어도 되겠냐?" 입력
  │
  ├─ DialogueTurnCompleted(User, "너를 믿어도 되겠냐?") 이벤트
  │
  ├─ EmotionAgent 수신
  │    → UtteranceAnalyzer Tool로 PAD 분석: (P:-0.2, A:0.3, D:0.4)
  │    → ApplyStimulus 커맨드 발행
  │    → StimulusApplied 이벤트 (Fear↑, Hope↓)
  │    → [Beat 조건 미충족 → 전환 없음]
  │
  ├─ GuideAgent 수신 (StimulusApplied)
  │    → 감정 변화가 GUIDE_REGEN_THRESHOLD 초과 시
  │    → GenerateGuide 커맨드 → GuideGenerated 이벤트
  │
  ├─ DialogueAgent 수신 (GuideGenerated)
  │    → RAG: "신뢰" 관련 과거 기억 검색
  │      "3일 전 이 사람이 약속을 지킨 적 있다"
  │    → LLM: 갱신된 프롬프트 + 기억으로 대사 생성
  │      "...전에 약속을 지켰으니, 한 번 더 믿어보겠다."
  │    → DialogueTurnCompleted(Assistant) 이벤트
  │
  └─ MemoryAgent: 양쪽 대사 모두 인덱싱
```

---

## 13. 마이그레이션 전략

### 11.1 점진적 전환 (Strangler Fig Pattern)

현재 `MindService`를 한 번에 교체하지 않고, 이벤트 기반 래퍼로 감싸서 점진 전환:

```rust
/// Phase 1: 기존 MindService를 이벤트 발행 래퍼로 감쌈
pub struct EventAwareMindService<R: MindRepository> {
    inner: MindService<R>,
    event_bus: Arc<EventBus>,
    event_store: Arc<dyn EventStore>,
}

impl<R: MindRepository> EventAwareMindService<R> {
    pub fn appraise(&mut self, req: AppraiseRequest) -> Result<AppraiseResult> {
        // 1. 기존 로직 그대로 실행
        let result = self.inner.appraise(req.clone())?;
        
        // 2. 결과를 이벤트로도 기록
        let event = DomainEvent::emotion_appraised(&req, &result);
        block_on(self.event_store.append(vec![event.clone()]))?;
        self.event_bus.publish(Arc::new(event));
        
        Ok(result)
    }
}
```

### 11.2 전환 단계

```
Step 1: EventAwareMindService 래퍼
  - 기존 기능 100% 유지
  - 모든 호출에 이벤트 발행 추가
  - Event Store에 히스토리 축적 시작

Step 2: Projection 도입
  - EmotionProjection, RelationshipProjection
  - 기존 InMemoryRepository와 병행 — 결과 일치 검증

Step 3: EmotionAgent 추출
  - appraise/stimulus를 에이전트로 이관
  - MindService에서 해당 로직 제거

Step 4: 나머지 에이전트 순차 추출
  - GuideAgent → DialogueAgent → MemoryAgent → StoryAgent

Step 5: MindService 제거
  - 모든 로직이 에이전트로 이관 완료
  - EventAwareMindService → 순수 Command Dispatcher로 전환
```

---

## 14. Trade-off 분석

### 12.1 EventBus

| 장점 | 단점 |
|------|------|
| 에이전트 간 완전한 디커플링 | 디버깅 복잡도 증가 (이벤트 추적 필요) |
| 새 에이전트 추가 시 기존 코드 무수정 | 이벤트 순서 보장이 중요한 경우 복잡 |
| 비동기 처리로 병렬성 확보 | 최종 일관성(eventual consistency) |

**위험 완화**: correlation_id로 이벤트 체인 추적, tracing span으로 에이전트별 로깅

### 12.2 CQRS

| 장점 | 단점 |
|------|------|
| Read/Write 독립 스케일링 | 코드량 증가 (Command + Handler + Projection) |
| 읽기 최적화된 Projection | Projection 동기화 지연 가능 |
| 관심사 분리 명확 | 학습 곡선 |

**현실적 판단**: 싱글 프로세스 게임에서는 Projection이 동기적으로 갱신되므로 일관성 문제 없음

### 12.3 Event Sourcing

| 장점 | 단점 |
|------|------|
| 완전한 히스토리 (RAG 데이터 소스) | 스토리지 증가 (스냅샷으로 완화) |
| 시간 여행(temporal query) 가능 | 이벤트 스키마 진화 관리 필요 |
| 버그 재현이 이벤트 replay로 가능 | 초기 구현 비용 |

**핵심 가치**: NPC 게임에서 "과거에 무슨 일이 있었는가"는 게임플레이의 핵심. Event Sourcing이 자연스러운 선택.

### 12.4 Multi-Agent

| 장점 | 단점 |
|------|------|
| 각 에이전트 독립 테스트 가능 | 에이전트 간 협업 프로토콜 설계 필요 |
| 에이전트별 LLM 교체/튜닝 | 전체 시스템 동작 파악이 어려움 |
| 순차 도입으로 리스크 분산 | 오버엔지니어링 위험 |

**1인 개발자 고려**: Phase 1 (EmotionAgent + GuideAgent)만으로도 충분한 가치. 나머지는 필요 시 추가.

### 12.5 RAG

| 장점 | 단점 |
|------|------|
| NPC가 과거를 "기억" → 몰입감 | 임베딩 모델 의존성 (이미 embed feature 있음) |
| Event Store 데이터 자연 활용 | 검색 품질이 게임 품질에 직결 |
| 무협지 특유의 은원/인연 표현 가능 | 인덱싱 오버헤드 |

---

## 15. 기술 선택

| 결정 | 선택 | 이유 |
|------|------|------|
| EventBus 구현 | `tokio::broadcast` | 이미 tokio 사용 중, 단순, 충분한 성능 |
| Event Store (Phase 1) | In-memory | 빠른 프로토타이핑, 테스트 용이 |
| Event Store (Phase 2) | SQLite WAL | 싱글 프로세스, 파일 기반, Rust 생태계 성숙 |
| RAG 임베딩 | 기존 `bge-m3-onnx-rust` 재사용 | 추가 의존성 없음 |
| RAG 저장소 (Phase 1) | In-memory + cosine | 개발 속도 우선 |
| 직렬화 | `serde_json` | 이벤트 디버깅 용이, 스키마 진화에 유리 |
| 에이전트 런타임 | `tokio::spawn` + `broadcast::Receiver` | 기존 async 런타임 활용 |

---

## 16. 수정 사항 (현재 ports.rs 영향)

### 유지되는 포트

- `AppraisalWeights`, `StimulusWeights` — 순수 도메인, 변경 없음
- `Appraiser`, `StimulusProcessor` — 에이전트 내부에서 사용
- `GuideFormatter` — Tool로 래핑되지만 트레이트는 유지
- `TextEmbedder`, `UtteranceAnalyzer`, `PadAnchorSource` — Tool로 래핑

### 변경되는 포트

- `MindRepository` (NpcWorld + EmotionStore + SceneStore)
  → **Event Store + Projections**로 대체
  - `EmotionStore` → `EmotionProjection` (이벤트 파생)
  - `SceneStore` → `SceneProjection` (이벤트 파생)
  - `NpcWorld` → NPC 데이터는 이벤트 또는 초기 로딩

### 새로 추가되는 포트

- `EventStore` — 이벤트 영속화
- `MemoryStore` — RAG 인덱스
- `Agent` — 에이전트 트레이트
- `Tool` — 도구 추상화

---

## 17. 다음 단계

1. **Phase 1 구현 시작**: `DomainEvent` enum + `InMemoryEventStore` + `EventBus`
2. **EventAwareMindService 래퍼**: 기존 테스트 100% 통과 확인
3. **EmotionProjection + RelationshipProjection**: Event replay로 상태 재구성 검증
4. **EmotionAgent + GuideAgent 추출**: MindService에서 로직 이관
5. **MemoryAgent + RAG**: DialogueHistoryProjection → 임베딩 인덱싱

---

## 부록 A: 이벤트 스키마 진화 전략

```rust
/// 이벤트 버전 관리 — 하위 호환성 유지
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedEvent {
    pub version: u32,  // 스키마 버전
    pub payload: serde_json::Value,  // 원본 JSON
}

/// 업캐스터 — 구 버전 이벤트를 최신 버전으로 변환
pub trait Upcaster {
    fn can_upcast(&self, event_type: &str, version: u32) -> bool;
    fn upcast(&self, event: VersionedEvent) -> VersionedEvent;
}
```

규칙: 필드 추가는 `Option<T>`로. 필드 삭제/타입 변경은 새 버전 + Upcaster.

## 부록 B: 모니터링/디버깅

```
[EventInspector UI]
  - 이벤트 스트림 실시간 뷰 (Mind Studio SSE 확장)
  - 에이전트별 이벤트 처리 현황
  - Projection 상태 스냅샷
  - correlation_id로 이벤트 체인 시각화
```

현재 Mind Studio의 SSE(`/api/events`)를 확장하여 DomainEvent도 브로드캐스트. 프론트엔드에 EventInspector 패널 추가.
