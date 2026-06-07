# Phase 2.4.4 — listener_perspective 축별 sign 분리

> 🟢 **종결** (2026-06-07 · check-in ② 검증 통과). 이 세션 직접 구현. `cargo test --lib` 555P/0F.
> 입력 디자인: `docs/emotion/09-utterance-pad.html §7` (v1.5.0 freeze)
> 의존: Phase 2.4.3 종결 (구현 `51ec65a`). 코드 영역 미겹침 — PAD 벤치만 공유 가능성(§5).
> Git: Claude 직접 커밋.

## §0 메타 · baseline

- **범위**: 화자 PAD(쾌락·각성·우세) → 청자 PAD 변환식의 **D축에 sign 적용**. `converter.rs::build_result` 1개 함수 + 주석.
- **baseline**: `cargo test --lib` **555P / 0F** (2.4.3 종결 `51ec65a` 기준, 직전 검증 실측).
- **PAD 벤치**: converter는 임베딩 *이후* 단계(화자 PAD→청자 PAD)라 PAD 벤치(utterance→화자 PAD 임베딩 경로)와 **독립 가능성** — §5 게이트서 재측정 판정.

## §1 Stage 0 실측

### 1.1 현재 변환식 (`converter.rs::build_result` L248-262)
```
P_L = sign × coef_p × P_S      (sign ±1, P축 전용)
A_L =        coef_a × A_S      (sign 없음 — A_S 부호 복사)
D_L =        coef_d × D_S      (sign 없음 — D_S 부호 복사)
```
- `p_s` = prefilter hit → `p_s_default` / miss → `speaker_pad.pleasure`.
- coef (magnitude_coef): weak 0.5/0.5/0.4 · normal 1/1/1 · strong 1.5/1.3/1.3.
- `applied_d_coef = d_coef` (meta, sign 미포함).

### 1.2 약점 3 (09 §7.1)
- ① D 부호 복사(`d_coef × D_S`, 항상 +) → 위축/반발(상보성) 불가.
- ② A 부호 고정 → 청자 역방향 각성 불가.
- ③ sign P 전용 → 반어("허허, 훌륭하시오") 시 D 미반전(표면 복종·실제 조롱).

### 1.3 09 §7 freeze
- **개선안 A 채택**: sign 1개 → P·D 양축 적용. 시그니처 유지(SignClassifier 단일 출력 그대로).
- **개선안 B 보류**: "청자 현재 D 반영"은 08 pad_dot D 격차 배율과 **이중 적용** → 09는 발화 D 방향만, 격차는 08 소관.
- **경계**: `D_sign`은 *발화 의미상 반전(반어)*만. 청자 격차(위축/반발)는 08 — 09서 재현 안 함.

## §2 결정 (B-D)

| ID | 항목 | 결정 |
|---|---|---|
| B-D1 | 개선안 | **A** (축별 sign 분리). B 보류(08 pad_dot 소관) |
| B-D2 | sign 적용 축 | **(가) P·D만**. A 제외 — ② 미해소(각성 반전 의미 약함, 09 권장) |
| B-D3 | `D_sign` 값 | **= `P_sign` = SignClassifier 단일 출력**. 별도 분류기 없음 → 시그니처 유지 |

## §3 변경 명세

### 3.1 `build_result` (converter.rs L248-262)
```rust
// D_L: sign 적용 추가 (B-D3)
sign.as_f32() * d_coef * speaker_pad.dominance,   // was: d_coef * speaker_pad.dominance
// meta
applied_d_coef: sign.as_f32() * d_coef,           // was: d_coef
```
P_L·A_L **불변** (P는 이미 sign / A는 ② 범위 밖).

### 3.2 헤더 주석 (converter.rs L14-16)
```
D_L = sign × coef_d × D_S    // was: D_L = coef_d × D_S
```

## §4 위험 · 영향

- **영향 테스트**: `prefilter_hit_uses_category_values` D_L 단언(L527-528). sign=invert·strong: `D_L = -1 × 1.3 × -0.1 = +0.13` (was −0.13) — **부호 반전, 박제 갱신**.
- `classifier_path_uses_speaker_p`(sign=keep): D 무변 → 보존.
- `applied_d_coef` 소비처(meta 사용) 확인.
- 위험 **낮음** — 1함수·시그니처 유지·테스트 거의 보존.

## §5 검증 게이트

1. `cargo test --lib` 555P/0F (D 테스트 1개 박제 갱신 후 유지).
2. listener_perspective 통합 벤치 재측정 (있으면 — `tests/listener_perspective_integration_bench.rs` 등).
3. **PAD 벤치 20 재측정** — converter가 임베딩 이후라 독립 예상이나, 반어 케이스 포함 시 청자 D 변동 가능. 실측 deviation 확인. 편차 시 **Bekay 승인 없이 기대값 변경 금지**.
4. grep: `build_result` D_L 라인에 `sign.as_f32()` 등장 / `applied_d_coef` = sign 포함.

## §6 비스코프

- **② A축 sign** (각성 반전) — 의미 약함, 미해소 잔류. (필요 시 별도 검토)
- **① D 상보성**(위축/반발, 청자 D 격차) → 개선안 B / **08 pad_dot 소관**. 09 재현 금지(이중 적용 방지).
