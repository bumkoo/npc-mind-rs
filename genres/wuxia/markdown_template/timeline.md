---
id: timeline-{slug}
kind: history                      # history | biographical
name: Timeline 명
aliases: []                        # 별호·옛 이름 (예: "270년 연표"·"main-history")
summary: |
  1-2 문장 요약 — 본 timeline이 묶는 시간 범위와 의도.
tags: [wuxia, timeline]
references:                        # eras.id 배열 — Era → Event 두 단계 합성
  - era-{slug-1}                   # 작성 순서 = 시간순 권장
  - era-{slug-2}
extras:
  game_role: ~                     # 본 timeline의 서사적 역할 (한 줄)
  player_relevance: ~              # 1-5 (★)
---

## 개요
산문 — 본 timeline이 포착한 시간 범위·의도.

## Era 변천
산문 — 시대들 사이의 전환·인과.

## 핵심 인과 사슬
산문 — 사건 단위 인과 흐름. `causal_chain(seed_event)` view 메서드의 narrative 보완.

## 게임 시점에서의 활용
- 본 timeline이 NPC 대사·메인 퀘스트 단서·서사 분기점에 어떻게 활용되는가
