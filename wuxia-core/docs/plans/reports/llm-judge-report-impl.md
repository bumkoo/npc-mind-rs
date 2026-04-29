# Phase 2: LLM 판정 지표 + 비교 리포트 구현 계획

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 별도 LLM(Claude API)으로 캐릭터 일관성/문맥 일관성/hallucination을 채점하고, 모델/프롬프트 변경 전후 품질 비교 리포트를 자동 생성한다.

**Architecture:** `JudgePort` 트레이트로 채점 로직을 추상화. `ClaudeJudge`는 Anthropic Messages API를 `reqwest::blocking`으로 호출. `MockJudge`로 unit test. `FullBenchReport`는 Phase 1 자동 지표 + Phase 2 LLM 판정을 합치고, 터미널 테이블 + JSON으로 출력.

**Tech Stack:** Rust, reqwest (blocking + json), serde/serde_json, Anthropic Messages API

**Design Doc:** `docs/plans/2026-02-23-phase2-llm-judge-report-design.md`

---

## Task 1: Cargo.toml 의존성 추가

**Files:**
- Modify: `crates/wuxia-llm/Cargo.toml`

**Step 1: 의존성과 feature 추가**

```toml
# [dependencies] 섹션에 추가
reqwest = { version = "0.12", features = ["blocking", "json"], optional = true }
serde_json = "1"

# [features] 섹션에 추가
claude-judge = ["dep:reqwest"]
```

**Step 2: 컴파일 확인**

Run: `cargo check -p wuxia-llm`
Expected: PASS (새 의존성은 optional이라 기존 빌드 영향 없음)

**Step 3: feature 활성화 확인**

Run: `cargo check -p wuxia-llm --features claude-judge`
Expected: PASS (reqwest 다운로드 + 컴파일)

**Step 4: Commit**

```bash
git add crates/wuxia-llm/Cargo.toml
git commit -m "build(wuxia-llm): add reqwest + serde_json for claude-judge feature"
```

---

## Task 2: JudgePort 트레이트 + 타입 정의

**Files:**
- Create: `crates/wuxia-llm/src/quality/judge.rs`
- Modify: `crates/wuxia-llm/src/quality/mod.rs`

**Step 1: mod.rs에 judge 모듈 등록**

`crates/wuxia-llm/src/quality/mod.rs` 에 추가:
```rust
pub mod judge;
```

**Step 2: judge.rs에 타입 정의 작성**

`crates/wuxia-llm/src/quality/judge.rs`:

```rust
// wuxia-llm/src/quality/judge.rs
//
// LLM 채점기 — Step 3.7.4.
//
// 별도 LLM으로 대화 품질을 채점한다.
// 3개 지표: 캐릭터 일관성, 문맥 일관성, hallucination 감지.
//
// 구현체:
//   ClaudeJudge — Anthropic Messages API 호출 (feature: claude-judge)
//   MockJudge  — 고정 점수 반환 (단위 테스트용)

use std::fmt;

// ---------------------------------------------------------------------------
// 트레이트
// ---------------------------------------------------------------------------

/// LLM 채점 포트.
///
/// 대화 컨텍스트를 받아 3개 지표를 채점한다.
/// ClaudeJudge(Claude API) 또는 MockJudge(테스트)로 구현.
pub trait JudgePort: Send + Sync {
    fn judge_all(&self, ctx: &JudgeContext) -> Result<Vec<JudgeResult>, JudgeError>;
}

// ---------------------------------------------------------------------------
// 요청/응답 타입
// ---------------------------------------------------------------------------

/// 채점에 필요한 대화 컨텍스트.
pub struct JudgeContext {
    /// build_system_prompt() 결과 — Persona XML 전체.
    pub persona_xml: String,
    /// 대화 턴 목록 (플레이어 + NPC 쌍).
    pub conversation: Vec<TurnPair>,
    /// 시나리오에 주입된 기억 목록.
    pub injected_memories: Vec<String>,
}

/// 대화 한 턴 (플레이어 입력 + NPC 응답).
pub struct TurnPair {
    pub player: String,
    pub npc: String,
}

/// 채점 대상 지표.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JudgeMetric {
    /// 캐릭터 설정 준수 (1~10점).
    CharacterConsistency,
    /// 대화 흐름 일관성 (1~10점).
    ContextCoherence,
    /// Persona+Memory 대비 사실 오류 (건수).
    HallucinationDetect,
}

impl fmt::Display for JudgeMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JudgeMetric::CharacterConsistency => write!(f, "character_consistency"),
            JudgeMetric::ContextCoherence => write!(f, "context_coherence"),
            JudgeMetric::HallucinationDetect => write!(f, "hallucination_detect"),
        }
    }
}

/// 단일 지표 채점 결과.
#[derive(Debug, Clone)]
pub struct JudgeResult {
    pub metric: JudgeMetric,
    /// 점수. CharacterConsistency/ContextCoherence: 1.0~10.0.
    /// HallucinationDetect: 감지 건수 (0.0 = 없음).
    pub score: f32,
    /// 채점 이유 (한 줄).
    pub reasoning: String,
}

/// 채점 오류.
#[derive(Debug)]
pub enum JudgeError {
    /// HTTP 또는 API 오류.
    ApiError(String),
    /// 응답 파싱 실패 (score/reason 추출 불가).
    ParseError(String),
    /// API rate limit 초과.
    RateLimited,
    /// ANTHROPIC_API_KEY 환경변수 미설정.
    MissingApiKey,
}

impl fmt::Display for JudgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JudgeError::ApiError(e) => write!(f, "API error: {e}"),
            JudgeError::ParseError(e) => write!(f, "Parse error: {e}"),
            JudgeError::RateLimited => write!(f, "Rate limited"),
            JudgeError::MissingApiKey => write!(f, "ANTHROPIC_API_KEY not set"),
        }
    }
}

impl std::error::Error for JudgeError {}
```

**Step 3: 컴파일 확인**

Run: `cargo check -p wuxia-llm`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/wuxia-llm/src/quality/judge.rs crates/wuxia-llm/src/quality/mod.rs
git commit -m "feat(quality): add JudgePort trait and judge types (Step 3.7.4)"
```

---

## Task 3: MockJudge 구현 + 테스트

**Files:**
- Modify: `crates/wuxia-llm/src/quality/judge.rs`

**Step 1: 실패하는 테스트 작성**

`judge.rs` 하단에 추가:

```rust
// ---------------------------------------------------------------------------
// MockJudge — 테스트용
// ---------------------------------------------------------------------------

/// 고정 점수를 반환하는 테스트용 채점기.
pub struct MockJudge {
    pub fixed_score: f32,
}

impl MockJudge {
    pub fn new(score: f32) -> Self {
        MockJudge { fixed_score: score }
    }
}

impl JudgePort for MockJudge {
    fn judge_all(&self, _ctx: &JudgeContext) -> Result<Vec<JudgeResult>, JudgeError> {
        Ok(vec![
            JudgeResult {
                metric: JudgeMetric::CharacterConsistency,
                score: self.fixed_score,
                reasoning: "mock".to_string(),
            },
            JudgeResult {
                metric: JudgeMetric::ContextCoherence,
                score: self.fixed_score,
                reasoning: "mock".to_string(),
            },
            JudgeResult {
                metric: JudgeMetric::HallucinationDetect,
                score: 0.0,
                reasoning: "mock: no hallucination".to_string(),
            },
        ])
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_context() -> JudgeContext {
        JudgeContext {
            persona_xml: "<Persona><Identity>소연</Identity></Persona>".to_string(),
            conversation: vec![
                TurnPair {
                    player: "안녕?".to_string(),
                    npc: "오~ 누구야?".to_string(),
                },
            ],
            injected_memories: vec![],
        }
    }

    #[test]
    fn mock_judge_returns_three_metrics() {
        let judge = MockJudge::new(8.0);
        let results = judge.judge_all(&sample_context()).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn mock_judge_uses_fixed_score() {
        let judge = MockJudge::new(7.5);
        let results = judge.judge_all(&sample_context()).unwrap();

        let char_score = results.iter()
            .find(|r| r.metric == JudgeMetric::CharacterConsistency)
            .unwrap();
        assert!((char_score.score - 7.5).abs() < f32::EPSILON);
    }

    #[test]
    fn mock_judge_hallucination_always_zero() {
        let judge = MockJudge::new(9.0);
        let results = judge.judge_all(&sample_context()).unwrap();

        let hall = results.iter()
            .find(|r| r.metric == JudgeMetric::HallucinationDetect)
            .unwrap();
        assert!((hall.score - 0.0).abs() < f32::EPSILON);
    }
}
```

**Step 2: 테스트 실행**

Run: `cargo test -p wuxia-llm quality::judge`
Expected: 3 tests PASS

**Step 3: Commit**

```bash
git add crates/wuxia-llm/src/quality/judge.rs
git commit -m "feat(quality): add MockJudge with tests"
```

---

## Task 4: 채점 프롬프트 모듈

**Files:**
- Modify: `crates/wuxia-llm/src/quality/judge.rs`

**Step 1: 프롬프트 빌더 함수 작성**

`judge.rs`의 MockJudge 위에 추가:

```rust
// ---------------------------------------------------------------------------
// 채점 프롬프트 빌더
// ---------------------------------------------------------------------------

/// 대화를 "[플레이어] / [NPC]" 형식으로 직렬화.
fn format_conversation(turns: &[TurnPair]) -> String {
    turns
        .iter()
        .enumerate()
        .map(|(i, t)| format!("턴 {}: 플레이어: {}\nNPC: {}", i + 1, t.player, t.npc))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// 기억 목록을 문자열로 직렬화.
fn format_memories(memories: &[String]) -> String {
    if memories.is_empty() {
        return "(없음)".to_string();
    }
    memories
        .iter()
        .enumerate()
        .map(|(i, m)| format!("{}. {}", i + 1, m))
        .collect::<Vec<_>>()
        .join("\n")
}

/// character_consistency 채점 프롬프트.
fn build_character_prompt(ctx: &JudgeContext) -> String {
    format!(
        r#"다음은 NPC의 Persona 설정과 실제 대화 기록이다.

<Persona_Setting>
{persona}
</Persona_Setting>

<Conversation>
{conv}
</Conversation>

NPC가 Persona 설정(성격, 말투, 가치관, 배경)을 얼마나 충실히 준수했는지 1~10점으로 평가하라.

반드시 다음 형식으로만 답하라:
[score: N]
[reason: 한 줄 이유]"#,
        persona = ctx.persona_xml,
        conv = format_conversation(&ctx.conversation),
    )
}

/// context_coherence 채점 프롬프트.
fn build_coherence_prompt(ctx: &JudgeContext) -> String {
    format!(
        r#"다음 대화에서 NPC의 응답이 문맥상 자연스럽고 일관성 있는지 1~10점으로 평가하라.

평가 기준:
- 플레이어의 질문/발언에 적절히 응답하는가?
- 앞뒤 턴 간 맥락이 이어지는가?
- 갑작스러운 주제 전환이나 비논리적 응답이 없는가?

<Conversation>
{conv}
</Conversation>

반드시 다음 형식으로만 답하라:
[score: N]
[reason: 한 줄 이유]"#,
        conv = format_conversation(&ctx.conversation),
    )
}

/// hallucination_detect 채점 프롬프트.
fn build_hallucination_prompt(ctx: &JudgeContext) -> String {
    format!(
        r#"다음은 NPC의 Persona 설정, 주입된 기억, 그리고 실제 대화이다.

<Persona_Setting>
{persona}
</Persona_Setting>

<Injected_Memories>
{memories}
</Injected_Memories>

<Conversation>
{conv}
</Conversation>

NPC의 응답에서 Persona 설정이나 주입된 기억과 모순되는 사실 오류(hallucination)가 몇 건인지 세어라.
예: 설정에 없는 장소를 언급, 기억과 다른 사실을 말함, 자신의 이전 발언과 모순.

반드시 다음 형식으로만 답하라:
[score: N]
[reason: 한 줄 이유]

N은 발견된 오류 건수이다 (0이면 오류 없음)."#,
        persona = ctx.persona_xml,
        memories = format_memories(&ctx.injected_memories),
        conv = format_conversation(&ctx.conversation),
    )
}

/// 지표별 채점 프롬프트를 생성한다.
fn build_judge_prompt(ctx: &JudgeContext, metric: JudgeMetric) -> String {
    match metric {
        JudgeMetric::CharacterConsistency => build_character_prompt(ctx),
        JudgeMetric::ContextCoherence => build_coherence_prompt(ctx),
        JudgeMetric::HallucinationDetect => build_hallucination_prompt(ctx),
    }
}
```

**Step 2: 응답 파서 작성**

```rust
/// LLM 채점 응답에서 [score: N]과 [reason: ...]을 추출한다.
fn parse_judge_response(text: &str, metric: JudgeMetric) -> Result<JudgeResult, JudgeError> {
    // [score: N] 추출
    let score = extract_judge_score(text)
        .ok_or_else(|| JudgeError::ParseError(
            format!("Failed to extract [score: N] from: {}", text)
        ))?;

    // [reason: ...] 추출 (없으면 빈 문자열)
    let reasoning = extract_judge_reason(text).unwrap_or_default();

    Ok(JudgeResult { metric, score, reasoning })
}

/// "[score: N]" 패턴에서 N을 추출.
fn extract_judge_score(text: &str) -> Option<f32> {
    let lower = text.to_lowercase();
    let start = lower.find("[score:")?;
    let after = &text[start + 7..]; // "[score:" 이후
    let end = after.find(']')?;
    let num_str = after[..end].trim();
    num_str.parse::<f32>().ok()
}

/// "[reason: ...]" 패턴에서 이유 문자열을 추출.
fn extract_judge_reason(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let start = lower.find("[reason:")?;
    let after = &text[start + 8..]; // "[reason:" 이후
    let end = after.find(']')?;
    Some(after[..end].trim().to_string())
}
```

**Step 3: 프롬프트 빌더 + 파서 테스트 추가**

테스트 모듈에 추가:

```rust
    #[test]
    fn build_character_prompt_includes_persona_and_conversation() {
        let ctx = sample_context();
        let prompt = build_character_prompt(&ctx);
        assert!(prompt.contains("<Persona_Setting>"));
        assert!(prompt.contains("소연"));
        assert!(prompt.contains("안녕?"));
        assert!(prompt.contains("누구야?"));
        assert!(prompt.contains("[score: N]"));
    }

    #[test]
    fn build_hallucination_prompt_includes_memories() {
        let ctx = JudgeContext {
            persona_xml: "<Persona/>".to_string(),
            conversation: vec![TurnPair {
                player: "서문 쪽 소식?".to_string(),
                npc: "수상한 사내가 있었어.".to_string(),
            }],
            injected_memories: vec!["서문에서 수상한 사내를 보았다.".to_string()],
        };
        let prompt = build_hallucination_prompt(&ctx);
        assert!(prompt.contains("서문에서 수상한 사내"));
        assert!(prompt.contains("<Injected_Memories>"));
    }

    #[test]
    fn parse_judge_response_valid() {
        let text = "[score: 8]\n[reason: Persona 설정을 잘 준수함]";
        let result = parse_judge_response(text, JudgeMetric::CharacterConsistency).unwrap();
        assert!((result.score - 8.0).abs() < f32::EPSILON);
        assert!(result.reasoning.contains("Persona"));
    }

    #[test]
    fn parse_judge_response_missing_score() {
        let text = "좋은 대화입니다.";
        let result = parse_judge_response(text, JudgeMetric::CharacterConsistency);
        assert!(result.is_err());
    }

    #[test]
    fn parse_judge_response_no_reason_ok() {
        let text = "[score: 7]";
        let result = parse_judge_response(text, JudgeMetric::ContextCoherence).unwrap();
        assert!((result.score - 7.0).abs() < f32::EPSILON);
        assert!(result.reasoning.is_empty());
    }

    #[test]
    fn extract_score_various_formats() {
        assert!((extract_judge_score("[score: 10]").unwrap() - 10.0).abs() < f32::EPSILON);
        assert!((extract_judge_score("[Score: 3]").unwrap() - 3.0).abs() < f32::EPSILON);
        assert!((extract_judge_score("[score:7]").unwrap() - 7.0).abs() < f32::EPSILON);
        assert!((extract_judge_score("[score: 8.5]").unwrap() - 8.5).abs() < f32::EPSILON);
    }
```

**Step 4: 테스트 실행**

Run: `cargo test -p wuxia-llm quality::judge`
Expected: 이전 3개 + 신규 6개 = 9 tests PASS

**Step 5: Commit**

```bash
git add crates/wuxia-llm/src/quality/judge.rs
git commit -m "feat(quality): add judge prompt builders and response parser"
```

---

## Task 5: ClaudeJudge 구현

**Files:**
- Modify: `crates/wuxia-llm/src/quality/judge.rs`

**Step 1: ClaudeJudge 구조체 + 생성자**

`judge.rs`에서 MockJudge 위에 추가 (feature gate 적용):

```rust
// ---------------------------------------------------------------------------
// ClaudeJudge — Anthropic Messages API
// ---------------------------------------------------------------------------

#[cfg(feature = "claude-judge")]
pub struct ClaudeJudge {
    api_key: String,
    model: String,
    client: reqwest::blocking::Client,
}

#[cfg(feature = "claude-judge")]
impl ClaudeJudge {
    /// API 키와 모델을 직접 지정.
    pub fn new(api_key: String, model: String) -> Self {
        ClaudeJudge {
            api_key,
            model,
            client: reqwest::blocking::Client::new(),
        }
    }

    /// 환경변수에서 생성.
    /// - `ANTHROPIC_API_KEY` (필수)
    /// - `JUDGE_MODEL` (선택, 기본: claude-sonnet-4-20250514)
    pub fn from_env() -> Result<Self, JudgeError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| JudgeError::MissingApiKey)?;
        let model = std::env::var("JUDGE_MODEL")
            .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
        Ok(Self::new(api_key, model))
    }

    /// 단일 metric 채점 요청.
    fn judge_one(
        &self,
        ctx: &JudgeContext,
        metric: JudgeMetric,
    ) -> Result<JudgeResult, JudgeError> {
        let prompt = build_judge_prompt(ctx, metric);

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 256,
            "messages": [{
                "role": "user",
                "content": prompt,
            }],
        });

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| JudgeError::ApiError(e.to_string()))?;

        if response.status() == 429 {
            return Err(JudgeError::RateLimited);
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().unwrap_or_default();
            return Err(JudgeError::ApiError(
                format!("HTTP {}: {}", status, text)
            ));
        }

        let json: serde_json::Value = response
            .json()
            .map_err(|e| JudgeError::ParseError(e.to_string()))?;

        let content_text = json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| JudgeError::ParseError(
                "Missing content[0].text in response".to_string()
            ))?;

        parse_judge_response(content_text, metric)
    }
}

#[cfg(feature = "claude-judge")]
impl JudgePort for ClaudeJudge {
    fn judge_all(&self, ctx: &JudgeContext) -> Result<Vec<JudgeResult>, JudgeError> {
        let metrics = [
            JudgeMetric::CharacterConsistency,
            JudgeMetric::ContextCoherence,
            JudgeMetric::HallucinationDetect,
        ];

        metrics
            .iter()
            .map(|&m| self.judge_one(ctx, m))
            .collect()
    }
}
```

**Step 2: serde_json import 추가**

파일 상단에 추가:
```rust
#[cfg(feature = "claude-judge")]
use serde_json;
```

**Step 3: 컴파일 확인 (feature 없이)**

Run: `cargo check -p wuxia-llm`
Expected: PASS (ClaudeJudge는 feature gate 뒤에 있음)

**Step 4: 컴파일 확인 (feature 있이)**

Run: `cargo check -p wuxia-llm --features claude-judge`
Expected: PASS

**Step 5: 기존 테스트 회귀 확인**

Run: `cargo test -p wuxia-llm quality::judge`
Expected: 9 tests PASS (MockJudge 테스트 변경 없음)

**Step 6: Commit**

```bash
git add crates/wuxia-llm/src/quality/judge.rs
git commit -m "feat(quality): add ClaudeJudge (Anthropic Messages API, feature-gated)"
```

---

## Task 6: FullBenchReport + JSON 직렬화

**Files:**
- Create: `crates/wuxia-llm/src/quality/report.rs`
- Modify: `crates/wuxia-llm/src/quality/mod.rs`

**Step 1: mod.rs에 report 모듈 등록**

```rust
pub mod report;
```

**Step 2: 실패하는 테스트 작성**

`crates/wuxia-llm/src/quality/report.rs`:

```rust
// wuxia-llm/src/quality/report.rs
//
// 비교 리포트 — Step 3.7.5.
//
// FullBenchReport: Phase 1 자동 지표 + Phase 2 LLM 판정 통합.
// ComparisonReport: 두 리포트 간 차이 계산.
// 출력: 터미널 테이블 (사람용) + JSON (기계용).

use serde::{Deserialize, Serialize};
use std::path::Path;

use super::judge::{JudgeMetric, JudgeResult};
use super::metrics::QualityReport;

// ---------------------------------------------------------------------------
// FullBenchReport
// ---------------------------------------------------------------------------

/// Phase 1 + Phase 2 통합 벤치마크 결과.
#[derive(Debug, Serialize, Deserialize)]
pub struct FullBenchReport {
    /// 시나리오 ID.
    pub scenario_id: String,
    /// 모델 이름 (예: "gemma-3-4b-it").
    pub model_name: String,
    /// 측정 시간 (ISO 8601).
    pub timestamp: String,
    /// Phase 1 자동 지표.
    pub auto_metrics: AutoMetricsSummary,
    /// Phase 2 LLM 판정 결과.
    pub judge_metrics: Vec<JudgeMetricEntry>,
    /// 전체 합격 여부.
    pub pass: bool,
}

/// Phase 1 자동 지표 요약 (직렬화 가능).
#[derive(Debug, Serialize, Deserialize)]
pub struct AutoMetricsSummary {
    pub affinity_tag_rate: f32,
    pub speech_violation_count: usize,
    pub repetition_score: f32,
    pub avg_response_length: f32,
    pub forbidden_word_count: usize,
    pub memory_utilization: f32,
}

/// LLM 판정 결과 항목 (직렬화 가능).
#[derive(Debug, Serialize, Deserialize)]
pub struct JudgeMetricEntry {
    pub metric: String,
    pub score: f32,
    pub reasoning: String,
}

// ---------------------------------------------------------------------------
// 변환 함수
// ---------------------------------------------------------------------------

/// QualityReport → AutoMetricsSummary.
pub fn summarize_auto_metrics(q: &QualityReport) -> AutoMetricsSummary {
    let avg_len = if q.response_lengths.is_empty() {
        0.0
    } else {
        q.response_lengths.iter().sum::<usize>() as f32 / q.response_lengths.len() as f32
    };

    AutoMetricsSummary {
        affinity_tag_rate: q.affinity_tag_rate,
        speech_violation_count: q.speech_violations.len(),
        repetition_score: q.repetition_score,
        avg_response_length: avg_len,
        forbidden_word_count: q.forbidden_word_leaks.len(),
        memory_utilization: q.memory_utilization,
    }
}

/// JudgeResult → JudgeMetricEntry (직렬화용).
pub fn to_judge_entry(r: &JudgeResult) -> JudgeMetricEntry {
    JudgeMetricEntry {
        metric: r.metric.to_string(),
        score: r.score,
        reasoning: r.reasoning.clone(),
    }
}

/// 합격 여부 판정.
///
/// 기준:
///   - affinity_tag_rate ≥ 0.9
///   - speech_violation_count == 0
///   - repetition_score ≤ 0.2
///   - forbidden_word_count == 0
///   - character_consistency ≥ 7.0
///   - context_coherence ≥ 7.0
///   - hallucination_detect == 0
pub fn check_pass(auto: &AutoMetricsSummary, judge: &[JudgeMetricEntry]) -> bool {
    let auto_pass = auto.affinity_tag_rate >= 0.9
        && auto.speech_violation_count == 0
        && auto.repetition_score <= 0.2
        && auto.forbidden_word_count == 0;

    let judge_pass = judge.iter().all(|j| match j.metric.as_str() {
        "character_consistency" => j.score >= 7.0,
        "context_coherence" => j.score >= 7.0,
        "hallucination_detect" => j.score < 1.0,
        _ => true,
    });

    auto_pass && judge_pass
}

/// FullBenchReport 조립.
pub fn build_full_report(
    scenario_id: &str,
    model_name: &str,
    quality: &QualityReport,
    judge_results: &[JudgeResult],
) -> FullBenchReport {
    let auto_metrics = summarize_auto_metrics(quality);
    let judge_metrics: Vec<JudgeMetricEntry> =
        judge_results.iter().map(to_judge_entry).collect();
    let pass = check_pass(&auto_metrics, &judge_metrics);

    FullBenchReport {
        scenario_id: scenario_id.to_string(),
        model_name: model_name.to_string(),
        timestamp: now_iso8601(),
        auto_metrics,
        judge_metrics,
        pass,
    }
}

fn now_iso8601() -> String {
    // 간단한 구현: 정확한 시간이 필요하면 chrono를 추가하되,
    // 현재는 placeholder.
    "2026-01-01T00:00:00Z".to_string()
}

// ---------------------------------------------------------------------------
// JSON I/O
// ---------------------------------------------------------------------------

/// JSON 파일로 저장.
pub fn save_report_json(report: &FullBenchReport, path: &Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(report)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, json)
}

/// JSON 파일에서 로딩.
pub fn load_report_json(path: &Path) -> std::io::Result<FullBenchReport> {
    let content = std::fs::read_to_string(path)?;
    serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::metrics::{QualityReport, Violation, ForbiddenWordLeak};

    fn good_quality() -> QualityReport {
        QualityReport {
            affinity_tag_rate: 0.95,
            speech_violations: vec![],
            repetition_score: 0.1,
            response_lengths: vec![2, 2, 3],
            forbidden_word_leaks: vec![],
            memory_utilization: 0.6,
        }
    }

    fn bad_quality() -> QualityReport {
        QualityReport {
            affinity_tag_rate: 0.4,
            speech_violations: vec![Violation {
                turn: 0,
                matched_pattern: "습니다".to_string(),
                context: "반갑습니다.".to_string(),
            }],
            repetition_score: 0.35,
            response_lengths: vec![5, 6],
            forbidden_word_leaks: vec![ForbiddenWordLeak {
                turn: 1,
                word: "납치".to_string(),
                context: "납치당했다.".to_string(),
            }],
            memory_utilization: 0.2,
        }
    }

    fn good_judge() -> Vec<JudgeResult> {
        vec![
            JudgeResult {
                metric: JudgeMetric::CharacterConsistency,
                score: 8.5,
                reasoning: "잘 맞음".to_string(),
            },
            JudgeResult {
                metric: JudgeMetric::ContextCoherence,
                score: 7.8,
                reasoning: "자연스러움".to_string(),
            },
            JudgeResult {
                metric: JudgeMetric::HallucinationDetect,
                score: 0.0,
                reasoning: "오류 없음".to_string(),
            },
        ]
    }

    #[test]
    fn full_report_passes_with_good_data() {
        let report = build_full_report("test", "4b", &good_quality(), &good_judge());
        assert!(report.pass);
    }

    #[test]
    fn full_report_fails_with_bad_auto_metrics() {
        let report = build_full_report("test", "4b", &bad_quality(), &good_judge());
        assert!(!report.pass);
    }

    #[test]
    fn full_report_fails_with_bad_judge() {
        let bad_judge = vec![JudgeResult {
            metric: JudgeMetric::CharacterConsistency,
            score: 4.0,
            reasoning: "설정 위반".to_string(),
        }];
        let report = build_full_report("test", "4b", &good_quality(), &bad_judge);
        assert!(!report.pass);
    }

    #[test]
    fn json_round_trip() {
        let report = build_full_report("test", "4b", &good_quality(), &good_judge());
        let tmp = std::env::temp_dir().join("wuxia_test_report.json");
        save_report_json(&report, &tmp).unwrap();
        let loaded = load_report_json(&tmp).unwrap();
        assert_eq!(loaded.scenario_id, "test");
        assert_eq!(loaded.model_name, "4b");
        assert!(loaded.pass);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn summarize_auto_metrics_avg_length() {
        let q = good_quality();
        let summary = summarize_auto_metrics(&q);
        // (2 + 2 + 3) / 3 = 2.333...
        assert!((summary.avg_response_length - 2.333).abs() < 0.01);
    }
}
```

**Step 3: 테스트 실행**

Run: `cargo test -p wuxia-llm quality::report`
Expected: 5 tests PASS

**Step 4: Commit**

```bash
git add crates/wuxia-llm/src/quality/report.rs crates/wuxia-llm/src/quality/mod.rs
git commit -m "feat(quality): add FullBenchReport + JSON I/O (Step 3.7.5)"
```

---

## Task 7: ComparisonReport + 터미널 테이블 출력

**Files:**
- Modify: `crates/wuxia-llm/src/quality/report.rs`

**Step 1: ComparisonReport 타입 추가**

```rust
// ---------------------------------------------------------------------------
// ComparisonReport — 두 리포트 비교
// ---------------------------------------------------------------------------

/// 두 FullBenchReport 간 지표별 차이.
#[derive(Debug)]
pub struct ComparisonReport {
    pub baseline_model: String,
    pub current_model: String,
    pub diffs: Vec<MetricDiff>,
    pub baseline_pass: bool,
    pub current_pass: bool,
}

/// 단일 지표의 변화.
#[derive(Debug)]
pub struct MetricDiff {
    /// 지표 이름 (예: "affinity_tag_rate").
    pub name: String,
    /// baseline 값.
    pub baseline_value: f64,
    /// current 값.
    pub current_value: f64,
    /// 변화량 (current - baseline).
    pub delta: f64,
    /// 개선 여부 (true = 좋아짐).
    pub improved: bool,
}

/// 두 리포트를 비교한다.
pub fn compare_reports(
    baseline: &FullBenchReport,
    current: &FullBenchReport,
) -> ComparisonReport {
    let mut diffs = Vec::new();

    // Auto metrics
    let b = &baseline.auto_metrics;
    let c = &current.auto_metrics;

    diffs.push(make_diff("affinity_tag_rate", b.affinity_tag_rate as f64, c.affinity_tag_rate as f64, true));
    diffs.push(make_diff("speech_violation", b.speech_violation_count as f64, c.speech_violation_count as f64, false));
    diffs.push(make_diff("repetition_score", b.repetition_score as f64, c.repetition_score as f64, false));
    diffs.push(make_diff("forbidden_word", b.forbidden_word_count as f64, c.forbidden_word_count as f64, false));
    diffs.push(make_diff("memory_utilization", b.memory_utilization as f64, c.memory_utilization as f64, true));

    // Judge metrics
    for bj in &baseline.judge_metrics {
        if let Some(cj) = current.judge_metrics.iter().find(|j| j.metric == bj.metric) {
            let higher_better = bj.metric != "hallucination_detect";
            diffs.push(make_diff(&bj.metric, bj.score as f64, cj.score as f64, higher_better));
        }
    }

    ComparisonReport {
        baseline_model: baseline.model_name.clone(),
        current_model: current.model_name.clone(),
        diffs,
        baseline_pass: baseline.pass,
        current_pass: current.pass,
    }
}

fn make_diff(name: &str, baseline: f64, current: f64, higher_is_better: bool) -> MetricDiff {
    let delta = current - baseline;
    let improved = if higher_is_better { delta > 0.0 } else { delta < 0.0 };
    MetricDiff {
        name: name.to_string(),
        baseline_value: baseline,
        current_value: current,
        delta,
        improved,
    }
}
```

**Step 2: 터미널 테이블 출력**

```rust
// ---------------------------------------------------------------------------
// 터미널 출력
// ---------------------------------------------------------------------------

/// 비교 리포트를 터미널 테이블로 출력.
pub fn print_comparison_table(report: &ComparisonReport) {
    let w_name = 22;
    let w_val = 10;

    // 헤더
    println!("╔{:═<w_name$}╦{:═<w_val$}╦{:═<w_val$}╦{:═<w_val$}╗",
        "", "", "", "");
    println!("║{:<w_name$}║{:^w_val$}║{:^w_val$}║{:^w_val$}║",
        " 지표", &report.baseline_model, &report.current_model, "변화");
    println!("╠{:═<w_name$}╬{:═<w_val$}╬{:═<w_val$}╬{:═<w_val$}╣",
        "", "", "", "");

    // 행
    for diff in &report.diffs {
        let b_str = format_value(&diff.name, diff.baseline_value);
        let c_str = format_value(&diff.name, diff.current_value);
        let d_str = format_delta(diff);

        println!("║ {:<w$}║{:^w_val$}║{:^w_val$}║{:^w_val$}║",
            diff.name, b_str, c_str, d_str, w = w_name - 1);
    }

    // PASS/FAIL
    println!("╠{:═<w_name$}╬{:═<w_val$}╬{:═<w_val$}╬{:═<w_val$}╣",
        "", "", "", "");
    let b_pass = if report.baseline_pass { "PASS" } else { "FAIL" };
    let c_pass = if report.current_pass { "PASS" } else { "FAIL" };
    println!("║ {:<w$}║{:^w_val$}║{:^w_val$}║{:^w_val$}║",
        "RESULT", b_pass, c_pass, "", w = w_name - 1);
    println!("╚{:═<w_name$}╩{:═<w_val$}╩{:═<w_val$}╩{:═<w_val$}╝",
        "", "", "", "");
}

fn format_value(name: &str, value: f64) -> String {
    match name {
        "affinity_tag_rate" | "repetition_score" | "memory_utilization" => {
            format!("{:.0}%", value * 100.0)
        }
        "speech_violation" | "forbidden_word" | "hallucination_detect" => {
            format!("{}건", value as i64)
        }
        _ => format!("{:.1}", value),
    }
}

fn format_delta(diff: &MetricDiff) -> String {
    if diff.delta.abs() < 0.001 {
        return "—".to_string();
    }

    let sign = if diff.delta > 0.0 { "+" } else { "" };
    let icon = if diff.improved { " ✅" } else { " ↓" };

    match diff.name.as_str() {
        "affinity_tag_rate" | "repetition_score" | "memory_utilization" => {
            format!("{}{:.0}%{}", sign, diff.delta * 100.0, icon)
        }
        "speech_violation" | "forbidden_word" | "hallucination_detect" => {
            format!("{}{}{}", sign, diff.delta as i64, icon)
        }
        _ => format!("{}{:.1}{}", sign, diff.delta, icon),
    }
}
```

**Step 3: 비교 테스트 추가**

```rust
    #[test]
    fn compare_detects_improvement() {
        let baseline = build_full_report("test", "4b", &bad_quality(), &good_judge());
        let current = build_full_report("test", "12b", &good_quality(), &good_judge());
        let comparison = compare_reports(&baseline, &current);

        let tag_diff = comparison.diffs.iter()
            .find(|d| d.name == "affinity_tag_rate")
            .unwrap();
        assert!(tag_diff.improved);
        assert!(tag_diff.delta > 0.0);
    }

    #[test]
    fn compare_detects_regression() {
        let baseline = build_full_report("test", "12b", &good_quality(), &good_judge());
        let current = build_full_report("test", "4b", &bad_quality(), &good_judge());
        let comparison = compare_reports(&baseline, &current);

        let tag_diff = comparison.diffs.iter()
            .find(|d| d.name == "affinity_tag_rate")
            .unwrap();
        assert!(!tag_diff.improved);
    }
```

**Step 4: 테스트 실행**

Run: `cargo test -p wuxia-llm quality::report`
Expected: 이전 5개 + 신규 2개 = 7 tests PASS

**Step 5: Commit**

```bash
git add crates/wuxia-llm/src/quality/report.rs
git commit -m "feat(quality): add ComparisonReport + terminal table output"
```

---

## Task 8: Runner 통합 — BenchResult → FullBenchReport 워크플로

**Files:**
- Modify: `crates/wuxia-llm/src/quality/runner.rs`

**Step 1: run_full_bench 함수 추가**

`runner.rs` 하단 (tests 위)에 추가:

```rust
use super::judge::{JudgePort, JudgeContext, JudgeError, TurnPair};
use super::report::{self, FullBenchReport};

/// 시나리오 실행 + LLM 채점 + FullBenchReport 조립.
///
/// Phase 1 (자동 지표) + Phase 2 (LLM 판정) 전체 워크플로.
pub fn run_full_bench<L: LlmPort, J: JudgePort>(
    scenario: &Scenario,
    llm: &L,
    judge: &J,
    prompt_data: &CharacterPromptData,
    speech: &SpeechRules,
    memories: Vec<String>,
    config: ConversationConfig,
    model_name: &str,
) -> Result<FullBenchReport, BenchError> {
    // 1. 시나리오 실행 (Phase 1)
    let bench = run_scenario(scenario, llm, prompt_data, speech, memories.clone(), config)
        .map_err(BenchError::Llm)?;

    // 2. JudgeContext 조립
    let persona_xml = build_system_prompt(
        prompt_data,
        speech,
        &Locale::Ko,
        &PromptContext {
            memories: memories.clone(),
            conversation_summaries: vec![],
            ..Default::default()
        },
        &default_prompt_config(),
    );

    let conversation: Vec<TurnPair> = scenario.turns.iter()
        .zip(bench.replies.iter())
        .map(|(spec, reply)| TurnPair {
            player: spec.player.clone(),
            npc: reply.npc_text.clone(),
        })
        .collect();

    let ctx = JudgeContext {
        persona_xml,
        conversation,
        injected_memories: memories,
    };

    // 3. LLM 채점 (Phase 2)
    let judge_results = judge.judge_all(&ctx).map_err(BenchError::Judge)?;

    // 4. FullBenchReport 조립
    Ok(report::build_full_report(
        &bench.scenario_id,
        model_name,
        &bench.quality,
        &judge_results,
    ))
}

/// 전체 벤치마크 오류.
#[derive(Debug)]
pub enum BenchError {
    Llm(LlmError),
    Judge(JudgeError),
}

impl std::fmt::Display for BenchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BenchError::Llm(e) => write!(f, "LLM error: {e}"),
            BenchError::Judge(e) => write!(f, "Judge error: {e}"),
        }
    }
}
```

**Step 2: 통합 테스트 추가**

`runner.rs` 테스트 모듈에:

```rust
    use super::super::judge::MockJudge;

    #[test]
    fn full_bench_with_mock() {
        let scenario = greeting_scenario();
        let llm = MockLlm::fixed("반가워~ [affinity: +1]");
        let judge = MockJudge::new(8.0);
        let prompt_data = soyeon_prompt_data();
        let speech = soyeon_speech_ko();

        let report = run_full_bench(
            &scenario,
            &llm,
            &judge,
            &prompt_data,
            &speech,
            vec![],
            ConversationConfig::default(),
            "mock-test",
        )
        .unwrap();

        assert_eq!(report.scenario_id, "test_greeting");
        assert_eq!(report.model_name, "mock-test");
        assert_eq!(report.judge_metrics.len(), 3);
        assert!(report.pass); // good auto + good judge = pass
    }
```

**Step 3: 테스트 실행**

Run: `cargo test -p wuxia-llm quality::runner::tests::full_bench`
Expected: PASS

**Step 4: 전체 테스트 회귀 확인**

Run: `cargo test -p wuxia-llm`
Expected: 모든 기존 + 신규 테스트 PASS

**Step 5: Commit**

```bash
git add crates/wuxia-llm/src/quality/runner.rs
git commit -m "feat(quality): add run_full_bench integrating judge + report"
```

---

## Task 9: 공개 API 정리 + clippy + fmt

**Files:**
- Modify: `crates/wuxia-llm/src/quality/mod.rs`

**Step 1: mod.rs에서 주요 타입 re-export**

```rust
pub mod judge;
pub mod metrics;
pub mod report;
pub mod runner;
pub mod scenario;

// 편의 re-export
pub use judge::{JudgePort, JudgeContext, JudgeResult, JudgeMetric, JudgeError, MockJudge};
pub use report::{FullBenchReport, ComparisonReport, compare_reports, print_comparison_table};
pub use runner::{BenchResult, BenchError, run_full_bench};
```

**Step 2: cargo fmt**

Run: `cargo fmt -p wuxia-llm`

**Step 3: cargo clippy**

Run: `cargo clippy -p wuxia-llm -- -D warnings`
Expected: 0 warnings. 발견 시 수정.

**Step 4: 전체 테스트**

Run: `cargo test -p wuxia-llm`
Expected: 모든 테스트 PASS

**Step 5: Commit**

```bash
git add crates/wuxia-llm/src/quality/
git commit -m "refactor(quality): clean up Phase 2 public API, fmt + clippy"
```

---

## Task 10: 설계 문서 업데이트 + 최종 커밋

**Files:**
- Modify: `docs/plans/2026-02-23-phase2-llm-judge-report-design.md`

**Step 1: 설계 문서에 "구현 완료" 상태 추가**

파일 상단에:
```markdown
**상태:** ✅ 구현 완료 (2026-02-23)
```

**Step 2: Commit**

```bash
git add docs/plans/
git commit -m "docs: mark Phase 2 design as implemented"
```

---

## 요약

| Task | 내용 | 파일 |
|:----:|------|------|
| 1 | Cargo.toml 의존성 | `Cargo.toml` |
| 2 | JudgePort 트레이트 + 타입 | `judge.rs`, `mod.rs` |
| 3 | MockJudge + 테스트 | `judge.rs` |
| 4 | 채점 프롬프트 + 파서 | `judge.rs` |
| 5 | ClaudeJudge 구현 | `judge.rs` |
| 6 | FullBenchReport + JSON | `report.rs`, `mod.rs` |
| 7 | ComparisonReport + 터미널 | `report.rs` |
| 8 | Runner 통합 | `runner.rs` |
| 9 | API 정리 + clippy + fmt | `mod.rs` |
| 10 | 문서 업데이트 | `docs/plans/` |
