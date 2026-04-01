# CLAUDE.md

NPC Mind Engine — HEXACO 성격이 OCC 감정을 생성하고, LLM이 연기할 수 있도록 가이드를 출력하는 Rust 라이브러리.

라이브러리 형태로 배포되며, `MindService`가 유일한 공개 진입점입니다.

### 기술 스택
- **Language:** Rust (Edition 2024)
- **Architecture:** Hexagonal Architecture (Ports and Adapters) + DDD
- **Libraries:** `serde`/`serde_json`, `thiserror`, `axum`/`tokio`(WebUI), `tracing`(Appraisal Trace), `ort`(ONNX 임베딩)

## 빌드 & 테스트

```bash
cargo build                        # 기본 빌드 (임베딩 제외)
cargo build --features embed       # 임베딩 포함 빌드 (bge-m3-onnx-rust)
cargo test                         # 기본 테스트
cargo test --features embed        # 전체 테스트 (임베딩 포함)

# 개별 테스트
cargo test --test application_test    # Application Service API
cargo test --test emotion_test        # OCC 감정 평가 + merge + trigger
cargo test --test relationship_test   # 관계 3축 모델 및 변동
cargo test --test personality_test    # HEXACO 성격 모델
cargo test --test guide_test          # LLM 연기 가이드 생성 + PowerLevel
cargo test --test directive_test      # ActingDirective 의사결정 트리 전 분기 검증
cargo test --test pad_test            # PAD 공간 분석
cargo test --test stimulus_test       # 대사 자극 감정 변동 + 관성
cargo test --test dialogue_flow_test  # 대화 흐름 통합 테스트
cargo test --test locale_test         # 언어 설정 + 플러거블 포맷터
cargo test --test port_injection_test # 포트 주입 + Scene/Beat 통합
cargo test --test coverage_gap_test   # 커버리지 갭 보완 (valence, merge, PAD 좌표 등)
cargo test --test scene_test          # Scene 도메인 애그리거트 단위 테스트
cargo test --test repository_test     # InMemoryRepository (JSON 로드, Scene, 서비스 연동)
cargo test --test anchor_source_test  # PAD 앵커 소스 어댑터 (TOML/JSON 파싱 + 캐시)
cargo test --test embed_test          # PAD 앵커 임베딩 통합 (embed feature 필요)

# PAD 벤치마크 & 분석 (embed feature 필요)
cargo test --features embed --test pad_benchmark_test      # PAD 분석기 품질 벤치마크
cargo test --features embed --test pad_anchor_count_bench  # PAD 앵커 개수별 정확도 비교
cargo test --features embed --test pad_colbert_bench       # Dense vs ColBERT PAD 분석 비교
cargo test --features embed --test pad_gemini_bench        # Gemini 제안 앵커 vs 현재 앵커 비교
cargo test --features embed --test pad_individual_scores   # 앵커별 개별 cos_sim 행렬 검증

# mind-studio 빌드 & 실행
cargo run --features mind-studio --bin npc-mind-studio          # http://127.0.0.1:3000
cargo run --features mind-studio,embed --bin npc-mind-studio    # 대사→PAD 분석 포함
```

### 빌드 주의사항 (Windows)

`--features embed` 사용 시 ort(ONNX Runtime) 정적 링크를 위해
`.cargo/config.toml`에서 CRT를 동적으로 통일해야 함. 변경 후에는 `cargo clean` 필수.

## 프로젝트 구조

```
src/
  application/                    # 어플리케이션 계층 (라이브러리 진입점)
    mind_service.rs               # MindService — 핵심 오케스트레이션
    formatted_service.rs          # FormattedMindService — MindService + 포맷터 조합
    dto.rs                        # API 데이터 전송 객체 (Result/Response 분리)
  domain/
    tuning.rs                     # 튜닝 상수 — 모든 조정 가능 파라미터 중앙 관리
    personality.rs                # HEXACO 성격 모델 + 성격→감정 가중치 상수
    relationship.rs               # 관계 모델 (closeness, trust, power) + significance
    pad.rs                        # PAD 감정 공간 분석
    pad_table.rs                  # OCC → PAD 좌표 매핑 테이블 (Gebhard 2005 기반)
    pad_anchors.rs                # PAD 앵커 레지스트리 (빌트인 TOML 로드)
    emotion/
      appraisal/                  # 세부 평가 모듈 (event, action, object, compound, helpers)
      engine.rs                   # AppraisalEngine (Appraiser 포트 구현)
      types.rs                    # OCC 감정 타입, EmotionState, merge_from_beat
      situation.rs                # 상황 모델 + Event/Action/Object Focus
      scene.rs                    # Scene 도메인 애그리거트 (check_trigger, set_active_focus, initial_focus)
      stimulus.rs                 # StimulusEngine (StimulusProcessor 포트 구현)
    guide/                        # LLM 연기 가이드 생성 로직
      enums.rs                    # Tone, Attitude, BehavioralTendency 등
      directive.rs                # ActingDirective (감정+성격 → 연기 지시)
      snapshot.rs                 # PersonalitySnapshot, EmotionSnapshot, RelationshipSnapshot
  ports.rs                        # 포트 트레이트 (MindRepository, Appraiser, GuideFormatter 등)
  adapter/                        # 포트 구현 (인프라 어댑터)
    memory_repository.rs          # InMemoryRepository — 기본 MindRepository 구현체 (JSON 로드)
    ort_embedder.rs               # ORT ONNX 임베딩 (embed feature)
    toml_anchor_source.rs         # TOML 기반 PadAnchorSource 구현
    json_anchor_source.rs         # JSON 기반 PadAnchorSource 구현
  presentation/                   # 다국어 지원 및 텍스트 포맷팅
    mod.rs                        # 빌트인 로케일 레지스트리 (ko, en)
    formatter.rs                  # LocaleFormatter (TOML 기반 포맷터)
    locale.rs                     # LocaleBundle (TOML 파싱 + deep merge)
    korean.rs                     # KoreanFormatter (편의 래퍼)
  bin/mind-studio/                # NPC Mind Studio (Axum 서버)
    main.rs                       # Axum 웹 서버 진입점
    handlers.rs                   # HTTP 엔드포인트 핸들러
    state.rs                      # 공유 애플리케이션 상태
    trace_collector.rs            # Appraisal Trace 수집기
    static/index.html             # 웹 UI
tests/
  common/mod.rs                   # TestContext, InMemoryRepository 별칭, Fixtures
  application_test.rs             # MindService API + after_beat/after_dialogue 비교
  emotion_test.rs                 # OCC 감정 + 전망확인 + merge + trigger
  relationship_test.rs            # 관계 모델 + significance 배율
  directive_test.rs               # ActingDirective Tone/Attitude/Behavior/Restriction 전 분기
  coverage_gap_test.rs            # valence, merge 경계값, PAD 좌표, 수식 정밀 검증
  locale_test.rs                  # 언어 설정 + 플러거블 포맷터
  port_injection_test.rs          # 포트 주입 + Scene/Beat 통합
  repository_test.rs              # InMemoryRepository (JSON 로드, Scene, 서비스 연동)
  scene_test.rs                   # Scene 도메인 애그리거트 단위 테스트
  embed_test.rs                   # PAD 앵커 임베딩 통합 (embed feature)
  pad_benchmark_test.rs           # PAD 앵커 정확도 벤치마크 (embed feature)
  pad_anchor_count_bench.rs       # PAD 앵커 개수별 성능 비교 (embed feature)
  pad_colbert_bench.rs            # ColBERT 스코어링 벤치마크 (embed feature)
  pad_gemini_bench.rs             # Gemini 임베딩 벤치마크 (embed feature)
  pad_individual_scores.rs        # PAD 개별 감정 스코어 분석 (embed feature)
locales/
  ko.toml                          # 한국어 로케일 TOML
  en.toml                          # 영어 로케일 TOML
  anchors/
    ko.toml                        # 한국어 PAD 앵커 텍스트 (무협 도메인)
docs/
  api/
    api-reference.md                # 공개 API 레퍼런스 (서비스, DTO, 포트, 도메인 타입)
    integration-guide.md            # 외부 프로젝트 통합 가이드 (단계별)
  architecture/
    architecture-v2.md              # 아키텍처 v2 설계 문서
    situation-structure.md          # 상황 구조 설계
  emotion/
    appraisal-engine.md             # 감정 평가 엔진 설계
    occ-emotion-model.md            # OCC 감정 모델 정의
    hexaco-occ-emotion-mapping.md   # HEXACO→OCC 매핑 설계
    pad-anchor-score-matrix.md      # PAD 앵커 스코어 매트릭스
    pad-stimulus-design-decisions.md # PAD 자극 설계 결정
  guide/
    guide-mapping-table.md          # 가이드 매핑 테이블
  personality/
    hexaco-research.md              # HEXACO 연구 자료
    facets/                         # 6차원별 상세 가이드 (h,e,x,a,c,o)
  infra/
    embedding-adapter-migration.md  # 임베딩 어댑터 마이그레이션
  collaboration-workflow.md         # 협업 워크플로우
  locale-guide.md                   # 언어 설정 가이드 (개발자/사용자)
data/
  huckleberry_finn/                 # 소설 기반 테스트 시나리오
    ch8_jackson_island_meeting/     # 8장: 잭슨 섬 만남
      session_001/                  # scenario.json + evaluation + turn logs
      session_002/                  # scenario.json + evaluation
    ch15_fog_trash/                 # 15장: 안개 속 쓰레기
      session_001/                  # scenario.json
  presets/                          # 캐릭터 사전설정 JSON (so_ho, shu_lien, mu_baek, gyo_ryong)
mcp/
  mind_studio_server.py             # MCP 서버 — Mind Studio API 16개 도구 노출 (Python + mcp SDK)
  requirements.txt                  # Python 의존성 (mcp, httpx)
  README.md                         # AI Agent용 MCP 설정 가이드
.mcp.json                           # 프로젝트 MCP 서버 설정 (mind-studio)
```

## 아키텍처 (DDD + 헥사고날 + 계층화)

### 계층 구조 (Layering)
1.  **Domain**: 순수 비즈니스 로직 (`src/domain`). 외부 의존성 없음.
2.  **Application**: 도메인 객체 조립 및 흐름 제어 (`src/application`). 라이브러리 사용자의 주요 진입점.
3.  **Ports**: 헥사고날 경계 정의 (`src/ports.rs`). 모든 포트 트레이트가 여기에 위치.
4.  **Infrastructure/Presentation**: 외부 라이브러리 구현 및 API 노출 (`src/adapter`, `src/presentation`, `src/bin/mind-studio`).

### 핵심 진입점

라이브러리에 내장된 `InMemoryRepository`를 사용하거나, `MindRepository` 포트를 직접 구현하여 서비스를 생성합니다.

**`InMemoryRepository`** — 기본 제공 MindRepository 구현체 (`adapter/memory_repository.rs`)
- `InMemoryRepository::from_file("scenario.json")` — Mind Studio JSON 로드
- `InMemoryRepository::from_json(json_str)` — JSON 문자열에서 로드
- `InMemoryRepository::new()` — 빈 상태 + `add_npc()`/`add_relationship()`/`add_object()`
- `scenario_name()`, `scenario_description()`, `turn_history()` — 메타데이터 접근자

**`MindService<R, A, S>`** — 도메인 결과 반환 (포맷팅 없음)
- 제네릭 `A: Appraiser`, `S: StimulusProcessor` (기본값: `AppraisalEngine`, `StimulusEngine`)
- `MindService::new(repo)` — 기본 엔진 사용
- `MindService::with_engines(repo, appraiser, stimulus)` — 커스텀 엔진 주입
- 반환 타입: `AppraiseResult`, `StimulusResult`, `GuideResult` 등 (`ActingGuide` 포함)

**`FormattedMindService<R, A, S>`** — 포맷팅된 프롬프트 반환
- `FormattedMindService::new(repo, "ko")` — 빌트인 언어
- `FormattedMindService::with_overrides(repo, "ko", custom_toml)` — 부분 커스터마이징
- `FormattedMindService::with_custom_locale(repo, full_toml)` — 완전 커스텀
- `FormattedMindService::with_formatter(repo, impl GuideFormatter)` — 트레이트 직접 구현
- 반환 타입: `AppraiseResponse`, `StimulusResponse` 등 (`prompt: String` 포함)

### 주요 메서드
- `appraise()`: 초기 상황 판단 및 감정 생성
- `apply_stimulus()`: 대화 중 실시간 감정 변화 + Beat 전환 자동 처리
- `start_scene()`: Scene 시작 — Focus 목록 등록 + 초기 Focus appraise
- `scene_info()`: 현재 Scene Focus 상태 조회
- `load_scene_focuses()`: 시나리오 로드 시 scene 복원
- `after_beat()`: Beat 종료 후 관계 갱신 (감정 유지)
- `after_dialogue()`: 대화(Scene) 종료 후 관계 갱신 + 감정 초기화
- `generate_guide()`: 현재 감정 상태에서 가이드 재생성

### 포트 (ports.rs)

모든 포트 트레이트가 `ports.rs`에 위치합니다:

| 포트 | 용도 | 구현체 |
|------|------|--------|
| `NpcWorld` | NPC/관계/오브젝트 조회 및 관계 갱신 | `InMemoryRepository` (기본) |
| `EmotionStore` | NPC별 감정 상태 CRUD | `InMemoryRepository` (기본) |
| `SceneStore` | Scene 애그리거트 CRUD (`get_scene`/`save_scene`/`clear_scene`) | `InMemoryRepository` (기본) |
| `MindRepository` | 위 3개 포트 통합 (super-trait) | 자동 blanket impl |
| `Appraiser` | 감정 평가 엔진 | `AppraisalEngine` (기본) |
| `StimulusProcessor` | 자극 처리 엔진 | `StimulusEngine` (기본) |
| `PersonalityProfile` | 성격 차원 평균 제공 (가이드용) | `HexacoProfile` |
| `AppraisalWeights` | 성격 → 감정 가중치 | `HexacoProfile` |
| `StimulusWeights` | 성격 → 자극 수용도 | `HexacoProfile` |
| `GuideFormatter` | 가이드 → 텍스트/JSON | `LocaleFormatter`, `KoreanFormatter` |
| `PadAnchorSource` | PAD 앵커 로드/캐시 | `TomlAnchorSource`, `JsonAnchorSource` |
| `TextEmbedder` | 텍스트 → 벡터 | `OrtEmbedder` (embed feature) |
| `UtteranceAnalyzer` | 대사 → PAD | `PadAnalyzer` |

### DTO 분리 (Result / Response)
- `*Result` (도메인): `ActingGuide` 포함, 포맷팅 전. `MindService`가 반환.
- `*Response` (포맷팅 완료): `prompt: String` 포함. `FormattedMindService`가 반환.
- 변환: `result.format(&formatter)` → `Response`

### 감정 평가 (Appraisal) 모듈화
`AppraisalEngine`은 `Appraiser` 트레이트를 구현하며, 내부적으로 세부 모듈을 호출합니다:
- `event`: 사건의 바람직함 평가 (Joy, Distress, Hope, Fear 등)
- `action`: 행위의 정당성 평가 (Pride, Admiration, Anger 등)
- `object`: 대상의 매력도 평가 (Love, Hate)
- `compound`: 기초 감정 결합 (Gratitude, Remorse 등)

성격 가중치 패턴: `BASE + (Score × W)` — personality.rs 내부 상수(`W_STANDARD=0.3`, `W_STRONG=0.4`, `W_DOMINANT=0.7`, `W_MILD=0.2`)로 관리됩니다.

### 복합 감정 (Compound Emotions)
기초 감정들이 결합하여 고차원 감정을 생성합니다:
- **Gratification:** Pride + Joy / **Remorse:** Shame + Distress
- **Gratitude:** Admiration + Joy / **Anger:** Reproach + Distress

### 관계에 의한 변조 (Relationship Modifiers)
- **친밀도(Closeness):** 타인 감정에 대한 공감/적대 반응 강도 및 타인 행동 평가의 기본 배율
- **신뢰도(Trust):** 타인의 행동(Admiration/Reproach) 평가 시 가중치

## 개발 컨벤션

### 에러 처리 및 응답
- 서비스 계층은 `MindServiceError`를 반환합니다.
- 웹 계층(`mind-studio`)은 `AppError`를 통해 서비스 에러를 적절한 HTTP 상태 코드와 JSON으로 자동 변환(`IntoResponse`)합니다.

### 데이터 변환 (Mapping)
- DTO(`SituationInput` 등)는 `to_domain()` 메서드를 통해 도메인 모델로 변환됩니다. 이 과정에서 필요한 관계 조회 등을 위해 `MindRepository`를 참조합니다.

### 테스트 원칙 (TestContext)
- 모든 통합 테스트는 `tests/common/mod.rs`의 `TestContext`를 사용합니다.
- 캐릭터 생성이나 저장소 초기화 등의 중복 코드를 방지하고, 일관된 테스트 환경을 보장합니다.

## Mind Studio (axum 기반 협업 도구)

Claude(API)와 Bekay(브라우저)가 동시에 사용하는 심리 엔진 시뮬레이터입니다.
Mind Studio handlers는 `MindService` API만 호출하는 얇은 래퍼입니다.
- 서버: `cargo run --features mind-studio --bin npc-mind-studio` → http://127.0.0.1:3000
- 임베딩 포함: `cargo run --features mind-studio,embed --bin npc-mind-studio` (대사→PAD 자동 분석 활성화)
- 턴 히스토리: 각 API 호출 결과가 `TurnRecord`로 기록되어 시각화됩니다.

### Mind Studio 주요 기능
- NPC/관계/오브젝트 CRUD
- OCC 감정 평가 (appraise) 및 LLM 연기 가이드 생성
- **대사 기반 PAD 자극 분석**: 상대 대사 입력 → PadAnalyzer(BGE-M3)로 PAD 자동 추출 → 슬라이더 반영 (embed feature 필요, 없으면 수동 입력)
- 시나리오 로드/세이브 및 현재 시나리오명 헤더 표시
- 턴별 히스토리, Appraisal Trace 로그
- **Scene Focus 패널**: 시나리오 JSON에 정의된 Focus 옵션 목록을 읽기 전용으로 표시 (활성/대기 상태, trigger 조건)
- **Beat 전환 표시**: stimulus 결과에서 Beat 전환 발생 시 시각적 배너
- **상황 중요도 슬라이더**: after_dialogue 시 significance (0.0~1.0) 설정

### Mind Studio API 엔드포인트

**CRUD:**
- `GET /api/npcs` — NPC 목록 조회
- `POST /api/npcs` — NPC 생성/수정
- `DELETE /api/npcs/{id}` — NPC 삭제
- `GET /api/relationships` — 관계 목록 조회
- `POST /api/relationships` — 관계 생성/수정
- `DELETE /api/relationships/{owner}/{target}` — 관계 삭제
- `GET /api/objects` — 오브젝트 목록 조회
- `POST /api/objects` — 오브젝트 생성/수정
- `DELETE /api/objects/{id}` — 오브젝트 삭제

**감정 파이프라인:**
- `POST /api/appraise` — 감정 평가 실행
- `POST /api/stimulus` — PAD 자극 적용 + Focus 전환 판단
- `POST /api/guide` — 현재 감정에서 가이드 재생성
- `POST /api/after-dialogue` — 대화 종료 후 관계 갱신

**Scene:**
- `POST /api/scene` — Scene 시작: Focus 옵션 목록 등록 + 초기 Focus 자동 appraise
- `GET /api/scene-info` — 현재 Scene Focus 상태 조회

**분석:**
- `POST /api/analyze-utterance` — 대사 → PAD 자동 분석 (embed feature)

**시나리오 & 상태:**
- `GET /api/scenarios` — 시나리오 파일 목록
- `GET /api/scenario-meta` — 로드된 시나리오 메타정보
- `POST /api/save` — 현재 상태 저장
- `POST /api/load` — 시나리오 로드 (scene 필드 시 Focus 자동 등록)
- `GET /api/situation` — 상황 패널 상태 조회
- `PUT /api/situation` — 상황 패널 상태 저장
- `GET /api/history` — 턴별 히스토리 조회

## 용어 정의

| 용어 | 영문 | 정의 | 엔진 호출 |
|------|------|------|----------|
| **장면** | Scene | 하나의 연속된 대화 단위. 시작과 끝이 있음. | `after_dialogue()` 1회 |
| **비트** | Beat | 장면 안에서 감정 흐름이 전환되는 시점. | `appraise()` 1회 |
| **대사** | Utterance | 실제 캐릭터가 말하는 한 줄의 대사. | `stimulus()` 입력 |

## Scene Focus 시스템

게임이 Scene 시작 시 Focus 옵션 목록을 제공하고, 엔진이 stimulus 처리 중
감정 상태 조건(FocusTrigger)을 평가하여 자동으로 Beat 전환을 판단합니다.

Beat 전환 로직은 `MindService.apply_stimulus()` → `transition_beat()` 에서 처리됩니다.

### 데이터 구조
- `Scene`: 도메인 애그리거트 루트 (npc_id, partner_id, focuses, active_focus_id)
- `SceneFocus`: Focus 옵션 (id, description, trigger, event/action/object)
- `FocusTrigger`: Initial (즉시 적용) 또는 Conditions (감정 조건)
- `EmotionCondition`: 감정 유형 + 임계값 (Below/Above/Absent)
- 조건 구조: `OR [ AND[...], AND[...] ]` — 외부 배열 OR, 내부 배열 AND

### Scene 애그리거트 메서드
- `Scene::new(npc_id, partner_id, focuses)` — 생성
- `Scene::check_trigger(&self, state: &EmotionState)` — 대기 Focus 중 조건 충족된 것 반환
- `Scene::set_active_focus(&mut self, focus_id)` — 활성 Focus 설정
- `Scene::initial_focus(&self)` — Initial 트리거를 가진 Focus 검색

### Beat 전환 흐름
```
apply_stimulus 호출
  → 1. 감정 강도 조정 (관성 적용)
  → 2. scene.check_trigger(&state) — 대기 중 Focus의 조건 체크
  → 3. 조건 충족 시 → transition_beat():
       a. update_beat_relationship() — 관계 갱신 (감정 유지)
       b. scene.set_active_focus() + 새 Focus로 appraise
       c. merge_from_beat (이전 감정 + 새 감정 합치기)
  → 4. repo.save_scene() + StimulusResult.beat_changed = true
```

### Scene 초기화 공통 로직
`start_scene()`과 `load_scene_focuses()`는 내부적으로 공통 헬퍼를 사용합니다:
- `Scene::new()` + `repo.save_scene()`: Scene 애그리거트 생성 및 저장
- `appraise_initial_focus()`: `scene.initial_focus()` → appraise → 감정 저장

### 감정 합치기 (merge_from_beat)
- 같은 감정: max 기준으로 강도 + context 유지
- 이전 감정 중 BEAT_MERGE_THRESHOLD(0.2) 미만: 소멸
- 새 감정만 있으면: 그대로 추가

## 튜닝 상수 (`src/domain/tuning.rs`)

모든 조정 가능한 수치 파라미터를 한 곳에 모아 관리합니다.

| 상수 | 값 | 용도 |
|------|-----|------|
| STIMULUS_IMPACT_RATE | 0.5 | stimulus 감정 변동 계수 |
| STIMULUS_FADE_THRESHOLD | 0.05 | 감정 자연 소멸 기준 |
| STIMULUS_MIN_INERTIA | 0.30 | 관성 최소값 (intensity=1.0에서도 반응) |
| BEAT_MERGE_THRESHOLD | 0.2 | Beat 합치기 시 이전 감정 소멸 기준 |
| BEAT_DEFAULT_SIGNIFICANCE | 0.5 | Beat 전환 시 기본 significance |
| TRUST_UPDATE_RATE | 0.1 | 신뢰 갱신 계수 |
| CLOSENESS_UPDATE_RATE | 0.05 | 친밀도 갱신 계수 |
| SIGNIFICANCE_SCALE | 3.0 | 상황 중요도 배율 (sig=1.0 → 4배) |
| PAD_D_SCALE_WEIGHT | 0.3 | PAD D축 격차 스케일러 |
| PAD_AXIS_DEAD_ZONE | 0.02 | PAD 축 데드존 (미세 변동 무시) |
| PAD_AXIS_SCALE | 3.0 | PAD 축 스케일 계수 |
| MOOD_THRESHOLD | 0.3 | 기분 분기 임계값 |
| HONESTY_RESTRICTION_THRESHOLD | 0.5 | 정직성 제약 임계값 |
| EMOTION_THRESHOLD | 0.2 | 감정 유의미 판단 기준 (가이드 반영) |
| TRAIT_THRESHOLD | 0.3 | 성격 특성 추출 임계값 |
| REL_CLOSENESS_INTENSITY_WEIGHT | 0.5 | closeness → 감정 강도 배율 |
| REL_TRUST_EMOTION_WEIGHT | 0.3 | trust → 행동 평가 배율 |
| REL_CLOSENESS_EMPATHY_WEIGHT | 0.3 | closeness → 공감 배율 |
| REL_CLOSENESS_HOSTILITY_WEIGHT | 0.3 | closeness → 적대 배율 |
| LEVEL_VERY_HIGH_THRESHOLD | 0.6 | 레벨 분류: VeryHigh 기준 |
| LEVEL_HIGH_THRESHOLD | 0.2 | 레벨 분류: High 기준 |
| LEVEL_LOW_THRESHOLD | -0.2 | 레벨 분류: Low 기준 |
| LEVEL_VERY_LOW_THRESHOLD | -0.6 | 레벨 분류: VeryLow 기준 |

### 파일별 로컬 상수

| 파일 | 상수 | 용도 |
|------|------|------|
| `personality.rs` | W_STANDARD(0.3), W_STRONG(0.4), W_DOMINANT(0.7), W_MILD(0.2) | 성격→감정 가중치 |
| `personality.rs` | BASE_SELF(1.0), BASE_EMPATHY(0.5), BASE_HOSTILITY(0.0) | 가중치 기저값 |
| `personality.rs` | CLAMP_STANDARD(0.5,1.5), CLAMP_OPTIONAL(0.0,1.5), CLAMP_STIMULUS(0.1,2.0) | 가중치 범위 |
| `personality.rs` | SCORE_MIN(-1.0), SCORE_MAX(1.0), SCORE_NEUTRAL(0.0) | 성격 점수 범위 |
| `pad_table.rs` | 22개 감정별 PAD 좌표 (Gebhard 2005 기반, 커스텀 조정 포함) | OCC→PAD 매핑 |

## stimulus 관성 공식

```
inertia = max(1.0 - intensity, STIMULUS_MIN_INERTIA)
delta = pad_dot × absorb_rate × STIMULUS_IMPACT_RATE × inertia
```

- 강한 감정(intensity 높음) → inertia 작음 → 자극에 덜 흔들림
- 약한 감정(intensity 낮음) → inertia 큼 → 자극에 쉽게 변함
- intensity=1.0이어도 최소 관성(0.30)으로 변동 보장
