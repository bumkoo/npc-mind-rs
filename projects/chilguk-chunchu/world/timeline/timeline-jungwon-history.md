---
id: timeline-jungwon-history
kind: history
name: 칠국춘추 270년사
aliases:
  - 중원사
  - main-history
  - 270년 연표
summary: |
  원년부터 현재(270년차)까지 270년의 메인 시간선. 5 시대 + Phase 5a의 6 핵심 사건을
  하나의 인과 흐름으로 묶음. 게임 시작 시점 NPC 대사·메인 퀘스트 단서·서사 분기점이
  모두 본 timeline의 사건들에서 인과를 끌어옴.
tags: [wuxia, timeline, history, main]
references:
  - era-founding              # 원년~50년차 (창세전쟁)
  - era-prosperity            # 50~120년차 (태평성세)
  - era-turning               # 120~200년차 (사파 형성·자치 운동·혈교 잔당 발견)
  - era-decline               # 200~240년차 (태무제 시기·2차 혈교 침공)
  - era-fall-of-empire        # 240~270년차 (붕괴기·6국 형성)
extras:
  game_role: 메인 시간선 — 모든 NPC 대사·메인 퀘스트 단서·서사 분기점의 인과 출발점
  player_relevance: 5
  narrative_seed_count: 6     # Phase 5a 6 사건 (1 founding + 5 fall-of-empire)
---

## 개요
원년(year_relative=−270)부터 현재(270년차, year_relative=0)까지 270년을 5 시대로 정형화한
메인 시간선. 5 시대가 각자의 핵심 사건(`key_events`)을 가지며, 본 timeline은 5 era를
references로 묶어 두 단계 합성(timeline → era → event)으로 6 핵심 사건의 인과 흐름을
드러낸다.

`events_in` view 메서드가 era.key_events를 평면화 — Phase 5a의 6 시드 사건이
era-founding(1) + era-fall-of-empire(5)에 분포하므로 `events_in(repo)` 결과 = 6.
`events_during(era-fall-of-empire, repo)` = 5. `causal_chain(event-bloody-night, repo)`은
bloody-night의 related_events([blood-disappearance, hwasan-fall, six-states-independence])
를 BFS로 traversal하여 timeline 경계 안의 인과 사슬을 합성.

## Era 변천
- **건국기 → 전성기 (50년차)**: 혈교 격퇴 + 대진 건국의 안정화 → 경제 성장기로 전환.
- **전성기 → 변곡기 (120년차)**: 태평성세의 표면 안에서 자란 갈등 씨앗(상방·자치·정파 균열)이
  변곡기에서 본격 발현 시작.
- **변곡기 → 쇠퇴기 (200년차)**: 80년 전 혈교 잔당 첫 발견 + 자치 운동 누적 + 태무제 즉위
  (237년차) 직전 균열 누적.
- **쇠퇴기 → 붕괴기 (240년차)**: 2차 혈교 침공이 boundary 사건 — 격퇴되나 황실 정당성
  결정적 손상. 경계 정책 §3.3에 따라 사건의 era_id는 era-fall-of-empire로 매핑되어
  붕괴기 시작 트리거로 분류됨.

## 핵심 인과 사슬
Phase 5a 6 시드 사건의 인과 흐름:

```
empire-founding (-270, era-founding)
  └─→ bloody-cult-rebellion-2nd (-30, era-fall-of-empire)
       │  (270년 전 격퇴된 혈교의 부활)
       └─→ blood-disappearance (-12, era-fall-of-empire)
            │  (혈교 정보 인물 선택적 제거 — 사전 정지 작업)
            ├─→ bloody-night (-10, era-fall-of-empire)  ←┐
            │    │  (황실 와해)                            │  양방향
            │    └─→ hwasan-fall (-10, era-fall-of-empire) ←┘
            │         (화산파 멸문)
            └─→ six-states-independence (-7, era-fall-of-empire)
                 (6국 형성 → 게임 시작 시점 정치 지도)
```

bloody-night ↔ hwasan-fall 양방향 시연: `causal_chain(bloody-night)`이 BFS로 두 사건을
모두 포함하되 `visited` set으로 cycle 방지 (timeline 경계 안에서 한 번씩만 방문).

## 게임 시점에서의 활용
- **NPC 대사**: "선조의 영광"(era-founding), "270년 전 약속"(era-founding), "30년 전 혈교
  전쟁"(era-fall-of-empire boundary), "10년 전 변란"(era-fall-of-empire) 모두 본 timeline의
  era·event 참조.
- **메인 퀘스트 단서**: 조고 추적은 `causal_chain(bloody-night)` 결과에 등장하는 사건들 —
  blood-disappearance(사전 정지) + hwasan-fall(직접 결과) + six-states-independence(후속 결과).
- **서사 분기점**: player의 출생·트라우마·현재 시점이 모두 era-fall-of-empire 안. 본
  timeline의 마지막 era가 player의 personal arc.
- **view 메서드 활용**:
  - `eras_in(repo)` → 5 era 일람 (NPC 대사용 시간 컨테이너)
  - `events_in(repo)` → 6 사건 평면화 (메인 퀘스트 단서 모음)
  - `events_during(era_id, repo)` → 시대별 사건 (NPC가 특정 시대를 회상할 때)
  - `causal_chain(seed, repo)` → 인과 사슬 추적 (조고 추적 등 메인 퀘스트)
