# MCP Testing Guide — NPC Mind Engine

> **대상 독자**: AI 에이전트 (Claude, Claude Code 등). Mind Studio MCP 도구를 사용해 시나리오 테스트를 수행할 때 이 문서를 참조한다.
> **전제**: 협업 철학과 역할 분담은 [`docs/collaboration-workflow.md`](docs/collaboration-workflow.md) 참조.

---

## 핵심 원칙

**AI는 테스트 수행 시 다음 세 가지 원칙을 반드시 준수한다.**

### 원칙 1: 시나리오는 한국어가 기본
- NPC `description`, Scene/Focus `description`, Event/Action `description` 모두 한국어로 작성한다
- 테스트 레포트(`update_test_report`)도 한국어로 작성한다
- PAD 앵커는 한국어 버전을 사용하므로 한국어 대사가 자동 분석 정확도가 높다
- 원작이 영어(예: Treasure Island, Huckleberry Finn)여도 **엔진에 입력하는 description·대사는 한국어로 번역**하여 사용한다

### 원칙 2: NPC는 Asset을 만들고 활용함
- NPC 프로필은 일회성으로 생성하지 않고 **재사용 가능한 Asset으로 저장**한다
- 저장 위치: `data/{book}/assets/npcs/{character_id}.json`
- 관계 초깃값도 별도 Asset으로: `data/{book}/assets/relationships/`
- 테스트 시작 전 반드시 기존 Asset 확인:
  ```
  1. data/{book}/assets/npcs/ 디렉토리 조회
  2. 필요한 NPC가 있으면 → 해당 asset 로드
  3. 없으면 → 새로 생성하여 data/{book}/assets/npcs/에 저장
  4. 시나리오에서는 asset을 참조하여 사용
  ```
- 같은 인물이 여러 시나리오에 등장해도 HEXACO 프로필은 하나만 유지한다
- Asset 수정 시 영향받는 시나리오 목록을 함께 검토한다

### 원칙 3: 사용자 대사 입력 시 상황 지문 넣지 않음
- `dialogue_turn(utterance=...)`에 입력하는 대사는 **순수 발화만** 포함한다
- **금지**: `*핸즈가 으르렁거리며 단검을 들고 돌진해온다* "이 녀석!" *고함친다*`
- **권장**: `"이 녀석! 거기 서라!"`
- 이유:
  - 상황 지문은 `situation.description`과 `focus.description`에 있어야 함 (분리된 책임)
  - PAD 자동 분석이 지문 텍스트에 흔들리지 않음 (순수 대사 톤만 분석)
  - LLM 연기 시 동일 정보 중복 입력 방지
- 상황 변화가 필요하면 `situation_description` 파라미터 또는 Scene의 Beat 전환을 활용한다

---

## 0. 전제 조건

### 서버 실행
```powershell
cargo run --release --features mind-studio,embed --bin npc-mind-studio
```
- 포트: `http://127.0.0.1:3000`
- SSE MCP 엔드포인트: `/mcp/sse`
- `embed` feature 필수 — `analyze_utterance`에서 BGE-M3 임베딩 사용
- `chat` feature — `dialogue_*` 도구 사용 시 필요 (LLM 연기)

### MCP 연결 확인
AI는 MCP 도구 목록에 `npc-mind-studio:*`가 보이면 연결된 상태이다.
서버가 재시작되면 세션 상태(appraise 결과 등)가 초기화되므로 `load_scenario`부터 다시 시작한다.

---

## 1. 테스트 세션 전체 흐름

```
0. NPC Asset 확인           data/{book}/assets/npcs/ 에서 기존 Asset 조회 (원칙 2)
1. load_scenario              시나리오 파일 로드 (NPC/관계/Scene 복원)
2. get_scene_info             Scene 활성화 확인 (has_scene: true, active_focus_id)
3. dialogue_start             세션 시작 + 초기 appraise (script_cursor 0 초기화)
4. [test_script 있으면] get_next_utterance → dialogue_turn(utterance=대사) × N
   [test_script 없으면] dialogue_turn(utterance=즉흥 대사) × N
                              ※ 대사는 순수 발화만 (원칙 3)
5. dialogue_end               (선택) 세션 종료
6. after_dialogue             관계 갱신 (closeness/trust 수치 변동)
7. save_scenario(type="result")  결과 JSON 먼저 저장 (턴 히스토리 포함)
8. 정량 평가 수행              get_history → 3개 지표 채점 (quality-metrics.md 참조)
                              - Character Fidelity (캐릭터 충실도)
                              - Scene Appropriateness (장면 적절성)
                              - Directive Clarity (디렉티브 명확성)
9. update_test_report         분석 레포트 작성 (한국어, 원칙 1, 정량 섹션 포함)
10. save_scenario(type="all") result JSON + report MD 동시 저장
```

**정량 평가는 원칙이다** — 특별한 요청이 없어도 세션마다 수행한다. 채점 근거는
[`quality-metrics.md`](quality-metrics.md)에 정의되어 있다.

**LLM 연기 없이 엔진만 검증할 때** (`chat` feature 없음):
```
1. load_scenario
2. appraise                   초기 상황 → 감정 생성
3. analyze_utterance          대사 → PAD 추출
4. apply_stimulus             PAD → 감정 갱신 + Beat 전환
5. (3-4 반복)
6. after_dialogue
7. save_scenario
```

---

## 2. NPC Asset 관리 (원칙 2 상세)

### 저장 구조
```
data/{book}/assets/
├── npcs/              # NPC HEXACO 프로필
└── relationships/     # 관계 초깃값 (closeness/trust/power)
```

**실제 예시**:
```
data/wuxia_world/assets/npcs/
├── mu_baek.json        # 무백 (정직한 검객)
├── gyo_ryong.json      # 교룡 (반항적 여검객)
├── shu_lien.json       # 수련 (절제의 여검객)
└── so_ho.json          # 소호 (자유로운 낭인)

data/treasure_island/assets/npcs/
├── jim.json            # 짐 호킨스
└── israel_hands.json   # 이즈라엘 핸즈

data/huckleberry_finn/assets/npcs/
└── ...
```

### Asset 사용 흐름
```
1. data/{book}/assets/npcs/ 조회
2. 필요한 캐릭터가 있으면:
   - 해당 JSON 읽어서 create_full_scenario의 npcs 필드에 포함
3. 없으면:
   - HEXACO 24 facets 초안 작성 (한국어 description, 원칙 1)
   - data/{book}/assets/npcs/{id}.json에 저장
   - 이후 시나리오에서 재사용
```

### Asset 관리 원칙
- 테스트 중 성격 조정이 필요하면 **시나리오 파일이 아니라 asset 파일을 수정**한다
- asset 수정 시 해당 인물이 등장하는 시나리오 전체에 영향이 있으므로 변경 이유를 기록한다
- 같은 인물을 약간 다르게 표현하고 싶으면 새 asset으로 분기한다 (예: `billy_bones_drunk.json`)
- 작품 간에는 인물을 공유하지 않는다 (같은 이름이어도 작품별로 개별 asset 생성)

### Asset JSON 예시 (treasure_island/assets/npcs/jim.json)
```json
{
  "id": "jim",
  "name": "Jim Hawkins",
  "description": "13세 소년. 여관집 아들. 호기심 많고 모험심이 강하다. 도덕적이고 정직하지만, 위기 순간에 기지를 발휘한다.",
  "sincerity": 0.5, "fairness": 0.6, "greed_avoidance": 0.4, "modesty": 0.4,
  "fearfulness": 0.5, "anxiety": 0.4, "dependence": 0.3, "sentimentality": 0.3,
  "...": "..."
}
```

---

## 3. 각 단계 상세

### 2.1 시나리오 로드
```
load_scenario(path="treasure_island/ch26_mast_duel/짐vs이즈라엘.json")
```
- 경로는 `data/` 하위 상대 경로
- 로드 후 `get_scenario_meta`로 로드 확인

### 2.2 Scene 상태 확인
```
get_scene_info()
```
**기대 응답**:
```json
{
  "has_scene": true,
  "active_focus_id": "cornered",
  "trigger_display": "(Fear > 0.7) OR (Distress > 0.7)"
}
```
`has_scene: false`이면 Scene이 로드되지 않음 — 시나리오 JSON에 `scene` 필드 확인.

### 2.3 대화 세션 시작
```
dialogue_start(session_id="unique_id_t1", npc_id="jim", partner_id="israel_hands")
```
- `session_id`는 이 세션에서 고유해야 함
- 서버가 자동으로 initial focus에 대한 appraise 수행
- 응답에 초기 감정 + dominant + mood 포함

### 2.4 대화 턴 실행

**test_script가 있는 경우** — `get_next_utterance` → `dialogue_turn` 순서:
```
get_next_utterance()
→ { utterance: "핸즈, 뒤에서 무슨 소리가 났어...", index: 0, remaining: 2, exhausted: false }

dialogue_turn(
  session_id="unique_id_t1",
  npc_id="jim",
  partner_id="israel_hands",
  utterance="핸즈, 뒤에서 무슨 소리가 났어..."
)
```
- `get_next_utterance(advance=false)` → peek만 (커서 전진 없음)
- `exhausted: true`이면 스크립트 소진 → 즉흥 대사로 전환하거나 Beat 전환 대기
- Beat 전환 시 커서 자동 리셋 → 새 Beat의 test_script 처음부터 사용

**test_script가 없는 경우** — 즉흥 대사 직접 입력:
```
dialogue_turn(
  session_id="unique_id_t1",
  npc_id="jim",
  partner_id="israel_hands",
  utterance="순수 대사만 입력 (지문 없이)",
  pad={pleasure: -0.5, arousal: 0.9, dominance: 0.4}  // 선택
)
```
- `pad` 생략 시 → `utterance`를 자동 분석 (embed feature)
- LLM 연기 응답 + 감정 갱신 결과 + `beat_changed` + `active_focus_id` 반환
- **대사 작성 규칙** (원칙 3):
  - ❌ 금지: `*놀라며* "무슨 소리야?" *주위를 둘러본다*`
  - ✅ 권장: `"무슨 소리야?"`
  - 상황 변화는 `situation_description` 파라미터나 Scene Beat 전환으로 표현

### 2.5 관계 갱신
```
after_dialogue(req={npc_id: "jim", partner_id: "israel_hands", significance: 1.0})
```
- `significance`: 0.0~1.0, pivotal scene은 1.0
- 응답에 before/after 관계 수치 포함

### 2.6 결과 저장
```
save_scenario(
  path="treasure_island/ch26_mast_duel/짐vs이즈라엘_result/test_001.json",
  save_type="all"
)
```
- `save_type="all"` 사용 시 자동으로 `.json`과 `.md` 두 파일 저장
- `.md` 파일 경로는 `.json`의 확장자만 스왑

---

## 4. PAD 입력 규칙

### 자동 분석 사용 (기본값)
```
dialogue_turn(..., utterance="대사")  // pad 생략
```
- 한국어 대사 → 한국어 앵커로 분석
- 자연스러운 대사 흐름에 적합

### 수동 PAD 입력
다음 상황에서 수동 입력 필요:
- **D축 보정**: 자동 분석이 D=0.00 반환 (구조적 한계)
- **반사적/무의식 행동**: 대사에 감정이 명시되지 않은 격한 순간
- **원작 재현**: 특정 감정 강도가 필요한 극적 장면

**수동 PAD 판단 기준표**:
| 상황 | pleasure | arousal | dominance |
|---|---|---|---|
| 차가운 위협 (우위) | -0.3 | 0.3 | +0.6 |
| 애원/굴복 | -0.5 | 0.5 | -0.7 |
| 반사적 분노 발사 | -0.5 | 0.9 | +0.4 |
| 침착한 명령 | 0.0 | -0.2 | +0.7 |
| 충격/경악 | -0.3 | 0.8 | -0.3 |
| 안도 | +0.5 | -0.4 | +0.2 |

### 청자 관점 변환 (진행 예정)
현재 `analyze_utterance`는 **화자(speaker) 톤**을 분석한다.
청자(listener)의 PAD 반응은 관계/해석에 따라 달라지므로 향후 별도 파이프라인 추가 예정.
수동 입력으로 우회할 것.

---

## 5. Beat 전환 관찰

### Beat와 Trigger의 본질

**Beat 전환의 역할**:
- Beat 전환 = **새로운 appraise 실행** = **새 OCC 감정이 태어나는 순간**
- 하나의 Beat는 하나의 심리적 "국면". 국면이 바뀌면 새 감정이 태어남

**Trigger 설계 원칙**:
- Trigger는 "이미 그 감정이 있을 때"가 아니라, **"기존 감정이 어떤 상태로 변하면 새 감정이 피어날 심리적 토양이 되는가"** 를 표현한다
- `apply_stimulus`는 PAD 자극으로 **기존 감정의 강도만 조절**할 뿐, 새 OCC 감정을 생성하지 않는다
- 따라서 **appraise가 만들지 않은 감정은 trigger에 써도 영원히 충족되지 않는다**

**잘못된 trigger 예시** (시맨틱 반전):
```json
// triumphant (승리 beat) — Fear/Distress 높을 때 전환???
"trigger": [
  [{"above": 0.7, "emotion": "Fear"}],
  [{"above": 0.7, "emotion": "Distress"}]
]
```
"공포가 극심할 때 승리감으로 전환"은 논리적 모순. 게다가 Beat 1에서 이미 Fear/Distress가 높으므로 첫 턴부터 자동 전환됨.

**잘못된 trigger 예시** (생성되지 않는 감정 참조):
```json
// cornered beat에서는 Pride가 애초에 생성되지 않음
"trigger": [
  [{"above": 0.5, "emotion": "Pride"}]
]
```
Beat 1의 appraise 입력이 `event.ds<0, action.pw<0` (부정 상황)이라면 Pride는 0에서 출발. stimulus로는 0에서 못 올라감 → trigger 영원히 미충족.

**올바른 trigger 예시** (심리 전환점 모델링):
```json
// cornered → triumphant: 짐이 반격 태세를 갖추는 순간
"trigger": [
  [
    {"below": 0.4, "emotion": "Fear"},
    {"above": 0.5, "emotion": "Anger"}
  ],
  [
    {"below": 0.3, "emotion": "Fear"},
    {"below": 0.4, "emotion": "Distress"}
  ]
]
```
- 경로 1 (AND): 공포 가라앉음 + 분노/결의 치솟음 → 능동적 대응 태세
- 경로 2 (AND): 공포 거의 사라짐 + 고통 완화 → 안전 확보 후 여유
- 이 상태가 되면 Beat 2의 새 appraise가 짐의 praiseworthy action을 평가해 **Pride**를 새로 생성

### Trigger 설계 체크리스트
1. **이 Beat에서 태어나야 할 감정은 무엇인가?** (OCC 이론 기반: Pride, Joy, Relief, Satisfaction 등)
2. **그 감정이 피어나려면 이전 Beat의 어떤 감정이 어떻게 변해야 하는가?** (완화/강화/결합)
3. **그 변화가 stimulus(PAD 자극)만으로 도달 가능한가?** (기존 감정의 강도 조절만 가능)
4. **이전 Beat의 appraise에서 참조 대상 감정이 실제로 생성되는가?** (0에서 출발 방지)

### agent_id 사용 규칙 (Pride/Shame 생성)

시나리오 JSON의 `action.agent_id`는 **자기 행위**와 **타인 행위**를 구분한다:
- `agent_id` 생략 또는 `agent_id: "<npc_id>"` → **자기 행위** (Pride/Shame/Gratification 생성)
- `agent_id: "<다른_id>"` → **타인 행위** (Admiration/Reproach/Gratitude 생성)

DTO 변환 레이어가 `agent_id == npc_id`를 자동으로 `None`으로 정규화하므로, 시나리오 작성 시 가독성을 위해 `agent_id: "jim"` 처럼 명시해도 된다.

**자기 행위 예시** (짐의 재장전 → Pride 생성):
```json
{
  "id": "triumphant",
  "action": {
    "agent_id": "jim",           // == npc_id → 자기 행위로 처리
    "praiseworthiness": 0.6,     // 칭찬받을 만함 → Pride
    "description": "짐이 기지를 발휘해 재장전에 성공"
  }
}
```

### 정상 Beat 전환
```json
{
  "beat_changed": true,
  "active_focus_id": "triumphant"  // 새 focus
}
```

### State Latching (수정 후 동작)
활성 Focus는 재전환 대상에서 제외된다:
```
Turn 1: Distress 0.95 → triumphant 전환 (beat_changed: true)
Turn 2: Distress 0.97 → beat_changed: false (이미 triumphant)
Turn 3: Joy 0.9 & Distress 계속 → beat_changed: false (아직 다른 focus trigger 없음)
Turn 4: Satisfaction 0.8 → relieved 전환 (beat_changed: true)
```

### Trigger 조건 확인
```
get_scene_info()
→ focuses[].trigger_display: "(Fear > 0.7) OR (Distress > 0.7)"
```

---

## 6. 저장 및 경로 규칙

### save_type 종류
| save_type | 저장 대상 | 언제 사용 |
|---|---|---|
| `"scenario"` | 시나리오 JSON (turn_history 제외) | 시나리오 원본 수정 후 |
| `"result"` | 결과 JSON (turn_history 포함) | 테스트 결과만 저장 |
| `"report"` | test_report 필드 → .md 파일 | 레포트만 별도 저장 |
| `"all"` | result JSON + report MD | **세션 종료 시 권장** |

### 표준 저장 경로
```
data/{book}/{chapter}/{scenario}.json                   # 시나리오 원본
data/{book}/{chapter}/{scenario}/test_001.json          # 테스트 결과
data/{book}/{chapter}/{scenario}/test_001.md            # 테스트 레포트
```

`get_save_dir`가 현재 로드된 시나리오의 결과 디렉토리 경로를 자동 계산해준다 (시나리오 파일명에서 `.json` 확장자를 제거한 이름).

---

## 7. 테스트 레포트 템플릿

`update_test_report`로 작성하는 마크다운 템플릿 (한국어 작성, 원칙 1).

채점 기준의 상세 정의는 [`quality-metrics.md`](quality-metrics.md) 참조.

```markdown
# Test Report: {시나리오명}

## 테스트 정보
- 시나리오: {이름}
- NPC: {주체}
- Partner: {상대}
- 테스트 일시: {YYYY-MM-DD}
- LLM: {모델명} / temperature={값} / top_p={값}

## Scene 구성
- Beat 1 ({focus_id}): {설명}
- Beat 2 ({focus_id}): {설명}
- Trigger: {조건}

## 감정 흐름 요약
| 턴 | 지배 감정 | 강도 | Mood | Beat | PAD | CF | SA | DC | 비고 |
|---|---|---|---|---|---|---|---|---|---|
| 1 | {emotion} | {강도} | {mood} | {focus} | {pad} | {점수} | {점수} | {점수} | {이슈 표시} |

> CF=Character Fidelity, SA=Scene Appropriateness, DC=Directive Clarity (각 1-5점)

## 관계 변화 ({npc} → {partner})
| 축 | Before | After | Delta |
|---|---|---|---|

## 정량 평가

### 세션 평균
| 지표 | 점수 | 비고 |
|---|---|---|
| Character Fidelity | {평균}/5 | {한 줄 평} |
| Scene Appropriateness | {평균}/5 | {한 줄 평} |
| Directive Clarity | {평균}/5 | {한 줄 평} |
| **Overall** | **{평균}/5** | |

### 턴별 채점 근거
- **Turn 1**: CF {점수} — {근거}; SA {점수} — {근거}; DC {점수} — {근거}
- **Turn 2**: CF {점수} — {근거}; SA {점수} — {근거}; DC {점수} — {근거}
- ...

## 회귀 비교 (이전 세션 대비)
> 이전 세션이 없거나 첫 회차면 이 섹션 생략.

| 지표 | 이전 ({session_id}) | 현재 | Δ |
|---|---|---|---|
| Character Fidelity | {이전} | {현재} | {+/-delta} |
| Scene Appropriateness | {이전} | {현재} | {+/-delta} |
| Directive Clarity | {이전} | {현재} | {+/-delta} |

주요 변동 원인:
- {지표가 하락했다면 무엇 때문인지}
- {상승했다면 어떤 엔진/시나리오 변경이 기여했는지}

## 긍정적 관찰
1. ...

## 발견된 이슈
### Issue 1: {제목} (우선순위)
- 현상
- 원인
- 영향
- 개선 방안
- **관련 정량 지표**: {어느 지표 점수가 이 이슈를 반영하는지}

## 결론
- 엔진 검증 성공 항목
- 우선 수정 항목
- 원작 충실도
- **정량 평가 총평**: {세션 평균 점수 + 회귀 방향 한 줄 요약}
```

### 레포트 작성 전 체크리스트
- [ ] `get_history`로 전체 턴 데이터 조회했는가
- [ ] 각 턴에 대해 3개 지표 모두 채점했는가
- [ ] 채점 근거가 구체적인가 (facet 이름, 지시사항 인용)
- [ ] 5점 만점을 남발하지 않았는가
- [ ] 이전 세션 레포트를 읽고 회귀 비교했는가 (있을 경우)

---

## 8. 트러블슈팅

### "세션을 찾을 수 없습니다"
- 이미 `dialogue_end`로 닫혔거나 서버 재시작됨
- `dialogue_start`로 새 세션 시작

### `has_scene: false`
- 시나리오 JSON에 `scene` 필드 누락
- `focuses`는 배열, `trigger`는 `[[{above, emotion}]]` 이중 중첩 배열 형식
- 관계 키는 콜론 형식 (`npc_a:npc_b`)

### PAD D축이 항상 0.00
- 알려진 이슈 — D축 앵커의 구조적 한계 (천장 ~76%)
- 수동 PAD 입력으로 우회

### Beat가 매 턴 전환 (state latching 수정 전)
- `src/domain/emotion/scene.rs`의 `check_trigger`가 활성 focus 제외하는지 확인
- 2026-04 수정 완료 — `cargo test scene`으로 검증

### `update_test_report`는 성공하는데 디스크에 파일이 없음
- 메모리에만 저장된 상태 — `save_scenario(save_type="report")` 또는 `"all"`로 파일 저장 필요

### Windows에서 HTTP 테스트
- `curl`의 JSON payload quote stripping 불안정
- Python `urllib.request` + `.encode("utf-8")` 권장
- PowerShell: `[System.Text.Encoding]::UTF8.GetBytes($json)` 사용

### MCP 도구가 보이지 않음
- 서버 실행 확인: `http://127.0.0.1:3000/api/npcs`
- Claude Desktop: 완전 종료 후 재시작 필요

---

## 9. 주요 MCP 도구 ↔ 내부 서비스 매핑

| MCP 도구 | 내부 처리 주체 | 역할 |
|---|---|---|
| `appraise` | `SituationService` | 상황 DTO → 도메인 변환 후 감정 평가 |
| `apply_stimulus` | `SceneService` | PAD 자극 적용 및 Beat 전환 트리거 체크 |
| `analyze_utterance` | `PadAnalyzer` | 대사 → PAD 자동 분석 |
| `after_dialogue` | `RelationshipService` | 관계 수치 최종 갱신 |
| `dialogue_start/turn/end` | `DialogueTestService` | LLM 연기 포함 대화 세션 관리 |
| `get_next_utterance` | `StateInner` | test_script 커서 조회/전진 |
| `load_scenario` | `StateInner` | 시나리오 JSON 로드 |
| `save_scenario` | `StateInner` | 상태 저장 (scenario/result/report/all) |
| `get_history` | `TurnRecord` | trace + input_pad 포함 히스토리 |
| `get_scene_info` | `SceneService` | Scene 활성 상태 조회 |
| `get_save_dir` | `StateInner` | 결과 저장 경로 자동 계산 |
| `update_test_report` | `State` | 마크다운 레포트 메모리 저장 |
| `create_full_scenario` | `State` | NPC/관계/Scene 일괄 생성 |
