# Phase 2.4.3 — RelationshipModifiers 통합 재설계

> 🟢 **FROZEN** (2026-06-07 · check-in ① 승인). Claude Code 핸드오프.
> 입력 디자인 문서: `docs/emotion/06-relationship.html §5` (v0.8 / 2026-06-05 freeze)
> 의존: Phase 2.4.2 종결 (commit `ad92932`)
> Git: Claude 직접 커밋 (staging 의도 파일만, push는 명시 지시 시)

## §0 메타 · baseline

- **범위**: (A) modifier 산출 = `Relationship::modifiers()` + 소비처(`appraisal/action.rs`·`event.rs`) + `tuning.rs`. (B) 4축 갱신(`mapping.rs`)은 **비범위**(B-D3).
- **baseline**: `cargo test --lib` **554P / 0F** (2.4.2 종결 `ad92932` 기준 실측 기록). 구현 착수 전 재확인 — 코드 무변경이면 동일.
- **PAD 벤치 게이트**: deviation 0 예상(§1.4 근거) — 형식 확인만.

## §1 Stage 0 실측 (추론 금지 / 실측 우선)

### 1.1 현재 구조 (문서 ↔ 코드 일치 확인)
- `domain/relationship/mod.rs:170` `modifiers()` → 4필드 반환, **affinity·trust 2축만** 사용(respect·wariness 미사용).
- weight (`tuning.rs:72-75`): `REL_AFFINITY_INTENSITY_WEIGHT=0.005` · `REL_TRUST_EMOTION_WEIGHT=0.003` · `REL_AFFINITY_EMPATHY_WEIGHT=0.003` · `REL_AFFINITY_HOSTILITY_WEIGHT=0.003`.
- `intensity_multiplier`·`empathy_modifier`·`hostility_modifier`만 `.max(0.0)`. **`trust_modifier`는 clamp 없음.**
- `appraisal/action.rs:34` — `modifier = intensity_multiplier × trust_modifier` → **⑥ 이중곱 confirmed**.
- `appraisal/event.rs:55,68` — emp/hos 가지는 **단일** modifier(이중곱 아님).
- `appraisal/helpers.rs::add_valence` — `val = base_val × weight × modifier`, **modifier에 clamp 전혀 없음**. base_val 부호로 pos/neg 분기.

### 1.2 정정 (문서 ①⑥⑦ 표현 보정)
- ① "trust_modifier 부호 반전 가능" → 현 w=0.003 + trust ±100 clamp에서 `trust_modifier ∈ [0.7, 1.3]` → **현재 미발생**. clamp 방어 부재 = w 튜닝/범위 변경 시 잠재 위험.
- ⑥ "음수 감정 생성 경로" → `intensity_multiplier`가 `.max(0)`이라 이중곱 ∈ **[0, 1.95]** → 음수 진입 불가(현 구조). add_valence clamp 부재는 사실이나 도달 경로 막힘.
- ② "폭주" → 현 구조 상한 **1.95×**(intensity 1.5 × trust 1.3).
- → 2.4.3 성격 = 긴급 버그 수정 아님. **예방적 구조화(①②⑥) + 신규 기능(③④⑦)**.

### 1.3 base_delta wariness (참고 — (B) 비범위, 회복 경로 실측)
- 올림: Anger +25 / Hate +20 / Reproach +15 / Resentment +15 / Shame +5.
- 내림: **Gratitude −10 / Love −5** (감정 기반 회복 존재).
- → "발산" 아님. clamp(0~100) + 감정 회복 작동. 상향 편향뿐 → 시간 감쇠는 후속(B-D3).

### 1.4 PAD(쾌락·각성·우세) 벤치 경로
- `pad-anchor-score-matrix.md`: 60 앵커 × 20 대사, **BGE-M3 INT8 임베딩 코사인**(앵커·대상 모두 발화 텍스트). OCC(인지평가) 강도·modifier 미경유.
- → modifier 변경 → **deviation 0 예상**. weight 자유도 확보(연속성·서사 기준).

## §2 결정 (B-D)

| ID | 항목 | 결정 |
|---|---|---|
| B-D1 | ① 범위 | **(b)** 공감(HappyFor·Pity)·적대(Resentment·Gloating)도 렌즈 모델 통합 |
| B-D2 | ① 후속 | **(b-2)** 공감/적대에 magnitude(trust) **미적용**(=1.0). 렌즈만 공유 |
| B-D3 | ② wariness 안정화 | **(가)** (B) 갱신 감쇠 **후속 분리**. 2.4.3은 clamp만. 감쇠 vs 히스테리시스 → 감쇠로 좁힘(후속) |
| B-D4 | ③ clamp | **③-A** 각 그룹(magnitude·tilt) 단일 clamp[0.5,1.5]. 최종 곱 [0.25, 2.25] 허용 |
| B-D5 | ④ weight | magnitude `w_t`=0.003 재사용. tilt 초기: affinity 0.003 · respect 0.002 · wariness 0.003 (narrative 미세조정) |
| B-D6 | ⑤ gentleness | 결정 아님 — 통합 테스트로 합산 과억제 검증 |

## §3 목표 구조

### 3.1 두 렌즈 + magnitude
```
FLOOR = 0.5 · CEIL = 1.5
magnitude  = clamp(1 + trust·w_t,                               FLOOR, CEIL)   // trust 볼륨
tilt_warm  = clamp(1 + affinity·w_a + respect·w_r − wariness·w_w, FLOOR, CEIL)  // 따뜻함
tilt_cold  = clamp(1 − affinity·w_a − respect·w_r + wariness·w_w, FLOOR, CEIL)  // 차가움
```
초기 weight: `w_t=0.003 · w_a=0.003 · w_r=0.002 · w_w=0.003` → 단일축 ±100에서 tilt 합 ≈ ±0.5 (clamp 직전).

### 3.2 가지 배정
| 가지 | 소비처 | modifier |
|---|---|---|
| Admiration (pw≥0) | action.rs | magnitude × tilt_warm |
| Reproach (pw<0) | action.rs | magnitude × tilt_cold |
| HappyFor·Pity | event.rs 공감 | tilt_warm (magnitude=1) |
| Resentment·Gloating | event.rs 적대 | tilt_cold (magnitude=1) |

Pride/Shame(자기 행동) = modifier 1.0 하드코딩 유지(`action.rs` agent_id=None 가지) — 무영향.

## §4 구현 명세

### 4.1 struct 재설계 (`domain/emotion/situation.rs:37-53`)
```rust
pub struct RelationshipModifiers {
    pub magnitude: f32,   // trust 볼륨 (was: intensity_multiplier × trust_modifier 분리)
    pub tilt_warm: f32,   // 따뜻함 렌즈 (Admiration·HappyFor·Pity)
    pub tilt_cold: f32,   // 차가움 렌즈 (Reproach·Resentment·Gloating)
}
// Default = { 1.0, 1.0, 1.0 }
```
기존 4필드(intensity_multiplier/trust_modifier/empathy_modifier/hostility_modifier) → 3필드.

### 4.2 `modifiers()` 재작성 (`mod.rs:170`)
```rust
pub fn modifiers(&self) -> RelationshipModifiers {
    let trust = self.trust.value();
    let affinity = self.affinity.value();
    let respect = self.respect.value();
    let wariness = self.wariness.value();
    let p = profile();
    let clamp = |x: f32| x.clamp(p.rel_mod_floor, p.rel_mod_ceil);
    RelationshipModifiers {
        magnitude: clamp(1.0 + trust * p.rel_trust_emotion_weight),
        tilt_warm: clamp(1.0 + affinity * p.rel_affinity_tilt_weight
                             + respect  * p.rel_respect_tilt_weight
                             - wariness * p.rel_wariness_tilt_weight),
        tilt_cold: clamp(1.0 - affinity * p.rel_affinity_tilt_weight
                             - respect  * p.rel_respect_tilt_weight
                             + wariness * p.rel_wariness_tilt_weight),
    }
}
```

### 4.3 소비처 변경
- `action.rs:34` (타인 행동):
  ```rust
  let mods = mods.as_ref().unwrap_or(dialogue_modifiers);
  let tilt = if pw >= 0.0 { mods.tilt_warm } else { mods.tilt_cold };
  let modifier = mods.magnitude * tilt;
  ```
- `event.rs` 공감(55): `let emp_mod = other.modifiers.tilt_warm;` (magnitude 미적용)
- `event.rs` 적대(68): `let hos_mod = other.modifiers.tilt_cold;`
- `add_valence` 시그니처 **불변** (호출 측이 warm/cold 선택해 단일 modifier 전달).

### 4.4 `tuning.rs` 신규/폐기 const
```rust
pub const REL_TRUST_EMOTION_WEIGHT: f32 = 0.003;   // 재사용 (magnitude)
pub const REL_AFFINITY_TILT_WEIGHT: f32 = 0.003;   // 신규
pub const REL_RESPECT_TILT_WEIGHT: f32  = 0.002;   // 신규
pub const REL_WARINESS_TILT_WEIGHT: f32 = 0.003;   // 신규
pub const REL_MOD_FLOOR: f32 = 0.5;                // 신규
pub const REL_MOD_CEIL: f32  = 1.5;                // 신규
```
폐기: `REL_AFFINITY_INTENSITY_WEIGHT` · `REL_AFFINITY_EMPATHY_WEIGHT` · `REL_AFFINITY_HOSTILITY_WEIGHT` (+ `AppraisalWeights` trait/impl 노출 지점 동반 정리: `ports/` + `tuning.rs` impl).

### 4.5 ①–⑦ 해소 매핑
- ①② → magnitude·tilt 공통 clamp[0.5,1.5]
- ⑥ → 이중곱 제거(trust=magnitude / affinity·respect·wariness=tilt), FLOOR≥0.5 음수 차단
- ⑤ → 역할 명확화(2 기능: 볼륨/렌즈)
- ③ → respect를 tilt에 편입 (Admir.↑/Reproach↓)
- ④ → wariness를 tilt에 편입 (Admir.↓/Reproach↑). 양의 피드백 안정화는 후속(B-D3), clamp가 1차 제동
- ⑦ → trust 대칭(배신감 의도 유지) + affinity 렌즈 분리 → "봐줌"/"배신" 둘 다 표현

## §5 위험 (C)

- **empathy/hostility 가지 회귀** — event.rs 두 가지가 새 필드(tilt_warm/cold) 사용. 기존 단일 affinity modifier와 수치 달라짐 → narrative 재측정 필수.
- **gentleness 합산 과억제** (⑤) — 온화 NPC × 친한 상대: tilt_cold FLOOR 0.5 × gentleness(−Gen) → Reproach 과소. 통합 테스트로 확인.
- **struct 필드 변경 파급** — `ActionFocus.modifiers: Option<RelationshipModifiers>`(시나리오 override) · `memory_repository.rs` 직렬화 · situation.rs Default·단위테스트(mod.rs:360~390). 컴파일러가 대부분 포착하나 직렬화 스키마 확인.
- **tilt 합산 clamp 빈발** — 3축 동시 극단 시 clamp 자주 → 변별 저하. 초기 weight 단일축 ±0.5라 완화. narrative로 확인.

## §6 검증 게이트

1. `cargo check` + `cargo test --lib` 회귀 0 (baseline 554P/0F 유지).
2. **PAD 벤치 20 deviation 0** 확인(§1.4 — 통과 예상. 편차 시 Bekay 승인 없이 기대값 변경 금지).
3. **narrative S1~S4 재측정** (`appraise-validation/`) — Admiration/Reproach + 공감/적대 가지 새 modifier 반영. 박제값 갱신(신호 격리).
4. **gentleness 합산 통합 테스트** — 온화 NPC × 친밀 상대 Reproach 과억제 여부.
5. grep 게이트:
   - `intensity_multiplier|empathy_modifier|hostility_modifier` → src 0건
   - `REL_AFFINITY_INTENSITY_WEIGHT|REL_AFFINITY_EMPATHY_WEIGHT|REL_AFFINITY_HOSTILITY_WEIGHT` → 0건
   - `tilt_warm|tilt_cold|magnitude` → modifiers() + action.rs + event.rs 등장

## §7 비스코프

- **(B) wariness 시간 감쇠 / Scene 경계 1회 적용** → 후속(B-D3, 로드맵 §5 "호출 횟수 비의존" 트랙).
- **PerceivedSituation 층** (행동 심각도 임계 "큰 배신" + praiseworthiness 부호 재해석) → Phase 2.5 이후 별도.
- **O+ Unconv. hexaco_modifier placeholder** → (B) 영역, 무관.
