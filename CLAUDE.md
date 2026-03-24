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
  lib.rs                    # 루트 모듈 (domain, ports, presentation 선언)
  domain/
    mod.rs                  # 모듈 선언
    personality.rs          # HEXACO 성격 모델 (6차원 24개 facet)
    emotion.rs              # OCC 감정 엔진 (22가지 감정, 상황 평가)
    guide.rs                # LLM 연기 가이드 (enum 기반 구조화된 데이터)
  ports.rs                  # 포트 트레이트 (Appraiser, GuideFormatter)
  presentation/
    mod.rs                  # 프레젠테이션 모듈
    locale.rs               # 로케일 번들 (TOML 로딩, VariantName 트레이트)
    formatter.rs            # LocaleFormatter (언어 무관 포맷터)
    korean.rs               # 한국어 포맷터 (KoreanFormatter — ko.toml 내장 래퍼)
locales/
  ko.toml                   # 한국어 로케일 (감정/어조/태도/템플릿 등)
  en.toml                   # 영어 로케일
tests/
  personality_test.rs       # 성격 모델 테스트
  emotion_test.rs           # 감정 엔진 테스트
  guide_test.rs             # 가이드 생성 + 포맷터 테스트
docs/                       # 설계 문서 (한국어)
```

## 아키텍처 (DDD + 헥사고날)

```
domain/          순수 도메인 핵심 (Value Object, Entity, Domain Service)
ports.rs         포트 트레이트 (확장 포인트)
presentation/    어댑터 (한국어 포맷터 등)
```

### 핵심 파이프라인

HEXACO 성격 → OCC 감정 평가 → LLM 연기 가이드 생성

같은 상황이라도 NPC 성격에 따라 다른 감정 → 다른 연기 지시가 생성됨.

### 포트 트레이트

- `Appraiser` — 감정 평가 추상화 (다른 심리 모델 교체 가능)
- `GuideFormatter` — 가이드 포맷 추상화 (다국어/다른 LLM 포맷 지원)

### 도메인 enum 타입

guide.rs의 연기 지시는 문자열이 아닌 enum으로 타입화:
- `Tone` (18종), `Attitude` (7종), `BehavioralTendency` (8종), `Restriction` (5종)
- `PersonalityTrait` (12종), `SpeechStyle` (12종)

텍스트 변환은 TOML 로케일 파일 + `LocaleFormatter`가 담당.
`KoreanFormatter`는 `ko.toml`을 내장한 편의 래퍼.

### 다국어 지원 (TOML 로케일)

`locales/` 디렉토리에 언어별 TOML 파일을 두고, `LocaleFormatter`로 로드:
- `locales/ko.toml` — 한국어 (기본)
- `locales/en.toml` — 영어
- 새 언어 추가: TOML 파일만 작성하면 코드 변경 없이 지원

## 코드 컨벤션

- **언어**: 코드 주석, 도메인 용어, 문서 모두 한국어
- **에러 처리**: `thiserror` 사용, fallible 함수는 `Result<T, E>` 반환
- **네이밍**: PascalCase (타입/열거형), snake_case (함수/변수), 차원 약어(h, e, x, a, c, o)
- **패턴**: Builder (NpcBuilder), Value Object (Score), DDD (Aggregate Root)
- **캡슐화**: Entity(Npc), Value Object(Emotion, EmotionState)는 private 필드 + getter
- **직렬화**: 모든 도메인 타입에 `Serialize`/`Deserialize` 구현
- **Score 범위**: -1.0 ~ 1.0 (경계값 검증 필수)
- **unsafe 코드 사용 금지**

## 의존성

- `serde` + `serde_json` — 직렬화
- `thiserror` — 에러 타입 정의
- `toml` — TOML 로케일 파일 파싱
- `approx` (dev) — 부동소수점 비교 테스트
