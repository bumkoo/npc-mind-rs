# NPC Mind Engine — 테스트 레포트

## 테스트 정보
- **도서**: The Adventures of Huckleberry Finn (Mark Twain)
- **장면**: Chapter VIII — 잭슨 섬에서 헉과 짐의 첫 만남
- **세션**: session_001
- **날짜**: 2026-03-28
- **엔진 버전**: npc-mind 0.1.0
- **테스트 방식**: webui API (Invoke-WebRequest)

---

## 1. 장면 개요

잭슨 섬에서 Huck Finn이 도주한 노예 Jim을 우연히 발견하는 장면.
Jim은 Huck이 죽은 줄 알고 있었기 때문에 유령으로 착각하고 공포에 질린다.
이후 Huck이 살아있음을 알게 되고, 서로의 도주 사실을 고백하며 신뢰를 형성한다.

### 원문 핵심 대사

> "Hello, Jim!" and skipped out.
> He bounced up and stared at me wild. Then he drops down on his knees...
> "Doan' hurt me — don't! I hain't ever done no harm to a ghos'."

> "Well, I b'lieve you, Huck. I — I run off."
> "Jim!"
> "But mind, you said you wouldn' tell — you know you said you wouldn' tell, Huck."

> "Well, I did. I said I wouldn't, and I'll stick to it. Honest injun I will.
> People would call me a low down Abolitionist and despise me for keeping mum
> — but that don't make no difference."

---

## 2. NPC 프로필 설계

### Huck Finn
| 차원 | facet | 값 | 설계 근거 |
|------|-------|-----|----------|
| H 정직-겸손 | sincerity | -0.3 | 거짓말에 능하지만 중요한 순간에는 정직 |
| | fairness | 0.2 | 기본적 공정감은 있음 |
| | greed_avoidance | 0.4 | 물질욕 낮음 |
| | modesty | 0.3 | 겸손한 편 |
| E 정서성 | fearfulness | -0.5 | 위험에 담담 |
| | anxiety | -0.3 | 불안감 낮음 |
| | dependence | -0.7 | 매우 독립적 |
| | sentimentality | -0.2 | 감성 표현 적음 |
| X 외향성 | social_self_esteem | -0.2 | 자존감 약간 낮음 (사회적 위치) |
| | social_boldness | 0.4 | 낯선 상황에서 주도적 |
| | sociability | 0.1 | 혼자도 괜찮음 |
| | liveliness | 0.3 | 적당히 활발 |
| A 원만성 | forgiveness | 0.3 | 용서할 줄 앎 |
| | gentleness | 0.2 | 약간 부드러움 |
| | flexibility | 0.5 | 편견에 유연 (핵심 특성) |
| | patience | 0.3 | 적당한 인내 |
| C 성실성 | organization | -0.6 | 규칙 무시 |
| | diligence | -0.3 | 근면하지 않음 |
| | perfectionism | -0.5 | 완벽주의 없음 |
| | prudence | -0.4 | 충동적 |
| O 개방성 | aesthetic_appreciation | 0.2 | 자연 감상 |
| | inquisitiveness | 0.6 | 호기심 강함 |
| | creativity | 0.5 | 창의적 문제해결 |
| | unconventionality | 0.8 | 사회 관습 강하게 거부 (핵심 특성) |

### Jim
| 차원 | facet | 값 | 설계 근거 |
|------|-------|-----|----------|
| H 정직-겸손 | sincerity | 0.7 | 진실되고 솔직 |
| | fairness | 0.6 | 공정한 성품 |
| | greed_avoidance | 0.5 | 탐욕 없음 |
| | modesty | 0.7 | 겸손하고 자기를 낮춤 |
| E 정서성 | fearfulness | 0.6 | 두려움 많음 (핵심 특성) |
| | anxiety | 0.5 | 걱정 많음 |
| | dependence | 0.4 | 유대 의존적 |
| | sentimentality | 0.8 | 매우 감성적 (핵심 특성) |
| X 외향성 | social_self_esteem | -0.4 | 자존감 낮음 (노예 신분) |
| | social_boldness | -0.5 | 소극적 |
| | sociability | 0.3 | 사람과의 교류 원함 |
| | liveliness | 0.2 | 조용한 편 |
| A 원만성 | forgiveness | 0.8 | 매우 관용적 (핵심 특성) |
| | gentleness | 0.7 | 온화함 |
| | flexibility | 0.6 | 유연함 |
| | patience | 0.7 | 인내심 강함 |
| C 성실성 | organization | 0.3 | 적당히 체계적 |
| | diligence | 0.6 | 성실함 |
| | perfectionism | 0.3 | 완벽주의 아님 |
| | prudence | 0.5 | 신중함 |
| O 개방성 | aesthetic_appreciation | 0.3 | 약간의 미적 감각 |
| | inquisitiveness | 0.2 | 탐구심 낮음 |
| | creativity | 0.1 | 창의성 낮음 |
| | unconventionality | -0.3 | 전통적, 미신 믿음 |

### 초기 관계 설정
| 관계 | closeness | trust | power | 근거 |
|------|-----------|-------|-------|------|
| Jim → Huck | 0.1 | 0.1 | -0.3 | 같은 집에서 알긴 하지만 가깝지 않음. Jim은 노예로서 낮은 지위 |
| Huck → Jim | 0.1 | 0.1 | 0.3 | 약간 아는 사이. Huck은 백인 소년으로 약간 높은 지위 |

---

## 3. 대화 턴별 결과 (Jim의 관점)

### Turn 1: 유령 공포

**상황**: 잠을 자던 중, 죽었다고 알고 있던 헉이 갑자기 나타나 "Hello, Jim!"이라고 말했다.

**API 요청**:
- event.description: "죽은 줄 알았던 헉이 눈앞에 나타남 — 유령의 출현"
- event.desirability_for_self: -0.8
- prospect: 없음 (이미 일어난 사건)

**결과**:
| 감정 | 강도 | context |
|------|------|---------|
| Distress | 0.650 | 죽은 줄 알았던 헉이 눈앞에 나타남 — 유령의 출현 |

**분석**: Jim의 높은 fearfulness(0.6)가 Distress 강도를 끌어올림. OCC 이론상 이미 일어난 부정적 사건 → Distress가 정확한 매핑.

---

### Turn 2: 안도 — 헉이 살아있다!

**상황**: 헉이 유령이 아니라 살아있는 사람이라는 것을 알게 되었다.

**API 요청**:
- event.description: "헉이 살아있다 — 유령이 아니었다. 외딴 섬에서 혼자가 아니게 되었다"
- event.desirability_for_self: 0.8
- prospect: "fear_unrealized"

**결과**:
| 감정 | 강도 | context |
|------|------|---------|
| Relief | 0.858 | 헉이 살아있다 — 유령이 아니었다. 외딴 섬에서 혼자가 아니게 되었다 |

**분석**: `fear_unrealized` 전망이 Joy 대신 Relief를 정확히 생성. Jim의 높은 sentimentality(0.8)가 안도감을 극대화. OCC 이론의 "두려웠던 것이 실현되지 않은 안도" 매핑 정확.

---

### Turn 3: 도주 고백

**상황**: Jim이 Miss Watson의 집에서 도망친 사실을 고백한다. 헉이 신고할 수도 있다는 두려움.

**API 요청**:
- event.description: "도주 사실을 고백함 — 신고당할 위험을 감수"
- event.desirability_for_self: -0.5
- event.prospect: "anticipation"
- action.description: "노예 신분에서 도주한 자신의 행위"
- action.agent_id: null (자기 행동)
- action.praiseworthiness: -0.2

**결과**:
| 감정 | 강도 | context |
|------|------|---------|
| Fear | 0.676 | 도주 사실을 고백함 — 신고당할 위험을 감수 |
| Shame | 0.268 | 노예 신분에서 도주한 자신의 행위 |

**분석**:
- Fear(0.676): `anticipation` 전망 + 부정적 사건. Jim의 fearfulness(0.6)가 반영
- Shame(0.268): 자기 행동에 대한 부정적 도덕 평가(-0.2). 사회가 심어준 죄책감이지만 강도는 낮음 — Jim의 sincerity(0.7)가 높아서 자기 행동을 강하게 비난하지 않음
- 두 감정이 동시에 나온 것이 장면의 복합적 심리를 잘 포착

---

### Turn 4: 약속 — "Honest injun, I will"

**상황**: 헉이 사회적 평판을 걸고 Jim의 비밀을 지키겠다고 맹세.

**API 요청**:
- event.description: "헉이 사회적 비난을 감수하고 비밀을 지키겠다고 약속함"
- event.desirability_for_self: 0.8
- event.prospect: "hope_fulfilled"
- action.description: "헉이 자신의 평판을 걸고 도망 노예를 보호하겠다고 맹세하는 행위"
- action.agent_id: "huck" (대화 상대의 행동)
- action.praiseworthiness: 0.8

**결과**:
| 감정 | 강도 | context |
|------|------|---------|
| Admiration | 1.000 | 헉이 자신의 평판을 걸고 도망 노예를 보호하겠다고 맹세하는 행위 |
| Satisfaction | 0.858 | 헉이 사회적 비난을 감수하고 비밀을 지키겠다고 약속함 |

**분석**:
- Admiration(1.000): 최대치로 클램프. Jim의 sincerity(0.7)와 fairness(0.6)가 도덕적 행위에 대한 감탄을 극대화
- Satisfaction(0.858): `hope_fulfilled` 전망. 비밀을 지켜줄 거라는 희망이 실현
- mood: 0.929 (매우 긍정적)

---

## 4. 관계 변동 (대화 종료 후)

| | before | after | 변동 | 평가 |
|--|--------|-------|------|------|
| closeness | 0.100 | 0.146 | +0.046 | 변동 너무 작음 |
| trust | 0.100 | 0.180 | +0.080 | 변동 너무 작음 |
| power | -0.300 | -0.300 | 0 | 정상 |

---

## 5. 감정 아크 요약

```
Turn 1 (공포)     ■■■■■■■░░░  Distress 0.650
Turn 2 (안도)     ■■■■■■■■■░  Relief   0.858
Turn 3 (고백)     ■■■■■■■░░░  Fear     0.676
                  ■■■░░░░░░░  Shame    0.268
Turn 4 (약속)     ■■■■■■■■■■  Admiration 1.000
                  ■■■■■■■■■░  Satisfaction 0.858
```

감정 흐름: 공포 → 안도 → 두려움+수치 → 감탄+만족
이 아크는 소설 원문의 감정 변화와 일치함.

---

## 6. 프롬프트 출력 검증

### Turn 4 프롬프트 (최종 상태)

```
[NPC: Jim]
Miss Watson의 노예. 온순하고 감성적이며 미신을 깊이 믿는다.

[성격]
진실되고 공정하며 겸손한 성격이다. 감정이 풍부하고 불안해하기 쉬운 성격이다.
관용적이고 온화하며 인내심이 강하다.

[현재 감정]
지배 감정: 감탄(극도로 강한) — 헉이 자신의 평판을 걸고 도망 노예를 보호하겠다고 맹세하는 행위
활성 감정: 감탄(극도로 강한), 만족(극도로 강한) — 헉이 사회적 비난을 감수하고 비밀을 지키겠다고 약속함

[연기 지시]
어조: 편안하고 온화한 어조
태도: 호의적이고 개방적인 태도
행동 경향: 대화에 적극적으로 참여하고 협조한다.
```

**프롬프트 평가**: context 정보가 감정에 포함되어 LLM이 "왜 이 감정인지" 파악 가능. 성격 묘사와 연기 지시가 Jim의 캐릭터와 일치.

---

## 7. 종합 평가

### 잘 동작한 것

1. **감정 유형 선택 정확**: 4개 턴 모두 OCC 이론에 충실한 감정이 생성됨
2. **성격→감정 강도 반영**: Jim의 fearfulness(0.6)→공포 강화, sentimentality(0.8)→안도 강화 확인
3. **전망(prospect) 시스템**: anticipation→Fear, fear_unrealized→Relief, hope_fulfilled→Satisfaction 모두 정상
4. **context 전달**: 프롬프트에 "감탄(극도로 강한) — 헉이 자신의 평판을 걸고..." 형태로 원인 포함
5. **복합 심리 포착**: Turn 3에서 Fear+Shame 동시 생성이 장면의 복잡한 심리를 잘 표현
6. **프로필 설계 적합성**: Huck(자유분방+정직)과 Jim(감성적+두려움+관용적)의 대비가 원작과 부합

### 개선 필요 사항

| # | 이슈 | 우선순위 | 설명 |
|---|------|---------|------|
| 1 | Gratitude 누락 | 높음 | Turn 4에서 Gratitude가 없음. Compound 감정 Gratitude=Admiration+Joy인데, Joy 대신 Satisfaction이 나와서 compound 미생성. 직관적으로 Jim은 여기서 감사를 느껴야 함 |
| 2 | 관계 변동 너무 작음 | 높음 | closeness +0.046, trust +0.080 — 인생을 건 약속에 비해 변동 미미. after_dialogue 공식의 변동 계수를 검토 필요 |
| 3 | 턴 간 감정 누적 없음 | 중간 | 각 턴이 독립적 appraise. Turn 1의 Distress가 Turn 2의 Relief에 영향 안 미침. stateless 설계의 의도된 제약이나, stimulus를 통한 우회 워크플로우 필요 |
| 4 | power 라벨 오류 | 낮음 | power=-0.3인데 프롬프트에 "대등한 관계"로 표시. "하위자"가 올바름 |

### 다음 세션에서 할 것

1. 이슈 #1(Gratitude 누락) 수정 후 session_002로 재테스트
2. 이슈 #2(관계 변동 계수) 조정 후 비교
3. Huck의 관점에서도 동일 장면 테스트 (성격 대비 확인)
4. stimulus를 활용한 턴 간 감정 누적 워크플로우 실험
5. Ch.15 안개 장면으로 확장 (관계 파열/회복 테스트)

---

## 8. 파일 목록

| 파일 | 내용 |
|------|------|
| scenario.json | NPC 프로필 + 관계 + 오브젝트 (서버 상태 스냅샷) |
| evaluation.md | 간략 평가 + 프로필 설계 근거 |
| test_report.md | 본 레포트 (상세 테스트 결과) |
| turn1_ghost_fear.txt | Turn 1 프롬프트 출력 |
| turn2_relief.txt | Turn 2 프롬프트 출력 |
| turn3_confession.txt | Turn 3 프롬프트 출력 |
| turn4_promise.txt | Turn 4 프롬프트 출력 |
