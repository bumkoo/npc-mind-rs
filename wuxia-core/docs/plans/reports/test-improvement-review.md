# test-improvement-plan.md 리뷰

> **대상:** `docs/plans/test-improvement-plan.md` v1.1.0
> **리뷰 일자:** 2026-02-24
> **검증 방법:** 코드베이스 대조 (`runner.rs`, `metrics.rs`, `report.rs`, `context.rs`, `parser.rs`, `types.rs`)

---

## 1. 전체 평가

**잘 된 점:**
- 문제 정의가 명확함 — "점수만 기록하고 과정을 기록하지 않는다" (§1.2)
- 3계층 로깅 아키텍처(TurnTrace → SessionTrace → BenchReport)의 계층 분리가 적절
- Phase 1~4 점진적 접근이 현실적이며, 의존관계도 정확
- 성능 영향 분석(§5)이 합리적 — LLM 추론 대비 0.1% 미만
- 활용 시나리오(§6) 4건이 구체적이고 실용적
- 에러 처리 전략이 상세하며, "에러 턴도 기록하되 다음 턴 계속" 방침이 올바름

---

## 2. 치명적 문제 (구현 전 반드시 수정)

### 2.1 `ChatReply` 수정 불필요 — runner.rs 구조 오해 (§8.3)

**계획서 내용:** ChatReply에 `raw_response`와 `messages_snapshot` 필드 추가 제안.

**실제 코드:** `runner.rs:166-291`의 `run_conversation()`은 `ChatSession`을 사용하지 않는다. `ConversationManager` + `LlmPort`로 직접 대화 루프를 구성하여 raw 응답을 이미 수집하고 있다:

```rust
// runner.rs:257-259 (이미 raw 수집 중)
let response = llm.generate(&request)?;
let raw_text = response.text.clone();
raw_responses.push(raw_text);
```

`LlmRequest`도 직접 구성하므로 `messages`에 접근 가능 (runner.rs:249-255).

**결론:** ChatReply 수정 없이, `run_conversation()` 내부에서 TurnTrace를 직접 구성하면 된다. **§8.3 전체를 삭제**하고, §8.2의 수집 로직을 `run_conversation()` 기준으로 재작성해야 한다.

### 2.2 타입 불일치 — `Vec<serde_json::Value>` vs `Vec<Message>` (§8.1)

**계획서 내용:** `TurnTrace.llm_messages: Vec<serde_json::Value>`

**실제 코드:** `wuxia-core/src/llm/types.rs:62-66`

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}
```

`Message`는 이미 `Serialize, Deserialize`를 구현한다. `serde_json::Value`로 변환할 이유가 없다.

**수정:** `llm_messages: Vec<Message>`로 변경. 추가로 `system_prompt: String` 필드도 TurnTrace에 포함해야 함 (매 턴 컨텍스트에 따라 system prompt가 변할 수 있으므로 — §5.2 참고).

### 2.3 forbidden_words 위치 오류 (§2.3)

**계획서 내용:** `wuxia-llm/src/quality/metrics.rs → FORBIDDEN_WORDS: &[&str]` 상수 존재.

**실제 코드:** `FORBIDDEN_WORDS` 상수는 존재하지 않는다. 금지어 목록은 `SpeechRules.forbidden_words: Vec<String>`에 정의되어 있고, `measure_forbidden_words(replies, forbidden_words)` 함수에 매개변수로 전달된다 (`metrics.rs:308-329`, `template.rs:137`).

**수정:** §2.3의 금지어 정의 위치를 `SpeechRules.forbidden_words (prompt/template.rs)`로 정정.

### 2.4 파일 경로 오류 (§3.1, §8.3)

**계획서 내용:** `wuxia-llm/src/session.rs`

**실제 경로:** `wuxia-llm/src/conversation/session.rs`

---

## 3. 중요 문제 (구현 품질에 영향)

### 3.1 Phase 2 MemorySearchTrace — runner와 ContextProvider의 단절

**계획서 내용:** `LiveContextProvider::search_memories()`에 추적 옵션 추가 (§3.2).

**실제 코드 문제:**
- 벤치마크 runner (`run_conversation()`)는 `ContextProvider` trait를 사용하지 않는다
- `runner.rs:224`에서 `PromptContext.memories = memories.to_vec()`로 직접 주입
- 벤치마크 시나리오의 기억은 `run_scenario_mock()`의 `memories: Vec<String>` 파라미터로 전달 (`runner.rs:88-92`)
- `ContextProvider::search_memories()`는 `Vec<String>` (포맷팅 완료 문자열)만 반환 — cosine_score, final_score 등 중간 데이터에 접근 불가 (`context.rs:80`)

**영향:**
- Mock 벤치마크에서 MemorySearchTrace는 의미 있는 데이터가 없음 (고정 기억 주입이므로)
- Live 벤치마크에서 MemorySearchTrace를 채우려면 runner가 `LiveContextProvider`를 사용하도록 아키텍처 변경 필요
- 또는 `ContextProvider` trait에 `search_memories_with_trace()` 메서드 추가 필요

**제안:** Phase 2의 범위를 명확히 하라:
- Mock 벤치마크: `MemorySearchTrace = { query, injected_memories (고정), passed_count: all }`
- Live 벤치마크: `LiveContextProvider`에 추적 가능 variant 추가 (이때 trait 확장 또는 별도 메서드)

### 3.2 `TurnMetrics.memory_utilized` 턴 단위 측정 미지원

**계획서 내용:** `turn_metrics.memory_utilized: bool` — 주입 기억이 응답에 반영됐는가 (§2.2, §3.5).

**실제 코드:** `measure_memory_utilization()` (`metrics.rs:342-371`)은 전체 replies를 한번에 평가한다. 턴 단위 변형이 없다.

**수정:** 턴 단위 `measure_memory_utilization_single(reply, memories) -> bool` 함수 추가 필요. 또는 기존 함수를 1개 reply에 대해 호출하는 방식.

### 3.3 `--detailed` 플래그와 `run_full_bench()` 시그니처

**계획서 내용:** `--detailed` CLI 옵션 추가 (§4.3).

**실제 코드:** `run_full_bench()` (`runner.rs:299-352`)는 현재 `detailed` 파라미터가 없다. 이 함수의 시그니처를 변경하거나, `BenchConfig` 구조체를 도입해야 한다.

**제안:** 이미 `#[allow(clippy::too_many_arguments)]`가 있으므로 (`runner.rs:298`), 매개변수를 더 추가하기보다 `BenchConfig` 구조체로 묶는 것이 좋다:

```rust
pub struct BenchConfig {
    pub conversation: ConversationConfig,
    pub model_name: String,
    pub detailed: bool,
}
```

---

## 4. 경미한 문제

### 4.1 `serde_json` 의존성 — 이미 존재

계획서에서 별도 언급 없으나, `wuxia-llm/Cargo.toml:20`에 `serde_json = "1"` 이미 포함. 문제 없음.

### 4.2 `now_iso8601()` 하드코딩

`report.rs:169-171`에서 timestamp가 하드코딩 되어 있다(`"2026-01-01T00:00:00Z"`). `--detailed` 파일명에 타임스탬프를 포함하려면 (§4.1: `*_20260224T160000.json`) 실제 시간을 사용하는 구현이 필요. `chrono` 의존성 추가 또는 `std::time::SystemTime` 활용 고려.

### 4.3 CSV 내보내기의 `player_input` 50자 절단

§3.5에서 `player_input`과 `response_preview`를 "첫 50자 + ..."로 절단한다. 한국어는 3바이트/문자이므로 `chars().take(50)` 사용 필수 — 바이트 단위 절단하면 panic 가능.

### 4.4 `TurnTrace`의 Phase 2/3 필드 — `Option` 래핑 합리적

§8.1에서 Phase 2~3 필드를 주석으로 처리했는데, `Option<MemorySearchTrace>` 등으로 처리하는 것이 `#[serde(default)]` 패턴과 일관적이다. 이 부분은 이미 계획서에 의도된 것으로 보임.

---

## 5. 추가 제안

### 5.1 후방 호환성 테스트 추가

성공 기준(§9)에 `--detailed` 미지정 시 기존 동작 그대로인지 확인하는 테스트가 빠져 있다:
- `run_full_bench()` 기본 호출 → `session_trace: None` 확인
- 기존 summary JSON 형식 불변 확인 (JSON round-trip)

### 5.2 `TurnTrace`에 `system_prompt` 포함 고려

현재 TurnTrace에는 `llm_messages`만 있고 `system_prompt`는 SessionTrace에만 있다. 그러나 `run_conversation()`에서 매 턴 system prompt를 재빌드한다 (`runner.rs:231-232`). 기억이나 요약이 변하면 system prompt도 턴마다 다를 수 있다. 디버깅 목적이라면 턴별 system_prompt도 기록해야 한다.

### 5.3 `--replay` 에러 턴 시각화에 컬러 사용

§3.4의 에러 턴 표시에 `⚠ ERROR`만 있는데, 터미널 ANSI 컬러(빨간색)를 사용하면 가독성이 높아진다.

---

## 6. 검증 방법

구현 후 다음을 확인:

1. `cargo test -p wuxia-llm` — 기존 테스트 전부 통과
2. Phase 1 테스트: MockLlm 3턴 시나리오 → TurnTrace 3개 생성, llm_messages 포함 확인
3. `--detailed` 없이 실행 → 기존 동작 불변 확인
4. 에러 주입 테스트 → TurnTrace.error 채워짐 확인
5. JSON round-trip: 상세 리포트 저장 → 로드 → 필드 비교

---

## 요약: 수정 필요 항목 체크리스트

| # | 심각도 | 항목 | 조치 |
|---|--------|------|------|
| 2.1 | 치명적 | ChatReply 수정 불필요 | §8.3 삭제, §8.2를 run_conversation() 기준으로 재작성 |
| 2.2 | 치명적 | Vec\<serde_json::Value\> → Vec\<Message\> | §8.1 TurnTrace 타입 수정 |
| 2.3 | 치명적 | FORBIDDEN_WORDS 상수 없음 | §2.3 위치 정정 → SpeechRules.forbidden_words |
| 2.4 | 치명적 | 파일 경로 오류 | session.rs → conversation/session.rs |
| 3.1 | 중요 | Phase 2 MemorySearchTrace runner 단절 | Mock/Live 분리 전략 명시 |
| 3.2 | 중요 | memory_utilized 턴 단위 미지원 | 단일 턴 함수 추가 계획 |
| 3.3 | 중요 | run_full_bench 매개변수 폭발 | BenchConfig 구조체 도입 |
| 5.1 | 제안 | 후방 호환성 테스트 누락 | 성공 기준에 추가 |
| 5.2 | 제안 | 턴별 system_prompt 누락 | TurnTrace에 system_prompt 추가 |

---

## 7. Phase 4 방안 B (HTML 리포트) 리뷰

> 추가 리뷰 일자: 2026-02-24

### 7.1 현황 평가

방안 B는 현재 3줄짜리 아이디어 메모 수준 — 실행 가능한 계획이 아니다:
- 기능 목록만 나열 (접기/펼치기, 하이라이트, 그래프)
- 구현 방법, 의존성, 파일 매핑, 성공 기준 전무
- 방안 A (터미널 Pretty Print)와 비교하면 구체성이 크게 부족

### 7.2 핵심 분석 결과

**데이터 레이어는 이미 충분:** Phase 1~3에서 `FullBenchReport` → `SessionTrace` → `Vec<TurnTrace>` + `MemorySearchTrace` + `TimingTrace` + `TurnMetrics`가 전부 JSON 직렬화 가능한 상태. HTML 렌더링에 필요한 데이터 구조 추가 작업 없음.

**구현 방식 선택:**

| 방안 | 설명 | 판정 |
|------|------|------|
| askama `--export-html` | Rust에서 HTML 생성. 타입 안전하지만 재실행 필요. | 과잉 |
| **JSON 뷰어 패턴** | `assets/tools/bench-viewer.html` 단일 파일. JSON 드래그 앤 드롭. | **권장** |
| Markdown `--export-md` | 가볍지만 접기/펼치기 불가. | 보조적 |

JSON 뷰어 패턴이 이 프로젝트에 가장 적합한 이유:
- Rust 코드 변경 0, 의존성 추가 0, feature flag 불필요
- `FullBenchReport`가 이미 완전한 JSON 직렬화를 지원
- 방안 A(터미널)와 완전히 독립 — 병렬 개발 가능
- 같은 JSON을 다양한 방식으로 재조회 가능 (Rust 재컴파일 불필요)

### 7.3 방안 A와의 관계

방안 A(터미널)와 B(HTML)는 보완 관계이며, 순차 구현 권장:
- 방안 A: 개발자가 벤치마크 직후 즉시 디버깅용
- 방안 B: 팀 공유, 아카이빙, 인터랙티브 탐색용
- 구현 순서: Phase 4A → 4C(CSV) → 4B(HTML)

### 7.4 MVP 범위

구현 시 최소 범위 (0.5일):
1. Single-file HTML 뷰어 (`bench-viewer.html`)
2. 요약 대시보드 (pass/fail + 자동 지표 + judge 점수, 색상 코딩)
3. 턴 타임라인 (`<details>` 태그로 접기/펼치기)
4. 각 턴: player_input, NPC 응답, 파싱 결과, 에러 표시
5. 인라인 CSS (다크 테마), 최소 JS

MVP에서 제외: 차트/그래프, A/B 비교 HTML, 기억 하이라이트, 실시간 필터

### 7.5 시기 판단

"MVP 이후"는 적절하지만, Phase 4A + 4C 완료 후 바로 착수 가능. 포맷팅 로직을 재사용하면 0.5일이면 충분.

### 7.6 §3.4 수정 권장 사항

현재 3줄을 아래로 교체:
```
방안 B: HTML 리포트 (JSON 뷰어 패턴)
─────────────────────────────────
목표: --detailed JSON 리포트를 브라우저에서 시각화
구현: assets/tools/bench-viewer.html (단일 파일, Rust 변경 0)
  ├── JSON 파일 드래그 앤 드롭
  ├── 요약 대시보드 (pass/fail + 지표 색상 코딩)
  ├── 턴 타임라인 (접기/펼치기)
  ├── 기억 검색 결과 (통과/탈락 구분)
  └── 에러 턴 하이라이트 (빨간 배경)
시기: Phase 4A(--replay) + Phase 4C(--export-csv) 완료 후
의존성: 없음 (순수 HTML/CSS/JS)
```
