---
id: npc-07
kind: active
name: 천순제(天順帝)
aliases:
  - 대진 황제
  - 옥좌의 사람
  - 꼭두각시 황제
status: alive
hexaco:
  honesty_humility: 0.3         # 야망 추정 낮음 + 권력 행사 안 함 → modesty 양, sincerity 보통
  emotionality: 0.6             # 무력감 + 공포(꼭두각시) + 의존성(조고)
  extraversion: -0.5            # "옥좌에 앉되 말은 하지 못한다"
  agreeableness: 0.4            # 순응적, 저항 안 함 — 도구적 인내
  conscientiousness: 0.0        # 미상 — 권한 행사 못함
  openness: 0.0                 # 미상 — Phase 4 정밀 패스에서 보강 필요
temporal:
  birth_year: 235년차 즈음
  death_year: ~
  age_at_game_start: 35
  notes: |
    character-roster v1.1 §3 "30대?" — 본 Phase에선 35세로 가정. 즉위 시점
    추정 25세(약 10년 전 붉은 밤의 변 직후 대진 권력 공백기). 열전 미작성 —
    Phase 4 정밀 캐릭터 패스에서 보강 예정.
affiliation:
  - group-daejin-court          # 대진 황실 명목 원수
birthplace: place-daejin
current_location: place-daejin
summary: |
  대진의 명목 황제. 30대 중반 추정. 조고의 꼭두각시로 옥좌에 앉되 명령은 내리지
  못한다. 칙령은 형식적이며 실질 권력은 모두 십상시를 거친다. 본인의 진짜 의도·
  성격은 게임 도중 점진적으로 드러나는 잠재적 분기 인물.
tags:
  - wuxia
  - person
  - puppet-emperor
  - declining-empire
  - daejin
  - sketch-pending-phase4
extras:
  signature_skill: 미상 (열전 미작성)
  biography_short: 대진 명목 황제. 조고의 꼭두각시. 잠자는 변수.
  game_role: 꼭두각시 황제 — "꼭두각시가 깨어날 것인가" 메인 분기 트리거
  priority: "★★★"
  combat_style: 미상 — 황제 호위 외 본인 전투 능력 미정.
  story_role: 대진 황실 정당성의 명목 축. 조고 처단 후 권력 공백 변수.
  big_five_legacy: {}            # 열전 미작성 — Phase 4에서 채움
  values: {}
  hexaco_facets: {}
  source_status: heritage-pending  # 열전 미작성, character-roster + group-daejin-court 노트만 출처
---

## 개요
대진 명목 황제(天順帝). 30대 중반 추정. 조고와 십상시에 의해 옥좌에 앉혀진
꼭두각시로, 옥좌에 앉되 명령은 내리지 못한다. 본 Person은 character-roster v1.1
§3 우선순위 ★★★의 "열전 미작성" 인물 — 본 Phase 2 등록은 Phase 1 group-daejin-
court 멤버 외래키 활성화를 위한 최소 형태이며, HEXACO·동기·관계 정밀 매핑은
Phase 4 정밀 캐릭터 패스에서 수행된다. (`extras.source_status: heritage-pending`
플래그로 표시.)

## 배경
공식 기록상 약 10년 전(약 25세) 즉위 — 붉은 밤의 변과 태무제 사망/실종 직후
대진 권력 공백기에 조고가 옹립. 즉위 후 모든 칙령은 형식적이며 실질 통수권은
십상시를 거친다. 본인은 별궁에서 형식적 일과를 수행하며 외부와의 직접 접촉은
조고가 통제. 외모는 나이에 비해 침착·우울하다는 정파 측 첩보가 있으나 직접
관찰된 사례는 거의 없음.

## 동기
표층은 **순응** — 조고의 꼭두각시로 살아남는 것이 일상. 심층은 **미상** —
본인의 진짜 의도·욕망·계획이 있는지조차 확인되지 않음. character-roster의
"꼭두각시 황제" 한 줄 외에는 자료 없음. Phase 4 정밀 패스에서 (1) 진짜로 무력
한가 (2) 깨어날 가능성이 있는가 (3) 조고와의 비밀 거래가 있는가 결정 필요.

## 비밀
모름 — 정밀 매핑 보류. Phase 4에서 다음 후보 결정:
1. **각성 시나리오** — 조고의 통제에서 벗어날 잠재력의 유무.
2. **태무제와의 관계** — 혈연·계승·분파 어느 쪽인가.
3. **본인의 의향** — 꼭두각시를 자청한 것인가 강제된 것인가.

## HEXACO 분석
열전 미작성 인물 — Big Five 원전 값 부재. 다음은 character-roster + group-daejin-
court 메모("옥좌에 앉되 말은 하지 못한다", "꼭두각시 황제") + group 산문(천순제
vs 조고 갈등 축)에서 추론한 잠정 6 dim. **Phase 4 정밀 패스에서 재검토 필수**.

- **H +0.3 (정직-겸손, 약간 양수, 잠정)**: 권력 행사 안 함 → modesty 양수, greed_
  avoidance 양수(자기 욕망 표출 안 함), sincerity·fairness는 미상. 보통 + 약간
  양수로 가정.
- **E +0.6 (정서성, 높음, 잠정)**: 무력감 + 통제당하는 두려움 + 조고에 대한
  의존성 → fearfulness·anxiety·dependence 모두 양수. 단 sentimentality 미상.
- **X -0.5 (외향성, 낮음, 비교적 확실)**: "옥좌에 앉되 말은 하지 못한다" 직접
  인용 → sociability·liveliness 매우 낮음. social_self_esteem 매우 낮음.
- **A +0.4 (원만성, 약간 양수, 잠정)**: 순응적·저항 안 함 → patience·flexibility
  양수(도구적). forgiveness·gentleness 미상이라 보통.
- **C 0.0 (성실성, 중립, 미상)**: 능력 행사 못 하므로 organization·diligence·
  perfectionism·prudence 모두 관측 불가. 중립으로 가정.
- **O 0.0 (개방성, 중립, 미상)**: 자료 부재. 중립.

매핑 신뢰도 낮음 — 본 인물은 Phase 4 정밀 패스에서 별도 .md 작성 시 6 dim 재검토
필수. 회귀 가드: `extras.source_status = heritage-pending` 키가 있는 동안 본
HEXACO는 잠정으로 취급.

## 관계
- **group-daejin-court 안에서**: 명목 황제. 실권 없음.
- **npc-02 조고**: 통제자. 경멸 + 두려움 + 의존(자기 생존이 조고에 달림).
- **십상시(group-shipsangsi)**: 자기를 둘러싼 감시·호위·통제 도구.
- **태무제 단운(사망, H26)**: 선대 황제. 혈연·계승 관계는 미상.
- **소림·무당**: 명목상 황제 지지자. 직접 접촉은 조고가 차단.
- **플레이어**: 미상. 직접 만남이 후반 분기 가능성.

## 게임에서의 역할
잠재적 분기 변수. 메인 퀘스트 "꼭두각시가 깨어날 것인가" 분기의 트리거. 조고
처단 후 (1) 천순제 친정(親政) (2) 권력 공백 (3) 다른 인물의 옹립 — 세 분기
중 어느 쪽이 열리는지가 본 인물의 진짜 정체에 따라 결정. **Phase 4 정밀 캐릭터
패스에서 본 인물 단독 .md 작성 우선순위 ★★★ — character-roster의 작성 로드맵
참고.**
