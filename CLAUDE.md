# CLAUDE.md

NPC Mind Engine — LLM 기반 NPC 심리/대사 생성을 위한 Rust 라이브러리.

## 빌드 & 테스트

```bash
cargo build            # 빌드
cargo build --release  # 릴리스 빌드
cargo test             # 전체 테스트 실행 (39개)
cargo test --test personality_test  # 성격 모델 테스트만
cargo test --test emotion_test      # 감정 엔진 테스트만
cargo test --test guide_test        # 가이드 생성 테스트만
```

## 프로젝트 구조

```
src/
  lib.rs                  # 루트 모듈
  domain/
    mod.rs                # 모듈 선언
    personality.rs        # HEXACO 성격 모델 (6차원 24개 facet)
    emotion.rs            # OCC 감정 엔진 (22가지 감정, 상황 평가)
    guide.rs              # LLM 연기 가이드 생성 (프롬프트/JSON 출력)
tests/
  personality_test.rs     # 성격 모델 테스트
  emotion_test.rs         # 감정 엔진 테스트
  guide_test.rs           # 가이드 생성 테스트
docs/                     # 설계 문서 (한국어)
```

## 핵심 파이프라인

HEXACO 성격 → OCC 감정 평가 → LLM 연기 가이드 생성

같은 상황이라도 NPC 성격에 따라 다른 감정 → 다른 연기 지시가 생성됨.

## 코드 컨벤션

- **언어**: 코드 주석, 도메인 용어, 문서 모두 한국어
- **에러 처리**: `thiserror` 사용, fallible 함수는 `Result<T, E>` 반환
- **네이밍**: PascalCase (타입/열거형), snake_case (함수/변수), 차원 약어(h, e, x, a, c, o)
- **패턴**: Builder (NpcBuilder), Value Object (Score), DDD (Aggregate Root)
- **직렬화**: 모든 도메인 타입에 `Serialize`/`Deserialize` 구현
- **Score 범위**: -1.0 ~ 1.0 (경계값 검증 필수)
- **unsafe 코드 사용 금지**

## 의존성

- `serde` + `serde_json` — 직렬화
- `thiserror` — 에러 타입 정의
- `approx` (dev) — 부동소수점 비교 테스트
