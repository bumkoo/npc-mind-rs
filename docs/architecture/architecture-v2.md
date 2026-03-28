# NPC 심리 엔진 아키텍처 v2 (현행화)

## 개요

NPC 심리 엔진은 **성격(HEXACO)**이 **상황(Situation)**을 해석하여 **감정(OCC)**을 생성하고, 이를 LLM이 연기할 수 있도록 **가이드(ActingGuide)**를 출력하는 시스템이다.

v2 아키텍처의 핵심은 **다중 초점 상황 평가(Multi-focus Appraisal)**와 **PAD 기반 동적 자극 처리(Dynamic Stimulus)**의 결합이다.

---

## 4레이어 아키텍처

### 전체 흐름 및 데이터 전이

```
┌─────────────────────────────────────────────────────────┐
│  레이어 1: Situation (다중 초점: Option<Event/Action/Object>) │
│  "밀고(Action)와 독(Event)이 동시에 일어난 상황"            │
│                                                           │
│  레이어 2: HEXACO (성격 가중치, AppraisalWeights trait)     │
│                                                           │
│  레이어 3: Relationship (친밀도/신뢰도 기반 강도 보정)        │
│                                                           │
│  → AppraisalEngine.appraise() → 초기 EmotionState         │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│  레이어 4: PAD 자극 (대화 중 동적 변동)                      │
│                                                           │
│  대사 텍스트 → TextEmbedder → PadAnalyzer(앵커 비교)       │
│  → PAD 자극 × StimulusEngine(성격별 자극 수용도)           │
│  → 갱신된 EmotionState (FADE_THRESHOLD 기반 소멸 처리)      │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
 ActingGuide 생성 (Presentation Layer)
 → EmotionState + Personality + Situation → LLM 연기 가이드
                         │
                         ▼
 대화 종료 → Relationship.after_dialogue() → 관계 갱신
```

---

## 핵심 컴포넌트 설계

### 1. Situation (다중 초점 구조)
하나의 상황이 여러 성격을 가질 수 있도록 `Option<EventFocus>`, `Option<ActionFocus>`, `Option<ObjectFocus>` 3개 필드를 사용한다.
- **복합 감정 자동 생성**: 엔진이 Action과 Event의 동시 존재를 감지하여 Anger, Gratitude 등을 자동 도출.
- **Action 3분기**: `agent_id`가 None이면 자기(Pride/Shame), Some이면 대화 상대 또는 제3자(Admiration/Reproach).
- **Emotion context**: 각 Focus의 `description`이 감정에 부착되어 LLM 프롬프트에 포함됨.

### 2. Relationship (관계 기반 보정)
- **rel_mul**: `(1.0 + closeness × 0.5).max(0.0)` — **Admiration/Reproach에만 적용**. 나머지 감정은 rel_mul 없음.
- **trust_mod**: `1.0 + trust × 0.3` — Admiration/Reproach에 적용.
- **empathy/hostility_rel_modifier**: Fortune-of-others 감정에 별도 적용.

### 3. 성격 기반 감정 변조 (Personality Modulation)
성격(HEXACO)은 발생한 감정을 증폭하거나 억제하는 필터 역할을 한다. 범용 가중치 계수(`W`)는 **0.3**을 기본으로 한다.
- **증폭 공식**: `1.0 + (Score * W)` (예: 외향성(X)이 높을수록 긍정적 사회적 감정 강화)
- **억제 공식**: `1.0 - (max(0, Score) * W)` (예: 인내심(Agreeableness-Gentleness)이 높을수록 분노 억제)

### 4. 감정 처리의 이원화 (Appraisal vs Stimulus)
시스템은 사건에 대한 즉각적인 반응과 지속적인 대화 흐름을 분리하여 처리한다.
- **AppraisalEngine (초기 생성)**: 상황(Situation) 발생 시 즉각적인 감정의 **'베이스라인'**을 결정한다 (Step Response).
- **StimulusEngine (동적 튜닝)**: 대화 중 발생하는 PAD 자극을 통해 감정의 강도를 실시간으로 **'미세 조정'**한다 (Dynamic Tuning).

### 5. PAD 자극 및 StimulusEngine
대화 중 감정의 흐름을 담당한다.
- **D-스케일러 내적**: `(P1*P2 + A1*A2) * (1.0 + D_gap * 0.3)`. P·A가 방향을 결정하고 D축 격차가 효과의 크기를 조절한다.
- **자극 수용도(Absorb Rate)**: 성격(E, A, C)에 따라 자극을 증폭하거나 완충(Buffer)하여 감정의 변화 속도를 조절한다.
- **자연 소멸(Fade)**: 강도가 0.05 미만으로 떨어진 감정은 목록에서 삭제되어 '심리적 해소'를 표현한다.

### 6. Acting Guide (지시문 생성)
도메인에서 생성된 감정을 사용자가 이해할 수 있는 형태(ActingDirective)로 변환한다.
- **데이터 결합**: 현재의 `EmotionState`, `Personality`, `Situation` 맥락을 결합하여 최적의 연기 지침을 도출한다.
- **Presentation 레이어**: `presentation/` 모듈을 통해 다국어(ko, en)를 지원하며, LLM이 이해하기 쉬운 구조화된 텍스트로 포맷팅한다.

---

## 포트 앤드 어댑터 (Hexagonal Architecture)

외부 의존성(I/O, ML 모델)과 순수 도메인 로직을 분리한다.

| 구분 | 컴포넌트 | 역할 |
|------|----------|------|
| **도메인** | `AppraisalEngine`, `StimulusEngine` | 성격/상황/자극 기반 감정 연산 (순수 함수) |
| **포트** | `TextEmbedder`, `RelationshipRepository` | 외부 세계와의 인터페이스 정의 |
| **어댑터** | `OrtEmbedder` (ONNX), `InMemoryRepo` | 구체적인 기술 구현 (임베딩 모델 실행 등) |

---

## 구현 로드맵 및 상태 (사이클)

### 완료
- [x] 사이클 1~3: HEXACO, OCC, ActingGuide 기본 모델
- [x] 사이클 4~5: Relationship 도메인 모델 및 AppraisalEngine 통합
- [x] 사이클 6~7: PAD 모델 및 StimulusEngine (D-스케일러 반영)
- [x] 사이클 10~10.5: ONNX(ort) 기반 임베딩 어댑터 리팩터링
- [x] DDD 리팩토링: rel_mul 정리, Action 3분기, Emotion context
- [x] WebUI: axum 기반 협업 도구 (API + 브라우저 SPA)
- [x] 턴 히스토리: 장면 설정/감정 평가/프롬프트를 scenario.json에 보존
- [x] 테스트 시나리오: 허클베리핀 Ch.8 잭슨 섬 첫 만남 (Jim 관점 4턴)

### 예정
- [ ] PAD 앵커 동적 관리 (앵커 편집 + 재임베딩)
- [ ] Power → Tone/Attitude 매핑
- [ ] 톤 시프트 감지 설계
- [ ] 대화 맥락 유지 기능 고도화

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| 0.1.0 | 2026-03-24 | 초기 설계안 |
| 0.9.0 | 2026-03-26 | 다중 초점 Situation, D-스케일러 PAD 공식, 관계 보정 수치, 포트 앤드 어댑터 구조 반영 |
| 1.0.0 | 2026-03-28 | Situation→Option 기반 전환, Action 3분기, Emotion context, rel_mul 범위 한정(Admiration/Reproach만), devgui 삭제→webui 단일화, 턴 히스토리, 테스트 데이터 폴더 구조 |
