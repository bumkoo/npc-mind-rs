# GEMINI.md - NPC Mind Engine 개발 가이드

이 파일은 `npc-mind-rs` 프로젝트의 구조, 도메인 로직, 개발 컨벤션 및 실행 방법을 정의합니다. 모든 AI 에이전트와 개발자는 이 가이드를 최우선으로 준수해야 합니다.

## 1. 프로젝트 개요
`npc-mind-rs`는 NPC의 **성격(HEXACO)**과 **관계(Relationship)**를 바탕으로 **상황(Situation)**에 따른 **감정(OCC)**을 생성하고, 이를 LLM이 연기할 수 있는 **지시문(Acting Guide)**으로 변환하는 Rust 기반 심리 엔진입니다.

### 핵심 기술 스택
- **Language:** Rust (Edition 2021)
- **Architecture:** Hexagonal Architecture (Ports and Adapters) + DDD
- **Models:** HEXACO (Personality), OCC (Emotion), PAD (Emotional Space)
- **Libraries:** `serde` (Serialization), `thiserror` (Error handling), `toml` (Locales), `ort` (ONNX Runtime for Embeddings)

## 2. 프로젝트 구조
```
src/
  domain/         # 핵심 도메인 로직 (순수 함수 및 상태 관리)
    personality.rs # HEXACO 모델, Score VO
    emotion/      # OCC 감정 엔진, Appraisal, Stimulus 로직
    relationship.rs # NPC 간 관계 (Closeness, Trust, Power)
    pad.rs        # 감정 공간 매핑 및 분석
    guide/        # LLM 연기 지시문 생성 로직
  ports.rs        # 외부 인터페이스 정의 (Repository, Embedder 등)
  adapter/        # 포트의 구체적 구현 (ONNX Embedder 등)
  presentation/   # 다국어 지원 및 텍스트 포맷팅
tests/            # 각 도메인별 상세 단위 및 통합 테스트
docs/             # 도메인 연구 및 설계 문서
```

## 3. 핵심 도메인 규칙 및 공식

### 성격 가중치 패턴 (Personalization)
감정 강도는 성격 점수(`Score`: -1.0 ~ 1.0)에 의해 변조됩니다. 범용 가중치 계수(`W`)는 **0.3**을 사용합니다.
- **증폭:** `1.0 + (Score * W)` (예: 성격이 강할수록 감정 강화)
- **억제:** `1.0 - (max(0, Score) * W)` (예: 인내심이 높을수록 분노 억제)

### 감정 상태 관리 (EmotionState)
- **구조:** 성능 최적화를 위해 22종의 OCC 감정을 `[f32; 22]` 고정 크기 배열로 관리합니다.
- **합산:** 동일 감정 발생 시 강도를 합산하며, 모든 값은 `0.0 ~ 1.0` 범위로 클램핑됩니다.

### 자극 처리 (Stimulus)
- **IMPACT_RATE (0.1):** 한 턴의 대사가 감정에 미치는 최대 영향력.
- **FADE_THRESHOLD (0.05):** 감정 강도가 이 수치 미만으로 떨어지면 자연 소멸된 것으로 간주합니다.

## 4. 개발 컨벤션

### 네이밍 및 주석
- **언어:** 모든 코드 주석, 도메인 용어, 테스트 이름은 **한글**을 기본으로 합니다.
- **용어:** 
  - `~Type`: 종류 구분 (예: `EmotionType`)
  - `~Level`: 단계/정도 (예: `RelationshipLevel`)
  - `~Engine`/`~Analyzer`: 도메인 서비스

### 에러 처리 및 타입 안전성
- `Score` 타입을 사용하여 -1.0 ~ 1.0 범위를 강제하고, 비교 시에는 `.value()` 또는 `.intensity()`를 사용합니다.
- `thiserror`를 사용하여 도메인별 구체적인 에러 타입을 정의합니다.

### 헥사고날 원칙
- `domain/` 내부 코드는 외부 환경(I/O, DB, 외부 라이브러리)에 의존하지 않는 순수 로직이어야 합니다.
- 외부 의존성이 필요한 경우 `ports.rs`에 트레이트를 정의하고 `adapter/`에서 구현합니다.

## 5. 실행 및 테스트 명령

### 빌드 및 테스트
```bash
# 기본 빌드 및 테스트
cargo build
cargo test

# 임베딩 기능(ONNX) 포함 테스트 (모델 파일 필요)
cargo test --features embed
```

### 개별 도메인 테스트
- 성격 모델: `cargo test --test personality_test`
- 감정 엔진: `cargo test --test emotion_test`
- 연기 가이드: `cargo test --test guide_test`

## 6. 특이 사항 (Windows 환경)
임베딩 기능(`--features embed`) 사용 시 `ort` 라이브러리의 정적 링크를 위해 `.cargo/config.toml` 설정이 필요합니다. CRT 설정 변경 후에는 반드시 `cargo clean`을 수행해야 합니다.

---
**주의:** `GEMINI.md`는 프로젝트의 헌법과 같습니다. 새로운 기능을 추가하거나 리팩토링할 때 이 문서의 원칙을 위반하지 않도록 주의하십시오.
