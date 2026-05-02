---
id: era-{slug}
kind: founding                   # founding | prosperity | turning | decline | fall
name: 시대 명
aliases: []                      # 별호·옛 이름 (예: "6국 분열기"·"240-270년차")
summary: |
  1-2 문장 요약 — 본 시대의 핵심 흐름.
tags: [wuxia, era]
temporal:
  start_year_relative: ~         # 270년차 기준 inclusive (예: -270 = 원년)
  end_year_relative: ~           # 270년차 기준 exclusive (예: -220 = 다음 시대 시작)
  notes: ~                       # 자유 메모 (boundary 정책 적용 의도, 시대 종료 트리거 등)
key_events: []                   # events.id 배열 — Era → Event 단방향 외래키 (시간순 권장)
extras:
  game_role: ~                   # 본 시대의 서사적 역할 (한 줄)
  player_relevance: ~            # 1-5 (★)
---

## 개요
산문 — 본 시대의 핵심 흐름.

## 핵심 트리거
산문 — 직전 시대에서 본 시대로 넘어가는 트리거.

## 결과
산문 — 본 시대가 만든 결과·다음 시대로의 이행.

## 핵심 인물
- (npc 미등록) 인물명: 본 시대에서의 역할
- npc-id 이름: 본 시대에서의 역할

## 게임에서의 역할
- 메인/사이드 서사 분기점들의 시간 컨테이너
- player와의 직간접 연결
