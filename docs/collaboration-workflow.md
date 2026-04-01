# NPC Mind Engine — 협업 워크플로우 가이드

## 개요

이 문서는 Bekay와 Claude가 NPC 심리 엔진을 **반복적으로 개선**하기 위한 협업 루프를 정의한다.
핵심 도구는 Mind Studio (http://127.0.0.1:3000)이며, Claude는 MCP 도구 또는 API로, Bekay는 브라우저로 동시에 사용한다.

---

## 용어 정의

> 용어(Scene, Beat, Utterance)의 정의와 엔진 호출 매핑은 [CLAUDE.md 용어 정의](../../CLAUDE.md#용어-정의) 참조.

**구조 관계:**
```
도서 (허클베리 핀)
 └── 챕터 (Ch.15)
      └── Scene (안개/Trash)    ← 하나의 대화 단위
           ├── Beat 1          ← 감정 전환 비트, appraise 1회
           │    ├── Utterance   ← 대사, stimulus 입력
           │    └── Utterance
           ├── Beat 2
           │    └── Utterance
           └── Scene 종료      ← after_dialogue
```

**Beat 전환 주체:**
- 게임 외부 이벤트 (적 습격, 보물 발견 등) → 게임이 직접 새 Scene 또는 appraise 호출
- 대화 중 감정/의도 변화 → 엔진이 Focus 조건 기반으로 자동 판단

---

## 개선 루프 (Improvement Loop)

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  ① 장면 선택          "허클베리핀 Ch.8 잭슨 섬 첫 만남"      │
│       │                                                     │
│       ▼                                                     │
│  ② 인물 프로파일 생성   HEXACO 24 facet 설계 + 관계 설정     │
│       │                                                     │
│       ▼                                                     │
│  ③ 감정 평가 실행       상황 설정 → 감정 결과 + 프롬프트      │
│       │                                                     │
│       ▼                                                     │
│  ④ 결과 검증            감정 타당성, 프롬프트 품질 평가       │
│       │                                                     │
│       ▼                                                     │
│  ⑤ 개선점 식별          무엇을 고쳐야 하는가?                │
│       │                                                     │
│       ├── 프로파일 문제  → ② 로 복귀                         │
│       ├── 가중치 문제    → 엔진 코드 수정 → ③ 재실행         │
│       ├── PAD 앵커 문제  → 앵커 문장 개선 → ③ 재실행         │
│       ├── 가이드 문제    → directive 로직 수정 → ③ 재실행    │
│       ├── 도구 문제      → Mind Studio 기능 개선 → ③ 재실행  │
│       └── 만족          → ⑥ 저장                            │
│                                                             │
│       ▼                                                     │
│  ⑥ 저장 + 다음 장면     session 저장 → 다음 장면으로 이동    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 각 단계 상세

### ① 장면 선택

**누가**: Bekay
**입력**: 원작 텍스트 (PDF 첨부 또는 참고 자료)
**산출물**: 장면 정의 (인물, 상황, 대화 턴 수)

**기준**: 엔진의 어떤 기능을 테스트하는가?

| 테스트 목적 | 장면 특성 | 예시 |
|------------|----------|------|
| 감정 유형 다양성 | 한 장면에서 여러 감정 전환 | Ch.8 잭슨 섬 (공포→안도→고백→약속) |
| 관계 파열/회복 | 신뢰가 급변하는 장면 | Ch.15 안개 거짓말 후 사과 |
| 내면 갈등 | 상반된 감정 동시 발생 | Ch.31 도덕적 위기 |
| 성격 대비 | 같은 상황에 다른 성격 | 무백 vs 교룡 배신 장면 |
| Compound 감정 | Action+Event 결합 | 배신 (Reproach+Distress→Anger) |

**폴더 생성**: `data/{도서명}/{장면명}/`

---

### ② 인물 프로파일 생성

**누가**: Claude (초안) → Bekay (검토/조정)
**입력**: 원작 텍스트 + 캐릭터 분석
**산출물**: NPC JSON (`POST /api/npcs`) + 관계 JSON (`POST /api/relationships`)

**프로파일 설계 프로세스**:

1. **원작 분석**: Claude가 첨부 자료에서 인물의 행동 패턴, 대사 스타일, 가치관 추출
2. **HEXACO 매핑**: 6차원 24 facet을 -1.0~1.0 범위로 설정
   - 핵심 질문: "이 인물이 ___한 상황에서 어떻게 반응할까?"
   - 각 facet마다 원작의 근거를 기록
3. **API 등록**: Claude가 `POST /api/npcs`로 서버에 등록
4. **Bekay 검토**: 브라우저에서 슬라이더 조정 → "이 facet이 너무 높은/낮은 것 같아"
5. **관계 설정**: 인물 간 closeness/trust/power 초기값 설정

**체크리스트**:
- [ ] 각 facet에 원작 근거가 있는가?
- [ ] 성격 대비가 의미 있는가? (너무 비슷하면 감정 차이 안 남)
- [ ] 관계 초기값이 장면 시작 시점과 맞는가?

---

### ③ 감정 평가 실행

**누가**: Claude (API 호출) + Bekay (브라우저 조작)
**입력**: 상황 설정 (description, Event/Action/Object)
**산출물**: 감정 상태 + 프롬프트 + trace

**상황 설계 프로세스**:

**방법 A: 수동 Beat 실행**
1. **Beat 분할**: 장면을 3-5개 Beat(감정 전환 시점)으로 분할
2. **각 Beat마다 Situation 구성**:
   - `description`: 전체 상황 맥락 (Compound 감정의 context)
   - `event.description`: 사건 설명 (Joy/Distress 등의 context)
   - `event.desirability_for_self`: -1.0~1.0
   - `event.prospect`: anticipation/hope_fulfilled/fear_unrealized 등
   - `action.description`: 행동 설명 (Pride/Shame/Admiration/Reproach의 context)
   - `action.agent_id`: None(자기) / Some(id)(타인)
   - `action.praiseworthiness`: -1.0~1.0
   - `object`: 대상 매력도 (필요시)
3. **감정 평가**: Claude가 `POST /api/appraise` 호출
4. **PAD 자극 적용**: 대사 PAD를 `POST /api/stimulus`에 전달 (아래 "대사 PAD 측정" 참조)
5. **대화 종료**: `POST /api/after-dialogue`로 관계 갱신

**방법 B: Scene Focus 자동 전환**
1. **시나리오 JSON에 scene 필드 작성**: Focus 옵션 목록 + trigger 조건 정의
2. **시나리오 로드**: `POST /api/load` → scene 필드 파싱 → `Scene` 애그리거트 생성 → `scene.initial_focus()` 자동 appraise
3. **stimulus 반복**: 대사 PAD를 반복 적용 → `scene.check_trigger(state)` → 조건 충족 시 자동 Beat 전환
4. **상태 관찰**: `GET /api/scene-info`로 활성/대기 Focus 상태 확인. WebUI에서 Beat 전환 배너 표시
5. **대화 종료**: `POST /api/after-dialogue`로 최종 관계 갱신

**대사 PAD 측정 (stimulus 입력값 결정)**:

| 모드 | 조건 | 흐름 |
|------|------|------|
| **자동 분석** | `--features embed` 빌드 | 대사 텍스트 → `POST /api/analyze-utterance` → BGE-M3 임베딩 → 앵커 비교 → PAD(P,A,D) 자동 산출 → 슬라이더 반영 |
| **수동 입력** | embed feature 없음 | 대사의 감정 톤을 직접 판단하여 PAD 슬라이더 수동 조정 |

자동 분석 결과가 직관과 다르면 → ⑤ PAD 앵커 개선으로 분기.


---

### ④ 결과 검증

**누가**: Claude (1차 분석) → Bekay (최종 판단)
**입력**: 감정 상태, 프롬프트, trace
**산출물**: 평가 노트

**검증 관점**:

| 관점 | 질문 | 확인 방법 |
|------|------|----------|
| **감정 타당성** | 이 상황에서 이 감정이 맞는가? | 원작 인물의 반응과 비교 |
| **강도 적절성** | 너무 강하거나 약하지 않은가? | 0.0~1.0 범위에서 직관과 비교 |
| **성격 반영** | 성격이 감정에 영향을 미쳤는가? | trace에서 weight 확인 |
| **프롬프트 품질** | LLM이 이 프롬프트로 좋은 대사를 만들 수 있는가? | context가 구체적인가, 연기 지시가 일관적인가 |
| **관계 변동** | 대화 후 관계 변동이 합리적인가? | before/after 비교 |
| **Beat 전환** | 전환 시점이 자연스러운가? 감정 합치기 결과가 적절한가? | `GET /api/scene-info`로 활성/대기 Focus 확인, trigger 임계값 + merge 후 잔여 감정 |
| **대사 PAD 정확도** | 대사의 감정 톤이 올바르게 추출되었는가? | `POST /api/analyze-utterance` 결과 PAD를 직관과 비교. 예: 분노 대사가 P=+0.2면 앵커 문제 |

---

### ⑤ 개선점 식별

**누가**: Claude + Bekay (함께)
**입력**: 검증 결과
**산출물**: 구체적인 수정 사항 목록

**개선 유형별 대응**:

#### A. 프로파일 문제 — "이 인물이 이렇게 반응하면 안 되는데"
- 증상: 감정 유형은 맞는데 강도가 이상하거나, 예상과 다른 감정이 나옴
- 대응: HEXACO facet 값 조정 → ②로 복귀
- 도구: 브라우저 슬라이더 or Claude API
- 예: "Jim의 fearfulness를 0.6→0.4로 낮추면 Distress가 줄어드는지 확인"

#### B. 가중치/수식 문제 — "엔진 공식 자체가 잘못된 것 같아"
- 증상: 어떤 facet을 조정해도 원하는 결과가 안 나옴
- 대응: `engine.rs` 또는 `personality.rs`의 가중치 공식 수정
- 도구: 코드 수정 → `cargo test` → 서버 재시작 → ③ 재실행
- 예: "Gratitude가 안 나옴 → Compound 감정 생성 조건 검토"

#### C. PAD 앵커 문제 — "대사의 감정 톤이 잘못 추출됨"
- 증상: `POST /api/analyze-utterance`로 대사를 분석했을 때 PAD 값이 직관과 불일치
- 진단:
  1. 문제 대사를 `analyze-utterance`로 PAD 측정
  2. 예상 PAD와 실제 PAD를 비교 (예: 분노 대사인데 P=+0.2)
  3. 원인 추정: 해당 감정 영역의 앵커 문장이 부족하거나 도메인(무협/사극 등) 미커버
- 앵커 개선 루프:
  1. `pad.rs`의 앵커 문장 수정/추가 (도메인 특화 표현 확장)
  2. `cargo build --features embed` → 서버 재시작 (앵커 재임베딩)
  3. 동일 대사 `analyze-utterance` 재분석 → before/after PAD 비교
  4. 개선될 때까지 1-3 반복 → 만족 시 ③ 재실행
- 도메인별 앵커 전략:
  - **무협 어투**: "이 대역 무도한 놈!" 같은 무협 특유 분노/경멸 표현
  - **사극 어투**: "전하, 통촉하여 주시옵소서" 같은 격식체 비탄/호소 표현
  - **일상 어투**: 기본 앵커로 커버 가능
- 예: "무협 어투의 분노 대사가 P=+0.2로 나옴 → 부정 앵커에 무협 분노 표현 추가 → P=-0.5로 개선 확인"

#### D. 가이드/디렉티브 문제 — "감정은 맞는데 프롬프트가 어색해"
- 증상: 감정 결과는 적절한데 어조/태도/행동 지시가 부자연스러움
- 대응: `directive.rs`의 분기 로직 또는 `ko.toml`의 라벨 수정
- 도구: 코드 수정 → ③ 재실행
- 예: "분노 상태인데 '편안하고 온화한 어조'가 나옴 → Tone 분기 임계값 조정"

#### E. Mind Studio 도구 문제 — "결과는 맞는데 검증/분석이 불편해"
- 증상: 엔진 결과 자체는 적절하지만, Mind Studio의 시각화/워크플로우가 검증을 어렵게 만듦
- 대응: Mind Studio 프론트엔드 또는 핸들러 개선
- 도구: `src/bin/mind-studio/` 코드 수정 → 서버 재시작 → ③ 재실행

**현재 파악된 개선 후보**:

| 영역 | 현재 한계 | 개선 방향 | 우선순위 |
|------|----------|----------|---------|
| **감정 타임라인** | 턴 히스토리가 raw JSON으로만 표시 | 턴별 감정 변화를 시각적 그래프/차트로 표시 | 높음 |
| **히스토리 비교** | 턴 간 before/after 감정 차이를 직접 비교 불가 | 턴 간 감정 diff 요약 표시 | 높음 |
| **대사 PAD 일괄 분석** | `analyze-utterance`가 단건만 지원 | 대사 목록 일괄 PAD 분석 + 비교표 생성 | 높음 |
| **프롬프트 맥락** | 어떤 감정이 프롬프트의 어떤 부분을 결정했는지 불투명 | 감정→프롬프트 매핑 하이라이트 | 중간 |
| **Scene Focus 편집** | UI에서 Focus 생성/수정 불가 (JSON 직접 편집 필요) | Focus 편집 모달 추가 | 중간 |
| **관계 시각화** | 관계가 텍스트 리스트로만 표시 | 관계 매트릭스 또는 네트워크 뷰 | 낮음 |
| **시나리오 에디터** | scenario.json을 외부에서 직접 편집해야 함 | UI 내 시나리오 편집기 | 낮음 |

**개선 루프**:
1. 협업 중 도구 불편 사항 식별 → Bekay가 구체적 불편 사례 공유
2. Claude가 개선 방안 제안 + 구현 난이도 추정
3. 합의 → Claude가 Mind Studio 코드 수정 (`handlers.rs`, `static/index.html`)
4. `cargo run --features mind-studio` → 서버 재시작 → 브라우저 새로고침 → 개선 확인
5. ③ 재실행하여 개선된 도구로 검증 반복

---

### ⑥ 저장 + 다음 장면

**누가**: Claude
**입력**: 최종 결과
**산출물**: session 폴더

**저장 내용**:
- `scenario.json`: NPC + 관계 + turn_history + scene (Focus 옵션/trigger 조건 포함, API로 자동 기록됨)
- `test_report.md`: 상세 테스트 레포트
- `evaluation.md`: 평가 요약 + 개선 이력
- `turn{N}_{label}.txt`: 턴별 프롬프트

**다음 장면 선택 기준**:
- 이전 장면에서 발견한 이슈를 검증할 수 있는 장면
- 새로운 감정 유형/패턴을 테스트할 수 있는 장면
- 관계가 발전한 상태에서의 후속 장면


---

## 프로젝트 발전 로드맵

### Phase 1: 기초 검증 (완료)
**목표**: 엔진이 기본적으로 작동하는지 확인
**장면**: 잘 알려진 문학 작품 (허클베리핀 등)
**초점**: 감정 유형 선택 정확성, 성격 반영 여부
**달성**: 모든 기초 이슈 해결 — Compound 감정, significance, PowerLevel, Scene Focus 시스템

### Phase 2: 감정 정밀도 (완료)
**목표**: 감정 강도와 복합 감정의 정밀 튜닝 + Scene/Beat 시스템 검증
**장면**: Ch.15 안개 (신뢰 파열/회복), Ch.31 도덕적 위기
**초점**: Beat 전환 자동 판단, stimulus 관성 밸런스, merge_from_beat 동작 검증
**달성**: Scene Focus 시스템 구현 + Scene 애그리거트 캡슐화, Ch.15에서 5회 stimulus → 자동 Beat 전환 검증 완료

### Phase 3: PAD 앵커 최적화
**목표**: 대사 텍스트에서 정확한 감정 톤 추출
**장면**: 무협 도메인 대사 (청강만리, 와호장룡 스타일)
**초점**: 한국어/무협 어투 앵커 확장, before/after 비교

**워크플로우**:
1. **기준 대사 수집**: 도메인별 대표 대사 목록 작성 (분노, 비탄, 기쁨, 경멸 등 감정별)
2. **현재 PAD 측정**: `POST /api/analyze-utterance`로 각 대사의 PAD 자동 산출
3. **직관 비교표 작성**: 대사 | 예상 PAD | 실제 PAD | 차이
4. **앵커 개선**: 차이가 큰 감정 영역의 앵커 문장 추가/수정 (`pad.rs`)
5. **재빌드**: `cargo build --features embed` → 서버 재시작 (앵커 재임베딩)
6. **A/B 비교**: 동일 대사 재분석 → before/after PAD 비교 → 개선 효과 측정
7. **반복**: 4-6을 수렴할 때까지 반복

**도메인별 앵커 전략**:
- 무협: 분노/경멸/호소 등 격렬한 감정 표현 중심
- 사극: 격식체 비탄/충격/간곡함 중심
- 일상: 기본 앵커로 대부분 커버, 은어/속어 보강

### Phase 4: 가이드 품질
**목표**: LLM이 받아서 좋은 대사를 만들 수 있는 프롬프트
**장면**: 실제 LLM 대사 생성 → 대사 품질 평가
**초점**: 어조/태도/행동 분기의 세밀화, 금지 사항 정확성

### Phase 5: 무협 RPG 적용
**목표**: 칠국춘추 세계관의 인물들로 테스트
**장면**: 오리지널 시나리오 (무백/교룡/수련/소호)
**초점**: 게임 맥락에서의 대화 시뮬레이션

---

## 세션 네이밍 규칙

```
data/{도서명}/{장면명}/session_{NNN}/
```

| 구성 요소 | 규칙 | 예시 |
|----------|------|------|
| 도서명 | snake_case 영문 | `huckleberry_finn` |
| 장면명 | `ch{N}_{핵심키워드}` | `ch8_jackson_island_meeting` |
| 세션 번호 | 3자리 패딩 | `session_001` |

세션 간 비교가 핵심이므로, 같은 장면의 session_001과 session_002를 비교하여 개선 효과를 측정한다.

---

## 실전 예시: 협업 대화 패턴

### 예시 1: 수동 Beat (방법 A)

**Bekay → Claude**:
```
"허클베리핀 Ch.15 안개 장면으로 테스트해줘.
첨부 파일 참고해서 인물 설정하고, 짐이 헉의 거짓말에 화내는 장면부터
사과 받는 장면까지 3턴으로 나눠서 돌려줘."
```

**Claude 작업 순서**:
1. 첨부 자료에서 Ch.15 원문 추출
2. Jim/Huck 프로필이 이미 있으면 로드, 없으면 생성
3. 관계를 Ch.8 이후 상태로 설정 (closeness=0.146, trust=0.180)
4. Turn 1: 거짓말 발각 → Turn 2: 짐의 비난 → Turn 3: 헉의 사과
5. 각 턴마다 `POST /api/appraise` + 결과 분석
6. test_report.md 작성 + session 저장

### 예시 2: Scene Focus 자동 전환 + 대사 PAD 분석 (방법 B)

**Bekay → Claude**:
```
"Ch.15 시나리오 JSON에 scene 필드 추가해서 Focus 자동 전환 테스트해줘.
거짓말 발각(Initial) → 사과 수용(Anger < 0.4) 두 Focus로 설정하고,
대사 PAD는 embed 모델로 자동 분석해줘."
```

**Claude 작업 순서**:
1. 시나리오 JSON에 `scene` 필드 작성 (Focus 2개 + trigger 조건)
2. `POST /api/load`로 시나리오 로드 → Initial Focus 자동 appraise
3. 대사 "네 이놈! 날 걱정하며 울었는데 그걸 장난감으로 삼다니!" → `POST /api/analyze-utterance` → PAD 확인
4. 산출된 PAD를 `POST /api/stimulus`에 반영 → 감정 변동 + Beat 전환 체크
5. `GET /api/scene-info`로 Focus 상태 확인 → 자동 전환 여부 관찰
6. 대사 반복 → Anger가 0.4 아래로 떨어지면 "사과 수용" Focus로 자동 전환
7. `POST /api/after-dialogue`로 관계 갱신 + session 저장

### 예시 3: PAD 앵커 개선

**Bekay → Claude**:
```
"무협 어투 대사 PAD가 이상해. '이 대역 무도한 놈!'이 P=+0.1로 나오는데
분노 대사니까 P가 마이너스여야 하잖아. 앵커 개선해보자."
```

**Claude 분석 → 대응**:
1. `POST /api/analyze-utterance`로 문제 대사 PAD 측정 → P=+0.1, A=+0.6, D=+0.4
2. 원인: 무협 어투의 분노 표현이 앵커에 없어서 "강한 명령/지배" 쪽으로 매핑됨
3. `pad.rs`에 무협 분노 앵커 추가: "이 대역 무도한 놈!", "네놈의 만행을 용서치 않겠다" 등
4. `cargo build --features embed` → 서버 재시작
5. 동일 대사 재분석 → P=-0.5, A=+0.7, D=+0.3 (개선 확인)
6. 다른 무협 분노 대사로도 검증 → session 저장

### 예시 4: 감정 강도 개선 요청

**Bekay → Claude**:
```
"Turn 2에서 짐의 Anger가 0.3인데 너무 약한 것 같아.
원작에서 짐은 이 장면에서 진심으로 상처받아서 화내거든.
성격 문제인지 가중치 문제인지 같이 봐보자."
```

**Claude 분석 → 대응**:
1. trace 확인: `Anger: comp1=Reproach(0.X), comp2=Distress(0.Y), result=0.3`
2. 원인 분석: "Jim의 A(원만성)가 너무 높아서 patience가 Anger를 억제하고 있음"
3. 대안 제시:
   - A: patience를 0.7→0.5로 낮추고 재실행
   - B: Compound 감정의 patience 브레이크 계수를 조정
4. Bekay가 선택 → 수정 → session_002로 재실행 → 비교

---

## Mind Studio API 참고표

워크플로우 각 단계에서 사용하는 주요 엔드포인트:

| 단계 | 엔드포인트 | 설명 |
|------|-----------|------|
| **② 프로파일** | `POST /api/npcs` | NPC 생성/수정 |
| | `POST /api/relationships` | 관계 생성/수정 |
| | `POST /api/objects` | 오브젝트 생성/수정 |
| **③ 감정 평가** | `POST /api/appraise` | 상황 평가 → 감정 생성 |
| | `POST /api/stimulus` | PAD 자극 적용 + Beat 전환 체크 |
| | `POST /api/analyze-utterance` | 대사 텍스트 → PAD 자동 산출 (embed) |
| | `POST /api/scene` | Scene 시작 (Focus 등록 + Initial appraise) |
| | `POST /api/guide` | 현재 감정에서 가이드 재생성 |
| **④ 결과 검증** | `GET /api/scene-info` | 활성/대기 Focus 상태 조회 |
| | `GET /api/history` | 턴별 히스토리 조회 |
| | `GET /api/situation` | 현재 상황 폼 상태 조회 |
| **⑥ 저장** | `POST /api/save` | 전체 상태 JSON 저장 |
| | `POST /api/load` | 시나리오 로드 (scene 필드 자동 처리) |
| | `POST /api/after-dialogue` | 대화 종료 + 관계 갱신 |
| **조회** | `GET /api/scenarios` | 시나리오 목록 |
| | `GET /api/scenario-meta` | 현재 시나리오 메타데이터 |

## MCP Server (AI Agent 연동)

Claude Code 등 AI Agent가 Mind Studio를 자율적으로 사용할 때는 MCP 서버를 통해 도구로 호출합니다.

### 설정

```bash
# Mind Studio 서버 실행
cargo run --features mind-studio --bin npc-mind-studio

# Python MCP 의존성 설치
pip install -r mcp/requirements.txt
```

프로젝트의 `.mcp.json`:
```json
{
  "mcpServers": {
    "mind-studio": {
      "command": "python",
      "args": ["mcp/mind_studio_server.py"],
      "env": { "MIND_STUDIO_URL": "http://localhost:3000" }
    }
  }
}
```

### MCP 도구 ↔ HTTP API 매핑

| MCP 도구 | HTTP 엔드포인트 | 워크플로우 단계 |
|----------|----------------|---------------|
| `create_npc` | `POST /api/npcs` | ② 프로파일 |
| `create_relationship` | `POST /api/relationships` | ② 프로파일 |
| `create_object` | `POST /api/objects` | ② 프로파일 |
| `appraise` | `POST /api/appraise` | ③ 감정 평가 |
| `apply_stimulus` | `POST /api/stimulus` | ③ 감정 평가 |
| `analyze_utterance` | `POST /api/analyze-utterance` | ③ 대사 PAD 분석 |
| `generate_guide` | `POST /api/guide` | ③ 감정 평가 |
| `start_scene` | `POST /api/scene` | ③ 감정 평가 |
| `get_scene_info` | `GET /api/scene-info` | ④ 결과 검증 |
| `get_history` | `GET /api/history` | ④ 결과 검증 |
| `get_situation` | `GET /api/situation` | ④ 결과 검증 |
| `update_situation` | `PUT /api/situation` | ④ WebUI 동기화 |
| `get_scenario_meta` | `GET /api/scenario-meta` | 조회 |
| `after_dialogue` | `POST /api/after-dialogue` | ⑥ 저장 |
| `save_scenario` | `POST /api/save` | ⑥ 저장 |
| `load_scenario` | `POST /api/load` | ⑥ 저장 |
| `list_scenarios` | `GET /api/scenarios` | 조회 |
| `list_npcs` | `GET /api/npcs` | 조회 |
| `list_relationships` | `GET /api/relationships` | 조회 |
| `list_objects` | `GET /api/objects` | 조회 |
| `delete_npc` | `DELETE /api/npcs/{id}` | 관리 |
| `delete_relationship` | `DELETE /api/relationships/{owner}/{target}` | 관리 |
| `delete_object` | `DELETE /api/objects/{id}` | 관리 |

자세한 도구 파라미터와 사용 예시는 [mcp/README.md](../../mcp/README.md) 참고.

### MCP Agent 워크플로우 예시: 대사 PAD 분석 + 결과 검증

AI Agent가 MCP 도구만으로 대사 분석 → stimulus → 히스토리 검증까지 수행하는 흐름:

```
1. load_scenario(path="huckleberry_finn/ch15_fog_trash/session_001")
   # 시나리오 로드

2. get_scenario_meta()
   # → {"name":"Ch.15 안개 속 쓰레기", ...} — 로드 확인

3. analyze_utterance(utterance="네 이놈! 날 걱정하며 울었는데 그걸 장난감으로 삼다니!")
   # → {"pleasure":-0.5, "arousal":0.7, "dominance":0.3}

4. apply_stimulus(npc_id="jim", partner_id="huck",
                  pleasure=-0.5, arousal=0.7, dominance=0.3,
                  situation_description="헉의 거짓말에 분노")
   # → 감정 갱신 + beat_changed 확인

5. get_history()
   # → 턴별 감정 변화 추적 — Turn 1: scene/appraise → Turn 2: stimulus

6. get_scene_info()
   # → 활성 Focus 확인, trigger 조건 충족 여부

7. # 대사 반복 (4-6) → Beat 전환 시 프롬프트 변화 관찰

8. after_dialogue(npc_id="jim", partner_id="huck",
                  praiseworthiness=0.3, significance=0.7)

9. save_scenario(path="huckleberry_finn/ch15_fog_trash/session_002")
```
