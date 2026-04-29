# Sprint 3 — "소연이 영원히 기억한다" 진행 상황

**버전:** v2.0.0  
**수정일:** 2026-02-23 15:00:00

---

## 목표

프로그램을 종료해도 소연이 기억한다.  
LanceDB 벡터 검색으로 의미 기반 기억을 구현하고,  
소연과의 관계(호감도/신뢰도)가 대화에 반영되는 플레이 가능한 데모를 만든다.

```
Sprint 3 — "소연이 영원히 기억한다" 전체 흐름

  플레이어               ChatSession            LanceDB            Embedding
  ─────────             ────────────           ─────────          ──────────
       │                      │                    │                  │
       │ "저번에 사파 얘기     │                    │                  │
       │  했잖아"             │                    │                  │
       ├─────────────────────►│                    │                  │
       │                      │  "사파 얘기" 임베딩  │                  │
       │                      ├───────────────────────────────────►  │
       │                      │  [0.12, -0.34, ...] 벡터 반환        │
       │                      │◄───────────────────────────────────  │
       │                      │                    │                  │
       │                      │  벡터 유사도 검색   │                  │
       │                      ├───────────────────►│                  │
       │                      │  "혈교 대화" 기억   │                  │
       │                      │  (유사도 0.87)      │                  │
       │                      │◄───────────────────┤                  │
       │                      │                    │                  │
       │                      │  프롬프트 조립      │                  │
       │                      │  시스템+[기억]+[관계]+대화              │
       │                      ├──────────────────────────────────► LLM
       │                      │  "그래, 혈교 이야기 했었지..."        │
       │                      │◄──────────────────────────────────   │
       │                      │                    │                  │
       │                      │  💾 대화 기억 저장  │                  │
       │                      │  (임베딩 + 메타데이터)                  │
       │                      ├───────────────────►│                  │
       │                      │                    │                  │
       │ 소연: "그래, 혈교..." │                    │                  │
       │◄─────────────────────┤                    │                  │
       │                      │                    │                  │
       │ (프로그램 종료)       │                    │                  │
       │                      │              [디스크에 영속]           │
       │                      │                    │                  │
       │ (다음날 다시 실행)    │                    │                  │
       │ "어제 무슨 얘기했지?" │                    │                  │
       ├─────────────────────►│  벡터 검색 ────────►│                  │
       │                      │  어제 기억 반환 ◄───┤                  │
       │ 소연: "어제 혈교..."  │                    │                  │
       │◄─────────────────────┤                    │                  │
```

---

## 선행 조건

- ✅ Sprint 1 완료 (LlmPort, LlamaCppAdapter, KV cache)
- ✅ Sprint 2 Phase A 완료:
  - MemoryEntry + MemoryRepository trait (wuxia-core)
  - InMemoryRepository (wuxia-memory)
  - retrieval_score() + OCC 훅 (wuxia-core)
  - PromptContext + build_system_prompt() 기억 삽입 (wuxia-llm)
  - ConversationManager ctx 압축 (wuxia-llm)
  - ChatSession 대화 루프 (wuxia-llm)
- ✅ soyeon_chat v1 (터미널 대화 동작)

---

## 스텝 진행표

| Step | 이름 | crate | 상태 | 테스트 | 날짜 | 현행화 (2026-03-03) |
|------|------|-------|------|--------|------|---------------------|
| 3.1 | EmbeddingPort + 모델 선정 | wuxia-core + wuxia-memory | ✅ 완료 | 기존 통과 | 2026-02-21 | 🔄 수정: EmbeddingPort 위치 memory/→shared/로 이동, 에러 String→PortError, embed_document()+model_name() 추가 |
| 3.2.1 | LanceDB 연결 + 스키마 + save/count | wuxia-memory | ✅ 완료 | 11 tests | 2026-02-21 | ✅ 유지 (lancedb.rs→lancedb/ 디렉토리로 변경, arrow_convert.rs 분리) |
| 3.2.2 | find_recent (metadata filter + sorting) | wuxia-memory | ✅ 완료 | +4 tests (15 total) | 2026-02-21 | ✅ 유지 |
| 3.2.3 | LanceDB 0.23→0.26 + Arrow 56→57 업그레이드 | wuxia-memory | ✅ 완료 | 52 tests passing | 2026-02-21 | ✅ 유지 |
| 3.3 | search (벡터 유사도) + threshold + config | wuxia-memory | ✅ 완료 | 740 tests passing | 2026-02-22 | 🔄 수정: 기본 임베딩 모델 gemma-qat→bge-m3로 변경, 프로파일 기반 config |
| 3.4 | Relationship 기본 (소연 1명분) | wuxia-core | ✅ 완료 | +41 tests (785 total) | 2026-02-23 | 🔄 **대폭 수정**: 3축→2축 모델, hostility ❌ 삭제, affinity -100~+100, 레벨 7→8단계(Wary 추가), 파일 4→13개 |
| 3.5 | ChatSession v2 (기억 영속 + 관계) | wuxia-llm + wuxia-core | ✅ 완료 | Iter1~4 ✅ | 2026-02-23~22 | 🔄 수정: ContextProvider 슬림화, skip_affinity_directive, 감정 판정 파이프라인 통합 |
| 3.6 | soyeon_chat v2 (플레이 가능 데모) | wuxia-app example | ✅ 개발완료 | Iter1~2 ✅ | 2026-02-22 | 🔄 수정: Iter3(관계 영속)→별도 모듈(relationship_store/chronicle)로 구현, Iter4(CLI) 일부 미구현 |
| 3.7 | 대화 품질 측정 테스트 체계 | wuxia-llm + wuxia-app | ✅ 개발완료 | 340 tests (wuxia-llm 전체) | 2026-02-23~ | ✅ **전체 구현**: quality/ 12파일, 시나리오 러너, 6 지표, LLM 채점기, 비교 리포트, 트레이스, 리플레이 |

---

## Step 상세

### Step 3.1 — EmbeddingPort + 임베딩 모델 선정 [wuxia-core + wuxia-memory]
> 텍스트를 벡터로 변환하는 포트를 정의하고, 한국어 임베딩 모델을 선정한다

> **🔄 현행화 (2026-03-03):** EmbeddingPort trait이 `memory/embedding.rs`에서 **`shared/embedding.rs`로 이동됨** (shared kernel으로 승격). 에러 타입 `Result<..., String>` → `Result<..., PortError>`로 변경. `embed_document()` 메서드 추가 (비대칭 모델 지원, 기본 구현은 `embed()`에 위임). `model_name()` 메서드 추가. `cosine_similarity()`, `l2_normalize()` 유틸리티 함수도 shared에 위치.

**3.1A — EmbeddingPort trait 정의 [wuxia-core]**

MemoryRepository, LlmPort와 동일한 헥사고날 패턴으로 임베딩 포트를 정의한다.

```rust
// wuxia-core/src/llm/embedding.rs (또는 memory/embedding.rs)

/// 텍스트 → 벡터 변환 포트.
///
/// 비유: 기억을 숫자로 바꾸는 서기관.
///   InMemory 시대에는 키워드로 찾았지만,
///   벡터 시대에는 "의미"로 찾는다.
///
/// 구현체:
///   - MockEmbedding: 테스트용 (고정 벡터 반환)
///   - FastEmbedAdapter: 로컬 경량 모델
///   - LlamaCppEmbedding: llama.cpp 임베딩 모드
pub trait EmbeddingPort: Send + Sync {
    /// 텍스트를 벡터로 변환.
    fn embed(&self, text: &str) -> Result<Vec<f32>, String>;
    
    /// 여러 텍스트를 한 번에 변환 (배치).
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String>;
    
    /// 벡터 차원 수.
    fn dimension(&self) -> usize;
}
```

```
  EmbeddingPort의 위치:

  wuxia-core (trait 정의)
       │
       ├──► wuxia-memory (LanceDbRepository가 사용)
       │         └── save() 시: text → embed() → 벡터 + 메타데이터 저장
       │         └── search() 시: query → embed() → 벡터 유사도 검색
       │
       └──► wuxia-llm (향후: ChatSession이 직접 사용할 수도)

  의존성: wuxia-core ← wuxia-memory (기존과 동일 방향)
```

**3.1B — 임베딩 모델 선정 + 벤치마크**

RTX 2070S 8GB에서 메인 LLM과 공존해야 한다는 제약이 핵심이다.

```
VRAM 예산 (RTX 2070S 8GB):

  gemma3n:e4b (메인 LLM):  ~3.9 GB
  CUDA 오버헤드:            ~0.3 GB
  KV Cache (2048 ctx):      ~0.3 GB
  ─────────────────────────────────
  남은 VRAM:                ~3.5 GB  ← 임베딩 모델 여기 안에
  
  또는: 임베딩은 CPU에서 돌리면 VRAM 부담 0
```

임베딩 모델 후보 3가지를 비교 벤치마크한다.

```
  후보 A: fastembed-rs (ONNX Runtime 기반)
  ────────────────────────────────────────
  모델: multilingual-e5-small (~130MB)
  장점: Rust 네이티브, CPU 최적화, 한국어 지원
  단점: 외부 ONNX 의존성
  VRAM: 0 (CPU 전용) 또는 GPU 가속 가능
  
  후보 B: candle (Hugging Face Rust ML)
  ────────────────────────────────────────
  모델: paraphrase-multilingual-MiniLM (~120MB)
  장점: 순수 Rust, HuggingFace 생태계
  단점: candle 빌드 복잡할 수 있음
  VRAM: 선택 가능 (CPU/GPU)
  
  후보 C: llama.cpp 임베딩 모드
  ────────────────────────────────────────
  모델: 메인 LLM의 임베딩 레이어 재활용
  장점: 추가 모델 불필요, llama-cpp-2 이미 있음
  단점: 메인 LLM과 VRAM 경쟁, 별도 임베딩용 모델 필요할 수 있음
  VRAM: 공유
```

벤치마크 평가 기준:

| 기준 | 가중치 | 설명 |
|------|:------:|------|
| 한국어 품질 | 5 | "사파" 검색 → "혈교" 기억 찾기 |
| 속도 | 4 | 대화 중 실시간 임베딩 (< 100ms 목표) |
| VRAM 사용 | 4 | 메인 LLM과 공존 가능한가 |
| Rust 통합 | 3 | Cargo 빌드 난이도, 의존성 |
| 벡터 차원 | 2 | 384~768 적정 (LanceDB 저장 효율) |

- [ ] 후보 3개 각각 설치 + "혈교/사파/정파" 한국어 유사도 테스트
- [ ] 속도 측정 (단일 문장, 배치 10문장)
- [ ] VRAM 사용량 측정
- [ ] 선정 결과 문서화 (sprint3-embedding-benchmark.md)

**3.1C — MockEmbedding 구현 [wuxia-memory]**

벤치마크와 병행하여 테스트용 Mock을 먼저 만든다.

```rust
// wuxia-memory/src/embedding/mock.rs

/// 테스트용 Mock 임베딩.
/// 키워드 해싱으로 결정론적 벡터를 생성한다.
/// 같은 텍스트 → 같은 벡터 (재현 가능)
pub struct MockEmbedding {
    dimension: usize,
}

impl EmbeddingPort for MockEmbedding {
    fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        // 해시 기반 결정론적 벡터 생성
        // 테스트에서 유사도 비교가 의미있게 동작
        Ok(hash_to_vector(text, self.dimension))
    }
    // ...
}
```

- [ ] MockEmbedding: 해시 기반 결정론적 벡터
- [ ] 같은 텍스트 → 동일 벡터 보장 (테스트 재현성)
- [ ] "혈교"와 "사파"가 "만두"보다 유사하게 나오도록 간단 해시 설계
- **테스트**: embed → 벡터 길이 확인, 동일 입력 동일 출력, embed_batch 동작

---

### Step 3.2 — LanceDbRepository 구현 [wuxia-memory]
> MemoryRepository trait을 LanceDB로 Iteration 방식으로 구현한다

**Iteration 진행 현황:**

| Iteration | 내용 | 상태 | 테스트 |
|-----------|------|------|--------|
| 1 | scaffold + save + count | ✅ | 11 tests |
| 2 | find_recent (filter + sort) | ✅ | +4 tests |
| 3 | search + update_importance | ✅ (Step 3.3에서 완료) | +7 tests |

**Step 3.2.1 — Iteration 1: scaffold + save + count [✅ 완료]**

구현 완료 내용:
- LanceDbRepository struct (table, embedder, runtime, id_counter, vector_dim)
- Arrow 스키마 12개 컨럼 (id, character_id, content, importance, memory_type, game_year/month/day, keywords, source_ids, reflection_tier, vector)
- entry_to_batch() — MemoryEntry + 벡터 → RecordBatch 1행
- batch_to_entries() — RecordBatch → Vec<MemoryEntry> (역변환)
- save(): embedder.embed() → RecordBatch → table.add()
- count(): SQL 필터(character_id) → 행 수 합산
- sync trait + block_on 브릿지 (tokio current_thread Runtime)

아키텍처 결정:
- Decision 1: sync trait + block_on 브릿지 (B안) — MemoryRepository는 sync, LanceDB는 async
- Decision 2: LanceDbRepository가 EmbeddingPort를 소유 — save() 시 자동 벡터 생성
- Decision 3: MVP 스키마 (MemoryEntry 필드만)

**Step 3.2.2 — Iteration 2: find_recent [✅ 완료]**

구현 완료 내용:
- character_id 필터로 LanceDB 쿼리 (only_if)
- batch_to_entries()로 도메인 타입 복원
- game_time 내림차순 정렬 (Rust 측)
- truncate(n)으로 상위 n개 반환

아키텍처 결정:
- Decision 4: 정렬을 Rust 측에서 수행 — LanceDB only_if는 ORDER BY 미지원, NPC 1명당 수백 건 수준이라 성능 문제 없음

**Step 3.2.3 — LanceDB 0.23→0.26 업그레이드 [✅ 완료]**

변경 내용:
- lancedb 0.23.1 → 0.26.2, arrow-array/arrow-schema 56 → 57
- lance-core 0.39.0 → 2.0.0, datafusion ^50.1 → ^51.0
- API 변경: `Box::new(RecordBatchIterator::new(...))` → `RecordBatchIterator::new(...)` 직접 전달
  - IntoArrow 제네릭 도입으로 Box<dyn RecordBatchReader> 불필요
  - 단, Vec<RecordBatch>는 IntoArrow 미구현 — RecordBatchIterator는 여전히 필요
- save() 반환 타입: () → AddResult — .map(|_| ()) 추가
- protoc 빌드 의존성 추가 (lance-encoding 2.0.0 요구)

빌드 의존성 추가:
- protoc (Protocol Buffers compiler) — lance-encoding 2.0.0이 빌드 시 요구
- 설치: `winget install Google.Protobuf`

---

#### Step 3.2 기존 계획 참고 (완료 — 아카이브)
> 아래는 초기 계획 내용이며, Iteration 3(search + update_importance)는 Step 3.3에서 완료되었다.

**3.2A — LanceDB crate 조사 + Cargo.toml 설정**

```toml
# wuxia-memory/Cargo.toml

[features]
live-db = ["dep:lancedb", "dep:arrow-array", "dep:arrow-schema"]

[dependencies.lancedb]
version = "0.16"     # 최신 버전 Context7에서 확인
optional = true

[dependencies.arrow-array]
version = "54"       # lancedb 호환 버전
optional = true

[dependencies.arrow-schema]
version = "54"
optional = true
```

- [ ] Context7에서 lancedb-rs 최신 문서 확인
- [ ] 최소 예제: DB 열기 → 테이블 생성 → 레코드 삽입 → 조회
- [ ] Windows + Rust 빌드 확인 (protobuf 등 시스템 의존성)

**3.2B — 스키마 정의**

```
  npc_memories 테이블 스키마:
  
  ┌───────────────┬────────────┬───────────────────────────┐
  │ 컬럼          │ 타입       │ 설명                       │
  ├───────────────┼────────────┼───────────────────────────┤
  │ id            │ String     │ MemoryId (고유 식별자)      │
  │ character_id  │ String     │ CharacterId (NPC)          │
  │ content       │ String     │ 기억 내용 (원문)            │
  │ importance    │ Float32    │ 중요도 (0.0~10.0)          │
  │ memory_type   │ String     │ Observation/Reflection/Plan │
  │ game_year     │ Int32      │ 게임 연도                   │
  │ game_month    │ Int32      │ 게임 월                     │
  │ game_day      │ Int32      │ 게임 일                     │
  │ keywords      │ String     │ 키워드 (JSON 배열 문자열)    │
  │ embedding     │ Vector(N)  │ 임베딩 벡터 (N=모델 차원)    │
  │ created_at    │ String     │ 실제 저장 시각 (ISO 8601)    │
  └───────────────┴────────────┴───────────────────────────┘
  
  GameTime을 3개 Int 컬럼으로 분리하는 이유:
  → LanceDB에서 범위 쿼리 가능 (game_year > 1200)
  → 구조체 직렬화보다 쿼리 최적화 우선
```

- [ ] `wuxia-memory/src/schema.rs` 생성
- [ ] Arrow 스키마 정의 (Schema::new with Fields)
- [ ] MemoryEntry ↔ Arrow RecordBatch 변환 함수
- [ ] RecordBatch → MemoryEntry 역변환 함수
- **테스트**: MemoryEntry → RecordBatch → MemoryEntry 라운드트립 검증

**3.2C — DB 연결 + 테이블 생성**

- [ ] `wuxia-memory/src/lancedb_adapter.rs` 생성
- [ ] LanceDbAdapter struct: Connection + Table 참조
- [ ] `new(db_path: &str) -> Result<Self>`: DB 열기/생성, 테이블 없으면 생성
- [ ] `open(db_path: &str) -> Result<Self>`: 기존 DB 열기
- [ ] DB 파일 경로: `./data/memory.lance` (기본값)
- **테스트**: DB 생성 → 재오픈 → 테이블 존재 확인 (임시 디렉토리 사용)

---

### Step 3.3 — search (벡터 유사도) + threshold + config [wuxia-memory] [✅ 완료]
> LanceDB 벡터 검색, 임베딩 모델 최종 선정, threshold 벤치마크, config 시스템을 구현한다

**핵심 성과:**
- LanceDB search()에 벡터 유사도 + 2-stage 필터링 구현
- embeddinggemma-300m-qat-Q8_0 최종 선정 (bge-m3에서 교체)
- 5개 모델 threshold 벤치마크로 언어별 최적값 도출
- EmbeddingConfig TOML 파서로 재컴파일 없이 모델/threshold 변경 가능
- MemoryEntry에 lang 필드 추가, LanceDB 스키마에 lang 컬럼 추가
- 전체 740 tests passing (workspace)

**Iteration 진행 현황:**

| Iteration | 내용 | 상태 | 비고 |
|-----------|------|:----:|------|
| Pre-1 | LanceDB search + update_importance | ✅ | lancedb.rs 벡터 검색 기본 구현 |
| Pre-2 | threshold_analyzer.rs 벤치마크 도구 | ✅ | 5모델×3언어 L2/L3/L4 분석 |
| Pre-3 | 모델 최종 선정 (F': gemma-qat-q8) | ✅ | bge-m3에서 교체 결정 |
| 1 | EmbeddingConfig + MemoryEntry lang | ✅ | config.rs, TOML 파서, lang 필드 |
| 2 | LlamaCppEmbedding task_prompt | ✅ | from_config() 팩토리 메서드 |
| 3 | LanceDB lang 컬럼 + 2-stage search | ✅ | Stage1: threshold, Stage2: keyword |
| 4 | 보고서 업데이트 | ✅ | step3.1 v2.1, step3.3 신규 |
| 5 | 테스트 호환성 확인 | ✅ | 변경 불필요, 740 tests pass |

**임베딩 모델 최종 선정 — F': embeddinggemma-300m-qat-Q8_0 (+task prompt)**

> **🔄 현행화 (2026-03-03):** 이후 **bge-m3가 active default 프로파일로 변경됨** (Bge-M3-567M-Q8_0, 1024차원, CPU-only, symmetric). gemma 프로파일은 대체 옵션으로 유지. `embedding.toml`이 프로파일 기반 구조로 변경됨 (`profiles.gemma` / `profiles.bge-m3` 전환 가능). `embedding-bge-m3.toml` 별도 설정 파일 추가. threshold 값도 프로파일별로 관리됨.

Step 3.1에서 bge-m3-Q4_K_M을 선정했으나, threshold 벤치마크 과정에서
embeddinggemma가 안전 마진에서 압도적 우위를 보여 교체 결정.

```
  선정 근거 6가지:
  ═══════════════════════════════════════════════════
  ① KO β (L2_min 기반 threshold) = 0.285  ← 가장 보수적
  ② 3언어 β 편차 = 0.028                  ← 가장 균일
  ③ L4 역전 = 0/24건                      ← 완전 방어
  ④ KO GAP = +0.0660                      ← 여유 확보
  ⑤ 벡터 차원 768                          ← bge-m3(1024) 대비 25% 절약
  ⑥ 단건 속도 28ms                         ← 실시간 임베딩 충분
  
  vs 이전 선정 (bge-m3-Q4_K_M):
  ┌──────────────┬──────────┬──────────┐
  │ 기준          │ bge-m3   │ gemma-qat│
  ├──────────────┼──────────┼──────────┤
  │ KO L4 역전   │ 4~5건    │ 0건 ✅   │
  │ KO margin    │ 약 0     │ +0.031   │
  │ 벡터 차원    │ 1024     │ 768 ✅   │
  │ VRAM         │ ~2.4GB   │ ~0.3GB ✅│
  └──────────────┴──────────┴──────────┘
```

**Threshold 값 (L2_min 기준, 관련 기억 0% 손실 보장):**

| 언어 | threshold | GAP | L4 역전 |
|:----:|:---------:|:---:|:-------:|
| KO | 0.4656 | +0.0660 | 0/8 |
| EN | 0.4580 | +0.0653 | 0/8 |
| ZH | 0.4668 | +0.0625 | 0/8 |

**2-Stage Search 구현:**

```
  Stage 1: Vector Similarity Filter
  ─────────────────────────────────
  cosine_sim >= threshold_for(lang)  → 통과
  cosine_sim <  threshold_for(lang)  → 탈락

  Stage 2: Keyword Overlap Boost
  ─────────────────────────────────
  cosine_sim >= threshold × boost_ratio(1.1)  → keyword 없어도 통과
  cosine_sim <  boost_threshold               → keyword overlap > 0 필요
  
  예시 (KO, threshold=0.4656, boost=0.5122):
    sim=0.55 + keyword=0 → 통과 (0.55 >= 0.5122)
    sim=0.48 + keyword=0 → 탈락 (0.48 < 0.5122, keyword=0)
    sim=0.48 + keyword=1 → 통과 (keyword > 0)
```

**EmbeddingConfig 시스템 (assets/ai/embedding.toml):**

```toml
[model]
name = "embeddinggemma-300m-qat-Q8_0"
file = "models/embeddinggemma-300m-qat-Q8_0.gguf"
family = "gemma"
dimension = 768
task_prompt = "task: search result | query: "

[threshold]
KO = 0.4656
EN = 0.4580
ZH = 0.4668
default = 0.4656

[search]
boost_ratio = 1.1
candidate_multiplier = 3
```

**MemoryEntry lang 필드:**
- `MemoryEntry::new()` → lang = "KO" 기본값
- `entry.set_lang("EN")` → 언어 변경 가능
- LanceDB 스키마에 `lang` 컬럼 추가 (String)
- search() 시 entry.lang()으로 threshold 자동 결정

**생성/수정된 파일:**
- `wuxia-memory/src/config.rs` — EmbeddingConfig TOML 파서 (신규)
- `wuxia-memory/src/embedding/llamacpp_adapter.rs` — from_config() 추가
- `wuxia-memory/src/lancedb.rs` — lang 컬럼, 2-stage search
- `wuxia-core/src/memory/types.rs` — MemoryEntry lang 필드
- `assets/ai/embedding.toml` — 설정 파일 (신규)
- `wuxia-memory/examples/threshold_analyzer.rs` — 벤치마크 도구 (신규)
- `docs/step3.1-embedding-benchmark-report.md` — v2.1.0 최종 선정 추가
- `docs/step3.3-threshold-analyzer-report.md` — 신규 v1.0.0

**참고 문서:**
- `docs/step3.1-embedding-benchmark-report.md` v2.1.0 — 모델 선정 상세
- `docs/step3.3-threshold-analyzer-report.md` v1.0.0 — threshold 벤치마크 상세

---

### Step 3.4 — Relationship 기본 (소연 1명분) [wuxia-core] [✅ 완료]
> 플레이어와 소연 사이의 호감도/신뢰도/적대도를 추적한다

> **🔄 현행화 (2026-03-03) — 대폭 수정:**
> - **3축→2축 모델로 변경:** affinity(-100.0~+100.0) + trust(0.0~100.0). ❌ **hostility 축 삭제** — affinity 음수 값이 적대를 표현.
> - **레벨 7→8단계:** Wary(경계) 레벨 추가. 판정 로직이 hostility 기반 → affinity 음수 기반으로 변경:
>   - `affinity <= -80` → Enemy, `affinity <= -40` → Hostile, `affinity <= -10` → Wary
>   - `affinity >= 80 AND trust >= 70` → Intimate, `>= 70 AND >= 50` → Close, `>= 50 AND >= 30` → Friendly, `>= 20 OR trust >= 20` → Acquaintance, else → Stranger
> - **파일 구조 4→13개:** types.rs에서 level.rs, trust_level.rs, relationship_type.rs 분리. 추가 파일: chronicle.rs (연대기), description.rs (관계 설명), sentiment.rs (감정 판정), sentiment_tests.rs, types_tests.rs
> - **새 타입:** `TrustLevel`(5단계), `RelationshipChronicle`, `ChangeType`, `CauseSource`, `ExtremeAnchorSet`, `TurnCounter`, `SentimentJudgment`, `DeltaSource`
> - **새 포트:** `ChronicleRepository` (연대기 저장), `RelationshipRepository` (관계 상태 저장)
> - 테스트: 41개 → 현재 114개 (wuxia-core relationship 영역)

**핵심 성과:**
- ~~3축 관계 모델 구현 (호감도+신뢰도+적대도, 각 0~100)~~ → 🔄 **2축 관계 모델 (affinity -100~+100, trust 0~100)**
- ~~적대 우선 판정 규칙~~ → 🔄 **affinity 음수 기반 적대 판정 (8단계)**
- RelationshipEvent 7종 + DomainEvent 통합
- RelationshipRepository trait (헥사고날 출력 포트)
- ~~전체 785 tests passing~~ → 현재 전체 ~1,463 tests passing

**설계 결정 3건:**

| 결정 | 선택 | 근거 | 현행화 |
|------|------|------|--------|
| 수치 축 구조 | ~~affinity+hostility+trust 3축 (각 0~100)~~ | ~~복합 감정 표현~~ | 🔄 **2축으로 변경** (affinity -100~+100, trust 0~100). hostility 삭제 — affinity 음수가 대체 |
| RelationshipType | enum 정의 + Option으로 포함 | None으로 시작→향후 사제/연인/적 전환 가능 | ✅ 유지 (8종: MasterDisciple, Siblings, Rivals, Allies, FamilyBond, Lovers, SwornSiblings, Enemies) |
| Level 판정 기준 | ~~affinity+trust+hostility 복합 판정, 적대 우선~~ | ~~소연 퀘스트 트리거 대응~~ | 🔄 **2축 기반 8단계 판정**으로 변경 (Wary 추가) |

**~~3축~~ 2축 관계 모델:**

```
  호감도 (affinity)  -100~+100 — "이 사람이 좋은가?" (음수=적대)
  신뢰도 (trust)     0~100     — "이 사람을 믿을 수 있는가?"
  ❌ 적대도 (hostility) — 삭제됨 (affinity 음수가 대체)

  예시:
    소연↔사부: 호감+80 + 신뢰90  = 깊은 유대 (Intimate)
    설화↔바투: 호감-20 + 신뢰20  = 경계 (Wary)
    소연↔조고: 호감-90 + 신뢰0   = 원수 (Enemy)
```

**RelationshipLevel 판정 (2축 기반, 8단계):**

```
  affinity <= -80                     → Enemy       (원수)
  affinity <= -40                     → Hostile     (적대)
  affinity <= -10                     → Wary        (경계) — 신규 추가
  affinity >= 80 AND trust >= 70      → Intimate    (깊은 유대)
  affinity >= 70 AND trust >= 50      → Close       (가까운 사이)
  affinity >= 50 AND trust >= 30      → Friendly    (친근)
  affinity >= 20 OR  trust >= 20      → Acquaintance (아는 사이)
  else                                → Stranger    (모르는 사이)
```

**Iteration 진행 현황:**

| Iteration | 내용 | 상태 | 테스트 |
|-----------|------|:----:|--------|
| 1 | types.rs — Relationship, RelationshipType, RelationshipLevel | ✅ | 20 tests |
| 2 | event.rs — RelationshipEvent 7종 + DomainEvent 통합 | ✅ | +11 tests |
| 3 | port.rs — RelationshipRepository trait | ✅ | +10 tests |

**생성/수정된 파일:**
- `wuxia-core/src/relationship/mod.rs` — 모듈 선언 + re-export (신규)
- `wuxia-core/src/relationship/types.rs` — Relationship, RelationshipType, RelationshipLevel (신규)
- `wuxia-core/src/relationship/event.rs` — RelationshipEvent 7종 (신규)
- `wuxia-core/src/relationship/port.rs` — RelationshipRepository trait (신규)
- `wuxia-core/src/shared/id.rs` — RelationshipId 추가
- `wuxia-core/src/shared/event.rs` — DomainEvent::Relationship + From 구현
- `wuxia-core/src/lib.rs` — pub mod relationship 추가

> **🔄 현행화 (2026-03-03):** 현재 relationship/ 모듈은 13개 파일:
> - `types.rs` — Relationship 어그리게이트 (2축: affinity, trust), `types_tests.rs`
> - `level.rs` — RelationshipLevel (8단계, types.rs에서 분리)
> - `trust_level.rs` — TrustLevel (5단계, 신규)
> - `relationship_type.rs` — RelationshipType (8종, types.rs에서 분리)
> - `event.rs` — RelationshipEvent
> - `port.rs` — RelationshipRepository + ChronicleRepository (포트 2개)
> - `effect.rs` — ConversationEffect + apply_conversation_effect()
> - `chronicle.rs` — RelationshipChronicle, ChangeType, CauseSource (신규)
> - `description.rs` — LocalizedDesc, RelationshipDescriptions (신규)
> - `sentiment.rs` + `sentiment_tests.rs` — ExtremeAnchorSet, TurnCounter, SentimentJudgment, DeltaSource, judgment_to_delta (신규)

**MVP 제외 (향후 Phase 3):**
- RelationshipMap (1:N 관계 관리)
- InteractionType enum (대화/선물/수련/전투/구출/배반/간호)
- apply_time_decay (미접촉 소원해짐)
- 관계별 피로 회복 계수 (연인 -20/일, 적대 +5/일)
- 의형제 맹약 시스템, 세력 가입 유형
- 배반 감지 (심리+관계 조합)

---

### Step 3.5 — ChatSession v2 (기억 영속 + 관계 반영) [wuxia-llm]
> ChatSession에 영속 기억과 관계를 연결한다

Sprint 2의 ChatSession은 base_memories(고정 Vec<String>)를 받았다. Sprint 3에서는 MemoryRepository에서 실시간으로 기억을 검색하고, 대화 결과를 기억으로 저장한다.

```
  ChatSession v1 (Sprint 2)           ChatSession v2 (Sprint 3)
  ──────────────────────              ──────────────────────────
  base_memories: Vec<String>          memory_repo: MemoryRepository
  (고정, 외부에서 주입)                (매 턴 검색 + 대화 종료 시 저장)
  
  관계 정보 없음                      relationship: Relationship
                                      (대화 결과 → 호감도/신뢰도 변화)
  
  send() → LLM 호출만                send() → 기억 검색 → LLM → 기억 저장
  end() → 요약 텍스트 반환            end() → Observation 저장 → 관계 업데이트
```

**send() v2 흐름:**

```
  1. user_input 받기
  2. memory_repo.search(character_id, user_input, top_k=5)
     → 관련 기억 검색 (벡터 유사도)
  3. rank_memories() → 최종 순위 결정
  4. format_memories_for_prompt() → 프롬프트용 문자열
  5. 관계 상태를 system_reminder에 삽입
     "[관계 상태: 호감도 +35, 신뢰도 42, 친밀한 사이]"
  6. 기존 send() 로직 (압축 → 프롬프트 → LLM → 파싱)
  7. 반환
```

**end() v2 흐름:**

```
  1. 대화 요약 생성 (기존 로직)
  2. Observation으로 memory_repo.save()
     - content: 대화 요약
     - importance: 대화 길이/감정 강도 기반
     - keywords: 핵심 키워드 추출
  3. 관계 업데이트
     - 대화 톤 분석 → affinity 변화
     - 정보 공유 여부 → trust 변화
```

**Iteration 진행 현황:**

| Iteration | 내용 | 상태 | 테스트 |
|-----------|------|:----:|--------|
| 1 | 3계층 관계 설명 아키텍처 (TrustLevel/HostilityLevel + descriptions.toml + 프롬프트 통합) | ✅ | +25 tests (820 total) |
| 2 | ContextProvider trait 분리 (ChatSession<L> → ChatSession<L,C>) | ✅ | +8 tests (context) + 21 session 호환 |
| 3 | parse_response_with_tags + ChatReply 확장 + LiveContextProvider + send() delta 반영 | ✅ | 작업①②③④ 완료 |
| 4 | Core Domain Service 추출 + ChatSession 리팩터링 (recall_memories, apply_conversation_effect, ContextProvider 슬림화, SessionEndResult) | ✅ | recall 6 + effect 13 + session 3 신규 |

**Iteration 1 — 3계층 관계 설명 아키텍처 [✅ 완료]**

상세 내용은 v0.6.0 변경 이력 참조.

**Iteration 2 — ContextProvider trait 분리 [✅ 완료]**

ChatSession에서 컨텍스트 공급 책임을 ContextProvider trait으로 분리.
base_memories: Vec<String> 삭제 → context_provider: C 필드로 교체.

```
  ChatSession v1 (Sprint 2)           ChatSession v2 (Iter 2)
  ──────────────────────              ────────────────────────
  ChatSession<L: LlmPort>            ChatSession<L: LlmPort, C: ContextProvider>
  base_memories: Vec<String>          context_provider: C
  send(): self.base_memories.clone()  send(): ctx.search_memories(user_input)
  관계 정보 없음 (항상 None)         send(): ctx.relationship_summary()
```

ContextProvider trait:
```rust
pub trait ContextProvider {
    fn search_memories(&self, query: &str) -> Vec<String>;
    fn relationship_summary(&self) -> Option<String>;
}
```

구현체 3종:
- NullContextProvider — 테스트용 (기억 없음, 관계 없음, vec![] 대체)
- StaticContextProvider — Sprint 2 호환 (고정 기억 + 선택적 관계)
- LiveContextProvider — Sprint 3 핵심 (Iteration 3에서 구현 예정)

생성/수정된 파일:
- `wuxia-llm/src/conversation/context.rs` — ContextProvider trait + Null/Static 구현체 (신규)
- `wuxia-llm/src/conversation/mod.rs` — pub mod context + re-export 추가
- `wuxia-llm/src/conversation/session.rs` — 6군데 변경 (struct/new/send/테스트)

**Iteration 3 — affinity 태그 파싱 + ChatReply 확장 + LiveContextProvider [✅ 완료]**

LLM 응답에 `[affinity: N]` 태그를 붙여 매 턴 친밀도 변화를 반영하는 구조.

설계 결정 4건:

| 결정 | 선택 | 근거 |
|------|------|------|
| 기억 검색 시점 | 매 턴 검색 | ~80ms, LLM 대비 무시 가능 |
| 관계 반영 방식 | LLM 응답에 [affinity: N] 태그 | 추가 비용 0ms, 기존 파싱 확장 |
| LiveContextProvider 범위 | 검색+갱신만 (저장은 호출자) | 단일 책임 원칙 |
| Iteration 분할 | 2단계 (Iter3=내부로직, Iter4=통합) | 실패 시 원인 추적 용이 |

작업 진행:

| # | 작업 | 상태 | 비고 |
|---|------|:----:|------|
| ① | parse_response_with_tags + extract_affinity_tag | ✅ | parser 24개 테스트 통과 |
| ② | ChatReply에 affinity_delta 필드 추가 | ✅ | session 테스트 +2개, 전체 통과 |
| ③ | LiveContextProvider 구현 | ✅ | 6 테스트, 테스트 헬퍼 리팩터링 완료 |
| ④ | send()에서 delta → relationship 반영 | ✅ | context 테스트 +4개, 전체 통과 |

작업③ 상세:
- LiveContextProvider<R: MemoryRepository> 제네릭 구조체 (정적 디스패치)
- 3단계 파이프라인: 벡터 검색(search_top_k=10) → 4축 랭킹(rank_top_k=5) → 행동 지시 라벨 포맷팅
- Builder 패턴: with_weights(), with_top_k()
- relationship owned (세션 중 불변 스냅샷)
- 테스트 6개: 빈 repo, 검색+라벨, 미매칭, 관계 요약, trait object 호환, custom top_k
- 테스트 헬퍼 추출: make_live_ctx(), make_empty_ctx() → 보일러플레이트 ~50% 감소
- wuxia-llm Cargo.toml에 wuxia-memory dev-dep 추가

부수 작업 (행동 지시형 라벨 최적화):
- prompt_config.toml v1.3.0: 자기설명적 → 행동 지시형 라벨 변경
  - 변경 전: "떠올리면 지금도 감정이 올라오는 기억" (상태 설명)
  - 변경 후: "말투가 흔들려라" (행동 지시)
  - 근거: 4B 모델은 추상 개념→행동 변환 추론 부족, 라벨은 LLM 행동 지시용
- template.rs 테스트 4곳 TOML 의존 제거 (importance_to_label() 경유로 변경)
- wuxia-data TEST_TOML에 memory_labels 섹션 추가
- wuxia-core doc test에 MemoryLabelsConfig 추가
- memory-importance-label-design.md v1.3 반영
- **코드 로직 변경 0줄** — TOML+테스트+문서만 변경

작업① 상세:
- `ParsedNpcResponse` 구조체 (text + affinity_delta: i8)
- `parse_response_with_tags()` — 기존 parse_response() 재사용 + 태그 추출
- `extract_affinity_tag()` — 모든 [affinity: N] 태그 제거, 마지막 값 사용
- 태그 규칙: -5~+5 범위 clamp, 대소문자 무관, 태그 없으면 0
- 테스트 24개 (기본/clamp/에러/대소문자/멀티라인/태그 2개)
- 기존 parse_response() 변경 0줄 (하위 호환 완벽)

작업② 상세:
- `ChatReply` 구조체에 `affinity_delta: i8` 필드 추가
- `send()`에서 `parse_response` → `parse_response_with_tags` 교체
- fallback(파싱 실패)에서도 `extract_affinity_tag`로 태그 제거 (플레이어 노출 방지)
- `extract_affinity_tag` 가시성 `fn` → `pub(crate) fn` 변경
- 미사용 `parse_response` import 제거
- 테스트 2개 추가: `session_affinity_delta_no_tag` (delta=0), `session_affinity_delta_with_tag` (delta=3)
- 기존 session 테스트 전체 통과 (하위 호환)
- 부수 작업: 4개 문서/코드에서 12b→4b ADR 참조로 변경
- 부수 작업: `adr-llm-model-4b-migration.md` v1.1.0 작성 (시장 조사, KV Cache, VRAM 예산)

**전체 체크리스트:**

- [x] ~~ChatSession에 `memory_repo: Box<dyn MemoryRepository>` 추가 (Iter 4)~~ → 🔄 ContextProvider 경유로 구현됨 (직접 소유 아님)
- [x] ChatSession에 `relationship: Relationship` 추가 → ✅ `relationship: Option<Relationship>` + `descs: Option<RelationshipDescriptions>` 직접 소유
- [x] send()에서 매 턴 기억 검색 + PromptContext.memories 동적 갱신 (Iter 2 — ContextProvider 경유)
- [x] send()에서 관계 상태를 프롬프트에 반영 (Iter 1+2)
- [x] send()에서 LLM 응답의 affinity delta를 관계에 즉시 반영 (Iter 3 작업④)
- [x] ~~end()에서 Observation 기억 저장 (Iter 4)~~ → ✅ `SessionEndResult`의 `ObservationDraft`로 구현 (저장은 호출자 책임)
- [x] ~~end()에서 관계 업데이트 (간단한 휴리스틱: 턴 수 기반)~~ → ✅ `SessionEndResult`에 최종 `Relationship` 포함
- [x] 기존 MockLlm 테스트 호환 유지 (NullContextProvider로 마이그레이션)
- **테스트**:
  - [x] context.rs 단위 테스트 8개 통과
  - [x] 기존 session.rs 21개 테스트 통과 (NullContextProvider 호환)
  - [x] LiveContextProvider 기억 검색 → 프롬프트 포함 확인
  - [x] apply_affinity_delta → 관계 수치 변화 + 레벨 전환 확인
  - [x] ~~대화 종료 → 기억 저장 확인~~ → ✅ ObservationDraft 생성 테스트 통과
  - [x] ~~관계 상태 프롬프트 반영 확인~~ → ✅ Relationship 프롬프트 섹션 + 감정 판정 파이프라인 통합

> **✅ 현행화 (2026-03-03):** Step 3.5 **전체 체크리스트 개발완료**. 추가로 감정 판정 파이프라인(`SentimentPipeline`)이 ChatSession에 통합됨 — 2단계 하이브리드: 극단 앵커 임베딩(~7ms/턴) + 주기적 LLM 판정(~300ms/12턴). `skip_affinity_directive` 필드 추가 (파이프라인 활성 시 레거시 [affinity: N] 태그 지시 억제).

---

### Step 3.6 — soyeon_chat v2 (플레이 가능 데모) [wuxia-app example]
> 터미널에서 소연과 대화하면 기억이 영구 저장되고, 관계가 성장한다

Sprint 1의 soyeon_chat.rs를 확장하여 전체 기능을 통합한다.

**Iteration 진행 현황:**

| Iteration | 내용 | 상태 | 비고 | 현행화 (2026-03-03) |
|-----------|------|:----:|------|---------------------|
| 1 | NullContextProvider 데모 (ChatSession v2 인프라 검증) | ✅ | 22턴 플레이테스트 통과 | ✅ 유지 |
| 2 | LiveContextProvider + LanceDB 기억 영속 | ✅ | 컴파일 통과, 6개 파일 수정 | ✅ 유지 |
| 3 | Relationship + JSON 영속 | ✅ 개발완료 | — | 🔄 **별도 모듈로 구현됨**: `wuxia-memory/src/relationship_store/` (InMemoryRelRepo + JsonFileRelRepo) + `wuxia-memory/src/chronicle/` (InMemoryChronicleRepo + JsonlChronicleRepo). `RelationshipRepository` + `ChronicleRepository` 포트가 `wuxia-core/src/relationship/port.rs`에 정의됨. |
| 4 | CLI 완성 (/memories, /relationship, /forget) | 📋 일부 미구현 | — | 🔄 기본 대화 루프 및 종료 시 저장은 구현됨. /memories, /relationship, /forget 등 확장 CLI 명령어는 일부 미구현. |

**Iteration 1 — NullContextProvider 데모 [✅ 완료]**

Application Service 패턴으로 soyeon_chat_v2.rs 신규 작성.
ChatSession v2 인프라를 NullContextProvider(기억 없음, 관계 없음)로 검증.

생성/수정된 파일:
- `crates/wuxia-app/Cargo.toml` — live-demo feature 추가 (wuxia-llm, wuxia-memory 의존성)
- `crates/wuxia-app/examples/soyeon_chat_v2.rs` — 신규 (~250줄)

VRAM 예산 (4b 데모):
- 모델: gemma-3-4b-it Q4_K_M ~3.0GB
- KV Cache Q8_0 (n_ctx=4096): ~52MB
- 총 GPU: ~3.0GB, 남은 VRAM: ~5.0GB

22턴 플레이테스트 결과:

```
  평가 항목               점수        비고
  ──────────────────────────────────────────
  ChatSession 인프라      ★★★★★      완벽 동작
  KV cache 효율           ★★★★★      95%+ (턴20)
  응답 속도               ★★★★★      0.5~1.5초/턴
  캐릭터 성격 일관성       ★★★☆☆      반말 유지, 간헐적 존댓말
  affinity 태그 준수       ★★☆☆☆      턴5 이후 대부분 누락
  사실 일관성(장소 등)     ★☆☆☆☆      hallucination (검새→달빛)
  표현 다양성             ★★☆☆☆      동일 문장 5회 반복
```

4b 모델 품질 이슈 5건:
1. 장소 hallucination — "검새 술집"→"달빛 가게" 자기모순
2. affinity 태그 누락 — 긴 대화에서 출력 포맷 지시 망각
3. 반복 루프 — "뭘 원하는지 분명히 말해" 5회 반복
4. 존댓말 혼용 — "가세요", "물어보세요" (Speech_Rules 위반)
5. 문맥 오해 — "이 검 팔아줘" → 자기 검으로 오해

해결 방향:
- A) Iter 2~3 해결: hallucination→Memory_Bank 주입, 문맥 오해→Conversation_Summary
- B) 모델 크기 한계: affinity 누락/반복/존댓말 → gemma3:12b 전환 시 개선 예상

결론: **ChatSession v2 인프라 완벽 검증.** 4b 이슈는 데모 수준, 실제 게임은 12b 사용 예정.

**Iteration 2 — LiveContextProvider + LanceDB 기억 영속 [✅ 완료]**

NullContextProvider → LiveContextProvider<LanceDbRepository> 전환.
대화 종료 시 Observation을 LanceDB에 저장, 재실행 시 기억 불러오기.

설계 결정 3건:

| 결정 | 선택 | 근거 |
|------|------|------|
| 백엔드 공유 | Box::leak → &'static 공유 | llama.cpp 백엔드 1회 초기화 제약, LLM+임베딩 양쪽 참조 |
| 모델 로딩 순서 | 임베딩(CPU) → LLM(GPU) | VRAM 경쟁 방지, CPU 임베딩 모델 먼저 로딩 |
| 지시문 참조 방식 | 따옴표('Persona') → XML 태그(<Persona>) | LLM이 실제 블록과 동일한 태그로 인식, 일관성 향상 |

작업 진행:

| # | 작업 | 상태 | 비고 |
|---|------|:----:|------|
| ① | prompt_config.toml v1.5.0→v1.6.0 지시문 XML 태그 전환 | ✅ | template.rs 테스트 동기화 |
| ② | soyeon_chat_v2.rs 전면 재작성 (LiveContextProvider 통합) | ✅ | ~10개 코드 영역 변경 |
| ③ | BackendAlreadyInitialized 해결 (백엔드 공유 패턴) | ✅ | 4개 파일 수정 |
| ④ | 비대칭 임베딩 채택 + LanceDB Cosine metric 수정 | ✅ | threshold 0.4656→0.4423, GAP +0.0707 |
| ⑤ | 짧은 대화 LLM 요약 + 임베딩 디버그 로그 강화 | ✅ | turn<=3 분기 제거, embed_with_prefix 로그 |

작업① 프롬프트 개선:
- prompt_config.toml v1.6.0: 지시문 참조를 따옴표에서 XML 태그로 변경
  - 변경 전: `'Persona' 블록에 정의된 성격을 참고하여`
  - 변경 후: `<Persona> 블록에 정의된 성격을 참고하여`
- template.rs 테스트 검증 문자열 동기화

작업② soyeon_chat_v2.rs 전면 재작성:
- NullContextProvider → LiveContextProvider<LanceDbRepository>
- 임베딩 모델(CPU) + LLM(GPU) 순차 로딩
- LanceDB 저장소 초기화 + 기존 기억 개수 표시
- finalize_session(): ObservationDraft → MemoryEntry 변환 + repo.save()
- type SoyeonSession 별칭 (긴 제네릭 타입 정리)
- CliArgs에 --db-path 추가
- 미사용 import 정리 (LlmPort, MemoryId, mut 경고)

작업③ 백엔드 공유 패턴:
```
  문제: llama.cpp 백엔드를 2번 초기화 시 BackendAlreadyInitialized 에러
  
  해결: Box::leak으로 &'static 참조 공유
  
  main() {
    let backend = LlamaBackend::init()?;         // 1회 초기화
    let backend: &'static _ = Box::leak(Box::new(backend));
    
    let embedder = LlamaCppEmbedding::from_config_with_backend(
        &config, backend                          // 참조만, Drop 안 함
    )?;
    
    let llm = LlamaCppAdapter::new_with_backend(
        &model_path, params, backend              // 참조만, Drop 안 함  
    )?;
  }
```

수정 파일:
- `crates/wuxia-llm/src/adapter/llama_cpp.rs` — backend를 Option<LlamaBackend>로 변경 + new_with_backend() 추가
- `crates/wuxia-llm/src/adapter/mod.rs` — LlamaBackend re-export
- `crates/wuxia-memory/src/embedding/llamacpp_adapter.rs` — from_config_with_backend() 추가
- `crates/wuxia-app/examples/soyeon_chat_v2.rs` — 전면 재작성

부수 변경:
- `assets/data/prompt/prompt_config.toml` — v1.6.0 지시문 XML 태그 전환
- `crates/wuxia-llm/src/prompt/template.rs` — 테스트 검증 문자열 동기화

작업④ 비대칭 임베딩 채택 + LanceDB Cosine metric 수정:

threshold_analyzer.rs v1.1.0 업그레이드:
- L5 레벨 추가: orig↔long 쌍 (짧은 쿼리 vs 긴 LLM 요약 — 실전 시나리오)
- `--asymmetric` CLI 플래그: Google 공식 비대칭 prefix 적용
- effective_min = min(L2_min, L5_min) 기반 threshold 재산출

비대칭 임베딩 벤치마크 결과:
```
  | 지표       | 대칭     | 비대칭   | 차이      |
  |-----------|---------|---------|----------|
  | L2_min    | 0.4656  | 0.4511  | -0.0145  |
  | L5_min    | 0.4609  | 0.4684  | +0.0075  |
  | L3_max    | 0.3996  | 0.3805  | -0.0191  |
  | GAP       | +0.0613 | +0.0707 | +0.0094↑ |
  | threshold | 0.4533  | 0.4423  | -0.0110  |
```
→ 비대칭 채택 (Google 공식 가이드, GAP 더 넓음)

LanceDB Cosine metric 수정:
- **문제**: `vector_search()` 호출 시 distance_type 미지정 → 기본값 L2 사용
  - L2 distance [0,∞) → `1.0 - distance` 변환 시 음수 가능 → threshold 필터 오작동
- **해결**: `query_builder.distance_type(DistanceType::Cosine)` 명시 추가
  - Cosine distance [0,2] → similarity = 1.0 - distance → [-1,1]

embedding.toml v1.2.0:
- KO threshold: 0.4656 → 0.4423 (비대칭 벤치마크 반영)
- 벤치마크 주석에 L5_min, eff_min 추가

실전 검증 (soyeon_chat_v2):
```
  "안녕?"    → sim=0.4016, thresh=0.4656, pass=false (인사↔혈교 무관, 정상 탈락)
  "혈교라고 알아?" → sim=0.5633, thresh=0.4656, pass=true ✅ (기억 검색 성공)
```

수정 파일:
- `crates/wuxia-memory/examples/threshold_analyzer.rs` — v1.1.0, L5+비대칭 추가
- `crates/wuxia-memory/src/lancedb.rs` — DistanceType::Cosine 명시
- `assets/ai/embedding.toml` — v1.2.0, threshold 0.4423

작업⑤ 짧은 대화 LLM 요약 + 임베딩 디버그 로그 강화:

session.rs end() 변경:
- **이전**: turn_count <= 3이면 LLM 미호출, 원문 그대로, 중요도 고정 5.0
- **현재**: 대화가 있으면 항상 LLM 요약 수행 (turn_count 무관)
- 테스트 `session_end_short_conversation` 업데이트: `make_scripted_session`으로 전환

llamacpp_adapter.rs 디버그 로그:
- `embed_with_prefix()`에 디버그 출력 추가
- 모드 자동 판별: DOC(저장) / QUERY(검색) / RAW(prefix 없음)
- 본문 360자 truncate + `...` 표시

실전 확인 (soyeon_chat_v2, 1턴 대화):
```
  저장: [DEBUG embed/DOC] "title: none | text: [NPC는 플레이어에게 친근하게 인사...]"
  검색: [DEBUG embed/QUERY] "task: search result | query: 안녕?"
  요약: "NPC는 플레이어에게 친근하게 인사를 건네며..." importance=4.0
```

수정 파일:
- `crates/wuxia-llm/src/conversation/session.rs` — turn<=3 분기 제거 + 테스트 업데이트
- `crates/wuxia-memory/src/embedding/llamacpp_adapter.rs` — 디버그 로그 추가

```
  soyeon_chat v2 실행 플로우:
  
  ┌─ 시작 ─────────────────────────────────────┐
  │                                              │
  │  1. 명령줄 인자 파싱                          │
  │     --model <path>                           │
  │     --memory inmemory|lancedb (기본: lancedb) │
  │     --db-path <path> (기본: ./data/memory)    │
  │     --debug                                  │
  │     --reset (기억 초기화)                      │
  │                                              │
  │  2. LLM 모델 로딩                             │
  │  3. 임베딩 모델 로딩                           │
  │  4. LanceDB 열기 (또는 InMemory)               │
  │  5. 기존 기억 수 표시: "소연의 기억: 23개"      │
  │  6. 기존 관계 표시: "관계: 친근 (호감 +42)"    │
  │                                              │
  └─ 대화 루프 ────────────────────────────────────┤
  │                                              │
  │  플레이어 > (입력)                             │
  │                                              │
  │  [디버그] 기억 검색: 3개 (0.87, 0.65, 0.52)   │
  │  [디버그] 관계: 호감 +42, 신뢰 35             │
  │                                              │
  │  소연 > (응답)                                │
  │         (15 tok, 12.3 tok/s, 1.2초)          │
  │                                              │
  │  명령어:                                      │
  │    /quit → 대화 종료 + 기억 저장 + 관계 갱신    │
  │    /reset → 대화 초기화 (기억은 유지)           │
  │    /forget → 모든 기억 삭제                    │
  │    /info → 상태 (턴, 기억수, 관계, ctx 사용률)  │
  │    /memories → 최근 기억 5개 표시               │
  │    /relationship → 관계 상세 표시               │
  │                                              │
  └────────────────────────────────────────────────┘
```

**검증 시나리오 (Sprint 3 성공 기준):**

```
  시나리오 1: 세션 간 기억 연속
  ─────────────────────────
  1회차: "혈교가 뭐야?" → 소연 대답 → /quit
  2회차: "저번에 사파 얘기 했잖아" → 소연이 혈교 맥락 기억 ✅
  
  시나리오 2: 관계 성장
  ─────────────────────
  1회차: 처음 만남 (호감 0, 신뢰 0) → 3턴 대화 → /quit
  2회차: "나 기억해?" → 소연이 이전 대화 참조 + 약간 친근한 태도 ✅
  5회차: 신뢰 50+ → 소연이 개인적인 이야기 시작 ✅
  
  시나리오 3: 의미 기반 검색
  ─────────────────────────
  "사파"로 검색 → "혈교" 기억 반환 (벡터 유사도) ✅
  "무공"으로 검색 → "수련" 기억 반환 ✅
  키워드 정확히 일치하지 않아도 의미적으로 관련된 기억 찾기 ✅
```

- [x] soyeon_chat.rs 확장 (v2) → ✅ `wuxia-app/examples/soyeon_chat_v2.rs` 구현
- [x] 명령줄 인자: --memory, --db-path, --reset ~~, --forget~~ → ✅ 구현 (--forget은 미구현)
- [x] LanceDB/InMemory 선택적 초기화 → ✅ 구현
- [x] 시작 시 기존 기억/관계 로딩 + 표시 → ✅ 기억 수 표시
- [x] ChatSession v2 사용 (기억 검색 + 저장 + 관계) → ✅ LiveContextProvider + LanceDB
- [ ] /memories, /relationship 명령어 → 📋 미구현
- [x] /quit 시 기억 저장 + 관계 갱신 확인 메시지 → ✅ finalize_session()
- [x] 디버그 모드: 기억 검색 결과, 벡터 유사도, 관계 변화 출력 → ✅ --debug 지원
- **검증**: 위 3개 시나리오 수동 테스트 통과

> **✅ 현행화 (2026-03-03):** Step 3.6 핵심 기능 **개발완료**. CLI 확장 명령어(/memories, /relationship, /forget) 일부 미구현.

---

### Step 3.7 — 대화 품질 측정 테스트 체계 [wuxia-llm + wuxia-app] [✅ 개발완료]
> 수동 플레이테스트를 자동화된 품질 벤치마크로 전환한다

> **✅ 현행화 (2026-03-03):** Step 3.7 **전체 개발완료**. `wuxia-llm/src/quality/` 모듈 12개 파일로 구현. 계획의 Phase 1~2 모두 구현됨:
> - **시나리오 러너**: `scenario.rs` (TOML 파싱), `runner.rs` + `runner_tests.rs` (시나리오 자동 실행), `execution.rs` (대화 루프 실행)
> - **자동 측정 지표 6개**: `metrics.rs`
> - **LLM 채점기 3종**: `judge.rs` (JudgePort + MockJudge trait), `judge_prompt.rs` (채점 프롬프트), `judge_live.rs` (ClaudeJudge + OpenAiJudge, feature-gated)
> - **리포트**: `report.rs` (FullBenchReport + PassCriteria + 터미널 테이블), `comparison.rs` (ComparisonReport A/B 테스트)
> - **트레이스 + 리플레이**: `trace.rs` (TurnTrace, SessionTrace, TimingTrace, MemoryHit), `replay.rs` (터미널 pretty-print)
> - **벤치마크 예제**: `wuxia-app/examples/conversation_bench.rs` (quality-bench feature)
> - **테스트 시나리오 3개**: `assets/test/scenarios/` (01_greeting, 02_info_request, 03_long_chat)
> - **벤치마크 결과 저장**: `data/bench_reports/` (JSON)
> - Feature flags: `claude-judge` (Claude API), `openai-judge` (OpenAI API)

**상세 계획서**: `docs/step3_7-conversation-quality-test-plan.md` v1.0.0

**배경**: Step 3.6 Iter 1 플레이테스트에서 5건의 품질 이슈 발견 (hallucination, affinity 태그 누락, 반복, 존댓말 혼용, 문맥 오해). 현재 테스트 체계(MockLlm 단위 테스트)로는 LLM 출력 품질을 측정할 수 없음.

**Phase 구성:**

| Phase | 내용 | 핵심 산출물 | 현행화 |
|:-----:|------|-----------|--------|
| 1 | 시나리오 TOML 정의 + 테스트 러너 + 자동 지표 6개 | conversation_bench.rs, quality_metrics.rs | ✅ 개발완료 |
| 2 | LLM 채점기 + 비교 리포트 생성 | quality_judge.rs, quality_report.rs | ✅ 개발완료 |
| 3 | 기준선 확정 + 회귀 테스트 | baseline.json, CI 연동 | 📋 CI 연동은 미구현 |

**자동 측정 지표 (Phase 1):** ✅ 전체 구현
- affinity_tag_rate: [affinity: N] 태그 출력률 (≥90%)
- speech_style_violation: 존댓말/경어 사용 (0회)
- repetition_score: 연속 유사 응답 비율 (≤20%)
- response_length: 응답 문장 수 (1~3문장)
- forbidden_word_leak: 금지어 노출 (0회)
- memory_utilization: 기억→응답 반영률 (≥50%)

**LLM 판정 지표 (Phase 2):** ✅ 전체 구현
- character_consistency: 캐릭터 설정 준수 (≥7/10)
- context_coherence: 문맥 일관성 (≥7/10)
- hallucination_detect: 사실 오류 감지 (0건)

**우선순위 시나리오 (P0):** ✅ 전체 구현
1. 기본 인사 (1~2턴) — 반말, 캐릭터 반응, 태그 → `assets/test/scenarios/01_greeting.toml`
2. 정보 요청/혈교 (3턴) — 배경 지식, 감정 표현 → `assets/test/scenarios/02_info_request.toml`
3. 긴 대화 (10턴) — 태그 유지, 반복 방지, 일관성 → `assets/test/scenarios/03_long_chat.toml`

---

## 기술 선택 사항 (Step 3.1B에서 확정)

### 임베딩 모델

✅ **embeddinggemma-300m-qat-Q8_0 최종 선정** (2026-02-22, 상세: step3.1-embedding-benchmark-report.md v2.1.0)
- 768차원, ~0.3GB, llama.cpp 기반, 비대칭 task prompt 적용
- 비대칭 prefix: query="task: search result | query: ", doc="title: none | text: "
- KO threshold 0.4423 (비대칭 벤치마크, L4 역전 0/24건, GAP +0.0707)
- RTX 2070S에서 28ms 추론 속도
- LanceDB DistanceType::Cosine 명시 필수

> **🔄 현행화 (2026-03-03):** 이후 **bge-m3가 active default 프로파일로 변경됨**. `embedding.toml`이 프로파일 기반 구조로 리팩터링되어 `gemma`/`bge-m3` 프로파일 간 전환 가능. `embedding-bge-m3.toml` 별도 설정 파일 추가됨.
> - **현재 기본값**: Bge-M3-567M-Q8_0 (1024차원, CPU-only, symmetric)
> - **대체 프로파일**: embeddinggemma-300m-qat-Q8_0 (768차원, CPU-only, asymmetric)

이전 선정(Step 3.1): bge-m3-Q4_K_M → Step 3.3 threshold 벤치마크에서 L4 역전 문제 발견으로 교체

모델 선정 이력:

| 시점 | 선정 모델 | 차원 | VRAM | 교체 사유 |
|------|-----------|:----:|:----:|-----------|
| Step 3.1 (초기) | bge-m3-Q4_K_M | 1024 | ~2.4GB | 한국어 교차언어 유사도 최고 |
| Step 3.3 (최종) | embeddinggemma-300m-qat-Q8_0 | 768 | ~0.3GB | L4 역전 0건, margin 3.9배, VRAM 87% 절약 |
| 이후 (현행화) | **Bge-M3-567M-Q8_0 (active default)** | 1024 | CPU-only | 프로파일 기반 전환, symmetric 모델로 변경 |

### LanceDB 버전

✅ **lancedb 0.26.2 확정** (2026-02-21)
- arrow-array/arrow-schema 57
- lance-core 2.0.0, datafusion ^51.0
- protoc 빌드 의존성 필수 (lance-encoding 2.0.0)
- IntoArrow 제네릭 API (Box 불필요, RecordBatchIterator 직접 전달)

### 비동기 처리

LanceDB는 async API를 제공한다. 현재 soyeon_chat은 동기(blocking)이므로 `tokio::runtime::Runtime::new().block_on()` 패턴으로 브릿지한다. Bevy 통합 시(Phase 5) 비동기 시스템으로 전환한다.

---

## Sprint 2 ↔ Sprint 3 연결

```
  Sprint 2 (완료)                    Sprint 3 (현재 — Step 3.3 완료)
  ═══════════════                   ═══════════════════════════════
  
  MemoryRepository trait ──────────► LanceDbRepository ✅ 완료
  InMemoryRepository ──────────────► 테스트 비교 기준선 ✅
  retrieval_score() ───────────────► 벡터 유사도 + recency + importance
  PromptContext.memories ──────────► 동적 기억 검색으로 채울 예정 (Step 3.5)
  ChatSession ─────────────────────► v2 확장 예정 (Step 3.5)
  ConversationManager ─────────────► 동일 (ctx 압축 그대로 사용)
  soyeon_chat v1 ──────────────────► v2 확장 예정 (Step 3.6)
  
  [Sprint 3에서 추가 — 완료]
  EmbeddingPort trait ─────────────► ✅ 텍스트 → 벡터 변환 포트
  embeddinggemma-300m-qat-Q8_0 ────► ✅ 768차원, 28ms, 한국어 최적
  EmbeddingConfig (TOML) ──────────► ✅ 모델/threshold 외부 설정
  2-stage search ──────────────────► ✅ threshold + keyword overlap
  MemoryEntry lang field ──────────► ✅ 다국어 threshold 분기
  
  [Sprint 3에서 추가]
  Relationship struct ─────────────► ✅ 🔄 2축 모델 (affinity -100~+100, trust 0~100) (3축에서 변경)
  RelationshipEvent ───────────────► ✅ DomainEvent 통합 완료
  RelationshipRepository trait ────► ✅ 헥사고날 출력 포트 완료 + ChronicleRepository 추가

  [Sprint 3 이후 추가된 기능]
  SentimentPipeline ─────────────────► ✅ 2단계 하이브리드 감정 판정 (극단 앵커 + LLM 주기적 판정)
  Quality Benchmarking ──────────────► ✅ 시나리오 러너, 6 지표, LLM 채점기, 비교 리포트
  Psychology Domain ─────────────────► ✅ 7층 NPC 심리 (HEXACO, 3축 가치관, OCC 감정, PAD, 프리셋)
  Relationship Persistence ──────────► ✅ JSON/JSONL 저장 (RelationshipRepository + ChronicleRepository)
  text_utils ────────────────────────► ✅ 한국어 텍스트 처리 (문장 분리, 바이그램, 키워드)
  PortError ─────────────────────────► ✅ 중앙화된 에러 타입 (String → PortError)
```

---

## crate 변경 현황

```
  wuxia-core/src/
  ├── memory/
  │   ├── embedding.rs     ← [3.1] EmbeddingPort trait ✅
  │   ├── recall.rs        ← [3.5-Iter4] recall_memories() 도메인 서비스 ✅ (신규)
  │   ├── types.rs         ← [3.3] MemoryEntry lang 필드 ✅
  │   └── (기존 파일 유지)
  ├── relationship/        ← [3.4] 새 모듈 ✅ 완료 → 🔄 현행화: 4→13파일 확장, 3축→2축 변경
  │   ├── mod.rs           ← 모듈 선언 + re-export ✅
  │   ├── types.rs         ← Relationship (2축: affinity -100~+100, trust 0~100) ✅ 🔄 수정
  │   ├── types_tests.rs   ← 관계 타입 테스트 (신규, types.rs에서 분리)
  │   ├── level.rs         ← RelationshipLevel (8단계, Wary 추가) ✅ (types.rs에서 분리)
  │   ├── trust_level.rs   ← TrustLevel (5단계) ✅ (신규)
  │   ├── relationship_type.rs ← RelationshipType (8종) ✅ (types.rs에서 분리)
  │   ├── port.rs          ← RelationshipRepository + ChronicleRepository 포트 ✅ 🔄 수정
  │   ├── event.rs         ← RelationshipEvent ✅
  │   ├── effect.rs        ← ConversationEffect + apply_conversation_effect() ✅
  │   ├── chronicle.rs     ← RelationshipChronicle, ChangeType, CauseSource ✅ (신규)
  │   ├── description.rs   ← LocalizedDesc, RelationshipDescriptions ✅ (신규)
  │   ├── sentiment.rs     ← ExtremeAnchorSet, TurnCounter, SentimentJudgment, DeltaSource ✅ (신규)
  │   └── sentiment_tests.rs ← 감정 판정 테스트 (신규)
  ├── psychology/          ← [Phase 4.5] 심리 도메인 ✅ (Sprint 3 문서 이후 추가)
  │   ├── mod.rs, personality.rs, personality_tests.rs  ← HEXACO 성격 (①층)
  │   ├── three_axis.rs, three_axis_tests.rs            ← 3축 가치관 (②층)
  │   ├── values.rs, values_tests.rs                    ← 실천적 가치 (③층)
  │   ├── emotion.rs, emotion_tests.rs                  ← OCC 감정 22종 (④층)
  │   ├── mood.rs, mood_tests.rs                        ← PAD 기분 (⑤층)
  │   ├── appraisal.rs, appraisal_tests.rs              ← 인지평가
  │   ├── filter.rs, filter_tests.rs                    ← HEXACO 감정 필터
  │   ├── decay.rs                                      ← 감정 감쇠
  │   ├── event.rs                                      ← PsychologyEvent
  │   └── preset.rs                                     ← NPC 프리셋 6종
  └── shared/              ← 🔄 현행화: embedding.rs, port_error.rs, sentiment.rs 추가

  wuxia-memory/src/
  ├── config.rs            ← [3.3] EmbeddingConfig TOML 파서 ✅ (프로파일 기반으로 확장)
  ├── error.rs             ← MemoryAdapterError ✅ (신규)
  ├── in_memory.rs         (기존 유지) + in_memory_tests.rs (테스트 분리)
  ├── lancedb/             ← [3.2~3.3] 🔄 lancedb.rs → 디렉토리로 변경
  │   ├── mod.rs           ← LanceDbRepository ✅ (lang+2-stage)
  │   └── arrow_convert.rs ← Arrow RecordBatch ↔ MemoryEntry 변환 ✅ (분리)
  ├── embedding/           ← [3.1]
  │   ├── mod.rs           ✅
  │   ├── mock.rs          ← MockEmbedding ✅
  │   ├── llamacpp_adapter.rs ← LlamaCppEmbedding ✅
  │   └── archived/        ← 벤치마크된 과거 어댑터 (candle, fastembed)
  ├── chronicle/           ← [v4-E] 관계 변화 연대기 저장 ✅ (Sprint 3 문서 이후 추가)
  │   ├── mod.rs
  │   ├── in_memory.rs     ← InMemoryChronicleRepo (테스트용)
  │   └── jsonl.rs         ← JsonlChronicleRepo (프로덕션 MVP)
  ├── relationship_store/  ← [v4-E] 관계 현재 상태 저장 ✅ (Sprint 3 문서 이후 추가)
  │   ├── mod.rs
  │   ├── in_memory.rs     ← InMemoryRelRepo (테스트용)
  │   └── json_file.rs     ← JsonFileRelRepo (프로덕션 MVP)
  └── lib.rs               (feature gate, re-exports)

  wuxia-memory/examples/
  └── threshold_analyzer.rs ← [3.3] 벤치마크 도구 ✅ (신규)

  assets/ai/
  ├── embedding.toml       ← [3.3] 임베딩 설정 파일 ✅ → 프로파일 기반 (gemma/bge-m3)
  ├── embedding-bge-m3.toml ← BGE-M3 별도 설정 ✅ (신규)
  ├── extreme-anchors.toml  ← 극단 앵커 데이터 ✅ (신규, 감정 판정용)
  └── sentiment-judge.toml  ← LLM 감정 판정 설정 ✅ (신규)

  assets/test/scenarios/    ← [3.7] 품질 벤치마크 시나리오 ✅ (신규)
  ├── 01_greeting.toml
  ├── 02_info_request.toml
  └── 03_long_chat.toml

  wuxia-data/src/           ← 🔄 현행화: extreme_anchors.rs 추가
  ├── loader.rs             ← TOML/JSON 로딩
  ├── prompt_config.rs      ← PromptConfig 로딩
  ├── relationship_desc.rs  ← RelationshipDescriptions 로딩 (신규)
  └── extreme_anchors.rs    ← ExtremeAnchorsData + SentimentJudgeData 로딩 (신규)

  wuxia-llm/src/
  ├── adapter/
  │   ├── llama_cpp.rs    ← [3.6-Iter2] backend Option 전환 + new_with_backend() ✅
  │   └── mod.rs          ← [3.6-Iter2] LlamaBackend re-export ✅
  ├── conversation/
  │   ├── context.rs      ← [3.5-Iter2+3+4] ContextProvider 슬림화 (메서드 3→1) ✅
  │   ├── mod.rs          ← [3.5-Iter2+3+4] re-export + SessionEndResult ✅
  │   ├── session.rs      ← [3.5-Iter2+4] ChatSession<L,C> + Relationship 소유 + SessionEndResult + 짧은대화 LLM 요약 ✅
  │   └── parser.rs       ← [3.5-Iter3] parse_response_with_tags + extract_affinity_tag ✅
  ├── prompt/
  │   └── template.rs     ← [3.5-Iter3] 테스트 TOML 의존 제거 리팩터링 ✅
  └── Cargo.toml          ← [3.5-Iter3] dev-dep wuxia-memory 추가

  assets/data/prompt/
  └── prompt_config.toml  ← [3.5-Iter3] v1.3.0 → [3.6-Iter2] v1.6.0 행동 지시형 라벨 + XML 태그 참조 ✅

  docs/design/
  └── memory-importance-label-design.md ← [3.5-Iter3] v1.3 반영 ✅

  wuxia-app/                           ← [3.6] 신규 추가
  ├── Cargo.toml          ← [3.6-Iter1] live-demo feature (wuxia-llm, wuxia-memory 의존성)
  ├── src/
  │   ├── main.rs          ← Placeholder (Phase 5)
  │   ├── lib.rs
  │   └── context.rs       ← LiveContextProvider (도메인 랭킹 + 어댑터 포맷팅) ✅ (신규)
  └── examples/
      ├── soyeon_chat_v2.rs ← [3.6-Iter1~2] LiveCtx+LanceDB 통합 ✅
      └── conversation_bench.rs ← [3.7] 자동 품질 벤치마크 러너 ✅ 개발완료

  wuxia-llm/src/
  ├── sentiment/           ← [v4.3~4.4] 감정 판정 파이프라인 ✅ (신규, Sprint 3 문서 이후 추가)
  │   ├── mod.rs
  │   ├── judge.rs         ← SentimentJudge trait + LlmSentimentJudge + MockSentimentJudge
  │   ├── parser.rs        ← JSON 판정 파서
  │   ├── pipeline.rs      ← SentimentPipeline (extreme-anchor + LLM 판정)
  │   └── pipeline_tests.rs
  ├── quality/             ← [3.7] 품질 측정 모듈 ✅ 개발완료
  │   ├── mod.rs
  │   ├── scenario.rs      ← TOML 시나리오 파싱
  │   ├── runner.rs        ← 시나리오 자동 실행
  │   ├── runner_tests.rs
  │   ├── execution.rs     ← 대화 루프 실행
  │   ├── metrics.rs       ← 자동 측정 지표 6개
  │   ├── judge.rs         ← JudgePort + MockJudge
  │   ├── judge_prompt.rs  ← 채점 프롬프트
  │   ├── judge_live.rs    ← ClaudeJudge + OpenAiJudge (feature-gated)
  │   ├── report.rs        ← FullBenchReport + PassCriteria
  │   ├── comparison.rs    ← ComparisonReport (A/B 테스트)
  │   ├── trace.rs         ← TurnTrace, SessionTrace
  │   └── replay.rs        ← 터미널 pretty-print
  ├── text_utils.rs        ← 한국어 텍스트 처리 ✅ (신규)
```

---

## 변경 이력

| 버전 | 변경일시 | 변경 내역 |
|------|----------|-----------|
| v0.1.0 | 2026-02-21 05:00:00 | 초기 작성. Sprint 2 Phase B(LanceDB+Embedding) 기반. 6 Step 계획. 임베딩 후보 3개 비교 설계. Relationship 최소 구현(소연 1명). soyeon_chat v2 데모 포함. |
| v0.2.0 | 2026-02-21 08:30:00 | Step 3.1 완료. EmbeddingPort trait(wuxia-core) + MockEmbedding, FastEmbedAdapter, LlamaCppEmbedding 어댑터(wuxia-memory) 구현. 4개 모델 벤치마크 실행 완료. bge-m3-Q4_K_M 선정 (1024차원, 34ms, 교차언어 0.84). 상세: step3.1-embedding-benchmark-report.md |
| v0.3.0 | 2026-02-21 22:30:00 | Step 3.2.1 Iteration 1 완료 (save/count, 11 tests). Step 3.2.2 Iteration 2 완료 (find_recent, +4 tests). LanceDB 0.23→0.26 + Arrow 56→57 업그레이드. protoc 빌드 의존성 추가. IntoArrow API 변경 대응. 전체 52 tests passing. 아키텍처 결정 4건 기록 (sync block_on, embedder 소유, MVP 스키마, Rust측 정렬). 다음: Iteration 3 search (벡터 유사도 검색). |
| v0.4.0 | 2026-02-22 20:00:00 | Step 3.3 완료. LanceDB search()+update_importance() 구현, 5개 모델 threshold 벤치마크 실행, embeddinggemma-300m-qat-Q8_0 최종 선정 (bge-m3에서 교체, L4 역전 0건+margin 3.9배+VRAM 87% 절약). EmbeddingConfig TOML 파서, MemoryEntry lang 필드, LanceDB 2-stage search (threshold+keyword overlap) 구현. 보고서 2건 (step3.1 v2.1.0, step3.3 v1.0.0). 전체 740 tests passing. Step 진행표 재정리 (기존 3.4 update_importance를 3.3에 통합, 3.5~3.7을 3.4~3.6으로 재번호). 다음: Step 3.4 Relationship 기본. |
| v0.5.0 | 2026-02-23 01:30:00 | Step 3.4 완료. 3축 관계 모델(호감도+신뢰도+적대도, 각 0~100) 구현. 설계 결정 3건 (3축 분리, RelationshipType Option, 적대 우선 복합 판정). Iteration 3회: types.rs(20 tests) → event.rs(11 tests) → port.rs(10 tests). RelationshipEvent 7종 + DomainEvent::Relationship 통합. RelationshipRepository trait (헥사고날 출력 포트). 전체 785 tests passing. 다음: Step 3.5 ChatSession v2 (기억 영속 + 관계 반영). |
| v0.6.0 | 2026-02-23 15:30:00 | Step 3.5 Iteration 1 완료. 3계층 관계 설명 아키텍처 구현: (1) wuxia-core — TrustLevel(5단계)/HostilityLevel(5단계) enum + key()/trust_level()/hostility_level() 메서드, (2) descriptions.toml — 7+5+5=17개 관계 수준별 한영 자연어 설명, (3) wuxia-core/description.rs — LocalizedDesc/RelationshipDescriptions 타입 + lookup 메서드, (4) wuxia-data/relationship_desc.rs — TOML 로딩, (5) wuxia-llm/template.rs — PromptContext.relationship_summary 필드 + [관계 상태] 프롬프트 섹션 + format_relationship_for_prompt() 함수. session.rs PromptContext 호환 수정. 신규 테스트 ~25개 추가. 전체 820 tests passing. 다음: Step 3.5 Iteration 2 — ChatSession v2 생성자 (MemoryRepository+Relationship 주입, send() 동적 검색). |
| v0.7.0 | 2026-02-24 01:00:00 | Step 3.5 Iteration 2 완료. ContextProvider trait 분리: ChatSession<L> → ChatSession<L, C: ContextProvider> 리팩터링. base_memories: Vec<String> 삭제 → context_provider: C 필드로 교체. 구현체 3종: NullContextProvider(테스트), StaticContextProvider(Sprint 2 호환), LiveContextProvider(향후). send()에서 매 턴 search_memories(user_input) + relationship_summary() 동적 호출. 기존 21개 session 테스트 NullContextProvider로 마이그레이션 완료. 신규 context.rs 8개 테스트. 전체 29개 wuxia-llm 테스트 통과. 다음: Iteration 3 — LiveContextProvider 구현 + soyeon_chat v2 통합. |
| v0.8.0 | 2026-02-24 02:30:00 | Step 3.5 Iteration 3 작업① 완료. parse_response_with_tags() + ParsedNpcResponse 구조체 + extract_affinity_tag() 추가. [affinity: N] 태그 파싱 (-5~+5 clamp, 대소문자 무관, 모든 태그 제거+마지막 값 사용). 기존 parse_response() 변경 0줄. parser 테스트 24개 통과 (기존 14 + 신규 10). 설계 결정 4건 확정: 매 턴 검색, LLM 태그 방식, LiveContextProvider 검색+갱신만, 2단계 분할(Iter3+Iter4). 다음: 작업② ChatReply 확장. |
| v0.9.0 | 2026-02-24 04:00:00 | Step 3.5 Iteration 3 작업② 완료. ChatReply에 affinity_delta: i8 필드 추가. send()에서 parse_response→parse_response_with_tags 교체. fallback에서도 extract_affinity_tag로 태그 제거(플레이어 노출 방지). extract_affinity_tag pub(crate) 승격. session 테스트 +2개(no_tag/with_tag). 부수: adr-llm-model-4b-migration.md v1.1.0 작성(시장조사+KV Cache+VRAM 예산), 4개 문서/코드 12b→4b ADR 참조 변경. 전체 wuxia-llm 테스트 통과. 다음: 작업③ LiveContextProvider 구현. |
| v1.0.0 | 2026-02-21 08:34:00 | Step 3.5 Iteration 3 작업③ 완료. LiveContextProvider<R: MemoryRepository> 제네릭 구현 (벡터 검색→4축 랭킹→행동 지시 라벨 포맷팅 3단계 파이프라인). Builder 패턴(with_weights/with_top_k). 테스트 6개 + 헬퍼 추출(make_live_ctx/make_empty_ctx, 보일러플레이트 ~50% 감소). 부수: prompt_config.toml v1.3.0 행동 지시형 라벨 최적화(4B 모델 특화), template.rs 테스트 TOML 의존 제거 4곳, wuxia-data TEST_TOML memory_labels 섹션 추가, wuxia-core doc test MemoryLabelsConfig 추가. wuxia-llm Cargo.toml dev-dep wuxia-memory 추가. 코드 로직 변경 0줄(TOML+테스트+문서만). 전체 workspace 테스트 통과. 다음: 작업④ send()에서 delta→relationship 반영. |
| v1.1.0 | 2026-02-21 09:10:00 | 방안 A 구현 완료 (프롬프트 최적화 대응계획 Phase 1). prompt_config.toml v1.4.0 — ko/en directive_affinity 템플릿 추가 ("응답 마지막에 반드시 [affinity: N] 태그를 붙여라"). wuxia-core PromptTemplates에 directive_affinity: String 필드 추가. wuxia-llm build_system_prompt() 11번 [지시] 섹션에 affinity 출력 규칙 삽입 (role→react→affinity→lang 순서). wuxia-data TEST_TOML 갱신. doc test + 테스트 픽스처 3곳 갱신. 전체 workspace 테스트 통과. 토큰 예산 +35 (총 ~1425 토큰, ctx 8192의 0.4%). 다음: Phase 2 실전 검증 (soyeon_chat 10턴 태그 출력률 측정) 또는 작업④ delta→relationship 반영. |
| v1.2.0 | 2026-02-21 23:40:00 | XML 프롬프트 구조 전환 완료 (Iteration 3 부수작업). **6개 파일 변경**: (1) prompt_config.toml v1.5.0 — [headers] 섹션 완전 삭제, directive_role/react/affinity → directive_1~5 + directive_output_example 교체, {affinity_tag_prefix} 플레이스홀더 도입, ko/en 양쪽 적용. (2) prompt_config.rs (wuxia-core) — PromptHeaders 구조체+get_headers() 완전 삭제, PromptTemplates에 directive_1~5 + directive_output_example 필드 추가, 테스트 make_templates() 헬퍼로 리팩터링. (3) template.rs (wuxia-llm) — XML 태그 상수 14개(TAG_PERSONA~TAG_DIRECTIVES), AFFINITY_TAG_PREFIX pub const 상수, wrap_xml() 헬퍼, build_system_prompt() 전면 재작성(2계층 그룹핑: Persona→Current_Context→Directives), TOML {tag_*} 플레이스홀더 치환 로직. (4) parser.rs — extract_affinity_tag()이 AFFINITY_TAG_PREFIX 단일 원본 참조로 전환(const TAG_PREFIX 삭제). (5) prompt/mod.rs — AFFINITY_TAG_PREFIX re-export 추가. (6) wuxia-data TEST_TOML — directive_1~5 + directive_output_example 반영. **아키텍처 결정**: 단일 원본 원칙(XML 태그명=template.rs 상수, 번역 텍스트=TOML, 태그 접두사=AFFINITY_TAG_PREFIX). AFFINITY_TAG_PREFIX="[affinity:" 유지 결정([호감:]은 향후 검토). 테스트 [호감: N]→[affinity: N] 일관성 수정 2건. PromptHeaders 잔여 참조 0건 확인. 전체 workspace 테스트 통과. |
| v1.3.0 | 2026-02-22 00:15:00 | Step 3.5 Iteration 3 작업④ 완료. ContextProvider trait에 `apply_affinity_delta(&mut self, i8)` 메서드 추가. 구현체 3종: NullContextProvider(no-op), StaticContextProvider(no-op), LiveContextProvider(`relationship.update_affinity(delta as f32)`). session.rs send()에서 파싱(7번)→관계 갱신(8번)→턴 기록(9번) 순서로 삽입. **핵심 효과**: LLM 응답의 [affinity: N] 태그가 매 턴 Relationship 수치에 즉시 반영 → 다음 턴 프롬프트의 <Relationship> 섹션 톤이 실시간 변화 (Stranger→Acquaintance→Friendly 등). MVP에서는 affinity만 변경, 향후 성격/감정 시스템 도입 시 3축 분배 규칙 추가 예정. 테스트 +4개(null no-op, static no-op, live 수치 변화, live 레벨 전환). **Iteration 3 전체 완료** (작업①②③④). 전체 workspace 테스트 통과. 다음: Iteration 4 — end()에서 Observation 저장 + 관계 최종 업데이트, 또는 Step 3.6 soyeon_chat v2 통합. |
| v1.4.0 | 2026-02-22 13:00:00 | Step 3.5 Iteration 4 완료. **Core Domain Service 추출 + ChatSession 리팩터링**. (1) wuxia-core recall.rs — `recall_memories()` 도메인 서비스 (repo.search+rank_memories 조합, Mock 테스트 6개). (2) wuxia-core effect.rs — `ConversationEffect` Value Object + `apply_conversation_effect()` 도메인 서비스 (affinity 캡슐화, 테스트 13개). (3) wuxia-llm context.rs — ContextProvider trait 슬림화 (메서드 3→1, search_memories만 잔존). LiveContextProvider 필드 10→8. (4) wuxia-llm session.rs — ChatSession이 `relationship: Option<Relationship>` + `descs: Option<RelationshipDescriptions>` 직접 소유. send() step8에서 Core `apply_conversation_effect()` 호출. `SessionEndResult` 신규 struct (ObservationDraft+Relationship). end() 반환 타입 변경. 신규 테스트 3개 (단일delta, 누적delta, 관계없는 세션). wuxia-core 607 unit+70 doc tests, wuxia-llm 전체 테스트 통과. 다음: Step 3.6 soyeon_chat v2 통합. |
| v1.5.0 | 2026-02-22 18:30:00 | Step 3.6 Iteration 1 구현. soyeon_chat_v2.rs 신규 작성 (Application Service 패턴: main→parse_cli→create_session→run_chat_loop). wuxia-app Cargo.toml에 live-demo feature 추가. NullContextProvider로 ChatSession v2 인프라 검증. cargo check 통과. 다음: Iter 1 플레이테스트. |
| v1.6.0 | 2026-02-22 22:30:00 | Step 3.6 Iteration 1 플레이테스트 완료. 22턴 대화 성공. 성능: 응답 0.5~1.5초, KV cache 95%+, ctx 44%(22턴 후). ChatSession 인프라 완벽 검증 (★★★★★). 4b 모델 품질 이슈 5건 기록 (hallucination, affinity 누락, 반복, 존댓말 혼용, 문맥 오해). 해결 방향: Iter 2~3 Memory_Bank 주입 + 12b 전환. 다음: Iteration 2 (LiveContextProvider + LanceDB 기억 영속). |
| v1.7.0 | 2026-02-22 23:45:00 | Step 3.6 Iteration 2 완료. **NullContextProvider→LiveContextProvider<LanceDbRepository> 전환**. (1) prompt_config.toml v1.6.0 — 지시문 참조 따옴표→XML 태그 전환. (2) soyeon_chat_v2.rs 전면 재작성 — 임베딩(CPU)+LLM(GPU) 순차 로딩, LanceDB 저장소 초기화, finalize_session() ObservationDraft→MemoryEntry 변환+저장, SoyeonSession 타입 별칭, --db-path CLI 인자. (3) 백엔드 공유 패턴 — Box::leak→&'static으로 LLM+임베딩 양쪽 참조, LlamaCppAdapter new_with_backend(), LlamaCppEmbedding from_config_with_backend(), LlamaBackend re-export. 설계 결정 3건 (백엔드 공유, 모델 로딩 순서, 지시문 참조 방식). 수정 파일 6개. 컴파일 통과. 다음: Iteration 2 플레이테스트 또는 Iteration 3 (Relationship JSON 영속). |
| v1.8.0 | 2026-02-23 05:00:00 | Step 3.6 Iteration 2 작업④ 완료. **비대칭 임베딩 채택 + LanceDB Cosine metric 수정**. (1) threshold_analyzer.rs v1.1.0 — L5 레벨(orig↔long 쌍) 추가, --asymmetric CLI 플래그, effective_min 기반 threshold 재산출. (2) 비대칭 벤치마크: GAP +0.0707 (대칭 +0.0613 대비 15% 향상), threshold 0.4533→0.4423. Google 공식 가이드에 따라 비대칭 채택. (3) LanceDB lancedb.rs — DistanceType::Cosine 명시 (기존 L2 기본값 → score 변환 오류 수정). (4) embedding.toml v1.2.0 — threshold 0.4656→0.4423 반영. 실전 검증: "혈교라고 알아?" sim=0.5633 pass=true ✅, "안녕?" sim=0.4016 pass=false ✅. 기억 검색 파이프라인 완전 복구. |
| v1.9.0 | 2026-02-23 14:30:00 | Step 3.6 Iteration 2 작업⑤ 완료. **짧은 대화도 LLM 요약 + 임베딩 디버그 로그 강화**. (1) session.rs end() — turn_count<=3 분기 제거, 1턴이라도 LLM 요약+중요도 산정 (이전: 원문 그대로 importance=5.0 고정). 테스트 session_end_short_conversation make_scripted_session으로 전환. (2) llamacpp_adapter.rs embed_with_prefix() — 디버그 로그 추가 (DOC/QUERY/RAW 모드 자동 판별, 360자 truncate). 실전 확인: 1턴 인사 대화 → LLM 요약 "NPC는 플레이어에게 친근하게 인사...", importance=4.0. 다음: Iteration 3 (Relationship JSON 영속) 또는 중복 기억 필터링 검토. |
| v2.0.0 | 2026-02-23 15:00:00 | **Step 3.7 계획 수립**. 대화 품질 측정 테스트 체계 구축 계획서 작성 (docs/step3_7-conversation-quality-test-plan.md v1.0.0). 3 Phase 6 Step 구성: Phase 1(시나리오 TOML+러너+자동 지표 6개), Phase 2(LLM 채점기+비교 리포트), Phase 3(기준선+회귀 테스트). 자동 측정 지표 6개 + LLM 판정 3개 + 성능 4개 정의. P0 시나리오 3개(인사/정보요청/긴 대화) 설계. sprint3-progress.md에 Step 3.7 섹션 + crate 변경 현황 추가. 다음: Step 3.7.1 시나리오 TOML 정의. |
| v2.0.1 | 2026-03-03 | **현행화.** 현재 코드베이스(~1,463 tests) 기준으로 전체 스텝 상태 업데이트. 주요 변경: (1) **Step 3.4 관계 모델 대폭 수정** — 3축→2축(hostility 삭제, affinity -100~+100), 8단계 레벨(Wary 추가), 파일 4→13개, 신규 타입(chronicle, sentiment, description). (2) **Step 3.1 EmbeddingPort 이동** — memory/→shared/, 에러 String→PortError, embed_document()/model_name() 추가. (3) **Step 3.3 임베딩 모델 변경** — gemma-qat→bge-m3(active default), 프로파일 기반 config. (4) **Step 3.6 ✅ 개발완료** — Iter3 관계 영속→별도 모듈(relationship_store/chronicle), Iter4 CLI 일부 미구현. (5) **Step 3.7 ✅ 전체 개발완료** — quality/ 12파일, 시나리오 러너, 6 지표, LLM 채점기, 비교 리포트, 트레이스, 리플레이. (6) **Sprint 3 이후 추가 기능**: 감정 판정 파이프라인(sentiment/ 5파일), 심리 도메인(psychology/ 18파일 207 테스트), 관계 영속(chronicle/ + relationship_store/), text_utils.rs, PortError 도입. crate 변경 현황 섹션 전면 업데이트. |
