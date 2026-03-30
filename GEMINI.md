# GEMINI.md - NPC Mind Engine 개발 가이드

이 파일은 `npc-mind-rs` 프로젝트의 구조, 도메인 로직, 개발 컨벤션 및 실행 방법을 정의합니다. 모든 AI 에이전트와 개발자는 이 가이드를 최우선으로 준수해야 합니다.

## 1. 프로젝트 개요
`npc-mind-rs`는 NPC의 **성격(HEXACO)**과 **관계(Relationship)**를 바탕으로 **상황(Context)**을 해석하여 **감정(OCC)**을 생성하고, 이를 LLM이 연기할 수 있는 **지시문(Acting Guide)**으로 변환하는 Rust 기반 심리 엔진입니다.

### 핵심 기술 스택
- **Language:** Rust (Edition 2021)
- **Architecture:** Hexagonal Architecture (Ports and Adapters) + DDD
- **Layering:** Domain (Logic) → Application (Orchestration) → Infrastructure/Presentation (WebUI, Adapters)
- **Libraries:**
  - `serde`, `serde_json`: 데이터 직렬화 및 JSON API 지원
  - `thiserror`: 도메인 및 서비스 계층 에러 핸들링
  - `axum`, `tokio`: Web UI 서버 및 비동기 런타임
  - `tracing`: 감정 생성 과정의 투명한 추적 (Appraisal Trace)
  - `ort`: ONNX Runtime (텍스트 임베딩/PAD 분석용)

## 2. 프로젝트 구조
```
src/
  application/    # 어플리케이션 계층 (라이브러리 진입점)
    mind_service.rs    # MindService — 핵심 오케스트레이션
    formatted_service.rs # FormattedMindService — MindService + 포맷터 조합
    dto.rs             # API 데이터 전송 객체 및 도메인 변환 로직
  domain/         # 핵심 도메인 로직 (순수 함수 및 상태 관리)
    tuning.rs     # 튜닝 상수 — 모든 조정 가능 파라미터 중앙 관리
    personality.rs # HEXACO 모델, Score VO
    emotion/      # OCC 감정 엔진 및 상태 관리
      appraisal/  # 세부 평가 모듈 (event, action, object, compound)
      engine.rs   # 감정 평가 조정자 (AppraisalEngine)
      situation.rs # 상황 컨텍스트 정의
      stimulus.rs  # PAD 자극 처리 엔진
    relationship.rs # NPC 간 관계 (Closeness, Trust, Power)
    pad.rs        # 감정 공간(PAD) 매핑 및 분석
    guide/        # LLM 연기 지시문 생성 로직
      enums.rs    # Tone, Attitude, BehavioralTendency 등
      directive.rs # ActingDirective (감정+성격 → 연기 지시)
      snapshot.rs  # PersonalitySnapshot, EmotionSnapshot, RelationshipSnapshot
  ports.rs        # 포트 트레이트 (NpcWorld, EmotionStore, SceneStore 등)
  adapter/        # 포트의 구체적 구현 (ORT Embedder 등)
  presentation/   # 다국어 지원 (Locales) 및 포맷팅
  bin/webui/      # 실험용 Web UI (Axum 서버)
tests/            # TestContext 기반 통합/유닛 테스트 (245건)
```

## 3. 핵심 도메인 규칙 및 공식

### 감정 평가 (Appraisal) 원칙
상황이 주어지면 성격 모델이 상황의 각 요소(사건, 행동, 대상)를 주관적으로 해석합니다. 로직은 `appraisal/` 서브 모듈로 물리적으로 분리되어 관리됩니다.
- **성격 가중치:** `1.0 + (Score * 0.3)` 패턴을 기본으로 감정을 증폭/억제합니다.
- **데이터 변환:** DTO 계층에서 `to_domain` 패턴을 사용하여 입력을 도메인 모델로 변환하며, 이때 저장소(`MindRepository`)를 참조합니다.

### 복합 감정 (Compound Emotions)
기초 감정들이 결합하여 더 고차원적인 감정을 생성합니다.
- **Gratification:** Pride + Joy / **Remorse:** Shame + Distress
- **Gratitude:** Admiration + Joy / **Anger:** Reproach + Distress

### 관계에 의한 변조 (Relationship Modifiers)
- **친밀도(Closeness):** 타인의 감정에 대한 공감/적대적 반응 강도 및 타인 행동 평가의 기본 배율을 결정합니다.
- **신뢰도(Trust):** 타인의 행동(Admiration/Reproach) 평가 시 가중치로 작용합니다.

## 4. 개발 컨벤션

### 에러 처리 (Standardized)
- **도메인 에러:** `MindServiceError`를 통해 비즈니스 로직의 실패를 정의합니다.
- **웹 에러:** `AppError`와 `IntoResponse`를 사용하여 서비스 에러를 자동으로 적절한 HTTP 상태 코드와 JSON 메시지로 변환합니다.

### 테스트 인프라 (TestContext)
모든 테스트는 `tests/common/mod.rs`의 `TestContext`를 사용해야 합니다.
- **Fixture:** 무백, 교룡 등의 기본 캐릭터가 미리 셋업된 환경을 제공합니다.
- **Mocking:** `MockRepository`를 통해 인메모리에서 모든 상태 변화를 검증합니다.

## 5. 실행 및 테스트 명령

### 주요 실행 명령
```bash
# Web UI 실행 (http://127.0.0.1:3000)
cargo run --features webui --bin npc-webui

# Web UI + 대사→PAD 자동 분석 (embed 포함)
cargo run --features webui,embed --bin npc-webui

# 모든 테스트 실행 (245건)
cargo test
```

### 주요 테스트 파일
- `tests/application_test.rs`: MindService API 흐름 및 DTO 변환 검증 (5건)
- `tests/emotion_test.rs`: OCC 감정 평가 + 전망확인 + merge + trigger (52건)
- `tests/relationship_test.rs`: 관계 3축 모델 및 변동 + significance 배율 (29건)
- `tests/directive_test.rs`: ActingDirective Tone/Attitude/Behavior/Restriction 전 분기 (32건)
- `tests/coverage_gap_test.rs`: valence, merge 경계값, PAD 좌표, 수식 정밀 검증 (23건)
- `tests/guide_test.rs`: LLM 연기 가이드 생성 + PowerLevel (15건)
- `tests/pad_test.rs`: PAD 공간 분석 (24건)
- `tests/stimulus_test.rs`: 대사 자극 감정 변동 + 관성 (10건)
- `tests/dialogue_flow_test.rs`: 대화 흐름 통합 테스트 (7건)
- `tests/locale_test.rs`: 언어 설정 + 플러거블 포맷터 (20건)
- `tests/port_injection_test.rs`: 포트 주입 + Scene/Beat 통합 (14건)
- `tests/personality_test.rs`: HEXACO 성격 모델 (14건)

## 6. WebUI 주요 기능
- NPC/관계/오브젝트 CRUD 및 OCC 감정 평가
- **대사 기반 PAD 자극 분석**: 상대 대사 입력 → `PadAnalyzer`(BGE-M3)로 PAD 자동 추출 → 슬라이더 반영 (`embed` feature 필요, 없으면 수동 입력 fallback)
- 시나리오 로드/세이브 및 현재 시나리오명 헤더 표시
- LLM 연기 가이드 생성/재생성 (상황 컨텍스트 자동 반영)

## 7. 특이 사항
- **라이브러리 지향:** 이 프로젝트는 라이브러리 형태로 배포되는 것을 목표로 하며, `MindService`가 유일한 진입점 역할을 합니다.
- **Windows 환경:** 임베딩 기능 사용 시 `ort` 라이브러리의 정적 링크 설정을 확인해야 합니다.

---
**주의:** `GEMINI.md`는 프로젝트의 헌법과 같습니다. 새로운 기능을 추가하거나 리팩토링할 때 이 문서의 원칙을 위반하지 않도록 주의하십시오.
