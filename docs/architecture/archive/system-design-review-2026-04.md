# NPC Mind Engine — 시스템 설계 검토 및 개선 방향

**작성일**: 2026-04-11  
**범위**: 아키텍처 전반 (Domain, Application, Adapter, Presentation, Mind Studio)

---

## 1. 현재 아키텍처 요약

```
┌─────────────────────────────────────────────────────────────────┐
│                     Mind Studio (bin)                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────────┐  │
│  │ Axum REST │  │ MCP/SSE  │  │ SSE Sync │  │ Static Files  │  │
│  └─────┬─────┘  └────┬─────┘  └─────┬────┘  └───────────────┘  │
│        │              │              │                           │
│        └──────┬───────┘──────────────┘                          │
│               ▼                                                 │
│         ┌─────────────┐                                         │
│         │  AppState    │ ← Arc<RwLock<StateInner>> + 서비스들    │
│         └──────┬──────┘                                         │
└────────────────┼────────────────────────────────────────────────┘
                 │
┌────────────────┼────────────────────────────────────────────────┐
│  Application   │                                                │
│  ┌─────────────▼───────────┐  ┌─────────────────────────────┐  │
│  │ MindService<R, A, S>    │  │ FormattedMindService        │  │
│  │  • appraise()           │  │  (MindService + Formatter)  │  │
│  │  • apply_stimulus()     │  └─────────────────────────────┘  │
│  │  • start_scene()        │                                    │
│  │  • after_dialogue()     │  ┌─────────────────────────────┐  │
│  │  • transition_beat()    │  │ DialogueTestService         │  │
│  └────┬──────┬──────┬──────┘  │  (chat feature)             │  │
│       │      │      │         └─────────────────────────────┘  │
│  ┌────▼──┐ ┌─▼───┐ ┌▼──────┐                                  │
│  │Rel.Svc│ │Scene│ │Sit.Svc│  (내부 서비스)                     │
│  └───────┘ │ Svc │ └───────┘                                    │
│            └─────┘                                              │
└────────────────────────────────────────────────────────────────┘
                 │
┌────────────────┼────────────────────────────────────────────────┐
│  Domain        │  (순수 비즈니스 로직, 외부 의존성 없음)           │
│                │                                                │
│  ┌─────────────▼──┐  ┌──────────┐  ┌───────────────┐           │
│  │ AppraisalEngine│  │ Stimulus │  │ ActingGuide   │           │
│  │  event/action/ │  │ Engine   │  │  Directive    │           │
│  │  object/compnd │  │ (PAD→Δ)  │  │  Snapshot     │           │
│  └────────────────┘  └──────────┘  └───────────────┘           │
│                                                                 │
│  ┌─────────┐ ┌────────────┐ ┌─────┐ ┌──────┐ ┌──────────┐    │
│  │HEXACO   │ │Relationship│ │ PAD │ │Scene │ │Situation │    │
│  │Profile  │ │(3-axis)    │ │Table│ │Focus │ │(E/A/O)   │    │
│  └─────────┘ └────────────┘ └─────┘ └──────┘ └──────────┘    │
└────────────────────────────────────────────────────────────────┘
                 │
┌────────────────┼────────────────────────────────────────────────┐
│  Ports         │  (ports.rs — 헥사고날 경계)                     │
│  MindRepository = NpcWorld + EmotionStore + SceneStore          │
│  Appraiser, StimulusProcessor, GuideFormatter                   │
│  TextEmbedder, PadAnchorSource, UtteranceAnalyzer              │
│  ConversationPort, LlamaServerMonitor (chat)                    │
└────────────────────────────────────────────────────────────────┘
                 │
┌────────────────┼────────────────────────────────────────────────┐
│  Adapters      │                                                │
│  InMemoryRepository │ FileAnchorSource │ OrtEmbedder (embed)   │
│  RigChatAdapter + TimingsCapturingClient (chat)                 │
│  LocaleFormatter (presentation)                                 │
└────────────────────────────────────────────────────────────────┘
```

### 핵심 데이터 흐름

```
Scene 시작 → Situation 구성 → appraise() → EmotionState + ActingGuide
                                                    │
대사 입력 → PAD 분석 → apply_stimulus() ─────────────┤
                          │                          │
                    [trigger 충족?]──Yes──→ transition_beat()
                          │                   (관계갱신 + 감정병합)
                          No                         │
                          │                          ▼
                          └──────────────── StimulusResult
                                                     │
Scene 종료 → after_dialogue() → 관계갱신 + 감정초기화
```

---

## 2. 강점 (유지해야 할 것)

### 2-1. 헥사고날 아키텍처의 올바른 적용

도메인 계층이 외부 의존성으로부터 완전히 격리되어 있다. `AppraisalEngine`과 `StimulusEngine`은 zero-sized type으로, 순수 함수형 계산만 수행한다. 성격 모델을 교체하려면 `AppraisalWeights`와 `StimulusWeights` 트레이트만 구현하면 되고, 도메인 코드를 건드릴 필요가 없다. 이 설계 덕분에 HEXACO → Big Five 전환이나 커스텀 성격 모델 추가가 구조적으로 보장된다.

### 2-2. OCC+PAD 이중 모델의 효과적 조합

이산적 OCC 감정(22종)과 연속적 PAD 공간을 분리한 설계가 잘 작동하고 있다. Appraisal은 OCC로 명확한 감정 레이블을 생성하고, Stimulus는 PAD 공간에서 연속적 변화를 계산한 뒤 다시 OCC 강도에 반영한다. 이 이중 구조 덕분에 "화나는 중인데 사과를 들으면 점진적으로 누그러진다"는 자연스러운 감정 흐름이 가능하다.

### 2-3. Feature Gate를 통한 점진적 확장

`embed`, `chat`, `mind-studio` 세 feature가 깔끔하게 분리되어 있어, 라이브러리 코어만 사용하는 게임 엔진과 전체 개발 도구를 사용하는 시나리오가 동일한 코드베이스에서 공존한다. 라이브러리 사용자는 `npc-mind = { features = [] }` 만으로 경량 감정 엔진을 얻을 수 있다.

### 2-4. Scene Focus 자동 전환 시스템

선언적 트리거 조건(OR of AND groups)으로 Beat 전환을 정의하는 방식은, 게임 디자이너가 코드를 모르더라도 시나리오 JSON만 편집하여 감정적 전개를 설계할 수 있게 한다. 이 패턴은 게임 산업의 Behavior Tree와 유사한 접근이면서 감정 도메인에 특화되어 있다.

### 2-5. 테스트 인프라

`TestContext` 기반 통합 테스트 24개가 핵심 시나리오(배신, 경쟁, 공포+희망, 다턴 도발 등)를 커버한다. 도메인 계층이 순수 로직이므로 외부 의존성 없이 빠르게 테스트가 실행된다.

---

## 3. 문제점 및 개선 필요 영역

### 3-1. MindService God-Object 경향

**현상**: `mind_service.rs` 570줄, 12개 public 메서드, `transition_beat()` 내부에서 관계갱신 + 감정병합 + 가이드생성이 한 메서드에 묶여 있다.

**영향**: 개별 워크플로우 단위 테스트가 어렵고, `apply_stimulus()` 내부의 "일반 자극" vs "Beat 전환" 분기가 복잡하다.

**Trade-off**: 현재 1인 개발 규모에서는 한 곳에 로직이 모여있는 게 오히려 파악이 쉬울 수 있다. 다만 이 상태로 Multi-NPC나 감정 감쇠 기능이 추가되면 복잡도가 급격히 증가한다.

### 3-2. AppState 과도한 결합 (Mind Studio)

**현상**: `AppState`가 17개 이상의 필드를 가지며, `Arc<RwLock<StateInner>>` 하나에 NPC, 관계, 감정, 씬, 턴 히스토리가 전부 들어있다. 모든 핸들러와 MCP 도구가 이 단일 잠금을 공유한다.

**영향**: 감정 평가(write lock) 중에 NPC 목록 조회(read lock)도 대기해야 한다. 현재 단일 사용자 환경이라 체감은 적지만, 향후 멀티 NPC 동시 대화 시 병목이 된다.

### 3-3. MCP 서버 모놀리식 (34개 도구 단일 match)

**현상**: `mcp_server.rs`의 `call_tool()`이 34개 도구를 하나의 거대한 match 문으로 처리. 각 분기에서 state 잠금 → 수정 → 이벤트 발행이 인라인으로 반복된다.

**영향**: 새 도구 추가 시 파일이 계속 비대해지고, 이벤트 발행 패턴의 불일치가 생기기 쉽다. 현재도 800줄 이상으로 추정된다.

### 3-4. 감정 감쇠(Decay) 부재

**현상**: 감정이 시간에 따라 자연 감쇠하지 않는다. 10턴 전의 분노가 자극 없이도 동일 강도로 유지된다.

**영향**: 장기 대화에서 감정이 비현실적으로 축적된다. 무협 소설의 장면 전환(예: 시간 경과 후 재회)을 자연스럽게 표현할 수 없다.

### 3-5. Single-NPC 관점 한계

**현상**: 현재 한 번에 하나의 NPC 관점만 처리. "NPC A가 NPC B와 대화하는 것을 NPC C가 엿듣는" 시나리오를 표현할 수 없다.

**영향**: 무협 소설의 핵심인 다자간 갈등 구도(예: 문파 내 세력 다툼)를 구현하려면 구조적 확장이 필요하다.

### 3-6. Directive 결정 로직의 비일관성

**현상**: Tone, Attitude, BehavioralTendency, Restriction 4개 enum이 각각 다른 패턴의 결정 로직을 사용한다. 어떤 것은 헬퍼 메서드를, 어떤 것은 매직 넘버를 사용한다.

**영향**: 새 Tone이나 Attitude 변형을 추가할 때 어느 패턴을 따라야 할지 모호하다. 유지보수 비용이 누적된다.

### 3-7. Repository의 이중 trait 구현

**현상**: `InMemoryRepository`가 `NpcWorld`, `EmotionStore`, `SceneStore`를 owned와 `&mut` 버전으로 두 번씩 구현하여 약 25%의 코드 중복이 있다.

**영향**: 저장소 인터페이스 변경 시 두 곳을 동시에 수정해야 한다.

---

## 4. 개선 방향

### 4-1. MindService 워크플로우 분리 [우선순위: 높음]

**현재**:
```
MindService.apply_stimulus()
  └─ 직접 호출: transition_beat() → update_beat_relationship() → appraise() → merge
```

**개선안**: Command 패턴 또는 내부 서비스 분리

```rust
// 안 1: 내부 서비스 추출
struct BeatTransitionService<'a, R, A, S> { /* MindService 내부 참조 */ }

impl BeatTransitionService {
    fn execute(&self, trigger: &SceneFocus, state: &EmotionState) -> TransitionResult {
        // 1. update_beat_relationship
        // 2. appraise new focus
        // 3. merge_from_beat
        // 4. save_scene
    }
}

// 안 2: apply_stimulus 결과를 단순화
enum StimulusOutcome {
    Updated(EmotionState),
    BeatTransition { new_state: EmotionState, old_focus: FocusId, new_focus: FocusId },
}
```

**기대효과**: transition_beat 단위 테스트 가능, apply_stimulus 의 분기 복잡도 감소.

### 4-2. AppState 분할 잠금 [우선순위: 높음]

**현재**:
```rust
struct AppState {
    inner: Arc<RwLock<StateInner>>,  // 모든 것이 하나의 잠금
}
```

**개선안**: 관심사별 분리 잠금

```rust
struct AppState {
    world: Arc<RwLock<WorldState>>,      // NPC, 관계, 오브젝트
    emotions: Arc<RwLock<EmotionState>>,  // 감정 상태 (빈번한 write)
    scenes: Arc<RwLock<SceneState>>,      // 씬/비트 상태
    history: Arc<RwLock<HistoryState>>,   // 턴 히스토리 (append-only)
    meta: Arc<RwLock<MetaState>>,         // 시나리오 메타, 테스트 레포트
    // 읽기 전용 서비스들은 잠금 불필요
    analyzer: Option<Arc<PadAnalyzer>>,
    formatter: Arc<LocaleFormatter>,
}
```

**Trade-off**: 잠금이 분산되면 "여러 상태를 동시에 읽어야 하는" 경우(예: appraise 시 NPC + 관계 + 씬 모두 필요)에 데드락 위험이 생긴다. 항상 world → emotions → scenes 순서로 잠금을 획득하는 규칙을 강제해야 한다.

### 4-3. MCP 도구 핸들러 모듈화 [우선순위: 중간]

**개선안**: REST 핸들러와 동일한 패턴으로 MCP 도구를 모듈별로 분리

```
mcp_server.rs (라우팅만)
  ├── mcp_tools/
  │   ├── world.rs      (create_npc, create_relationship, create_object, delete_*)
  │   ├── emotion.rs    (appraise, apply_stimulus, generate_guide, after_dialogue)
  │   ├── scene.rs      (start_scene, get_scene_info, load_scene_focuses)
  │   ├── scenario.rs   (list_scenarios, load_scenario, save_scenario, create_full)
  │   ├── dialogue.rs   (dialogue_start, dialogue_turn, dialogue_end)
  │   └── query.rs      (get_history, get_situation, load_result, ...)
```

각 모듈은 `MCP tool handler` 트레이트를 구현하고, 이벤트 발행은 공통 미들웨어로 추출:

```rust
fn emit_after<F>(state: &AppState, event: StateEvent, f: F) -> McpResult
where F: FnOnce(&mut StateInner) -> McpResult
{
    let result = f(&mut state.inner.write()?);
    state.emit(event);
    result
}
```

### 4-4. 감정 감쇠 시스템 도입 [우선순위: 중간]

**설계안**:

```rust
// domain/emotion/decay.rs
pub struct EmotionDecay;

impl EmotionDecay {
    /// 턴 수 기반 감쇠 (시간이 아닌 대화 턴 단위)
    pub fn apply(state: &mut EmotionState, elapsed_turns: u32) {
        let decay_rate = EMOTION_DECAY_PER_TURN; // 예: 0.03
        for emotion in state.emotions_mut() {
            let decay = decay_rate * elapsed_turns as f32;
            let new_intensity = (emotion.intensity() - decay).max(0.0);
            if new_intensity < STIMULUS_FADE_THRESHOLD {
                emotion.mark_for_removal();
            } else {
                emotion.set_intensity(new_intensity);
            }
        }
    }
}
```

**적용 지점**: `apply_stimulus()` 호출 시 이전 턴과의 간격을 계산하여 자동 감쇠. Scene 시작 시에도 이전 Scene 종료 후 경과 턴을 반영.

**튜닝 상수 후보**:
- `EMOTION_DECAY_PER_TURN`: 0.02~0.05 (턴당 감쇠율)
- `EMOTION_DECAY_FLOOR`: 0.1 (기본 감정은 바닥까지 감쇠하지 않음)
- `MOOD_DECAY_RATE`: 0.01 (전체 무드는 더 천천히 변화)

### 4-5. Multi-NPC 관점 확장 구조 [우선순위: 낮음 — 설계만]

**현재**: `MindService.appraise(npc_id, situation)` — 1 NPC, 1 상황

**목표**: 같은 이벤트를 여러 NPC가 각자의 성격과 관계로 해석

```rust
// 안: MultiMindService (MindService 래퍼)
struct MultiMindService<R, A, S> {
    service: MindService<R, A, S>,
}

impl MultiMindService {
    /// 하나의 상황을 여러 NPC 관점에서 평가
    fn appraise_group(
        &self,
        npc_ids: &[&str],
        situation: &Situation,
    ) -> Vec<(String, AppraiseResult)> {
        npc_ids.iter()
            .map(|id| {
                let result = self.service.appraise(/* id별 요청 */);
                (id.to_string(), result)
            })
            .collect()
    }

    /// NPC 간 감정 전파 (A의 분노를 B가 감지)
    fn propagate_emotion(
        &self,
        source_npc: &str,
        observer_npc: &str,
        observation_type: ObservationType, // Overheard, Witnessed, Reported
    ) -> StimulusResult { ... }
}
```

**데이터 모델 확장**:
- `ObservationType` enum: `Direct`, `Overheard`, `Witnessed`, `Rumored`
- 관찰 타입별 감정 전파 강도 계수 (직접 > 목격 > 전해들음)
- Scene에 `participants: Vec<NpcId>` 추가

**Trade-off**: Multi-NPC는 조합 폭발 위험이 있다 (N명 × N명 관계). 우선 2~3명 소규모 그룹에서 검증 후 확장해야 한다.

### 4-6. Directive 결정 로직 통일 [우선순위: 낮음]

**개선안**: 테이블 기반 결정 패턴으로 통일

```rust
struct DirectiveRule {
    condition: Box<dyn Fn(&EmotionSnapshot, &PersonalitySnapshot) -> bool>,
    result: DirectiveVariant,
    priority: u8,
}

// Tone, Attitude, BehavioralTendency 모두 동일 패턴
fn decide(rules: &[DirectiveRule], emotion: &EmotionSnapshot, personality: &PersonalitySnapshot) -> DirectiveVariant {
    rules.iter()
        .filter(|r| (r.condition)(emotion, personality))
        .max_by_key(|r| r.priority)
        .map(|r| r.result)
        .unwrap_or(DirectiveVariant::default())
}
```

**기대효과**: 새 지시어 변형 추가가 규칙 테이블에 행 추가로 단순화됨. 규칙을 TOML/JSON 외부 파일로 이동할 수도 있음.

### 4-7. Repository 트레이트 단순화 [우선순위: 낮음]

**개선안**: `&mut` 중복 구현 제거

```rust
// blanket impl으로 대체
impl<T: NpcWorld> NpcWorld for &mut T {
    fn get_npc(&self, id: &str) -> Option<&Npc> { (**self).get_npc(id) }
    // ... 위임
}
```

또는 저장소가 `&self`로 내부 가변성(`RefCell` 또는 이미 사용 중인 `RwLock`)을 사용하도록 전환.

---

## 5. 우선순위 로드맵

### Phase A: 구조 정리 (현재 → 다음 기능 추가 전)

| 항목 | 난이도 | 영향도 | 예상 작업량 |
|------|--------|--------|------------|
| MCP 도구 핸들러 모듈화 | 중 | 유지보수성 ↑ | 1일 |
| BeatTransition 서비스 추출 | 중 | 테스트 용이성 ↑ | 0.5일 |
| Repository `&mut` 중복 제거 | 하 | 코드량 ↓ | 0.5일 |

### Phase B: 감정 모델 고도화 (Phase 4와 병행)

| 항목 | 난이도 | 영향도 | 예상 작업량 |
|------|--------|--------|------------|
| 감정 감쇠 시스템 | 중 | 자연스러운 장기 대화 | 1일 |
| Directive 테이블 기반 전환 | 중 | 확장 용이성 ↑ | 1일 |
| Beat trigger 동적 조정 UI | 중 | 시나리오 튜닝 속도 ↑ | 1일 |

### Phase C: 확장성 (Phase 5)

| 항목 | 난이도 | 영향도 | 예상 작업량 |
|------|--------|--------|------------|
| AppState 분할 잠금 | 상 | 동시성 ↑ | 2일 |
| Multi-NPC 관점 기초 | 상 | 무협 다자 갈등 표현 | 3~5일 |
| 감정 전파 시스템 | 상 | NPC 간 상호작용 | 2~3일 |

---

## 6. 게임 통합 관점의 고려사항

### 6-1. 라이브러리 사용 시 의존성 무게

현재 기본 빌드(`cargo build`)의 의존성은 `serde`, `serde_json`, `toml`, `thiserror`, `tracing`으로 비교적 가볍다. 게임 엔진(Bevy, Godot-Rust 등)에 통합할 때 충돌 가능성이 낮은 구성이다. 다만 `embed` feature 활성화 시 ONNX Runtime이 바이너리 크기를 크게 늘리므로, 게임 빌드에서는 서버 사이드 임베딩을 고려해야 할 수 있다.

### 6-2. 실시간 성능

`appraise()`와 `apply_stimulus()`는 순수 계산이므로 μs 단위로 빠르다. 병목은 `embed` feature의 텍스트 임베딩(ms 단위)과 `chat` feature의 LLM 호출(초 단위)이다. 게임 루프에서는 감정 계산은 동기적으로, 임베딩과 LLM은 비동기로 처리하는 전략이 적합하다.

### 6-3. 시나리오 JSON 스키마 안정성

`data/` 디렉토리의 시나리오 JSON이 도메인 모델과 직접 매핑되므로, 도메인 구조가 변경되면 기존 시나리오 JSON이 깨질 수 있다. 스키마 버전 관리와 마이그레이션 유틸리티가 필요한 시점이 올 것이다.

---

## 7. 결론

NPC Mind Engine의 헥사고날 아키텍처와 OCC+PAD 이중 감정 모델은 견고한 기반이다. 현재 가장 시급한 개선은 Mind Studio 계층의 구조 정리(MCP 모듈화, AppState 분리)이며, 도메인 계층은 감정 감쇠 기능 추가를 제외하면 큰 변경이 필요하지 않다.

1인 개발 프로젝트에서 가장 중요한 것은 "유지보수 가능한 복잡도"를 유지하는 것이다. MindService에 새 기능을 계속 추가하기보다는, 먼저 BeatTransition을 분리하여 단위 테스트 가능한 구조를 만들고, 그 위에 감정 감쇠와 Multi-NPC를 쌓아 올리는 순서가 적합하다.
