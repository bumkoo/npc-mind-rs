---
name: npc-scenario-creator
description: "NPC Mind Engine MCP를 사용한 시나리오 생성 가이드. 원작 텍스트에서 NPC HEXACO 프로필을 설계하고, 관계/오브젝트/Scene Focus를 구성하여 완전한 시나리오 JSON을 생성한다. npc-mind-studio MCP가 연결된 상태에서 사용자가 시나리오 만들기, NPC 생성, 캐릭터 설계, Scene 구성, Focus/Trigger 설계, 원작 기반 시나리오, create_full_scenario 등을 요청하면 반드시 이 스킬을 사용할 것. 새 작품(book)을 추가하거나 기존 캐릭터의 새 장면을 만들 때도 해당."
---

# NPC Scenario Creator — MCP 시나리오 생성 가이드

원작 텍스트(소설, 무협지 등)를 읽고 NPC Mind Engine에 넣을 시나리오 JSON을 설계·생성하는 워크플로우.

## 언어 원칙

엔진에 입력하는 모든 description은 **한국어**로 작성한다. 원작이 영어여도 번역해서 입력한다. 이유는 PAD 앵커가 한국어 기반이라 감정 분석 정확도가 한국어에서 가장 높기 때문이다.

## 전체 흐름

```
1. 원작 텍스트 읽기        list_source_texts → read_source_text (챕터 단위)
2. NPC Asset 확인/생성     data/{book}/assets/npcs/ 조회 → 없으면 새로 설계
3. 관계 Asset 확인/생성     data/{book}/assets/relationships/ 조회
4. 오브젝트 Asset 확인/생성  data/{book}/assets/objects/ 조회 → Focus에서 쓸 것만
5. Scene Focus 설계        Beat별 감정 아크, Trigger 조건, prospect/object 연결
6. 시나리오 생성           create_full_scenario (NPC + 관계 + 오브젝트 + Scene 일괄)
7. 검증                   load_scenario → start_scene → 초기 감정 확인
```

## 1단계: 원작 텍스트 읽기

```
list_source_texts()          → data/ 하위 .txt 파일 목록
read_source_text(path, chapter)  → 챕터 단위로 읽기
```

원작에서 추출할 것:
- **등장인물**: 이름, 나이, 직업, 성격 묘사, 말투
- **인물 관계**: 친밀도(closeness), 신뢰(trust), 권력관계(power)
- **핵심 장면**: 감정 변곡점이 있는 장면 (Beat 후보)
- **오브젝트**: 장면에 중요한 사물/장소

## 2단계: NPC Asset 관리

NPC 프로필은 **재사용 가능한 Asset**으로 관리한다. 같은 인물이 여러 시나리오에 등장해도 HEXACO 프로필은 하나만 유지한다.

### 저장 구조
```
data/{book}/assets/
├── npcs/              # NPC HEXACO 프로필
├── relationships/     # 관계 초깃값
└── objects/           # 오브젝트 (물건, 장소 등)
```

### Asset 사용 흐름
1. `data/{book}/assets/npcs/` 디렉토리 확인
2. 필요한 NPC가 있으면 → 해당 JSON을 시나리오에 포함
3. 없으면 → HEXACO 24 facets 설계 후 asset 파일로 저장

### HEXACO 24 Facets 설계 가이드

각 facet은 -1.0 ~ 1.0 범위. 원작 인물의 행동/대사/서술에서 근거를 찾아 설정한다.

**6개 대차원 (각 4 facets)**:
- **Honesty-Humility**: sincerity, fairness, greed_avoidance, modesty
- **Emotionality**: fearfulness, anxiety, dependence, sentimentality
- **eXtraversion**: social_self_esteem, social_boldness, sociability, liveliness
- **Agreeableness**: forgiveness, gentleness, flexibility, patience
- **Conscientiousness**: organization, diligence, perfectionism, prudence
- **Openness**: aesthetic_appreciation, inquisitiveness, creativity, unconventionality

**설계 원칙**:
- 극단값(-1.0, 1.0)은 아주 특이한 인물에게만 사용. 대부분 -0.7~0.7 범위
- `description`에 인물의 핵심 특성을 한국어로 2~3문장 요약
- 원작의 **행동 근거**를 기록해두면 나중에 조정할 때 유용하다

### Asset JSON 형식

```json
{
  "id": "character_id",
  "name": "Character Name",
  "description": "한국어 인물 설명 2~3문장",
  "sincerity": 0.5,
  "fairness": 0.6,
  ... (24 facets)
}
```

### Asset 관리 원칙
- 테스트 중 성격 조정이 필요하면 **asset 파일을 수정** (시나리오 파일이 아니라)
- 같은 인물을 약간 다르게 표현하려면 새 asset으로 분기 (예: `billy_bones_drunk.json`)
- 작품 간에는 인물을 공유하지 않는다

## 3단계: 관계 설계

```json
{
  "owner_id": "jim",
  "target_id": "israel_hands",
  "closeness": -0.6,    // -1.0(적대) ~ 1.0(친밀)
  "trust": -0.9,         // -1.0(불신) ~ 1.0(신뢰)
  "power": -0.3          // -1.0(열등) ~ 1.0(우월)
}
```

관계는 **비대칭**이다. A→B와 B→A를 각각 설정해야 한다. 관계 키는 `owner_id:target_id` 형식.

## 4단계: Scene Focus 설계 (가장 중요)

Scene은 여러 개의 **Focus(Beat)**로 구성된다. 각 Beat는 하나의 심리적 "국면"이고, Beat 전환 시 새로운 OCC 감정이 생성된다.

### Focus 구조

```json
{
  "id": "focus_id",
  "description": "한국어 상황 설명",
  "event": {
    "description": "이 상황이 NPC에게 어떤 사건인지",
    "desirability_for_self": 0.7,   // -1.0(최악) ~ 1.0(최선)
    "prospect": "anticipation"       // 선택. 아래 전망 참조
  },
  "action": {
    "agent_id": "행위자_id",
    "description": "누구의 어떤 행동인지",
    "praiseworthiness": 0.6          // -1.0(비난) ~ 1.0(칭찬)
  },
  "object": {                        // 선택. 장면에서 중요한 사물/장소
    "target_id": "object_id",        // ⚠️ 필드명 주의: target_id (object_id 아님)
    "appealingness": 0.9             // ⚠️ 필드명 주의: appealingness (appeal 아님)
  },
  "trigger": null   // 첫 Beat는 null (Initial)
}
```

### Event 전망(Prospect) — Hope/Fear 생성

event에 `prospect` 필드를 설정하면 Hope 또는 Fear 감정이 생성된다:

| prospect 값 | 의미 | 생성 감정 |
|---|---|---|
| `null` (기본) | 현재/과거 사건 | Joy 또는 Distress |
| `"anticipation"` | 미래 전망 | ds>0 → **Hope**, ds<0 → **Fear** |
| `"hope_fulfilled"` | 바랐던 일 실현 | **Satisfaction** |
| `"hope_unfulfilled"` | 바랐던 일 불발 | **Disappointment** |
| `"fear_unrealized"` | 두려웠던 일 불발 | **Relief** |
| `"fear_confirmed"` | 두려웠던 일 실현 | **FearsConfirmed** |

**설계 팁**: "계산 중", "기다리는 중", "결과를 모르는" Beat에는 `"anticipation"` 사용. 이렇게 하면 후속 Beat trigger에서 `Hope > 0.x` 조건을 자연스럽게 걸 수 있다.

### Object 필드 — OCC 호감/혐오 평가

object는 **반드시 엔진에 등록된 오브젝트 ID**를 참조해야 한다. object를 asset으로만 만들고 focus에 연결하지 않으면 감정이 생성되지 않는다.

- `target_id`: 오브젝트 ID (create_object 또는 시나리오 JSON의 objects에 등록된 ID)
- `appealingness`: -1.0(혐오) ~ 1.0(매력). 양수 → **Love**, 음수 → **Hate**
```

### agent_id와 감정 생성

`action.agent_id`가 어떤 감정을 생성하는지 결정한다:
- `agent_id == npc_id` (자기 행위) → **Pride/Shame/Gratification/Remorse**
- `agent_id != npc_id` (타인 행위) → **Admiration/Reproach/Gratitude/Anger**

### Trigger 설계 — 핵심 규칙

Trigger는 "이전 Beat의 감정이 어떤 상태가 되면 새 Beat로 전환할지"를 정의한다.

**반드시 지켜야 할 4가지 체크리스트**:

1. **이 Beat에서 태어나야 할 감정은 무엇인가?** (OCC 이론: Pride, Joy, Relief 등)
2. **그 감정이 피어나려면 이전 Beat의 어떤 감정이 어떻게 변해야 하는가?**
3. **그 변화가 stimulus(PAD 자극)만으로 도달 가능한가?** — stimulus는 기존 감정의 강도만 조절할 수 있고, 새 감정을 생성하지 못한다
4. **이전 Beat의 appraise에서 참조 대상 감정이 실제로 생성되는가?** — 0에서 출발하는 감정은 stimulus로 올릴 수 없다

**흔한 실수들**:

❌ **시맨틱 반전**: "Fear가 높을 때 → triumphant(승리) Beat 전환" → 논리 모순
❌ **존재하지 않는 감정 참조**: Beat 1에서 Pride가 생성되지 않는데 `above: 0.5, emotion: Pride` → 영원히 미충족
❌ **첫 턴 즉시 전환**: 이전 Beat 초기 감정이 이미 trigger 조건을 만족 → 의도치 않은 즉시 전환

**올바른 예시**:
```json
// cornered → triumphant: 짐의 공포가 가라앉고 분노가 차오를 때
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
- 조건 구조: `OR [ AND[...], AND[...] ]` — 외부 배열 OR, 내부 배열 AND
- 경로 1: 공포 완화 + 분노 상승 → 능동적 대응
- 경로 2: 공포·고통 모두 완화 → 안전 확보 후 여유

## 4-1단계: 테스트 스크립트 설계 (test_script)

테스트 재현성을 위해 각 Beat에서 대화 상대가 보낼 대사를 미리 정의한다. `test_script`는 선택 필드이며, 정의하면 Mind Studio와 MCP에서 순서대로 대사를 사용할 수 있다.

### Focus 내 test_script 구조

```json
{
  "id": "cornered",
  "description": "갑판에서 핸즈가 짐을 쫓는다...",
  "trigger": null,
  "event": { ... },
  "action": { ... },
  "test_script": [
    "핸즈, 뒤에서 무슨 소리가 났어. 무슨 짓이야?",
    "이 배는 내가 되찾은 거야. 네 말은 듣지 않겠어.",
    "총이 젖었다고? 그래도 난 포기하지 않아."
  ]
}
```

### 작성 원칙

1. **Beat당 3~5턴**이 적당하다. 너무 적으면 감정 변화가 부족하고, 너무 많으면 Beat 전환이 지연된다.
2. **감정 유도 방향을 고려한다**: 이 Beat에서 NPC의 감정이 어떻게 변해야 하는지(trigger 조건)에 맞춰, 대사의 PAD 자극이 그 방향으로 유도되도록 한다.
3. **원작 대사를 활용한다**: 원작에서 해당 장면의 실제 대사를 참고하되, 엔진 테스트에 맞게 조정한다.
4. **다음 Beat 전환 유도**: 마지막 1~2턴은 trigger 조건을 충족시킬 수 있는 대사를 배치한다. 예: Fear를 낮추려면 안심시키는 대사, Anger를 올리려면 도발적 대사.
5. **한국어로 작성**: PAD 앵커가 한국어 기반이므로 한국어 대사가 감정 분석 정확도가 높다.
6. **순수 대사만**: 지문(예: `*으르렁거리며*`)을 포함하지 않는다. PAD 분석 정확도를 위해 순수 발화만 사용.

### 사용 흐름

- **MCP**: `get_next_utterance()` → 다음 대사 조회 → `dialogue_turn(utterance=...)` 전송
- **Mind Studio UI**: Focus 패널에서 대사 목록 확인, 대화 입력 영역에서 '스크립트 전송' 버튼 클릭
- **즉흥 대사**: test_script 외에 추가 대사를 보내려면 직접 `dialogue_turn(utterance="즉흥 대사")`로 전송 가능

### Beat 전환과 커서

- 각 Beat마다 독립적인 `test_script`와 커서가 있다
- Beat 전환 시 커서가 자동으로 0으로 리셋되어 새 Beat의 대사 목록을 처음부터 사용
- `dialogue_start` 시에도 커서가 0으로 초기화

## 5단계: 시나리오 생성

`create_full_scenario`로 NPC + 관계 + 오브젝트 + Scene을 한 번에 생성한다.

### 필수 입력 필드
- `scenario.name` / `scenario.description` / `scenario.notes[]` — 한국어
- `npcs` — HEXACO 프로필 (asset에서 가져오기)
- `relationships` — 키: `owner_id:target_id`
- `objects` — focus에서 참조할 오브젝트 (asset에서 가져오기)
- `scene.npc_id` / `scene.partner_id` — 주체/상대
- `scene.description` — ⚠️ **필수**. 장면 전체를 요약하는 한국어 문장. 누락 시 엔진이 scene 로드를 거부한다
- `scene.focuses[]` — Beat 배열 (initial_focus_id 포함)
- `scene.significance` — 0.0~1.0, 중요한 장면일수록 1.0에 가깝게

### ⚠️ 자주 틀리는 DTO 필드명

| 항목 | ✅ 올바른 필드명 | ❌ 흔한 실수 |
|---|---|---|
| Scene 설명 | `scene.description` (필수) | 누락 — Focus별 description만 쓰고 scene 레벨을 빼먹음 |
| Object ID | `target_id` | `object_id` |
| Object 매력도 | `appealingness` | `appeal`, `attractiveness` |
| Event 전망 | `prospect` | `expectation`, `hope` |
| Action 도덕성 | `praiseworthiness` | `pw`, `morality` |

## 6단계: 검증

생성 후 반드시 다음을 확인:

```
load_scenario(path)       → 로드 성공 확인
get_scene_info()          → has_scene: true, active_focus_id 확인
appraise(...)             → 초기 감정이 의도와 맞는지 확인
```

초기 appraise 결과에서:
- `desirability_for_self < 0`이면 Distress/Fear가 생성되는지
- `praiseworthiness > 0 + agent_id == npc_id`이면 Pride가 생성되는지
- Trigger에서 참조하는 감정이 실제로 존재하는지

문제가 있으면 Focus의 event/action 값을 조정하고 다시 appraise한다.

## 참고 문서

- **도구 스펙 상세**: `references/tools-quick-ref.md` 참조
- **Beat/Trigger 고급 패턴**: `references/trigger-patterns.md` 참조
