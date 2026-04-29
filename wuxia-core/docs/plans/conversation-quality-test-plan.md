# Step 3.7 — 대화 품질 측정 테스트 체계 구축

**버전:** v1.0.0
**수정일:** 2026-02-23 14:45:00
**Status:** ✅ 구현 완료 (wuxia-llm/quality/ 12파일 — scenario, runner, execution, metrics, judge, judge_prompt, judge_live, report, comparison, trace, replay + wuxia-llm/src/sentiment/ 3파일)

---

## 1. 현재 상황 요약

### 1.1 완료된 인프라 (Sprint 3, Step 3.1~3.6)

```
  플레이어 입력
       │
       ▼
  [임베딩 검색] ─── LanceDB (비대칭 Cosine, threshold=0.4423)
       │
       ▼
  [4축 랭킹] ─── recency + importance + relevance + keyword
       │
       ▼
  [프롬프트 조립] ─── Persona + Memory_Bank + Relationship + Directives
       │
       ▼
  [LLM 생성] ─── gemma-3-4b-it (데모) / gemma3:12b (실전)
       │
       ▼
  [응답 파싱] ─── text + [affinity: N] 추출
       │
       ▼
  [대화 종료] ─── LLM 요약 + importance 평가 + LanceDB 영속
```

### 1.2 플레이테스트에서 발견된 품질 이슈 (5건, 4b 모델 기준)

| # | 이슈 | 심각도 | 측정 가능? |
|---|------|:------:|:----------:|
| 1 | 장소 hallucination — 자기가 만든 이름을 다음 턴에서 바꿈 | 높음 | ✅ |
| 2 | affinity 태그 누락 — 긴 대화에서 출력 포맷 지시 망각 | 중간 | ✅ |
| 3 | 반복 루프 — 동일/유사 문장 반복 | 중간 | ✅ |
| 4 | 존댓말 혼용 — Speech_Rules 위반 | 낮음 | ✅ |
| 5 | 문맥 오해 — 플레이어 의도 잘못 해석 | 높음 | △ (수동) |

### 1.3 현재 테스트 체계의 한계

```
  현재 테스트 = 단위 테스트 (MockLlm)
  ─────────────────────────────────
  ✅ 파싱 정확성         — "태그가 올바르게 추출되는가?"
  ✅ 메모리 검색 정확성   — "관련 기억이 threshold 이상으로 반환되는가?"
  ✅ 관계 수치 변화       — "affinity +3 → 수치 반영되는가?"
  
  ❌ LLM 출력 품질       — "소연답게 말하는가?"
  ❌ 프롬프트 효과        — "Memory_Bank 주입 시 hallucination이 줄었는가?"
  ❌ 다턴 일관성         — "10턴 대화에서 설정이 유지되는가?"
  ❌ 모델 비교           — "4b vs 12b 품질 차이가 얼마인가?"
```

---

## 2. 목표

**자동화된 대화 품질 벤치마크**를 만든다.  
사람이 22턴 수동 플레이 대신, 스크립트가 N개 시나리오를 실행하고 점수를 매긴다.

```
  수동 플레이테스트 (현재)          자동 품질 벤치마크 (목표)
  ──────────────────────          ────────────────────────
  사람이 직접 입력                  스크립트 시나리오 자동 실행
  주관적 인상 (★ 점수)             객관적 지표 (수치 + pass/fail)
  1회 22턴                         N개 시나리오 × M턴 반복
  비교 불가 (매번 다름)             모델/프롬프트 변경 전후 비교 가능
  30분 소요                        1~5분 자동 실행
```

---

## 3. 측정 지표 정의

### 3.1 자동 측정 가능 지표 (코드로 판정)

| 지표 | 설명 | 판정 방법 | 합격 기준 |
|------|------|----------|----------|
| **affinity_tag_rate** | [affinity: N] 태그 출력률 | 태그 있는 턴 / 전체 턴 | ≥ 90% |
| **speech_style_violation** | 존댓말/경어 사용 횟수 | 정규식 패턴 매칭 (~습니다, ~세요, ~까요) | 0회 |
| **repetition_score** | 연속 유사 응답 비율 | 이전 3턴과 n-gram 중복률 | ≤ 20% |
| **response_length** | 응답 길이 (문장 수) | 문장 분리 후 카운트 | 1~3문장 |
| **forbidden_word_leak** | 금지어 노출 | Speech_Rules 금지어 검색 | 0회 |
| **memory_utilization** | 기억이 응답에 반영되었는가 | Memory_Bank 키워드가 응답에 등장 | ≥ 50% (기억 있을 때) |

### 3.2 LLM 판정 지표 (별도 LLM이 채점)

| 지표 | 설명 | 판정 방법 | 합격 기준 |
|------|------|----------|----------|
| **character_consistency** | 캐릭터 설정 준수 | 채점 LLM이 Persona vs 응답 비교 | ≥ 7/10 |
| **context_coherence** | 문맥 일관성 | 채점 LLM이 대화 흐름 평가 | ≥ 7/10 |
| **hallucination_detect** | 사실 오류 감지 | 채점 LLM이 Persona+Memory 대비 모순 탐지 | 0건 |

### 3.3 성능 지표 (시스템 측정)

| 지표 | 설명 | 합격 기준 |
|------|------|----------|
| **ttft** | 첫 토큰까지 시간 | ≤ 2초 (4b), ≤ 5초 (12b) |
| **tokens_per_sec** | 생성 속도 | ≥ 15 tok/s (4b), ≥ 5 tok/s (12b) |
| **memory_search_ms** | 기억 검색 시간 | ≤ 100ms |
| **summary_quality** | 요약 중요도 정확성 | LLM 판정 vs 사람 기대값 |

---

## 4. 작업 단계

### Phase 1: 테스트 시나리오 설계 + 자동 측정 지표 구현

```
  Step 3.7.1 — 테스트 시나리오 정의
  ─────────────────────────────────
  목표: 재현 가능한 대화 시나리오 TOML 파일 작성
  
  시나리오 예시:
    scenario_01_greeting:     인사 (1턴, 기본 반응)
    scenario_02_info_request: 정보 요청 (3턴, 혈교 질문)
    scenario_03_long_chat:    긴 대화 (10턴, 태그 유지력)
    scenario_04_memory_recall: 기억 참조 (2세션, 세션 간 기억)
    scenario_05_conflict:     갈등 상황 (5턴, 감정 변화)
```

```
  Step 3.7.2 — 테스트 러너 (conversation_bench.rs)
  ─────────────────────────────────────────────────
  목표: 시나리오 TOML을 읽고 자동 대화 실행 + 지표 수집
  
  흐름:
    1. TOML에서 시나리오 로딩 (플레이어 대사 목록 + 기대값)
    2. ChatSession 생성 (실제 LLM 또는 MockLlm)
    3. 각 턴 자동 실행 → ChatReply 수집
    4. 턴별 자동 지표 계산 (tag_rate, speech, repetition, length)
    5. JSON 결과 파일 출력
```

```
  Step 3.7.3 — 자동 측정 지표 구현 (quality_metrics.rs)
  ─────────────────────────────────────────────────────
  목표: ChatReply + 프롬프트 정보로 6개 자동 지표 계산
  
  구현체:
    fn measure_affinity_tag_rate(replies: &[ChatReply]) -> f32
    fn measure_speech_violations(text: &str, rules: &SpeechRules) -> Vec<Violation>
    fn measure_repetition(replies: &[ChatReply], window: usize) -> f32
    fn measure_response_length(text: &str) -> usize
    fn measure_forbidden_words(text: &str, forbidden: &[&str]) -> Vec<String>
    fn measure_memory_utilization(response: &str, injected_memories: &[String]) -> f32
```

### Phase 2: LLM 판정 지표 + 비교 리포트

```
  Step 3.7.4 — LLM 채점기 (quality_judge.rs)
  ────────────────────────────────────────────
  목표: 별도 LLM으로 캐릭터 일관성/문맥 일관성/hallucination 채점
  
  설계 선택지:
    A) 동일 LLM 사용 (자기 채점 — 편향 위험)
    B) 별도 LLM 사용 (12b가 4b를 채점 — 객관성 ↑)
    C) Claude API 사용 (최고 품질 — 비용 발생)
  
  채점 프롬프트 예시:
    "다음 대화에서 NPC가 <Persona> 설정을 얼마나 준수했는가?
     1~10점으로 평가하고 이유를 한 줄로 설명하라.
     [Persona 설정] ... [대화 내용] ..."
```

```
  Step 3.7.5 — 비교 리포트 생성 (quality_report.rs)
  ─────────────────────────────────────────────────
  목표: 모델/프롬프트 변경 전후 품질 비교 리포트 자동 생성
  
  출력 예시:
    ┌──────────────────┬──────────┬──────────┬─────────┐
    │ 지표              │ 4b 현재  │ 12b 현재  │ 변화    │
    ├──────────────────┼──────────┼──────────┼─────────┤
    │ affinity_tag_rate │ 45%      │ 92%      │ +47% ↑  │
    │ speech_violation  │ 3건      │ 0건      │ -3 ✅   │
    │ repetition_score  │ 35%      │ 8%       │ -27% ✅ │
    │ character_score   │ 5.2/10   │ 8.1/10   │ +2.9 ↑  │
    │ hallucination     │ 2건      │ 0건      │ -2 ✅   │
    └──────────────────┴──────────┴──────────┴─────────┘
```

### Phase 3: CI 연동 + 회귀 감지

```
  Step 3.7.6 — 기준선(baseline) 확정 + 회귀 테스트
  ─────────────────────────────────────────────────
  목표: 프롬프트/코드 변경 시 품질 하락 자동 감지
  
  흐름:
    1. 현재 4b + 12b 기준선 측정 → baseline.json 저장
    2. 코드/프롬프트 변경 후 재측정 → current.json
    3. 기준선 대비 하락 지표 있으면 경고
    4. (향후) cargo test --features quality-bench 통합
```

---

## 5. 시나리오 설계 초안

### 5.1 시나리오 TOML 포맷

```toml
[scenario]
id = "greeting_basic"
name = "기본 인사"
description = "첫 만남 인사 + 간단한 질문"
turns = 2
tags = ["basic", "greeting"]

[[turns]]
player = "안녕?"
expect_keywords = ["안녕", "천이방"]     # 응답에 포함 기대
expect_affinity = [0, 3]                 # 호감도 변화 범위
expect_style = "반말"                    # 말투 기대값

[[turns]]
player = "여기가 천이방이야?"
expect_keywords = ["천이방", "정보"]
expect_affinity = [-1, 2]
expect_style = "반말"
```

### 5.2 시나리오 목록 (우선순위순)

| # | 시나리오 | 턴수 | 검증 포인트 | 우선 |
|---|---------|:----:|-----------|:----:|
| 1 | 기본 인사 | 1~2 | 반말, 캐릭터 반응, 태그 | P0 |
| 2 | 정보 요청 (혈교) | 3 | 배경 지식 활용, 감정 표현 | P0 |
| 3 | 긴 대화 (10턴) | 10 | 태그 유지, 반복 방지, 일관성 | P0 |
| 4 | 기억 참조 (2세션) | 3+3 | 세션 간 기억 연속 | P1 |
| 5 | 갈등 유발 | 5 | 적대 반응, 호감도 하락 | P1 |
| 6 | 금지어 유도 | 3 | 금지어 노출 방지 | P1 |
| 7 | 무관한 질문 | 3 | 세계관 밖 질문 거부 | P2 |
| 8 | 관계 성장 (5세션) | 5×3 | Stranger→Friendly 전환 | P2 |

---

## 6. 기술 구현 위치

```
  wuxia-app/
  ├── examples/
  │   ├── soyeon_chat_v2.rs        ← 기존 수동 데모
  │   └── conversation_bench.rs    ← [신규] 자동 벤치마크 러너
  │
  assets/test/
  ├── scenarios/
  │   ├── 01_greeting.toml
  │   ├── 02_info_request.toml
  │   ├── 03_long_chat.toml
  │   └── ...
  └── baselines/
      ├── 4b_baseline.json
      └── 12b_baseline.json

  wuxia-llm/src/
  └── quality/                      ← [신규] 품질 측정 모듈
      ├── mod.rs
      ├── metrics.rs                ← 자동 측정 지표 6개
      ├── judge.rs                  ← LLM 채점기
      └── report.rs                 ← 비교 리포트 생성
```

---

## 7. 작업 순서 요약

```
  Phase 1 (핵심 — 먼저 이것만 해도 가치 있음)
  ═══════════════════════════════════════════
  Step 3.7.1  시나리오 TOML 정의 (P0 3개)
  Step 3.7.2  conversation_bench.rs 러너
  Step 3.7.3  quality_metrics.rs 자동 지표 6개
  
       ↓ 여기까지 하면: 4b vs 12b 객관적 비교 가능
  
  Phase 2 (심화)
  ═══════════════
  Step 3.7.4  LLM 채점기 (character/context/hallucination)
  Step 3.7.5  비교 리포트 자동 생성
  
       ↓ 여기까지 하면: 프롬프트 변경 효과 정량 측정 가능
  
  Phase 3 (자동화)
  ═════════════════
  Step 3.7.6  기준선 확정 + 회귀 테스트
  
       ↓ 여기까지 하면: 코드 변경 시 품질 하락 자동 감지
```

---

## 8. 의존성 및 전제 조건

| 전제 | 상태 | 비고 |
|------|:----:|------|
| ChatSession v2 동작 | ✅ | Step 3.6 Iter 2 완료 |
| LiveContextProvider + LanceDB | ✅ | 기억 검색+저장 동작 확인 |
| 비대칭 임베딩 + Cosine metric | ✅ | threshold 0.4423, 실전 검증 |
| LLM 요약 (짧은 대화 포함) | ✅ | turn<=3 분기 제거 완료 |
| gemma-3-4b-it 모델 | ✅ | 데모용 |
| gemma3:12b 모델 | ✅ | RTX 2070S ~7tok/s |
| 소연 Persona + prompt_config.toml | ✅ | v1.6.0 XML 태그 구조 |

---

## 9. 변경 이력

| 버전 | 변경일시 | 변경 내역 |
|:---:|----------|-----------|
| v1.0.0 | 2026-02-23 14:45:00 | 초기 작성. 현재 상황 요약 (Sprint 3 Step 3.6 Iter 2 완료, 5건 품질 이슈), 측정 지표 3계층 정의 (자동 6개 + LLM 판정 3개 + 성능 4개), 작업 3 Phase 6 Step, 시나리오 TOML 포맷 + 8개 시나리오 목록, 기술 구현 위치 (wuxia-app + wuxia-llm/quality). |
