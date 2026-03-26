# NPC 심리 엔진 아키텍처 v2 (현행화)

## 개요

NPC 심리 엔진은 **성격(HEXACO)**이 **상황(Situation)**을 해석하여 **감정(OCC)**을 생성하고, 이를 LLM이 연기할 수 있도록 **가이드(ActingGuide)**를 출력하는 시스템이다.

v2 아키텍처의 핵심은 **다중 초점 상황 평가(Multi-focus Appraisal)**와 **PAD 기반 동적 자극 처리(Dynamic Stimulus)**의 결합이다.

---

## 5레이어 아키텍처

### 전체 흐름 및 데이터 전이

```
┌─────────────────────────────────────────────────────────┐
│  레이어 1: Situation (v2: 다중 초점 지원, Vec<Focus>)      │
│  "밀고(Action)와 독(Event)이 동시에 일어난 상황"            │
│                                                           │
│  레이어 2: HEXACO (성격 가중치, Modifier 메서드 활용)        │
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
┌─────────────────────────────────────────────────────────┐
│  레이어 5: Acting Guide (Presentation)                    │
│                                                           │
│  최종 EmotionState + Personality + Situation              │
│  → GuideEngine → ActingDirective (지문, 톤, 행동 양식)      │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│  대화 종료 후 (결과 전이)                                   │
│  최종 EmotionState + Situation → Relationship 갱신         │
└─────────────────────────────────────────────────────────┘
```

---

## 핵심 컴포넌트 설계

### 1. Situation (다중 초점 구조)
v2에서는 하나의 상황이 여러 성격을 가질 수 있도록 `Vec<SituationFocus>`를 사용한다.
- **복합 감정 자동 생성**: 엔진이 `Action`과 `Event`의 동시 존재를 감지하여 `Anger`, `Gratitude` 등을 자동으로 도출한다.

### 2. Relationship (관계 기반 심각성 보정)
상대방과의 관계는 감정의 질과 양을 결정한다.
- **친밀도 배율 (`rel_mul`)**: `1.0 + closeness.intensity() * 0.5`. 관계가 깊을수록(적대적이든 친밀하든) 정서적 진폭이 커진다.
- **신뢰도 보정 (`trust_mod`)**: `1.0 + trust.value() * 0.3`. 신뢰하는 이의 행동에는 더 민감하게, 불신하는 이의 행동에는 덤덤하게 반응한다.

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

### 완료 (v2 코어 구현)
- [x] 사이클 1~3: HEXACO, OCC, ActingGuide 기본 모델
- [x] 사이클 4~5: Relationship 도메인 모델 및 AppraisalEngine 통합
- [x] 사이클 6~7: PAD 모델 및 StimulusEngine (D-스케일러 반영)
- [x] 사이클 10~10.5: ONNX(ort) 기반 임베딩 어댑터 리팩터링

### 진행 중 / 예정
- [ ] 사이클 11: 무협 도메인 특화 PAD 앵커 확장 및 튜닝
- [ ] 사이클 12: 대화 맥락 유지 기능 고도화 (Long-term Memory 검토)

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| 0.1.0 | 2026-03-24 | 초기 설계안 |
| 0.9.0 | 2026-03-26 | **대규모 현행화**: 다중 초점 Situation, D-스케일러 PAD 공식, 관계 보정 수치, 포트 앤드 어댑터 구조 반영 및 완료된 사이클 업데이트. |
