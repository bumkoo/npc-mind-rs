---
id: place-{slug}
layer: settlement
kind: nation                 # nation | autonomous-zone | city | sect
name: 장소명
aliases: []                  # 별호·옛 이름 (예: 자금성 ↔ 황궁, 낙양 ↔ 옛 황도)
summary: |
  1-2 문장 요약. 이 정치체가 게임 세계에서 어떤 위치를 차지하는가.
tags: [wuxia, place, settlement]
extras:
  capital: ~                 # 수도/본거지 명 (city/sect 본거지에 해당)
  capital_hanja: ~
  polity: ~                  # 왕조·자치령·공화국 등 자유 텍스트
  population_note: ~
  ki_concentration: 보통      # 무협 특화 — 기 농도 (희박|보통|농후)
  controlling_group: ~       # Phase 1 Group 외래키 (sect는 필수, nation은 선택)
spatial:
  parent_place: ~            # 영토상 부모 (도시→국가, 문파→국가). 최상위면 null.
  relative_position: ~       # schematic 위치 — Phase 4 Atlas에서 활용 ("center"/"west"/...)
  bordering_places: []       # 수평 인접 정치체
  geography_refs: []         # 어느 자연 지형 위에 layered (Phase 4·5+ Era overlay 기반)
---

## 개요
한두 단락 — 정치체의 현 상황 + 칠국 중 위상.

## 통치
산문 — 권력 구조. 명목 원수·실권자·관료·문파 영향력 등.

## 핵심 NPC
- (npc-id) — 역할 한 줄

## 핵심 갈등
산문 — 내부·외부 갈등 축. 게임 메인 스레드와의 연결.

## 플레이어가 방문할 이유
산문 — 메인/사이드 퀘스트 단서, 핵심 NPC 접촉, 자원 등.

## 전사(前史)
산문 — 옵션. 정치체의 형성 배경·과거 사건. (Phase 5+ Era overlay 시 정형화 예정)
