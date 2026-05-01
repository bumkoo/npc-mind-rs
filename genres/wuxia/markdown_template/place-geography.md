---
id: place-{slug}
layer: geography
kind: mountain-range         # mountain-range | coast | jungle | grassland | desert | forest | river | lake | landmark
name: 지형명
aliases: []                  # 별호·옛 이름 (자연 지형도 별칭 흔함)
summary: |
  1-2 문장 요약. 지형이 게임 세계에서 어떤 의미를 갖는가.
tags: [wuxia, place, geography]
extras:
  terrain_type: ~            # 자유 텍스트 (kind를 더 구체화). 예: "고산 산악"
  climate: ~                 # 기후 라벨/산문 (예: "고산 한랭, 겨울 폭설")
  hazards: []                # 위험 요소 (예: ["눈사태","협곡 안개","마수"])
  signature_features: []     # 대표 지점·랜드마크 (예: ["망주봉","십리협"])
spatial:
  parent_place: ~            # 광역 자연 영역의 일부면 부모 ID. 보통 null.
  relative_position: ~       # schematic 위치 — Phase 4 Atlas 활용
  bordering_places: []       # 수평 인접 (정치체·다른 자연 지형)
  # geography_refs는 settlement에서만 의미 — geography는 비움
---

## 개요
한두 단락 — 지형의 의미·외부와의 단절·플레이어가 마주칠 분위기.

## 지형·기후
산문 — 지질학적 특징·계절 변화.

## 위험·서식 생물
산문 — 무협 결의 마수·산적·자연 재해 등.

## 인접 정치체
- (place-id) — 인접 관계 한 줄

## 자원·산물
산문 — 약초·광물·특산물. 무협 경제·당가 같은 약재 산업과의 연결.

## 플레이어가 방문할 이유
산문 — 통로·비급·은거 고수·자원 채취 등.
