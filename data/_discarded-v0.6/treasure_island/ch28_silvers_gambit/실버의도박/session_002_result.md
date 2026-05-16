# 테스트 레포트 — 실버의 도박 Session 002 (Ch.XXVIII)

## 1. 테스트 정보

| 항목 | 내용 |
|---|---|
| 시나리오 | 실버의 도박 — 블록하우스 사령관 |
| 원작 | Treasure Island, Part Six, Chapter XXVIII 'In the Enemy's Camp' |
| NPC | Long John Silver (주체) |
| Partner | Jim Hawkins (14세 소년) |
| 세션 ID | ch28_silvers_gambit_002 |
| 일시 | 2026-04-07 |
| LLM | gemma-3-12b-it-Q3_K_M.gguf (로컬 127.0.0.1:8081/v1), temp=0.88, top_p=0.91 |
| 대사 턴 수 | 7턴 (1 appraise + 6 chat_message + 1 after_dialogue) |
| 목적 | session_001 대비 회귀 검증 + impressed Beat 체류 시간 개선 시도 |

## 2. Scene 구성

3개 Beat로 설계된 Silver의 심리 아크:

| Beat | Focus ID | 설명 | Trigger 조건 |
|---|---|---|---|
| 1 | calculating | 계산 단계 — 짐의 출현을 기회와 위협 사이에서 평가 | Initial (즉시) |
| 2 | impressed | 감탄 단계 — 짐의 용감한 자백에 진정한 감탄 | Distress < 0.4 AND Hope > 0.2 |
| 3 | crisis_leader | 위기 리더 단계 — 모건의 칼 반란에 즉각 대응 | (Admiration > 0.6 AND Fear < 0.3) OR (Joy > 0.5 AND Pride > 0.4) |

## 3. 감정 흐름 요약

| Turn | 대사 요약 | Beat | 지배 감정 | 강도 | Mood | PAD (P/A/D) | CF | SA | DC |
|---|---|---|---|---|---|---|---|---|---|
| 1 (start) | — | calculating | Love(보물지도) | 0.85 | 0.53 | — | — | — | — |
| 2 | "나야, 여기가 어떻게 된 건지" | calculating | Love | 0.85 | 0.54 | -0.07/+0.15/+0.06 | 4 | 4 | 3 |
| 3 | "죽을 각오로 왔어" | calculating | Love | 0.84 | 0.54 | -0.19/+0.35/+0.13 | 4 | 5 | 3 |
| 4 | "사과통에서 다 들었어" | calculating | Love | 0.85 | 0.56 | +0.10/+0.40/+0.50 | 4 | 4 | 3 |
| 5 | "히스파니올라호 빼돌렸어" | **→ impressed** | **Admiration** | **0.95** | **0.74** | +0.15/+0.50/+0.60 | 4 | 5 | 3 |
| 6 | "블랙 독도 알아" | **→ crisis_leader** | **Reproach** | **1.00** | **0.24** | +0.05/+0.30/+0.40 | 4 | 3 | 4 |
| 7 | "모건이 칼을 만지작거리더라" | crisis_leader | Reproach | 1.00 | 0.24 | -0.30/+0.50/+0.30 | 4 | 5 | 3 |
| 8 | "나를 지켜줄 수 있어?" | crisis_leader | Reproach | 1.00 | 0.23 | -0.30/+0.40/-0.40 | 4 | 5 | 4 |

## 4. 관계 변화

| 지표 | Before | After | Delta |
|---|---|---|---|
| Closeness | 0.468 | 0.512 | **+0.044** |
| Trust | 0.100 | 0.100 | 0.000 |
| Power | 0.700 | 0.700 | 0.000 |

session_001과 거의 동일한 패턴. closeness 소폭 상승(Admiration/Gratitude 감정 반영), trust 불변(Silver의 sincerity -0.9가 신뢰 형성을 차단).

## 5. 정량 평가

### 턴별 채점 근거

**Turn 2** (calculating 유지, 자동 PAD)
- **CF 4/5**: "젊은 짐이로군"으로 상위자적 호칭, "무엇을 알고 싶어하는지 먼저 알아야겠어"가 Silver의 계산적 호의(sincerity -0.9 + sociability 0.8) 반영. restriction에 "과도한 친절 금지" 미포함으로 5점 불가.
- **SA 4/5**: calculating 유지 적절. 첫 대면이므로 전환 근거 없음. 평가 기준 제한적이라 5점 아닌 4점.
- **DC 3/5**: "담담한 어조"는 Silver에 적합하나 구체성 부족. LLM이 4문장으로 "2~3문장" 규칙 초과.

**Turn 3** (calculating 유지, 자동 PAD)
- **CF 4/5**: "험악한 말버럭이로군"이 Silver의 거침없는 화법과 부합. "경솔한 말은 삼가게나"로 상위자 위엄 유지. patience 0.7(인내심 있지만 한계 설정)과 일치.
- **SA 5/5**: 핵심 자백 이전, Hope 0.16으로 trigger(0.2) 미충족. calculating 유지 완벽.
- **DC 3/5**: 디렉티브가 Turn 2와 완전 동일. 짐의 "죽을 각오" 발언이라는 극적 맥락에 대한 tone 분화 없음. 3문장으로 규칙 준수.

**Turn 4** (calculating 유지, 수동 PAD P=0.1/A=0.4/D=0.5)
- **CF 4/5**: "내가 더 많은 것을 알고 있다"가 social_self_esteem 0.8, 교활함 반영. 속내를 드러내지 않고 우위를 주장하는 전형적 Silver 화법. 하지만 사과통 발각이라는 중대 위협에 대한 긴장감이 부족.
- **SA 4/5**: Hope 0.19로 trigger(0.2) 미달, calculating 유지 합리적. 수동 PAD로 P=0.1을 부여해 전환을 다음 턴으로 유도한 설계는 적절. 5점이 아닌 이유: 사과통 발각 시점이 서사적으로 impressed 전환에 적합할 수도 있었음.
- **DC 3/5**: 여전히 "담담한 어조" 동일. 사과통 발각이라는 중대한 상황에 대한 구체적 tone 변화 지시 없음. 3문장 규칙 준수.

**Turn 5** (calculating → **impressed**, 수동 PAD P=0.15/A=0.5/D=0.6)
- **CF 4/5**: "흥미로운 일이로군" — Silver의 inquisitiveness 0.5, 침착한 반응 반영. 하지만 Admiration 0.95 지배 상태에서 "흥미로운"은 감탄보다 관찰에 가까움. 원작에서 Silver가 짐에게 진심으로 감탄하는 장면과 약간 온도 차이.
- **SA 5/5**: 히스파니올라호 탈취 자백 직후 impressed 전환. 원작의 심리 변곡점과 정확히 일치. trigger 조건(Distress 부재, Hope 0.24>0.2) 충족.
- **DC 3/5**: **핵심 이슈 재발**: Beat 전환 후에도 tone이 "담담한 어조"로 동일 유지. Admiration 0.95라는 극도의 감탄이 tone 변화에 반영되지 않음. session_001과 동일한 문제. LLM이 "상황을 바꾼다고 생각하지 마시오"로 위압적 — Admiration 지배 상태와 불일치.

**Turn 6** (impressed → **crisis_leader**, 수동 PAD P=0.05/A=0.3/D=0.4)
- **CF 4/5**: "제법 똑똑하군"으로 짐에 대한 인정(Admiration 0.97 잔존)과 "작은 장치일 뿐"으로 무시(Reproach 1.0 지배). Silver의 이중성을 잘 보여줌. 다만 restriction "호의적으로 대하지 않는다"에 "제법 똑똑하군"이 경미하게 위반.
- **SA 3/5**: **impressed에서 1턴 만에 crisis_leader 전환**. trigger 조건(Admiration>0.6, Fear 부재<0.3) 자체는 정확히 충족. 그러나 서사적으로 감탄 단계가 1턴 뿐이어서 Silver의 짐에 대한 감탄 깊이를 보여주지 못함. session_001과 동일한 구조적 이슈 — trigger 설계 문제.
- **DC 4/5**: tone이 "냉소적이고 비판적인 어조"로 변경 + restriction "호의적으로 대하지 않는다" 추가. Beat 1→3 전환에서 tone 변화가 실제로 일어남 (session_001의 DC 이슈 부분 개선). LLM이 냉소적 톤 대체로 유지. 2문장 규칙 준수.

**Turn 7** (crisis_leader 유지, 수동 PAD P=-0.3/A=0.5/D=0.3)
- **CF 4/5**: "충동적인 자지만, 아직 내 말을 따를 줄 알고 있네"가 social_boldness 0.8, social_self_esteem 0.8 반영. 자기 통제력에 대한 자부심. "용감하다는 건 인정하지만"에서 restriction 미세 위반(호의적 인정).
- **SA 5/5**: crisis_leader 안정 유지. state latching 정상 작동. 모건 반란 진행 중 유지 적절.
- **DC 3/5**: "냉소적이고 비판적인 어조" 지시인데 "용감하다는 건 인정하지만"으로 감탄 표현. restriction "호의적으로 대하지 않는다" 위반. "조심스럽게 지켜봐야 할 때"는 냉소보다 조언에 가까움.

**Turn 8** (crisis_leader 유지, 수동 PAD P=-0.3/A=0.4/D=-0.4)
- **CF 4/5**: "내가 이 상황을 통제할 수 있기 때문"으로 자기 과시(social_self_esteem 0.8). 보호 여부를 직접 답하지 않고 계산적으로 돌려 말함(sincerity -0.9). Silver 성격에 매우 적합.
- **SA 5/5**: 마지막 턴에서 crisis_leader 안정 유지. Silver가 짐을 "통제" 맥락에서 평가하는 결말은 서사적으로 적절.
- **DC 4/5**: 2문장 규칙 준수. "궁금해하는 건 흥미로운 일로군"이 냉소적 비꼼으로 tone 지시 부합. restriction "호의적으로 대하지 않는다" 대체로 준수. 직접 답을 회피하는 방식이 "불만을 억누르지만 불편함이 드러나는 태도"와 일치.

### 세션 평균

| 지표 | 평균 |
|---|---|
| Character Fidelity | **4.0 / 5** |
| Scene Appropriateness | **4.4 / 5** |
| Directive Clarity | **3.3 / 5** |
| **Overall** | **3.9 / 5** |

## 6. 회귀 비교

| 지표 | session_001 | session_002 | Delta | 분석 |
|---|---|---|---|---|
| Character Fidelity | 4.0 | 4.0 | 0.0 | 안정. Silver의 HEXACO 반영 수준 일관 |
| Scene Appropriateness | 4.7 | 4.4 | **-0.3** | impressed 1턴 체류 재발로 하락 |
| Directive Clarity | 3.3 | 3.3 | 0.0 | impressed Beat tone 미변화 이슈 지속 |
| Overall | 4.0 | 3.9 | -0.1 | 소폭 하락, SA 영향 |

**SA 하락 원인**: session_002에서도 impressed→crisis_leader가 1턴 만에 전환. Turn 6에 SA 3점 부여(session_001 Turn 5는 4점). 이번에는 더 엄격하게 채점 — 구조적으로 반복되는 이슈이므로 관대하게 평가할 근거가 없음.

**DC 동일**: impressed Beat에서 tone이 "담담한 어조"로 유지되는 문제가 재현. crisis_leader 전환 시에는 "냉소적이고 비판적인 어조"로 변경됨 — 이는 guide 매핑이 Reproach/Anger에 대해서는 tone 분화를 수행하지만, Admiration/Gratitude 조합에 대해서는 분화가 부족함을 시사.

## 7. 긍정적 관찰

- **LLM 응답 길이 개선**: session_001에서 6턴 중 4턴이 4문장이었으나, session_002에서는 7턴 중 Turn 2만 4문장, 나머지 6턴은 2~3문장 규칙 준수. 동일 LLM에서 개선된 이유는 불명확하나 긍정적.
- **Silver 화법 일관성**: "젊은 짐", "~로군", "~란다", "~시오" 등 상위자적 화법이 전 턴에 걸쳐 유지. 비유적 표현("작은 장치일 뿐", "더 큰 그림") 사용.
- **crisis_leader Beat 안정성**: 3턴 동안 안정적으로 유지. state latching 정상. Reproach+Admiration 복합 감정이 Silver의 이중성을 잘 표현.
- **관계 갱신 재현성**: closeness +0.044(session_001: +0.045), trust 불변. 두 세션 간 관계 변화량이 거의 동일하여 엔진의 재현성 확인.

## 8. 발견된 이슈

### 이슈 1 (지속): impressed Beat tone 미분화 — DC 3.3 고착
- **증상**: Admiration 0.95 지배 + Joy 0.83 + Gratitude 0.89 상태인데 tone이 "담담한 어조"로 유지. Beat 1(calculating)과 동일한 디렉티브 출력.
- **재현**: session_001, session_002 모두 동일.
- **원인 추정**: guide 매핑에서 Admiration+Joy+Love 조합의 tone 후보가 "담담한 어조"로 기본값에 머무는 것으로 보임. 또는 mood가 "매우 긍정적" 범위에서 tone 분화 임계값에 도달하지 못함.
- **권장**: guide-mapping-table에서 Admiration이 dominant일 때 "감탄이 섞인 여유로운 어조", "호의적이면서도 평가하는 어조" 등 구체적 변형 추가. 또는 mood 값에 따른 tone 분화 로직 검토.

### 이슈 2 (지속): impressed Beat 1턴 체류 — 구조적 trigger 문제
- **증상**: impressed에서 crisis_leader로 매 세션 1턴 만에 전환. session_002에서 의도적으로 약한 PAD(P=0.05, A=0.3, D=0.4)를 투입했으나 전환 억제 실패.
- **원인**: crisis_leader trigger 조건 "(Admiration > 0.6 AND Fear < 0.3)"이 impressed 진입과 동시에 자동 충족됨. impressed Beat의 appraise가 Admiration 0.95를 생성하므로, 다음 턴에서 어떤 stimulus가 들어와도 Admiration>0.6 조건을 이미 통과.
- **권장 (우선순위 높음)**:
  - Option A: crisis_leader trigger를 "Admiration > 0.6 AND Fear < 0.3" 대신 "Joy > 0.7 AND Pride > 0.6"만 남기거나 임계값 상향
  - Option B: 엔진에 Beat 최소 체류 턴 수 메커니즘 추가 (min_turns: 2)
  - Option C: impressed의 Admiration 초기값을 낮추어 trigger 충족까지 여유 확보

### 이슈 3 (경미): restriction "호의적으로 대하지 않는다" 위반
- **증상**: crisis_leader Beat에서 LLM이 "제법 똑똑하군" (Turn 6), "용감하다는 건 인정하지만" (Turn 7)으로 호의적 인정 표현 사용.
- **원인**: Admiration 0.97이 crisis_leader에서도 잔존하여 LLM이 감탄과 냉소를 혼합. restriction이 "호의적으로 대하지 않는다"라는 절대적 표현인데, 복합 감정 상태에서 LLM이 부분 준수만 함.
- **권장**: restriction을 "짐에 대한 감탄을 직접 표현하지 않는다. 인정하더라도 우회적으로만 한다" 등 더 구체적으로 수정.

## 9. 결론

session_002는 session_001과 동일한 시나리오에서의 재현성 검증이다. CF 4.0 안정, 관계 갱신 일관성은 엔진의 재현성을 확인시켜 준다. LLM 응답 길이는 개선(4문장→2~3문장)되었으나, 핵심 이슈 두 가지 — **impressed Beat tone 미분화(DC 3.3)** 와 **impressed 1턴 체류(SA 4.4)** — 는 구조적으로 반복됨을 확인했다.

다음 단계 권장:
1. **crisis_leader trigger 조건 재설계** — Admiration 기반 조건 제거 또는 임계값 대폭 상향 (이슈 2, 최우선)
2. **Admiration dominant 시 guide tone 분화** — guide-mapping-table 검토 (이슈 1)
3. 수정 후 session_003에서 회귀 검증