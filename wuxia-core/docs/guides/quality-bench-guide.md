# 대화 품질 벤치마크 운영 가이드

**대상:** Step 3.7 대화 품질 측정 체계 (Phase 1 + Phase 2 + 상세 추적 + 리플레이)
**최종 수정:** 2026-02-24 v2.0

---

## 1. 개요

대화 품질 벤치마크는 NPC 대화의 품질을 자동으로 측정하고 비교하는 도구다.

```
시나리오 TOML → 대화 실행 → 자동 지표 (Phase 1) + LLM 판정 (Phase 2) → 리포트
                              │
                              └── --detailed 모드 시:
                                  턴별 추적 데이터 수집 (SessionTrace → TurnTrace)
                                  → 상세 JSON → --replay로 재생
```

### Phase 1: 자동 지표 (6개)

| 지표 | 설명 | 합격 기준 |
|------|------|----------|
| affinity_tag_rate | `[affinity: N]` 태그 출력 비율 | >= 90% |
| speech_violations | 말투 규칙 위반 건수 | 0건 |
| repetition_score | 응답 간 반복도 (0~1) | <= 0.2 |
| response_lengths | 턴별 응답 길이 (문장 수) | 참고용 |
| forbidden_word_leaks | 금지어 노출 건수 | 0건 |
| memory_utilization | 주입된 기억의 응답 반영률 | 참고용 |

### Phase 2: LLM 판정 (3개)

| 지표 | 설명 | 합격 기준 |
|------|------|----------|
| character_consistency | NPC가 페르소나를 준수하는지 (0~10점) | >= 7.0 |
| context_coherence | 대화 흐름이 자연스러운지 (0~10점) | >= 7.0 |
| hallucination_detect | 사실 오류(환각) 건수 (0~10) | < 1.0 |

### 상세 추적 (--detailed 모드)

`--detailed` 플래그를 사용하면 턴별 파이프라인 전체를 추적한다:

| 추적 데이터 | 설명 |
|------------|------|
| TurnTrace | 턴별 LLM 입출력, 파싱 결과, 에러 정보 |
| MemorySearchTrace | 턴별 기억 검색 쿼리, 결과, 통과/탈락 |
| TimingTrace | 턴별 LLM 생성 시간, 토큰 수, tok/s |
| TurnMetrics | 턴별 개별 품질 지표 (tag, 위반, 반복도, 기억 반영) |

---

## 2. 빌드

### 기본 빌드 (Mock만 사용, API 불필요)

```bash
cargo build -p wuxia-llm
cargo test -p wuxia-llm
```

### Claude Judge 활성화

```bash
cargo build -p wuxia-llm --features claude-judge
```

### OpenAI Judge 활성화

```bash
cargo build -p wuxia-llm --features openai-judge
```

### 둘 다 활성화

```bash
cargo build -p wuxia-llm --features claude-judge --features openai-judge
```

### 벤치마크 CLI 빌드 (conversation_bench)

```bash
cargo build -p wuxia-app --example conversation_bench --features quality-bench
```

> `quality-bench` feature = `live-llm` + `openai-judge` + `serde` + `serde_json`

---

## 2.1. conversation_bench CLI 실행

### CLI 옵션

| 옵션 | 설명 | 기본값 |
|------|------|--------|
| `--model <path>` | GGUF 모델 파일 경로 | `models/gemma-3-4b-it-Q4_K_M.gguf` |
| `--model-name <name>` | 리포트에 기록할 모델 이름 | 파일명에서 자동 추출 |
| `--scenarios <dir>` | 시나리오 TOML 디렉터리 | `assets/test/scenarios/` |
| `--output <dir>` | 리포트 JSON 저장 디렉터리 | `data/bench_reports/` |
| `--judge <mock\|openai>` | Judge 종류 | `mock` |
| `--api-key <key>` | OpenAI API 키 (환경변수 대체) | `OPENAI_API_KEY` 환경변수 |
| `--baseline <path>` | 비교할 기준선 JSON 파일 경로 | - |
| `--mock` | MockLlm으로 파이프라인만 검증 (LLM 불필요) | - |
| `--detailed` | 상세 추적 활성화 (SessionTrace/TurnTrace 수집) | OFF |
| `--replay <path>` | 저장된 상세 리포트를 터미널에서 재생 | - |

### 실행 예시

#### 1단계: Mock 모드 — 파이프라인 검증 (LLM 불필요, API 불필요)

```bash
cargo run -p wuxia-app --example conversation_bench --features quality-bench -- --mock
```

> MockLlm + MockJudge로 시나리오 로딩 → 실행 → 리포트 저장까지 파이프라인이 정상 동작하는지 검증한다.
> API 키나 GGUF 모델 없이도 실행 가능.

#### 2단계: 실제 LLM + MockJudge — Phase 1 자동 지표만 측정

```bash
# 기본 모델 (gemma-3-4b)
cargo run -p wuxia-app --example conversation_bench --features quality-bench

# 12b 모델 지정
cargo run -p wuxia-app --example conversation_bench --features quality-bench -- \
  --model models/gemma-3-12b-it-Q3_K_M.gguf \
  --model-name gemma-3-12b-Q3KM
```

> 실제 LLM으로 대화를 생성하되, Judge는 Mock이므로 Phase 2 점수는 의미 없다.
> Phase 1 자동 지표 6개 (tag_rate, speech, repetition, forbidden, memory, length)만 유효.

#### 3단계: 실제 LLM + OpenAI Judge — Phase 1 + Phase 2 전체 측정

```bash
# --api-key로 직접 전달
cargo run -p wuxia-app --example conversation_bench --features quality-bench -- \
  --judge openai \
  --api-key sk-proj-...

# 환경변수 사용 (Windows cmd)
set OPENAI_API_KEY=sk-proj-...
cargo run -p wuxia-app --example conversation_bench --features quality-bench -- \
  --judge openai

# 환경변수 사용 (Windows PowerShell)
$env:OPENAI_API_KEY = "sk-proj-..."
cargo run -p wuxia-app --example conversation_bench --features quality-bench -- `
  --judge openai
```

> Phase 1 자동 지표 6개 + Phase 2 LLM 판정 3개 (character_consistency, context_coherence, hallucination_detect) 모두 측정.
> 시나리오당 OpenAI API 호출 3회, 비용 ~$0.01.

#### 4단계: 모델 비교 — 기준선 대비 비교

```bash
# 1) 4b 기준선 측정
cargo run -p wuxia-app --example conversation_bench --features quality-bench -- \
  --judge openai --api-key sk-proj-...

# 2) 12b로 재측정, 4b 결과와 비교
cargo run -p wuxia-app --example conversation_bench --features quality-bench -- \
  --model models/gemma-3-12b-it-Q3_K_M.gguf \
  --model-name gemma-3-12b-Q3KM \
  --judge openai --api-key sk-proj-... \
  --baseline data/bench_reports/gemma-3-4b-it-Q4_K_M_greeting_basic.json
```

> `--baseline` 옵션으로 이전 리포트 JSON을 지정하면, 같은 scenario_id의 결과를 비교 테이블로 출력한다.

#### 5단계: 상세 추적 — 턴별 파이프라인 디버깅

```bash
# Mock 모드 + 상세 추적
cargo run -p wuxia-app --example conversation_bench --features quality-bench -- \
  --mock --detailed

# 실제 LLM + 상세 추적
cargo run -p wuxia-app --example conversation_bench --features quality-bench -- \
  --detailed \
  --model models/gemma-3-12b-it-Q3_K_M.gguf
```

> `--detailed` 플래그를 추가하면 리포트에 `session_trace`가 포함된다.
> 턴별 LLM 입출력, 기억 검색, 파싱 결과, 타이밍, 품질 지표를 모두 기록.
> 리포트 파일 크기: 3턴 ~15-25KB, 10턴 ~50-80KB.

#### 6단계: 리플레이 — 상세 리포트 터미널 재생

```bash
cargo run -p wuxia-app --example conversation_bench --features quality-bench -- \
  --replay data/bench_reports/gemma-3-12b-Q3KM_greeting_basic.json
```

> `--replay`는 저장된 상세 리포트를 터미널에서 사람이 읽기 쉬운 형태로 출력한다.
> `--replay` 사용 시 다른 옵션은 무시되고 리포트 재생 후 즉시 종료.
> `session_trace`가 없는 리포트는 요약만 출력하고 경고 메시지를 표시.

### 출력 디렉터리 구조

```
data/bench_reports/
├── gemma-3-4b-it-Q4_K_M_greeting_basic.json
├── gemma-3-4b-it-Q4_K_M_info_request.json
├── gemma-3-4b-it-Q4_K_M_long_chat_10turns.json
├── gemma-3-12b-Q3KM_greeting_basic.json
├── gemma-3-12b-Q3KM_info_request.json
└── gemma-3-12b-Q3KM_long_chat_10turns.json
```

> 파일명: `{model_name}_{scenario_id}.json`
> `--detailed` 모드의 리포트도 같은 위치에 저장되며, `session_trace` 필드가 추가된다.

---

## 3. 환경변수

| 변수 | 필수 | 설명 | 기본값 |
|------|:----:|------|--------|
| `ANTHROPIC_API_KEY` | Claude 사용 시 | Anthropic API 키 | - |
| `OPENAI_API_KEY` | OpenAI 사용 시 | OpenAI API 키 | - |
| `JUDGE_MODEL` | 선택 | 채점에 사용할 모델명 | Claude: `claude-sonnet-4-20250514`, OpenAI: `gpt-4o` |

### 환경변수 설정 예시

```bash
# Claude 사용
export ANTHROPIC_API_KEY="sk-ant-..."
export JUDGE_MODEL="claude-sonnet-4-20250514"   # 선택

# OpenAI 사용
export OPENAI_API_KEY="sk-..."
export JUDGE_MODEL="gpt-4o"                     # 선택
```

Windows (cmd):
```cmd
set ANTHROPIC_API_KEY=sk-ant-...
set JUDGE_MODEL=claude-sonnet-4-20250514
```

---

## 4. 코드에서 사용하기

### MockJudge (테스트용, API 불필요)

```rust
use wuxia_llm::quality::{MockJudge, BenchConfig, run_full_bench};

let judge = MockJudge::new(8.0);  // 고정 점수
let config = BenchConfig::new("mock");
let report = run_full_bench(&scenario, &llm, &judge, &prompt_data, &speech, memories, &config)?;
```

### BenchConfig 설정

```rust
use wuxia_llm::quality::BenchConfig;
use wuxia_llm::conversation::ConversationConfig;

// 기본 설정 (detailed=false)
let config = BenchConfig::new("model-name");

// 상세 추적 활성화
let config = BenchConfig {
    conversation: ConversationConfig::default(),
    model_name: "model-name".to_string(),
    detailed: true,  // SessionTrace/TurnTrace 수집
};
```

### ClaudeJudge (feature: claude-judge)

```rust
use wuxia_llm::quality::ClaudeJudge;

// 환경변수에서 자동 생성
let judge = ClaudeJudge::from_env()?;

// 또는 직접 지정
let judge = ClaudeJudge::new(
    "sk-ant-...".to_string(),
    "claude-sonnet-4-20250514".to_string(),
);
```

### OpenAiJudge (feature: openai-judge)

```rust
use wuxia_llm::quality::OpenAiJudge;

// 환경변수에서 자동 생성
let judge = OpenAiJudge::from_env()?;

// 또는 직접 지정
let judge = OpenAiJudge::new(
    "sk-...".to_string(),
    "gpt-4o".to_string(),
);
```

### 전체 워크플로 (벤치마크 → 저장 → 비교 → 리플레이)

```rust
use wuxia_llm::quality::{
    BenchConfig, run_full_bench, compare_reports, print_comparison_table, print_replay,
};
use wuxia_llm::quality::report::{save_report_json, load_report_json};

// 1. 벤치마크 실행 (상세 추적 포함)
let config = BenchConfig {
    conversation: ConversationConfig::default(),
    model_name: "gemma-3-12b".to_string(),
    detailed: true,
};
let report = run_full_bench(
    &scenario, &llm, &judge,
    &prompt_data, &speech,
    memories, &config,
)?;

// 2. JSON 저장
save_report_json(&report, Path::new("report_v1.json"))?;

// 3. 이전 결과와 비교
let baseline = load_report_json(Path::new("report_v0.json"))?;
let comparison = compare_reports(&baseline, &report);
print_comparison_table(&comparison);

// 4. 상세 리포트 터미널 재생
let mut stdout = std::io::stdout();
print_replay(&report, &mut stdout)?;
```

---

## 5. 시나리오 작성

시나리오는 TOML 파일로 정의한다. 위치: `assets/test/scenarios/`

```toml
[scenario]
id = "greeting_basic"
name = "기본 인사"
description = "첫 만남 인사 + 간단한 질문"
tags = ["basic", "greeting"]

[[turns]]
player = "안녕?"
expect_keywords = ["안녕"]
expect_affinity = [0, 3]
expect_style = "반말"

[[turns]]
player = "여기가 천이방이야?"
expect_keywords = ["천이방"]
expect_affinity = [-1, 2]
expect_style = "반말"

[memories]
items = ["(3일 전) 서문에서 수상한 사내를 보았다."]
```

### 필드 설명

| 필드 | 필수 | 설명 |
|------|:----:|------|
| `scenario.id` | O | 고유 식별자 |
| `scenario.name` | O | 한국어 이름 |
| `scenario.description` | O | 설명 |
| `scenario.tags` | X | 분류 태그 |
| `turns[].player` | O | 플레이어 입력 |
| `turns[].expect_keywords` | X | NPC 응답에 포함될 키워드 |
| `turns[].expect_affinity` | X | 호감도 변화 범위 [min, max] |
| `turns[].expect_style` | X | 기대 말투 ("반말" / "존댓말") |
| `memories.items` | X | 주입할 기억 목록 |

---

## 6. 리포트 해석

### 6.1 비교 테이블 (터미널 출력)

```
┌──────────────────────┬──────────┬──────────┬─────────┐
│ 지표                 │ 4b       │ 12b      │ 변화    │
├──────────────────────┼──────────┼──────────┼─────────┤
│ affinity_tag_rate    │ 45%      │ 92%      │ +47% ✅ │
│ speech_violation     │ 2건      │ 0건      │ -2 ✅   │
│ repetition_score     │ 35%      │ 10%      │ -25% ✅ │
│ character_consistency│ 5.2      │ 8.1      │ +2.9 ✅ │
│ context_coherence    │ 6.0      │ 7.8      │ +1.8 ✅ │
│ hallucination_detect │ 3.0      │ 0.0      │ -3.0 ✅ │
├──────────────────────┼──────────┼──────────┼─────────┤
│ PASS/FAIL            │ FAIL     │ PASS     │         │
└──────────────────────┴──────────┴──────────┴─────────┘
```

### 6.2 리플레이 출력 (--replay)

`--replay` 명령으로 상세 리포트를 터미널에서 재생한 출력:

```
╔══════════════════════════════════════════════╗
║  대화 벤치마크 리플레이                        ║
╚══════════════════════════════════════════════╝

  시나리오: greeting_basic
  모델:     gemma-3-12b
  시간:     2026-02-24T16:00:00Z
  결과:     ✅ PASS

  ── 자동 지표 ──
  tag_rate: 100%  speech: 0건  repetition: 0.12  forbidden: 0건  memory: 50%

  ── Judge 채점 ──
  character_consistency: 8.1
  context_coherence: 7.5

  ── 시스템 프롬프트 (초기) ──
  너는 소연이다. 자유도시 천이방의 주인...
  ... (1200자 중 200자 표시)

  ── 주입 기억 (2건) ──
  1. (3일 전) 서문에서 수상한 사내를 보았다.
  2. 혈교 교주가 남궁가를 습격했다는 소문이 돈다.

  ══════════════════════════════════════════════
  턴별 상세 (3턴)
  ══════════════════════════════════════════════

  Turn 0 ────────────────────────────────────────
  Player: "안녕?"

  [기억 검색] query="안녕?" → 2건, 1건 통과 (mode: injected)
    ├── "서문에서 수상한 사내를..." (score=0.72) ✅
    └── "만두를 먹었다..." (score=0.58) 탈락

  [LLM 응답 원문]
  "흥, 또 누구야. 자유도시에서 뭔 볼일이야? [affinity: 0]"

  [파싱]
    text: "흥, 또 누구야. 자유도시에서 뭔 볼일이야?"
    affinity: 0 ✅  forbidden: 0 ✅

  [지표] tag=✅  violation=✅  repetition=0.00✅  length=2  memory=✅
  [성능] TTFT=850ms, total=2100ms, 23tok, 11.0tok/s

  Turn 1 ──────────────────────────── ⚠ ERROR
  Player: "그 이야기 더 해줘"

  [에러] LlmTimeout: "30초 타임아웃 초과"
         partial_data: false
```

> 에러 턴은 `⚠ ERROR` 마커로 표시되며, 에러 종류(LlmTimeout, LlmError, ParseFail)와 메시지가 출력된다.
> `session_trace`가 없는 리포트는 요약만 출력하고 "상세 추적 데이터 없음" 경고를 표시한다.

### 6.3 기본 JSON 리포트 구조

```json
{
  "scenario_id": "greeting_basic",
  "model_name": "gemma-3-12b-it",
  "timestamp": "2026-02-23T00:00:00Z",
  "auto_metrics": {
    "affinity_tag_rate": 0.92,
    "speech_violation_count": 0,
    "repetition_score": 0.1,
    "avg_response_length": 3.5,
    "forbidden_word_count": 0,
    "memory_utilization": 0.6
  },
  "judge_metrics": [
    { "metric": "character_consistency", "score": 8.1, "reasoning": "..." },
    { "metric": "context_coherence", "score": 7.8, "reasoning": "..." },
    { "metric": "hallucination_detect", "score": 0.0, "reasoning": "..." }
  ],
  "pass": true
}
```

### 6.4 상세 JSON 리포트 구조 (--detailed)

`--detailed` 모드에서는 `session_trace` 필드가 추가된다:

```json
{
  "scenario_id": "greeting_basic",
  "model_name": "gemma-3-12b",
  "timestamp": "2026-02-24T16:00:00Z",
  "auto_metrics": { "..." : "..." },
  "judge_metrics": [ "..." ],
  "pass": true,
  "session_trace": {
    "initial_system_prompt": "너는 소연이다. ...",
    "injected_memories": ["(3일 전) 서문에서 수상한 사내를 보았다."],
    "turns": [
      {
        "turn_index": 0,
        "player_input": "안녕?",
        "system_prompt": "너는 소연이다. ...",
        "llm_messages": [
          { "role": "System", "content": "너는 소연이다. ..." },
          { "role": "User", "content": "안녕?" }
        ],
        "llm_raw_response": "흥, 또 누구야. [affinity: 0]",
        "parsed": {
          "clean_text": "흥, 또 누구야.",
          "affinity_tag_found": true,
          "affinity_value": 0,
          "forbidden_words_found": []
        },
        "error": null,
        "memory_search": {
          "query": "안녕?",
          "results": [
            {
              "text": "서문에서 수상한 사내를 보았다",
              "cosine_score": 0.72,
              "final_score": 0.68,
              "importance": 3.0,
              "age_days": 5,
              "passed": true
            }
          ],
          "passed_count": 1,
          "search_time_ms": 5,
          "search_mode": "injected"
        },
        "timing": {
          "generation_ms": 2100,
          "tokens_generated": 23,
          "tokens_per_sec": 11.0,
          "ttft_ms": 850
        },
        "turn_metrics": {
          "has_affinity_tag": true,
          "speech_violation": false,
          "repetition_with_prev": 0.0,
          "response_length": 2,
          "memory_utilized": true
        }
      }
    ]
  }
}
```

### 6.5 상세 추적 데이터 타입 참조

| 구조체 | 위치 | 설명 |
|--------|------|------|
| `FullBenchReport` | report.rs | 최상위 리포트. `session_trace: Option<SessionTrace>` |
| `SessionTrace` | report.rs | 세션 전체: 시스템 프롬프트, 주입 기억, 턴 목록 |
| `TurnTrace` | report.rs | 턴 단위: LLM 입출력, 파싱, 에러, 기억 검색, 타이밍, 지표 |
| `ParsedTrace` | report.rs | 파싱 결과: clean_text, affinity_tag, forbidden_words |
| `MemorySearchTrace` | report.rs | 기억 검색: 쿼리, 결과 목록(MemoryHit), 통과 수, 검색 모드 |
| `MemoryHit` | report.rs | 개별 기억: 텍스트, cosine_score, final_score, 통과 여부 |
| `TimingTrace` | report.rs | LLM 성능: generation_ms, tokens_generated, tok/s, ttft_ms |
| `TurnMetrics` | report.rs | 턴별 품질: tag 여부, 위반, 반복도, 문장 수, 기억 반영 |
| `TurnError` | report.rs | 에러 정보: kind(LlmTimeout/LlmError/ParseFail), message |
| `BenchConfig` | runner.rs | 벤치마크 설정: conversation, model_name, detailed |

---

## 7. 파일 구조

```
wuxia-app/examples/
├── soyeon_chat_v2.rs        # 수동 대화 CLI (기존)
└── conversation_bench.rs    # 자동 벤치마크 CLI
                             #   --mock, --detailed, --replay, --baseline

wuxia-llm/src/quality/
├── mod.rs         # 모듈 등록 + re-export
├── scenario.rs    # TOML 시나리오 파서
├── runner.rs      # 시나리오 실행 + run_full_bench + BenchConfig
│                  #   detailed=true 시 TurnTrace/SessionTrace 수집
├── metrics.rs     # 자동 지표 6개 (순수 함수)
│                  #   + detect_speech_violations_in_text (턴별 변형)
│                  #   + measure_repetition_at_turn (턴별 변형)
├── judge.rs       # JudgePort + ClaudeJudge + OpenAiJudge + MockJudge
├── report.rs      # FullBenchReport + SessionTrace + TurnTrace + ComparisonReport
│                  #   + MemorySearchTrace, TimingTrace, TurnMetrics
│                  #   + TurnError/TurnErrorKind (에러 추적)
│                  #   + JSON 저장/로드 + 비교 테이블 출력
└── replay.rs      # print_replay() — 상세 리포트 터미널 재생

assets/test/scenarios/
├── 01_greeting.toml
├── 02_info_request.toml
└── 03_long_chat.toml

data/bench_reports/           # 벤치마크 결과 JSON
└── {model_name}_{scenario_id}.json
```

---

## 8. 비용 참고

LLM Judge는 시나리오당 API 호출 3회 (지표 3개 x 1회씩).

| 프로바이더 | 모델 | 예상 비용/시나리오 |
|-----------|------|-------------------|
| Claude | claude-sonnet-4-20250514 | ~$0.01 |
| OpenAI | gpt-4o | ~$0.01 |

> MockJudge를 사용하면 API 비용 없이 워크플로를 테스트할 수 있다.

`--detailed` 모드의 추가 비용: LLM 추론 대비 0.1% 미만 (턴당 ~2ms, LLM 추론 ~2000-5000ms).

---

## 9. 트러블슈팅

| 증상 | 원인 | 해결 |
|------|------|------|
| `API key not set` | 환경변수 미설정 | `ANTHROPIC_API_KEY` 또는 `OPENAI_API_KEY` 설정 |
| `Rate limited` | API 호출 빈도 초과 | 잠시 대기 후 재시도 |
| `HTTP 401` | API 키 무효 | 키 확인 및 재발급 |
| `Parse error` | LLM 응답 형식 불일치 | `[score: N]` 형식 유도 프롬프트 확인 |
| feature 미활성화 | `ClaudeJudge`/`OpenAiJudge` 사용 불가 | `--features claude-judge` 또는 `openai-judge` 추가 |
| `Blocking waiting for file lock` | 다른 cargo 빌드가 실행 중 | 다른 cargo 프로세스 종료 후 재시도 |
| Mock 모드에서 PASS인데 Live에서 FAIL | MockLlm은 항상 완벽한 응답 | Mock은 파이프라인 검증용, 품질은 Live로 측정 |
| `--replay`에서 "상세 추적 데이터 없음" | `--detailed` 없이 실행한 리포트 | `--detailed` 모드로 벤치마크 재실행 |
| 상세 리포트 파일이 큼 (50KB+) | 10턴 이상 시나리오 + detailed | 정상. 100회 누적해도 ~5MB |

---

## 10. 테스트

```bash
# 전체 테스트
cargo test -p wuxia-llm              # 276개 (replay 16 + runner + metrics + report + ...)

# 모듈별 테스트
cargo test -p wuxia-llm -- replay    # replay.rs 테스트 16개
cargo test -p wuxia-llm -- timing    # Phase 3 타이밍 테스트
cargo test -p wuxia-llm -- turn_metrics  # Phase 3 턴별 지표 테스트
cargo test -p wuxia-llm -- memory_search # Phase 2 기억 검색 추적 테스트
cargo test -p wuxia-llm -- session_trace # Phase 1 세션 추적 테스트

# wuxia-core 무변경 확인
cargo test -p wuxia-core             # 607개
```

---

## 11. 변경 이력

| 버전 | 변경일시 | 변경 내역 |
|:---:|----------|-----------|
| v1.0 | 2026-02-23 | 초기 작성. Phase 1 + Phase 2 지표 정의, 코드 사용법, 시나리오 작성법. |
| v1.1 | 2026-02-23 | conversation_bench CLI 추가. 실행 옵션(--model, --judge, --mock 등), 4단계 실행 예시(Mock→Live→Judge→비교), 출력 디렉터리 구조, 트러블슈팅 2건 추가. |
| v2.0 | 2026-02-24 | 상세 추적 + 리플레이 반영. ① `--detailed` 플래그 문서화(BenchConfig.detailed), ② `--replay` 플래그 + 터미널 재생 출력 예시, ③ 상세 JSON 리포트 구조(SessionTrace/TurnTrace/MemorySearchTrace/TimingTrace/TurnMetrics) 문서화, ④ 데이터 타입 참조 테이블 추가(§6.5), ⑤ 파일 구조에 replay.rs 추가(§7), ⑥ 코드 워크플로에 BenchConfig + print_replay 예시 추가(§4), ⑦ 테스트 섹션 신설(§10), ⑧ 트러블슈팅 2건 추가. |
