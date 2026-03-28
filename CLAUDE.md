# CLAUDE.md

NPC Mind Engine — HEXACO 성격이 OCC 감정을 생성하고, LLM이 연기할 수 있도록 가이드를 출력하는 Rust 라이브러리.

## 빌드 & 테스트

```bash
cargo build                        # 기본 빌드 (임베딩 제외)
cargo build --features embed       # 임베딩 포함 빌드 (bge-m3-onnx-rust)
cargo test                         # 기본 테스트 (123개)
cargo test --features embed        # 전체 테스트 (임베딩 포함)

# 개별 테스트
cargo test --test personality_test    # HEXACO 성격 모델 (14개)
cargo test --test emotion_test        # OCC 감정 + Relationship 통합 (35개)
cargo test --test guide_test          # LLM 연기 가이드 (10개)
cargo test --test pad_test            # PAD 공간 + OCC→PAD 매핑 (24개)
cargo test --test relationship_test   # 관계 3축 모델 (25개)
cargo test --test stimulus_test       # 대사 자극 감정 변동 (8개)
cargo test --test dialogue_flow_test  # 대화 흐름 통합 (7개)
cargo test --features embed --test embed_test  # 임베딩 PAD 추출

# webui 빌드 & 실행
cargo run --features webui --bin npc-webui   # http://127.0.0.1:3000
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
                                  # + impl AppraisalWeights, impl StimulusWeights
    relationship.rs               # 관계 모델 (closeness, trust, power 3축)
    pad.rs                        # PAD 감정 공간 + PadAnalyzer 도메인 서비스
    emotion/
      mod.rs
      types.rs                    # OCC 22개 감정 유형, Emotion, EmotionState
      situation.rs                # 상황 모델 (Event/Action/Object 3개 Option 필드)
      engine.rs                   # AppraisalEngine (상수 0개, trait만 의존, tracing)
      stimulus.rs                 # StimulusEngine (StimulusWeights trait 의존, tracing)
    guide/
      mod.rs                      # ActingGuide (최종 산출물)
      enums.rs                    # Tone, Attitude, BehavioralTendency 등
      snapshot.rs                 # PersonalitySnapshot, EmotionSnapshot
      directive.rs                # ActingDirective (감정+성격→연기 지시)
  adapter/
    mod.rs                        # feature gate (embed → ort_embedder)
    ort_embedder.rs               # bge-m3-onnx-rust TextEmbedder 어댑터
  presentation/
    mod.rs
    locale.rs                     # LocaleBundle (TOML 로딩, VariantName)
    formatter.rs                  # LocaleFormatter (언어 무관 포맷터)
    korean.rs                     # KoreanFormatter (ko.toml 내장 래퍼)
  bin/webui/
    main.rs                       # axum 서버 진입점 + tracing subscriber 초기화
    handlers.rs                   # API 핸들러 (CRUD + 파이프라인 + 저장/로드)
    state.rs                      # AppState, NpcProfile, RelationshipData, TurnRecord
    trace_collector.rs            # AppraisalCollector (tracing Layer 구현)
    static/index.html             # React CDN 기반 SPA (프론트엔드)
locales/
  ko.toml                         # 한국어 로케일
  en.toml                         # 영어 로케일
tests/
  common/mod.rs                   # 4인 캐릭터 빌더 (무백, 교룡, 수련, 소호)
  personality_test.rs             # HEXACO 성격 모델 (14개)
  emotion_test.rs                 # 감정 + Relationship 통합 (35개)
  guide_test.rs                   # 연기 가이드 + 관계 포함 (10개)
  pad_test.rs                     # PAD 구조체, 내적, OCC→PAD 매핑 (24개)
  relationship_test.rs            # 관계 3축 기본 + 갱신 (25개)
  stimulus_test.rs                # apply_stimulus 감정 변동 (8개)
  dialogue_flow_test.rs           # 전체 대화 흐름 통합 (7개)
  embed_test.rs                   # 임베딩 PAD 추출 (--features embed)
docs/                              # 설계 문서 (한국어)
data/
  presets/                         # 4인 프리셋 JSON (무백, 교룡, 수련, 소호)
  {도서명}/                        # 테스트 시나리오 폴더 구조
    {장면명}/
      session_{NNN}/
        scenario.json              # NPC + 관계 + turn_history (서버 상태 스냅샷)
        test_report.md             # 테스트 레포트
        evaluation.md              # 평가 노트
        turn{N}_{label}.txt        # 턴별 프롬프트 출력
```

## 아키텍처 (DDD + 헥사고날 + 포트 앤드 어댑터)

### 4레이어 감정 파이프라인

```
레이어1: Situation    세계관 객관 기준 (고정)
레이어2: HEXACO       성격 6차원×4facet → AppraisalWeights/StimulusWeights trait
레이어3: Relationship 상대별 관계 3축 (대화 중 고정, 대화 후 갱신)
레이어4: PAD 자극     대사의 감정적 자극 (매 턴 변동)
```

### 핵심 설계: 엔진-성격 분리 (AppraisalWeights / StimulusWeights)

엔진은 성격 모델(HEXACO)의 내부를 모른다. trait을 통해 "가중치"만 받아 사용한다.

```
AppraisalEngine                    HexacoProfile
─────────────────                  ──────────────────
let w = p.desirability_self_weight(d);   base + E×0.3 + X×0.3  ← 내부 계산
Joy = d × w                             엔진은 w만 받음 (rel_mul 없음)
```

**AppraisalWeights trait (ports.rs)** — 7개 메서드:

| 메서드 | 발동 감정 | HEXACO 관여 facet | clamp |
|--------|-----------|-------------------|-------|
| `desirability_self_weight(d)` | Joy, Distress | d>0: E+X, d<0: E-A-Pru | 0.5~1.5 |
| `desirability_prospect_weight(d)` | Hope, Fear | d>0: E+X-Pru, d<0: E+Fear | 0.5~1.5 |
| `desirability_confirmation_weight(d)` | Satisfaction 등 4종 | E-Pru | 0.5~1.5 |
| `empathy_weight(d)` | HappyFor, Pity | d>0: H+A, d<0: A+Sent | 0.0~1.5 |
| `hostility_weight(d)` | Resentment, Gloating | d>0: -H, d<0: -H-A | 0.0~1.5 |
| `praiseworthiness_weight(is_self, pw)` | Pride/Shame/Admiration/Reproach | C ± Mod/Gen | 0.5~1.5 |
| `appealingness_weight(ap)` | Love, Hate | Aes | 0.5~1.5 |

**StimulusWeights trait (ports.rs)** — 1개 메서드:

| 메서드 | 역할 | HEXACO 관여 facet | clamp |
|--------|------|-------------------|-------|
| `stimulus_absorb_rate(stimulus)` | 자극 수용도 | E-Pru-patience(부정시) | 0.1~2.0 |

**empathy/hostility 분리**: 0이면 미발동, >0이면 강도. 같은 상황에서 공감과 적대가 동시에 발생할 수 있음.
- empathy base=0.5 (타인의 운은 자기 감정보다 약함)
- hostility base=0.0 (기본 미발동, 성격에 의해서만 발동)

### Relationship 캡슐화

엔진은 Relationship의 내부(closeness/trust/power Score)를 모른다. 메서드만 호출한다.

| 메서드 | 공식 | 용도 |
|--------|------|------|
| `emotion_intensity_multiplier()` | `(1.0 + closeness × 0.5).max(0.0)` | 전체 감정 배율 (선형, 적대→절제) |
| `trust_emotion_modifier()` | `1.0 + trust × 0.3` | Action 감정 배율 |
| `empathy_rel_modifier()` | `(1.0 + closeness × 0.3).max(0.0)` | Fortune-of-others 공감 배율 |
| `hostility_rel_modifier()` | `(1.0 - closeness × 0.3).max(0.0)` | Fortune-of-others 적대 배율 |
| `after_dialogue(state, Option<f32>)` | trust+closeness 점진 갱신 | Situation을 모름 (pw만 받음) |

### 엔진 의존성 현황 (상수 0개)

| engine.rs가 아는 것 | engine.rs가 모르는 것 |
|---|---|
| `AppraisalWeights` trait (7개 메서드) | HexacoProfile, Score, facet |
| `Relationship` 4개 메서드 | closeness/trust/power Score 값 |
| Situation, EmotionState | 성격 차원 평균, modifier 계산법 |

| stimulus.rs가 아는 것 | stimulus.rs가 모르는 것 |
|---|---|
| `StimulusWeights` trait (1개 메서드) | HexacoProfile, Score, facet |

| ports.rs | 상태 |
|---|---|
| `use HexacoProfile` | **제거됨** — trait만 정의 |

### 핵심 데이터 흐름

```
상황 진입:
  RelationshipRepository.find() → Relationship 로드
  AppraisalEngine.appraise<P: AppraisalWeights>(personality, situation, relationship)
    → EmotionState + trace!() 이벤트 방출
  ActingGuide 생성 → LLM → NPC 첫 대사

대화 중 (매 턴):
  TextEmbedder.embed(대사) → Vec<f32>
  PadAnalyzer.to_pad(벡터) → PAD
  StimulusEngine.apply_stimulus<P: StimulusWeights>(personality, state, PAD)
    → 갱신된 EmotionState + trace!() 이벤트 방출
  ActingGuide 생성 → LLM → NPC 응답

대화 종료 후:
  let pw = situation.action.as_ref().map(|a| a.praiseworthiness);
  Relationship.after_dialogue(final_state, pw)
  RelationshipRepository.save() → 관계 영속화
```

### 포트 트레이트 (ports.rs)

| 포트 | 레이어 | 역할 |
|------|--------|------|
| `AppraisalWeights` | 도메인 | 성격 → 감정 가중치 (7개 메서드) |
| `StimulusWeights` | 도메인 | 성격 → 자극 수용도 (1개 메서드) |
| `Appraiser` | 도메인 | 성격+상황+관계 → 감정 생성 (1회) |
| `StimulusProcessor` | 도메인 | PAD 자극 → 감정 변동 (매 턴) |
| `TextEmbedder` | 인프라 | 텍스트 → 벡터 변환 (임베딩 모델) |
| `UtteranceAnalyzer` | 도메인 | 대사 → PAD 변환 (앵커 비교) |
| `RelationshipRepository` | 인프라 | 관계 저장/로드 (어댑터 미구현) |
| `GuideFormatter` | 프레젠테이션 | 가이드 → 텍스트/JSON 변환 |

### tracing (구조화된 trace 이벤트)

도메인(engine.rs, stimulus.rs)이 `trace!()` 매크로로 구조화된 이벤트를 방출한다.
subscriber가 없으면 no-op (비용 0). 도메인은 subscriber/Layer를 모른다.

```rust
// 도메인 (engine.rs) — trace!() 한 줄만
trace!(emotion = ?EmotionType::Joy, base_val = d, weight = w, result = val, context = %ctx);

// Action 감정 (rel_mul + trust_mod 포함)
trace!(emotion = ?EmotionType::Admiration, base_val = pw, weight = w,
       rel_mul = rel_mul, trust_mod = trust_mod, result = val, context = %ctx);

// Compound 감정
trace!(emotion = ?EmotionType::Gratification, comp1_type = ?EmotionType::Pride,
       comp1_val = pride, comp2_type = ?EmotionType::Joy, comp2_val = joy, result = val);

// 호출부 (game, pipeline) — span으로 컨텍스트 추가
let _span = info_span!("appraisal_tick", agent_id = npc.id().0.as_str()).entered();
```

**trace 키 규칙**: 기본 감정은 `emotion, base_val, weight, result, context` 통일.
Action 감정은 추가로 `rel_mul, trust_mod`. Compound는 `comp1_type, comp1_val, comp2_type, comp2_val`.
Fortune-of-others는 `multiplier`(empathy/hostility_rel_modifier).

**Layer 구현**: `src/bin/webui/trace_collector.rs`의 `AppraisalCollector`.
webui, 라이브러리 사용자 모두 자기 subscriber를 연결 가능.

### 도메인 상수

```rust
StimulusEngine:
  IMPACT_RATE = 0.1           // 한 턴 감정 변동량 제한
  FADE_THRESHOLD = 0.05       // 감정 자연 소멸 기준

Relationship:
  TRUST_UPDATE_RATE = 0.1     // 대화 후 trust 갱신 속도
  CLOSENESS_UPDATE_RATE = 0.05 // 대화 후 closeness 갱신 속도
```

AppraisalEngine에는 상수가 없다. 모든 가중치는 AppraisalWeights trait이 반환한다.

### Score 타입 (Value Object)

`-1.0 ~ 1.0` 범위. 남은 메서드 6개:

| 메서드 | 용도 |
|--------|------|
| `new(value, field)` | 범위 검증 생성 |
| `clamped(value)` | 범위 클램핑 생성 |
| `neutral()` | 0.0 생성 |
| `value()` | 핵심 접근자 |
| `intensity()` | 절대값 (중립 거리 체크용) |
| `modifier(weight)` | `(1.0 + value × weight).max(0.0)` — 유일한 modifier |

삭제된 것: `abs_modifier`, `pos_modifier`, `neg_modifier`, `amplify`, `is_high`, `is_low`, `is_positive`, `is_negative`, `distance`.

### Situation 구조

```rust
pub struct Situation {
    pub description: String,          // 전체 상황 설명 (Compound 감정의 context)
    pub event: Option<EventFocus>,
    pub action: Option<ActionFocus>,
    pub object: Option<ObjectFocus>,
}

pub struct EventFocus {
    pub description: String,          // "문파 추방 위기" → Joy/Distress 등의 context
    pub desirability_for_self: f32,
    pub desirability_for_other: Option<DesirabilityForOther>,  // 제3자 포함
    pub prospect: Option<Prospect>,
}

pub struct ActionFocus {
    pub description: String,          // "비밀 누설" → Pride/Shame/Admiration/Reproach의 context
    pub agent_id: Option<String>,     // None=자기, Some(id)=타인
    pub praiseworthiness: f32,
    pub relationship: Option<Relationship>,  // 제3자면 관계 포함, 대화 상대면 None
}

pub struct ObjectFocus {
    pub target_id: String,            // 게임 시스템 참조용
    pub target_description: String,   // "천잠사 검" → Love/Hate의 context
    pub appealingness: f32,
}
```

`Situation::new()` 스마트 생성자: 최소 1개 Focus 필수 (`SituationError::NoFocus`).
3개가 동시에 존재할 수 있음 (Event+Action → Compound 자동 생성).

### Action 3분기

| agent_id | relationship | 의미 | rel_mul/trust_mod 출처 |
|---|---|---|---|
| `None` | `_` | 자기 → Pride/Shame | 없음 |
| `Some(_)` | `None` | 대화 상대 → Admiration/Reproach | appraise 파라미터 |
| `Some(_)` | `Some(rel)` | 제3자 → Admiration/Reproach | 제3자 relationship |

### rel_mul 적용 범위

rel_mul(emotion_intensity_multiplier)은 **Admiration/Reproach에만 적용**. 나머지 감정은 rel_mul 없음.

| 감정 | 공식 | rel_mul |
|---|---|---|
| Joy/Distress, Hope/Fear, 확인4종 | `d × personality_weight` | ❌ 없음 |
| HappyFor/Pity | `d × empathy_weight × empathy_rel_mod` | 자체 modifier |
| Resentment/Gloating | `d × hostility_weight × hostility_rel_mod` | 자체 modifier |
| Pride/Shame | `pw × personality_weight` | ❌ 없음 |
| Admiration/Reproach | `pw × weight × trust_mod × rel_mul` | ✅ 유일하게 적용 |
| Love/Hate | `ap × personality_weight` | ❌ 없음 |
| Compound | 기초 감정 결합 | 간접 반영 |

### Emotion context (감정 원인 추적)

모든 감정에 `context: Option<String>`이 부착됨. 엔진이 감정 생성 시 Focus의 description을 복사.

```rust
pub struct Emotion {
    emotion_type: EmotionType,
    intensity: f32,
    context: Option<String>,  // LLM 프롬프트에 포함됨
}
```

| 감정 | context 출처 |
|---|---|
| Joy, Distress, Hope, Fear, 확인4종 | `event.description` |
| HappyFor, Pity, Resentment, Gloating | `"{event.description} (대상: {target_id})"` |
| Pride, Shame, Admiration, Reproach | `action.description` |
| Love, Hate | `object.target_description` |
| Compound (Anger, Gratitude 등) | `situation.description` |

`EmotionState`도 `contexts: [Option<String>; 22]` 배열로 감정별 context를 보존.
`context_of(EmotionType)` 메서드로 조회 가능.

## 코드 컨벤션

- **언어**: 코드 주석, 도메인 용어, 테스트 이름 모두 한국어
- **에러 처리**: `thiserror` 사용, fallible 함수는 `Result<T, E>` 반환
- **네이밍**: PascalCase(타입), snake_case(함수/변수), 차원 약어(h, e, x, a, c, o)
- **캡슐화**: Entity/VO는 private 필드 + getter, pub(super) 내부 변경 메서드
- **직렬화**: 모든 도메인 타입에 `Serialize`/`Deserialize`
- **Score 범위**: -1.0 ~ 1.0 (경계값 검증 필수)
- **가중치 패턴**: AppraisalWeights trait이 가산 모델 (`base + facet×w`) 반환 → 엔진은 곱하기만
- **modifier 패턴**: `Score::modifier(w)` = `(1.0 + value × w).max(0.0)` — 유일한 선형 패턴
- **unsafe 코드 사용 금지**

### DDD 네이밍 룰

| DDD 패턴 | 네이밍 룰 | 현재 예시 |
|----------|----------|----------|
| Entity | 도메인 이름 `Xxx` + `XxxId` | `Npc`, `NpcId` |
| Value Object | 도메인 이름, doc에 `/// Value Object` | `Score`, `Pad`, `Emotion`, `Relationship` |
| Domain Service | `XxxEngine`/`XxxAnalyzer` | `AppraisalEngine`, `StimulusEngine`, `PadAnalyzer` |
| Application Service | `XxxService` | (없음, 향후 `DialogueService`) |
| Port (trait) | 행위/능력 명사, `ports.rs`에 집중 | `AppraisalWeights`, `StimulusWeights`, `Appraiser`, `TextEmbedder` |
| Adapter | 구현기술 + 포트명 | `OrtEmbedder` |
| Domain Event | 과거형 `XxxChanged` | (없음, 향후 `EmotionChanged`) |
| Snapshot / DTO | `XxxSnapshot` | `PersonalitySnapshot`, `EmotionSnapshot` |
| Builder | `XxxBuilder` | `NpcBuilder`, `RelationshipBuilder` |
| Error | 모듈 + `Error` | `PersonalityError`, `EmbedError`, `SituationError` |

**Domain Service vs Application Service**: `~Engine`/`~Analyzer`는 도메인 용어로만 동작하고 인프라를 모름.
`~Service`는 포트들을 조립하고 트랜잭션 흐름을 관리.

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
- `tracing` — 구조화된 trace 이벤트 (subscriber 없으면 no-op)
- `approx` (dev) — 부동소수점 비교 테스트

### webui feature
- `axum` — HTTP 서버 프레임워크
- `tokio` — 비동기 런타임
- `tower-http` — CORS, 정적 파일 서빙
- `tracing-subscriber` — tracing Layer/Subscriber 조합

### embed feature (선택적)
- `bge-m3-onnx-rust` (path = "../bge-m3-onnx-rust") — ort 기반 bge-m3 임베딩
  - 모델: `../models/bge-m3/model_quantized.onnx` (INT8, ~570MB)
  - 토크나이저: `../models/bge-m3/tokenizer.json`

## WebUI (axum 기반 협업 도구)

Claude(API)와 Bekay(브라우저)가 동시에 사용하는 NPC 심리 엔진 협업 도구.
서버: `cargo run --features webui --bin npc-webui` → http://127.0.0.1:3000

### API 엔드포인트

| 엔드포인트 | 메서드 | 용도 |
|---|---|---|
| `/api/npcs` | GET/POST | NPC CRUD |
| `/api/npcs/{id}` | DELETE | NPC 삭제 |
| `/api/relationships` | GET/POST | 관계 CRUD |
| `/api/relationships/{owner}/{target}` | DELETE | 관계 삭제 |
| `/api/objects` | GET/POST | 오브젝트 CRUD |
| `/api/objects/{id}` | DELETE | 오브젝트 삭제 |
| `/api/appraise` | POST | 감정 평가 (상황 → 감정 + 프롬프트) |
| `/api/stimulus` | POST | PAD 자극 적용 → 감정 변동 + 프롬프트 재생성 |
| `/api/guide` | POST | 현재 감정 기준 가이드 재생성 |
| `/api/after-dialogue` | POST | 대화 종료 → 관계 갱신 |
| `/api/scenarios` | GET | data/ 폴더 스캔 → 시나리오 목록 |
| `/api/history` | GET | 턴별 기록 조회 |
| `/api/save` | POST | JSON 파일 저장 (turn_history 포함) |
| `/api/load` | POST | JSON 파일 로드 |

### 턴 히스토리 (TurnRecord)

appraise/stimulus/after_dialogue 호출 시 요청+응답 JSON이 자동 기록됨.
`scenario.json` 저장 시 `turn_history` 필드에 포함되어 로드 시 복원.

```rust
pub struct TurnRecord {
    pub label: String,           // "Turn 1: appraise (jim→huck)"
    pub action: String,          // "appraise" | "stimulus" | "after_dialogue"
    pub request: serde_json::Value,
    pub response: serde_json::Value,
}
```

### 테스트 데이터 폴더 규칙

```
data/{도서명}/{장면명}/session_{NNN}/scenario.json
```
예: `data/huckleberry_finn/ch8_jackson_island_meeting/session_001/scenario.json`

## 설계 문서

- `docs/architecture-v2.md` — 4레이어 아키텍처, 포트 정의, 데이터 흐름
- `docs/hexaco-research.md` — HEXACO 이론, 4인 캐릭터 프로필
- `docs/occ-emotion-model.md` — OCC 22개 감정, 3분기 구조
- `docs/guide-mapping-table.md` — 가이드 생성 매핑 테이블
- `docs/hexaco-occ-emotion-mapping.md` — HEXACO→OCC 감정 매핑 관계
- `docs/pad-stimulus-design-decisions.md` — PAD 자극 설계 결정 근거
- `docs/situation-structure.md` — 상황 모델 구조 설계
- `docs/HEXACO/` — HEXACO 6차원별 상세 가이드
