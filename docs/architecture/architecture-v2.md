# NPC 심리 엔진 아키텍처 v2 (현행화)

## 개요

NPC 심리 엔진은 **성격(HEXACO)**이 **상황(Situation)**을 해석하여 **감정(OCC)**을 생성하고, 이를 LLM이 연기할 수 있도록 **가이드(ActingGuide)**를 출력하는 시스템이다.

v2 아키텍처의 핵심은 **3계층 구조(Domain-Application-Infrastructure)**로의 분리와 **모듈화된 감정 평가 엔진**이다.

---

## 3계층 아키텍처 (Layered Architecture)

### 전체 흐름 및 데이터 전이

```
┌─────────────────────────────────────────────────────────────────┐
│ Infrastructure / Presentation Layer (WebUI, CLI, Adapters)      │
│ - Axum Handlers: HTTP 요청 처리 및 응답 변환                    │
│ - ORT Embedder: ONNX 기반 텍스트 임베딩                         │
└───────────────┬─────────────────────────────────────────────────┘
                │ AppRequest (DTO)
                ▼
┌─────────────────────────────────────────────────────────────────┐
│ Application Layer (MindService, DTO Mappers)                    │
│ - MindService: 도메인 객체 조립 및 오케스트레이션               │
│ - DTO Mapping: SituationInput → Situation 도메인 모델 변환      │
└───────────────┬─────────────────────────────────────────────────┘
                │ Domain Models
                ▼
┌─────────────────────────────────────────────────────────────────┐
│ Domain Layer (Pure Logic & State)                               │
│ - AppraisalEngine: 모듈화된 감정 평가 (Event/Action/Object)     │
│ - StimulusEngine: PAD 기반 동적 자극 처리                       │
│ - Personality/Relationship: 핵심 심리 모델 및 규칙              │
└─────────────────────────────────────────────────────────────────┘
```

---

## 핵심 컴포넌트 설계

### 1. MindService (Application Entry Point)
라이브러리의 유일한 진입점으로, 복잡한 도메인 로직의 실행 순서를 관리한다.
- **오케스트레이션**: NPC/관계 로드 → 상황 평가 → 감정 갱신 → 가이드 생성의 흐름을 제어.
- **저장소 추상화**: `MindRepository` 포트를 통해 외부 저장소(DB, Memory)에 의존하지 않고 상태를 관리.

### 2. AppraisalEngine (Modularized)
기존의 거대했던 감정 평가 로직을 관심사에 따라 물리적인 서브 모듈로 분리하였다.
- **Event Module**: 사건의 바람직함 평가 (Joy, Distress, Hope, Fear 등).
- **Action Module**: 행위의 도덕성 평가 (Pride, Admiration, Reproach 등).
- **Object Module**: 대상의 매력도 평가 (Love, Hate).
- **Compound Module**: 기초 감정의 결합 (Anger, Gratitude 등).
- **Helpers**: 중복된 감정 생성 및 Tracing 로직을 `add_valence` 등의 헬퍼로 통합.

### 3. DTO & Mapping (Standardized)
외부 입력(JSON 등)과 도메인 모델 간의 변환 책임을 명확히 분리한다.
- **to_domain 패턴**: DTO가 직접 도메인 모델로 변환되는 메서드를 가지며, 이때 필요한 컨텍스트(관계, 객체 정보 등)를 `MindRepository`로부터 주입받는다.

### 4. Relationship (관계 기반 보정)
- **rel_mul**: `(1.0 + closeness × 0.5).max(0.0)` — Admiration/Reproach에만 적용.
- **trust_mod**: `1.0 + trust × 0.3` — Action 감정 가중치로 작용.
- **Social Modifiers**: 타인의 운에 대한 공감(`empathy`) 및 적대(`hostility`) 배율 별도 관리.

---

## 포트 앤드 어댑터 (Hexagonal Architecture)

외부 의존성과 순수 도메인 로직을 포트(Interface)를 통해 철저히 격리한다.

| 구분 | 컴포넌트 | 역할 |
|------|----------|------|
| **도메인** | `AppraisalEngine`, `StimulusEngine` | 순수 심리 연산 (I/O 없음) |
| **포트** | `MindRepository`, `TextEmbedder` | 외부 세계와의 인터페이스 정의 |
| **어댑터** | `OrtEmbedder`, `AppStateRepository` | 구체적인 기술 구현 (ONNX, InMemory 등) |

---

## 구현 로드맵 및 상태 (사이클)

### 완료
- [x] 사이클 1~10: HEXACO, OCC, PAD, 임베딩 어댑터 기본 구현
- [x] Application Service 도입: `MindService`를 통한 API 진입점 단일화
- [x] AppraisalEngine 리팩터링: 물리적 모듈 분리 및 중복 로직 통합
- [x] 에러 처리 표준화: `AppError`, `IntoResponse` 도입
- [x] 테스트 인프라 혁신: `TestContext` 기반의 표준화된 테스트 Fixture 구축

### 예정
- [ ] PAD 앵커 동적 관리 (앵커 편집 + 재임베딩)
- [ ] Power → Tone/Attitude 매핑 고도화
- [ ] 멀티 NPC 대화 맥락(Context) 유지 기능

---

## 버전 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| 0.1.0 | 2026-03-24 | 초기 설계안 |
| 1.0.0 | 2026-03-28 | Situation→Option 전환, Action 3분기, Emotion context 반영 |
| 1.1.0 | 2026-03-29 | Application 계층 도입, AppraisalEngine 모듈화, TestContext 인프라 구축 반영 |
