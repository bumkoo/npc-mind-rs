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
라이브러리의 핵심 진입점으로, 복잡한 도메인 로직의 실행 순서를 관리한다.
- **제네릭 엔진 주입**: `MindService<R, A: Appraiser, S: StimulusProcessor>`. 기본값으로 `AppraisalEngine`, `StimulusEngine` 제공.
- **저장소 추상화**: `MindRepository` 포트를 통해 외부 저장소(DB, Memory)에 의존하지 않고 상태를 관리.
- **Scene/Beat 통합**: `apply_stimulus()` 내부에서 Beat 전환을 자동 처리. `start_scene()`, `scene_info()`, `load_scene_focuses()` 제공.
- **관계 갱신 분리**: `after_beat()` (Beat 종료, 감정 유지) vs `after_dialogue()` (Scene 종료, 감정 초기화).
- **포맷팅 분리**: `MindService`는 도메인 결과(`*Result`)만 반환. 포맷팅은 `FormattedMindService` 또는 `result.format()` 사용.

### 2. AppraisalEngine (Modularized)
기존의 거대했던 감정 평가 로직을 관심사에 따라 물리적인 서브 모듈로 분리하였다.
- **Event Module**: 사건의 바람직함 평가 (Joy, Distress, Hope, Fear 등).
- **Action Module**: 행위의 도덕성 평가 (Pride, Admiration, Reproach 등).
- **Object Module**: 대상의 매력도 평가 (Love, Hate).
- **Compound Module**: 기초 감정의 결합 (Anger, Gratitude 등).
- **Helpers**: 중복된 감정 생성 및 Tracing 로직을 `add_valence` 등의 헬퍼로 통합.

### 2.5. StimulusEngine (관성 적용)
대사 자극에 의한 감정 변동 처리. 관성 공식으로 강한 감정은 자극에 덜 흔들린다.
- **관성**: `inertia = max(1.0 - intensity, MIN_INERTIA)` — intensity=1.0이어도 최소 반응 보장.
- **새 감정 생성 안 함**: 기존 감정의 강도만 조정. 새 감정은 appraise의 역할.

### 2.6. Scene 도메인 애그리거트
`Scene`은 장면 내 Focus/Beat 전환 로직을 캡슐화하는 도메인 애그리거트 루트이다.
- **Scene**: 애그리거트 루트 (npc_id, partner_id, focuses, active_focus_id). `SceneStore` 포트를 통해 통째로 저장/조회/삭제.
  - `check_trigger(state)` — 대기 중 Focus의 감정 조건 체크, 충족된 Focus 반환.
  - `set_active_focus(focus_id)` — 활성 Focus 설정.
  - `initial_focus()` — Initial 트리거를 가진 Focus 검색.
- **SceneFocus**: Focus 옵션 (id, description, trigger, event/action/object).
- **FocusTrigger**: Initial (즉시 적용) 또는 Conditions (감정 상태 조건, `OR[AND[...]]` 구조).
- **merge_from_beat**: Beat 전환 시 이전 감정과 새 감정 합치기 (같은 감정은 max, threshold 미만 소멸).

### 2.7. 튜닝 상수 (`tuning.rs`)
모든 조정 가능한 파라미터를 한 파일에 중앙 관리. 플레이테스트 시 이 파일만 수정.

### 3. DTO & Mapping (Standardized)
외부 입력(JSON 등)과 도메인 모델 간의 변환 책임을 명확히 분리한다.
- **to_domain 패턴**: DTO가 직접 도메인 모델로 변환되는 메서드를 가지며, 이때 필요한 컨텍스트(관계, 객체 정보 등)를 `MindRepository`로부터 주입받는다.

### 4. Relationship (관계 기반 보정)
- **rel_mul**: `(1.0 + closeness × 0.5).max(0.0)` — Admiration/Reproach에만 적용.
- **trust_mod**: `1.0 + trust × 0.3` — Action 감정 가중치로 작용.
- **Social Modifiers**: 타인의 운에 대한 공감(`empathy`) 및 적대(`hostility`) 배율 별도 관리.
- **significance**: 상황 중요도 (0.0~1.0). 관계 갱신 배율 = `1.0 + significance × 3.0` (최대 4배).
- **PowerLevel**: 5단계 (VeryHigh/High/Neutral/Low/VeryLow) + 행동 지시 포함 라벨.

---

## 포트 앤드 어댑터 (Hexagonal Architecture)

외부 의존성과 순수 도메인 로직을 포트(Interface)를 통해 철저히 격리한다.

| 구분 | 컴포넌트 | 역할 |
|------|----------|------|
| **도메인** | `AppraisalEngine`, `StimulusEngine` | 순수 심리 연산 (I/O 없음) |
| **포트** | `MindRepository`(`NpcWorld`+`EmotionStore`+`SceneStore`), `Appraiser`, `StimulusProcessor`, `GuideFormatter`, `TextEmbedder` | 외부 세계와의 인터페이스 정의 (모두 `ports.rs`에 위치). `SceneStore`는 Scene 애그리거트 단위로 `get_scene`/`save_scene`/`clear_scene` 제공 |
| **어댑터** | `InMemoryRepository` (기본 MindRepository), `OrtEmbedder`, `LocaleFormatter`, `KoreanFormatter`; Mind Studio 전용: `AppStateRepository` | 구체적인 기술 구현 (InMemory/JSON, ONNX, TOML 등) |

---

## 구현 로드맵 및 상태 (사이클)

### 완료
- [x] 사이클 1~10: HEXACO, OCC, PAD, 임베딩 어댑터 기본 구현
- [x] Application Service 도입: `MindService`를 통한 API 진입점 단일화
- [x] AppraisalEngine 리팩터링: 물리적 모듈 분리 및 중복 로직 통합
- [x] 에러 처리 표준화: `AppError`, `IntoResponse` 도입
- [x] 테스트 인프라 혁신: `TestContext` 기반의 표준화된 테스트 Fixture 구축
- [x] HopeFulfilled/FearConfirmed 시 Joy/Distress 동시 생성 (Gratitude 등 Compound 정상화)
- [x] PowerLevel 3→5단계 확장 + 행동 지시 라벨
- [x] significance 파라미터: 상황 중요도에 따른 관계 변동 배율
- [x] stimulus 관성 공식: 강한 감정은 자극에 덜 흔들림
- [x] Scene Focus 시스템: Focus 옵션 목록 + 감정 조건 기반 자동 Beat 전환
- [x] merge_from_beat: Beat 전환 시 이전/새 감정 합치기
- [x] after_beat vs after_dialogue 분리
- [x] 튜닝 상수 중앙 관리 (tuning.rs)
- [x] 플러거블 포맷터: MindService에서 Presentation 분리, FormattedMindService 도입
- [x] 다국어 로케일: 빌트인 (ko/en) + 커스텀 오버라이드 + TOML deep merge
- [x] 포트 주입: Appraiser/StimulusProcessor 제네릭 주입 (기본값 제공)
- [x] Scene/Beat 로직 MindService 이동: WebUI handlers에서 도메인 엔진 직접 호출 제거
- [x] MindRepository ports.rs 이동 + RelationshipRepository 제거 (중복)
- [x] NpcId newtype 제거: String으로 통일
- [x] AppraisalEngineImpl 트레이트 제거: 단순 함수로 대체
- [x] directive 감정 조회 반복 제거 + significant() 불필요 할당 제거
- [x] Scene 도메인 애그리거트 캡슐화: check_trigger/set_active_focus/initial_focus 메서드 이동, SceneStore 포트 단순화 (get_scene/save_scene/clear_scene)
- [x] Scene 독립 단위 테스트 (scene_test.rs) 추가

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
| 2.0.0 | 2026-03-30 | Scene Focus 시스템, Beat 전환, stimulus 관성, merge_from_beat, significance, PowerLevel 5단계, tuning.rs 중앙 관리 |
| 2.1.0 | 2026-03-30 | 플러거블 포맷터, 다국어 로케일, 포트 주입, Scene/Beat MindService 이동, 코드 정리 |
| 2.2.0 | 2026-03-31 | Scene 도메인 애그리거트 캡슐화, SceneStore 포트 단순화, Scene 독립 단위 테스트 |
