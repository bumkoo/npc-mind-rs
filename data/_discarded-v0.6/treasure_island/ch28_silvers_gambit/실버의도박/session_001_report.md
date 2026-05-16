# 테스트 레포트 — 실버의 도박 (Ch.XXVIII)

## 1. 테스트 정보

| 항목 | 내용 |
|---|---|
| 시나리오 | 실버의 도박 — 블록하우스 사령관 |
| 원작 | Treasure Island, Part Six, Chapter XXVIII 'In the Enemy's Camp' |
| NPC | Long John Silver (주체) |
| Partner | Jim Hawkins (14세 소년) |
| 세션 ID | ch28_silvers_gambit_001 |
| 일시 | 2026-04-06 |
| LLM | 로컬 서버 (127.0.0.1:8081/v1), temp=0.88, top_p=0.91 |
| 대사 턴 수 | 6턴 (chat_message) + 1 appraise + 1 after_dialogue |

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
| 2 | "나야, 짐이야" | calculating | Love | 0.85 | 0.54 | +0.07/0.00/+0.31 | 4 | 4 | 3 |
| 3 | "죽는다고 생각하고 왔어" | calculating | Love | 0.85 | 0.54 | -0.07/+0.20/0.00 | 4 | 5 | 3 |
| 4 | "사과통에서 계획 들었어" | **→ impressed** | **Admiration** | **0.95** | **0.73** | +0.10/+0.50/+0.60 | 4 | 5 | 3 |
| 5 | "핸즈를 내가 처리했어" | **→ crisis_leader** | **Reproach** | **1.00** | **0.24** | +0.20/+0.40/+0.70 | 4 | 4 | 4 |
| 6 | "누굴 통제하는 건지 모르겠어" | crisis_leader | Reproach | 1.00 | 0.24 | -0.30/+0.60/+0.30 | 4 | 5 | 3 |
| 7 | "나를 지켜줄 수 있어?" | crisis_leader | Reproach | 1.00 | 0.23 | -0.40/+0.50/-0.50 | 4 | 5 | 4 |

## 4. 관계 변화

| 지표 | Before | After | Delta |
|---|---|---|---|
| Closeness | 0.465 | 0.510 | **+0.045** |
| Trust | 0.100 | 0.100 | 0.000 |
| Power | 0.700 | 0.700 | 0.000 |

closeness 소폭 상승은 장면의 높은 significance(0.95)와 Admiration/Gratitude 감정이 반영된 결과. trust 불변은 Silver의 근본적 불신(sincerity -0.9)이 짐에 대한 감탄에도 불구하고 신뢰를 쉽게 허락하지 않음을 보여준다.

## 5. 정량 평가

### 턴별 채점 근거

**Turn 2** (calculating 유지)
- **CF 4/5**: "불쾌할 이유는 없지"가 Silver의 계산적 호의(sincerity -0.9 + sociability 0.8)와 부합. 속내를 숨기며 상대를 관찰하는 tone 적절. 다만 restriction에 "과도한 친절 금지" 미포함.
- **SA 4/5**: calculating 유지 적절. 첫 턴이라 전환 근거 없음. 5점이 아닌 이유: 첫 대면이므로 평가 기준이 제한적.
- **DC 3/5**: "담담한 어조"는 Silver에게 적합하지만 구체성 부족. LLM이 3문장 이상(4문장)으로 응답 규칙 약간 초과.

**Turn 3** (calculating 유지)
- **CF 4/5**: "어리둥절할 따름이야"가 Silver의 patience 0.7에 부합하나, "어리둥절"이라는 표현은 Silver의 냉정한 이미지와 약간 불일치. 전체적으로 침착하게 듣는 자세는 적절.
- **SA 5/5**: 핵심 자백 이전이므로 calculating 유지 완벽. trigger 조건(Distress<0.4 AND Hope>0.2) 미충족 상태 확인됨.
- **DC 3/5**: 디렉티브가 Turn 2와 동일하여 대사 맥락의 변화(죽음 각오)에 대한 구체적 지시 부재. LLM이 4문장으로 규칙 초과.

**Turn 4** (calculating → **impressed** Beat 전환)
- **CF 4/5**: Admiration 0.95가 Silver의 inquisitiveness 0.5, creativity 0.7과 부합 — 영리한 상대를 알아보는 것은 자연스러움. "파이프 담배 연기처럼 허무하게 사라질 줄 알았지" 같은 비유는 creativity 0.7 반영.
- **SA 5/5**: 짐의 핵심 자백(사과통 음모 + 히스파니올라호 탈취) 직후 impressed 전환. 원작의 심리 변곡점과 정확히 일치. trigger 조건(Distress 부재, Hope 0.15→0.20) 충족.
- **DC 3/5**: Beat 전환 후에도 tone이 "담담한 어조"로 동일 유지. Admiration 0.95라는 극도의 감탄이 tone 변화에 반영되지 않음이 아쉬움. LLM이 4문장으로 장황.

**Turn 5** (impressed → **crisis_leader** Beat 전환)
- **CF 4/5**: Reproach 1.0 + Anger 0.79에 "냉소적이고 비판적인 어조" 배정 적절. "자네의 용맹함은 인정하겠네"로 Admiration 유지 반영, "잊혀질 일이 아니야"로 경고. Silver의 patience 0.7(인내심 있지만 선 넘으면 단호)과 부합.
- **SA 4/5**: impressed에서 1턴 만에 crisis_leader로 급전환. trigger 조건(Admiration>0.6 AND Fear<0.3) 충족은 정확. 서사적으로 짐의 도발이 해적들의 분노를 촉발하는 것은 자연스러우나, impressed 단계가 1턴 뿐이어서 감탄의 깊이를 보여줄 여유가 부족했다.
- **DC 4/5**: "냉소적이고 비판적인 어조" + restriction "호의적으로 대하지 않는다"가 구체적. LLM이 감탄 인정 + 경고를 적절히 혼합. 다만 "자네가 저지른 행동은 잊혀질 일이 아니야"가 위협인지 경고인지 모호.

**Turn 6** (crisis_leader 유지)
- **CF 4/5**: "내 자신의 길을 만들어가는 자" — social_self_esteem 0.8, social_boldness 0.8 잘 반영. Silver의 자부심과 독립성이 드러남.
- **SA 5/5**: 모건 반란 진행 중 crisis_leader 유지 적절. state latching 정상 작동.
- **DC 3/5**: LLM 응답이 "냉소적이고 비판적"이라기보다 "자신감 있고 과시적"에 가까움. restriction "호의적으로 대하지 않는다"는 준수했으나, "자네가 보는 것은 내 계획의 한 부분" 같은 표현은 불필요한 장광설로 보임.

**Turn 7** (crisis_leader 유지)
- **CF 4/5**: "자네의 가치를 알아본다면, 자네를 활용할 수도 있지" — sincerity -0.9(불성실), greed_avoidance -0.8(탐욕) 잘 반영. 자기 이익 중심의 계산적 태도.
- **SA 5/5**: 마지막 턴에서 crisis_leader 안정 유지. 서사적으로 Silver가 짐을 협상 카드로 평가하는 결말 적절.
- **DC 4/5**: restriction "호의적으로 대하지 않는다" 준수. "굳이 지켜줄 필요는 없지만"의 냉소적 톤. 2~3문장 규칙 준수. LLM의 마지막 질문("자네는 내게 무엇을 줄 수 있는가?")이 Silver의 협상가적 면모를 잘 보여줌.

### 세션 평균

| 지표 | 평균 |
|---|---|
| Character Fidelity | **4.0 / 5** |
| Scene Appropriateness | **4.7 / 5** |
| Directive Clarity | **3.3 / 5** |
| **Overall** | **4.0 / 5** |

## 6. 회귀 비교

이전 세션 없음 (첫 테스트). 이 세션이 baseline이 된다.

## 7. 긍정적 관찰

- **Beat 전환 타이밍 우수**: 3개 Beat가 모두 의도대로 전환됨. 특히 Turn 4의 impressed 전환은 원작의 짐 자백 장면과 정확히 일치.
- **감정 복합성**: crisis_leader 단계에서 Reproach 1.0 + Admiration 0.98이 공존하는 복합 감정 상태가 Silver의 이중적 성격을 잘 표현. 모건에 대한 분노와 짐에 대한 감탄이 동시에 존재하는 것은 원작의 핵심.
- **관계 갱신 합리성**: closeness만 소폭 상승(+0.045), trust 불변. Silver가 짐에게 호감은 갖되 신뢰는 주지 않는 원작 캐릭터성과 일치.
- **LLM 연기 일관성**: Silver의 비유적 화법("파이프 담배 연기처럼"), 상위자적 말투("자네"), 계산적 질문("무엇을 줄 수 있는가?")이 전 턴에 걸쳐 유지됨.

## 8. 발견된 이슈

### 이슈 1: Directive Clarity 전반적 미흡 (DC 평균 3.3)
- **증상**: Beat 1→Beat 2 전환 시 tone이 "담담한 어조"로 동일 유지. Admiration 0.95라는 극도의 감탄이 tone 변화로 이어지지 않음.
- **원인 추정**: 엔진의 guide 매핑에서 Admiration + Joy + Love 조합에 대한 tone 분화가 부족할 수 있음. 또는 mood가 "매우 긍정적" 범위에 머물러 tone 변화 임계값에 도달하지 못했을 가능성.
- **권장**: guide 매핑 테이블에서 Admiration이 dominant일 때의 tone 후보를 검토. "감탄이 섞인 여유로운 어조" 등 구체적 변형 추가 고려.

### 이슈 2: LLM 응답 길이 규칙 위반
- **증상**: 6턴 중 4턴에서 3문장 이상(4문장) 응답. "2~3문장으로 간결하게" 규칙 초과.
- **원인 추정**: 로컬 LLM의 instruction following 능력 한계. 또는 system prompt에서 규칙이 마지막에 위치하여 attention 약화.
- **권장**: 응답 규칙을 system prompt 상단으로 이동하거나, "최대 3문장" 등 hard limit 표현 강화.

### 이슈 3: impressed Beat 체류 시간 짧음 (1턴)
- **증상**: impressed에서 crisis_leader로 1턴 만에 전환. 감탄의 서사적 깊이를 보여줄 여유 부족.
- **원인 추정**: Turn 5의 수동 PAD(P=0.2, A=0.4, D=0.7)가 Joy/Pride를 빠르게 끌어올려 trigger 조건(Joy>0.5 AND Pride>0.4) 즉시 충족.
- **권장**: crisis_leader trigger 임계값 상향 조정(예: Joy>0.7 AND Pride>0.6) 또는 Beat 최소 체류 턴 수 메커니즘 검토.

## 9. 결론

Silver의 3단계 심리 아크(계산 → 감탄 → 위기 관리)가 엔진에 의해 성공적으로 구현되었다. Beat 전환 로직과 감정 복합성은 우수(SA 4.7)하며, HEXACO 프로필과의 일치도도 양호(CF 4.0)하다. 핵심 개선 영역은 **Directive Clarity(3.3)** — 특히 Beat 전환 시 tone/attitude의 구체적 변화와 LLM 응답 길이 제어이다. 다음 세션에서는 impressed 단계의 대사 턴을 2~3회로 늘려 감탄 → 위기의 서사적 전환 깊이를 검증할 필요가 있다.