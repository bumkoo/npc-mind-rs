---
id: group-{slug}
kind: dynasty-court        # dynasty-court | clan | sect-religious | mendicant-order | alliance | covert-band | tribe-confederacy | merchants-council
name: 그룹명
aliases: []                 # 별호·옛 이름 (예: 무림맹 ↔ 구파일방)
summary: |
  1-2 문장 요약. 게임에서 이 그룹이 무엇을 하는지.
tags: [wuxia, group]
temporal:
  founded_at: ~             # "원년 (270년 전)" "현재 황조 즉위 시" 등 자유 텍스트
  dissolved_at: ~
  status: active            # active | declining | dissolved | dormant
  notes: ~
members: []                 # [{ person_id, display_name, role, note }]
headquarters: ~             # Place ID 텍스트 (Phase 3 외래키 활성)
parent_group: ~             # 수직 포함 (예: 십상시 → 대진 황실)
allied_groups: []           # 수평 우호
rival_groups: []            # 수평 적대
extras:
  alignment: neutral        # wuxia 진영 표준화 (orthodox|heterodox|demonic|outland|imperial|neutral)
---

## 개요
한두 단락 — 그룹의 현 상황.

## 권력 구조
산문 — 누가 실권자, 명목 원수, 부수적 권력 등.

## 외부 갈등
산문 — 다른 그룹과의 관계.

## 핵심 갈등
산문 — 그룹 내부의 결단·운명적 모순.

## 시간 변화
산문 — temporal.notes 확장. 시기별 변동.

## 게임에서의 역할
산문 — 이 그룹이 게임 진행에 어떻게 등장하는가.
