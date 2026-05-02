---
id: atlas-{slug}
kind: continent                # continent | region | city-map
name: Atlas 명
aliases: []                    # 별호·옛 이름
summary: |
  1-2 문장 요약. 본 atlas가 어느 영역·시점을 포착하는가.
tags: [wuxia, atlas]
extras:
  era: ~                       # 잠정 시대 텍스트 (예: "현재 (270년차)"). Phase 5 Era에서 정형
  era_id: ~                    # Phase 5 Era 외래키 자리 — Phase 4엔 비움
  source_section: ~            # 원전 섹션 추적 (예: seven-nations.md §0.3)
extent:
  projection: schematic        # Phase 4: schematic만. Phase N+ cartesian/hex-grid
  width_units: ~               # schematic 격자 폭 (선택)
  height_units: ~              # schematic 격자 높이 (선택)
  unit: schematic              # Phase 4: 의미 없는 단위. Phase N+ km/li
references:                    # 본 atlas에 등장하는 Place들 — 좌상→우하 권장
  - place-{slug-1}
  - place-{slug-2}
---

## 개요
한두 단락 — atlas가 포착한 영역의 종론.

## (선택) 일람
표 또는 산문 — 본 atlas의 주요 항목 일람. nation 일람·도시 일람 등.

## 배치 다이어그램
```
ASCII 다이어그램을 여기에 byte-exact 보존.
박스 그림(┌─┐ │ └─┘) + 위치 라벨 자유.
```

## 자연 영역 분포
산문 — 자연 지형(geography) layer 매핑. settlement ↔ geography 합성 설명.

## 정치체 분포
산문 또는 표 — 어느 정치체가 어디에 자리.

## 주요 통로·연결
- 통로 항목 1 (settlement ↔ settlement / geography 경유 등)
- 통로 항목 2

## 전사(前史)
산문 — 옵션. Atlas 형성 배경. Phase 5+ Era overlay 시 정형 분리 예정.
