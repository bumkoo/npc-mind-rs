# CLAUDE.md

NPC Mind Engine — HEXACO 성격이 OCC 감정을 생성하고, LLM이 연기할 수 있도록 가이드를 출력하는 Rust 라이브러리.

라이브러리 형태로 배포되며, `MindService`가 유일한 공개 진입점입니다.

## 기술 스택
- **Language:** Rust (Edition 2024)
- **Architecture:** Hexagonal Architecture (Ports and Adapters) + DDD
- **Libraries:** `serde`/`serde_json`, `thiserror`, `axum`/`tokio`(WebUI), `tracing`, `ort`(ONNX 임베딩), `rig-core`(LLM Agent 대화)

## 빌드 & 테스트

```bash
cargo build                        # 기본 빌드
cargo build --features embed       # 임베딩 포함 (bge-m3-onnx-rust)
cargo build --features chat        # LLM 대화 에이전트 포함 (rig-core)
cargo test                         # 기본 테스트
cargo test --features embed        # 전체 테스트 (임베딩 포함)

# 개별 테스트는 tests/ 디렉토리 참조
# PAD 벤치마크(pad_benchmark_test 등)는 --features embed 필요

# mind-studio 실행
cargo run --features mind-studio,chat,embed --bin npc-mind-studio  # http://127.0.0.1:3000
```

### 환경변수 (주요)

```
NPC_MIND_CHAT_URL=http://127.0.0.1:8081/v1   # 로컬 LLM 서버 [chat feature]
NPC_MIND_MODEL_DIR=../models/bge-m3          # ONNX 모델 [embed feature]
NPC_MIND_ANCHOR_LANG=ko                       # PAD 앵커 언어 [embed feature]
MIND_STUDIO_PORT=3000                         # 서버 포트 [mind-studio feature]
```

### 빌드 주의사항 (Windows)

- `--features embed`: ort(ONNX Runtime) 정적 링크를 위해 `.cargo/config.toml`에서 CRT를 동적으로 통일해야 함. 변경 후 `cargo clean` 필수.
- `--features chat`: rig-core 기본 TLS 백엔드(`aws-lc-sys`)가 MSVC에서 `__builtin_bswap` 링크 실패. Cargo.toml에서 `default-features = false, features = ["reqwest-native-tls"]` 사용.
- rig 0.33+ OpenAI provider는 기본 Responses API(`/v1/responses`) 사용. llama.cpp 등 로컬 서버는 Chat Completions만 지원하므로 `.completions_api()` 호출 필수.

## 프로젝트 구조 (주요 디렉토리)

```
src/
  application/    어플리케이션 계층, 라이브러리 진입점 (MindService, FormattedMindService)
                  + relationship_service, scene_service, situation_service, dialogue_test_service
  domain/         순수 도메인 로직
                  - personality (HEXACO), emotion (OCC appraisal), relationship, pad, guide
                  - tuning.rs (조정 가능 파라미터 중앙 관리)
  ports.rs        헥사고날 포트 트레이트 전체
  adapter/        포트 구현 (InMemoryRepository, OrtEmbedder, RigChatAdapter, FileAnchorSource)
  presentation/   다국어 포맷터 (ko, en TOML 기반, deep merge 지원)
  bin/mind-studio/  Axum 기반 웹 UI + REST API + 네이티브 SSE MCP 서버
tests/            통합 테스트 (TestContext 공유)
locales/          ko.toml, en.toml + PAD 앵커 (locales/anchors/)
docs/             아키텍처/감정/성격/가이드 상세 문서
data/             소설 기반 테스트 시나리오 + 캐릭터 프리셋(presets/)
```

## 아키텍처

### 계층 구조
1. **Domain** (`src/domain`): 순수 비즈니스 로직, 외부 의존성 없음
2. **Application** (`src/application`): 도메인 조립 및 흐름 제어, 라이브러리 사용자 진입점
3. **Ports** (`src/ports.rs`): 헥사고날 경계 정의
4. **Infrastructure/Presentation** (`src/adapter`, `src/presentation`, `src/bin`): 외부 구현 및 API 노출

### 핵심 진입점

**`InMemoryRepository`** — 기본 `MindRepository` 구현체 (`adapter/memory_repository.rs`)
- `from_file("scenario.json")` / `from_json(json_str)` / `new()` + `add_npc()`/`add_relationship()`/`add_object()`
- `scenario_name()`, `scenario_description()`, `turn_history()` 메타데이터 접근자

**`MindService<R, A, S>`** — 도메인 결과 반환 (포맷팅 없음)
- `MindService::new(repo)` 또는 `::with_engines(repo, appraiser, stimulus)`
- 반환: `AppraiseResult`, `StimulusResult`, `GuideResult` (`ActingGuide` 포함)

**`FormattedMindService<R, A, S>`** — 포맷팅된 프롬프트 반환
- `::new(repo, "ko")` / `::with_overrides()` / `::with_custom_locale()` / `::with_formatter()`
- 반환: `AppraiseResponse`, `StimulusResponse` 등 (`prompt: String` 포함)

### 주요 메서드
- `appraise()` — 초기 상황 판단 및 감정 생성
- `apply_stimulus()` — 대화 중 실시간 감정 변화 + Beat 전환 자동 처리
- `start_scene()` / `scene_info()` / `load_scene_focuses()` — Scene 관리
- `after_beat()` / `after_dialogue()` — 관계 갱신
- `generate_guide()` — 현재 감정에서 가이드 재생성

### 주요 포트 (전체는 `ports.rs` 참조)

| 포트 | 용도 | 기본 구현체 |
|------|------|----------|
| `MindRepository` | `NpcWorld` + `EmotionStore` + `SceneStore` 통합 super-trait | `InMemoryRepository` |
| `Appraiser` | OCC 감정 평가 엔진 | `AppraisalEngine` |
| `StimulusProcessor` | PAD 자극 처리 엔진 | `StimulusEngine` |
| `GuideFormatter` | 가이드 → 텍스트/JSON | `LocaleFormatter` |
| `UtteranceAnalyzer` | 대사 → PAD ([embed feature]) | `PadAnalyzer` |
| `ConversationPort` | LLM 다턴 대화 세션 ([chat feature]) | `RigChatAdapter` |

### 감정 평가 흐름

`AppraisalEngine`은 세부 모듈로 분리되어 있습니다:
- **event** (Joy/Distress/Hope/Fear), **action** (Pride/Admiration/Anger), **object** (Love/Hate)
- **compound**: 기초 감정 결합 — Gratification(Pride+Joy), Remorse(Shame+Distress), Gratitude(Admiration+Joy), Anger(Reproach+Distress)
- 성격 가중치 패턴: `BASE + (Score × W)` — `personality.rs` 내부 상수 관리
- 관계 변조: closeness(공감/적대 강도 배율), trust(행동 평가 가중치)


## 개발 컨벤션

### DTO 분리 (Result / Response)
- `*Result` (도메인): `ActingGuide` 포함, 포맷팅 전. `MindService`가 반환
- `*Response` (포맷팅 완료): `prompt: String` 포함. `FormattedMindService`가 반환
- 변환: `result.format(&formatter)` → `Response` (`CanFormat` 트레이트)

### 네이밍 (DDD)
- Domain Services: `~Engine` / `~Analyzer`
- Application Services: `~Service`
- Ports: 행위 명사 (`ports.rs`)
- Domain Events: 과거형

### 에러 처리
- 서비스 계층: `MindServiceError` 반환
- 웹 계층(`mind-studio`): `AppError` → 적절한 HTTP 상태 코드와 JSON으로 자동 변환 (`IntoResponse`)

### 데이터 변환 (Mapping)
- DTO(`SituationInput` 등)는 `SituationService`를 통해 도메인 모델로 변환
- DTO는 저장소 의존성 없는 순수 데이터 구조체
- 서비스가 저장소(`MindRepository`)에서 관계/오브젝트 정보를 조회하여 변환 시 주입

### 테스트 (TestContext)
- 모든 통합 테스트는 `tests/common/mod.rs`의 `TestContext`를 사용
- 캐릭터 생성 / 저장소 초기화 중복 코드 방지, 일관된 테스트 환경 보장

## 용어 정의

| 용어 | 영문 | 정의 | 엔진 호출 |
|------|------|------|----------|
| **장면** | Scene | 하나의 연속된 대화 단위. 시작과 끝이 있음 | `after_dialogue()` 1회 |
| **비트** | Beat | 장면 안에서 감정 흐름이 전환되는 시점 | `appraise()` 1회 |
| **대사** | Utterance | 실제 캐릭터가 말하는 한 줄의 대사 | `apply_stimulus()` 입력 |

## Scene Focus 시스템

게임이 Scene 시작 시 Focus 옵션 목록을 제공하고, 엔진이 stimulus 처리 중 감정 상태 조건(`FocusTrigger`)을 평가하여 자동으로 Beat 전환을 판단합니다. Beat 전환 로직은 `MindService.apply_stimulus()` → `transition_beat()`에서 처리됩니다.

### 데이터 구조
- `Scene`: 도메인 애그리거트 루트 (npc_id, partner_id, focuses, active_focus_id)
- `SceneFocus`: Focus 옵션 (id, description, trigger, event/action/object)
- `FocusTrigger`: `Initial`(즉시 적용) 또는 `Conditions`(감정 조건)
- `EmotionCondition`: 감정 유형 + 임계값 (`Below`/`Above`/`Absent`)
- 조건 구조: `OR [ AND[...], AND[...] ]` — 외부 배열 OR, 내부 배열 AND

### Scene 애그리거트 메서드
- `Scene::new(npc_id, partner_id, focuses)` — 생성
- `Scene::check_trigger(&state)` — 대기 Focus 중 조건 충족된 것 반환
- `Scene::set_active_focus(focus_id)` — 활성 Focus 설정
- `Scene::initial_focus()` — `Initial` 트리거를 가진 Focus 검색

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

### 감정 합치기 (merge_from_beat)
- 같은 감정: max 기준으로 강도 + context 유지
- 이전 감정 중 `BEAT_MERGE_THRESHOLD`(0.2) 미만: 소멸
- 새 감정만 있으면: 그대로 추가

## Stimulus 관성 공식

```
inertia = max(1.0 - intensity, STIMULUS_MIN_INERTIA)
delta = pad_dot × absorb_rate × STIMULUS_IMPACT_RATE × inertia
```

- 강한 감정(intensity 높음) → inertia 작음 → 자극에 덜 흔들림
- 약한 감정(intensity 낮음) → inertia 큼 → 자극에 쉽게 변함
- intensity=1.0이어도 최소 관성(0.30)으로 변동 보장

## 튜닝 상수 (주요, 전체는 `src/domain/tuning.rs` 참조)

| 상수 | 값 | 용도 |
|------|-----|------|
| `STIMULUS_IMPACT_RATE` | 0.5 | stimulus 감정 변동 계수 |
| `STIMULUS_MIN_INERTIA` | 0.30 | 관성 최소값 (intensity=1.0에서도 반응 보장) |
| `BEAT_MERGE_THRESHOLD` | 0.2 | Beat 합치기 시 이전 감정 소멸 기준 |
| `TRUST_UPDATE_RATE` | 0.1 | 신뢰 갱신 계수 |
| `CLOSENESS_UPDATE_RATE` | 0.05 | 친밀도 갱신 계수 |
| `SIGNIFICANCE_SCALE` | 3.0 | 상황 중요도 배율 (sig=1.0 → 4배) |
| `EMOTION_THRESHOLD` | 0.2 | 감정 유의미 판단 기준 (가이드 반영) |
| `TRAIT_THRESHOLD` | 0.3 | 성격 특성 추출 임계값 |

파일별 로컬 상수(`personality.rs`의 `W_STANDARD`/`BASE_*`/`CLAMP_*` 등, `pad_table.rs`의 22개 감정별 PAD 좌표)는 해당 파일 상단에 정의되어 있습니다.

## Mind Studio (개발 도구)

Claude(API)와 Bekay(브라우저)가 동시에 사용하는 심리 엔진 시뮬레이터. Mind Studio handlers는 `MindService` API의 얇은 래퍼입니다.

- 서버 실행: `cargo run --features mind-studio,chat,embed --bin npc-mind-studio` → http://127.0.0.1:3000
- 주요 기능: NPC/관계/오브젝트 CRUD, 감정 평가, 가이드 생성, 대사→PAD 자동 분석(embed), 시나리오 로드/세이브, 턴 히스토리, 테스트 레포트
- **Scene Focus 패널**: 시나리오 JSON에 정의된 Focus 옵션 목록을 읽기 전용으로 표시 (활성/대기 상태, trigger 조건, test_script)
- **Beat 전환 표시**: stimulus 결과에서 Beat 전환 발생 시 시각적 배너
- **테스트 스크립트**: 각 Beat의 `test_script` 대사 목록을 Focus 패널에 표시하고, 대화 입력 영역에서 '스크립트 전송' 버튼으로 순차 전송 가능
- **LLM 대화 테스트**(`chat` feature): 로컬 LLM과 다턴 대화, Beat 전환 시 system prompt 동적 갱신
- REST API 엔드포인트 전체는 `src/bin/mind-studio/handlers.rs` 참조

## LLM 대화 테스트 (`chat` feature)

Mind Engine이 생성한 프롬프트를 실제 LLM에 system prompt로 주입하고 다턴 대화로 NPC 연기 품질을 검증합니다.

- **ConversationPort** (`ports.rs`): LLM 대화 세션 추상화 — `start_session`, `send_message`, `update_system_prompt`, `end_session`
- **RigChatAdapter** (`adapter/rig_chat.rs`): rig-core 0.33 `openai::CompletionsClient` 기반 구현. 세션별 system_prompt + rig_history + dialogue_history 관리
- **DialogueTestService** (`application/dialogue_test_service.rs`): `FormattedMindService` + `ConversationPort` 오케스트레이터

대화 루프:
```
appraise → start_session(prompt)
  → { send_message(상대 대사) → apply_stimulus(PAD) → [Beat 전환 시 update_system_prompt] }
  → end_session → after_dialogue(관계 갱신)
```

## 외부 문서 인덱스

- **API 레퍼런스**: [`docs/api/api-reference.md`](docs/api/api-reference.md) — 공개 API, DTO, 포트, 도메인 타입
- **통합 가이드**: [`docs/api/integration-guide.md`](docs/api/integration-guide.md) — 외부 프로젝트 통합 단계별 가이드
- **아키텍처 v2**: [`docs/architecture/architecture-v2.md`](docs/architecture/architecture-v2.md)
- **협업 워크플로우**: [`docs/collaboration-workflow.md`](docs/collaboration-workflow.md)
- **감정 엔진**: [`docs/emotion/`](docs/emotion/) — OCC 모델, HEXACO 매핑, PAD 앵커 매트릭스, appraisal 엔진 설계
- **성격 모델**: [`docs/personality/`](docs/personality/) — HEXACO 6차원 facet 상세
- **가이드 매핑**: [`docs/guide/guide-mapping-table.md`](docs/guide/guide-mapping-table.md)
- **테스트 스크립트**: `mcp/skills/npc-scenario-creator/SKILL.md` (4-1단계) + `mcp/skills/npc-mind-testing/SKILL.md` (원칙 4, 커서 관리)
- **언어 설정**: [`docs/locale-guide.md`](docs/locale-guide.md)
- **MCP 서버 설정**: `.mcp.json` (프로젝트 루트)
