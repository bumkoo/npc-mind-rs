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
> - 모든 성격 가중치는 `Score::modifier(weight)` = `(1.0 + value × weight).max(0.0)` 패턴을 사용.
> - `●` = 직접 영향, `○` = 조건부/간접 영향, `—` = 해당 없음

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

감정별로 관계 배율이 다르게 적용된다.

| 배율 | 공식 | 적용 감정 |
|------|------|----------|
| `rel_mul` | `(1.0 + closeness × 0.5).max(0.0)` | **Admiration, Reproach에만 적용** |
| `trust_mod` | `1.0 + trust × 0.3` | **Admiration, Reproach에만 적용** |
| `empathy_rel_modifier` | `(1.0 + closeness × 0.3).max(0.0)` | HappyFor, Pity |
| `hostility_rel_modifier` | `(1.0 - closeness × 0.3).max(0.0)` | Resentment, Gloating |

**주의**: 이전 버전에서는 rel_mul이 모든 감정에 적용되었으나, 현재는 Admiration/Reproach에만 한정됨.
Joy, Distress, Hope, Fear, Pride, Shame, Love, Hate, Compound 감정에는 관계 배율이 적용되지 않음.

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| 0.1.0 | 2026-03-24 | 초기 작성 |
| 0.2.0 | 2026-03-26 | **전면 현행화**: 실제 코드의 선별적 가중치 적용 로직(X는 Joy/Hope에만 등)을 엄격히 반영. |
| 0.3.0 | 2026-03-28 | rel_mul Admiration/Reproach 한정, modifier 메서드→Score::modifier(weight) 통일, 관계 배율 감정별 분리 |
