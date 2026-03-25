# CLAUDE.md

NPC Mind Engine — HEXACO 성격이 OCC 감정을 생성하고, LLM이 연기할 수 있도록 가이드를 출력하는 Rust 라이브러리.

## 빌드 & 테스트

```bash
cargo build                        # 기본 빌드 (임베딩 제외)
cargo build --features embed       # 임베딩 포함 빌드 (bge-m3-onnx-rust)
cargo test                         # 기본 테스트 (96개)
cargo test --features embed        # 전체 테스트 (102개, 임베딩 포함)

# 개별 테스트
cargo test --test personality_test    # HEXACO 성격 모델 (18개)
cargo test --test emotion_test        # OCC 감정 + Relationship 통합 (15개)
cargo test --test guide_test          # LLM 연기 가이드 (10개)
cargo test --test pad_test            # PAD 공간 + OCC→PAD 매핑 (16개)
cargo test --test relationship_test   # 관계 3축 모델 (22개)
cargo test --test stimulus_test       # 대사 자극 감정 변동 (8개)
cargo test --test dialogue_flow_test  # 대화 흐름 통합 (7개)
cargo test --features embed --test embed_test  # 임베딩 PAD 추출 (6개)

# PAD 수치 출력 확인
cargo test --features embed --test embed_test -- --nocapture
```

### 빌드 주의사항 (Windows)

`--features embed` 사용 시 ort(ONNX Runtime) 정적 링크를 위해
`.cargo/config.toml`에서 CRT를 동적으로 통일해야 함:

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=-crt-static"]
[env]
CFLAGS = "/MD"
CXXFLAGS = "/MD"
```

CRT 설정 변경 후에는 `cargo clean` 필수.

## 프로젝트 구조

```
src/
  lib.rs                          # 루트 모듈
  ports.rs                        # 포트 트레이트 (헥사고날 확장 포인트)
  domain/
    mod.rs
    personality.rs                # HEXACO 성격 모델 (6차원 24 facet)
    relationship.rs               # 관계 모델 (closeness, trust, power 3축)
    pad.rs                        # PAD 감정 공간 + PadAnalyzer 도메인 서비스
    emotion/
      mod.rs
      types.rs                    # OCC 22개 감정 유형, Emotion, EmotionState
      situation.rs                # 상황 모델 (Event/Action/Object 3분기)
      engine.rs                   # AppraisalEngine (상수 3개, Relationship 통합)
      stimulus.rs                 # StimulusEngine (PAD 자극 → 감정 변동)
    guide/
      mod.rs                      # ActingGuide (최종 산출물)
      enums.rs                    # Tone, Attitude, BehavioralTendency 등
      snapshot.rs                 # PersonalitySnapshot, EmotionSnapshot, RelationshipSnapshot
      directive.rs                # ActingDirective (감정+성격→연기 지시)
  adapter/
    mod.rs                        # feature gate (embed → ort_embedder)
    ort_embedder.rs               # bge-m3-onnx-rust TextEmbedder 어댑터
  presentation/
    mod.rs
    locale.rs                     # LocaleBundle (TOML 로딩, VariantName)
    formatter.rs                  # LocaleFormatter (언어 무관 포맷터)
    korean.rs                     # KoreanFormatter (ko.toml 내장 래퍼)
locales/
  ko.toml                         # 한국어 로케일
  en.toml                         # 영어 로케일
tests/
  common/mod.rs                   # 4인 캐릭터 빌더 (무백, 교룡, 수련, 소호)
  personality_test.rs             # HEXACO 성격 모델 (18개)
  emotion_test.rs                 # 감정 + Relationship 통합 (15개)
  guide_test.rs                   # 연기 가이드 + 관계 포함 (10개)
  pad_test.rs                     # PAD 구조체, 내적, OCC→PAD 매핑 (16개)
  relationship_test.rs            # 관계 3축 기본 + 갱신 (22개)
  stimulus_test.rs                # apply_stimulus 감정 변동 (8개)
  dialogue_flow_test.rs           # 전체 대화 흐름 통합 (7개)
  embed_test.rs                   # 임베딩 PAD 추출 (6개, --features embed)
docs/                              # 설계 문서 (한국어)
  architecture-v2.md              # 4레이어 아키텍처 설계서
  embedding-adapter-migration.md  # fastembed→ort 교체 보고서
.cargo/
  config.toml                     # CRT 동적 링크 설정 (ort 빌드용)
```

## 아키텍처 (DDD + 헥사고날 + 포트 앤드 어댑터)

### 4레이어 감정 파이프라인

```
레이어1: Situation    세계관 객관 기준 (고정)
레이어2: HEXACO       성격 6차원×4facet (고정)
레이어3: Relationship 상대별 관계 3축 (대화 중 고정, 대화 후 갱신)
레이어4: PAD 자극     대사의 감정적 자극 (매 턴 변동)
```

### 핵심 데이터 흐름

```
상황 진입:
  RelationshipRepository.find() → Relationship 로드
  AppraisalEngine.appraise(personality, situation, relationship) → EmotionState
  ActingGuide 생성 → LLM → NPC 첫 대사

대화 중 (매 턴):
  TextEmbedder.embed(대사) → Vec<f32>
  PadAnalyzer.to_pad(벡터) → PAD
  StimulusEngine.apply_stimulus(personality, state, PAD) → 갱신된 EmotionState
  ActingGuide 생성 → LLM → NPC 응답

대화 종료 후:
  Relationship.update_after_dialogue(final_state, situation)
  RelationshipRepository.save() → 관계 영속화
```

### 포트 트레이트 (ports.rs)

| 포트 | 레이어 | 역할 |
|------|--------|------|
| `Appraiser` | 도메인 | 성격+상황+관계 → 감정 생성 (1회) |
| `StimulusProcessor` | 도메인 | PAD 자극 → 감정 변동 (매 턴) |
| `TextEmbedder` | 인프라 | 텍스트 → 벡터 변환 (임베딩 모델) |
| `UtteranceAnalyzer` | 도메인 | 대사 → PAD 변환 (앵커 비교) |
| `RelationshipRepository` | 인프라 | 관계 저장/로드 (어댑터 미구현) |
| `GuideFormatter` | 프레젠테이션 | 가이드 → 텍스트/JSON 변환 |

### 도메인 핵심 상수

```rust
AppraisalEngine:
  PERSONALITY_WEIGHT = 0.3    // 성격 facet 가중치 (12개→1개 통일)
  EMPATHY_BASE = 0.5          // Fortune-of-others 공감 강도
  FORTUNE_THRESHOLD = -0.2    // H↓/A↓ 판정 임계값

StimulusEngine:
  IMPACT_RATE = 0.1           // 한 턴 감정 변동량 제한
  FADE_THRESHOLD = 0.05       // 감정 자연 소멸 기준

Relationship:
  TRUST_UPDATE_RATE = 0.1     // 대화 후 trust 갱신 속도
  CLOSENESS_UPDATE_RATE = 0.05 // 대화 후 closeness 갱신 속도
```

### 도메인 enum 타입

- `EmotionType` (22종) — OCC 감정 유형
- `Tone` (18종), `Attitude` (7종), `BehavioralTendency` (8종), `Restriction` (5종)
- `PersonalityTrait` (12종), `SpeechStyle` (12종)
- `RelationshipLevel` (5종), `PowerLevel` (3종)

텍스트 변환은 TOML 로케일 파일 + `LocaleFormatter`가 담당.

### 다국어 지원

`locales/` 디렉토리에 언어별 TOML 파일. 새 언어 추가 시 TOML만 작성:
- `ko.toml` — 한국어 (기본, KoreanFormatter로 내장)
- `en.toml` — 영어

## 코드 컨벤션

- **언어**: 코드 주석, 도메인 용어, 테스트 이름 모두 한국어
- **에러 처리**: `thiserror` 사용, fallible 함수는 `Result<T, E>` 반환
- **네이밍**: PascalCase(타입), snake_case(함수/변수), 차원 약어(h, e, x, a, c, o)
- **캡슐화**: Entity/VO는 private 필드 + getter, pub(super) 내부 변경 메서드
- **직렬화**: 모든 도메인 타입에 `Serialize`/`Deserialize`
- **Score 범위**: -1.0 ~ 1.0 (경계값 검증 필수)
- **가중치 패턴**: `1.0 ± facet × PERSONALITY_WEIGHT` 통일
- **unsafe 코드 사용 금지**

### DDD 네이밍 룰

이름만 보고 DDD 역할을 알 수 있어야 한다. Rust 관용어와 충돌하지 않는다.

| DDD 패턴 | 네이밍 룰 | 현재 예시 | 향후 예시 |
|----------|----------|----------|----------|
| Entity | 도메인 이름 그대로 `Xxx` + `XxxId` | `Npc`, `NpcId` | `Player`, `PlayerId` |
| Value Object | 도메인 이름 그대로, doc에 `/// Value Object` | `Score`, `Pad`, `Emotion`, `Relationship` | `DialogueContext` |
| Domain Service | 역할 동사+명사 `XxxEngine`/`XxxAnalyzer`, doc에 `/// 도메인 서비스` | `AppraisalEngine`, `StimulusEngine`, `PadAnalyzer` | `KeywordDetector` |
| Application Service | `XxxService` 접미사 (유스케이스 오케스트레이션) | (없음) | `DialogueService` |
| Port (trait) | 행위/능력 명사, `ports.rs`에 집중, doc에 driving/driven 명시 | `Appraiser`, `TextEmbedder`, `RelationshipRepository` | `NpcRepository`, `EventPublisher` |
| Adapter | 구현기술 + 포트명 `XxxYyy` | `OrtEmbedder` | `InMemoryRelationshipRepo`, `SqliteNpcRepository` |
| Domain Event | 과거형 동사 `XxxChanged`/`XxxOccurred` | (없음) | `EmotionChanged`, `RelationshipUpdated` |
| Snapshot / DTO | `XxxSnapshot` 접미사 (읽기 전용 요약) | `PersonalitySnapshot`, `EmotionSnapshot` | `NpcSnapshot` |
| Builder | `XxxBuilder` 접미사 | `NpcBuilder`, `RelationshipBuilder` | `SituationBuilder` |
| Policy / Specification | `XxxPolicy`/`XxxRule` (비즈니스 규칙 캡슐화) | (없음) | `DialogueRefusalPolicy` |
| Error | 소속 모듈 + `Error` 접미사 | `PersonalityError`, `EmbedError` | `DialogueError` |

**Aggregate Root**: 별도 접미사 없이 Entity와 같은 이름. doc에 `/// Aggregate Root` 명시. 현재 후보: `Npc`.

**Enum 접미사**: "종류"면 `~Type`(`EmotionType`), "정도"면 `~Level`(`RelationshipLevel`, `PowerLevel`), "분류"면 `~Branch`/`~Kind`(`EmotionBranch`).

**Domain Service vs Application Service 구분**:
- Domain Service (`~Engine`/`~Analyzer`): 도메인 용어로만 동작, 인프라를 모름
- Application Service (`~Service`): 포트들을 조립하고 트랜잭션 흐름을 관리

**모듈 위치로 역할 표현**:

```
src/
  domain/         ← Entity, VO, Domain Service, Domain Event
  ports.rs        ← Port (trait 정의만)
  adapter/        ← Adapter (포트 구현)
  application/    ← Application Service (향후)
  presentation/   ← Formatter, Snapshot 렌더링
```

## 의존성

### 기본 (항상)
- `serde` + `serde_json` — 직렬화
- `thiserror` — 에러 타입 정의
- `toml` — TOML 로케일 파일 파싱
- `approx` (dev) — 부동소수점 비교 테스트

### embed feature (선택적)
- `bge-m3-onnx-rust` (path = "../bge-m3-onnx-rust") — ort 기반 bge-m3 임베딩
  - 내부: `ort` v2.0.0-rc.12 + `tokenizers` v0.21
  - 모델: `../models/bge-m3/model_quantized.onnx` (INT8, ~570MB)
  - 토크나이저: `../models/bge-m3/tokenizer.json`

## 외부 모델 파일 (git 미포함)

```
C:\Users\bumko\projects\models\bge-m3\
  ├── model_quantized.onnx    # INT8 양자화 (~570MB, gpahal/bge-m3-onnx-int8)
  └── tokenizer.json          # XLM-RoBERTa 토크나이저
```

## 설계 문서

- `docs/architecture-v2.md` — 4레이어 아키텍처, 포트 정의, 데이터 흐름, 구현 순서
- `docs/embedding-adapter-migration.md` — fastembed→ort 교체 비교 보고서
- `docs/hexaco-research.md` — HEXACO 이론, 4인 캐릭터 프로필
- `docs/occ-emotion-model.md` — OCC 22개 감정, 3분기 구조
- `docs/appraisal-engine.md` — 감정 평가 엔진 초기 설계
