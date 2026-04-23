# EventHandler 카탈로그

> **Deep-Dive #2.** [`system-overview.md` §7](system-overview.md) 의 2번 항목.
> [`dispatch-v2-internals.md`](dispatch-v2-internals.md) 가 "dispatcher가 어떻게 동작하는가" 를 다뤘다면, 이 문서는 **어떤 핸들러들이 등록되어 있으며 각각 무엇을 먹고 무엇을 뱉는가** 를 카탈로그 형태로 정리한다.
>
> 대상 핸들러: 8 Transactional Agent + 8 Inline Handler (Projection 3 + Memory Ingestion 5) = **총 16개**.
> 근거 파일 위치: `src/application/command/agents/*.rs` · `src/application/command/*_handler.rs` · `src/application/command/projection_handlers.rs`.

---

## 1. 읽는 법

각 핸들러는 **카드 형식**으로 기술한다:

```
이름 (파일 경로)
  priority  : DeliveryMode 안의 priority 상수
  mode      : Transactional { can_emit_follow_up } / Inline
  interest  : 구독하는 EventKind
  입력      : 이벤트 payload에서 읽는 핵심 필드
  일        : 수행하는 도메인 로직 요약
  HandlerShared: 세팅하는 필드 (Transactional만)
  follow-ups : 발행하는 이벤트 (Transactional만)
  부수효과   : 외부 저장소 쓰기 (Inline만)
  실패      : 반환 가능한 HandlerError variant
  Step      : 해당 Memory Step (B0/C1/C2/C3/D)
```

**Transactional vs Inline 차이 복습** (`dispatch-v2-internals.md` §5 상세):

| | Transactional | Inline |
|---|---|---|
| 실행 시점 | 커밋 이전, BFS 루프 안 | 커밋 직후, 같은 스레드 |
| 실패 시 | 커맨드 전체 롤백 | `tracing::warn` 로그만 |
| follow-up 발행 | O (`can_emit_follow_up = true` 시) | X |
| `HandlerShared` 쓰기 | O | 관례상 X (read-only) |
| 외부 저장소 쓰기 | X (`repo`는 read-only) | O (MemoryStore/RumorStore) |

---

## 2. 실행 순서 한눈에

`priority.rs` 의 상수값에 따라 `register_*` 시점에 한 번 정렬되고, 이후 매 이벤트마다 순회는 이 정렬 순서로 일어난다.

### 2.1 Transactional (작은 값 먼저, 커맨드 안에서 BFS)

```
5   SceneAgent             — SCENE_START
10  EmotionAgent           — EMOTION_APPRAISAL
15  StimulusAgent          — STIMULUS_APPLICATION
20  GuideAgent             — GUIDE_GENERATION
25  WorldOverlayAgent      — WORLD_OVERLAY
30  RelationshipAgent      — RELATIONSHIP_UPDATE
35  InformationAgent       — INFORMATION_TELLING
40  RumorAgent             — RUMOR_SPREAD
(90 AUDIT — 예약)
```

### 2.2 Inline (작은 값 먼저, 커밋 직후 동기)

```
10  EmotionProjectionHandler         — EMOTION_PROJECTION
20  RelationshipProjectionHandler    — RELATIONSHIP_PROJECTION
30  SceneProjectionHandler           — SCENE_PROJECTION
40  TellingIngestionHandler          — MEMORY_INGESTION
40  RumorDistributionHandler         — MEMORY_INGESTION (같은 slot 공유)
45  WorldOverlayHandler              — WORLD_OVERLAY_INGESTION
50  RelationshipMemoryHandler        — RELATIONSHIP_MEMORY
60  SceneConsolidationHandler        — SCENE_CONSOLIDATION
```

---

## 3. Transactional Agents (8)

### 3.1 SceneAgent (`agents/scene_agent.rs`)

```
priority      : priority::transactional::SCENE_START      (5)
mode          : Transactional { can_emit_follow_up: true }
interest      : Kinds([SceneStartRequested])
입력          : npc_id, partner_id, significance, initial_focus_id, prebuilt_scene
일            : prebuilt_scene을 HandlerShared.scene에 세팅.
               initial_focus가 있으면 그 focus.to_situation() 을 만들어
               AppraisalEngine을 즉시 호출 — 초기 감정을 미리 산출.
HandlerShared : scene, (초기 focus 있을 때) emotion_state, relationship
follow-ups    : SceneStarted (항상),
               EmotionAppraised (초기 focus 있을 때만)
실패          : NpcNotFound, RelationshipNotFound, InvalidInput
Step          : C1 (Scene 시작)
```

**포인트**: `Command::StartScene` 이 dispatcher의 `build_initial_event` 에서 **이미 `Scene` 도메인 객체를 빌드해 payload에 넣어둔 상태**로 이 핸들러에 도착한다 (`dispatch-v2-internals.md` §4.1). 그래서 SceneAgent는 focus 빌드에 실패할 걱정 없이 그냥 `ctx.shared.scene = Some(prebuilt_scene)` 만 하면 됨.

**초기 focus의 즉시 appraise** 덕분에 `StartScene` 한 번으로 `SceneStarted → EmotionAppraised → (depth 2로) GuideGenerated` 체인이 같은 트랜잭션에 포함된다.

### 3.2 EmotionAgent (`agents/emotion_agent.rs`)

```
priority      : priority::transactional::EMOTION_APPRAISAL (10)
mode          : Transactional { can_emit_follow_up: true }
interest      : Kinds([AppraiseRequested])
입력          : npc_id, partner_id, situation (Situation 도메인 타입)
일            : AppraisalEngine.appraise(personality, situation, modifiers)
               → HEXACO × OCC × relationship-modifier 기반 22개 기초 감정 계산
               + compound 결합 (Gratification/Remorse/Gratitude/Anger)
HandlerShared : emotion_state, relationship
follow-ups    : EmotionAppraised { npc_id, partner_id, situation_description,
                                   dominant, mood, emotion_snapshot }
실패          : NpcNotFound, RelationshipNotFound
Step          : C1
```

**포인트**: `Command::Appraise` 직접 호출 + `SceneAgent`의 cascade 둘 다에서 트리거된다. 후자의 경우 Scene 내부에서 이미 검증된 NPC/관계이므로 실패 확률이 낮지만, `Appraise` 커맨드 직접 호출 시에는 repo lookup 실패가 흔한 400/404 원인.

### 3.3 StimulusAgent (`agents/stimulus_agent.rs`)

```
priority      : priority::transactional::STIMULUS_APPLICATION (15)
mode          : Transactional { can_emit_follow_up: true }
interest      : Kinds([StimulusApplyRequested])
입력          : npc_id, partner_id, pad: (p, a, d), situation_description
일            : 1) stimulus_processor.apply(pad, &emotion_state)
                 → 관성(1-intensity, 최소 0.30) × 흡수율 × PAD dot
                 → 감정 변동 + intensity 재계산
               2) Scene 활성일 때 scene.check_trigger(&new_state)
                 대기 focus 중 조건 충족된 것 있으면 transition_beat:
                   a. update_beat_relationship (관계 갱신)
                   b. active_focus 교체 + 새 focus로 appraise
                   c. merge_from_beat — 이전+새 감정 max 병합 (0.2 미만 소멸)
HandlerShared : emotion_state (stimulated 또는 merged), relationship, scene
follow-ups    : StimulusApplied (항상),
               BeatTransitioned { partner_id, ... } (전환 시)
실패          : NpcNotFound, RelationshipNotFound, EmotionStateNotFound
Step          : C1
```

**포인트**: 이 핸들러 하나가 **두 가지 성격의 일**을 한다 — 일상적 PAD 자극(한 턴 감정 변화) + Beat 전환(서사 단계 이동). Beat 전환 여부는 `scene.check_trigger` 가 결정하는데, Scene의 focus 데이터가 `FocusTrigger::Conditions` 를 갖고 있으면 감정 조건 충족 시 자동 전환.

**B4 S3 Option A 이력**: `BeatTransitioned` payload에 `partner_id` 가 **필수 필드**로 포함된다. multi-scene 환경에서 어떤 파트너와의 Beat 전환인지 라우팅 정확성을 위해 추가된 것 (B4 S3 회귀 가드 테스트 `dispatch_v2_test.rs`).

### 3.4 GuideAgent (`agents/guide_agent.rs`)

```
priority      : priority::transactional::GUIDE_GENERATION (20)
mode          : Transactional { can_emit_follow_up: true }
interest      : Kinds([EmotionAppraised, StimulusApplied, GuideRequested])
입력          : npc_id, partner_id (+ ctx.shared.emotion_state 또는 repo fallback)
일            : emotion_state (shared 우선, 없으면 repo.get_emotion_state) 를 기반으로
               ActingGuide::build(personality, emotion_state, relationship) 호출.
               22개 감정 → 가이드 dict 매핑 (docs/guide/guide-mapping-table.md).
HandlerShared : guide
follow-ups    : GuideGenerated { npc_id, partner_id }
실패          : NpcNotFound, EmotionStateNotFound
                (partner_id의 관계 없으면 neutral로 fallback — 실패 안 함)
Step          : C1
```

**포인트**: 같은 커맨드 안에서 **앞선 핸들러가 HandlerShared에 쓴 emotion_state를 읽는** 대표 사례. 만약 SceneAgent → EmotionAgent → GuideAgent 체인이라면 GuideAgent는 repo를 안 건드리고 scratchpad만 참조하므로 빠르다. 단독 `Command::GenerateGuide` 호출 시에는 scratchpad가 비어있어 repo에서 읽음.

Interest가 세 종류인 이유: EmotionAppraised / StimulusApplied는 cascade로 자동 가이드 갱신, GuideRequested는 수동 요청. 결과 이벤트는 모두 `GuideGenerated` 하나로 단일화.

### 3.5 RelationshipAgent (`agents/relationship_agent.rs`)

```
priority      : priority::transactional::RELATIONSHIP_UPDATE (30)
mode          : Transactional { can_emit_follow_up: true }
interest      : Kinds([BeatTransitioned, RelationshipUpdateRequested, DialogueEndRequested])
입력          : 이벤트 종류별 다름 (아래 3분기)
일            :
  ▸ BeatTransitioned:         merged_emotion으로 after_dialogue 갱신
                              cause = SceneInteraction { scene_id }
  ▸ RelationshipUpdateRequested: shared_emotion으로 after_dialogue 갱신
                                 cause = Unspecified
  ▸ DialogueEndRequested:     after_dialogue + clear_emotion_for + clear_scene 신호
HandlerShared : relationship, (DialogueEnd시) clear_emotion_for, clear_scene
follow-ups    : RelationshipUpdated (3종 모두),
               + EmotionCleared + SceneEnded (DialogueEnd만)
실패          : RelationshipNotFound, EmotionStateNotFound
Step          : D (cause variant별 RelationshipMemory 트리거의 근거)
```

**포인트**: 이 프로젝트에서 **한 Agent가 세 가지 이벤트를 처리하는 유일한 사례**. 세 분기가 "관계가 갱신되는 상황"이라는 공통점으로 묶여있지만, 각기 다른 맥락(Beat 안 · 명시적 갱신 · 대화 종료)을 표현.

**DialogueEnd의 follow-up 3종**은 이 프로젝트에서 **한 핸들러가 가장 많은 follow-up을 발행하는 사례**. `HandlerShared.clear_emotion_for` / `clear_scene` 플래그가 같이 세팅되어 `apply_shared_to_repository` 가 save + clear 순서로 repo에 반영.

### 3.6 WorldOverlayAgent (`agents/world_overlay_agent.rs`)

```
priority      : priority::transactional::WORLD_OVERLAY (25)
mode          : Transactional { can_emit_follow_up: true }
interest      : Kinds([ApplyWorldEventRequested])
입력          : world_id, topic, fact, significance, witnesses
일            : Requested → Occurred 1:1 변환 (passthrough).
               도메인 로직 없음 — 실제 MemoryStore 쓰기는
               Inline WorldOverlayHandler에서.
HandlerShared : — (사용 안 함)
follow-ups    : WorldEventOccurred { 동일 payload 전부 }
실패          : 없음 (입력 검증은 dispatcher build_initial_event가 담당)
Step          : D
```

**포인트**: 왜 그냥 passthrough인가? 대칭성 유지 — 다른 커맨드가 `*Requested → *Handled/*Occurred` 패턴을 따르므로 WorldOverlay도 같은 경로로 두어 구독자 관점 일관성을 확보. 실제 비즈니스 로직(Canonical MemoryEntry 생성 + topic supersede) 은 Inline 단에서 처리하는데, 이는 외부 저장소 쓰기이므로 Transactional에 두면 실패 시 전체 커맨드 롤백이 과도하기 때문.

### 3.7 RumorAgent (`agents/rumor_agent.rs`)

```
priority      : priority::transactional::RUMOR_SPREAD (40)
mode          : Transactional { can_emit_follow_up: true }
interest      : Kinds([SeedRumorRequested, SpreadRumorRequested])
입력          :
  ▸ Seed  : pending_id, topic?, seed_content?, reach, origin
  ▸ Spread: rumor_id, extra_recipients
일            :
  ▸ Seed  : Rumor 애그리거트 생성 (Canonical/Orphan/Forecast 분류),
           자체 AtomicU64 카운터로 rumor_id 발급, RumorStore.save
  ▸ Spread: store에서 rumor 로드, 홉 추가 (recipient dedup + cycle check)
           hop_index 증가, RumorStore.save
HandlerShared : — (자체 카운터 관리)
follow-ups    : RumorSeeded / RumorSpread
실패          : InvalidInput (orphan without seed_content, unknown rumor),
               Infrastructure (RumorStore 호출 실패)
Step          : C3
```

**포인트**: `Command::SeedRumor` 의 초기 aggregate_id는 `"pending-<seq>"` 임시 값이지만 (dispatcher가 발급), 이 핸들러가 **실제 rumor_id** 를 배정하고 `RumorSeeded.rumor_id` 에 담아 follow-up으로 돌려준다. 이후 구독자·Inline `RumorDistributionHandler` 가 진짜 id를 본다.

**rumor_id 전략**: Transactional 핸들러가 상태를 갖는 드문 사례. `next_rumor_id: AtomicU64` 를 RumorAgent 자체가 소유 (dispatcher의 `command_seq`와 별개). Step C3 사후 리뷰 M1 에서 결정된 설계.

### 3.8 InformationAgent (`agents/information_agent.rs`)

```
priority      : priority::transactional::INFORMATION_TELLING (35)
mode          : Transactional { can_emit_follow_up: true }
interest      : Kinds([TellInformationRequested])
입력          : speaker, listeners, overhearers, claim,
               stated_confidence, origin_chain_in, topic
일            : listeners ∪ overhearers 중복 제거 → 각 청자마다 1개 InformationTold 발행.
               listener_role = Direct(listeners) / Overhearer(overhearers).
               실제 MemoryEntry 저장은 Inline TellingIngestionHandler.
HandlerShared : — (사용 안 함)
follow-ups    : InformationTold × N (청자 수)
               각 InformationTold의 aggregate_id = listener (화자 아님)
실패          : 없음 (입력 검증은 dispatcher)
Step          : C2
```

**포인트**: 청자별 follow-up에 **aggregate_id를 청자로 설정** 하는 설계 (Step C2 B5 결정). dispatcher의 `commit_staging_buffer` 가 이벤트별 `aggregate_key().npc_id_hint()` 로 id를 찍으므로, `EventStore.get_events(listener_id)` 가 정확히 그 청자가 들은 정보들을 돌려준다.

**팬아웃 배수**: listeners 5명 + overhearers 3명 → 한 커맨드가 8개 InformationTold를 만든다. `MAX_EVENTS_PER_COMMAND = 20` 한계를 고려할 때 팬아웃이 큰 방송 시나리오는 주의.

---

## 4. Inline Handlers (8)

### 4.1 ~ 4.3 Projection 3종 (`projection_handlers.rs`)

한 파일에 세 개 wrapper가 같이 산다. 셋 다 유사한 구조.

**EmotionProjectionHandler**
```
priority      : priority::inline::EMOTION_PROJECTION (10)
mode          : Inline
interest      : Kinds([EmotionAppraised, StimulusApplied, EmotionCleared])
부수효과       : EmotionProjection(Arc<Mutex<...>>).apply(event)
                → NPC별 mood / dominant emotion 캐시 갱신
실패          : Infrastructure("emotion projection mutex poisoned")
Step          : B0 foundation
```

**RelationshipProjectionHandler**
```
priority      : priority::inline::RELATIONSHIP_PROJECTION (20)
mode          : Inline
interest      : Kinds([RelationshipUpdated])
부수효과       : RelationshipProjection.apply(event)
                → (owner_id, target_id) 쌍의 after 값(trust/closeness/respect) 캐시
실패          : Infrastructure("relationship projection mutex poisoned")
```

**SceneProjectionHandler**
```
priority      : priority::inline::SCENE_PROJECTION (30)
mode          : Inline
interest      : Kinds([SceneStarted, BeatTransitioned, SceneEnded])
부수효과       : SceneProjection.apply(event)
                → 활성 Scene의 active_focus_id 추적, 종료 시 clear
실패          : Infrastructure("scene projection mutex poisoned")
```

**포인트**: 이 셋은 **B안 B2 시절 v1 `Projection` trait을 v2로 끌어오기 위한 wrapper**로 만들어졌다. Projection 자체는 `Arc<Mutex<T>>` 로 공유돼서 여러 쿼리 경로가 동시에 `.get_*()` 호출 가능. `projection()` getter로 외부 핸들을 얻을 수 있고, Mind Studio의 상태 스냅샷 API가 이걸 통해 직접 조회한다.

**공통 실패 모드**: Mutex poison만 에러로 올린다. 실제 비즈니스 로직은 `Projection::apply()` 안에 있고 거기서는 에러를 반환하지 않는다 (이벤트가 관심사 아니면 조용히 무시).

### 4.4 TellingIngestionHandler (`telling_ingestion_handler.rs`)

```
priority      : priority::inline::MEMORY_INGESTION (40)
mode          : Inline
interest      : Kinds([InformationTold])
부수효과       : MemoryStore.index(MemoryEntry) — 청자 관점 Personal scope 엔트리 저장
               Source 결정:
                 origin_chain_in.len() == 1 → Heard (첫 전달)
                 ≥ 2                       → Rumor (다단계 전파 의심)
               Confidence 계산:
                 stated_confidence × normalized_trust(listener→speaker)
                 관계 없으면 trust = 0.5 로 가정
               Topic: 이벤트 payload의 topic 그대로 전파
실패          : MemoryStore.index 실패 → warn 로그만, 커맨드 계속
Step          : C2
```

**포인트**: 한 커맨드(TellInformation)가 InformationAgent에서 **청자 수만큼 이벤트를 만들고**, 이 핸들러가 **각 이벤트마다 MemoryEntry 하나씩** 저장하므로 실제 DB 쓰기는 N회. store I/O가 실패해도 로그만 남기는 이유는 Inline 계약 — 저장소 장애로 대화 커맨드 자체를 실패시키지 않기 위함.

**Source 분기의 의미**: Alice가 직접 목격한 사실을 Bob에게 말하면 origin_chain = [Alice], len=1 → Bob의 기억은 Heard. Bob이 Carol에게 다시 전하면 origin_chain = [Alice, Bob], len=2 → Carol의 기억은 Rumor. Memory Ranker가 Source 우선순위로 정렬할 때 이 차이가 작동.

### 4.5 RumorDistributionHandler (`rumor_distribution_handler.rs`)

```
priority      : priority::inline::MEMORY_INGESTION (40)     — Telling과 같은 슬롯
mode          : Inline
interest      : Kinds([RumorSpread])
부수효과       : MemoryStore.index(MemoryEntry) — 수신자별 Personal scope Rumor 엔트리
               Content 해소 (3-tier):
                 1. rumor의 최신 RumorDistortion.content_version
                 2. (1 없으면) topic 기준 Canonical MemoryEntry 조회
                 3. (2 없으면) rumor.seed_content
                 4. 전부 없으면 "[내용 없음]" 플레이스홀더
               Confidence: 1.0 × RUMOR_HOP_CONFIDENCE_DECAY^hop_index
                          (RUMOR_MIN_CONFIDENCE 하한)
실패          : MemoryStore / RumorStore 호출 실패 → warn 로그
Step          : C3
```

**포인트**: `RumorSpread` 이벤트는 한 번에 여러 수신자를 담고 있고, 이 핸들러가 각 수신자별로 MemoryEntry를 만든다. 따라서 한 `Command::SpreadRumor` 가 들어오면 "RumorAgent가 1개 RumorSpread 발행 → 이 핸들러가 수신자 N명에게 N개 MemoryEntry 저장" 패턴.

**Content 3-tier 해소**가 핵심 설계 — 소문은 "누가 전달하느냐"가 아니라 "그 topic에 대한 현재 세계의 canonical 사실이 뭐냐"가 중요하기 때문. Canonical이 갱신되면(WorldOverlayHandler) 이후의 소문 전파가 그 갱신을 반영.

### 4.6 WorldOverlayHandler (`world_overlay_handler.rs`)

```
priority      : priority::inline::WORLD_OVERLAY_INGESTION (45)
mode          : Inline
interest      : Kinds([WorldEventOccurred])
부수효과       : 1) MemoryStore.index(MemoryEntry)
                 scope = World(world_id)
                 provenance = Seeded
                 topic = 이벤트의 topic
               2) topic이 있을 때만: mark_superseded 호출
                 같은 topic의 **기존 Canonical 1건만** 덮어씀.
                 다른 NPC들의 Personal scope Heard/Rumor 엔트리는 보존 (리뷰 B1)
               3) topic 없으면 supersede 생략 (독립 사건으로 취급)
실패          : get_canonical_by_topic / mark_superseded / index 실패 모두 warn 로그
Step          : D
```

**포인트**: "세계의 canonical 사실 갱신" 이 이 핸들러의 존재 이유. "용이 화산을 뿜었다" 같은 이벤트가 들어오면 같은 topic ("dragon_eruption" 등) 의 기존 canonical 엔트리가 superseded 되고 새 엔트리가 canonical이 된다.

**Heard/Rumor를 supersede하지 않는 이유**: 개별 NPC가 기억하는 "누구에게 들은 소문" 은 그 NPC 관점의 과거 사실이므로 지워져선 안 된다. canonical만 최신화하고, NPC들이 "아, 내가 들은 소문이 사실이 아니었구나"를 알아가는 건 서사 흐름이 결정할 일.

### 4.7 RelationshipMemoryHandler (`relationship_memory_handler.rs`)

```
priority      : priority::inline::RELATIONSHIP_MEMORY (50)
mode          : Inline
interest      : Kinds([RelationshipUpdated])
부수효과       : MemoryStore.index(MemoryEntry) — owner 관점 Personal scope 엔트리
               cause variant별 source/topic/content 분기:
                 SceneInteraction{scene_id} → topic="scene:{a}:{b}", content 포함 주도 축 라벨
                 InformationTold{...}       → topic="info:{subject}"
                 WorldEventOverlay{...}     → topic="world:{event}"
                 Rumor{...}                 → topic="rumor:{rumor_id}"
                 Unspecified               → topic=None
               Threshold: 3축 max(|Δtrust|, |Δcloseness|, |Δrespect|) ≥ 0.05
                         미만이면 저장 생략 (미세변동 스킵)
               Content 형식: "{인간 설명} [{주도축} Δ={값:.2}]"
실패          : MemoryStore.index 실패 → warn 로그
Step          : D
```

**포인트**: `RelationshipUpdated.cause` enum의 각 variant가 **topic prefix 체계**로 매핑되어 MemoryStore 쿼리 시 관계 변화 원인별로 필터 가능. 예: "이 NPC에게 영향 준 WorldEvent 기억만" → `topic LIKE 'world:%'`.

**Threshold 스킵의 의미**: Beat 전환이나 대화 종료 시 관계가 0.01 단위로 살짝씩 변하는 경우, 이걸 전부 기록하면 Memory가 노이즈로 가득 찬다. 0.05 이상 움직일 때만 "의미 있는 변화" 로 간주해 기록.

### 4.8 SceneConsolidationHandler (`scene_consolidation_handler.rs`)

```
priority      : priority::inline::SCENE_CONSOLIDATION (60)    — 맨 마지막
mode          : Inline
interest      : Kinds([SceneEnded])
부수효과       : 참여 NPC별로 (a, b 각각):
               1) MemoryStore.search — 해당 NPC Personal scope 내
                  Layer A (DialogueTurn / BeatTransition) 엔트리 수집
               2) 휴리스틱 요약 생성:
                  "{N}턴 간 대화 요약: {첫content 120자} ... {끝content 120자}"
               3) MemoryStore.index — Layer B SceneSummary 엔트리 저장
                  topic = "scene:{a}:{b}" (a≤b 정규화)
               4) Layer A 엔트리들을 mark_consolidated(summary_id)
실패          : search 실패 시 해당 NPC skip (반쪽 consolidation 방지)
               index / mark_consolidated 실패 → warn 로그
Step          : D
```

**포인트**: Scene 종료 시점에 **여러 턴의 미세 기억을 한 덩어리 요약으로 승격** 하는 핸들러. 현재는 휴리스틱(첫·끝 발화 + 턴 수) 기반이라 요약 품질은 제한적 — Step F에서 LLM 기반 consolidator로 교체 예정.

**Per-NPC 분리**: 같은 Scene이라도 무백과 교룡이 서로 다른 요약을 갖는다 (각자의 Personal Memory). topic은 `scene:{a}:{b}` 로 정규화 (a≤b) 되어 두 NPC가 같은 topic으로 서로의 기억을 교차 조회 가능.

**마지막 priority인 이유**: 다른 모든 Memory 쓰기(Telling/Rumor/World/Relationship)가 끝난 뒤에 돌아야, 이 Scene에서 만들어진 Layer A 엔트리를 놓치지 않고 전부 흡수할 수 있다.

---

## 5. 매트릭스 요약

### 5.1 Transactional — 입력 → 출력 매트릭스

| 에이전트 | 구독 EventKind | 쓰는 HandlerShared | 발행 follow-ups |
|---|---|---|---|
| SceneAgent | SceneStartRequested | scene, emotion_state*, relationship* | SceneStarted + EmotionAppraised* |
| EmotionAgent | AppraiseRequested | emotion_state, relationship | EmotionAppraised |
| StimulusAgent | StimulusApplyRequested | emotion_state, relationship, scene | StimulusApplied + BeatTransitioned* |
| GuideAgent | EmotionAppraised / StimulusApplied / GuideRequested | guide | GuideGenerated |
| WorldOverlayAgent | ApplyWorldEventRequested | — | WorldEventOccurred |
| RelationshipAgent | BeatTransitioned / RelationshipUpdateRequested / DialogueEndRequested | relationship, clear_emotion_for*, clear_scene* | RelationshipUpdated + EmotionCleared* + SceneEnded* |
| InformationAgent | TellInformationRequested | — | InformationTold × N |
| RumorAgent | SeedRumorRequested / SpreadRumorRequested | — (자체 카운터) | RumorSeeded / RumorSpread |

`*` = 조건부 / optional.

### 5.2 Inline — 구독 → 부수효과 매트릭스

| 핸들러 | 구독 EventKind | 외부 쓰기 대상 |
|---|---|---|
| EmotionProjectionHandler | EmotionAppraised / StimulusApplied / EmotionCleared | `Arc<Mutex<EmotionProjection>>` 인메모리 캐시 |
| RelationshipProjectionHandler | RelationshipUpdated | `Arc<Mutex<RelationshipProjection>>` |
| SceneProjectionHandler | SceneStarted / BeatTransitioned / SceneEnded | `Arc<Mutex<SceneProjection>>` |
| TellingIngestionHandler | InformationTold | `MemoryStore.index` (Personal Heard/Rumor) |
| RumorDistributionHandler | RumorSpread | `MemoryStore.index` (Personal Rumor, 수신자별 N) |
| WorldOverlayHandler | WorldEventOccurred | `MemoryStore.index` (World Canonical) + `mark_superseded` |
| RelationshipMemoryHandler | RelationshipUpdated | `MemoryStore.index` (Personal RelationshipChange) |
| SceneConsolidationHandler | SceneEnded | `MemoryStore.search` + `index` (Layer B) + `mark_consolidated` |

### 5.3 이벤트 → 소비자 역매트릭스

"어떤 이벤트가 어디로 흐르는가" 관점. 같은 이벤트가 Transactional과 Inline 양쪽에서 소비될 수 있다.

| EventKind | Transactional 소비 | Inline 소비 |
|---|---|---|
| `EmotionAppraised` | GuideAgent | EmotionProjection |
| `StimulusApplied` | GuideAgent | EmotionProjection |
| `BeatTransitioned` | RelationshipAgent | SceneProjection |
| `RelationshipUpdated` | — | RelationshipProjection, **RelationshipMemory** |
| `SceneStarted` | — | SceneProjection |
| `SceneEnded` | — | SceneProjection, **SceneConsolidation** |
| `EmotionCleared` | — | EmotionProjection |
| `GuideGenerated` | — | — (구독자 없음, fanout만) |
| `DialogueTurnCompleted` | — | — (MemoryAgent가 broadcast로 받음) |
| `InformationTold` | — | **TellingIngestion** |
| `RumorSeeded` | — | — (broadcast만) |
| `RumorSpread` | — | **RumorDistribution** |
| `WorldEventOccurred` | — | **WorldOverlay** |

`RelationshipUpdated` 가 **Inline에서 두 핸들러에 소비** 되는 것이 인상적. Projection(읽기 뷰)은 최신 값 캐시만 하지만, Memory 쪽은 cause variant를 보고 MemoryEntry를 기록 — 서로 관심사가 다르다.

---

## 6. 공통 패턴과 함정

### 6.1 핸들러 작성 공통 패턴

1. **`handle` 첫머리에서 payload 패턴 매칭 → else arm**: 내 관심사 아닌 이벤트가 들어오면 `Ok(HandlerResult::default())`. `interest()` 필터로 이미 걸러지지만 방어적으로 한 번 더.
2. **repo는 read-only로만**: 쓰기는 `ctx.shared` 에만. 실수로 `RefCell` 같은 우회로 뚫으면 `apply_shared_to_repository` 와 충돌.
3. **follow-up은 payload만 채우고 id/seq는 0**: `DomainEvent::new(0, npc_id, 0, payload)`. 실제 id/seq는 dispatcher의 `commit_staging_buffer` 가 할당.
4. **HandlerError variant 선택**: NPC/관계 부재 → `*NotFound` (404), 입력 검증 실패 → `InvalidInput` (400), store 장애 → `Infrastructure` (500).
5. **테스트는 `HandlerTestHarness`**: dispatcher 없이 Agent 하나만 격리 테스트. `harness.with_npc(...).with_relationship(...).dispatch(&agent, event)`.

### 6.2 자주 마주치는 함정

| 함정 | 증상 | 올바른 방식 |
|---|---|---|
| `follow_up_events` 에 과거 이벤트 재삽입 | depth 가드 폭발, 중복 실행 | 새 이벤트 페이로드로 새 DomainEvent 만들기 |
| Transactional에서 store 직접 쓰기 | 실패 시 EventStore와 일관성 깨짐 | store 쓰기는 Inline으로 분리 (WorldOverlay 패턴 참고) |
| Inline에서 `follow_up_events` 돌려주기 | dispatcher가 무시, 본인만 혼란 | `Ok(HandlerResult::default())` 반환 |
| 같은 커맨드에서 emotion save + clear 동시 | clear가 이김 | save만 필요하면 `clear_emotion_for = None` 유지 |
| partner 없는 Scene에서 BeatTransitioned | multi-scene 라우팅 오작동 | `BeatTransitioned.partner_id` 반드시 채우기 (Option A) |
| InformationTold follow-up의 aggregate_id를 speaker로 | 청자 기반 쿼리 실패 | listener로 설정 (Step C2 B5 결정) |
| Inline 핸들러의 심각한 invariant 위반을 warn으로만 처리 | 사일런트 데이터 손실 | `Infrastructure(static_str)` 로 에스컬레이트, dispatcher가 로그 레벨 결정 |

### 6.3 확장 체크리스트 (새 Agent 추가 시)

[`dispatch-v2-internals.md §9.2`](dispatch-v2-internals.md) 의 7단계 + 이 카탈로그에서 추가로 고려할 것:

- 같은 이벤트를 구독하는 기존 핸들러와 **priority 충돌** 없는지 확인 (`priority.rs` + invariants 테스트 추가)
- `HandlerShared` 신규 필드 필요하면 PR 리뷰 필수 + `apply_shared_to_repository` 대응
- 외부 저장소 쓰기가 필요하면 Transactional 금지 → Inline으로 분리
- cascade depth 고려 (`MAX_CASCADE_DEPTH = 4`)
- 팬아웃 배수 고려 (`MAX_EVENTS_PER_COMMAND = 20`)

---

## 7. 등록 매트릭스 (Builder 기준)

어떤 builder를 호출했는가에 따라 **실제로 dispatcher에 등록되는 핸들러**가 달라진다. `dispatch-v2-internals.md §8` 에 builder 종류는 있으니 여기서는 핸들러 기준으로 정리:

| 핸들러 | `default` | `with_memory` | `with_memory_full` | `with_rumor` |
|---|---|---|---|---|
| SceneAgent | ✓ | | | |
| EmotionAgent | ✓ | | | |
| StimulusAgent | ✓ | | | |
| GuideAgent | ✓ | | | |
| RelationshipAgent | ✓ | | | |
| InformationAgent | ✓ | | | |
| WorldOverlayAgent | ✓ | | | |
| RumorAgent | | | | ✓ |
| EmotionProjectionHandler | ✓ | | | |
| RelationshipProjectionHandler | ✓ | | | |
| SceneProjectionHandler | ✓ | | | |
| TellingIngestionHandler | | ✓ | ✓ | |
| RumorDistributionHandler | | | | ✓ |
| WorldOverlayHandler | | | ✓ | |
| RelationshipMemoryHandler | | | ✓ | |
| SceneConsolidationHandler | | | ✓ | |

**Mind Studio 부팅 기본 조합**: `default + with_memory_full + with_rumor` (embed feature on).

**경고 재확인**: `with_memory` 와 `with_memory_full` 을 둘 다 호출하면 `TellingIngestionHandler` 가 **두 번 등록**되어 같은 InformationTold에 두 번 MemoryEntry 쓰기가 일어난다. 현재 방어 장치 없음 — 항상 한 쪽만 선택.

---

## 8. 관련 문서

- [`system-overview.md`](system-overview.md) — 전체 구조 개관
- [`dispatch-v2-internals.md`](dispatch-v2-internals.md) — dispatcher 수준 (Deep-Dive #1)
- [`system-design-eventbus-cqrs.md`](system-design-eventbus-cqrs.md) — EventBus · CQRS · Event Sourcing 통합 설계
- [`../memory/03-implementation-design.md`](../memory/03-implementation-design.md) — MemoryStore·MemoryEntry·MemoryRanker 내부 설계 (Step A/B)
- 다음 Deep-Dive 후보: **#5 Scene · Beat · Focus Trigger 엔진** — 상태기계와 `check_trigger` / `merge_from_beat` 내부
