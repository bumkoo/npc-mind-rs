---
id: event-{slug}
kind: betrayal                   # betrayal | war | founding | disaster | ritual | discovery
category: historical             # historical | scheduled | legendary
name: 사건 명
aliases: []                      # 별호·옛 이름 (예: "붉은 밤"·"10년 전 변란")
summary: |
  1-2 문장 요약 — 사건의 핵심을 한 호흡에 잡는다.
tags: [wuxia, event]
temporal:
  year: ~                        # 자유 텍스트 (예: "10년 전 (260년차)", "원년", "수년 후")
  year_relative: ~               # 270년차 기준 절대 연도 (예: -10). 정렬·필터에 사용.
  duration: ~                    # 자유 텍스트 (예: "사흘 밤", "수년")
  notes: ~                       # 자유 메모 (Phase 5b 마이그레이션 의도 등)
era_id: ~                        # Phase 5b Era 외래키 자리 — Phase 5a엔 텍스트만
participants:
  people: []                     # NPC ids — Phase 2 외래키 검증 활성
  groups: []                     # Group ids — Phase 1 외래키 검증 활성
  places: []                     # Place ids — Phase 3 외래키 검증 활성
related_events: []               # 다른 Event ids — 자체 도메인. Phase 5a 활성 (cycle 검증은 비활성)
extras:
  trigger: ~                     # 발단 한 줄
  outcome: ~                     # 결과 한 줄
  game_role: ~                   # 게임 내 서사적 역할
  player_relevance: ~            # 1-5 (★)
---

## 개요
한두 단락 — 사건의 본질·맥락 종론.

## 발단
산문 — 사건 직전 상황 + 트리거.

## 전개
산문 — 사건 진행. 핵심 인물·장소 명시.

## 결과
산문 — 영토·관계·세력 변화 + 후속 인과.

## 핵심 인물
- npc-id 이름: 본 사건에서의 역할
- npc-id 이름: 본 사건에서의 역할
- (npc 미등록) 이름: 정형화 예정 (Phase N+)

## 게임에서의 역할
- 메인/사이드 서사 분기점 등급
- player와의 직간접 연결
- 후속 사건의 인과적 트리거 여부
