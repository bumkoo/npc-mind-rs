# API Reference

`npc-mind` 라이브러리의 공개 API 레퍼런스입니다.

---

## Service Layer (진입점)

### MindService — 핵심 퍼사드 및 오케스트레이션

도메인 로직을 조율하고 저장소와 연동하는 주 진입점입니다. 포맷팅 없이 도메인 결과(`*Result`)를 반환합니다.

```rust
use npc_mind::application::mind_service::MindService;

// 기본 엔진 사용
let mut service = MindService::new(repo);
```

### 서브 서비스 (Internal/Advanced)

`MindService`의 비대한 책임을 분리하여 전문화된 기능을 제공합니다.

| 서비스 | 용도 | 핵심 역할 |
|--------|------|----------|
| `SituationService` | DTO 변환 | `SituationInput` → `Situation` 도메인 모델 변환 (컨텍스트 조회 포함) |
| `RelationshipService` | 관계 관리 | 대화/비트 종료 후 관계 수치 계산 및 갱신 |
| `SceneService` | 장면 제어 | Scene 상태 관리, Beat 전환 트리거 체크 및 전환 로직 수행 |

### FormattedMindService — 포맷팅 포함 서비스
...

`MindService` + `GuideFormatter` 조합. 모든 응답에 `prompt: String`이 포함됩니다.

```rust
use npc_mind::FormattedMindService;

// 빌트인 언어 (ko, en)
let mut service = FormattedMindService::new(repo, "ko")?;

// 빌트인 + 부분 오버라이드
let mut service = FormattedMindService::with_overrides(repo, "ko", custom_toml)?;

// 완전 커스텀 TOML
let mut service = FormattedMindService::with_custom_locale(repo, full_toml)?;

// GuideFormatter 트레이트 직접 구현
let mut service = FormattedMindService::with_formatter(repo, my_formatter);
```

| 생성 메서드 | 용도 | 반환 타입 |
|------------|------|-----------|
| `MindService::new(repo)` | 도메인 결과 (포맷팅 없음) | `*Result` |
| `FormattedMindService::new(repo, "ko")` | 빌트인 로케일 | `*Response` (`prompt` 포함) |
| `FormattedMindService::with_overrides(repo, "ko", toml)` | 부분 커스터마이징 | `*Response` |
| `FormattedMindService::with_custom_locale(repo, toml)` | 완전 커스텀 | `*Response` |
| `FormattedMindService::with_formatter(repo, formatter)` | 트레이트 직접 구현 | `*Response` |

---

## Service Methods

모든 메서드는 `MindService`와 `FormattedMindService` 양쪽에서 동일하게 사용 가능합니다.

### appraise — 상황 평가

상황을 평가하여 OCC 감정을 생성하고 연기 가이드를 반환합니다.

```rust
pub fn appraise(
    &mut self,
    req: AppraiseRequest,
    before_eval: impl FnMut(),           // 평가 전 콜백
    after_eval: impl FnMut() -> Vec<String>,  // 평가 후 트레이스 수집
) -> Result<AppraiseResult, MindServiceError>
```

### apply_stimulus — 대사 자극 적용

PAD 자극을 적용하여 감정을 갱신합니다. Scene이 있으면 Beat 전환을 자동 판단합니다.

```rust
pub fn apply_stimulus(
    &mut self,
    req: StimulusRequest,
    before_eval: impl FnMut(),
    after_eval: impl FnMut() -> Vec<String>,
) -> Result<StimulusResult, MindServiceError>
```

### start_scene — Scene 시작

Focus 옵션 목록을 등록하고 Initial Focus를 자동 appraise합니다.

```rust
pub fn start_scene(
    &mut self,
    req: SceneRequest,
    before_eval: impl FnMut(),
    after_eval: impl FnMut() -> Vec<String>,
) -> Result<SceneResult, MindServiceError>
```

### scene_info — Scene 상태 조회

```rust
pub fn scene_info(&self) -> SceneInfoResult
```

### load_scene_focuses — 시나리오 복원

저장된 시나리오에서 Scene을 복원합니다.

```rust
pub fn load_scene_focuses(
    &mut self,
    focuses: Vec<SceneFocus>,
    npc_id: String,
    partner_id: String,
    significance: f32,
) -> Result<Option<AppraiseResult>, MindServiceError>
```

### generate_guide — 가이드 재생성

현재 감정 상태에서 연기 가이드를 다시 생성합니다.

```rust
pub fn generate_guide(&self, req: GuideRequest) -> Result<GuideResult, MindServiceError>
```

### after_beat — Beat 종료

관계를 갱신하되 감정은 유지합니다.

```rust
pub fn after_beat(&mut self, req: AfterDialogueRequest) -> Result<AfterDialogueResponse, MindServiceError>
```

### after_dialogue — Scene 종료

관계를 갱신하고 감정과 Scene을 초기화합니다.

```rust
pub fn after_dialogue(&mut self, req: AfterDialogueRequest) -> Result<AfterDialogueResponse, MindServiceError>
```

### 메서드 요약

| 메서드 | 용도 | 입력 | 출력 |
|--------|------|------|------|
| `appraise()` | 상황 평가 → 감정 + 가이드 생성 | `AppraiseRequest` | `AppraiseResult` / `Response` |
| `apply_stimulus()` | PAD 자극 → 감정 갱신 + Beat 전환 | `StimulusRequest` | `StimulusResult` / `Response` |
| `start_scene()` | Scene 시작 + 초기 Focus appraise | `SceneRequest` | `SceneResult` / `Response` |
| `scene_info()` | Scene 상태 조회 | — | `SceneInfoResult` |
| `load_scene_focuses()` | 시나리오 복원 | `Vec<SceneFocus>` | `Option<AppraiseResult>` |
| `generate_guide()` | 현재 감정으로 가이드 재생성 | `GuideRequest` | `GuideResult` / `Response` |
| `after_beat()` | Beat 종료 → 관계 갱신 (감정 유지) | `AfterDialogueRequest` | `AfterDialogueResponse` |
| `after_dialogue()` | Scene 종료 → 관계 갱신 + 감정 초기화 | `AfterDialogueRequest` | `AfterDialogueResponse` |

---

## DTO Types (Request / Response)

### AppraiseRequest

```rust
pub struct AppraiseRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub situation: SituationInput,
}
```

### SituationInput

```rust
pub struct SituationInput {
    pub description: String,
    pub event: Option<EventInput>,    // 하나 이상 필수
    pub action: Option<ActionInput>,
    pub object: Option<ObjectInput>,
}
```

### EventInput

```rust
pub struct EventInput {
    pub description: String,
    pub desirability_for_self: f32,       // -1.0 ~ 1.0
    pub other: Option<EventOtherInput>,   // 타인에 대한 영향
    pub prospect: Option<String>,         // "anticipation", "hope_fulfilled",
                                          // "hope_unfulfilled", "fear_unrealized", "fear_confirmed"
}

pub struct EventOtherInput {
    pub target_id: String,
    pub desirability: f32,   // -1.0 ~ 1.0
}
```

### ActionInput

```rust
pub struct ActionInput {
    pub description: String,
    pub agent_id: Option<String>,   // None = 자기 행동, Some = 타인 행동
    pub praiseworthiness: f32,      // -1.0 (비난) ~ 1.0 (칭찬)
}
```

### ObjectInput

```rust
pub struct ObjectInput {
    pub target_id: String,
    pub appealingness: f32,   // -1.0 (혐오) ~ 1.0 (매력)
}
```

### StimulusRequest

```rust
pub struct StimulusRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub situation_description: Option<String>,
    pub pleasure: f32,     // -1.0 ~ 1.0
    pub arousal: f32,      // -1.0 ~ 1.0
    pub dominance: f32,    // -1.0 ~ 1.0
}
```

### GuideRequest

```rust
pub struct GuideRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub situation_description: Option<String>,
}
```

### AfterDialogueRequest

```rust
pub struct AfterDialogueRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub praiseworthiness: Option<f32>,  // 상대 행동 평가
    pub significance: Option<f32>,       // 0.0 (일상) ~ 1.0 (중대 사건)
}
```

### SceneRequest

```rust
pub struct SceneRequest {
    pub npc_id: String,
    pub partner_id: String,
    pub description: String,
    pub focuses: Vec<SceneFocusInput>,
}
```

### SceneFocusInput

```rust
pub struct SceneFocusInput {
    pub id: String,
    pub description: String,
    pub trigger: Option<Vec<Vec<ConditionInput>>>,  // None = Initial, Some = 조건부
    pub event: Option<EventInput>,
    pub action: Option<ActionInput>,
    pub object: Option<ObjectInput>,
}
```

### ConditionInput

```rust
pub struct ConditionInput {
    pub emotion: String,        // "Joy", "Fear", "Anger" 등
    pub below: Option<f32>,     // 임계값 미만
    pub above: Option<f32>,     // 임계값 초과
    pub absent: Option<bool>,   // 감정 부재
}
```

### CanFormat 트레이트 (Standard Formatting)

도메인 결과 타입(`*Result`)은 `CanFormat` 트레이트를 구현하여 일관된 포맷팅 인터페이스를 제공합니다. `FormattedMindService`는 내부적으로 이 트레이트를 사용하여 결과를 자동으로 변환합니다.

| 도메인 결과 | 응답 타입 (`Response`) | 변환 메서드 |
|------------|-----------------------|-----------|
| `AppraiseResult` | `AppraiseResponse` | `.format(&formatter)` |
| `StimulusResult` | `StimulusResponse` | `.format(&formatter)` |
| `GuideResult` | `GuideResponse` | `.format(&formatter)` |
| `SceneResult` | `SceneResponse` | `.format(&formatter)` |

### Response Types

```rust
pub struct AppraiseResponse {
    pub emotions: Vec<EmotionOutput>,
    pub dominant: Option<EmotionOutput>,
    pub mood: f32,
    pub prompt: String,       // 포맷팅된 LLM 프롬프트
    pub trace: Vec<String>,
}

pub struct StimulusResponse {
    pub emotions: Vec<EmotionOutput>,
    pub dominant: Option<EmotionOutput>,
    pub mood: f32,
    pub prompt: String,
    pub trace: Vec<String>,
    pub beat_changed: bool,              // Beat 전환 여부
    pub active_focus_id: Option<String>, // 현재 활성 Focus
}

pub struct GuideResponse {
    pub prompt: String,
    pub json: String,
}

pub struct AfterDialogueResponse {
    pub before: RelationshipValues,
    pub after: RelationshipValues,
}

pub struct SceneResponse {
    pub focus_count: usize,
    pub initial_appraise: Option<AppraiseResponse>,
    pub active_focus_id: Option<String>,
}

pub struct SceneInfoResult {
    pub has_scene: bool,
    pub npc_id: Option<String>,
    pub partner_id: Option<String>,
    pub active_focus_id: Option<String>,
    pub focuses: Vec<FocusInfoItem>,
}

pub struct EmotionOutput {
    pub emotion_type: String,
    pub intensity: f32,
    pub context: Option<String>,
}
```

---

## Repository Ports

3개의 저장소 트레이트입니다. 기본 제공 `InMemoryRepository`를 사용하거나 직접 구현할 수 있습니다.

### InMemoryRepository — 기본 제공 구현체

```rust
use npc_mind::InMemoryRepository;

// Mind Studio JSON 로드 (권장)
let repo = InMemoryRepository::from_file("data/scenario.json")?;

// JSON 문자열에서 로드
let repo = InMemoryRepository::from_json(json_str)?;

// 프로그래밍 방식
let mut repo = InMemoryRepository::new();
repo.add_npc(npc);
repo.add_relationship(rel);
repo.add_object("sword", "명검");

// 메타데이터 접근자
repo.scenario_name();        // 시나리오 이름
repo.scenario_description(); // 시나리오 설명
repo.turn_history();         // 턴 히스토리
```

### NpcWorld — 게임 세계 데이터

```rust
pub trait NpcWorld {
    fn get_npc(&self, id: &str) -> Option<Npc>;
    fn get_relationship(&self, owner_id: &str, target_id: &str) -> Option<Relationship>;
    fn get_object_description(&self, object_id: &str) -> Option<String>;
    fn save_relationship(&mut self, owner_id: &str, target_id: &str, rel: Relationship);
}
```

### EmotionStore — 감정 상태 관리

```rust
pub trait EmotionStore {
    fn get_emotion_state(&self, npc_id: &str) -> Option<EmotionState>;
    fn save_emotion_state(&mut self, npc_id: &str, state: EmotionState);
    fn clear_emotion_state(&mut self, npc_id: &str);
}
```

### SceneStore — Scene 관리

```rust
pub trait SceneStore {
    fn get_scene(&self) -> Option<Scene>;
    fn save_scene(&mut self, scene: Scene);
    fn clear_scene(&mut self);
}
```

### MindRepository — 통합 super-trait

3개 포트를 모두 구현하면 `MindRepository`가 자동으로 파생됩니다.

```rust
pub trait MindRepository: NpcWorld + EmotionStore + SceneStore {}
impl<T: NpcWorld + EmotionStore + SceneStore> MindRepository for T {}
```

---

## Domain Types

### Npc

```rust
pub struct Npc { /* private fields */ }

impl Npc {
    pub fn new(id: impl Into<String>, name: impl Into<String>,
               description: impl Into<String>, personality: HexacoProfile) -> Self;
    pub fn id(&self) -> &str;
    pub fn name(&self) -> &str;
    pub fn description(&self) -> &str;
    pub fn personality(&self) -> &HexacoProfile;
}
```

### NpcBuilder

```rust
let npc = NpcBuilder::new("id", "name")
    .description("설명")
    .honesty_humility(|h| { h.sincerity = Score::new(0.7, "sincerity").unwrap(); })
    .emotionality(|e| { ... })
    .extraversion(|x| { ... })
    .agreeableness(|a| { ... })
    .conscientiousness(|c| { ... })
    .openness(|o| { ... })
    .build();
```

### Score (성격/관계 점수)

```rust
pub struct Score { /* private */ }

impl Score {
    pub fn new(value: f32, field: &str) -> Result<Self, PersonalityError>;  // 범위 검증
    pub fn clamped(value: f32) -> Self;   // 자동 클램프
    pub fn neutral() -> Self;             // 0.0
    pub fn value(&self) -> f32;
    pub fn intensity(&self) -> f32;       // 절대값
}
```

### HEXACO Profile — 6차원 24 패싯

| 차원 | 패싯 |
|------|------|
| **HonestyHumility** | `sincerity`, `fairness`, `greed_avoidance`, `modesty` |
| **Emotionality** | `fearfulness`, `anxiety`, `dependence`, `sentimentality` |
| **Extraversion** | `social_self_esteem`, `social_boldness`, `sociability`, `liveliness` |
| **Agreeableness** | `forgiveness`, `gentleness`, `flexibility`, `patience` |
| **Conscientiousness** | `organization`, `diligence`, `perfectionism`, `prudence` |
| **Openness** | `aesthetic_appreciation`, `inquisitiveness`, `creativity`, `unconventionality` |

각 패싯은 `Score` 타입이며 범위는 `-1.0 ~ 1.0`입니다.

### Relationship

```rust
pub struct Relationship { /* private fields */ }

impl Relationship {
    pub fn new(owner_id, target_id, closeness: Score, trust: Score, power: Score) -> Self;
    pub fn neutral(owner_id, target_id) -> Self;
    pub fn closeness(&self) -> Score;
    pub fn trust(&self) -> Score;
    pub fn power(&self) -> Score;
}
```

### RelationshipBuilder

```rust
let rel = RelationshipBuilder::new("owner", "target")
    .closeness(Score::clamped(0.6))
    .trust(Score::clamped(0.8))
    .power(Score::clamped(0.0))
    .build();
```

3축 범위: `-1.0 ~ 1.0`
- **closeness**: -1.0 (적대) → 1.0 (친밀)
- **trust**: -1.0 (불신) → 1.0 (신뢰)
- **power**: -1.0 (하위) → 1.0 (상위)

---

## OCC Emotion Types (22종)

| 분류 | 감정 |
|------|------|
| **Event → Self** | Joy, Distress |
| **Event → Prospect** | Hope, Fear, Satisfaction, Disappointment, Relief, FearsConfirmed |
| **Event → Other** | HappyFor, Pity, Gloating, Resentment |
| **Action → Self** | Pride, Shame |
| **Action → Other** | Admiration, Reproach |
| **Action → Compound** | Gratification (Pride+Joy), Remorse (Shame+Distress), Gratitude (Admiration+Joy), Anger (Reproach+Distress) |
| **Object** | Love, Hate |

---

## Acting Guide Output

`ActingGuide`는 감정 + 성격 → LLM 연기 지시를 캡슐화합니다.

### ActingGuide 필드

| 필드 | 타입 | 설명 |
|------|------|------|
| `npc_name` | `String` | NPC 이름 |
| `npc_description` | `String` | NPC 설명 |
| `personality` | `PersonalitySnapshot` | 성격 특성 + 말투 |
| `emotion` | `EmotionSnapshot` | 현재 감정 (dominant, 목록, mood) |
| `situation_description` | `Option<String>` | 상황 설명 |
| `directive` | `ActingDirective` | 연기 지시 (Tone + Attitude + Behavior + Restriction) |
| `relationship` | `Option<RelationshipSnapshot>` | 관계 수준 |

### ActingDirective 구성요소

#### Tone (18종)

`SuppressedCold`, `RoughAggressive`, `AnxiousTrembling`, `SomberRestrained`, `BrightLively`, `VigilantCalm`, `TenseAnxious`, `ShrinkingSmall`, `QuietConfidence`, `ProudArrogant`, `CynicalCritical`, `DeepSighing`, `SincerelyWarm`, `JealousBitter`, `CompassionateSoft`, `RelaxedGentle`, `Heavy`, `Calm`

#### Attitude (7종)

`HostileAggressive`, `SuppressedDiscomfort`, `Judgmental`, `GuardedDefensive`, `FriendlyOpen`, `DefensiveClosed`, `NeutralObservant`

#### BehavioralTendency (8종)

`ImmediateConfrontation`, `StrategicResponse`, `ExpressAndObserve`, `BraveConfrontation`, `SeekSafety`, `AvoidOrDeflect`, `ActiveCooperation`, `ObserveAndRespond`

#### Restriction (5종)

`NoHumorOrLightTone`, `NoFriendliness`, `NoSelfJustification`, `NoBravado`, `NoLyingOrExaggeration`

### PersonalitySnapshot

성격 기반으로 추출된 특성과 말투 스타일:

**PersonalityTrait (12종)**: `HonestAndModest` (H+), `CunningAndAmbitious` (H-), `EmotionalAndAnxious` (E+), `BoldAndIndependent` (E-), `ConfidentAndSociable` (X+), `IntrovertedAndQuiet` (X-), `TolerantAndGentle` (A+), `GrudgingAndCritical` (A-), `SystematicAndDiligent` (C+), `FreeAndImpulsive` (C-), `CuriousAndCreative` (O+), `TraditionalAndConservative` (O-)

**SpeechStyle (12종)**: `FrankAndUnadorned` (H+), `HidesInnerThoughts` (H-), `ExpressiveAndWorried` (E+), `CalmAndComposed` (E-), `ActiveAndForceful` (X+), `BriefAndConcise` (X-), `SoftAndConsiderate` (A+), `SharpAndDirect` (A-), `LogicalAndRational` (C+), `UnfilteredAndSpontaneous` (C-), `MetaphoricalAndUnique` (O+), `FormalAndTraditional` (O-)

### RelationshipSnapshot

| 필드 | 타입 | 설명 |
|------|------|------|
| `target_name` | `String` | 대상 이름 |
| `closeness_level` | `RelationshipLevel` | VeryHigh / High / Neutral / Low / VeryLow |
| `trust_level` | `RelationshipLevel` | VeryHigh / High / Neutral / Low / VeryLow |
| `power_level` | `PowerLevel` | VeryHigh / High / Neutral / Low / VeryLow |

---

## Error Types

```rust
pub enum MindServiceError {
    NpcNotFound(String),
    RelationshipNotFound(String, String),
    InvalidSituation(String),
    EmotionStateNotFound,
    LocaleError(String),
}
```

---

## Optional Ports (확장용)

| 포트 | 용도 | 기본 구현체 |
|------|------|------------|
| `Appraiser` | 감정 평가 엔진 | `AppraisalEngine` |
| `StimulusProcessor` | 자극 처리 엔진 | `StimulusEngine` |
| `GuideFormatter` | 가이드 포맷팅 | `LocaleFormatter` |
| `PadAnchorSource` | PAD 앵커 텍스트 로드 | `FileAnchorSource` (JSON/TOML 통합) |
| `TextEmbedder` | 텍스트 임베딩 | `OrtEmbedder` (embed feature) |
| `UtteranceAnalyzer` | 대사 → PAD 변환 | `PadAnalyzer` |
| `ConversationPort` | LLM 다턴 대화 세션 (chat feature) | `RigChatAdapter` |
| `LlmInfoProvider` | LLM 모델 메타데이터 (chat feature) | `RigChatAdapter` |
| `LlmModelDetector` | 모델 런타임 재감지 (chat feature) | `RigChatAdapter` |
| `LlamaServerMonitor` | llama-server health/slots/metrics (chat feature) | `RigChatAdapter` |
| `MemoryStore` | 기억 저장/검색 (RAG) | `SqliteMemoryStore` (embed feature) |
| `RumorStore` | Rumor 애그리거트 저장/검색 (Step C1~) | `SqliteRumorStore` (embed feature). 테스트 전용 `InMemoryRumorStore`. |

---

## Memory API (Step A~D)

`docs/memory/03-implementation-design.md`의 Step A·B·C·D가 구현된 상태의 공개 API.
**Step D (Consolidation & World Overlay)** 포함: `Command::ApplyWorldEvent`,
`WorldOverlayAgent`/`WorldOverlayHandler`, `SceneConsolidationHandler`,
`RelationshipMemoryHandler`.

### MemoryEntry — 기억 항목

```rust
pub struct MemoryEntry {
    // 식별·Event Sourcing
    pub id: String,                          // "mem-000001" 포맷
    pub created_seq: u64,                    // EventStore append 순번 (I-ME-10)
    pub event_id: u64,                       // 생성 트리거 이벤트 id

    // 분류 VO
    pub scope: MemoryScope,                  // 생성 후 불변
    pub source: MemorySource,                // 생성 후 불변
    pub provenance: Provenance,              // Seeded | Runtime
    pub memory_type: MemoryType,
    pub layer: MemoryLayer,                  // A(구체) | B(요약) 한 방향 전이

    // 내용
    pub content: String,
    pub topic: Option<String>,               // 논리 Topic 식별자
    pub emotional_context: Option<(f32,f32,f32)>,  // PAD

    // 시간
    pub timestamp_ms: u64,
    pub last_recalled_at: Option<u64>,
    pub recall_count: u32,

    // Source 메타
    pub origin_chain: Vec<String>,           // 전달 체인
    pub confidence: f32,                     // [0,1] — 생성 시 1회 계산, 불변
    pub acquired_by: Option<String>,         // Faction/Family 공용 기억 획득자

    // 관계
    pub superseded_by: Option<String>,
    pub consolidated_into: Option<String>,

    // 레거시 (deprecated, scope.owner_a() 투영으로 grand-father)
    #[deprecated(note = "Use entry.scope")]
    pub npc_id: String,
}

impl MemoryEntry {
    /// Personal Scope용 호환 생성자 — 신규 필드 기본값 자동 채움.
    pub fn personal(id, npc_id, content, emotional_context, timestamp_ms, event_id, memory_type) -> Self;

    /// `npc_id` 읽기 전용 접근자 — 비-Personal Scope에서는 `scope.owner_a()` 반환.
    pub fn legacy_npc_id(&self) -> &str;
}
```

### MemoryScope — 소유·접근 범위

```rust
pub enum MemoryScope {
    Personal { npc_id: String },
    Relationship { a: String, b: String },  // a ≤ b 정규화
    Faction { faction_id: String },
    Family { family_id: String },
    World { world_id: String },
}

impl MemoryScope {
    pub fn relationship(x, y) -> Self;         // 대칭 정규화 생성자
    pub fn kind(&self) -> &'static str;        // SQLite scope_kind 컬럼
    pub fn owner_a(&self) -> &str;
    pub fn owner_b(&self) -> Option<&str>;
    pub fn partition_key(&self) -> String;     // "personal:<id>" 등 — ID에 `:` 금지
}
```

### MemorySource / Provenance / MemoryLayer

```rust
pub enum MemorySource { Experienced, Witnessed, Heard, Rumor }
pub enum Provenance { Seeded, Runtime }  // Canonical = Seeded ∧ World (τ=∞)
pub enum MemoryLayer { A, B }

impl MemorySource {
    pub fn weight(self) -> f32;    // Ranker 2단계 (1.00/0.85/0.60/0.35)
    pub fn priority(self) -> u8;   // 0..=3 (0=Experienced, 3=Rumor)
    pub fn from_origin_chain(chain_len: usize, hint: Option<Self>) -> Self;
}
```

### MemoryType — 기억 유형 (serde alias로 구 JSON 역호환)

```rust
pub enum MemoryType {
    DialogueTurn,        // alias "Dialogue"
    RelationshipChange,  // alias "Relationship"
    BeatTransition,
    SceneSummary,        // alias "SceneEnd"
    GameEvent,
    WorldEvent,          // Step D 이후
    FactionKnowledge,    // Step C 이후
    FamilyFact,          // Step C 이후
}
```

### MemoryStore — 저장/검색 포트

```rust
pub trait MemoryStore: Send + Sync {
    // `index` · `count` — 계속 권장 경로
    fn index(&self, entry: MemoryEntry, embedding: Option<Vec<f32>>) -> Result<(), MemoryError>;
    fn count(&self) -> usize;

    // Step B에서 #[deprecated(since="0.4.0")] 마킹 — 신규 코드는 `search(MemoryQuery)` 사용.
    #[deprecated(since = "0.4.0", note = "Use MemoryStore::search(MemoryQuery { embedding: Some(..), .. })")]
    fn search_by_meaning(&self, query: &[f32], npc_id: Option<&str>, limit: usize) -> Result<Vec<MemoryResult>, MemoryError>;
    #[deprecated(since = "0.4.0", note = "Use MemoryStore::search(MemoryQuery { text: Some(..), .. })")]
    fn search_by_keyword(&self, kw: &str, npc_id: Option<&str>, limit: usize) -> Result<Vec<MemoryResult>, MemoryError>;
    #[deprecated(since = "0.4.0", note = "Use MemoryStore::search(MemoryQuery { scope_filter: Some(NpcAllowed(..)), .. })")]
    fn get_recent(&self, npc_id: &str, limit: usize) -> Result<Vec<MemoryEntry>, MemoryError>;

    // Step A 신규
    fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryResult>, MemoryError>;
    fn get_by_id(&self, id: &str) -> Result<Option<MemoryEntry>, MemoryError>;
    fn get_by_topic_latest(&self, topic: &str) -> Result<Option<MemoryEntry>, MemoryError>;
    fn get_canonical_by_topic(&self, topic: &str) -> Result<Option<MemoryEntry>, MemoryError>;
    fn mark_superseded(&self, old_id: &str, new_id: &str) -> Result<(), MemoryError>;
    fn mark_consolidated(&self, a_ids: &[String], b_id: &str) -> Result<(), MemoryError>;
    fn record_recall(&self, id: &str, now_ms: u64) -> Result<(), MemoryError>;
}

pub struct MemoryQuery {
    pub text: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub scope_filter: Option<MemoryScopeFilter>,
    pub source_filter: Option<Vec<MemorySource>>,
    pub layer_filter: Option<MemoryLayer>,
    pub topic: Option<String>,
    pub exclude_superseded: bool,
    pub exclude_consolidated_source: bool,
    pub min_retention: Option<f32>,
    pub current_pad: Option<(f32, f32, f32)>,
    pub limit: usize,
}

pub enum MemoryScopeFilter {
    Any,
    Exact(MemoryScope),
    /// Personal(해당 NPC) + World + Relationship(참여). Faction/Family Join은 Step C 예정.
    NpcAllowed(String),
}
```

### MemoryRanker — 2단계 랭커 (도메인 순수 함수)

```rust
use npc_mind::domain::memory::ranker::{MemoryRanker, DecayTauTable, Candidate, RankQuery, RankedEntry};

let tau = DecayTauTable::default_table();
let ranker = MemoryRanker::new(&tau);
let ranked: Vec<RankedEntry> = ranker.rank(candidates, &query, now_ms);
// 1단계: Topic/클러스터별 min(source.priority()) 필터
// 2단계: vec_similarity × retention × source_confidence × emotion_proximity × temporal_recency
```

Step B에서 `DialogueAgent::with_memory(store, framer)`가 활성화되면
`inject_memory_push`가 위 Ranker를 호출해 시스템 프롬프트 prepend용 블록을 만든다 (아래 참조).

### MemoryFramer — 기억 엔트리 → 프롬프트 블록 (Step B)

```rust
pub trait MemoryFramer: Send + Sync {
    /// 단일 엔트리를 source별 라벨로 포맷 (예: "[겪음] content").
    fn frame(&self, entry: &MemoryEntry, locale: &str) -> String;
    /// header/footer + 엔트리 줄바꿈 결합. 빈 slice → 빈 문자열.
    fn frame_block(&self, entries: &[MemoryEntry], locale: &str) -> String;
}
```

**기본 구현 `LocaleMemoryFramer`** (`presentation/memory_formatter.rs`):
- `LocaleMemoryFramer::new()` — 빌트인 `[memory.framing]` ko/en 자동 로드.
- `with_locale_toml(locale, toml_str)` — 외부 TOML 추가.
- `with_default_locale(locale)` — fallback locale 변경 (기본 `"ko"`).

**Locale TOML 스키마** (`locales/ko.toml`):
```toml
[memory.framing]
experienced = "[겪음] {content}"
witnessed   = "[목격] {content}"
heard       = "[전해 들음] {content}"
rumor       = "[강호에 떠도는 소문] {content}"

[memory.framing.block]
header = "\n# 떠오르는 기억\n"
footer = "\n"
```

영어 locale은 `[Experienced] / [Witnessed] / [Heard] / [Rumor]` + `# Recollections` 헤더.

### DialogueAgent::with_memory — 프롬프트 주입 활성화 (Step B, [chat feature])

```rust
use std::sync::Arc;
use npc_mind::ports::{MemoryFramer, MemoryStore};
use npc_mind::presentation::memory_formatter::LocaleMemoryFramer;

let store: Arc<dyn MemoryStore> = ...;
let framer: Arc<dyn MemoryFramer> = Arc::new(LocaleMemoryFramer::new());

let agent = DialogueAgent::new(dispatcher, chat, formatter)
    .with_memory(store, framer)              // Opt-in
    .with_memory_locale("ko");               // 기본 "ko"
```

활성화 시 동작 (미부착 시 모든 훅 no-op):
- `start_session` 1회: `situation.description`(없으면 `partner_id`)을 쿼리로 검색·랭킹·포맷,
  appraise 프롬프트 앞에 prepend.
- `BeatTransitioned` 발생 시: user utterance + listener-converted PAD를 쿼리로 재구성,
  `update_system_prompt` 직전에 prepend.

**검색 설정** (코드 고정):
- `MemoryScopeFilter::NpcAllowed(npc_id)` (Personal + World + Relationship 참여).
- `exclude_superseded: true`, `exclude_consolidated_source: true`.
- `min_retention: MEMORY_RETENTION_CUTOFF (0.10)`.
- 검색 limit `MEMORY_PUSH_TOP_K * 3`로 oversample → Ranker가 `MEMORY_PUSH_TOP_K=5`로 컷.
- 결과 엔트리에 `record_recall(id, now_ms)` 호출 (best-effort, 실패는 debug 로그만).

### RelationshipUpdated 이벤트 — `cause` 필드 (A8 hook, Step D 활성)

```rust
EventPayload::RelationshipUpdated {
    // 기존 8 필드 …
    cause: RelationshipChangeCause,
}

pub enum RelationshipChangeCause {
    SceneInteraction { scene_id: SceneId },       // Step D: BeatTransitioned 경로에서 설정
    InformationTold { origin_chain: Vec<String> },// (Step F 예정 — 자동 채움)
    WorldEventOverlay { topic: Option<String> },  // (Step F 예정)
    Rumor { rumor_id: String },                   // (Step F 예정)
    Unspecified,                                  // DialogueEnd/UpdateRelationship 기본값
}
```

**Step D 현재 설정 지점**:
- `RelationshipAgent.handle_relationship_update_with_cause` — `BeatTransitioned` 경로에서
  cause=`SceneInteraction { scene_id: SceneId::new(npc, partner) }` 자동 설정.
- `DialogueEndRequested`/`RelationshipUpdateRequested` 경로는 여전히 `Unspecified`
  (TODO step-f: DialogueEnd는 scene_id 필드 추가 후 SceneInteraction 승격 예정).

`RelationshipMemoryHandler` (Step D, Inline)가 이 cause를 읽어 variant별로 source/topic/
content/origin_chain을 분기한 `MemoryEntry(RelationshipChange)`를 생성한다 (§Step D API 참조).

---

## Step C — Telling & Rumor API

Step C1/C2/C3 완료 후 공개된 커맨드·도메인·포트.

### Commands

```rust
// Step C2 — Mind 컨텍스트
Command::TellInformation(TellInformationRequest)

pub struct TellInformationRequest {
    pub speaker: String,
    pub listeners: Vec<String>,       // Direct 청자
    #[serde(default)]
    pub overhearers: Vec<String>,     // Overhearer 청자 (엿들은 자)
    pub claim: String,                 // 전달 본문
    pub stated_confidence: f32,        // [0, 1]. dispatcher에서 clamp
    #[serde(default)]
    pub origin_chain_in: Vec<String>,  // 화자가 상속받은 체인. 청자 chain은 [speaker, ...inherited]
    #[serde(default)]
    pub topic: Option<String>,         // Canonical 연결 키
}
```

→ `InformationAgent`가 listener + overhearer 각자에게 `InformationTold` follow-up 발행
(listeners ∩ overhearers 중복은 Direct 우선으로 dedup).
→ `TellingIngestionHandler` (Inline)가 각 청자의 `MemoryEntry(Personal, source=Heard/Rumor)`
생성. `confidence = stated × (trust.value()+1)/2`, 관계 부재 시 0.5. `origin_chain` 길이가
1이면 Heard, 2+면 Rumor (자동 분류).

```rust
// Step C3 — Memory 컨텍스트
Command::SeedRumor(SeedRumorRequest)
Command::SpreadRumor(SpreadRumorRequest)

pub struct SeedRumorRequest {
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub seed_content: Option<String>,  // 고아 Rumor(topic=None)이면 필수
    pub reach: RumorReachInput,
    pub origin: RumorOriginInput,
}

pub struct SpreadRumorRequest {
    pub rumor_id: String,
    pub recipients: Vec<String>,
    #[serde(default)]
    pub content_version: Option<String>,  // Distortion id 참조 (원본이면 None)
}

pub struct RumorReachInput {
    pub regions: Vec<String>,
    pub factions: Vec<String>,
    pub npc_ids: Vec<String>,
    pub min_significance: f32,  // rumor가 가치 있다고 볼 최소 중요도
}

#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RumorOriginInput {
    Seeded,
    FromWorldEvent { event_id: u64 },
    Authored { #[serde(default)] by: Option<String> },
}
```

→ `RumorAgent`가 `Rumor` 애그리거트 생성/갱신 + `RumorStore.save` + `RumorSeeded`/`RumorSpread`
follow-up. Seed의 커맨드별 고유 aggregate는 `SeedRumorRequested.pending_id` +
dispatcher `command_seq` 조합 (여러 고아 Rumor가 같은 버킷 공유 방지).
→ `RumorDistributionHandler` (Inline)가 `RumorSpread` 수신자에게 `MemoryEntry(Rumor)` 생성.
Confidence는 `RUMOR_HOP_CONFIDENCE_DECAY(0.8)^hop_index × RUMOR_MIN_CONFIDENCE(0.1) 하한`.
콘텐츠 해소 3-tier: Distortion → `MemoryStore::get_canonical_by_topic` → `seed_content`.

### Rumor 도메인 (`src/domain/rumor.rs`)

```rust
pub struct Rumor {
    pub id: String,
    pub topic: Option<String>,
    pub seed_content: Option<String>,
    pub origin: RumorOrigin,
    pub reach_policy: ReachPolicy,
    pub created_at: u64,
    // hops / distortions / status는 read-only accessor로만 노출
}

impl Rumor {
    pub fn new(id, topic, origin, reach, created_at) -> Self;                      // 일반 소문
    pub fn with_forecast_content(id, topic, seed, origin, reach, at) -> Self;      // 예보된 사실
    pub fn orphan(id, seed, origin, reach, created_at) -> Self;                    // 고아 Rumor
    pub fn add_hop(&mut self, hop: RumorHop) -> Result<(), RumorError>;
    pub fn add_distortion(&mut self, d: RumorDistortion) -> Result<(), RumorError>;
    pub fn transition_to(&mut self, status: RumorStatus) -> Result<(), RumorError>;
    pub fn validate(&self) -> Result<(), RumorError>;  // 불변식 I-RU-1~6
    pub(crate) fn from_parts(...) -> Result<Self, RumorError>;  // 저장소 로드 전용
}

pub enum RumorStatus { Active, Fading, Faded }
pub enum RumorOrigin { Seeded, FromWorldEvent { event_id }, Authored { by } }
```

### RumorStore 포트

```rust
pub trait RumorStore: Send + Sync {
    fn save(&self, rumor: &Rumor) -> Result<(), MemoryError>;
    fn load(&self, id: &str) -> Result<Option<Rumor>, MemoryError>;
    fn find_by_topic(&self, topic: &str) -> Result<Vec<Rumor>, MemoryError>;
    fn find_active_in_reach(&self, reach: &ReachPolicy) -> Result<Vec<Rumor>, MemoryError>;
    /// 저장된 모든 rumor 목록 (Active/Fading/Faded 전부 포함, Sqlite는 `created_at DESC`).
    /// Mind Studio `GET /api/rumors` 전용 (Step E1).
    fn list_all(&self) -> Result<Vec<Rumor>, MemoryError>;
}
```

**기본 구현**: `SqliteRumorStore` (embed feature). `rumors` / `rumor_hops` /
`rumor_distortions` 3 테이블, `rumor_distortions`는 `PRIMARY KEY (rumor_id, id)`
composite (schema v3).

**테스트 전용**: `tests/common/in_memory_rumor.rs::InMemoryRumorStore` — `find_active_in_reach`에
`status IN (Active, Fading)` 필터 적용.

### CommandDispatcher 빌더 API

```rust
let dispatcher = CommandDispatcher::new(repo, event_store, event_bus)
    .with_default_handlers()                                          // 7 Agent + 3 Projection (Step D: +WorldOverlayAgent)
    .with_memory(memory_store.clone())                                // lean: TellingIngestionHandler만 (Step C2 호환)
    // or: .with_memory_full(memory_store.clone())                    // Step D 번들: Telling + WorldOverlay + RelationshipMemory + SceneConsolidation
    .with_rumor(memory_store.clone(), rumor_store.clone());           // + RumorAgent + RumorDistributionHandler (Step C3)
```

- **`with_memory(Arc<dyn MemoryStore>)`** — **lean** (Step C2 호환, 리뷰 H5).
  `TellingIngestionHandler` 1개만 등록. 기존 C2 콜러의 silent behavior break 방지용.
- **`with_memory_full(Arc<dyn MemoryStore>)`** — **Step D 전체 번들**. 4개 Inline
  핸들러를 일괄 등록: `TellingIngestionHandler` (C2) + `WorldOverlayHandler` (D) +
  `RelationshipMemoryHandler` (D) + `SceneConsolidationHandler` (D).
- **`with_rumor(Arc<dyn MemoryStore>, Arc<dyn RumorStore>)`** — `RumorAgent` (Transactional,
  priority 40) + `RumorDistributionHandler` (Inline) 일괄 등록.
- 모든 빌더는 생략 가능 — 이벤트만 발행되고 실제 저장은 외부 구독자 책임.

### Transactional priority (§6.5 확장, Step D 반영)

| 상수 | 값 | 역할 |
|---|---|---|
| `SCENE_START` | 5 | Scene 시작 |
| `EMOTION_APPRAISAL` | 10 | 감정 평가 |
| `STIMULUS_APPLICATION` | 15 | 자극 적용 |
| `GUIDE_GENERATION` | 20 | 가이드 생성 |
| **`WORLD_OVERLAY`** | **25** | **세계 오버레이 팬아웃 (Step D)** |
| `RELATIONSHIP_UPDATE` | 30 | 관계 갱신 |
| **`INFORMATION_TELLING`** | **35** | **정보 전달 팬아웃 (Step C2)** |
| **`RUMOR_SPREAD`** | **40** | **소문 확산 (Step C3)** |
| `AUDIT` | 90 | 감사 로그 |

### Inline priority (Step D 반영)

| 상수 | 값 | 역할 |
|---|---|---|
| `EMOTION_PROJECTION` | 10 | Emotion/Scene/Relationship 프로젝션 |
| `RELATIONSHIP_PROJECTION` | 20 | 관계 프로젝션 |
| `SCENE_PROJECTION` | 30 | Scene 프로젝션 |
| `MEMORY_INGESTION` | 40 | TellingIngestion / RumorDistribution (C2/C3) |
| **`WORLD_OVERLAY_INGESTION`** | **45** | **WorldOverlayHandler (Step D) — Canonical + supersede** |
| **`RELATIONSHIP_MEMORY`** | **50** | **RelationshipMemoryHandler (Step D) — cause 분기** |
| **`SCENE_CONSOLIDATION`** | **60** | **SceneConsolidationHandler (Step D) — Layer A→B 흡수** |

### 원자성 경계 (§14 재정의)

Step C3의 `SpreadRumor` 커맨드는 **Rumor aggregate + `RumorSpread` 이벤트까지만** 원자적으로
commit한다. 수신자별 `MemoryEntry` 쓰기는 `RumorDistributionHandler`가 **Inline
best-effort**로 수행하므로, `MemoryStore.index` 실패는 `tracing::warn!`만 남고 커맨드는
성공 마무리된다. 완전한 cross-store 원자성은 분산 트랜잭션 없이 불가능 → Step F 재시도
큐/sidecar로 해소 예정. I-RU-5는 "aggregate 일관성" 수준.

---

## Step D — Consolidation & World Overlay API

Step D 완료 후 공개된 커맨드·이벤트·핸들러.

### Commands

```rust
// Step D — Mind 컨텍스트
Command::ApplyWorldEvent(ApplyWorldEventRequest)

pub struct ApplyWorldEventRequest {
    pub world_id: String,           // AggregateKey::World 라우팅
    #[serde(default)]
    pub topic: Option<String>,      // Canonical 연결 키. None이면 supersede 없음
    pub fact: String,               // 세계 사실 본문 (MemoryEntry.content)
    #[serde(default = "default_world_significance")] // 기본 0.5
    pub significance: f32,          // [0, 1]. dispatcher에서 clamp
    #[serde(default)]
    pub witnesses: Vec<String>,     // 목격자 목록 (Step F 소비 예정)
}
```

빈 `world_id` 또는 `fact.trim().is_empty()`면 `DispatchV2Error::InvalidSituation`로
조기 reject.

### 이벤트

```rust
// 초기 이벤트 (dispatcher가 발행)
EventPayload::ApplyWorldEventRequested {
    world_id: String,
    topic: Option<String>,
    fact: String,
    significance: f32,
    witnesses: Vec<String>,
}

// follow-up (WorldOverlayAgent가 발행)
EventPayload::WorldEventOccurred {
    world_id: String,
    topic: Option<String>,
    fact: String,                   // §3.1 원래 이름 "updated_fact"
    significance: f32,
    witnesses: Vec<String>,
}
```

두 이벤트 모두 `AggregateKey::World(world_id)`로 라우팅.

### Agents · Handlers

| 파일 | 이름 | Mode | Priority | 역할 |
|---|---|---|---|---|
| `agents/world_overlay_agent.rs` | `WorldOverlayAgent` | Transactional | 25 (WORLD_OVERLAY) | ApplyWorldEventRequested → WorldEventOccurred 1:1 |
| `world_overlay_handler.rs` | `WorldOverlayHandler` | Inline | 45 (WORLD_OVERLAY_INGESTION) | Canonical `MemoryEntry(World, Seeded)` 생성 + 같은 topic **Canonical 1건** supersede |
| `relationship_memory_handler.rs` | `RelationshipMemoryHandler` | Inline | 50 (RELATIONSHIP_MEMORY) | RelationshipUpdated.cause variant별 source/topic/content 분기 |
| `scene_consolidation_handler.rs` | `SceneConsolidationHandler` | Inline | 60 (SCENE_CONSOLIDATION) | SceneEnded → 참여 NPC별 Personal SceneSummary + Layer A `consolidated_into` 마킹 |

### WorldOverlay supersede 정책 (리뷰 B1)

`topic` 있는 `ApplyWorldEvent`는 **`get_canonical_by_topic(topic)` 단건만** supersede.
개별 NPC의 Personal `Heard`/`Rumor` 엔트리(주관 기억)는 보존 — 엔진이 "사실 변경"을
이유로 사용자 기억을 파괴하지 않는다. 각 NPC는 다음 상호작용에서 자연스럽게 갱신된 사실을
학습한다.

### SceneConsolidation 관점 분리 (리뷰 B3)

`SceneEnded` 수신 시 **참여 NPC별**로 1개씩 Personal Scope SceneSummary를 만든다.
각 summary는 **그 NPC 관점의 Layer A만** 흡수:

- `alice` summary: `scope=Personal{alice}`, `alice`의 Personal + `alice↔bob` Relationship의
  Layer A만 `consolidated_into = alice_summary_id` 마킹.
- `bob` summary: 동일하되 `bob` 관점.
- `topic = Some("scene:{a}:{b}")` (a ≤ b 정규화) — 후속 `get_by_topic_latest` 조회 편의.
- 한 쪽 NPC만 Layer A가 있으면 그쪽 summary만 생성.
- 대상 타입: `DialogueTurn`/`BeatTransition`만 (RelationshipChange/WorldEvent 제외).
- 휴리스틱 요약: `{count}턴 간 대화 요약: {첫} ... {끝}` (120자 cap, UTF-8 safe).

### RelationshipMemoryHandler cause 분기 (§8.3)

| cause | source | topic | content 포맷 | origin_chain |
|---|---|---|---|---|
| `SceneInteraction { scene_id }` | Experienced | None | "장면에서 {target}과(와)의 관계 변화 [{axis} Δ={value}]" | `[]` |
| `InformationTold { origin_chain }` | Heard(len=1) or Rumor(len≥2, len=0) | None | "정보 전달로 {target} 관련 감정 변화 [{axis} Δ={value}]" | 입력 체인 계승 |
| `WorldEventOverlay { topic }` | Experienced | topic 계승 | "세계 사건({topic})으로 {target} 관련 변화 [{axis} Δ={value}]" | `[]` |
| `Rumor { rumor_id }` | Rumor | None | "소문({rumor_id}) 여파로 {target} 관련 변화 [{axis} Δ={value}]" | `[rumor:{rumor_id}]` |
| `Unspecified` | Experienced | None | "{target}과(와)의 관계 변화 [{axis} Δ={value}]" | `[]` |

**주도 축 라벨** (리뷰 H4): `closeness`/`trust`/`power` 중 |Δ|가 가장 큰 축 이름과 값을
content 끝에 `[{axis} Δ={value:.2}]` 포맷으로 붙인다. 동률이면 closeness → trust → power 선점.

**threshold**: `MEMORY_RELATIONSHIP_DELTA_THRESHOLD = 0.05` 미만 Δ는 skip.

**관점**: owner → target 관점 엔트리만 생성 (Step F 확장 예정 — target 관점 자동 미러는
도메인 판단 필요).

## Step E1 — Mind Studio Memory/Rumor REST 엔드포인트

`embed` feature 활성 시에만 등록된다. `AppState::new()`에서 `shared_dispatcher`에
`with_memory_full` + `with_rumor`를 자동 부착하고, `NPC_MIND_MEMORY_DB` 환경변수로
DB 파일 경로 지정 (미설정 시 in-memory SQLite).

### Memory 엔드포인트 (`src/bin/mind-studio/handlers/memory.rs`)

| 메서드·경로 | 요청 | 응답 |
|---|---|---|
| `GET /api/memory/search?npc=&topic=&layer=&source=&limit=&q=` | 쿼리 파라미터만 | `{ entries: Vec<MemoryEntry> }` |
| `GET /api/memory/by-npc/{id}?limit=&layer=` | path `id` + 옵션 쿼리 | `{ entries: Vec<MemoryEntry> }` |
| `GET /api/memory/by-topic/{topic}?limit=` | path `topic` + 옵션 쿼리 (기본 50) | `{ entries: Vec<MemoryEntry> }` — supersede 이력 전체 포함 |
| `GET /api/memory/canonical/{topic}` | path `topic` | `{ entry: Option<MemoryEntry> }` |
| `POST /api/memory/entries` | `MemoryEntry` JSON | 201 CREATED (embedding 생성 안 함 — 메타 저장만) |
| `POST /api/memory/tell` | `TellInformationRequest` | `{ listeners_informed: usize }` |

- `search`의 `source` 파라미터는 CSV (`experienced,witnessed,heard,rumor`). `layer`는 `A`/`B`.
- `search`의 `q` 파라미터는 현재 무시 (semantic 검색 미통합 — Step B 후속 과제).
- `by-topic`은 `exclude_superseded=false` — Topic 역사 완전 공개용.

### World 엔드포인트 (`handlers/world.rs`)

| 메서드·경로 | 요청 | 응답 |
|---|---|---|
| `POST /api/world/apply-event` | `ApplyWorldEventRequest` | `{ applied: bool }` |

실제 Canonical `MemoryEntry` 생성은 Inline `WorldOverlayHandler`가 담당.
SSE `MemorySuperseded`는 dispatch 전에 `get_canonical_by_topic`으로 기존 존재를 확인한 뒤
실제 supersede가 일어났을 때만 방출(리뷰 대응 M1).

### Rumor 엔드포인트 (`handlers/rumor.rs`)

| 메서드·경로 | 요청 | 응답 |
|---|---|---|
| `GET /api/rumors` | 없음 | `{ rumors: Vec<Rumor> }` — Active/Fading/Faded 전부 |
| `POST /api/rumors/seed` | `SeedRumorRequest` | `{ rumor_id: String }` |
| `POST /api/rumors/{id}/spread` | path `id` + `{ recipients, content_version? }` | `{ hop_index: u32, recipient_count: usize }` |

`/api/rumors/{id}/spread`의 body에는 `rumor_id`가 없다(path가 담음). 내부적으로
`SpreadRumorRequest`를 재구성.

### SSE StateEvent (Step E1 신규 5종)

`GET /api/events` SSE 스트림에 다음 이벤트명이 추가된다(`snake_case` 이름, 페이로드 없음 —
프런트엔드는 이름만 받고 해당 엔드포인트를 refetch):

- `memory_created` — Tell/ApplyWorldEvent/SpreadRumor/manual entry 성공 시
- `memory_superseded` — ApplyWorldEvent이 기존 Canonical을 대체했을 때
- `memory_consolidated` — (현재 방출 지점 없음 — Step F 이벤트 팬아웃에서 연결)
- `rumor_seeded` — SeedRumor 성공 시
- `rumor_spread` — SpreadRumor 성공 시

### 저장소 구성 (환경변수)

```
NPC_MIND_MEMORY_DB=/path/to/studio.db   # 파일 SQLite (영속)
# 미설정 시 SqliteMemoryStore::in_memory() + SqliteRumorStore::in_memory() 사용
```

두 store가 같은 DB 파일을 공유한다. `SqliteMemoryStore::init_schema`가 rumor 테이블을
선제 생성하고 `SqliteRumorStore::init_schema`도 `IF NOT EXISTS`로 무충돌(§7.4).

### 범위 외 (E1 기준)

- 프런트엔드 Memory/Rumor UI → Step E2 (완료, `aeb005c`)
- Topic 히스토리 + 런타임 소문 편집 GUI → Step E3.1 (완료, `84b2510`)
- 시나리오 JSON `initial_rumors`/`world_knowledge` seeding → Step E3.2 (완료, `8ff0829`)
- 시드 조회 패널 + 로드 경고 가시화 → Step E3.3 (완료, `fcf50ec`)
- `MemoryEntryCreated/Superseded/Consolidated` 이벤트 EventStore 팬아웃 → Step F
- Semantic `q` 검색 (SqliteMemoryStore::search의 vec0 통합) → Step B 후속
- `director_v2` 경로에 memory/rumor 배선 (E1은 shared_dispatcher만)

## Step E3.2/E3.3 — 시나리오 JSON seeding + 조회 API

시나리오 작가가 JSON 파일에 `initial_rumors` / `world_knowledge` /
`faction_knowledge` / `family_facts` 4 섹션을 최상위 필드로 선언하면, 로드 시
E1의 `memory_store` / `rumor_store`에 자동 주입된다. E3.3은 그 결과를 UI에
가시화하는 조회 채널.

### ScenarioSeeds 구조 (`src/application/scenario_seeds.rs`)

```rust
pub struct ScenarioSeeds {
    pub initial_rumors: Vec<RumorSeedInput>,          // 소문 aggregate 시드
    pub world_knowledge: Vec<WorldKnowledgeSeed>,     // Canonical World 엔트리
    pub faction_knowledge: HashMap<String, Vec<MemoryEntrySeedInput>>,  // 문파별
    pub family_facts: HashMap<String, Vec<MemoryEntrySeedInput>>,       // 가문별
}
```

네 섹션 모두 optional (`#[serde(default, skip_serializing_if = ...)]`). 빈 섹션은
직렬화에서 빠지므로 기존 시나리오 JSON 포맷과 호환.

- **`MemoryEntrySeedInput`**: scope 무관 공통 필드 (`content` 필수, 나머지는
  기본값 규칙). `into_entry(scope, fallback_id)`로 `MemoryEntry` 빌드. provenance는
  항상 Seeded 강제. 미지정 기본값:
  - `memory_type`: scope로부터 추론 (World→WorldEvent / Faction→FactionKnowledge /
    Family→FamilyFact / Personal→DialogueTurn).
  - `source`: `Experienced`, `layer`: `memory_type.initial_layer()`, `confidence`: `1.0`,
    `timestamp_ms`: `0`, `id`: `seed-{fallback_id}`.
- **`WorldKnowledgeSeed`**: `world_id: String` + `#[serde(flatten)] entry: MemoryEntrySeedInput`.
- **`RumorSeedInput`**: `topic` / `seed_content` 조합에 따라 3-tier 해소 + 에러:
  - `(topic, seed_content)` → `Rumor::with_forecast_content` (예보된 사실)
  - `(topic, None)` → `Rumor::new` (Canonical 참조)
  - `(None, seed_content)` → `Rumor::orphan` (고아)
  - `(None, None)` → `RumorError::OrphanRumorMissingSeed`
  - `origin` 미지정 시 `Seeded` 기본 주입.

### `POST /api/load` — LoadResponse 확장 (E3.2)

시나리오 로드 시 `apply_scenario_seeds`가 각 섹션을 순회하며 best-effort로
MemoryStore/RumorStore에 저장. 개별 실패는 `SeedReport.warnings`에 수집되고
`tracing::warn!`에도 기록.

```json
POST /api/load { "path": "data/sample.json" }
→ 200 OK
{
  "applied_rumors": 3,
  "applied_memories": 5,
  "warnings": [
    "rumor-seed-2: orphan rumor requires seed_content",
    "world_knowledge[0]: topic mismatch with existing Canonical"
  ]
}
```

embed 미포함 빌드는 seeds 적용이 건너뛰어지므로 `applied_*=0`, `warnings=[]`.
구 클라이언트는 모든 신규 필드를 무시하고 성공으로 처리.

### `GET /api/scenario-seeds` — 현재 선언 조회 (E3.3, 읽기 전용)

`StateInner.scenario_seeds`를 그대로 직렬화. 작가가 "현재 시나리오에 무엇이
선언되어 있나" 확인하는 UI 조회 전용 채널. embed 무관하게 항상 활성 (필드가
always-on). 빈 시드면 빈 객체 `{}` 반환 (섹션별 `skip_serializing_if`).

```json
GET /api/scenario-seeds
→ 200 OK
{
  "initial_rumors": [
    { "id": "r-1", "topic": "sword-of-north",
      "seed_content": "북방의 검이 돌아왔다.",
      "origin": { "kind": "seeded" },
      "reach": { "regions": ["jianghu"], "factions": [], "npc_ids": [],
                 "min_significance": 0.0 } }
  ],
  "world_knowledge": [
    { "world_id": "jianghu", "topic": "sword-of-north",
      "content": "북방의 검이 돌아왔다 — Canonical." }
  ]
}
```

### 적용 순서 (로드 핸들러)

1. 시나리오 JSON 파일 읽기 → `StateInner` 대체 (4 seed 섹션은 flatten으로 흡수)
2. `rebuild_repo_from_inner` (공유 repo 재구성)
3. `apply_scenario_seeds` (embed gated, `SeedReport` 수집)
4. scene 로드 (Focus 옵션 등)
5. `StateEvent::ScenarioLoaded` SSE 방출

프런트엔드(`loadHandlers.loadScenario`)는 `applied_*` count를 success 토스트에,
`warnings`는 error 토스트(3건 초과 시 첫 건 + 총 건수 + `console.warn` 폴백)로
노출. `useStateSync`는 `scenario_loaded`/`result_loaded` 이벤트 + 최초 마운트에서
`/api/scenario-seeds`를 fetch해 "시드" 탭에 반영한다.
