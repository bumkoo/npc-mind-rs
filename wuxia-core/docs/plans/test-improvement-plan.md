# 대화 품질 벤치마크 — 테스트 개선 계획

**버전:** v1.1.0
**수정일:** 2026-02-24 22:30:00
**Status:** ✅ 구현 완료 (TurnTrace, SessionTrace, TimingTrace, MemoryHit → quality/trace.rs; terminal replay → quality/replay.rs; 2026-03-03 확인)
**관련 문서:** `quality-bench-guide.md` (v1.1), `npc-conversation-memory-architecture.md`, `sprint3-progress.md`

---

## 1. 현황 분석

### 1.1 현재 테스트가 기록하는 것

```
현재 JSON 리포트 (quality-bench-guide.md §6 기준):

  ┌─────────────────────────────────────┐
  │ scenario_id                         │  ← 어떤 시나리오인지
  │ model_name                          │  ← 어떤 모델인지
  │ timestamp                           │  ← 언제 측정했는지
  │ auto_metrics (6개 수치)              │  ← 결과 점수
  │ judge_metrics (3개 점수 + reasoning) │  ← LLM 판정 점수
  │ pass (true/false)                   │  ← 합격 여부
  └─────────────────────────────────────┘
```

### 1.2 현재 기록에서 빠져 있는 것 (문제점)

```
지금 리포트로는 "왜 실패했는지"를 역추적할 수 없다

  예: affinity_tag_rate = 45% (FAIL)
      → 어떤 턴에서 태그가 누락됐는가?          ← 모름
      → 그 턴의 시스템 프롬프트가 어떠했는가?     ← 모름
      → 기억 검색 결과가 프롬프트에 들어갔는가?   ← 모름
      → LLM이 실제로 뭐라고 응답했는가?          ← 모름
      → 파싱 후 태그가 올바르게 추출됐는가?       ← 모름
```

**핵심 문제:** 리포트가 **"점수만"** 기록하고 **"과정"**을 기록하지 않는다.

### 1.3 개선 목표

```
개선 전:  시나리오 → [블랙박스] → 점수
개선 후:  시나리오 → [매 턴의 전체 파이프라인 추적] → 점수 + 근거 데이터

  ┌──────────────────────────────────────────────────┐
  │  Turn 1                                          │
  │  ├── 입력: "안녕?"                                │
  │  ├── 시스템 프롬프트 (전문)                        │
  │  ├── 기억 검색 쿼리 → 검색 결과 (점수, 텍스트)     │
  │  ├── 관계 상태 (호감, 신뢰)                        │
  │  ├── 조립된 메시지 배열 (최종 LLM 입력)            │
  │  ├── LLM 원시 응답 (파싱 전)                       │
  │  ├── 파싱 결과 (태그 추출값, 정제된 텍스트)         │
  │  ├── 턴별 자동 지표 (tag? violation? repetition?)  │
  │  ├── 에러 정보 (실패 시)                           │
  │  └── 소요 시간 (TTFT, 전체)                       │
  │                                                  │
  │  Turn 2 ...                                      │
  │  Turn 3 ...                                      │
  └──────────────────────────────────────────────────┘
```

---

## 2. 개선 설계 — 3계층 로깅 아키텍처

### 2.1 전체 구조

```
  계층 1: TurnTrace (턴 단위 추적)
  ─────────────────────────────
  매 턴마다 파이프라인의 모든 중간 상태를 기록

  계층 2: SessionTrace (세션 단위 추적)
  ─────────────────────────────────
  전체 대화 세션의 턴 목록 + 세션 수준 메타데이터

  계층 3: BenchReport (기존 리포트 확장)
  ────────────────────────────────────
  기존 점수 + SessionTrace 포함
```

### 2.2 데이터 구조 (Rust 관점)

```
BenchReport (기존 확장)
├── scenario_id: String
├── model_name: String
├── timestamp: DateTime
├── auto_metrics: AutoMetrics          ← 기존 유지
├── judge_metrics: Vec<JudgeMetric>    ← 기존 유지
├── pass: bool                         ← 기존 유지
│
└── session_trace: SessionTrace        ← ★ 신규
    ├── system_prompt: String              전체 시스템 프롬프트 텍스트
    ├── model_config: ModelConfig           temperature, max_tokens 등
    ├── injected_memories: Vec<String>      시나리오에서 주입한 기억
    ├── initial_relationship: RelState      대화 시작 시 관계 상태
    │
    └── turns: Vec<TurnTrace>          ← ★ 핵심
        ├── turn_index: usize
        ├── player_input: String           플레이어 입력 원문
        │
        ├── llm_messages: Vec<Message>     ★ LLM에 전달된 최종 메시지 배열
        │
        ├── memory_search: MemorySearchTrace
        │   ├── query: String              검색 쿼리 (= player_input)
        │   ├── results: Vec<MemoryHit>    검색된 기억들
        │   │   ├── text: String               기억 텍스트
        │   │   ├── cosine_score: f32          벡터 유사도
        │   │   ├── final_score: f32           4축 랭킹 후 최종 점수
        │   │   ├── importance: f32            중요도
        │   │   └── age_days: u32              경과 일수
        │   ├── threshold: f32             적용된 임계값
        │   ├── passed_count: usize        임계값 통과 수
        │   └── search_time_ms: u64        검색 소요 시간
        │
        ├── context_snapshot: ContextSnapshot
        │   ├── relationship_summary: String   관계 상태 문자열
        │   ├── memory_labels: Vec<String>     행동 지시형 라벨 목록
        │   ├── conversation_summary: Option<String>  압축된 이전 대화
        │   └── total_context_tokens: usize    추정 토큰 수
        │
        ├── llm_raw_response: String       ★ LLM 원시 응답 (파싱 전)
        ├── parsed_response: ParsedTrace
        │   ├── clean_text: String             태그 제거 후 텍스트
        │   ├── affinity_tag_found: bool       [affinity: N] 발견 여부
        │   ├── affinity_value: Option<i8>     추출된 호감도 값
        │   └── forbidden_words: Vec<String>   발견된 금지어 목록
        │
        ├── turn_metrics: TurnMetrics      ★ 턴 단위 자동 지표
        │   ├── has_affinity_tag: bool
        │   ├── speech_violation: bool
        │   ├── repetition_with_prev: f32      이전 턴과의 반복도
        │   ├── response_length: usize         문장 수
        │   └── memory_utilized: bool          주입 기억이 응답에 반영됐는가
        │
        ├── error: Option<TurnError>       ★ 신규 — 에러 추적
        │   ├── kind: TurnErrorKind            (LlmTimeout | LlmError | ParseFail | MemorySearchFail)
        │   ├── message: String                에러 메시지
        │   └── partial_data: bool             부분 데이터 수집 여부
        │
        └── timing: TimingTrace
            ├── ttft_ms: u64               첫 토큰 시간
            ├── generation_ms: u64         생성 총 시간
            ├── tokens_generated: usize    생성 토큰 수
            └── tokens_per_sec: f32        초당 토큰
```

### 2.3 금지어(forbidden_words) 매칭 방식

```
금지어 목록 정의 위치:
  wuxia-llm/src/quality/metrics.rs  →  FORBIDDEN_WORDS: &[&str]

매칭 로직:
  파싱 시 clean_text에 대해 금지어 목록 순회 → 포함 여부 확인

  ┌─────────────────────────────────────────────────────┐
  │ for word in FORBIDDEN_WORDS {                       │
  │     if clean_text.contains(word) {                  │
  │         parsed.forbidden_words.push(word.to_owned());│
  │     }                                               │
  │ }                                                   │
  └─────────────────────────────────────────────────────┘

금지어 예시 (캐릭터 일관성 파괴 표현):
  - 현대어 표현: "알겠습니다", "네", "감사합니다"
  - 설명적 표현: "저는 NPC입니다", "게임 내에서"
  - 세계관 파괴: 현대 용어, 외래어 등
  → 전체 목록은 metrics.rs 참조
```

---

## 3. 구현 계획 — 4단계 Iterative 접근

### 3.1 Phase 1: TurnTrace 기본 골격 (우선순위 최고)

```
목표: 가장 핵심적인 3가지를 기록한다
  ① LLM에 보낸 메시지 전문 (llm_messages)
  ② LLM 원시 응답 (llm_raw_response)
  ③ 파싱 결과 (parsed_response)
  + 에러 발생 시 에러 정보 (error)

구현 위치:
  wuxia-llm/src/quality/runner.rs
    └── run_full_bench() 내부에서 매 턴 TurnTrace 수집

변경 파일:
  ┌─────────────────────────────────────────┬───────────────┐
  │ 파일                                    │ 변경 내용      │
  ├─────────────────────────────────────────┼───────────────┤
  │ wuxia-llm/src/quality/report.rs         │ TurnTrace, TurnError 구조체 정의 + Serialize │
  │ wuxia-llm/src/quality/runner.rs         │ run_full_bench에서 수집 로직 │
  │ wuxia-llm/src/session.rs                │ send()가 중간 데이터 반환 옵션 │
  └─────────────────────────────────────────┴───────────────┘

에러 처리 전략:
  ┌───────────────────────────────────────────────────────────┐
  │ LLM 호출 실패 시:                                          │
  │   → TurnTrace는 생성하되 error 필드를 채움                   │
  │   → llm_raw_response = "" (빈 문자열)                      │
  │   → parsed_response = 기본값                               │
  │   → 벤치마크는 해당 턴을 FAIL로 기록하고 다음 턴 계속          │
  │                                                           │
  │ 파싱 실패 시:                                               │
  │   → llm_raw_response는 기록 (원본 보존)                     │
  │   → error.kind = ParseFail                                │
  │   → error.partial_data = true                             │
  │                                                           │
  │ 기억 검색 실패 시 (Phase 2):                                │
  │   → memory_search에 빈 results + error 기록                │
  │   → 대화는 기억 없이 계속 진행                               │
  └───────────────────────────────────────────────────────────┘

테스트:
  - MockLlm으로 3턴 시나리오 실행 → TurnTrace 3개 생성 확인
  - llm_messages가 시스템 프롬프트를 포함하는지 확인
  - llm_raw_response와 parsed clean_text가 다른지 확인 (태그 제거)
  - MockLlm 에러 주입 → TurnTrace.error가 채워지는지 확인
```

### 3.2 Phase 2: 기억 검색 추적 (MemorySearchTrace)

```
목표: 매 턴의 기억 검색 과정을 기록한다
  ① 검색 쿼리
  ② 검색 결과 (유사도 점수 포함)
  ③ 4축 랭킹 후 최종 점수
  ④ 임계값 통과/탈락 여부

구현 위치:
  wuxia-llm/src/context.rs
    └── LiveContextProvider::search_memories()에 추적 옵션 추가

  wuxia-memory/src/repository/
    └── search() 결과에 cosine_score 포함 (이미 있을 수 있음)

설계 선택지:

  방안 A: Callback 방식 (권장)
  ──────────────────────────
  search_memories()에 Option<&mut MemorySearchTrace> 매개변수 추가.
  있으면 기록, 없으면 무시 → 기존 코드 영향 0

    장점:
      - 기존 호출부 변경 없음 (None 전달)
      - 런타임 분기로 추적 ON/OFF 제어
      - 프로덕션 코드와 벤치마크 코드 공유 가능

    단점:
      - 함수 시그니처에 추적 전용 매개변수 침투 (API 오염)
      - 추적 항목 증가 시 매개변수가 계속 늘어남
      - 향후 리팩토링 대상: TracingContext 구조체로 통합 고려

  방안 B: 반환값 확장 방식
  ──────────────────────────
  search_memories()가 (Vec<MemoryLabel>, MemorySearchTrace)를 반환.
  더 깔끔하지만 기존 호출부 수정 필요

    장점:
      - 반환값에 추적 데이터 포함 → API 의도가 명확
      - 호출부가 추적 데이터를 무시하면 됨

    단점:
      - 기존 모든 호출부에서 반환값 destructuring 수정 필요
      - 항상 추적 데이터를 생성하므로 미미한 성능 오버헤드

  선택: 방안 A
  이유: Phase 1~2에서 빠른 검증이 우선이고, 기존 코드 변경을 최소화.
        추적 매개변수가 3개 이상으로 늘어나면 TracingContext 구조체로 리팩토링.

테스트:
  - InMemoryRepository에 기억 3개 → 검색 → trace에 3개 hit 기록 확인
  - threshold 이하 기억이 탈락으로 기록되는지 확인
  - 기억 0개일 때 빈 trace 생성 확인
  - 기억 검색 에러 시 error 필드 채워지는지 확인
```

### 3.3 Phase 3: 타이밍 + 턴 단위 지표

```
목표: 성능 측정과 턴별 세분화 지표를 추가한다
  ① TTFT, 생성 시간, tok/s
  ② 턴별 개별 지표 (기존은 전체 평균만 있음)

구현:
  - TimingTrace는 LlamaCppAdapter의 기존 성능 로그에서 추출
  - TurnMetrics는 기존 metrics.rs의 함수를 턴 단위로 호출

활용 가치:
  "12턴 대화에서 후반부(8턴~)에 반복이 급증한다"
  → turn_metrics[7].repetition_with_prev = 0.45 (임계 0.2 초과)
  → 이 시점의 context_snapshot.total_context_tokens = 6100 (ctx 75%)
  → 원인: 컨텍스트 포화 시점에서 반복 증가 패턴 확인
```

### 3.4 Phase 4: 디버그 뷰어 + CSV 내보내기

```
목표: JSON을 사람이 읽기 쉽게 시각화한다

  방안 A: 터미널 Pretty Print
  ──────────────────────────
  conversation_bench --replay data/bench_reports/detailed/xxx.json

  Turn 1 ─────────────────────────────────────────
  Player: "안녕?"

  [기억 검색] query="안녕?" → 2건 (0.72, 0.58)
    ├── "어제 서문에서 만난 행인..." (score=0.72, imp=3)
    └── "만두를 먹었다..."          (score=0.58, imp=2) ← 탈락 (threshold)

  [컨텍스트] tokens=1250, relationship="중립 (호감 0)"

  [LLM 응답 원문]
  "흥, 또 누구야. 자유도시에서 뭔 볼일이야? [affinity: 0]"

  [파싱] text="흥, 또 누구야. 자유도시에서 뭔 볼일이야?"
         affinity=0 ✅, violation=0 ✅, forbidden=0 ✅

  [성능] TTFT=850ms, total=2.1s, 23 tok, 11.0 tok/s

  [에러 발생 턴의 표시 예시]
  Turn 5 ───────────────────────── ⚠ ERROR ─────
  Player: "그 이야기 더 해줘"

  [에러] LlmTimeout: "30초 타임아웃 초과"
         partial_data: false


  방안 B: HTML 리포트 (향후)
  ─────────────────────────
  턴별 접기/펼치기, 기억 검색 하이라이트, 점수 그래프
  → MVP 이후 고려
```

### 3.5 --export-csv 컬럼 명세

```
CSV 컬럼 정의 (턴 1행 = CSV 1행):

  ┌────────────────────────┬──────────┬───────────────────────────────┐
  │ 컬럼명                  │ 타입     │ 설명                          │
  ├────────────────────────┼──────────┼───────────────────────────────┤
  │ scenario_id            │ String   │ 시나리오 식별자                │
  │ model_name             │ String   │ 모델명                        │
  │ turn_index             │ usize    │ 턴 번호 (0-based)             │
  │ player_input           │ String   │ 플레이어 입력 (첫 50자 + ...) │
  │ response_preview       │ String   │ NPC 응답 (첫 50자 + ...)      │
  │ affinity_tag_found     │ bool     │ 호감도 태그 발견 여부          │
  │ affinity_value         │ i8?      │ 추출된 호감도 값               │
  │ speech_violation       │ bool     │ 말투 위반 여부                 │
  │ forbidden_word_count   │ usize    │ 금지어 발견 수                 │
  │ repetition_score       │ f32      │ 이전 턴 대비 반복도            │
  │ memory_hit_count       │ usize    │ 기억 검색 결과 수              │
  │ memory_passed_count    │ usize    │ 임계값 통과 기억 수            │
  │ memory_utilized        │ bool     │ 기억이 응답에 반영됐는가       │
  │ context_tokens         │ usize    │ 컨텍스트 토큰 수               │
  │ ttft_ms                │ u64      │ 첫 토큰까지 시간 (ms)         │
  │ generation_ms          │ u64      │ 전체 생성 시간 (ms)           │
  │ tokens_per_sec         │ f32      │ 초당 토큰                     │
  │ has_error              │ bool     │ 에러 발생 여부                 │
  │ error_kind             │ String?  │ 에러 유형                     │
  └────────────────────────┴──────────┴───────────────────────────────┘

활용 예시:
  - 스프레드시트에서 피벗 테이블 → 모델별/턴별 지표 비교
  - Python pandas로 시각화 → 턴 진행에 따른 반복도 추이 그래프
  - repetition_score > 0.2 턴 필터링 → 컨텍스트 포화 지점 분석
```

---

## 4. 저장 구조

### 4.1 파일 구조

```
data/bench_reports/
├── summary/                          ← 기존 형식 (간결한 점수만)
│   ├── gemma-3-4b_greeting_basic.json
│   └── gemma-3-12b_greeting_basic.json
│
└── detailed/                         ← ★ 신규 (전체 추적 데이터)
    ├── gemma-3-4b_greeting_basic_20260224T160000.json
    └── gemma-3-12b_greeting_basic_20260224T163000.json
```

### 4.2 파일 크기 추정

```
  시나리오당 파일 크기 추정:

  기존 summary JSON:         ~2 KB
  신규 detailed JSON (3턴):  ~15-25 KB
  신규 detailed JSON (10턴): ~50-80 KB

  내역:
    시스템 프롬프트:      ~3 KB (1회)
    턴당 메시지 배열:     ~2-4 KB (누적됨)
    턴당 기억 검색:       ~1-2 KB
    턴당 응답 + 파싱:     ~1-2 KB
    턴당 지표 + 타이밍:   ~0.3 KB
    턴당 에러 정보:       ~0.1 KB (에러 시에만)

  10턴 × ~5KB/턴 + 3KB 시스템 = ~53 KB

  100회 벤치마크 누적: ~5 MB (부담 없음)
```

### 4.3 CLI 옵션 확장

```
기존:
  --mock              파이프라인 검증
  --model <path>      모델 경로
  --judge <type>      Judge 종류
  --baseline <path>   비교 기준

신규 추가:
  --detailed          상세 추적 활성화 (detailed/ 에 저장)
  --replay <path>     저장된 상세 리포트를 터미널에서 재생
  --export-csv <path> 턴별 지표를 CSV로 내보내기 (§3.5 컬럼 명세 참조)
```

---

## 5. 성능 영향 분석

```
--detailed 모드가 벤치마크 실행 속도에 미치는 영향 추정:

  ┌───────────────────────────┬──────────────┬──────────────────────────┐
  │ 항목                       │ 추가 비용     │ 근거                     │
  ├───────────────────────────┼──────────────┼──────────────────────────┤
  │ TurnTrace 구조체 생성      │ < 0.1 ms/턴  │ String clone 몇 개       │
  │ llm_messages clone        │ < 1 ms/턴    │ 시스템프롬프트 포함 ~4KB  │
  │ MemorySearchTrace 수집    │ < 0.5 ms/턴  │ Vec<MemoryHit> clone     │
  │ TimingTrace 수집           │ ~0 ms/턴     │ 숫자 몇 개 기록          │
  │ JSON 직렬화 (저장 시)       │ < 5 ms/세션  │ 50-80KB JSON write       │
  └───────────────────────────┴──────────────┴──────────────────────────┘

  총 추가 비용: 턴당 ~2 ms 미만
  LLM 추론 시간: 턴당 ~2,000-5,000 ms (gemma-3-4b 기준)

  결론: --detailed 오버헤드는 LLM 추론 시간의 0.1% 미만.
        벤치마크 결과에 영향 없음. 성능 저하 무시 가능.

  주의사항:
  - llm_messages는 턴이 진행될수록 누적 크기 증가 (대화 히스토리 포함)
  - 10턴 이상 시나리오에서 메모리 사용량 증가 가능 (~수 MB 수준)
  - 메모리가 부족한 환경에서는 --detailed 비활성화 권장
```

---

## 6. 향후 활용 시나리오

### 6.1 프롬프트 엔지니어링 A/B 테스트

```
활용: 프롬프트 변경 전후 비교

  1) 기존 프롬프트로 --detailed 벤치마크 실행 → baseline_detailed.json
  2) 프롬프트 수정
  3) 수정 프롬프트로 --detailed 벤치마크 실행 → modified_detailed.json
  4) 턴별 비교:
     - 같은 턴에서 시스템 프롬프트 diff
     - 같은 입력에 대한 LLM 응답 차이
     - 기억 검색 결과가 프롬프트에 어떻게 반영됐는지

  기대 효과: "프롬프트에서 X를 바꾸면 Y 지표가 이렇게 변한다"를
             데이터로 입증 가능
```

### 6.2 기억 시스템 튜닝

```
활용: 임계값, 가중치 조정 근거 확보

  문제: memory_utilization이 30%로 낮음

  detailed 리포트 분석:
    Turn 3 기억 검색:
      query = "혈교가 뭐야?"
      results:
        ├── "혈교 교주가 남궁가를 습격했다" (cosine=0.72, final=0.68) ✅ 통과
        ├── "사파 3대 세력은 혈교, 마교, 독문이다" (cosine=0.51, final=0.45) ← 탈락!
        └── "서량국 국경에서 혈교 잔당을 목격" (cosine=0.48, final=0.41) ← 탈락!
      threshold = 0.4656
      boost_threshold = 0.5122

  진단: 관련 기억 2건이 boost_threshold 아래 + keyword=0이라 탈락
  조치: boost_ratio를 1.1 → 1.05로 낮추면 통과 가능
  검증: 다시 --detailed 벤치마크 → memory_utilization 변화 확인
```

### 6.3 모델 비교 심층 분석

```
활용: 4b vs 12b 차이의 근본 원인 파악

  기존: "12b가 character_consistency 8.1, 4b가 5.2"
        → 왜? 모름.

  개선 후: 같은 시나리오, 같은 턴의 원시 응답 비교

  Turn 5 (4b):
    LLM: "네가 혈교에 대해 물어봤지? 혈교는 나쁜 놈들이야.
          조심해. [affinity: 1]"
    → 캐릭터 붕괴: 소연은 "나쁜 놈들이야" 같은 단순 표현 안 씀

  Turn 5 (12b):
    LLM: "...혈교? 흥, 그 이름을 꺼내다니 배짱은 있군.
          알고 싶으면 대가를 치러. [affinity: -1]"
    → 캐릭터 유지: 정보 대가 요구 + 소연 특유의 비꼬는 말투
```

### 6.4 Fine-Tuning 데이터셋 구축 기반

```
활용: 양질의 대화 데이터를 Fine-Tuning 학습 데이터로 재활용

  --detailed 리포트에서 "PASS + 높은 점수" 턴만 추출:

  추출 기준:
    character_consistency >= 8.0
    speech_violation == 0
    affinity_tag_found == true

  추출 형식 (instruction tuning):
    {
      "instruction": "{시스템 프롬프트 + 기억 + 관계}",
      "input": "안녕?",
      "output": "흥, 또 누구야. 자유도시에서 뭔 볼일이야?"
    }

  → 12b 모델의 우수 응답을 4b Fine-Tuning 데이터로 활용
  → "교사 모델(12b) → 학생 모델(4b)" 지식 증류 패턴
```

---

## 7. 구현 우선순위 및 일정

```
  Phase 1 ──── TurnTrace 기본 골격 ──── 예상 0.5일
  │  report.rs에 TurnTrace + TurnError 구조체 정의
  │  runner.rs에서 수집 로직 (llm_messages 포함)
  │  에러 핸들링 전략 적용
  │  MockLlm 테스트 4건 (정상 3건 + 에러 1건)
  │
  Phase 2 ──── 기억 검색 추적 ───────── 예상 0.5일
  │  MemorySearchTrace 구조체
  │  context.rs에 추적 옵션 (방안 A: callback)
  │  InMemoryRepository 테스트 4건 (정상 3건 + 검색 실패 1건)
  │
  Phase 3 ──── 타이밍 + 턴별 지표 ───── 예상 0.5일
  │  TimingTrace 수집
  │  TurnMetrics 턴 단위 분할
  │  Live LLM 테스트 (수동 확인)
  │
  Phase 4 ──── 디버그 뷰어 + CSV ────── 예상 0.5일
     --replay 터미널 출력 (에러 턴 하이라이트)
     --export-csv 내보내기 (§3.5 컬럼 명세 기준)
```

### 의존 관계

```
  Phase 1 (필수, 독립)
      │
      ├──► Phase 2 (Phase 1의 TurnTrace에 추가)
      │
      ├──► Phase 3 (Phase 1의 TurnTrace에 추가)
      │
      └──► Phase 4 (Phase 1~3 데이터를 시각화)
```

---

## 8. 코드 변경 범위 (Phase 1 상세)

### 8.1 신규 구조체 — report.rs 확장

```rust
// wuxia-llm/src/quality/report.rs (기존 파일에 추가)

/// LLM 호출 또는 파싱 실패 시 에러 정보.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TurnErrorKind {
    LlmTimeout,
    LlmError,
    ParseFail,
    MemorySearchFail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnError {
    pub kind: TurnErrorKind,
    pub message: String,
    pub partial_data: bool,  // true면 일부 필드는 유효
}

/// 턴 단위 추적 데이터. 벤치마크 --detailed 옵션 시에만 수집.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnTrace {
    pub turn_index: usize,
    pub player_input: String,
    pub llm_messages: Vec<serde_json::Value>,  // ★ LLM에 보낸 메시지 배열
    pub llm_raw_response: String,
    pub parsed_text: String,
    pub affinity_tag_found: bool,
    pub affinity_value: Option<i8>,
    pub forbidden_words: Vec<String>,
    pub error: Option<TurnError>,              // ★ 에러 추적
    // Phase 2에서 추가: pub memory_search: Option<MemorySearchTrace>,
    // Phase 3에서 추가: pub timing: Option<TimingTrace>,
    // Phase 3에서 추가: pub turn_metrics: Option<TurnMetrics>,
}

/// 세션 전체 추적 데이터.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTrace {
    pub system_prompt: String,
    pub turns: Vec<TurnTrace>,
}

// 기존 FullBenchReport에 필드 추가
// pub session_trace: Option<SessionTrace>,  // --detailed 시에만 Some
```

### 8.2 수집 로직 — runner.rs 변경

```rust
// wuxia-llm/src/quality/runner.rs
// run_full_bench() 내부 의사코드

// 개선 후:
//   let mut turn_traces = Vec::new();
//   for (i, turn) in scenario.turns.iter().enumerate() {
//       match session.send(&turn.player) {
//           Ok(response) => {
//               if detailed {
//                   turn_traces.push(TurnTrace {
//                       turn_index: i,
//                       player_input: turn.player.clone(),
//                       llm_messages: response.messages_snapshot
//                           .unwrap_or_default(),           // ★ Phase 1 핵심
//                       llm_raw_response: response.raw_response
//                           .unwrap_or_default(),
//                       parsed_text: response.text.clone(),
//                       affinity_tag_found: response.raw_response
//                           .as_deref()
//                           .map(|r| r.contains("[affinity:"))
//                           .unwrap_or(false),              // ★ 수정: 태그 존재 여부만 확인
//                       affinity_value: Some(response.affinity_delta),
//                       forbidden_words: extract_forbidden_words(&response.text),
//                       error: None,
//                   });
//               }
//               responses.push(response);
//           }
//           Err(e) => {
//               if detailed {
//                   turn_traces.push(TurnTrace {
//                       turn_index: i,
//                       player_input: turn.player.clone(),
//                       llm_messages: vec![],
//                       llm_raw_response: String::new(),
//                       parsed_text: String::new(),
//                       affinity_tag_found: false,
//                       affinity_value: None,
//                       forbidden_words: vec![],
//                       error: Some(TurnError {
//                           kind: TurnErrorKind::LlmError,
//                           message: e.to_string(),
//                           partial_data: false,
//                       }),
//                   });
//               }
//               // 에러 턴도 기록하되 다음 턴 계속 진행
//           }
//       }
//   }
```

### 8.3 ChatReply 확장 — session.rs 변경

```rust
// wuxia-llm/src/session.rs

// 기존 ChatReply:
//   pub text: String,
//   pub affinity_delta: i8,
//   pub turn_index: usize,
//   pub compressed: bool,

// 추가 필드:
//   pub raw_response: Option<String>,           // 파싱 전 원시 응답 (detailed 모드에서만)
//   pub messages_snapshot: Option<Vec<serde_json::Value>>,  // LLM에 보낸 메시지 (detailed 모드에서만)
```

### 8.4 affinity_tag_found 판정 로직 설명

```
v1.0.0에서의 문제:
  affinity_tag_found = response.affinity_delta != 0
                       || response.raw_text.contains("[affinity:")

  문제 케이스:
    ① [affinity: 0] 태그가 있는데 delta=0 → contains만으로 잡힘 ✅
    ② 태그 없이 다른 경로로 delta≠0 → false positive 가능성 ⚠
    ③ 태그 파싱 실패로 delta=0인데 실제 태그 있음 → contains로 잡힘 ✅

v1.1.0 수정:
  affinity_tag_found = raw_response.contains("[affinity:")

  이유: affinity_tag_found는 "LLM이 태그를 출력했는가"를 측정하는 지표.
        delta 값과는 별개로, 원시 응답에 태그 문자열이 존재하는지만 확인하면 충분.
        delta 값은 별도 affinity_value 필드에서 추적.
```

---

## 9. 성공 기준

| 단계 | 기준 | 검증 방법 |
|------|------|----------|
| Phase 1 | 3턴 Mock 벤치마크에서 TurnTrace 3개 생성 | 단위 테스트 |
| Phase 1 | detailed JSON에 llm_messages + llm_raw_response 포함 | JSON 파싱 테스트 |
| Phase 1 | LLM 에러 시 TurnTrace.error 채워짐 | MockLlm 에러 주입 테스트 |
| Phase 1 | forbidden_words가 정확히 추출됨 | 금지어 포함 응답 테스트 |
| Phase 2 | 기억 3개 주입 시나리오에서 검색 결과 기록 | InMemory 테스트 |
| Phase 2 | 기억 검색 실패 시 에러 기록 | InMemory 에러 주입 테스트 |
| Phase 3 | Live LLM에서 TTFT, tok/s 기록 | 수동 확인 (soyeon_chat) |
| Phase 4 | --replay로 사람이 읽을 수 있는 출력 (에러 턴 포함) | 수동 확인 |
| Phase 4 | --export-csv로 §3.5 컬럼 명세 준수 CSV 생성 | CSV 파싱 테스트 |

---

## 변경 이력

| 버전 | 변경일시 | 변경 내역 |
|:---:|----------|-----------|
| v1.0.0 | 2026-02-24 16:00:00 | 초기 작성. 현황 분석(§1), 3계층 로깅 아키텍처 설계(§2), 4단계 구현 계획(§3), 저장 구조(§4), 활용 시나리오 4건(§5), 우선순위/일정(§6), Phase 1 코드 변경 상세(§7), 성공 기준(§8). |
| v1.1.0 | 2026-02-24 22:30:00 | 리뷰 반영 보완. ① Phase 1 TurnTrace에 llm_messages 필드 추가(§8.1), ② Phase 2 설계 선택지에 장단점 상세 기술(§3.2), ③ forbidden_words 매칭 방식 명세 추가(§2.3), ④ TurnError 구조체 + 에러 처리 전략 추가(§2.2, §3.1, §8.1, §8.2), ⑤ 성능 영향 분석 섹션 신설(§5), ⑥ --export-csv 컬럼 명세 추가(§3.5), ⑦ affinity_tag_found 판정 로직 수정(§8.4). 성공 기준 테스트 케이스 4건 추가(§9). |
