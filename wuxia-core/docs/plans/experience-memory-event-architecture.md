# 경험-기억-이벤트 통합 아키텍처 (Experience-Memory-Event Unified Architecture)

> **버전**: v3.1 | **최종 수정**: 2026-03-11T03:00:00+09:00  
> **역할**: 기억·관계·성장·심리 도메인을 관통하는 핵심 설계 원칙과 이벤트 구조 정의  
> 📎 도메인 전체 구조: [domain-analysis.md](domain-analysis.md)  
> 📎 기존 이벤트 소싱 검토: [adr-event-sourcing-cqrs-review.md](adr-event-sourcing-cqrs-review.md)  
> 📎 기억 도메인 상세: [memory-domain-analysis.md](memory-domain-analysis.md)  
> 📎 관계 도메인 상세: [relationship-mechanic.md](relationship-mechanic.md)  
> 📎 NPC 대화 아키텍처: [npc-conversation-memory-architecture.md](npc-conversation-memory-architecture.md)  
> 📎 감정 판정 설계: [embedding-sentiment-plan.md](embedding-sentiment-plan.md)  

---

## 1. 핵심 통찰

### 1.1 기존 설계의 한계

기존 12개 도메인은 동등한 형제로 설계되었다. 기억과 관계는 별도 도메인으로 서로 "맥락 제공"하는 보조적 관계였다. 도메인 간 이벤트 통신은 Application Service(TimeCharacterService)의 수동 매칭이 유일한 방식이었다.

### 1.2 새로운 관점 — "경험이 모든 것을 만든다"

```
  경험(=기억)이 쌓여서 현재의 모든 상태를 만든다.

  Bond(관계)도, 성장도, 심리도, 피로도, 전부
  기억의 누적이 만들어낸 현재 스냅샷이다.
```

---

## 2. 합의된 원칙 (10개)

### 원칙 1: 경험 = 기억 = 이벤트 (유일한 진실)

경험이 발생하면 그것이 곧 기억이 되고, 동시에 이벤트로 각 도메인에 전달된다.

```
  대화를 했다 → 기억이 생긴다 → 관계가 변하고, 감정이 변한다
  수련을 했다 → 기억이 생긴다 → 숙련도가 오르고, 피로가 쌓인다
  전투를 했다 → 기억이 생긴다 → 관계가 변하고, 부상이 생긴다
```

### 원칙 2: 모든 현재 상태는 기억의 누적 결과

```
  현재 상태 = Σ(경험 × 계수)

  Bond(나→소연) 신뢰 73     ← 과거 경험들의 누적
  화산검법 숙련도 47          ← 과거 경험들의 누적
  자신감 0.7                 ← 과거 경험들의 누적
  피로 65                    ← 과거 경험들의 누적
```

### 원칙 3: ExperienceEvent와 DomainEvent는 별도 역할

ExperienceEvent는 큐에 들어가는 유일한 것. DomainEvent는 큐에 들어가지 않고 ProcessingContext에 담기는 처리 부산물. 두 타입이 분리되어 있으므로 컴파일러가 실수를 방지.

```rust
  // 큐에 들어가는 것 — 경험 (새로 정의)
  pub enum ExperienceEvent {
      Training { ... },
      Conversation { ... },
      Combat { ... },
      Observation { ... },          // 감정 판정 결과도 여기
      ConversationSummarized { ... }, // 요약 완료도 여기
      ...
  }

  // ProcessingContext에 담기는 것 — 처리 부산물 (기존 DomainEvent 그대로 사용)
  DomainEvent::Character(CharacterEvent::FatigueChanged { ... })
  DomainEvent::Character(CharacterEvent::Injured { ... })
  DomainEvent::Growth(GrowthEvent::StatImproved { ... })
  DomainEvent::Growth(GrowthEvent::FortuneTriggered { ... })
  DomainEvent::Relationship(RelationshipEvent::AffinityChanged { ... })
  DomainEvent::Relationship(RelationshipEvent::LevelChanged { ... })
  DomainEvent::Memory(MemoryEvent::MemoryStored { ... })
  ...
```

기존 DomainEvent enum은 타입/코드 변경 없이 그대로 유지. 역할만 재정의: 큐에 넣는 용도가 아니라 ProcessingContext에 담기는 처리 부산물.

### 원칙 4: 각 도메인은 구독자, 자기 로직 + 필요한 계수로 상태 변경

```
  같은 "수련" 경험을:
    성장 도메인 → 숙련도 계산 (계수: 피로, 장소, 사제Bond, 경지)
    캐릭터 도메인 → 피로 계산 (계수: 나이, 현재 피로)
    Bond 도메인 → 관계 갱신 (계수: HEXACO, 기존 Bond)
    심리 도메인 → 감정 계산 (계수: HEXACO, PAD, 가치관)
    서사 도메인 → 퀘스트 확인 (계수: 퀘스트 조건)
```

### 원칙 5: HEXACO는 여러 계수 중 하나일 뿐

HEXACO는 유일한 초기값(기억으로 만들어지지 않음). 피로, PAD, 가치관, Bond값 등은 다른 도메인의 현재값이며, 각 도메인이 자기 로직에 필요한 계수를 읽기 전용으로 참조한다.

### 원칙 6: 경험 이벤트는 원시 데이터만, 서사 요약은 별도 이벤트로

경험 이벤트에 LLM 요약을 포함하지 않는다. 요약은 비동기 태스크로 처리한 뒤, 완료되면 별도의 경험 이벤트로 큐에 넣는다.

```
  대화 종료 시 흐름:
  ─────────────────────────────────────
  finish() → ActionResult {
      events: [Conversation { raw_dialogue, turns }],
      tasks: [Summarize { raw_dialogue }],
  }

  → Conversation 이벤트 → 큐 → 핸들러:
      기억: 벡터DB에 원시 데이터 저장 (summary=null)
      심리, 서사 등 각자 처리

  → Summarize 태스크 → spawn(CTX3) → 3초 후 완료

  → ConversationSummarized 이벤트 → 큐 → 핸들러:
      기억: 벡터DB 업데이트 (summary, vector 채움)
      서사: 요약 기반 퀘스트 조건 재확인 가능
```

이점:
- 요약 모델 교체 가능, 배치 처리 가능
- finish()가 즉시 반환 (3초 안 기다림)
- 모든 결과가 이벤트로 큐를 통과 — 예외 없음

### 원칙 7: 핸들러는 고정 순서로 실행, 변경은 즉시 반영

~~이전: "스냅샷 기준, 변경은 다음 경험부터"~~

스냅샷 방식 폐기 이유:
- 구현이 복잡 (모든 값 복사, 일괄 반영)
- 2차 이벤트와 충돌 (피로 81 돌파 → Exhausted를 즉시 못 감지)
- 부자연스러움 (수련하면서 피곤해지는 건 동시에 일어남)

```
  핸들러 실행 순서 (고정):
  ─────────────────────────────────────
  ① 캐릭터 (피로/부상 — 물리적 제약이 먼저)
  ② 성장   (숙련도/능력치 — 피로 반영한 효율 계산)
  ③ Bond   (관계 — 성장 결과도 반영)
  ④ 심리   (감정/기분 — 모든 변화를 느낌)
  ⑤ 서사   (퀘스트/기연 — 모든 상태 확인 후 트리거)
  ⑥ 기억   (벡터DB 저장 — 마지막에 기록)

  "몸이 먼저, 마음이 다음, 이야기가 마지막"
```

핸들러 간 연쇄 반응은 ProcessingContext로 전달. 앞 핸들러의 SideEffect를 뒤 핸들러가 참조.

```
  Experience::Training 처리:
  ─────────────────────────────────────
  ① 캐릭터: 피로 81 → ctx.add(ExhaustionReached)
  ② 성장: ctx에서 ExhaustionReached 확인 → 효율 감소 적용
          숙련도 50 돌파 → ctx.add(MasteryBreakthrough)
  ③ Bond: 명경 신뢰 +2
  ④ 심리: ctx에서 ExhaustionReached → 지침
  ⑤ 서사: ctx에서 MasteryBreakthrough → 기연 조건 확인!
  ⑥ 기억: 벡터DB 저장
```

순서가 고정이므로 같은 입력이면 항상 같은 결과 (결정론적).

### 원칙 8: 이벤트 큐에는 ExperienceEvent만, 빌 때까지 돌린다

~~이전: "ExperienceEvent + DomainEvent 섞어서 큐에"~~

큐에 들어가는 것은 ExperienceEvent뿐. 핸들러 간 연쇄 반응은 ProcessingContext(DomainEvent)로 같은 처리 라운드 내에서 해결. 비동기 태스크 완료 시에도 ExperienceEvent로 큐에 넣음.

```
  큐: [Training] → 꺼냄 → 핸들러 6개 순서대로 (ctx로 공유) → 끝
  큐: [] ← 비면 끝

  비동기 결과 도착 시:
  큐: [Observation] → 꺼냄 → 핸들러 6개 → 끝
  큐: [ConversationSummarized] → 꺼냄 → 핸들러 6개 → 끝
```

### 원칙 9: EventEnvelope 흡수

ExperienceEvent 하나가 이벤트(도메인 전달) + 기억(벡터DB 저장) + 이벤트 로그(감사/디버깅) 세 역할을 통합. 별도 EventEnvelope 불필요.

### 원칙 10: ExperienceEvent(신규)와 DomainEvent(기존)의 역할 분리

```
  ExperienceEvent (신규):               DomainEvent (기존 그대로):
  ─────────────────                     ─────────────────
  큐에 들어감         ✅                큐에 안 들어감      ❌
  벡터DB에 저장       ✅                벡터DB에 안 저장    ❌
  이벤트 로그에 기록   ✅                ctx에 담김          ✅
  핸들러가 구독       ✅                뒤 핸들러가 참조    ✅
  비동기 결과도 이 타입 ✅               로그/UI에 사용      ✅
```

기존 DomainEvent는 타입/코드 변경 없이 그대로 사용. 기존 테스트도 깨지지 않음.

---

## 3. 경험이 만들어내는 두 가지 변화

```
  경험 발생
       │
       ├── Bond 변화 (상대적, 나 ↔ 대상)
       │     사람, 무공, 조직, 장소, 신념, 사물, 동물
       │
       └── Self 변화 (절대적, 나 자신)
             성장(숙련도), 심리(감정/기분), 캐릭터(피로/부상)
```

Bond와 Self 구분 기준: **"성격에 따라 달라지는가?"** 달라지지 않으면 Self, 달라지면 Bond.

---

## 4. 도메인 재분류

```
  ┌─ 초기값 (태어날 때) ─────────────────────┐
  │  캐릭터: 이름, 나이, 성별                  │
  │  기질: HEXACO (거의 불변)                  │
  └────────────────────────────────────────┘
                     │
  ┌─ 기억(경험) = 이벤트 ─────────────────────┐
  │  모든 경험이 여기 쌓인다                    │
  │  불변. 한번 생긴 기억은 사라지지 않는다      │
  └──────────────────┬─────────────────────┘
                     │
  ┌─ 현재 상태 = 구독자 도메인들 ──────────────┐
  │  Bond, 성장, 심리, 캐릭터                   │
  └────────────────────────────────────────┘
                     │
  ┌─ 경험을 만드는 환경 ──────────────────────┐
  │  세계관, 공간, 시간, 사물, 서사, 전투/경제   │
  └────────────────────────────────────────┘
```

---

## 5. Action trait — 행동 추상화

### 5.1 설계 배경

게임 루프가 각 행동의 세부 로직을 알면 비대해진다. GDD 마이크로 루프 5가지 행동(대화/탐색/수련/전투/거래)을 일반화.

### 5.2 Action trait

```rust
  trait Action {
      fn tick(&mut self, input: &str) -> ActionResult;
      fn finish(&mut self) -> ActionResult;
      fn is_finished(&self) -> bool;
  }

  struct ActionResult {
      output: String,                  // 화면에 보여줄 내용
      events: Vec<ExperienceEvent>,    // 큐에 넣을 경험 이벤트
      tasks: Vec<AsyncTask>,           // spawn할 비동기 작업
  }
```

### 5.3 Action별 tick 의미

| Action | tick 1회 | 반복 | 종료 조건 |
|--------|---------|------|----------|
| ConversationAction | 대화 1턴 | 5~30회 | /quit 또는 ForceEnd |
| TrainingAction | 수련 1시간대 | 1~6회 | 목표 시간 도달 |
| CombatAction | 전투 1라운드 | 승패까지 | 승/패/도주 |
| TradeAction | 거래 1단계 | 1~3회 | 확정/취소 |
| ExploreAction | 이동 1구간 | 1~수회 | 도착 또는 조우 |

### 5.4 게임 루프 (Action을 모름)

```
  loop {
      // ① 비동기 결과 수신 — 전부 ExperienceEvent로 큐에
      for task in pending_tasks.drain_finished() {
          queue.push(task.result);  // 무조건 ExperienceEvent
      }

      // ② 이벤트 큐 소진 — ProcessingContext로 핸들러 간 공유
      while let Some(event) = queue.poll() {
          let mut ctx = ProcessingContext::new();
          for handler in &mut handlers {
              let result = handler.handle_event(&event, &ctx);
              ctx.extend(result.side_effects);
              for task in result.tasks {
                  pending_tasks.push(spawn(task));
              }
          }
      }

      // ③ 플레이어 입력
      let input = read_input();

      // ④ 현재 Action 한 틱
      let result = current_action.tick(&input);

      // ⑤ Action 반환물 처리
      for event in result.events { queue.push(event); }
      for task in result.tasks { pending_tasks.push(spawn(task)); }
      println!("{}", result.output);

      // ⑥ Action 끝나면 finish → 이벤트/태스크 처리
      if current_action.is_finished() {
          let final_result = current_action.finish();
          for event in final_result.events { queue.push(event); }
          for task in final_result.tasks { pending_tasks.push(spawn(task)); }
          current_action = select_next_action();
      }
  }
```

게임 루프는 Action이 뭔지 모름. 이벤트 큐 처리는 한 곳에서만. 비동기 결과도 큐에 넣기만.

---

## 6. EventHandler trait와 ProcessingContext

### 6.1 핸들러 trait

```rust
  pub trait EventHandler {
      fn handle_event(
          &mut self,
          event: &ExperienceEvent,
          ctx: &ProcessingContext,
      ) -> HandlerResult;
  }

  pub struct HandlerResult {
      pub side_effects: Vec<DomainEvent>,   // ctx에 담김 (기존 DomainEvent 그대로)
      pub tasks: Vec<AsyncTask>,            // pending_tasks에 추가
  }
```

### 6.2 ProcessingContext

하나의 ExperienceEvent를 처리하는 동안 핸들러 간 공유되는 부산물(DomainEvent). 이벤트 처리가 끝나면 버림.

```
  Experience::Training 처리:

  ctx = ProcessingContext::new()  ← 비어있음

  ① 캐릭터: 피로 81
     → ctx.add(ExhaustionReached)

  ② 성장: ctx.has(ExhaustionReached)? → 효율 감소
     숙련도 50 돌파!
     → ctx.add(MasteryBreakthrough)

  ③ Bond: 명경 신뢰 +2
     → ctx.add(AffinityChanged)

  ④ 심리: ctx.has(ExhaustionReached)? → 지침 감정

  ⑤ 서사: ctx.has(MasteryBreakthrough)? → 기연!
     → ctx.add(QuestTriggered)

  ⑥ 기억: 벡터DB 저장
     → tasks: [Summarize { ... }]  (비동기)

  처리 끝. ctx 버림. 큐에는 아무것도 안 넣음.
```

### 6.3 EventBus — Port & Adapter

```
  wuxia-core:  trait EventBus + InMemoryEventBus (VecDeque, 테스트)
  wuxia-game:  BevyEventBridge (Bevy EventWriter/EventReader, 프로덕션)
```

Bevy 내장 이벤트 시스템 활용. 별도 라이브러리 불필요.

---

## 7. 호감도 변경 — 극단 트리거 단일 경로

### 7.1 호감도 변경 경로

```
  즉각 반응 = 극단 트리거 (대화 중, 극단 표현 감지 시)
  점진 변화 = 성찰 reflexion (향후, 기억이 쌓인 뒤)
```

삭제한 것:
- ~~12턴 정기 LLM 판정~~ → 성찰에서 처리
- ~~대화 종료 시 호감도 측정~~ → 요약 목적은 "기억 생성"이지 "호감도 측정"이 아님

### 7.2 극단 체크 규칙

- **트리거 대상**: 플레이어 대사 + NPC 대사 둘 다
- **이벤트 발행**: NPC 대사 나온 뒤, 둘 합쳐서 한 번만
- **쿨다운**: 같은 방향 6턴 (다른 방향은 즉시 허용)
- **판정 대상**: 대화 전체 히스토리 (CTX2에 전달)

### 7.3 ConversationAction.tick() 내부 흐름

```
  tick(user_input):
    ① 플레이어 입력 극단 체크 (7ms)
       → 기억만 해둠 (이벤트 아직 안 발행)

    ② CTX1: LLM 대사 생성 (3초)

    ③ NPC 대사 극단 체크 (7ms)

    ④ 둘 합쳐서 판단:
       triggered = player_extreme || npc_extreme
       쿨다운 중 && 같은 방향 → skipped
       triggered && !skipped → 비동기 태스크 생성 + 쿨다운 갱신

    ⑤ ActionResult {
        output: npc_text,
        events: [],
        tasks: [SentimentJudgment { dialogue_history }]  // 트리거 시
       }
```

### 7.4 감정 판정 비동기 흐름

```
  CTX1: 대화 생성 (매 턴, KV 캐시 누적)
  CTX2: 감정 판정 (극단 트리거 시, 별도 캐시, spawn)
  CTX3: 대화 요약 (대화 종료 시, 별도 캐시, spawn)
```

감정 판정 완료 → ExperienceEvent::Observation으로 큐에:

```
  pending_tasks → CTX2 완료!
  → ExperienceEvent::Observation {
      subject: 플레이어,
      target: 소연,
      what: "소연이 사부 죽음 언급에 극도로 분노",
      sentiment_delta: -9,
  }
  → 큐에 넣음 → 핸들러 처리 → Bond 호감도 변경 → 기억으로도 저장
```

LLM 감정 판정 ~3초 소요. 비결정론적 지연(빠르면 다음 턴, 느리면 1~2턴 뒤). 현실에서도 감정 변화 타이밍은 가변적이므로 자연스러움.

### 7.5 대화 종료 시

```
  ConversationAction.finish():
  → ActionResult {
      events: [Conversation { raw_dialogue, turns }],
      tasks: [Summarize { raw_dialogue }],
  }

  Conversation → 큐 → 핸들러:
    기억: 벡터DB 저장 (summary=null)
    심리: 감정 계산
    서사: 퀘스트 확인
    Bond: 할 일 없음 (극단 트리거로 이미 처리)

  ... 3초 후 ...

  ConversationSummarized → 큐 → 핸들러:
    기억: 벡터DB 업데이트 (summary, vector 채움)
    서사: 요약 기반 조건 재확인 가능

  대화 요약의 역할:
    "무슨 일이 있었는지" 기록 + "얼마나 중요했는지" 평가
    호감도 측정 안 함.
```

---

## 8. 비동기 처리 구조

### 8.1 모든 비동기 결과는 ExperienceEvent로 큐에

```
  비동기 태스크 완료 → ExperienceEvent로 변환 → 큐에 넣음
  게임 루프는 분기 없이 그냥 큐에 넣기만.
  핸들러가 알아서 처리.

  감정 판정 완료 → ExperienceEvent::Observation → 큐
  대화 요약 완료 → ExperienceEvent::ConversationSummarized → 큐
  패턴이 하나. 예외 없음.
```

### 8.2 이중 채널

```
  ┌──────────────────────────────────────────┐
  │  게임 루프                                │
  │                                           │
  │  pending_tasks ←── spawn 결과 대기         │
  │       │ 끝난 것만                          │
  │       ▼                                   │
  │  event_queue ←── ExperienceEvent만         │
  │       │ 핸들러 처리 (고정 순서)             │
  │       ▼                                   │
  │  handlers + ProcessingContext              │
  │    side_effects(DomainEvent) → ctx         │
  │    tasks → spawn → pending                │
  └──────────────────────────────────────────┘
```

### 8.3 동기 vs 비동기 판단

| 구분 | 처리 방식 | 예시 | 소요 시간 |
|------|----------|------|----------|
| 동기 | 핸들러에서 직접 | 수치 계산, Bond 갱신, 감정 계산 | ~0.001ms |
| 비동기 | spawn | LLM 감정 판정 (CTX2) | ~3초 |
| 비동기 | spawn | LLM 대화 요약 (CTX3) | ~3초 |
| 상황별 | 동기 or 비동기 | 벡터DB 검색 | ~30ms |

개발자가 코드 작성 시 명시적으로 결정. Bevy에서는 AsyncComputeTaskPool.spawn() 사용.

---

## 9. ExperienceEvent 구조

### 9.1 공통 헤더

```
  ExperienceHeader {
      experience_id, subject, time, location, importance
  }
```

### 9.2 경험 유형별 페이로드

| 경험 유형 | 주요 필드 | 비고 |
|----------|----------|------|
| Training | skill, method, mentor, companion, duration, intensity | |
| Combat | opponent, combat_type, result, injury, technique_used/faced | |
| Conversation | counterpart, turns, raw_dialogue | 원시 데이터만 |
| ConversationSummarized | experience_id, summary, importance | 비동기 완료 후 |
| Observation | target, what, sentiment_delta | 감정 판정 결과 포함 |
| Trade | counterpart, items, fairness | |
| Travel | destination, companion, duration | |
| Rescue | saved, danger, risk_taken | |
| Betrayal | betrayer, betrayed, betrayal_type | |
| Care | patient, caregiver, injury_type | |
| Gift | giver, receiver, item | |
| Rest | method, recovery | |
| TimePassage | duration, without_contact | |

---

## 10. 구독 매트릭스

```
  거의 모든 경험을 구독:
    Bond (12/12), 기억 (12/12), 심리 (7/12)

  특정 경험만 구독:
    성장 (3/12), 캐릭터 (5/12), 서사 (5/12), 경제 (2/12), 공간 (1/12)
```

---

## 11. 벡터DB (LanceDB) 저장

```
  | experience_id | subject | time | location | importance |
  | payload_type | payload_json | summary | vector |

  Conversation 저장 시: payload_json = 원시 대화, summary = null, vector = null
  ConversationSummarized 도착 시: summary, vector 업데이트
```

조회: 런타임은 LanceDB 벡터+SQL, 개발/디버깅은 DuckDB 연동 (JOIN 가능).

---

## 12. 기존 ADR 통합

| ADR 고민 | 해소 방식 |
|----------|----------|
| 이벤트 영속화 | 경험=기억이니까 벡터DB가 이벤트 로그 |
| 상태 복원 | 논의 필요. 특정 이벤트 시점으로 되돌려서 다른 선택 → 스토리 분기 트리 (§13 ❼ 참조) |
| 이벤트 메타데이터 | ExperienceHeader가 역할 (EventEnvelope 흡수) |
| 이벤트 버스 | EventHandler trait + EventBus port |

---

## 13. 미결정 사항

| # | 항목 | 상태 | 비고 |
|---|------|------|------|
| ❶ | 경험 이벤트에 related_entities 포함 여부 | 미정 | 구현하면서 결정 |
| ❷ | Bond 모델 구체적 구조 (축 정의) | 미정 | MVP는 Person↔Person, 점진적 확장 |
| ❸ | 무한 루프 방지 메커니즘 | 미정 | 최대 반복 횟수 또는 반복 감지 |
| ❹ | 성찰(reflexion) 점진적 호감도 변경 설계 | 미정 | 향후 심리 도메인 구현 시 |
| ❺ | 핸들러 실행 순서 세부 조정 | 미정 | 구현하면서 조정 |
| ❻ | importance 추정 로직 (코드 기반) | 미정 | turns, 극단 횟수, delta 기반 |
| ❼ | 상태 복원 — 이벤트 트리 기반 스토리 분기 | 논의 필요 | 특정 이벤트 시점으로 되돌려서 다른 선택 시 스토리가 달라지는 구조. 이벤트 히스토리를 트리로 관리하여 분기점 시각화. 스냅샷 + 이벤트 replay 조합 가능성 검토 필요 |

---

## 14. 구현 전략

### 14.1 점진적 전환

```
  Phase 1: ExperienceEvent enum 정의
           EventHandler trait + ProcessingContext
           InMemoryEventBus (VecDeque)
           Action trait + ActionResult

  Phase 2 (MVP):
           ConversationAction (극단 체크, 비동기 감정 판정)
           Bond 도메인 (Person 대상만, 기존 Relationship 확장)
           Character에 handle_event() 추가
           기존 TimeCharacterService → 핸들러로 이동

  Phase 3: TrainingAction
           Growth에 handle_event() 추가
           심리 도메인 핸들러 연결
           나머지 경험 유형 순차 추가
           성찰(reflexion) → 점진적 호감도 변경

  Phase 5 (Bevy 통합):
           BevyEventBridge 어댑터
           AsyncComputeTaskPool 연동
           handle_event() 로직 수정 없이 그대로
```

---

## 15. 변경 이력

| 버전 | 변경일시 | 변경 내역 |
|:---:|----------|-----------|
| v1.0 | 2026-03-10T22:30:00+09:00 | 초기 작성. 10개 원칙. ExperienceEvent 구조. 구독 매트릭스. 단일 큐. EventHandler trait. Port & Adapter. ADR 통합. 미결정 4건. |
| v2.0 | 2026-03-11T01:30:00+09:00 | 원칙 7 수정 (스냅샷→고정순서+즉시반영). 원칙 8 수정 (코레오그래피→큐하나). Action trait 신설. 호감도 단순화 (극단 트리거만). 비동기 이중 채널. |
| v3.0 | 2026-03-11T02:30:00+09:00 | **원칙 3 수정**: ExperienceEvent와 SideEffect 타입 분리 확정. 기존 DomainEvent→SideEffect 재정의. DomainEvent wrapper enum 폐기. **원칙 8 수정**: 큐에는 ExperienceEvent만 들어감. 핸들러 간 연쇄는 ProcessingContext(SideEffect)로 처리. **원칙 10 수정**: 두 타입의 역할 명확화 (컴파일러가 실수 방지). **원칙 6 수정**: 서사 요약 완료 시 ConversationSummarized 이벤트로 큐에 넣음. 모든 비동기 결과는 ExperienceEvent로 큐를 통과. **§6 HandlerResult 확정**: side_effects + tasks 반환. 핸들러도 비동기 태스크를 요청 가능. **§5 finish() 반환 구조 확정**: events + tasks로 이벤트와 비동기 작업을 동시 반환. **미결정 6건**: importance 추정 로직 추가. |
| v3.1 | 2026-03-11T03:00:00+09:00 | **원칙 3/10 수정**: SideEffect 별도 타입 폐기 → 기존 DomainEvent를 그대로 사용. 타입 변경 없이 역할만 재정의 (큐 용도가 아닌 ProcessingContext용). 기존 코드/테스트 변경 불필요. **§14 Phase 재배치**: Bond 도메인을 Phase 3→Phase 2(MVP)로 이동. TrainingAction/Growth를 Phase 2→Phase 3으로 이동. **§12 ADR 상태 복원**: "안 함"→"논의 필요"로 변경. **§13 미결정 ❼ 추가**: 이벤트 트리 기반 스토리 분기 — 특정 시점으로 되돌려서 다른 선택 시 스토리가 달라지는 구조, 분기점 트리 시각화. |
