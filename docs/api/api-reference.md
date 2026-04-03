# API Reference

`npc-mind` 라이브러리의 공개 API 레퍼런스입니다.

---

## Service Layer (진입점)

### MindService — 도메인 결과 반환

포맷팅 없이 도메인 객체(`ActingGuide`)를 직접 반환합니다.

```rust
use npc_mind::MindService;

// 기본 엔진 사용
let mut service = MindService::new(repo);

// 커스텀 엔진 주입
let mut service = MindService::with_engines(repo, my_appraiser, my_stimulus);
```

### FormattedMindService — 포맷팅된 프롬프트 반환

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
| `PadAnchorSource` | PAD 앵커 텍스트 로드 | `TomlAnchorSource`, `JsonAnchorSource` |
| `TextEmbedder` | 텍스트 임베딩 | `OrtEmbedder` (embed feature) |
| `UtteranceAnalyzer` | 대사 → PAD 변환 | `PadAnalyzer` |
