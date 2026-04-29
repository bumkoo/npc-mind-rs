# 아키텍처 결정 기록: Bevy ECS + DDD 통합 전략

> **문서 버전**: v1.2  
> **작성일**: 2025-02-06 (v1.0 업데이트: 2026-02-16, v1.2 업데이트: 2026-02-21)  
> **상태**: Phase 4 진행 중
> **관련 문서**: domain-analysis.md, dev-plan.md  

---

## 1. 문제

DDD(Hexagonal Architecture)와 Bevy ECS는 데이터를 반대 방향으로 조직한다.

```
  DDD                              Bevy ECS
  ─────                            ─────────
  데이터 + 행동이 함께              데이터와 행동이 분리
  (struct + method)                (Component + System)

  Aggregate Root가 변경을 보호      아무 System이나 접근 가능
  Repository로 저장/조회            Bevy World가 저장소
  DomainEvent로 통신               Bevy Event로 통신
```

이 두 패러다임을 무리하게 합치면 이중 관리, 성능 손실, 복잡성 증가가 발생한다.

---

## 2. 결정

### 4가지 원칙 + 1가지 전략

**원칙 1. Pure Logic Lib (도메인 분리)**

게임 로직의 핵심(감정 모델, 전투 계산, 성장/쇠퇴, 가치관 평가 등)은 Bevy에 의존하지 않는 순수 Rust 라이브러리로 따로 만든다.

**원칙 2. Plugin 기반 분리**

Bevy의 기능을 Plugin 단위로 쪼개어, 각 플러그인이 하나의 도메인 서비스 역할을 하게 만든다. 11개 도메인 = 11개 Plugin.

**원칙 3. Non-Send Resource 활용**

외부 API(LLM)나 DB 연결(LanceDB)은 Bevy의 Resource로 등록하되, 내부적으로는 Pure Logic Lib에서 정의한 Outbound Port(trait)를 들고 있게 설계한다.

**원칙 4. 데이터 주도 국제화 (Data-Driven i18n)** — [v1.2 신설]

사용자에게 보이는 모든 문구(프롬프트 헤더, 관계 설명, UI 텍스트 등)는 코드에 직접 넣지 않는다. TOML 설정 파일에서 로딩하여 주입한다.

```
  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
  │  wuxia-core  │     │  wuxia-data  │     │  wuxia-llm   │
  │  타입 정의    │◄────│  TOML 로더   │────►│  사용처       │
  │  (구조만)     │     │  (파싱만)     │     │  (주입받음)   │
  └──────────────┘     └──────────────┘     └──────────────┘
        │                     ▲                     │
        │    struct 제공      │  파일 읽기           │  &Config 참조
        ▼                     │                     ▼
  PromptConfig          assets/data/         build_system_prompt(
  PromptHeaders         prompt_config.toml     ..., &prompt_config)
  RelationshipDescs     descriptions.toml
```

이유:
- 새 언어 추가 시 코드 변경 0건. TOML 파일만 추가.
- 번역자가 Rust를 몰라도 TOML만 편집하면 됨.
- 테스트에서 default_prompt_config() 헬퍼로 독립 검증 가능.

위반 사례 (금지):
- `match locale { Ko => "[기본 정보]", En => "[Basic Info]" }` ← 하드코딩
- `format!("{}세 {}", age, gender)` ← 어순이 언어마다 다름

허용 사례:
- `format!("{}", h.basic_info)` ← h는 TOML에서 로딩된 PromptHeaders
- `config.get_headers(locale.code())` ← locale 키로 HashMap 조회

**전략: 하이브리드 Component**

모든 데이터를 ECS Component로 쪼개지도 않고, 통째로 감싸지도 않는다. 접근 빈도에 따라 다르게 적용한다.

---

## 3. Cargo Workspace 구조 — [v1.0 업데이트]

```
wuxia-rpg/
├── Cargo.toml                    (workspace)
│
├── crates/
│   │
│   │  ── 도메인 계층 (외부 의존 없음) ──
│   │
│   ├── wuxia-core/               (Pure Logic Lib)
│   │   ├── Cargo.toml            (의존성: serde만. toml/json 파서 없음)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── shared/           (공유 타입, ID, 이벤트 wrapper)
│   │       │   ├── mod.rs
│   │       │   ├── id.rs         (CharacterId 등)
│   │       │   ├── time.rs       (GameTime, Season)
│   │       │   ├── event.rs      (DomainEvent wrapper enum)
│   │       │   ├── error.rs      (DomainError)
│   │       │   └── i18n.rs       (Locale, Translations — 포맷 중립)
│   │       ├── character/        (캐릭터 도메인)
│   │       │   ├── mod.rs
│   │       │   ├── model.rs      (Character aggregate)
│   │       │   ├── life_stage.rs (LifeStage)
│   │       │   ├── fatigue.rs    (FatigueLevel) [v1.0]
│   │       │   ├── injury.rs     (Injury, InjuryType) [v1.0]
│   │       │   └── event.rs      (CharacterEvent enum)
│   │       ├── time/             (시간 도메인)
│   │       │   ├── mod.rs
│   │       │   ├── clock.rs      (GameClock aggregate)
│   │       │   └── event.rs      (TimeEvent enum)
│   │       ├── growth/           (성장 도메인) [구현 완료]
│   │       │   ├── mod.rs
│   │       │   ├── stat.rs       (StatType, StatCategory, StatBlock)
│   │       │   ├── model.rs      (GrowthProfile aggregate)
│   │       │   ├── training.rs   (단련/연마 규칙 함수) [v1.0]
│   │       │   ├── martial_art.rs (MartialArt, Proficiency) [v1.0]
│   │       │   └── event.rs      (GrowthEvent enum)
│   │       ├── psychology/       (심리 도메인) [예정]
│   │       ├── relationship/     (관계 도메인) [예정]
│   │       ├── world/            (세계관 도메인) [예정]
│   │       ├── space/            (공간 도메인) [예정]
│   │       ├── narrative/        (서사 도메인) [예정]
│   │       ├── combat/           (전투 도메인) [예정]
│   │       ├── economy/          (경제 도메인) [예정]
│   │       ├── item/             (사물 도메인) [예정]
│   │       └── application/      (Application Services)
│   │           ├── mod.rs
│   │           ├── time_character.rs (TimeCharacterService)
│   │           └── training.rs       (TrainingService) [v1.0]
│   │
│   │  ── 인프라 어댑터 계층 (각자 1개 외부 의존) ──
│   │
│   ├── wuxia-data/               (데이터 로딩 어댑터) [v1.0 신설]
│   │   ├── Cargo.toml            (의존: wuxia-core, serde, serde_json, toml)
│   │   └── src/
│   │       ├── lib.rs
│   │       └── loader.rs         (cfg 분기: 개발=toml, 릴리즈=json)
│   │
│   ├── wuxia-llm/                (LLM 어댑터) [Phase 4 비대만]
│   │   ├── Cargo.toml            (의존: wuxia-core. feature: live-llm)
│   │   └── src/
│   │       └── lib.rs            (prompt/parser/mock/adapter 모듈 예정)
│   │
│   ├── wuxia-memory/             (벡터DB 어댑터) [Phase 4 비대만]
│   │   ├── Cargo.toml            (의존: wuxia-core. feature: live-db)
│   │   └── src/
│   │       └── lib.rs            (mock/adapter/schema 모듈 예정)
│   │
│   │  ── 게임 엔진 계층 (Bevy 의존) ──
│   │
│   ├── wuxia-game/               (Bevy Plugin/Component 라이브러리)
│   │   ├── Cargo.toml            (의존: bevy, wuxia-core)
│   │   └── src/
│   │       ├── lib.rs            (Plugin 리익스포트)
│   │       ├── main.rs           (개발용 실행: Mock LLM + InMemory DB)
│   │       ├── plugins/          (11개 도메인 Plugin) [Phase 5]
│   │       ├── components/       (Bevy Component 정의) [Phase 5]
│   │       ├── resources/        (Resource: LlmService, MemoryDb) [Phase 5]
│   │       └── events/           (DomainEvent → Bevy Event 변환) [Phase 5]
│   │
│   │  ── 최종 조립 계층 ──
│   │
│   └── wuxia-app/                (프로덕션 바이너리) [Phase 5 비대만]
│       ├── Cargo.toml            (의존: 전부)
│       └── src/
│           └── main.rs           (모든 crate 조립 + 실행)
│
├── assets/
│   ├── data/                     게임 데이터 (toml/json)
│   │   ├── characters/
│   │   ├── martial_arts/
│   │   ├── world/
│   │   └── items/
│   ├── locales/                  i18n 번역 파일
│   ├── maps/                     타일맵
│   ├── sprites/                  Pixel Art
│   │   ├── characters/
│   │   ├── tiles/
│   │   └── ui/
│   └── audio/                    BGM, 효과음
│
└── docs/
    ├── architecture/             아키텍처 설계
    ├── design/                   GDD, MVP, 온보딩
    ├── ai/                       LLM/NPC AI 설계 [v1.0 신설]
    │   └── benchmarks/           벤치마크 보고서
    ├── characters/               NPC 전기
    ├── psychology/                NPC 심리 아키텍처
    ├── reference/                외부 참조 (논문, 서적 요약)
    ├── world/                    세계관
    └── plans/                    스프린트 진행, 벤치마크
```

### 3.1 의존성 방향 — [v1.0 신설]

```
                    wuxia-core
                   (순수 도메인)
                   ⊘ 외부 의존
                        │
          ┌─────────────┼─────────────┐
          │ trait        │ trait       │ struct/enum
          │ LlmPort     │ MemoryRepo  │ (Deserialize)
          ▼             ▼             ▼
    wuxia-llm     wuxia-memory    wuxia-data
    (llama-cpp-2) (LanceDB)      (toml/json)
    ⊘ Bevy        ⊘ Bevy         ⊘ Bevy
    ⊘ DB          ⊘ LLM          ⊘ LLM, DB
          │             │             │
          └─────────────┼─────────────┘
                        │
                   wuxia-game
                   (Bevy Plugin들)
                        │
                        ▼
                    wuxia-app
                  (최종 조립 + main)
```

규칙: 화살표는 항상 아래로만. 역방향 의존 금지.

### 3.2 Feature Flag 전략 — [v1.0 신설]

| crate | 기본 (flag 없음) | feature flag 켜면 |
|-------|-----------------|-------------------|
| wuxia-core | 순수 Rust | - |
| wuxia-data | serde + toml + json | - |
| wuxia-llm | Mock + 프롬프트 조립 | `live-llm` → llama-cpp-2 |
| wuxia-memory | InMemory Mock | `live-db` → LanceDB |
| wuxia-game | Bevy + Mock 주입 | - |
| wuxia-app | 전부 조립 | `live-llm` + `live-db` 기본 활성 |

### 3.3 독립 실행 범위 — [v1.0 신설]

| 명령어 | LLM | DB | Bevy | 용도 |
|--------|-----|-----|------|------|
| `cargo test -p wuxia-core` | ⊘ | ⊘ | ⊘ | 도메인 로직 검증 |
| `cargo test -p wuxia-data` | ⊘ | ⊘ | ⊘ | 데이터 로딩 검증 |
| `cargo test -p wuxia-llm` | Mock | ⊘ | ⊘ | 프롬프트 조립/파싱 |
| `cargo test -p wuxia-llm --features live-llm` | 실제 | ⊘ | ⊘ | LLM 응답 품질 |
| `cargo test -p wuxia-memory --features live-db` | ⊘ | 실제 | ⊘ | 벡터 검색 검증 |
| `cargo run -p wuxia-game` | Mock | Mock | ✔ | Bevy UI 개발 |
| `cargo run -p wuxia-app` | 실제 | 실제 | ✔ | 프로덕션 |

### 3.4 데이터 로딩 전략 — [v1.0 신설, v1.2 확장]

```
wuxia-core:  struct에 Deserialize derive만. 포맷 모름.
wuxia-data:  cfg 분기로 로딩 방식 결정.

  #[cfg(debug_assertions)]   → TOML 파서 (사람이 읽기 쉬움)
  #[cfg(not(debug_assertions))] → JSON 파서 (릴리즈 성능)

인프라 설정 (game.toml 등):  항상 TOML (런타임에도)
i18n 번역 파일:             항상 TOML (사람이 편집)
```

#### 3.4.1 3계층 데이터 주도 패턴 — [v1.2 신설]

모든 locale-dependent 데이터에 동일한 3계층 패턴을 적용한다 (원칙 4 구현).

```
  계층 1: 타입 정의 (wuxia-core)
  ─────────────────────────────
  #[derive(Debug, Clone, Deserialize)]
  pub struct PromptHeaders {
      pub basic_info: String,
      pub personality: String,
      ...
  }
  → 포맷을 모른다. TOML인지 JSON인지 관심 없다.

  계층 2: 데이터 파일 (assets/data/)
  ─────────────────────────────────
  [headers.ko]
  basic_info = "[기본 정보]"
  personality = "[성격]"

  [headers.en]
  basic_info = "[Basic Info]"
  personality = "[Personality]"
  → 사람이 읽고 편집한다. 코드 변경 불필요.

  계층 3: 로더 + 사용 (wuxia-data → wuxia-llm)
  ─────────────────────────────────────────────
  // wuxia-data: 로딩
  let config = load_prompt_config("assets/data/prompt/prompt_config.toml")?;

  // wuxia-llm: 사용 (주입받음)
  let h = config.get_headers("ko");  // en fallback 내장
  format!("{}\n{}세 {}", h.basic_info, age, gender);
```

#### 3.4.2 적용 현황

| 데이터 | 파일 위치 | 상태 |
|--------|----------|------|
| 프롬프트 헤더 + 언어 지시문 | `assets/data/prompt/prompt_config.toml` | ✅ 적용 |
| 관계 설명 문구 | `assets/data/relationship/descriptions.toml` | ✅ 적용 |
| 게임 내 번역 | `assets/locales/ko.toml`, `en.toml` | ✅ 적용 |
| 감정 표시 문구 | `assets/data/psychology/emotion_labels.toml` | ⬚ 예정 |
| UI 텍스트 | `assets/data/ui/ui_strings.toml` | ⬚ 예정 |
| 전투 메시지 | `assets/data/combat/combat_messages.toml` | ⬚ 예정 |

---

## 4. 기술 스택 (이 문서가 단일 소유자) — [v1.1 신설]

> 다른 문서에서는 "ECS 게임엔진", "Local LLM", "벡터DB" 등 일반 용어만 사용하고,  
> 구체적 제품/버전은 이 섹션을 참조한다.

| 영역 | 기술 | 상태 | 비고 |
|------|------|------|------|
| 언어 | Rust 1.93, edition 2024 | ✅ 확정 | |
| 게임 엔진 | Bevy (ECS) | ✅ 확정 | wuxia-game crate |
| LLM 바인딩 | llama-cpp-2 | ✅ 확정 | wuxia-llm crate, `live-llm` feature |
| LLM 모델 | gemma3:4b Q4_K_M | ✅ 확정 | → [ADR: NPC LLM 모델 변경](adr-llm-model-4b-migration.md) |
| 벡터 DB | LanceDB | ✅ 확정 | wuxia-memory crate, `live-db` feature |
| 아키텍처 | Hexagonal Architecture + DDD | ✅ 확정 | Pure Logic Lib + Plugin + Hybrid Component |
| 직렬화 | serde + toml/json | ✅ 확정 | wuxia-data crate |

---

## 변경 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|------------|
| v0.8 | 2025-02-06 | 3원칙 + 하이브리드 Component 전략 확정 |
| v0.9 | 2025-02-06 | DomainEvent wrapper enum 구조 문서화. Cargo Workspace 상세화. |
| **v1.0** | **2026-02-16** | **6개 crate 구조 확정 (core/data/llm/memory/game/app). wuxia-core에서 toml 의존성 제거 → wuxia-data로 분리. Feature Flag 전략 신설 (§3.2). 독립 실행 범위 매트릭스 신설 (§3.3). 데이터 로딩 전략 신설 (§3.4). assets/ 폴더 구조 반영. docs/ai/ 폴더 신설. 테스트 471개 통과.** |
| **v1.1** | **2026-02-16** | **§4 기술 스택 섹션 신설 (단일 소유자). LLM 모델 Gemma→TBD 변경.** |
| **v1.2** | **2026-02-21** | **§2 원칙 4 "데이터 주도 국제화" 신설 (3→4 원칙). §3.4 확장: 3계층 데이터 주도 패턴(§3.4.1) + 적용 현황(§3.4.2) 추가. prompt_config.toml/descriptions.toml 반영.** |
| **v1.3** | **2026-02-24** | **§4 LLM 모델 TBD→gemma3:4b 확정. ADR 문서 참조 링크 추가.** |
