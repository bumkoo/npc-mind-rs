# Phase 2 설계: LLM 판정 지표 + 비교 리포트

**확정일:** 2026-02-23
**상태:** ✅ 구현 완료 (2026-02-23)
**범위:** Step 3.7.4 (LLM 채점기) + Step 3.7.5 (비교 리포트)
**전제:** Phase 1 완료 (quality/metrics.rs, runner.rs, scenario.rs)

---

## 결정 사항

| 항목 | 결정 |
|------|------|
| Judge LLM | Claude API 또는 OpenAI API (선택) |
| 기본 모델 | Claude: claude-sonnet-4-20250514 / OpenAI: gpt-4o |
| 모델 변경 | `JUDGE_MODEL` 환경변수로 변경 가능 |
| 아키텍처 | JudgePort 트레이트 분리 (LlmPort와 별도) |
| HTTP 클라이언트 | reqwest::blocking (sync) |
| 리포트 출력 | 터미널 테이블 + JSON 파일 |
| Feature gate | `claude-judge` / `openai-judge` (각각 독립) |

---

## Step 3.7.4 — LLM 채점기 (quality/judge.rs)

### 타입 정의

```rust
pub trait JudgePort: Send + Sync {
    fn judge_all(&self, ctx: &JudgeContext) -> Result<Vec<JudgeResult>, JudgeError>;
}

pub struct JudgeContext {
    pub persona_xml: String,
    pub conversation: Vec<TurnPair>,
    pub injected_memories: Vec<String>,
}

pub struct TurnPair {
    pub player: String,
    pub npc: String,
}

pub enum JudgeMetric {
    CharacterConsistency,  // 1~10점
    ContextCoherence,      // 1~10점
    HallucinationDetect,   // 건수
}

pub struct JudgeResult {
    pub metric: JudgeMetric,
    pub score: f32,
    pub reasoning: String,
}

pub enum JudgeError {
    ApiError(String),
    ParseError(String),
    RateLimited,
    MissingApiKey,
}
```

### 구현체

**ClaudeJudge** (`#[cfg(feature = "claude-judge")]`):
```rust
pub struct ClaudeJudge {
    api_key: String,
    model: String,
    client: reqwest::blocking::Client,
}

impl ClaudeJudge {
    pub fn new(api_key: String, model: String) -> Self;
    pub fn from_env() -> Result<Self, JudgeError>;
    // ANTHROPIC_API_KEY, JUDGE_MODEL (기본 claude-sonnet-4-20250514)
}
```

**OpenAiJudge** (`#[cfg(feature = "openai-judge")]`):
```rust
pub struct OpenAiJudge {
    api_key: String,
    model: String,
    client: reqwest::blocking::Client,
}

impl OpenAiJudge {
    pub fn new(api_key: String, model: String) -> Self;
    pub fn from_env() -> Result<Self, JudgeError>;
    // OPENAI_API_KEY, JUDGE_MODEL (기본 gpt-4o)
}
```

**MockJudge** (feature gate 없음, 항상 사용 가능):
```rust
pub struct MockJudge { pub fixed_score: f32 }
```

### 채점 프롬프트 (metric별 3개)

1. **character_consistency** — Persona XML + 대화 → 1~10점
2. **context_coherence** — 대화 흐름 전체 → 1~10점
3. **hallucination_detect** — Persona+Memory vs 응답 모순 → 건수

응답 포맷: `[score: N]\n[reason: 한 줄]`

### 의존성

```toml
[dependencies]
reqwest = { version = "0.12", features = ["blocking", "json"], optional = true }

[features]
claude-judge = ["dep:reqwest"]
openai-judge = ["dep:reqwest"]
```

---

## Step 3.7.5 — 비교 리포트 (quality/report.rs)

### 타입 정의

```rust
pub struct FullBenchReport {
    pub scenario_id: String,
    pub model_name: String,
    pub timestamp: String,
    pub auto_metrics: QualityReport,
    pub judge_metrics: Vec<JudgeResult>,
    pub pass: bool,
}

pub struct ComparisonReport {
    pub baseline: FullBenchReport,
    pub current: FullBenchReport,
    pub diffs: Vec<MetricDiff>,
}

pub struct MetricDiff {
    pub name: String,
    pub baseline_value: f64,
    pub current_value: f64,
    pub delta: f64,
    pub improved: bool,
}
```

### 출력 함수

```rust
pub fn print_comparison_table(report: &ComparisonReport);
pub fn save_report_json(report: &FullBenchReport, path: &Path) -> io::Result<()>;
pub fn load_report_json(path: &Path) -> io::Result<FullBenchReport>;
```

### 터미널 테이블 형식

```
╔══════════════════════╦══════════╦══════════╦═════════╗
║ 지표                 ║ baseline ║ current  ║ 변화    ║
╠══════════════════════╬══════════╬══════════╬═════════╣
║ affinity_tag_rate    ║ 45%      ║ 92%      ║ +47% ✅ ║
║ ...                  ║          ║          ║         ║
║ character_score [LLM]║ 5.2/10   ║ 8.1/10   ║ +2.9 ↑  ║
╠══════════════════════╬══════════╬══════════╬═════════╣
║ PASS/FAIL            ║ ❌ FAIL  ║ ✅ PASS  ║         ║
╚══════════════════════╩══════════╩══════════╩═════════╝
```

### 워크플로

```
run_scenario() → BenchResult (auto metrics)
      │
      ▼
judge.judge_all() → Vec<JudgeResult> (LLM metrics)
      │
      ▼
FullBenchReport { auto_metrics + judge_metrics }
      │
      ├→ print_comparison_table()  (터미널)
      └→ save_report_json()        (파일)
```

---

## 파일 위치

```
wuxia-llm/src/quality/
├── mod.rs         # 기존 — judge, report 모듈 추가
├── metrics.rs     # 기존 Phase 1
├── runner.rs      # 기존 Phase 1 — FullBenchReport 조립 로직 추가
├── scenario.rs    # 기존 Phase 1
├── judge.rs       # [신규] JudgePort + ClaudeJudge + OpenAiJudge + MockJudge
└── report.rs      # [신규] FullBenchReport + ComparisonReport + 출력
```
