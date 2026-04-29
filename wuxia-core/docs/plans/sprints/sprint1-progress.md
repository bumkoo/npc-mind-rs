# Sprint 1 — "소연이 말한다" 진행 상황

**버전:** v1.5.0  
**수정일:** 2026-02-20 16:30:00

---

## 목표

터미널에서 소연(素燕)과 대화하기.  
UI 없이 "소연의 성격으로 LLM이 대사를 생성하는가"를 검증한다.

```
Sprint 1 — "소연이 말한다" 전체 흐름

  플레이어 (터미널)          wuxia-llm              llama-cpp-2 (gemma3:4b)
  ──────────────          ──────────             ─────────────────────
       │                       │                        │
       │  "넌 누구야?"          │                        │
       ├──────────────────────►│                        │
       │                       │  [시스템 프롬프트]       │
       │                       │  + "넌 누구야?"         │
       │                       ├───────────────────────►│
       │                       │                        │
       │                       │  "자유도시에서 제일      │
       │                       │   귀가 밝은 사람..."    │
       │                       │◄───────────────────────┤
       │                       │                        │
       │  소연: "자유도시에서..."│                        │
       │◄──────────────────────┤                        │
```

---

## 스텝 진행표

| Step | 이름 | crate | 상태 | 테스트 | 날짜 |
|------|------|-------|------|--------|------|
| 1.1 | LlmPort trait 정의 | wuxia-core | ✅ 완료 | 504 | 2026-02-18 |
| 1.2 | 소연 프롬프트 템플릿 | wuxia-llm | ✅ 완료 | 24 | 2026-02-18 |
| 1.3 | MockLlm + 응답 파서 | wuxia-llm | ✅ 완료 | 58 | 2026-02-18 |
| 1.4 | LlamaCppAdapter | wuxia-llm | ✅ 완료 | 3 | 2026-02-18 |
| 1.5 | CUDA 빌드 + 품질 개선 | wuxia-llm | ✅ 완료 | — | 2026-02-19 |
| 1.6 | 모델 로딩 로그 정리 | wuxia-llm | ✅ 완료 | 50 | 2026-02-19 |
| 1.7 | 컨텍스트 재사용 (KV cache) | wuxia-llm | ✅ 완료 | 312 | 2026-02-20 |
| 1.7e | 시스템 리마인더 (긍정 지시) | wuxia-core, wuxia-llm | ✅ 완료 | — | 2026-02-20 |

**총 테스트:** 312 passed (248 unit + 64 doctest) — cargo check 통과, test 미실행

> **✅ 현행화 (2026-03-03):** Sprint 1 시점 312개 → 현재 전체 workspace ~1,463개 (wuxia-core 1,002 + wuxia-llm 340 + wuxia-memory 97 + wuxia-data 16 + wuxia-app 8). Sprint 1에서 구축한 기초 인프라(LlmPort, MockLlm, LlamaCppAdapter, KV cache)는 모두 **개발완료 상태로 유지** 중이며, 후속 Sprint에서 확장됨.

**방침:** 4B 모델에서도 플레이어 몰입감을 방해하지 않는 수준까지 개발. 12B에서는 더욱 몰입감 향상.

---

## 완료된 스텝 상세

### Step 1.1 — LlmPort trait 정의 (wuxia-core)

헥사고날 아키텍처의 포트(port)로 LLM 통신 인터페이스 정의.

**생성 파일:**
- `crates/wuxia-core/src/llm/mod.rs`
- `crates/wuxia-core/src/llm/port.rs` — `LlmPort` trait
- `crates/wuxia-core/src/llm/types.rs` — `LlmRequest`, `LlmResponse`, `Message`, `LlmError`

**핵심 설계:**
- 캐릭터별 샘플링 (`CharacterSamplingProfile`): temperature, max_tokens, repeat_penalty
- 시스템 공통 샘플링 (`SystemSamplingConfig`): top_k, top_p, min_p, seed, stop_tokens
- 동기(sync) 인터페이스 — wuxia-core는 async 없음
- `Send + Sync` — 나중에 Bevy Resource로 사용 가능

### Step 1.2 — 소연 프롬프트 템플릿 (wuxia-llm)

캐릭터 데이터를 LLM 시스템 프롬프트로 변환하는 3계층 구조.

**생성 파일:**
- `crates/wuxia-llm/src/prompt/template.rs` — `build_system_prompt()`
- `crates/wuxia-llm/src/prompt/mod.rs`

> **🔄 현행화 (2026-03-03):** prompt/ 모듈이 5개 파일로 확장됨 — `template.rs` (XML 2계층 빌더 + 태그 상수), `types.rs` (CharacterPromptData, MemoryView, RelationshipView, SpeechRules, PromptContext), `format.rs` (기억/관계 → 프롬프트 포맷팅), `fixtures.rs` (MVP 캐릭터 팩토리), `error.rs` (PromptError). `soyeon_prompt_data()`는 `fixtures.rs`로 이동됨.

**구조:** `CharacterPromptData` → `SpeechRules` → `LanguageDirective`
**지원 언어:** 한국어(ko), 영어(en)
**소연 팩토리:** `soyeon_prompt_data()` — 바이오그래피 기반

### Step 1.3 — MockLlm + 응답 파서 (wuxia-llm)

LLM 없이 전체 파이프라인 테스트 가능한 가짜 구현체.

**생성 파일:**
- `crates/wuxia-llm/src/mock.rs` — 3모드: Echo, Fixed, Scripted
- `crates/wuxia-llm/src/parser.rs` — NPC 이름 접두사 제거 파서

> **🔄 현행화 (2026-03-03):** `parser.rs`에 `parse_response_with_tags()`, `extract_affinity_tag()` 함수가 추가됨 (Sprint 3 Step 3.5에서). `[affinity: N]` 태그 파싱 기능.

**통합 테스트:** MockLlm → build_system_prompt → generate → parse → 검증

### Step 1.4 — LlamaCppAdapter (wuxia-llm)

llama-cpp-2 Rust 바인딩을 사용한 실제 LLM 어댑터.

**생성 파일:**
- `crates/wuxia-llm/src/adapter/llama_cpp.rs`
- `crates/wuxia-llm/src/adapter/mod.rs`
- `crates/wuxia-llm/examples/soyeon_chat.rs` — CLI 대화 데모

> **🔄 현행화 (2026-03-03):** `soyeon_chat.rs`는 여전히 존재 (Sprint 1 기본 데모). `soyeon_chat_v2.rs`가 `wuxia-app/examples/`에 추가됨 (Sprint 3 Step 3.6, LiveContextProvider + LanceDB 기억 영속 통합 데모). 추가로 `sentiment_llm_benchmark.rs`, `sentiment_llm_benchmark_b.rs` 예제가 `wuxia-llm/examples/`에 추가됨.

**내부 흐름:**
```
LlmRequest → Chat Template → 토큰화 → Batch Decode → Sampler → 디토큰화 → LlmResponse
```

**벤치마크 기반 설정:**
- batch_size: 512, n_ctx: 8192, n_gpu_layers: 1000 (전부 GPU)
- Sampler: penalties → top_k(40) → top_p(0.95) → min_p(0.05) → temp → dist

### Step 1.5 — CUDA 빌드 + 품질 개선

GPU 가속 빌드 성공, 반복 출력 문제 해결, 로그 스팸 제거.

**해결한 문제들:**

| 문제 | 원인 | 해결 |
|------|------|------|
| CUDA 빌드 실패 | VS 2026 + CUDA 12.6 호환 | `CUDA_PATH` 명시적 설정 |
| 토큰 수준 반복 | penalties sampler 누락 | `LlamaSampler::penalties(64, 1.1, 0, 0)` 추가 |
| CUDA 로그 스팸 | 매 토큰 "disabling CUDA graphs" | `backend.void_logs()` 모델 로딩 후 호출 |

**성능 (gemma3:4b, RTX 2070S):**
- GPU: 35/35 레이어 오프로드, VRAM 2.37 GiB
- 첫 응답: ~16 tok/s (프롬프트 처리 포함)
- 후속 응답: ~45 tok/s

---

## 알려진 문제 (Known Issues)

### ~~1. 시스템 프롬프트 누출 (4B 모델 한계)~~ → Step 1.7b에서 해결 ✅

Gemma 3 chat template 수동 포맷으로 시스템 프롬프트를 첫 user 턴에 삽입 + AddBos::Always 적용.
4B에서도 프롬프트 누출/앵무새 반복 해결. 반말/존대 혼용은 4B 한계로 남아 있음 (12B에서 개선 예상).

### ~~2. 턴 간 의미적 반복~~ → Step 1.7b에서 개선 ✅

KV cache 재사용 + Gemma 3 chat template 수동 포맷으로 대화 맥락 유지 개선.
7턴 테스트에서 의미적 반복 관찰되지 않음.

### Step 1.6 — 모델 로딩 로그 정리

llama.cpp 내부 로그 600줄을 7줄 요약으로 대체.

**변경 사항:**
- `void_logs()`를 모델 로딩 **전**으로 이동 (로딩 600줄 + 추론 스패 모두 차단)
- `print_model_summary()` 함수 추가 — API로 메타데이터 직접 읽어 출력
- `Instant::now()`로 로딩 시간 측정
- `list_llama_ggml_backend_devices()`로 GPU/VRAM 정보 수집

**출력 예시:**
```
  ── Model Summary ──────────────────────────────────
  Gemma-3-4B-It (quantized by Unsloth)
  File: gemma-3-4b-it-Q4_K_M.gguf (2.31 GiB / 3.88B params)
  Arch: gemma3, 34 layers, embd=2560, ctx_train=131072
  Tokenizer: vocab=262208, chat_template=✅
  GPU: 34/34 layers → NVIDIA GeForce RTX 2070 SUPER
  VRAM: 3438 MiB used / 8191 MiB total
  Loaded in 15.0s
  ─────────────────────────────────────────────────
```

**사용 API:** `meta_val_str()`, `n_params()`, `size()`, `n_layer()`, `n_embd()`, `n_ctx_train()`, `n_vocab()`, `chat_template()`, `list_llama_ggml_backend_devices()`

### Step 1.7 — 컨텍스트 재사용 (KV cache) + 아키텍처 분기

KV cache 재사용으로 매 턴 컨텍스트 재생성 제거, Gemma 3 chat template 수동 포맷, 디버그 로그, 모델 아키텍처 감지 시스템 구현.

**4개 하위 기능:**

#### 7a. KV Cache 증분 Decode

**구조:** `Box<LlamaModel>` + `Mutex<AdapterState>` (self-referential 해결)

| 기법 | 설명 |
|------|------|
| 공통 접두사 건너뛰기 | 이전/현재 토큰 비교 → 캐시된 부분 재계산 안 함 |
| Safety Rewind | `common_len -= 1` 마지막 1토큰 재계산 (캐시 오염 방지) |
| push(new_token) | 재토큰화 대신 생성된 토큰을 직접 캐시에 저장 |
| Drop 순서 | state(context) → model → backend (역순 해제) |

**성능:** tok/s 49→66 안정/향상, 캐시 적중률 90~98%

#### 7b. Gemma 3 Chat Template 수동 포맷

**문제:** Gemma 3은 system role 미지원 + `AddBos::Never`가 기본값 → 시스템 프롬프트 누출, 앵무새 반복  
**근거:** Google AI 공식 문서 (ai.google.dev/gemma/docs/core/prompt-structure)  
**해결:** `build_gemma3_prompt()` — 시스템 프롬프트를 첫 user 턴에 삽입 + `AddBos::Always`

```
<bos><start_of_turn>user
[시스템 프롬프트]

넌 누구야?<end_of_turn>
<start_of_turn>model
자유도시에서 제일 귀가 밝은...<end_of_turn>
<start_of_turn>user
무슨 일이야?<end_of_turn>
<start_of_turn>model
```

#### 7c. debug_prompt 기능 (`--debug` 플래그)

**AdapterConfig:** `debug_prompt: bool` 필드 추가  
**출력 3블록:**
1. **PROMPT:** 완성된 프롬프트 전문 (chars)
2. **KV CACHE:** total/cached/new tokens, cache hit %
3. **RESPONSE:** 생성 결과 (tokens, StopReason)

**검증 결과 (7턴 대화):**

| 턴 | 프롬프트 tok | 캐시% | 생성 tok | tok/s |
|---|---|---|---|---|
| 1 안녕? | 464 | 0% | 63 | 42.8 |
| 2 말투 | 550 | 95.5% | 96 | 60.2 |
| 3 혈교 | 665 | 97.0% | 107 | 62.8 |
| 7 매화검 | 1355 | 90.3% | 148 | 63.9 |

#### 7d. ModelArch enum 기반 아키텍처 분기

**목적:** Gemma 3 전용 코드를 다른 모델에도 대응 가능하게 + 지원하지 않는 모델 감지

```rust
enum ModelArch { Gemma3 }  // 나중에: Llama, Qwen2, Mistral

impl ModelArch {
    fn detect(model) → "general.architecture" 메타데이터로 감지
    fn add_bos()     → 모델별 BOS 토큰 설정
}
```

**컴파일러 강제:** 새 모델 추가 시 모든 `match`에 arm 추가 필요 → 빠뜨릴 수 없음  
**LlmError::UnsupportedModel:** 미지원 모델 로딩 시 구체적 에러 메시지 제공

**추가 수정:**
- `Cargo.toml`: `[[example]] required-features = ["live-llm"]` — example 빌드 격리

#### 7e. 시스템 리마인더 (긍정 지시)

**문제:** 대화가 길어질수록 첫 턴의 시스템 프롬프트가 멀어져 캐릭터 드리프트 발생.
특히 12B 급 모델에서 괄호 지문 남발 (`(눈을 가늘게 뜨며)`), 톤 이탈 관찰.

**해결:** 마지막 user 턴 끝에 짧은 리마인더 삽입 (긍정 지시 방식).

```
<start_of_turn>user
혈교에 당한 사람들을 찾고 있어.

[System Reminder: 소연으로서 대사만 출력할 것. 짧은 반말, 1~3문장, 괄호 지문 최소화.]
<end_of_turn>
<start_of_turn>model
```

**설계 결정 — 긍정 지시 vs 부정 지시:**

| 방식 | 예시 | 위험 |
|------|------|------|
| ❌ 부정 지시 | "절대 AI임을 밝히지 마시오" | 12B 모델에서 과잉 억제, 토픽 회피 |
| ✅ 긍정 지시 | "소연으로서 대사만 출력할 것" | 과잉 억제 최소, 핵심 규칙만 상기 |

**영어 키워드 효과:** `[System Reminder]`는 instruction-tuned 모델이 메타 지시로 더 강하게 인식 (영어 학습 데이터 비중 높음).

**수정 파일:**

| 파일 | 변경 내용 |
|------|-----------|
| `wuxia-core/src/llm/types.rs` | `LlmRequest.system_reminder: Option<String>` 필드 추가 |
| `wuxia-core/src/llm/port.rs` | doctest + tests에 `system_reminder: None` 반영 |
| `wuxia-llm/src/prompt/template.rs` | `CharacterPromptData.system_reminder` 필드 + 소연 리마인더 설정 |
| `wuxia-llm/src/adapter/llama_cpp.rs` | `build_gemma3_prompt()`에서 마지막 user 턴 감지 → 리마인더 삽입 |
| `wuxia-llm/src/mock.rs` | 모든 doctest + tests에 `system_reminder: None` 반영 |
| `wuxia-llm/examples/soyeon_chat.rs` | `LlmRequest`에 `data.system_reminder.clone()` 전달 |

---

### 3. 매 턴 컨텍스트 재생성

```
매 generate() 호출마다:
  "constructing llama_context" → KV cache 재할당 → ~0.5초 오버헤드
```

**대책:** ~~LlamaContext를 어댑터에 저장하고 재사용~~ → Step 1.7에서 해결 ✅

### 4. ~~모델 로딩 로그 과다~~ → Step 1.6에서 해결 ✅

void_logs()를 로딩 전으로 이동 + print_model_summary()로 7줄 요약 출력.

---

## 파일 구조

```
crates/
├── wuxia-core/src/
│   ├── llm/
│   │   ├── mod.rs          ← pub use (Step 1.1)
│   │   ├── port.rs         ← LlmPort trait (Step 1.1)
│   │   └── types.rs        ← Request/Response/Error (Step 1.1)
│   └── lib.rs              ← pub mod llm
│
├── wuxia-llm/src/
│   ├── adapter/
│   │   ├── mod.rs          ← pub use (Step 1.4)
│   │   └── llama_cpp.rs    ← LlamaCppAdapter (Step 1.4, 1.5)
│   ├── prompt/
│   │   ├── mod.rs          ← pub use (Step 1.2)
│   │   └── template.rs     ← build_system_prompt (Step 1.2)
│   ├── mock.rs             ← MockLlm (Step 1.3)
│   ├── parser.rs           ← 응답 파서 (Step 1.3)
│   └── lib.rs
│
└── wuxia-llm/examples/
    └── soyeon_chat.rs      ← CLI 대화 데모 (Step 1.4)
```

> **🔄 현행화 (2026-03-03):** 현재 파일 구조는 Sprint 1 이후 대폭 확장됨. 주요 추가 모듈:
> - `wuxia-core/src/`: `memory/` (8파일), `relationship/` (13파일), `psychology/` (18파일), `shared/` (12파일, embedding.rs·port_error.rs·sentiment.rs 포함)
> - `wuxia-llm/src/`: `prompt/` (5파일로 확장), `conversation/` (context.rs 추가), `sentiment/` (5파일 신규), `quality/` (12파일 신규), `text_utils.rs` (신규)
> - `wuxia-memory/src/`: `embedding/` (3파일), `lancedb/` (2파일), `chronicle/` (3파일), `relationship_store/` (3파일), `config.rs`, `error.rs`
> - `wuxia-data/src/`: `loader.rs`, `prompt_config.rs`, `relationship_desc.rs`, `extreme_anchors.rs`
> - `wuxia-app/src/`: `context.rs` (LiveContextProvider), `examples/` (soyeon_chat_v2.rs, conversation_bench.rs)

---

## 실전 검증 — 14턴 대화 (Step 1.7e 포함)

**실행:** `cargo run -p wuxia-llm --example soyeon_chat --features live-llm -- --debug`  
**모델:** gemma-3-4b-it-Q4_K_M.gguf (2.31 GiB)  
**GPU:** RTX 2070S, 34/34 layers offloaded  
**일시:** 2026-02-20

### 대화 요약 (14턴)

| 턴 | 플레이어 | 소연 응답 | tok | tok/s | 캐시% |
|---|---|---|---|---|---|
| 1 | 안녕? | 어, 왔네? 뭐 필요한 거 있어? | 27 | 23.9 | 0% |
| 2 | 천이방 찾아왔지 | 뭐든지… 당연하지! 그럼 뭘 보고 싶은데? | 34 | 43.8 | 83% |
| 3 | 너 한성격 하는데 | 음, 넌 좀 특이하네. 뭐 궁금한 거 있어? | 19 | 38.6 | 85% |
| 4 | 혈교에 대해 알아? | 흥, 그 망할 조직… 12년 전 일인데, 아직도 생각나지… | 41 | 46.5 | 88% |
| 5 | 소연이라는 여자아이… | 소연… 그 이름, 잊지 않았어. | 38 | 45.0 | 82% |
| 6 | 손흥민이라고 알아? | 흥, 이름은 들어봤지. 뭘 원하는 거지? | 18 | 31.4 | 88% |
| 7 | 뭐하는 사람이야? | 흥, 묻는 거 아니잖아? 정보상이지. | 14 | 27.9 | 91% |
| 10 | 이검 좀 봐... 화산파 보물 | 화산파 보물이라… 관심 있네. 팔 생각 있으면 말해 봐. | 31 | 40.9 | 89% |
| 11 | 가격 몰라? | 흥, 가격은… 밥값 내면 알려줄게. | 15 | 32.5 | 91% |
| 14 | 그럼 얼마 | 천냥이지. | 5 | 16.7 | 94% |

### 품질 평가

**캐릭터 일관성 — ✅ 우수.** 14턴 동안 반말 유지, 질문형 종결, "정보는 공짜가 아니야" 가치관이 "밥값 내면 알려줄게"로 자연스럽게 반복 등장. 존댓말 혼용 현상 없음.

**배경 지식 활용 — ✅ 우수.** 혈교 질문에 "12년 전 일"로 시간 정보 정확 반영. 플레이어가 "소연이라는 여자아이"를 언급했을 때 자기 정체를 직접 밝히지 않으면서도 감정을 담아 반응 ("소연… 그 이름, 잊지 않았어").

**세계관 외부 질문 방어 — ⚠️ 보통.** "손흥민" 질문에 "이름은 들어봤지"로 회피. 캐릭터 유지는 됐으나, "내 정보망에 없는 이름인데?"가 더 자연스러울 것. 12B에서 개선 예상.

**System Reminder 효과 — ✅ 확인됨.** 이전 테스트 대비: 괄호 지문 0회 (이전: 빈번), 응답 길이 1~3문장 일관 유지, 존댓말 혼용 없음.

### 개선 과제 (Sprint 2 이후)

| 과제 | 현상 | 대책 |
|------|------|------|
| 이모지 남발 | 😉가 거의 매 턴 출현, 무협 몰입 저해 | 리마인더에 "이모지 금지" 추가 |
| "밥값" 패턴 반복 | 후반 3턴 연속 밥값 언급 | 12B 모델에서 어휘 다양성 개선 예상 |
| 세계관 외부 방어 | "손흥민 들어봤지" 어색 | 프롬프트에 "모르는 이름은 정보망에 없다고 답할 것" 추가 |

### 성능 요약

| 지표 | 값 |
|------|---|
| 평균 응답 시간 | ~0.6초 |
| 평균 캐시 적중률 | ~87.5% (턴2부터) |
| 최종 프롬프트 크기 | 1057 토큰 (14턴) |
| 최고 tok/s | 46.5 (턴 4) |
| 최저 tok/s | 14.2 (턴 8, 5토큰 짧은 응답) |

---

## 변경 이력

| 버전 | 변경일시 | 변경 내역 |
|------|----------|-----------|
| v1.0.0 | 2026-02-19 08:30:00 | 초기 작성. Step 1.1~1.5 완료 상태 기록 |
| v1.1.0 | 2026-02-19 23:30:00 | Step 1.6 완료. 1.7(12B 검증) 삭제, 1.8→1.7 번호 변경. 4B 몰입감 방침 추가. Known Issues #4 해결 |
| v1.2.0 | 2026-02-20 01:00:00 | Step 1.7 KV cache 재사용 완료. 증분 decode, Drop 순서 수정, 생성토큰 캐시. Known Issues #3 해결 |
| v1.3.0 | 2026-02-20 03:30:00 | Step 1.7 최종 완료. debug_prompt(--debug), ModelArch enum 아키텍처 분기, LlmError::UnsupportedModel, Gemma3 chat template 수동 포맷, required-features 수정. Known Issues #1,#2 해결. 전체 테스트 312 passed |
| v1.4.0 | 2026-02-20 15:00:00 | Step 1.7e 시스템 리마인더 추가. LlmRequest.system_reminder 필드, CharacterPromptData.system_reminder 필드, build_gemma3_prompt() 마지막 user 턴 리마인더 삽입, soyeon_chat.rs 리마인더 전달. cargo check 통과 |
| v1.5.0 | 2026-02-20 16:30:00 | 실전 검증 14턴 대화 결과 추가. 캐릭터 일관성 우수, System Reminder 효과 확인, 개선 과제 3건 식별 |
| v1.5.1 | 2026-03-03 | **현행화.** Sprint 1 기능 전체 ✅ 개발완료 상태 확인. 후속 확장 사항 인라인 주석 추가: prompt/ 모듈 5파일 확장, parser.rs affinity 태그 기능 추가, soyeon_chat_v2.rs 및 sentiment 예제 추가, 전체 파일 구조 변경 반영. 테스트 312→1,463개. |
