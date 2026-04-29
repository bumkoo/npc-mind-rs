# ADR: Event Sourcing & CQRS 적용 검토

**버전:** v2.1.0
**작성일:** 2026-02-25
**갱신일:** 2026-03-01

---

## 상태

**검토 완료, 부분 구현 진행 중** (2026-03-01) — Step 0~1 완료. Relationship 이벤트 반환 Gap 해결됨. 관계 도메인 3축→2축 리팩터링 완료 (적대도 제거, 호감도 -100~+100). EventLog 포트 및 CQRS Projection은 로드맵에 따라 진행 예정.

## 맥락

칠국춘추는 AI-driven NPC가 기억·성장·관계를 통해 살아있는 서사를 만드는 무협 RPG다. 30년 전쟁의 역사 속에서 NPC 관계의 변화 이력 자체가 핵심 게임플레이 자산이다.

현재 아키텍처는 **Command-Event 패턴** — aggregate가 상태를 직접 변경(`&mut self`)한 후 `Vec<DomainEvent>`를 반환한다. 이벤트는 ephemeral하며, 어디에도 영속화되지 않는다.

이 문서는 Event Sourcing(ES)과 CQRS를 칠국춘추에 적용할 가치가 있는지, 있다면 어떤 범위와 방식으로 적용해야 하는지를 분석한다.

---

## 1. 현재 아키텍처 분석

### 1.1 ES 친화적 요소 (이미 있는 것)

**이벤트 반환 패턴이 완비되어 있다.** ES 도입에서 가장 어려운 "이벤트 식별"이 이미 완료되어 있다.

| 도메인 | 이벤트 수 | 반환 패턴 | 파일 |
|--------|----------|----------|------|
| Time | 4종 | `GameClock::tick()` → `Vec<DomainEvent>` | `time/event.rs` |
| Character | 5종 | `age_one_year()`, `add_fatigue()` 등 6개 메서드 | `character/event.rs` |
| Growth | 3종 | `train_stat()`, `practice_art()` 등 3개 메서드 | `growth/event.rs` |
| Memory | 3종 | (서비스 계층에서 생성) | `memory/event.rs` |
| Relationship | 6종 | ✅ `update_affinity()`, `update_trust()`, `set_relation_type()`, `record_interaction()` 4개 메서드 | `relationship/event.rs` |

추가 기반:
- 21개 이벤트 모두 `Serialize/Deserialize` derive — 즉시 영속화 가능
- 모든 이벤트에 `name() → &'static str` — event store의 `event_type` 컬럼으로 사용 가능
- `DomainEvent` wrapper enum이 도메인별 분리를 유지하면서 통합 전파 지원 (`shared/event.rs`)
- Newtype ID (`CharacterId(u64)` 등) — aggregate stream 식별에 바로 사용 가능
- No-op 규칙: 실제 변경 없으면 `Vec::new()` — 의미 있는 이벤트만 생성됨

**현재 패턴 예시 — Character** (`character/model.rs`):
```rust
pub fn age_one_year(&mut self) -> Vec<DomainEvent> {
    let old_stage = self.life_stage();
    self.current_age += 1;                    // ← 상태를 먼저 변경
    let new_stage = self.life_stage();
    let mut events = vec![
        CharacterEvent::Aged { character_id: self.id, new_age: self.current_age }.into()
    ];
    if old_stage != new_stage {
        events.push(CharacterEvent::LifeStageChanged { from: old_stage, to: new_stage }.into());
    }
    events                                     // ← 이벤트는 변경 후 반환
}
```

**현재 패턴 예시 — Relationship** (`relationship/types.rs`):
```rust
pub fn update_affinity(&mut self, delta: f32) -> Vec<DomainEvent> {
    self.update_axis(Axis::Affinity, delta)    // ← 2축 공통 로직 위임
}

fn update_axis(&mut self, axis: Axis, delta: f32) -> Vec<DomainEvent> {
    let old_value = match axis { /* 현재 값 읽기 */ };
    let old_level = self.level();
    let new_value = match axis {
        Axis::Affinity => clamp_affinity(old_value + delta),  // -100~+100
        Axis::Trust => clamp_trust(old_value + delta),        //    0~100
    };
    /* 상태 변경 */
    if old_value == new_value {
        return Vec::new();                     // ← no-op rule
    }
    let mut events = vec![/* AxisChanged 이벤트 */];
    let new_level = self.level();
    if old_level != new_level {
        events.push(RelationshipEvent::LevelChanged { .. }.into());
    }
    events
}
```

### 1.2 ES 부재 요소 (없는 것)

- **Event Store / Event Log 없음** — 이벤트가 반환된 후 소비·폐기됨
- **Event replay / reconstitution 없음** — `from_events()`, `apply_event()` 메서드 없음
- **이벤트 버스·구독 메커니즘 없음** — `TimeCharacterService`의 수동 매칭이 유일한 이벤트 처리
- **이벤트 메타데이터 없음** — `timestamp`, `sequence_number`, `aggregate_version` 없음
- **스냅샷 기반 직렬화가 유일한 영속 수단** — serde `Serialize/Deserialize`

### 1.3 CQRS 부분 요소

wuxia-llm에 **어댑터 소유 View 패턴**이 이미 존재한다 (`prompt/types.rs`):

| Domain Type | Adapter View | 변환 위치 | 소비자 |
|-------------|-------------|----------|--------|
| `RankedMemory` | `MemoryView` | `wuxia-app/context.rs` | LLM 프롬프트 |
| `Relationship` + descriptions | `RelationshipView` | `wuxia-app` | LLM 프롬프트 |
| 캐릭터 설정 | `CharacterPromptData` | TOML fixtures | LLM 프롬프트 |

이것은 write model(도메인 aggregate)과 read model(어댑터 뷰)이 분리된 **경량 CQRS의 씨앗**이다.

추가로 `ContextProvider` 트레이트(`conversation/context.rs`)가 읽기 전용 쿼리 인터페이스 역할을 한다.

### 1.4 ~~핵심 Gap: Relationship 이벤트 미반환~~ ✅ 해결 완료

> **[v2.0.0 갱신]** 이 Gap은 해결되었다. 모든 Relationship mutation 메서드가 `Vec<DomainEvent>`를 반환한다.

`Relationship` aggregate의 4개 mutation 메서드 모두 이벤트를 반환한다:

| 메서드 | 반환 이벤트 | No-op 조건 |
|--------|-----------|-----------|
| `update_affinity(delta)` | `AffinityChanged` + 선택적 `LevelChanged` | `old == new` (clamp 후) |
| `update_trust(delta)` | `TrustChanged` + 선택적 `LevelChanged` | `old == new` |
| `set_relation_type(type)` | `TypeChanged` | `old_type == new_type` |
| `record_interaction(time)` | `InteractionRecorded` | (항상 발행) |

2축 업데이트는 `update_axis()` 내부 헬퍼로 통합되어 DRY 원칙을 따른다. 호감도는 `clamp_affinity()` (-100~+100), 신뢰도는 `clamp_trust()` (0~100)로 축별 클램핑을 적용한다. `apply_conversation_effect()` 도메인 서비스가 `ConversationEffect` → `update_affinity()` → 이벤트 반환 체인을 완성한다.

---

## 2. Event Sourcing 적용 검토

### 2.1 이 프로젝트에 특화된 이점

**30년 전쟁 시간 질의 (Temporal Queries)**

게임 배경이 30년 전쟁이다. "1185년에 소연의 사부는 살아있었는가?", "혈교 습격 이후 소연과 조고의 관계는 어떻게 변했는가?" 같은 시간 기반 질의는 event replay로 자연스럽게 해결된다. 현재 `GameTime`(360일/년, 6 watch/일)이 이미 정의되어 있어, 이벤트에 `GameTime`을 첨부하면 특정 게임 시점의 상태 재구성이 가능하다.

**NPC 행동 디버깅**

"왜 소연이 갑자기 적대적이 되었는가?"에 답하려면 relationship 이벤트 히스토리가 필수다. 호감도가 음수로 떨어지는 과정(AffinityChanged 이벤트 시퀀스)을 추적하면 원인을 파악할 수 있다. LLM 대화 → 관계 변화 → 기억 저장으로 이어지는 체인의 완전한 재현이 가능해진다. Quality benchmarking 파이프라인(`wuxia-llm/quality/`)에 event log 기반 리플레이를 추가하면 NPC 행동 품질 분석이 크게 개선된다.

**게임 세이브/분기**

- Snapshot = 세이브 파일 (모든 aggregate에 `Serialize/Deserialize` 있음)
- Snapshot + event replay = 게임 로드 후 최근 이벤트만 재적용
- 분기 = 특정 시점의 snapshot + 다른 이벤트 스트림 → "what-if" 시나리오

**Relationship 변화 이력 = 핵심 서사 자산**

```
Stranger → 정보 거래 → Acquaintance → 개방 언급 → Friendly → 과거 고백 → Close → Intimate
```
또는:
```
Friendly → 조고 편에 섬 → affinity 급락(-80 이하) → Enemy → BondBroken
```

이 이력을 영속화하면: LLM에 관계 이력 context 제공, 플레이어에게 "인맥첩" UI 표시, 게임 후기 분석이 가능하다.

### 2.2 이 프로젝트에 특화된 비용

**Bevy 게임 루프와의 성능 고려**

Bevy 0.18 ECS는 60fps 기반이다. 다만 `GameClock::tick()`의 실제 빈도는 대화·UI 시간을 고려하면 초당 수회 이하이며, 대부분 이벤트는 대화 종료 시 일괄 발생한다. **실제 쓰기 부하는 심각하지 않다. 진짜 위험은 복잡성이다.**

**6+ 미구현 도메인의 이벤트 스키마 진화**

Psychology, World, Space, Narrative, Combat, Economy가 아직 미구현이다. Full ES에서는 과거 이벤트를 새 스키마로 읽어야 하는 migration 부담이 추가된다. 현재 `#[serde(default)]`로 backward compatibility를 관리하고 있지만, event upcasting까지 필요해질 수 있다.

**2D 픽셀아트 RPG에 Full ES는 과잉**

인디 규모 게임에 full ES를 도입하면 개발 속도 저하, 학습 곡선 증가, 미구현 도메인에 패턴 강제 등의 문제가 발생한다.

**기존 테스트 수정 부담**

wuxia-core 테스트가 스냅샷 기반으로 작성되어 있다. Full ES로 패러다임을 전환하면 대규모 수정이 필요하다.

### 2.3 Aggregate별 ES 적합도 분석

| Aggregate | ES 적합도 | 이벤트 수 | 근거 |
|-----------|----------|----------|------|
| **Relationship** | **높음** | 6종 | 2축 변화 이력이 서사에 핵심. "왜 원수가 됐는가" (호감도 음수 추적) 가치 최고 |
| **Memory** | 중간 (특수) | 3종 | 이미 append-only 구조. `ImportanceUpdated`만 추적 가치 |
| **Growth** | 중간 | 3종 | 수련 이력이 성장 서사에 유용하나, 단순 수치 누적이라 가치 대비 복잡성 높음 |
| **Character** | 낮음 | 5종 | 상태 변경이 단순(age, fatigue, injury). 스냅샷으로 충분 |
| **GameClock** | **없음** | 4종 | 단순 카운터. `WatchChanged` 2160개/년 — 노이즈 |

---

## 3. CQRS 적용 검토

### 3.1 기존 View 패턴의 CQRS 진화 경로

현재 아키텍처에 3단계의 CQRS가 존재한다:

```
Stage 0 (현재): Domain Aggregate → 직접 getter 호출
Stage 1 (현재): Domain Type → Adapter View Type (wuxia-llm)
Stage 2 (향후): Domain Event → Projection → Read Model
```

Stage 1이 이미 동작 중이므로, Stage 2로의 진화는 "이벤트 발생 시 read model을 자동 갱신"하는 projection handler를 추가하는 것이다.

### 3.2 유용한 Read Model Projection

| Projection | Source Events | 소비자 | 우선순위 |
|-----------|--------------|--------|---------|
| `NpcRelationshipSummary` | `RelationshipEvent` 전체 | LLM 프롬프트, UI | **P0** |
| `RelationshipHistoryView` | `RelationshipEvent` 전체 | "인맥첩" UI, LLM context | P1 |
| `CharacterTimelineLog` | `CharacterEvent`, `GrowthEvent` | "성장 일지" UI | P1 |
| `RecentEventFeed` | `DomainEvent` 전체 | 게임 내 "강호 소식" UI | P2 |
| `NpcBehaviorTrace` | `MemoryEvent` + `RelationshipEvent` | 품질 분석 도구 | P2 |

### 3.3 Full CQRS vs Lightweight CQRS

**Lightweight CQRS (권장):**
- Write side: 현재 aggregate 유지 (state-first)
- Read side: 발생한 이벤트를 수신하여 view model 갱신
- 동일 프로세스 내, 동기적, 별도 DB 불필요

**Full CQRS (불필요):**
- 별도 read DB + 비동기 event bus + eventual consistency 처리
- 분산 시스템을 위한 것이며, **싱글 플레이어 게임에는 과도하다**

---

## 4. 결정: Hybrid Event Log + Lightweight CQRS

Full ES도 아니고, 현상태 유지도 아닌 **실용적 중간 전략**을 채택한다.

```
현재:
  Aggregate.command() → State mutation + Vec<DomainEvent> → (버려짐)

제안:
  Aggregate.command() → State mutation + Vec<DomainEvent> → EventLog에 append
                                                             ↓
                                                        Read Model 갱신 (선택적)
```

### 핵심 원칙

1. **상태의 소스는 여전히 스냅샷** — 기존 패턴 유지, 테스트 수정 불필요
2. **이벤트 로그는 부가 기능** — 감사(audit), 타임라인, 디버깅, LLM context 풍부화
3. **이벤트에서 상태 복원은 하지 않음** — reconstitution 없음 (Relationship에 한해 검증용 replay만 선택적)
4. **기존 헥사고날 아키텍처와 일관** — EventLog를 Port trait으로 정의

---

## 5. 구체적 설계 제안

### 5.1 EventLog 포트

wuxia-core의 dependency rule(serde만 허용)을 준수한다.

```rust
// crates/wuxia-core/src/shared/event_log.rs (신규)

/// 이벤트 저장 시 첨부되는 메타데이터.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// 이벤트 순서 번호 (전역 단조 증가).
    pub sequence: u64,
    /// 이벤트가 발생한 게임 내 시간.
    pub game_time: GameTime,
    /// 이벤트 페이로드.
    pub event: DomainEvent,
    /// Aggregate 유형 (e.g., "Character", "Relationship").
    pub aggregate_type: String,
    /// Aggregate ID (raw u64).
    pub aggregate_id: u64,
}

/// Event Log 포트 (헥사고날 아키텍처).
///
/// MemoryRepository, RelationshipRepository와 동일한 패턴.
/// wuxia-core가 trait를 정의하고, wuxia-memory가 구현한다.
pub trait EventLog: Send + Sync {
    /// 이벤트를 append-only로 저장한다.
    fn append(&mut self, envelope: EventEnvelope) -> Result<(), String>;

    /// 특정 aggregate의 이벤트를 시간순으로 조회한다.
    fn events_for(
        &self,
        aggregate_type: &str,
        aggregate_id: u64,
    ) -> Vec<EventEnvelope>;

    /// 특정 시간 범위의 모든 이벤트를 조회한다 (분석용).
    fn events_in_range(
        &self,
        from: GameTime,
        to: GameTime,
    ) -> Vec<EventEnvelope>;

    /// 전체 이벤트 수 (모니터링용).
    fn total_count(&self) -> usize;
}
```

구현체:
- `InMemoryEventLog` (wuxia-memory): `Vec<EventEnvelope>` 기반, 테스트/MVP
- `PersistentEventLog` (향후): SQLite 또는 JSON Lines 파일

### 5.2 Application Service 변경

이벤트를 받아서 EventLog에 append하는 로직은 조립 계층에서 처리한다:

```rust
// 기존 (이벤트 소비 후 폐기)
let events = clock.tick();
let char_events = service.process_time_events(&events, &mut characters);

// 변경 후 (이벤트 영속화)
let events = clock.tick();
event_log.append_batch(&events, game_time)?;          // ← 추가
let char_events = service.process_time_events(&events, &mut characters);
event_log.append_batch(&char_events, game_time)?;     // ← 추가
```

### 5.3 ~~Relationship 이벤트 반환 보강~~ ✅ 구현 완료

> **[v2.0.0 갱신]** 이 제안은 이미 구현되었다. 실제 구현은 제안보다 개선된 `update_axis()` 패턴을 사용한다.

실제 구현 (`relationship/types.rs`):

```rust
/// 2축 공통 업데이트 로직. 값 변경 → 이벤트 발행 → 레벨 전이 감지.
fn update_axis(&mut self, axis: Axis, delta: f32) -> Vec<DomainEvent> {
    let old_value = match axis {
        Axis::Affinity => self.affinity,
        Axis::Trust => self.trust,
    };
    let old_level = self.level();
    let new_value = match axis {
        Axis::Affinity => clamp_affinity(old_value + delta),  // -100~+100
        Axis::Trust => clamp_trust(old_value + delta),        //    0~100
    };
    match axis {
        Axis::Affinity => self.affinity = new_value,
        Axis::Trust => self.trust = new_value,
    }
    if old_value == new_value {
        return Vec::new();  // no-op rule
    }
    let changed_event = match axis {
        Axis::Affinity => RelationshipEvent::AffinityChanged { .. },
        Axis::Trust => RelationshipEvent::TrustChanged { .. },
    };
    let mut events = vec![changed_event.into()];
    let new_level = self.level();
    if old_level != new_level {
        events.push(RelationshipEvent::LevelChanged { .. }.into());
    }
    events
}
```

ADR 제안 대비 개선점:
- **DRY**: 2축 모두 `update_axis()`로 통합 (코드 중복 제거)
- **축별 클램핑**: 호감도(-100~+100)와 신뢰도(0~100)에 별도 클램프 함수 적용
- **정확한 비교**: `f32::EPSILON` 대신 clamp 후 `==` 비교 (clamp 결과가 동일하면 no-op)
- **`set_relation_type()`과 `record_interaction()`도 이벤트 반환** — 원래 제안에 포함되지 않았던 메서드

### 5.4 Lightweight CQRS Projection

```rust
// crates/wuxia-core/src/shared/projection.rs (신규)

/// Read Model을 이벤트 기반으로 갱신하는 projection handler.
pub trait Projection: Send + Sync {
    fn handle(&mut self, event: &DomainEvent);
}
```

조립 계층(`wuxia-app`)에서 EventDispatcher가 이벤트 발생 후 등록된 Projection들에 전파한다.

### 5.5 헥사고날 아키텍처 통합도

```
wuxia-core (pure domain — serde only)
    ├── shared/event.rs          ← DomainEvent (기존)
    ├── shared/event_log.rs      ← EventLog trait + EventEnvelope (신규)
    ├── shared/projection.rs     ← Projection trait (신규)
    ├── relationship/types.rs    ← 2축 모델 (affinity -100~+100, trust 0~100)
    │
wuxia-memory (adapter)
    ├── in_memory.rs             ← InMemoryEventLog (신규)
    │
wuxia-app (assembly)
    ├── event_dispatcher.rs      ← EventDispatcher (신규)
    │
wuxia-game (Bevy — Phase 5)
    └── systems/                 ← Projection handlers
```

Dependency rule 준수: `EventLog` trait은 wuxia-core에, 구현체는 wuxia-memory에, 조립은 wuxia-app에.

---

## 6. 단계적 적용 로드맵

| 단계 | 시점 | 내용 | 영향 범위 | 상태 |
|------|------|------|----------|------|
| **Step 0** | 즉시 | 이 ADR 문서 작성 | `docs/` only | ✅ 완료 |
| **Step 1** | Phase 4 후반 | Relationship 이벤트 반환 Gap 해결 | `wuxia-core/relationship/` | ✅ 완료 — `update_axis()` 패턴으로 5개 메서드 모두 이벤트 반환 |
| **Step 2** | Phase 4 후반 | `EventLog` 포트 trait + `InMemoryEventLog` | `wuxia-core` (trait), `wuxia-memory` (impl) | 미착수 |
| **Step 3** | Phase 5 | Bevy 시스템에서 이벤트 → EventLog append 연결 | `wuxia-game` | 미착수 |
| **Step 4** | Phase 5 | Lightweight CQRS Projection 도입 | `wuxia-core` (trait), `wuxia-app` (impl) | 미착수 |
| **Step 5** | Phase 6+ | Read Model Projection 확장 (관계 네트워크, 성장 타임라인) | `wuxia-app` | 미착수 |

---

## 7. 결론 및 최종 권장

### DO

- ✅ ~~**Relationship 이벤트 반환 Gap 해결**~~ — 완료. `update_axis()` 패턴으로 5개 메서드 모두 `Vec<DomainEvent>` 반환
- **Hybrid Event Log 도입**: 이벤트를 append-only로 기록하되, 상태 복원에는 사용하지 않음 *(다음 우선순위)*
- **Relationship 도메인 이벤트 로깅 우선 적용**: 서사적 가치가 가장 높은 도메인
- **기존 View 패턴을 공식적인 Lightweight CQRS Read Model로 발전**: `MemoryView`, `RelationshipView` 패턴 확장
- **EventLog 포트를 헥사고날 포트로 정의**: `MemoryRepository` 패턴과 일관성 유지

### DON'T

- **Full Event Sourcing** (이벤트에서 상태 복원) — 2D 인디 RPG에 과잉
- **기존 테스트 대규모 수정** — 스냅샷 패턴 유지로 회피
- **모든 aggregate에 일괄 적용** — Relationship만 우선, 나머지는 필요 시 점진 확대
- **별도 Read DB 도입** — 싱글 플레이어 게임에 불필요
- **비동기 Event Bus** — 동일 프로세스 내 동기 dispatch로 충분
- **GameClock 이벤트 영속화** — `WatchChanged` 2160개/년은 노이즈
- **Event upcasting 인프라 선제 구축** — 도메인 안정화 이후 필요 시 도입

### 우선순위

1. ~~**(P0)** Relationship 이벤트 반환 Gap 해결~~ ✅ 완료 (2026-03-01 확인)
2. **(P1)** EventLog port trait 설계 + InMemoryEventLog 구현 ← **현재 다음 우선순위**
3. **(P1)** EventEnvelope에 GameTime 메타데이터 첨부
4. **(P2)** Lightweight CQRS Projection trait 정의
5. **(P3)** Relationship 히스토리 Projection → LLM context 풍부화
6. **(P4)** Phase 5 Bevy 통합 시 ECS read model

---

## 참조

- `crates/wuxia-core/src/shared/event.rs` — DomainEvent 래퍼 구조 (21개 이벤트, 5 도메인)
- `crates/wuxia-core/src/relationship/event.rs` — RelationshipEvent 6종 정의 (HostilityChanged 제거됨)
- `crates/wuxia-core/src/relationship/types.rs` — Relationship aggregate (2축 모델: 호감도 -100~+100, 신뢰도 0~100)
- `crates/wuxia-core/src/relationship/effect.rs` — `apply_conversation_effect()` 도메인 서비스 + `DeltaSource` 출처 추적
- `crates/wuxia-core/src/relationship/sentiment.rs` — 2-stage hybrid 감정 판정 (`ExtremeAnchorSet`, `TurnCounter`, `SentimentJudgment`)
- `crates/wuxia-core/src/memory/port.rs` — MemoryRepository 포트 패턴 (EventLog 설계 레퍼런스)
- `crates/wuxia-core/src/relationship/port.rs` — RelationshipRepository 포트 패턴
- `crates/wuxia-core/src/application/training.rs` — Application Service 이벤트 흐름
- `crates/wuxia-core/src/application/time_character.rs` — 이벤트 기반 cross-domain 조율
- `crates/wuxia-llm/src/prompt/types.rs` — View 패턴 (`MemoryView`, `RelationshipView`)
- `crates/wuxia-llm/src/conversation/context.rs` — ContextProvider read-side 패턴
- `crates/wuxia-llm/src/conversation/session.rs` — ChatSession (감정 파이프라인 → 관계 이벤트 체인)
- `crates/wuxia-llm/src/sentiment/pipeline.rs` — SentimentPipeline (극단 앵커 + 정기 LLM 판정)

---

## 변경 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|----------|
| v1.0.0 | 2026-02-25 | 초기 ADR 작성. ES/CQRS 적용 검토 및 Hybrid Event Log 전략 결정 |
| v2.0.0 | 2026-03-01 | Relationship 이벤트 반환 Gap 해결 반영 (Step 1 완료). 테스트 수 갱신 (757개). 실제 `update_axis()` 구현 반영. 참조 목록 확장 (감정 판정 파이프라인 추가) |
| v2.1.0 | 2026-03-01 | 관계 도메인 3축→2축 리팩터링 반영. 적대도(hostility) 축 완전 제거, 호감도 범위 0~100→-100~+100. HostilityChanged 이벤트 제거 (7종→6종, 전체 22→21). `update_axis()` 축별 클램핑(clamp_affinity/clamp_trust) 반영. Relationship mutation 4개 메서드로 정리. |
