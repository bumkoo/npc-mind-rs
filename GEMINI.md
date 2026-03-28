# GEMINI.md - NPC Mind Engine 개발 가이드

이 파일은 `npc-mind-rs` 프로젝트의 구조, 도메인 로직, 개발 컨벤션 및 실행 방법을 정의합니다. 모든 AI 에이전트와 개발자는 이 가이드를 최우선으로 준수해야 합니다.

## 1. 프로젝트 개요
`npc-mind-rs`는 NPC의 **성격(HEXACO)**과 **관계(Relationship)**를 바탕으로 **상황(Context)**을 해석하여 **감정(OCC)**을 생성하고, 이를 LLM이 연기할 수 있는 **지시문(Acting Guide)**으로 변환하는 Rust 기반 심리 엔진입니다.

### 핵심 기술 스택
- **Language:** Rust (Edition 2021)
- **Architecture:** Hexagonal Architecture (Ports and Adapters) + DDD
- **Models:** HEXACO (Personality), OCC (Emotion), PAD (Emotional Space)
- **Libraries:**
  - `serde`, `serde_json`: 데이터 직렬화 및 JSON API 지원
  - `thiserror`: 도메인 기반 에러 핸들링
  - `axum`, `tokio`: Web UI 및 API 서버 (Async Runtime)
  - `tracing`: 감정 생성 과정의 투명한 추적 (Appraisal Trace)
  - `ort`: ONNX Runtime (텍스트 임베딩/PAD 분석용)

## 2. 프로젝트 구조
```
src/
  domain/         # 핵심 도메인 로직 (순수 함수 및 상태 관리)
    personality.rs # HEXACO 모델, Score VO, AppraisalWeights 구현
    emotion/      # OCC 감정 엔진 (AppraisalEngine), Situation, Stimulus
    relationship.rs # NPC 간 관계 (Closeness, Trust, Power)
    pad.rs        # 감정 공간(PAD) 매핑 및 분석
    guide/        # LLM 연기 지시문 생성 로직 및 스냅샷
  ports.rs        # 외부 인터페이스 정의 (Appraiser, TextEmbedder, Repository 등)
  adapter/        # 포트의 구체적 구현 (ORT Embedder 등)
  presentation/   # 다국어 지원 (TOML locales) 및 텍스트 포맷팅
  bin/webui/      # 실험 및 시각화를 위한 Web UI (Axum 서버 + SPA)
tests/            # 도메인별 단위 테스트 및 시나리오 통합 테스트
data/             # 프리셋 NPC 데이터 및 소설(Huckleberry Finn) 기반 평가 데이터
docs/             # 상세 아키텍처 및 도메인 연구 문서 (v2 포함)
```

## 3. 핵심 도메인 규칙 및 공식

### 감정 평가 (Appraisal) 원칙
상황이 주어지면 성격 모델이 `AppraisalWeights`를 통해 상황의 각 요소(사건, 행동, 대상)를 주관적으로 해석합니다.
- **성격 가중치:** `1.0 + (Score * 0.3)` 패턴을 기본으로 하며, 성격 특질에 따라 특정 감정을 증폭하거나 억제합니다.
- **가중치 범위:** 극단적인 왜곡을 방지하기 위해 최종 가중치는 `0.5 ~ 1.5` 범위로 클램핑됩니다.

### 복합 감정 (Compound Emotions)
기초 감정들이 결합하여 더 고차원적인 감정을 생성합니다.
- **Gratification (만족감):** Pride(자부심) + Joy(기쁨)
- **Remorse (자책감):** Shame(수치심) + Distress(고통)
- **Gratitude (감사):** Admiration(찬사) + Joy(기쁨)
- **Anger (분노):** Reproach(비난) + Distress(고통)

### 관계에 의한 변조 (Relationship Modifiers)
- **공감(Empathy):** 친밀도가 높을수록 타인의 기쁨/고통을 자신의 기쁨/고통으로 강하게 수용합니다.
- **적대(Hostility):** 적대감이 높을수록 타인의 고통에 즐거움(Gloating)을 느끼거나 기쁨에 시기(Resentment)를 느낍니다.
- **신뢰/권력:** 타인의 행동 평가(Admiration/Reproach) 시 상대에 대한 신뢰도와 권력 차이가 강도에 영향을 미칩니다.

### 자극 처리 (Stimulus)
대화 중 발생하는 매 턴의 자극은 PAD 공간으로 변환되어 감정 상태를 실시간으로 변화시킵니다.
- **수용도:** 성격에 따라 외부 자극을 얼마나 민감하게 받아들일지 결정됩니다.
- **임팩트:** 한 턴의 대사가 감정에 미치는 영향은 최대 `0.1`로 제한됩니다.

## 4. 개발 컨벤션

### 네이밍 및 주석
- **언어:** 모든 코드 주석, 도메인 용어, 테스트 이름은 **한글**을 기본으로 합니다.
- **도메인 정합성:** `ports.rs`에 정의된 트레이트와 용어를 엄격히 준수합니다.

### 투명한 디버깅 (Tracing)
감정 생성의 "이유"를 알 수 있도록 `tracing::trace!`를 적극 활용합니다. Web UI에서는 이 로그를 수집하여 사용자에게 "왜 이 감정이 발생했는지"를 시각화합니다.

## 5. 실행 및 테스트 명령

### Web UI 실행 (실험 및 시각화)
```bash
# Web UI 서버 실행 (http://127.0.0.1:3000)
cargo run --features webui
```

### 빌드 및 테스트
```bash
# 기본 빌드
cargo build

# 모든 단위 테스트 실행
cargo test

# 임베딩 기능(ONNX) 포함 테스트 (모델 파일 필요)
cargo test --features embed
```

### 주요 테스트 파일
- `tests/emotion_test.rs`: OCC 감정 생성 로직 검증
- `tests/dialogue_flow_test.rs`: 대화 흐름에 따른 감정/관계 변화 테스트
- `tests/personality_test.rs`: HEXACO 성격 모델 및 가중치 테스트

## 6. 특이 사항 (Windows 환경)
- **임베딩:** `ort` 라이브러리 사용 시 `.cargo/config.toml`에 CRT 설정을 확인해야 합니다.
- **데이터:** NPC 프리셋은 `data/presets/` 하위에 JSON 형식으로 관리됩니다.

---
**주의:** `GEMINI.md`는 프로젝트의 헌법과 같습니다. 새로운 기능을 추가하거나 리팩토링할 때 이 문서의 원칙을 위반하지 않도록 주의하십시오.
