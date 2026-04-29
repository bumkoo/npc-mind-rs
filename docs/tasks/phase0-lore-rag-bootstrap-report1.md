# Phase 0 Lore RAG 부트스트랩 — 리뷰 보고서 #1 (인프라 완료)

> 대상 Task: [`task-phase0-lore-rag-bootstrap.md`](task-phase0-lore-rag-bootstrap.md)
> 브랜치: `claude/bootstrap-lore-rag-HKyAW`
> 커밋: `77bdc0d` (push 완료)
> 작성일: 2026-04-29

본 보고서는 Phase 0의 **인프라(코드/스키마/CLI/MCP) 완료**를 보고합니다. Step 3·4·5의
실연(체크포인트 1·2)은 corpus EPUB과 bge-m3 ONNX 모델이 있는 디렉터 머신에서만
실행 가능하므로, 별도 리뷰 보고서 #2(체크포인트 1)·#3(체크포인트 2)로 분할 제출
예정입니다.

---

## Done

### Task §4 Done Criteria 체크리스트

- [x] `data/corpus/manifest.toml`에 3권 등록 (corpus + editions, §6.1 스키마 준수)
- [x] `src/lore/{mod,corpus,ingest,query,store}.rs` 모듈 컴파일 통과
- [x] `Cargo.toml`에 EPUB 파서 deps 추가, `embed` feature 안에 게이팅
- [ ] **1권 ingest 시연 (Step 3) — Cowork 리뷰 통과** ← 보고서 #2 예정
- [ ] **3권 일괄 ingest 통과 (Step 4)** ← 보고서 #2/#3 예정
- [x] `bin/mind-studio` MCP에 도구 3개 등록: `search_lore`, `list_corpora`, `get_chunk`
- [ ] **한국어 5쿼리·중국어 5쿼리 정성 확인 (Step 5 보고서)** ← 보고서 #3 예정
- [x] `.gitignore` 갱신 (§3.2)
- [x] `cargo build --features embed` + `cargo test --features embed` 통과 (lore 10/10)

### 단계별 완료 상태

| Step | 항목 | 상태 |
|---|---|---|
| 1 | Manifest 작성 + 단위 테스트 (`manifest_parses` 외 2개) | 완료 |
| 2 | 모듈 스켈레톤 + `epub` 크레이트 deps + `.gitignore` | 완료 |
| 3 | Ingest 파이프라인 코드 (EpubReader/Chunker/Embedder/Store) + `lore-ingest` CLI | **코드 완료, 실 ingest 미수행** |
| 4 | 3권 일괄 ingest | **미수행 (자료/모델 부재)** |
| 5 | MCP 도구 3종 등록 + AppState 배선 | **코드 완료, 정성 평가 미수행** |

---

## Diff

### `git diff --stat HEAD~1..HEAD`

```
 .gitignore                        |   8 +
 Cargo.toml                        |  11 +-
 data/corpus/manifest.toml         |  54 ++++
 src/bin/lore_ingest.rs            | 197 +++++++++++++
 src/bin/mind-studio/main.rs       |  45 +++
 src/bin/mind-studio/mcp_server.rs | 126 ++++++++
 src/bin/mind-studio/state.rs      |  28 ++
 src/lib.rs                        |   1 +
 src/lore/corpus.rs                | 157 ++++++++++
 src/lore/ingest.rs                | 520 ++++++++++++++++++++++++++++++++++
 src/lore/mod.rs                   |  25 ++
 src/lore/query.rs                 |  78 +++++
 src/lore/store.rs                 | 583 ++++++++++++++++++++++++++++++++++++++
 13 files changed, 1832 insertions(+), 1 deletion(-)
```

### 핵심 파일 요약

**신규 생성 (8 파일)**
- `data/corpus/manifest.toml` — PD 원전 3권 entry (§6.1 스키마)
- `src/lore/mod.rs` — 공개 API 재노출 (feature gate 정리)
- `src/lore/corpus.rs` — `Manifest`/`CorpusMeta`/`Edition` + TOML 파서 + 단위 테스트 3
- `src/lore/query.rs` — `SearchQuery`/`SearchHit`/`ChunkContext`/`CorpusSummary` DTO
- `src/lore/store.rs` — `LoreStore` trait + `SqliteLoreStore` (FTS5 trigram + sqlite-vec
  vec0 FLOAT[1024], `SqliteMemoryStore` 패턴 미러링) + 라운드트립 테스트
- `src/lore/ingest.rs` — `ChunkConfig`(언어별)·`chunk_chapter`/`chunk_edition`·
  `EpubFileReader` (HTML→text 상태 머신) + 단위 테스트 3 + 청킹 테스트 3
- `src/bin/lore_ingest.rs` — CLI (`--all`/`--book`/`--reembed`/`--manifest`/`--db`)

**수정 (5 파일)**
- `Cargo.toml` — `epub = "2.1"` optional dep + `embed` feature에 추가 +
  `[[bin]] lore-ingest` (required-features = ["embed"])
- `.gitignore` — corpus EPUB/PDF 4종 패턴 + `data/corpus/lore.sqlite{,-shm,-wal}`
- `src/lib.rs` — `pub mod lore;` 한 줄
- `src/bin/mind-studio/state.rs` — `AppState.lore_store/lore_manifest` 필드 +
  `with_lore()` 빌더 (embed-gated)
- `src/bin/mind-studio/main.rs` — `NPC_MIND_LORE_DB` 존재 시 자동 부착 (4-way match,
  실패 시 graceful warn)
- `src/bin/mind-studio/mcp_server.rs` — `search_lore`/`list_corpora`/`get_chunk`
  list_tools 등록 + call_tool 디스패치 (분석기 재사용으로 임베딩 산출)

### 검증

```
cargo test --features embed lore::
  → 10 passed; 0 failed (corpus×3 + chunking×3 + html_to_text×3 + sqlite×1)

cargo test --lib lore::
  → 6 passed (embed 미활성에서도 trait·DTO·청킹·매니페스트 컴파일 + 동작 확인)

cargo build --features mind-studio,embed,chat
  → ok (warning 1 — 기존 events.rs Chat* dead variants, 본 변경과 무관)
```

기존 261 lib 테스트 + 통합 테스트 슈트는 회귀 없음. `embed_test.rs` 6개 실패는
`../models/bge-m3/` ONNX 파일 부재로 인한 사전 존재 실패로 본 변경과 무관.

---

## 데모 명령

### A. 인프라 컴파일·테스트 (어떤 머신에서도 가능)

```bash
cargo test --features embed lore::
```

### B. Ingest 실연 (디렉터 머신 — corpus EPUB + bge-m3 모델 필요)

```bash
# 사전 조건:
#   1. wuxia-core/docs/Chinese-Literature/ 아래 EPUB 3권 배치
#   2. ../models/bge-m3/{model_quantized.onnx, tokenizer.json} 배치
#   3. (선택) export NPC_MIND_LORE_DB=data/corpus/lore.sqlite

# 체크포인트 1: 단권 시연 (江湖奇俠傳, 3.4MB — 가장 가벼움)
cargo run --features embed --bin lore-ingest -- --book jianghu-qixia-zh-1922

# Step 4: 3권 일괄
cargo run --features embed --bin lore-ingest -- --all
```

### C. MCP 도구 검증 (체크포인트 2 — 실 인덱스 필요)

```bash
# Mind Studio 띄우기
cargo run --features mind-studio,chat,embed --bin npc-mind-studio

# MCP SSE 연결 후 search_lore 호출 (예: Claude Code MCP 또는 curl)
# → list_corpora 로 indexed_chunks 확인
# → search_lore { query: "강호의 의리", top_k: 5 } 정성 평가
# → get_chunk { chunk_id: <hit.chunk_id>, before: 1, after: 1 } 문맥 확장
```

### D. 환경변수 cheatsheet

```
NPC_MIND_LORE_DB         data/corpus/lore.sqlite        (없으면 in-memory 미사용 — 파일 부재 시 graceful skip)
NPC_MIND_LORE_MANIFEST   data/corpus/manifest.toml
NPC_MIND_MODEL_DIR       ../models/bge-m3
NPC_MIND_LORE_BATCH      32                              (임베딩 배치 크기)
```

---

## 결정 (Task에 안 적힌 미세 결정 — 디렉터 검토 환영)

| # | 결정 | 근거 / 대안 |
|---|---|---|
| D1 | **EPUB 라이브러리 = `epub = "2.1.5"`** | task §6.5가 1순위로 지명. 빌드 통과·공개 spine/ToC API 사용. 대안 `epub-rs`는 미시도. |
| D2 | **HTML→text는 의존성 없는 상태 머신** | scraper/html5ever 의존성 추가 회피. block-level 태그 줄바꿈 + script/style skip + 자주 보이는 엔티티 디코드 + utf-8 multibyte safe push. 단위 테스트 3개로 회귀 가드. 한자 EPUB은 일반적으로 well-formed XHTML이라 충분 판단. **만약 정성 평가에서 마크업 잔여가 발견되면 scraper 도입 재검토.** |
| D3 | **`chunk_id` 포맷 = `{edition_id}::ch{0001}::p{0000}`** | 결정적 ID — 같은 청킹 파라미터에서 동일 chunk_id 재생성. `--reembed` 시 INSERT OR REPLACE로 자연스럽게 덮어쓰기. |
| D4 | **vec0 partition key = `edition_id`** | task §6.4 스키마 미러링 차원에서 1축 partition만 허용 → corpus 단위가 아닌 edition 단위로 선택. 같은 작품의 다른 판본을 분리 검색·필터하기 용이. corpus_filter는 후처리에서 적용. |
| D5 | **search oversample = `top_k * 4`** | corpus_filter 후처리에서 일부가 떨어질 가능성 대비. top_k=5 → vec0 LIMIT 20. 평균 매칭률 1/4 가정 — 정성 평가에서 부족하면 상수 재조정. |
| D6 | **`SqliteLoreStore`는 `lore_chunks` / `_fts` / `_vec` 3 테이블** | task §6.4 명시. FTS5 인덱스는 upsert에선 채우지만 **외부 노출 메서드(`search_by_keyword`)는 미구현** — Phase 0 의도된 MVP. trait에 자리만 있고 호출 경로 없음. Phase 1+에서 RRF 하이브리드와 함께 추가 예정. |
| D7 | **임베딩 산출은 `analyzer` 재사용** | MCP `search_lore`가 PadAnalyzer의 `analyze_with_embedding`을 호출해 임베딩만 추출(PAD 결과 폐기). 별도 OrtEmbedder 인스턴스를 두지 않음 → bge-m3 모델 1회 로드. PAD 계산 비용은 무시 가능 수준. |
| D8 | **`AppState.lore_store: Option<Arc<dyn LoreStore>>`** | `NPC_MIND_LORE_DB` 파일 부재 시 부착 생략 → Mind Studio는 lore 없이도 정상 시동, MCP 호출 시점에 "lore index 미구성" 에러. graceful degradation. |
| D9 | **`lore-ingest` CLI는 손수 인자 파싱** | clap 의존 추가 회피. 5개 플래그(`--all`/`--book`/`--reembed`/`--manifest`/`--db`)만 지원. 명세 확장 시 clap 도입 재검토. |
| D10 | **트랜잭션 단위 = batch 1개** | upsert_batch가 단일 transaction. 32 청크씩 commit → 긴 EPUB(蜀山劍俠傳 등)에서 partial progress 복구 가능. ingest 도중 크래시 시 batch 단위 재진입. |

---

## 막힌 것 (디렉터 승인·자료 필요)

### B1. corpus EPUB 3종이 작업 환경에 없음
- 경로: `wuxia-core/docs/Chinese-Literature/*.epub`
- `.gitignore`되어 있어 의도된 부재. **체크포인트 1·2는 자료가 있는 디렉터 머신에서만 실행 가능.**
- 대응: 본 보고서 #1은 인프라 검증까지만, 실 ingest는 별도 보고서 분할.

### B2. bge-m3 ONNX 모델 부재
- 경로: `../models/bge-m3/{model_quantized.onnx, tokenizer.json}`
- 기존 `embed_test.rs` 6개도 같은 이유로 사전 실패 (본 변경과 무관, 회귀 아님).
- 대응: 동일 — 디렉터 머신에서 실행.

### B3. 水滸傳 (張啟疆 註釋) 본문/주석 분리 정책 — **승인 요청**
- task §3.1: "本문만 임베딩하고 註釋은 별도 edition으로 표시(또는 제외)"
- 현재 `EpubFileReader`는 spine 순서대로 모든 챕터를 그대로 추출 → 註釋이 본문에 섞여 들어갈 가능성.
- 옵션:
  - **(a) 본문만**: 註釋 EPUB 내 별도 spine 또는 CSS 클래스로 식별 가능하면 필터.
  - **(b) 분리 어려움 인정**: `manifest.toml`의 `license_note`에 명기하고 그대로 임베딩.
  - **(c) 보류**: 체크포인트 1을 江湖奇俠傳(3.4MB, 단일 본문)으로 진행, 水滸傳은 보고서 #2 검토 후 결정.
- **권장: (c)** — 체크포인트 1 정성 평가에서 註釋 혼입이 실제로 검색 품질을 떨어뜨리는지 확인 후 결정.

### B4. PAD 분석기 모델과 lore 임베딩 모델 동기화
- 현재 MCP `search_lore`가 `AppState.analyzer`를 재사용 → PadAnalyzer가 사용하는 모델 = lore 임베딩 모델.
- 향후 PAD용 모델과 lore용 모델을 분리하고 싶으면 `AppState.lore_embedder`를 별도 필드로 분리 필요.
- 현 단계엔 양쪽 모두 bge-m3 1024d라 문제 없음. **별도 결정 사항 아님 — 노트로만.**

---

## 다음 (의견)

### 권장 순서

1. **디렉터 머신에서 체크포인트 1 실행** (`--book jianghu-qixia-zh-1922`)
   - 가장 가벼운 EPUB(3.4MB) → 청킹·임베딩·SQLite 라운드트립 검증
   - 보고서 #2: `git diff --stat`(없음 — 코드 변경 0) + 청크 샘플 3개 + 한국어/중국어 쿼리 1개씩 + 실측(임베딩 시간/청크 수/SQLite 크기) + Cowork 리뷰

2. **체크포인트 1 통과 후 Step 4 일괄 실행**
   - 水滸傳 註釋 정책(B3) 결정 후 `--all` 또는 단권씩 순차

3. **체크포인트 2 — MCP 정성 평가**
   - Mind Studio 띄우고 `search_lore` 한국어 5 + 중국어 5 쿼리
   - 보고서 #3: 정성 평가 + cross-lingual 작동 여부 + 최종 SQLite 크기 + Phase 1 진행 의견

### Phase 1+ 진입 시 자연스럽게 따라올 항목 (지금 만들지 말 것)

- FTS5 키워드 검색 메서드 (`search_by_keyword`) trait 추가 + RRF 하이브리드
- PDF 파서 (`Edition.format` 디스패치) — 39권 확장 시
- 마이그레이션 v2 자리 (스키마 변경 발생 시)
- Mind Studio worldbuilding UI 패널 (Phase 3+)

### 체크포인트 1 진행 가능 여부 의견

> **인프라는 진행 가능 상태.** 디렉터가 corpus EPUB + bge-m3 모델을 배치한 뒤 위 데모
> 명령 B를 그대로 실행하면 됩니다. CLI는 batch 진행률을 100청크 단위로 출력하고
> 결과는 표 형식으로 stdout에 남깁니다. 실패 시 `LoreError::Storage`로 명확한
> 에러 — 추가 디버깅 코드는 현 단계에서 불필요.

---

## 부록: 인프라 안에서 "지을 자리는 마련했지만 비어 있는" 부분

코드 리뷰 시 누락처럼 보일 수 있어 사전 명시:

| 부분 | 상태 | 비고 |
|---|---|---|
| `lore_chunks_fts` 인덱스 | 데이터는 쌓임, 검색 메서드 미노출 | task 명세 외 — Phase 1+ |
| `Edition.format` 필드 분기 | 구조엔 있음, ingest는 항상 EPUB | task §7 OoS — PDF는 별도 TASK |
| `lore_schema_meta` v2 마이그레이션 | 슬롯만 있고 함수 없음 | 스키마 변경 시 추가 |
| `--reembed` 시 chunking 파라미터 변경에 의한 고아 청크 | 이론적 가능, 실제 미발생 | Phase 0에선 청킹 파라미터 고정 |
| WAL 모드 / 멀티 프로세스 동시 쓰기 | 미설정 | ingest는 1회성, Mind Studio는 read-only로 사용 |
| `SearchHit.text`의 토큰 길이 정보 | 통째 반환 | 호출 측에서 cut, 또는 Phase 1+에서 truncated_text 추가 |

---

**보고 끝.** 체크포인트 1 보고서는 디렉터가 데모 명령 B를 실행해 stdout 로그 +
`data/corpus/lore.sqlite` 통계를 회신하면 본 폴더에 `phase0-checkpoint1-report.md`로
이어 작성합니다.
