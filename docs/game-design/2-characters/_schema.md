# 인물 스키마 (Character Schema)

> Version: 0.2 — 2026-05-04
> 모든 인물 인스턴스가 따르는 공통 구조.
> 위치: `docs/game-design/2-characters/_schema.md`

## 설계 원칙

이 스키마는 **3개 층(Layer)**으로 구성된다:

- **Layer 1 — 본바탕 (Base)**: HEXACO 기질, 출신, 신체. *거의 변하지 않음.*
- **Layer 2 — 현재 표현 (Expression)**: 내적 나침반, 현재 감정, 관계, 화법. *상황 따라 흔들림.*
- **Layer 3 — 시간축 (Arc)**: 과거 전환점, 현재 갈등, 미래 궤적. *게임 진행이 변화시킴.*

핵심 원칙:
1. **HEXACO가 base, 모두에게 공통.** 협객·농부·상인·악당 모두 같은 스키마.
2. **가치 표현은 inner_compass 한 곳에서 끝낸다.** 별도 가치 점수표 없음.
3. **인물의 게임 내 비중에 따라 Tier로 부담 분배.** 단역에 풀 스키마 강요하지 않음.

---

## Tier 시스템

| Tier | 대상 | 필수 필드 |
|------|------|-----------|
| **Tier 1** | 단역 (이름 없는 위병, 행인) | HEXACO 6 factor + `compass` 한 줄 |
| **Tier 2** | 조연 (명명된 NPC, 단발성) | + HEXACO 24 facet + 전체 `inner_compass` + transition_point 1개 + voice 기본 |
| **Tier 3** | 주연·동료·주적 | 풀 스키마. `life_question` 필수. `voice_anchors` 3개 이상. |

`life_question`은 **Tier 3에서만 필수**. Tier 2는 선택.

---

## Layer 1 — 본바탕 (Base)

### identity
- `id`: 시스템 식별자 (예: `"guo_jing"`)
- `name`: 본명
- `nicknames`: 별명 목록 (예: 북협(北俠))
- `era`: 어느 시대 인물인가 → 참조: `1-world/time.md`
- `stage_of_life`: 청년기 / 장년기 / 노년기
  - 청년기 = 사조 톤(동행·기연), 장년기·노년기 = 와호 톤(절제·침묵). Pillar 1 참조.

### origin
- `birthplace`: 출생지 → 참조: `1-world/geography.md`
- `social_origin`: 귀족 / 양민 / 천민 / 미천 / 출가 / 이족(異族)
- `kingdom_of_origin`: 7국 중 → 참조: `1-world/kingdoms.md`
- `family_background`: 1~2 문장. 사상적·문화적 배경도 여기 자연스럽게 표현 ("법가 관리 가문의 셋째 아들" 등).

### temperament — HEXACO
> Tier 1: 6 factor 점수만. Tier 2 이상: 24 facet 모두.

```yaml
H_honesty_humility: { sincerity, fairness, greed_avoidance, modesty }
E_emotionality:     { fearfulness, anxiety, dependence, sentimentality }
X_extraversion:     { social_self_esteem, social_boldness, sociability, liveliness }
A_agreeableness:    { forgiveness, gentleness, flexibility, patience }
C_conscientiousness:{ organization, diligence, perfectionism, prudence }
O_openness:         { aesthetic_appreciation, inquisitiveness, creativity, unconventionality }
```

### body
- `physical_description`: 1~2 문장 (예: "키 작고 마름. 손가락이 길고 우아함.")
- `signature_feature`: 한 가지 기억에 남는 외형/습관 — 별명 생성과 연동.

---

## Layer 2 — 현재 표현 (Expression)

### inner_compass — 가치의 세 면 ★ 핵심

```yaml
inner_compass:
  compass: "양양을 지킨다"               # 지향. 의식적·능동적. 매일 결정 방향.
  taboo:   "동문(同門)을 베지 않는다"     # 경계. 반의식적·방어적. 깨지면 정체성 붕괴.
  life_question: "단순한 사람도 충분한가?"  # 의문. 무의식적·인력적. 닿이면 흔들림.
```

#### 각 필드의 작동 방식 (구분되어야 함)

- **compass** — 인물을 *움직이게* 한다. 시간 따라 바뀔 수 있음.
- **taboo** — 인물을 *멈추게* 한다. 깨지는 순간이 가장 큰 transition_point.
- **life_question** — 인물을 *흔들리게* 한다. 평생 풀고자 하는 질문.

#### life_question 작성 가이드

**유형 힌트** (제약이 아니라 영감 — 자유 텍스트):

- **세계론적**: "세상은 공평한가?" / "선이 정말 이기는가?"
- **인간론적**: "인간은 나아질 수 있는가?" / "사람을 믿을 수 있는가?"
- **관계적**: "그 사람은 나를 정말 좋아했나?" / "내가 잊혀졌나?"
- **자기평가적**: "그때 회피하지 않았다면?" / "나는 진짜 강한 사람인가?"
- **운명적**: "내가 이 길을 *선택*한 게 맞나?" / "다른 삶도 가능했나?"

**중요한 작성 원칙**:

1. **인물 본인은 의식하지 못할 수 있다.** 무의식적인 질문일수록 더 깊다.
2. **인물의 직접 대사에 그대로 나오면 안 된다.** 깊이가 사라진다.
3. **디자이너·LLM 메타 정보로만 사용.** acting directive에 "이 인물은 X를 묻고 있다, 다만 본인은 의식하지 못한다"로 전달.
4. **transition_points에서 *드러날 수 있다*.** 한 번 의식되는 순간이 전환점이다.

#### 엔진과의 연결

life_question은 npc-mind-rs 엔진에서 다음 역할:

- **PAD 자극의 비대칭 증폭기**: 의문에 *닿는* 사건은 일반 +5가 아니라 +20의 진폭을 만듦.
- **기연 트리거 단서** (Pillar 5): 의문에 부분적 답을 *암시하는* 만남·발견이 기연이 됨.
- **acting directive 깊이**: voice_anchors가 *어떻게* 말하는지를 정의한다면, life_question은 *왜 그렇게 말하는가의 sub-text*를 정의.

### current_state — 즉시 활성 변수
- `pad`: { pleasure, arousal, dominance } — npc-mind-rs 엔진 동기화
- `dominant_emotion`: OCC 우세 감정 (예: Pride, Distress)
- `active_focus`: 현재 가장 강한 동기 (예: "사부 보호", "복수")

### relationships (요약만)
> 본체는 별도 카테고리: `2-characters/relationships.md`

- `key_bonds`: 지기·사부·원수 등 활성 관계 식별자 목록
- `dormant_bonds`: 휴면 인연 — Pillar 4 "묻힌 인연 재활성화" 후보

### voice — LLM 연기용
> npc-mind-rs가 acting directive를 만들 때 LLM에 넘기는 핵심 필드.

- `speech_register`: 격조 / 일상 / 거친 / 소박
- `vocabulary_level`: 사대부 / 양민 / 강호 / 미천
- `tics`: 입버릇·어투 특징 (예: "흠…" 자주, 한자성어 많음)
- `voice_anchors`: **대표 대사 3~5개** (Tier 3 필수). few-shot 정렬용.

### titles (참조 필드)
> 평판 시스템 본체는 추후 디자인. 지금은 디자이너가 직접 부여.

- `titles`: 보유 칭호 목록 (예: `["북협"]`)
- 협객·신의·대유·대도 등이 여기에 들어감. 행동 누적으로 *얻고 잃는* 시스템은 별도 문서에서.

---

## Layer 3 — 시간축 (Arc)

### past
- `transition_points`: 인물을 *지금*으로 만든 사건 목록.

  ```yaml
  - age: 17
    event: "어머니 자결 → 사부에게 거둬짐"
    impact: "E+ Sentimentality 30→65, A+ Forgiveness 70→40"
    inner_resolution: "다시는 약해지지 않는다"
  ```

- `formative_relationships`: 결정적이었던 관계 (이미 끝났을 수 있음).

### present
- `unresolved_tension`: 1~3개. 현재 인물이 *해결하지 못한* 갈등.
  - 외부적: 적·수배·복수 대상
  - 내부적: 가치관 충돌·죄책감
  - 관계적: 오해·미답의 약속

### future hooks (선택)
- `tragic_seed`: 비극으로 향할 가능성의 씨앗 (Pillar 4 — 옛 약속의 메아리)
- `joyful_seed`: 구원·기연으로 향할 가능성의 씨앗 (Pillar 5)
- 디자이너의 *의도된 가능성*. NPC 자율성(Pillar 2)을 해치지 않는 *가이드*.

---

## 검증 체크리스트

새 인물 인스턴스 추가 시 Tier에 맞춰 확인.

### Tier 1 (단역)
1. HEXACO 6 factor 점수가 채워져 있나?
2. `compass` 한 줄이 있나?

### Tier 2 (조연)
1. HEXACO 24 facet 점수가 모두 채워져 있나?
2. `inner_compass` 세 필드 (compass, taboo, life_question) 중 *최소 compass + taboo* 가 있나?
3. `transition_points` 최소 1개?
4. `voice` 기본 정보 (register, tics) 있나?

### Tier 3 (주연·주요)
1. HEXACO 24 facet 모두?
2. **`inner_compass` 세 필드 모두 (life_question 필수)?**
3. `transition_points` 최소 2개?
4. `voice_anchors` 최소 3개?
5. `unresolved_tension` 최소 1개? (없으면 드라마 없음)
6. `tragic_seed` 또는 `joyful_seed` 중 하나?

### 일관성 검증 (모든 Tier)
- HEXACO ↔ taboo 일관성: 예) H- Sincerity = 20 인데 "거짓말 안 한다"는 모순.
- HEXACO ↔ compass 일관성: 깊은 충돌이 있다면 transition_point에 사유 명기.
- HEXACO ↔ life_question 일관성: 의문이 인물의 기질에서 *자연스럽게 떠오르는가?*

---

## 변경 이력

| 버전 | 일자 | 변경 |
|------|------|------|
| v0.1 | 2026-05-04 | 초안. 3-layer 구조. HEXACO=base, 협객 8축=expression, narrative=arc |
| v0.2 | 2026-05-04 | 협객 8축 / archetype / cultural_exposure 제거. `inner_compass` (compass + taboo + life_question) 도입. Tier 시스템 추가. |
