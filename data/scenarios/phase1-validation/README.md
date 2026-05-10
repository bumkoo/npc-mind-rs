# Phase 1 Mind Architecture — Narrative Validation 시나리오

`relationships.md` v0.7 §6 Scene Boundary Reflection의 Outer Loop 게이트가 *서사적 비중*과
일치하는지 검증하는 3 시나리오 + 통합 테스트 셋.

## 3 밴드 검증

| 밴드 | 시나리오 | 기대 reflection | 기대 follow-up | axes |
|---|---|---|---|---|
| **낮음 (잡담)** | [chitchat-passerby.json](chitchat-passerby.json) — 임충이 길에서 행인과 의례적 인사 | `is_chitchat=true`, `significance < 0.2` | 3 (DialogueReflected + EmotionCleared + SceneEnded) | **변화 0 (보존)** |
| **중간 (일상)** | [daily-training.json](daily-training.json) — 수련이 춘설병에게 호흡 검법 가르침 | `is_chitchat=false`, `significance ~ 0.4-0.6` | 4 (+ RelationshipUpdated) | 미세 (closeness/trust 약간 ↑) |
| **높음 (결단)** | [lin-chong-shanshenmiao.json](lin-chong-shanshenmiao.json) — 임충이 산신묘에서 육겸 처단 | `is_chitchat=false`, `significance ≥ 0.85` | 4 (+ RelationshipUpdated) | 큼 (closeness 양수→음수 큰 변화) |

## 자동 검증 — Mock LLM (CI)

각 시나리오에 대응하는 통합 테스트:
- `tests/phase1_chitchat_test.rs::chitchat_skips_outer_loop_emits_three_events_and_preserves_axes`
- `tests/phase1_daily_training_test.rs::daily_training_enters_outer_loop_emits_four_events_and_updates_axes`
- `tests/phase1_shanshenmiao_test.rs::shanshenmiao_high_band_emits_four_events_and_reverses_axes_strongly`

각 테스트는 시나리오 JSON 로드 (inner_compass A-min 호환 검증 포함) + 미리 정해진
`ReflectionResult`를 직접 `Command::EndDialogue`에 박아 `dispatch_v2` → events 시퀀스 +
axes 변화 검증. 실제 LLM 미사용 — 결정론, 수 ms 내 완료.

```bash
cargo test --features chat,embed,listener_perspective --test phase1_chitchat_test \
                                                       --test phase1_daily_training_test \
                                                       --test phase1_shanshenmiao_test
```

## 디자이너 검증 체크리스트 (Mind Studio 수동)

실제 LLM 판정 + 게이트 calibration이 *서사적 직관*과 일치하는지는 디자이너(Bekay)가
Mind Studio에서 수동 검증. spec §4.4의 narrative validation:

### 시나리오 1 (잡담) — `chitchat-passerby.json`
- [ ] Mind Studio 로드 → 4~6 turn 의례적 인사 진행
- [ ] end_session 응답에 `reflection.is_chitchat = true` 확인
- [ ] `reflection.significance_score < 0.3` 확인
- [ ] axes 변화 0 (`before == after`) 확인
- [ ] SSE 이벤트에 `RelationshipUpdated` **없음** 확인 (DialogueReflected만)

### 시나리오 2 (일상) — `daily-training.json`
- [ ] 8 turn 가르침 진행
- [ ] `reflection.is_chitchat = false` 확인
- [ ] `reflection.significance_score 0.3~0.7` 확인
- [ ] axes 미세 변화 (closeness 약간 ↑) 확인
- [ ] memory: summary 저장 확인

### 시나리오 3 (결단) — `lin-chong-shanshenmiao.json`
- [ ] 9~10 turn 처단 사건 진행
- [ ] `reflection.is_chitchat = false` 확인
- [ ] `reflection.significance_score >= 0.7` 확인
- [ ] `reflection.llm_reasoning`이 *왜 이 점수*인지 합리적 설명 (OCC peak / PAD / beat)
- [ ] axes 큰 변화 (lin_chong→lu_qian closeness 양수→음수 강하게) 확인
- [ ] memory: summary + 결단 인용 저장 확인

### Robustness
- [ ] LLM이 *invalid JSON* emit 시 fallback 동작 확인 (게임 진행 막힘 0)
- [ ] LLM 타임아웃 시 fallback 동작 확인 (`__COMPAT_LAYER`로 회피된 dispatch_v2_test와 별개)

### Calibration tuning (필요 시)
3 밴드의 `significance_score`가 *낮음 (<0.3) / 중간 (0.3-0.7) / 높음 (>0.7)*에 정확히
들어가지 않으면 가중치 (0.40/0.30/0.15/0.15) 또는 임계값 (0.3) 조정.
변경 시 `src/domain/reflection.rs::compute_significance` 가중치 + `relationships.md` v0.7 §6.3
동기 갱신.

## NPC inner_compass 명시

3 시나리오 모두 `npcs.<id>.inner_compass`에 compass 한 줄 명시 (Phase 1 A-min).
`Npc::compass_short_label()`로 회수되어 ReflectionService prompt builder가 LLM 컨텍스트로 주입.
taboo / life_question은 Phase 3c에서 `InnerCompass` 객체 승격 시 활성화.

## 변경 이력

| 일자 | 변경 |
|---|---|
| 2026-05-11 | 초안. Phase 1 Stage 4 narrative validation 3 시나리오 + 통합 테스트 + 디자이너 체크리스트. |
