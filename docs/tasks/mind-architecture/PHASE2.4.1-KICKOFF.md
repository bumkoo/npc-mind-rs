# Phase 2.4.1 KICKOFF — intensity weight 튜닝 (Claude Code 인계)

> 정본 인계 문서. 구현은 별도 worktree 세션에서 Claude Code가 수행.
> **단일 진실**: [task-rel-phase2.4.1-intensity-weight.md](task-rel-phase2.4.1-intensity-weight.md) (🟢 FROZEN). 계수·공식 동결 — 임의 변경 금지, 의문 시 중단·보고.

## 브랜치
`feat/phase2.4.1-intensity-weight` (worktree 분리)

## 구현 순서 (각 단계 후 `cargo test --lib` + 게이트, 단계별 커밋)

**① self/prospect** — `personality.rs` §3.1·3.2. 정서성(E)을 음수 분기로 이동. 본문만.
- `desirability_self_weight`: Joy=`base+X`, Distress=`base+E−A−Pru`(불변)
- `desirability_prospect_weight`: Hope=`base+X−Pru`, Fear=`base+E+fear`(불변)

**② confirmation 시그니처** — §3.3·§4. `desirability_confirmation_weight(_desirability: f32)` → `(is_fear_axis: bool)`.
- trait `AppraisalWeights`(`ports/personality.rs`) + 모든 impl/mock + 호출부(`appraisal/event.rs`) 동반.
- 호출부 매핑: `ProspectResult::FearUnrealized | FearConfirmed → true`, `HopeFulfilled | HopeUnfulfilled → false`.
- Fear축(Relief/FearsConfirmed)=`base+E−Pru`(현행 동일), Hope축(Satisfaction/Disappointment)=`base+X−Pru`(E→X).

**③ praiseworthiness facet 분해** — §3.4. 성실성평균 → `diligence×0.10 + perfectionism 비대칭`. org·prud 제외.
- perfectionism 계수: Pride −0.10 / Shame +0.20 / Admiration +0.15 / Reproach +0.20.
- sign은 `effect` 밖 적용(기존 modesty/gentleness 관례). modesty/gentleness 분기항 유지.

## 게이트 (spec §6)

1. `cargo check` + `cargo test --lib` — baseline **554P/0F** 유지(+ weight 직접검증 unit 기대값 갱신분 반영).
2. appraise-validation **S1~S4** ground truth 재측정 — Admiration/Reproach intensity 이동 박제 갱신.
3. **PAD 벤치 20케이스**(`docs/emotion/pad-anchor-score-matrix.md`): 변경 케이스만 재측정. 불변 케이스(Distress/Fear/Relief/FearsConfirmed/empathy/hostility/appealingness/stimulus) **편차 0** 확인 — 편차 나면 버그. ⚠ 기대값 변경은 **Bekay 승인 필요**, 임의 갱신 금지.
4. 통제군 fixture 재현: `load_scenario "appraise-test/prudence-intensity-fixtures/scenario.json"` → prudence hi/mid/lo Reproach weight 셋 다 동일(평탄화) + 정서성 인물 Joy 하락 확인.
5. 저-X(외향성) 인물 Hope/Satisfaction = `base+X−Pru`의 0.5 floor 접촉 점검.

## 검증 도구
`effect(w) = facet × w` 선형(2.4.0 trace 확인). MCP `appraise` trace로 weight 분리 관찰.

## 정책
- 커밋: 단계별 Claude 직접 커밋(의도 파일만 스테이징). **push는 Bekay 지시 시만.**
- 완료 후: 검증 결과 HTML 보고 + Bekay 확인②(종결/정정) 대기.

## 실행 환경 메모
- worktree `.mcp.json`에 직접 SSE URL → Claude Code에서 MCP appraise 검증 가능.
- PAD 벤치 측정 시 PowerShell UTF-8(`[System.Text.Encoding]::UTF8.GetBytes`) 주의.
