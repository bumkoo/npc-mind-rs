---
name: npc-mind-testing
description: "NPC Mind Engine MCP를 사용한 시나리오 테스트 및 대화 세션 수행 가이드. 시나리오를 로드하여 LLM 대화 테스트를 실행하고, 감정 흐름을 관찰하며, 정량 평가(Character Fidelity, Scene Appropriateness, Directive Clarity)를 수행한다. npc-mind-studio MCP가 연결된 상태에서 사용자가 시나리오 테스트, 대화 테스트, 감정 시뮬레이션, NPC 연기 테스트, Beat 전환 테스트, 디렉티브 품질 평가, 테스트 레포트 작성 등을 요청하면 반드시 이 스킬을 사용할 것. 엔진 수정 후 회귀 테스트에도 해당."
---

# NPC Mind Testing — MCP 테스트 세션 가이드

시나리오를 로드하여 LLM 대화 테스트를 실행하고, 감정 흐름을 관찰하며, 정량 평가까지 수행하는 워크플로우.

## 3가지 핵심 원칙

### 원칙 1: 한국어 기본
- 모든 description, 대사, 테스트 레포트는 **한국어**로 작성
- PAD 앵커가 한국어 기반이므로 한국어 대사의 자동 분석 정확도가 가장 높다
- 원작이 영어여도 엔진에 입력하는 description·대사는 한국어로 번역

### 원칙 2: NPC Asset 활용
- 테스트 전 `data/{book}/assets/npcs/` 디렉토리에서 기존 Asset 확인
- 있으면 로드, 없으면 새로 생성하여 저장
- 테스트 중 성격 조정이 필요하면 **asset 파일을 수정** (시나리오가 아니라)

### 원칙 3: 순수 대사만 입력
- `dialogue_turn(utterance=...)`에는 **순수 발화만** 포함
- ❌ 금지: `*핸즈가 으르렁거리며* "이 녀석!" *고함친다*`
- ✅ 권장: `"이 녀석! 거기 서라!"`
- 상황 지문은 `situation_description` 파라미터나 Scene Beat 전환으로 표현
- 이유: PAD 자동 분석이 지문에 흔들리지 않고, 순수 대사 톤만 분석해야 정확하다

## 테스트 세션 전체 흐름

```
0. NPC Asset 확인         data/{book}/assets/npcs/ 조회 (원칙 2)
1. load_scenario           시나리오 파일 로드
2. get_scene_info          Scene 활성화 확인 (has_scene: true)
3. dialogue_start          세션 시작 + 초기 appraise
4. dialogue_turn × N       상대 대사 입력 → NPC 연기 응답 + 감정 갱신
5. dialogue_end            세션 종료
6. after_dialogue          관계 갱신 (closeness/trust 변동)
7. save_scenario("result") 결과 JSON 저장 (턴 히스토리 포함)
8. 정량 평가 수행           get_history → 3개 지표 채점
9. update_test_report      레포트 작성 (한국어, 정량 섹션 포함)
10. save_scenario("all")   result JSON + report MD 동시 저장
```

**정량 평가는 원칙이다** — 특별한 요청이 없어도 세션마다 수행한다.

### LLM 연기 없이 엔진만 검증할 때 (chat feature 없음)

```
1. load_scenario
2. appraise               초기 상황 → 감정 생성
3. analyze_utterance       대사 → PAD 추출
4. apply_stimulus          PAD → 감정 갱신 + Beat 전환
5. (3-4 반복)
6. after_dialogue
7. save_scenario
```

## 각 단계 상세

### 시나리오 로드
```
load_scenario(path="treasure_island/ch26_mast_duel/짐vs이즈라엘.json")
```
경로는 `data/` 하위 상대 경로. 로드 후 `get_scenario_meta`로 확인.

### Scene 상태 확인
```
get_scene_info()
→ { has_scene: true, active_focus_id: "cornered", trigger_display: "..." }
```
`has_scene: false`이면 시나리오 JSON에 `scene` 필드가 누락된 것.

### 대화 세션 시작
```
dialogue_start(session_id="unique_id", npc_id="jim", partner_id="israel_hands")
```
서버가 자동으로 initial focus에 대한 appraise를 수행한다.

### 대화 턴 실행
```
dialogue_turn(
  session_id="unique_id",
  npc_id="jim",
  partner_id="israel_hands",
  utterance="순수 대사만 입력"
)
```
- `pad` 생략 시 → 대사를 자동 PAD 분석 (embed feature)
- 반환: LLM 연기 응답 + 감정 갱신 + `beat_changed` + `active_focus_id`

### PAD 입력

**자동 분석 (기본값)**: pad 파라미터를 생략하면 대사에서 자동 추출. 자연스러운 흐름에 적합.

**수동 입력이 필요한 경우**:
- D축이 0.00으로 반환될 때 (구조적 한계)
- 반사적/무의식 행동 (대사에 감정이 명시되지 않은 격한 순간)
- 원작 재현을 위해 특정 감정 강도가 필요할 때

**수동 PAD 판단 기준표**:
| 상황 | P | A | D |
|---|---|---|---|
| 차가운 위협 (우위) | -0.3 | 0.3 | +0.6 |
| 애원/굴복 | -0.5 | 0.5 | -0.7 |
| 반사적 분노 | -0.5 | 0.9 | +0.4 |
| 침착한 명령 | 0.0 | -0.2 | +0.7 |
| 충격/경악 | -0.3 | 0.8 | -0.3 |
| 안도 | +0.5 | -0.4 | +0.2 |

### Beat 전환 관찰

`dialogue_turn` 결과에서 `beat_changed: true`이면 새 Focus로 전환된 것.

**정상 동작**: 활성 Focus는 재전환 대상에서 제외된다 (state latching). 같은 Focus로 매 턴 전환되지 않는다.

전환이 의도와 다르면:
- `get_scene_info()`로 trigger 조건 확인
- `get_history()`로 감정 강도 변화 추적
- Trigger 설계가 문제면 시나리오 수정 (→ `npc-scenario-creator` 스킬 참조)

### 관계 갱신
```
after_dialogue(req={npc_id: "jim", partner_id: "israel_hands", significance: 1.0})
```
`significance`: 0.0~1.0. 중요한 장면일수록 1.0. before/after 관계 수치를 반환한다.

### 결과 저장
```
get_save_dir()  → 결과 저장 디렉토리 자동 계산
save_scenario(path="...", save_type="all")  → .json + .md 동시 저장
```

## 정량 평가 — 3개 필수 지표

매 세션 종료 후 `get_history()`로 전체 턴 데이터를 조회하고, 각 턴에 대해 3개 지표를 5점 척도로 채점한다. 상세 채점 기준은 `references/quality-metrics.md` 참조.

### 1. Character Fidelity (캐릭터 충실도)
디렉티브(tone/attitude/behavior/restriction)가 NPC의 HEXACO 프로필에서 논리적으로 도출되는가.

### 2. Scene Appropriateness (장면 적절성)
Beat/Focus 전환(또는 유지) 결정이 서사적으로 타당한가.

### 3. Directive Clarity (디렉티브 명확성)
디렉티브가 LLM이 이해하고 준수할 만큼 구체적·명확한가. 실제 LLM 응답이 디렉티브를 따랐는가.

### 채점 원칙
- **5점은 완벽 사례에만**. 우수한 일반 사례는 4점
- 근거를 항상 명시 ("좋음" 같은 모호한 표현 금지, 구체적 facet/지시사항 인용)
- 수행 주체 = 채점 주체이므로 자기 관대함 편향에 주의, 의식적으로 엄격하게

## 테스트 레포트 작성

`update_test_report`로 마크다운 레포트를 작성한다. 필수 섹션:

1. **테스트 정보** — 시나리오명, NPC, Partner, 일시, LLM 모델
2. **Scene 구성** — Beat별 설명, Trigger 조건
3. **감정 흐름 요약** — 턴별 지배 감정, 강도, Mood, Beat, PAD, CF/SA/DC 점수
4. **관계 변화** — Before/After/Delta 표
5. **정량 평가** — 세션 평균 + 턴별 채점 근거
6. **회귀 비교** — 이전 세션이 있으면 점수 비교 + 변동 원인 분석
7. **긍정적 관찰** / **발견된 이슈** / **결론**

### 레포트 작성 전 체크리스트
- [ ] `get_history`로 전체 턴 데이터 조회했는가
- [ ] 각 턴에 대해 3개 지표 모두 채점했는가
- [ ] 채점 근거가 구체적인가
- [ ] 5점 만점을 남발하지 않았는가
- [ ] 이전 세션 레포트를 읽고 회귀 비교했는가 (있을 경우)

## 트러블슈팅

| 증상 | 원인 | 해결 |
|---|---|---|
| "세션을 찾을 수 없습니다" | 이미 닫혔거나 서버 재시작 | `dialogue_start`로 새 세션 |
| `has_scene: false` | 시나리오에 `scene` 필드 누락 | JSON 확인 |
| PAD D축 항상 0.00 | D축 앵커 구조적 한계 | 수동 PAD 입력 |
| `update_test_report` 후 파일 없음 | 메모리에만 저장된 상태 | `save_scenario("all")` 필요 |

## 참고 문서

- **채점 기준 상세**: `references/quality-metrics.md` — 5점 척도 정의, 체크리스트, 예시
- **도구 스펙 상세**: `references/tools-quick-ref.md`
