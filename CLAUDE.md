# CLAUDE.md

NPC Mind Engine — HEXACO 성격이 OCC 감정을 생성하고, LLM이 연기할 수 있도록 가이드를 출력하는 Rust 라이브러리.

## 빌드 & 테스트

```bash
cargo build                        # 기본 빌드 (임베딩 제외)
cargo build --features embed       # 임베딩 포함 빌드 (bge-m3-onnx-rust)
cargo test                         # 기본 테스트 (약 80개)
cargo test --features embed        # 전체 테스트 (임베딩 포함)

# 개별 테스트
cargo test --test application_test    # Application Service API (3개)
cargo test --test emotion_test        # OCC 감정 평가 시나리오 (35개)
cargo test --test relationship_test   # 관계 3축 모델 및 변동 (26개)
cargo test --test personality_test    # HEXACO 성격 모델 (14개)
cargo test --test guide_test          # LLM 연기 가이드 생성 (10개)
cargo test --test pad_test            # PAD 공간 분석 (24개)
cargo test --test stimulus_test       # 대사 자극 감정 변동 (8개)
cargo test --test dialogue_flow_test  # 대화 흐름 통합 테스트 (7개)

# webui 빌드 & 실행
cargo run --features webui --bin npc-webui          # http://127.0.0.1:3000
cargo run --features webui,embed --bin npc-webui    # 대사→PAD 분석 포함
```

### 빌드 주의사항 (Windows)

`--features embed` 사용 시 ort(ONNX Runtime) 정적 링크를 위해
`.cargo/config.toml`에서 CRT를 동적으로 통일해야 함. 변경 후에는 `cargo clean` 필수.

## 프로젝트 구조

```
src/
  application/                    # [New] 어플리케이션 계층 (라이브러리 진입점)
    mind_service.rs               # 핵심 오케스트레이션 (appraise, stimulus 등)
    dto.rs                        # API 데이터 전송 객체 및 도메인 변환 로직
  domain/
    personality.rs                # HEXACO 성격 모델
    relationship.rs               # 관계 모델 (closeness, trust, power)
    pad.rs                        # PAD 감정 공간 분석
    emotion/
      appraisal/                  # [Refactored] 세부 평가 모듈 (event, action, object, compound)
      engine.rs                   # AppraisalEngine (세부 모듈 조정자)
      types.rs                    # OCC 감정 타입 및 EmotionState
      situation.rs                # 상황 컨텍스트 모델
      stimulus.rs                 # PAD 자극 처리 엔진
    guide/                        # LLM 연기 가이드 생성 로직
  ports.rs                        # 포트 트레이트 (헥사고날 확장 포인트)
  adapter/                        # 포트 구현 (ORT Embedder 등)
  presentation/                   # 다국어 지원 및 텍스트 포맷팅
  bin/webui/                      # 실험용 Web UI (Axum 서버)
tests/
  common/mod.rs                   # [Refactored] TestContext, MockRepository, Fixtures
  application_test.rs             # MindService 및 DTO 변환 테스트
  emotion_test.rs                 # OCC 감정 평가 통합 테스트 (35건)
  relationship_test.rs            # 관계 모델 테스트 (26건)
```

## 아키텍처 (DDD + 헥사고날 + 계층화)

### 계층 구조 (Layering)
1.  **Domain**: 순수 비즈니스 로직 (`src/domain`). 외부 의존성 없음.
2.  **Application**: 도메인 객체 조립 및 흐름 제어 (`src/application`). 라이브러리 사용자의 주요 진입점.
3.  **Infrastructure/Presentation**: 외부 라이브러리 구현 및 API 노출 (`src/adapter`, `src/bin/webui`).

### 핵심 진입점: `MindService`
라이브러리 사용자는 `MindRepository` 포트를 구현하여 `MindService`를 생성하고 사용합니다.
- `appraise()`: 초기 상황 판단 및 감정 생성
- `apply_stimulus()`: 대화 중 실시간 감정 변화 처리
- `after_dialogue()`: 대화 종료 후 관계 갱신

### 감정 평가 (Appraisal) 모듈화
`AppraisalEngine`은 물리적으로 분리된 세부 모듈을 호출하여 감정을 생성합니다.
- `event`: 사건의 바람직함 평가 (Joy, Distress, Hope, Fear 등)
- `action`: 행위의 정당성 평가 (Pride, Admiration, Anger 등)
- `object`: 대상의 매력도 평가 (Love, Hate)
- `compound`: 기초 감정 결합 (Gratitude, Remorse 등)

## 개발 컨벤션

### 에러 처리 및 응답
- 서비스 계층은 `MindServiceError`를 반환합니다.
- 웹 계층(`webui`)은 `AppError`를 통해 서비스 에러를 적절한 HTTP 상태 코드와 JSON으로 자동 변환(`IntoResponse`)합니다.

### 데이터 변환 (Mapping)
- DTO(`SituationInput` 등)는 `to_domain()` 메서드를 통해 도메인 모델로 변환됩니다. 이 과정에서 필요한 관계 조회 등을 위해 `MindRepository`를 참조합니다.

### 테스트 원칙 (TestContext)
- 모든 통합 테스트는 `tests/common/mod.rs`의 `TestContext`를 사용합니다.
- 캐릭터 생성이나 저장소 초기화 등의 중복 코드를 방지하고, 일관된 테스트 환경을 보장합니다.

## WebUI (axum 기반 협업 도구)

Claude(API)와 Bekay(브라우저)가 동시에 사용하는 심리 엔진 시뮬레이터입니다.
- 서버: `cargo run --features webui --bin npc-webui` → http://127.0.0.1:3000
- 임베딩 포함: `cargo run --features webui,embed --bin npc-webui` (대사→PAD 자동 분석 활성화)
- 턴 히스토리: 각 API 호출 결과가 `TurnRecord`로 기록되어 시각화됩니다.

### WebUI 주요 기능
- NPC/관계/오브젝트 CRUD
- OCC 감정 평가 (appraise) 및 LLM 연기 가이드 생성
- **대사 기반 PAD 자극 분석**: 상대 대사 입력 → PadAnalyzer(BGE-M3)로 PAD 자동 추출 → 슬라이더 반영 (embed feature 필요, 없으면 수동 입력)
- 시나리오 로드/세이브 및 현재 시나리오명 헤더 표시
- 턴별 히스토리, Appraisal Trace 로그
