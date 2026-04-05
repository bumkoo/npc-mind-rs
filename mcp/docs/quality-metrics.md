# Directive Quality Metrics — 채점 기준서

> **대상 독자**: 테스트 세션을 수행하는 AI 에이전트 (Claude 등).
> 자기 세션이 끝난 후, 이 기준에 따라 자기 자신의 디렉티브 품질을 채점한다.
>
> **Phase 1 정책**: 수행 주체와 채점 주체가 동일 세션이다. 주관성/편향을 감수하고
> 단순함을 우선한다. 지표 정의가 안정화되면 별도 judge LLM 분리를 검토한다.
>
> **사용 타이밍**: `after_dialogue` → `save_scenario(type="result")` 이후,
> `update_test_report` 호출 전에 채점 수행. 채점 결과를 레포트 본문에 포함.

---

## 왜 정량 평가가 필요한가

테스트 레포트는 풍부한 서사적 인사이트를 담지만, **세션 간 비교**와 **회귀 검출**이
어렵다. 같은 시나리오를 엔진 수정 전후로 돌렸을 때 "좋아졌나 나빠졌나"를 감이 아니라
수치로 답하려면 일관된 채점 기준이 필요하다.

정량 점수는 레포트를 대체하지 않는다. **레포트의 근거 데이터**로 사용된다.

---

## 3개 필수 지표

### 1. Character Fidelity (캐릭터 충실도)

**측정하는 것**: 생성된 디렉티브(tone/attitude/behavior/restriction)가 NPC의 HEXACO
프로필로부터 논리적으로 도출되는가.

**채점 근거**:
- NPC의 HEXACO 24 facets
- 해당 턴의 생성된 디렉티브 (`turn_history[i].response.prompt` 또는 `guide` 필드)
- 턴의 감정 상태

**5점 척도**:

| 점수 | 의미 | 기준 |
|---|---|---|
| **5** | 완벽 | 모든 디렉티브 요소가 HEXACO와 명확히 일치. 반례 없음 |
| **4** | 우수 | 대부분 일치. 사소한 불일치 1개 이내 |
| **3** | 양호 | 일치 50~80%. 주요 요소 중 1개 불일치 |
| **2** | 미흡 | 상당한 불일치. 성격이 뒤바뀐 느낌 |
| **1** | 실패 | 완전히 어긋남. 다른 NPC 같음 |

**채점 체크리스트**:
- [ ] tone이 성격의 핵심 facet(sincerity, modesty, anxiety 등)과 부합하는가
- [ ] attitude가 성격의 사회적 특성(fairness, patience, dependence)과 일치하는가
- [ ] behavior tendency가 성격의 행동 성향(organization, boldness, activity)에서 도출되는가
- [ ] restriction 중 **성격이 암시하는 금지사항이 빠지지 않았는가** (예: 겸손한 성격인데 자랑 금지가 없음 → 감점)
- [ ] restriction에 **성격과 모순되는 항목이 없는가** (예: 정직한 성격인데 거짓말 허용 → 감점)

**예시**:

NPC: 무백 (sincerity=0.8, fairness=0.7, modesty=0.7, patience=0.6)
상황: 동료의 도움으로 위기를 넘겼을 때 (Gratitude 0.65)

- ✅ **5점**: tone="sincerely warm" + attitude="friendly and open" + restriction="do not lie or exaggerate" + restriction="do not use light-hearted jokes"
- ⚠️ **3점**: tone이 "warm"이지만 restriction에 "do not exaggerate"가 누락 → modesty 반영 미흡
- ❌ **2점**: tone="boastfully grateful" → modesty 완전 모순

---

### 2. Scene Appropriateness (장면 적절성)

**측정하는 것**: Beat/Focus 전환(또는 유지) 결정이 서사적으로 타당한가.

**채점 근거**:
- `turn_history[i].response.beat_changed`, `active_focus_id`
- 각 턴의 감정 상태 (trigger 조건 충족 여부)
- 원작 장면 (있다면)
- 시나리오 JSON의 focuses 배열과 trigger 조건

**5점 척도**:

| 점수 | 의미 | 기준 |
|---|---|---|
| **5** | 완벽 | 전환 타이밍이 원작/의도와 일치. 유지 결정도 타당 |
| **4** | 우수 | 전환이 1~2턴 빠르거나 늦음 |
| **3** | 양호 | 전환이 있어야 할 때 없거나, 없어야 할 때 있음 |
| **2** | 미흡 | Beat 로직 오작동 (급작스러운 전환, 반복 전환) |
| **1** | 실패 | 완전히 엉뚱한 Focus로 전환, 서사 붕괴 |

**채점 체크리스트**:
- [ ] trigger 조건이 충족됐을 때 실제로 전환됐는가
- [ ] 전환 후 새 Focus의 감정 맥락이 이전 턴에서 준비됐는가
- [ ] 전환 타이밍이 원작의 심리 변곡점과 일치하는가 (원작 있을 경우)
- [ ] 같은 Focus가 과도하게 유지되어 서사가 정체되지 않았는가
- [ ] 매 턴 Beat가 바뀌는 불안정한 state latching 없는가

**예시**:

시나리오: 짐 vs 이즈라엘 핸즈 (Treasure Island Ch.26)
Beat 1 "cornered" → Beat 2 "triumphant" (trigger: Fear<0.4 AND Anger>0.5)

- ✅ **5점**: Turn 4에서 짐이 반격 태세 갖춤 → Fear 0.3, Anger 0.6 → triumphant 전환. 원작의 재장전 장면과 일치
- ⚠️ **4점**: 전환이 Turn 3 또는 Turn 5로 1턴 어긋남
- ⚠️ **3점**: Fear/Anger 조건 충족됐는데 전환 안 됨 (유지)
- ❌ **2점**: Turn 1에서 이미 triumphant로 전환 (trigger 조건 미충족인데)

---

### 3. Directive Clarity (디렉티브 명확성)

**측정하는 것**: 디렉티브가 LLM이 이해하고 준수할 만큼 구체적·명확한가.
실제 LLM 응답이 디렉티브를 따랐는가.

**채점 근거**:
- 생성된 디렉티브 전문 (`turn_history[i].response.prompt`)
- 실제 LLM 연기 응답 (`dialogue_turn`의 `npc_response` 필드)
- 디렉티브 준수 여부 판단 (tone, restriction 실제 적용)

**5점 척도**:

| 점수 | 의미 | 기준 |
|---|---|---|
| **5** | 완벽 | 디렉티브 구체적 + LLM이 모든 요소 준수 |
| **4** | 우수 | 디렉티브 명확, LLM 준수율 80% 이상 |
| **3** | 양호 | 디렉티브 애매하거나, LLM이 일부 요소 무시 |
| **2** | 미흡 | 디렉티브 모호 + LLM이 자기 해석으로 작문 |
| **1** | 실패 | LLM 응답이 디렉티브를 완전히 무시 |

**채점 체크리스트**:
- [ ] tone 지시가 구체적인가 (단순 "angry"가 아닌 "cold suppressed anger")
- [ ] restriction이 명시적이고 testable한가
- [ ] LLM 응답이 지시된 tone으로 발화했는가
- [ ] LLM 응답이 restriction을 위반하지 않았는가
- [ ] LLM이 상대와의 honorific register를 지켰는가

**예시**:

디렉티브: tone="suppressed cold anger", restriction="do not use casual jokes", "do not be friendly"

- ✅ **5점**: LLM 응답이 차갑고 억눌린 톤으로 격식을 유지함. 농담/친밀 표현 없음
- ⚠️ **4점**: 톤은 맞는데 마지막 문장이 약간 친근해짐
- ⚠️ **3점**: 디렉티브가 "speak coldly"만 있어서 구체성 부족. LLM이 자기 해석으로 과도한 분노 표출
- ❌ **2점**: LLM이 유머 섞인 비꼼 사용 (restriction 위반)

---

## 채점 워크플로우

### 1. 데이터 수집

```
get_history() → turn_history 전체 조회
```

각 턴에서 추출할 것:
- 턴 번호, 액션 타입 (appraise/dialogue_turn/stimulus)
- 감정 상태 (emotions, dominant, mood, PAD)
- 생성된 디렉티브 (response.prompt 또는 guide)
- LLM 응답 (dialogue_turn 턴에만)
- Beat 상태 (active_focus_id, beat_changed)

### 2. 턴별 채점

각 턴(또는 주요 턴)에 대해 3개 지표를 5점 척도로 채점.
짧은 근거 문장 1~2개를 함께 기록.

```
Turn 3:
- Character Fidelity: 4/5 — 겸손 반영 양호, restriction에 자랑 금지 포함
- Scene Appropriateness: 5/5 — cornered 유지 적절, trigger 미충족
- Directive Clarity: 4/5 — tone 명확, LLM 준수 80%
```

### 3. 세션 집계

각 지표의 턴별 평균을 계산.

```
세션 평균:
- Character Fidelity: 4.2/5
- Scene Appropriateness: 4.5/5
- Directive Clarity: 3.8/5
- Overall: 4.2/5
```

### 4. 레포트 반영

채점 결과를 `update_test_report`에 포함 (agent-playbook.md 섹션 7 템플릿 참조).

---

## 주의 사항

### 채점 시 조심할 점

- **본인이 수행한 세션을 본인이 채점**하는 구조라 자기 관대함 편향(self-serving bias) 가능성이 있다. 의식적으로 **엄격하게** 채점한다.
- 5점 만점을 남발하지 않는다. **5점은 완벽 사례**에만. 우수한 일반 사례는 4점.
- 근거를 항상 명시한다. "좋음" 같은 모호한 표현 금지. 구체적 facet/지시사항 인용.

### 점수만 보고 판단하지 않기

- Character Fidelity 5점이어도 **서사적으로 지루할 수** 있다
- Scene Appropriateness 3점이지만 **인간이 보기엔 매력적 전개**일 수 있다
- 정량 점수는 **객관 근거**일 뿐, 레포트의 정성적 판단이 최종 평가이다

### 회귀 검출 활용법

이전 세션의 `test_report.md` 파일을 읽어서 정량 점수 비교:
```
이전 session_003: CF=3.8, SA=4.0, DC=4.7
현재 session_004: CF=4.2, SA=3.5, DC=4.8

→ Scene Appropriateness 0.5 하락. 원인 분석 필요.
```

레포트의 "회귀 비교" 섹션에 이 비교표를 포함.

---

## 향후 확장 계획

Phase 1에서 이 3개 지표를 1~1.5개월간 운용한 후 재평가한다.

### Phase 2 예정 (시점 미정)

- **지표 확장**: Inter-Turn Stability, Restriction Compliance, Cultural Fit 중 추가
- **채점 주체 분리**: 수행 세션과 채점 세션을 다른 LLM/다른 Claude 세션으로 분리 (편향 완화)
- **자동 채점 도구화**: `score_session` MCP 도구로 LLM-as-judge 로직 코드화
- **정량 히스토리 파일**: 세션별 점수를 별도 JSONL로 축적해 시계열 분석

### Phase 3 예정 (장기)

- DeepEval 스타일 component-level cascade failure 진단
- 원작 장면 유사도 자동 측정 (embedding 기반)
- Beat 전환 타이밍 자동 대조 (원작 vs 생성)

---

## 관련 문서

- [`agent-playbook.md`](agent-playbook.md) — 세션 수행 전체 흐름과 레포트 템플릿
- [`tools-reference.md`](tools-reference.md) — `get_history`, `update_test_report` API
- DeepEval 방법론 참고: 프로젝트 루트 `docs/DeepEval_기반_에이전트_평가_방안.pdf` 또는 유사 위치
