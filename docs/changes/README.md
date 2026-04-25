# 변경 추적 (개인 리뷰용)

본인 코드 리뷰·복귀 시 빠른 스캔을 위한 PR 단위 카드 모음. 정본은 git log + 각종
설계 문서이며, 여기는 **"무엇이 왜 바뀌었나"** 를 짧게 압축해 둔 사이드 인덱스다.

## 작성 규약

- 한 Step = 한 파일. 머지 커밋이 끝나는 시점에 추가.
- 헤더 5섹션: 핵심 / 결정 / 변경 파일 / 검증 / 잔여
- 길이 1페이지 (≤80줄) 목표 — 스크롤 없이 보이게
- 커밋 해시·PR 번호·테스트 카운트 박아서 회귀 시 즉시 추적 가능하도록
- 관련 설계 문서로 링크만, 본문 복사 금지 (drift 방지)
- 의사코드/다이어그램 포함 시 **헤더에 commit 해시로 version pin** 명시 (예: `*(commit abc123 시점 코드 기준)*`)
- "사용자 승인 결정"은 **bullet 형식**으로, 설계 문서 §번호 참조 시 정확히 표기 (예: `**§17.1**: ...`)
- 신규 테스트 수는 **파일별 분해**로 표기 (예: "`memory.rs` 9 / `ranker.rs` 7 / `sqlite_memory_test.rs` 6")

## Memory System 진척

| Step | 상태 | 카드 |
|---|---|---|
| Step A — Foundation | ✅ 2026-04 | [`memory-step-a.md`](memory-step-a.md) |
| Step B — Injection & Framing | ✅ 2026-04 | [`memory-step-b.md`](memory-step-b.md) |
| Step C — Telling & Rumor | 미구현 | — |
| Step D — Consolidation & World Overlay | 미구현 | — |
| Step E — Mind Studio 편집 | 미구현 | — |
| Step F — Pull / 백그라운드 Rumor 틱 | 미구현 | — |

설계 정본: [`../memory/03-implementation-design.md`](../memory/03-implementation-design.md) §13

## 기타 진척

(Memory 외 변경은 별도 인덱스 추가 시 여기로)
