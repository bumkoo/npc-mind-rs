# HEXACO × OCC 22감정 매핑 보고서 (현행화)

## 개요

OCC 22개 감정은 HEXACO 6차원이 **가중치 변수**로 작용하여 강도와 발생 여부가 결정된다. NPC 심리 엔진은 성격 수치를 감정 강도 변조기(Modifier)로 변환하여 동일한 상황에서도 캐릭터마다 다른 정서적 반응을 보이게 한다.

관련 문서:
- [OCC 감정 모델 상세](occ-emotion-model.md)
- [AppraisalEngine 설계](appraisal-engine.md)

---

## 22감정 × HEXACO 매핑 전체 표 (엄격한 코드 반영)

`●` = 직접 영향(Modifier), `○` = 조건부/간접 영향, `—` = 해당 없음

| OCC 분기 | 감정 (EN) | H | E | X | A | C | O |
|----------|-----------|---|---|---|---|---|---|
| Well-being | Joy | — | ●(abs) | ●(pos) | — | — | — |
| Well-being | Distress | — | ●(abs) | — | ●(neg) | ●(neg) | — |
| Fortune-of-others | HappyFor | ○발동 | — | — | ●공감 | — | — |
| Fortune-of-others | Pity | — | ●(sent) | — | ●공감 | — | — |
| Fortune-of-others | Gloating | ○발동 | — | — | ○발동 | — | — |
| Fortune-of-others | Resentment | ○발동 | — | — | ●(neg) | — | — |
| Prospect | Hope | — | — | ●(pos) | — | — | — |
| Prospect | Fear | — | ●(abs) | — | — | — | — |
| Prospect | Satisfaction | — | ●(abs) | — | — | — | — |
| Prospect | Disappointment | — | ●(abs) | — | — | — | — |
| Prospect | Relief | — | ●(abs) | — | — | — | — |
| Prospect | FearsConfirmed | — | ●(abs) | — | — | — | — |
| Attribution | Pride | ●(neg) | — | — | — | ●(abs) | — |
| Attribution | Shame | — | — | — | — | ●(abs) | — |
| Attribution | Admiration | — | — | — | — | ●(abs) | — |
| Attribution | Reproach | — | — | — | ●(neg) | ●(abs) | — |
| Compound | Gratitude | ●(pos) | — | — | — | — | — |
| Compound | Anger | — | — | — | ●(neg) | — | — |
| Compound | Gratification | — | — | — | — | ●(abs) | — |
| Compound | Remorse | — | — | — | — | ●(abs) | — |
| Object | Love | — | — | — | — | — | ●(abs) |
| Object | Hate | — | — | — | — | — | ●(abs) |

> **Modifier 규칙**:
> - **(pos)**: `pos_modifier` (양수일 때 증폭)
> - **(neg)**: `neg_modifier` (양수일 때 억제)
> - **(abs)**: `abs_modifier` (절댓값 기반 증폭/변조)

---

## OCC 분기별 HEXACO 상세 로직 (선별적 적용 규칙)

### 1. Event-based: Well-being & Prospect
- **E (정서성)**: `emotional_amp`는 대부분의 사건 감정에 적용되나, **`Hope`에는 적용되지 않는다.** 희망은 감정적 민감도보다 외향적 기대에 더 의존하기 때문이다.
- **X (외향성)**: `positive_amp`는 **`Joy`와 `Hope`에만 적용된다.** `Satisfaction`이나 `Relief` 같은 결과 확인형 긍정 감정은 성격에 의한 추가 증폭을 받지 않는다.

### 2. Compound: Well-being + Attribution
- **복합 감정 규칙**: `Gratitude`, `Anger`, `Gratification`, `Remorse`는 `emotional_amp`(E)나 `positive_amp`(X)의 영향을 받지 않는다. 이 감정들은 상황의 사실 관계와 **도덕적 기준(C)**, **전용 브레이크(H, A)**에 의해서만 강도가 결정된다.

### 3. Fortune-of-others (타인의 운)
- **발동 임계값**: `H` 또는 `A`가 **`-0.2`** (FORTUNE_THRESHOLD) 이하일 때만 악의적 감정(시기, 고소함)이 발생한다.
- **공감 기반**: `HappyFor`와 `Pity`는 `EMPATHY_BASE(0.5)`를 기본값으로 하며, 성격 수치에 의해 증폭된다.

---

## 관계(Relationship)의 시너지

모든 감정 공식에는 성격 가중치 외에 **관계 배율**이 최종적으로 곱해진다.
- **`rel_mul`**: `1.0 + closeness.intensity() * 0.5`. 친밀하거나 적대적일수록 모든 감정 반응이 커진다.
- **`trust_mod`**: `1.0 + trust.value() * 0.3`. 신뢰하는 이의 행동에는 더 크게 반응하고, 불신하는 이의 행동에는 덤덤해진다. (Admiration, Reproach, Anger, Gratitude에 적용)

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| 0.1.0 | 2026-03-24 | 초기 작성 |
| 0.2.0 | 2026-03-26 | **전면 현행화**: 실제 코드의 선별적 가중치 적용 로직(X는 Joy/Hope에만 등)을 엄격히 반영. |
