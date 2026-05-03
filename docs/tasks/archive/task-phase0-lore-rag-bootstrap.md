# Phase 0: Lore RAG MCP 부트스트랩

> **For Claude Code.** 이 문서는 자급 자족이며 외부 링크 없이도 작업 시작 가능.
> 결정 사항을 임의 변경하지 말 것. 변경이 필요하면 보고서에서 디렉터 승인 요청.

## 1. 목표

장르 원전을 임베딩+RAG로 인덱싱하고 MCP 도구로 노출하는 `src/lore/` 컴포넌트 구축.

**부트스트랩 패턴**: 같은 RAG가 두 곳에서 호출됨 — (a) 도구 설계 단계에서 worldbuilding 결정마다 검증, (b) 완성된 도구의 AI 협업 기능. Phase 1+ 모든 단계가 이 RAG에 의존하므로 Phase 0가 가장 우선.

## 2. 연관 컨텍스트

- `CLAUDE.md` 프로젝트 루트 — `embed` feature, `OrtEmbedder`, `SqliteMemoryStore` 패턴, MCP 서버 구조
- 메모리 (Cowork 세션이 갖고 있음): worldbuilding 도구 = 장르 중립, 9 추상 카테고리(Place·Person·Group·Item·Skill·Knowledge·Lore·Event·Era), Lore RAG = 메타 도구·실제 도구 양쪽에서 사용
- 직전 결정: 라이선스·사이즈 제약 (§3) 엄수

## 3. 제약 (READ FIRST — 절대 임의 확장 금지)

### 3.1 라이선스 — 부트스트랩은 PD 원전 3권만

| 책 ID | 파일 | 비고 |
|---|---|---|
| `shuihuzhuan-zh-zhang` | `wuxia-core/docs/Chinese-Literature/水滸傳 (張啟疆) (z-library.sk, 1lib.sk, z-lib.sk).epub` | 시내암 14세기 원전. 張啟疆 註釋부 분리 시도 — 분리 어려우면 본문만 임베딩하고 註釋은 별도 edition으로 표시(또는 제외). |
| `jianghu-qixia-zh-1922` | `wuxia-core/docs/Chinese-Literature/江湖奇俠傳 (平江不肖生) (z-library.sk, 1lib.sk, z-lib.sk).epub` | 1922 발표, 작가 1957 사망. PD 확정. |
| `shushan-jianxia-zh-1932` | `wuxia-core/docs/Chinese-Literature/蜀山劍俠傳 (繁體中文) (還珠樓主) (z-library.sk, 1lib.sk, z-lib.sk).epub` | 1932 발표, 작가 1961 사망. 繁體中文. PD 확정. |

**번역본·학술서·anthology·역사 번역본 일체 제외.** 39권 전체 인덱싱은 라이선스 검증 후 별도 TASK.

### 3.2 사이즈 — Git 친화

추가될 `.gitignore` 항목:
```
# Lore corpus — 원본은 외부 자료, SQLite는 빌드 산출물
wuxia-core/docs/Chinese-Literature/*.epub
wuxia-core/docs/Chinese-Literature/*.pdf
wuxia-core/docs/Chinese-History/*.epub
wuxia-core/docs/Chinese-History/*.pdf
data/corpus/lore.sqlite
data/corpus/lore.sqlite-*
```

`data/corpus/manifest.toml`만 git에 들어감. README/CLAUDE.md에 "다른 머신 작업 시 자료 다운 후 `cargo run --features embed --bin lore-ingest -- --all`로 재생성" 한 줄 안내 추가.

## 4. Done Criteria

- [ ] `data/corpus/manifest.toml`에 3권 등록 (corpus + editions 구조 §6.1)
- [ ] `src/lore/{mod,corpus,ingest,query,store}.rs` 모듈 컴파일 통과
- [ ] `Cargo.toml`에 EPUB 파서 deps 추가, `embed` feature 안에 게이팅
- [ ] 1권 ingest 시연 통과 (Step 3) — Cowork 리뷰 통과
- [ ] 3권 일괄 ingest 통과 (Step 4)
- [ ] `bin/mind-studio` MCP에 도구 3개 등록: `search_lore`, `list_corpora`, `get_chunk`
- [ ] 한국어 5쿼리·중국어 5쿼리 정성 확인 (Step 5 보고서)
- [ ] `.gitignore` 갱신 (§3.2)
- [ ] `cargo build --features embed` + `cargo test --features embed` 통과

## 5. 단계별 작업

### Step 1 — Manifest

파일: `data/corpus/manifest.toml`. 스키마와 3권 entry는 §6.1 참조.

산출물 검증: `cargo test --features embed corpus::tests::manifest_parses` 같은 단위 테스트 1개로 파싱 통과 확인.

### Step 2 — 모듈 스켈레톤

```
src/lore/
├── mod.rs           # pub re-exports, feature gate
├── corpus.rs        # CorpusMeta, Edition, ManifestParser
├── ingest.rs        # EpubReader → Chunker → Embedder → Store 파이프라인
├── query.rs         # SearchQuery, SearchHit, ChunkContext
└── store.rs         # SqliteLoreStore (vec0 + FTS5)

src/bin/
└── lore_ingest.rs   # CLI — `--book <id>`, `--all`, `--reembed`
```

`Cargo.toml`:
- EPUB: `epub` 크레이트 (https://crates.io/crates/epub) 우선. 막히면 `epub-rs` 등 대안 — 자동 결정 가능.
- PDF는 이번 Phase 0에서 사용 안 함 (3권 다 EPUB). Step 5 이후 39권 확장 TASK에서 추가.
- 모두 `embed` feature 안에서 optional dep으로.

기존 코드 패턴 참고:
- `OrtEmbedder` (또는 동등 — bge-m3 ONNX 임베딩) 활용. 새 임베더 만들지 말 것.
- `SqliteMemoryStore`의 vec0 + FTS5 + 1024 dim 차원 처리 그대로 미러링. 새 패턴 발명하지 말 것.
- `MemoryStore` 트레잇 등록 위치와 같은 곳에 `LoreStore` 트레잇 추가.

산출물 검증: `cargo build --features embed` 통과. 빈 스켈레톤이라도 OK.

### Step 3 — 첫 ingest 시연 ★체크포인트 1★

대상: **江湖奇俠傳 (3.4MB)** — 가장 검증하기 좋은 분량.

파이프라인:
1. EPUB 열기 → 챕터별 텍스트 추출 (chapter title·index 메타 보존)
2. Chunker — 중국어 ~500자, overlap 200자. 챕터 경계는 청크 경계 유지(같은 청크 안에 두 챕터 안 섞이게).
3. `OrtEmbedder.embed_batch(texts)` → 1024 dim float32 벡터
4. `SqliteLoreStore.upsert(chunks_with_embeddings)` — vec0 + FTS5에 동시 인덱스
5. CLI 명령:
   ```
   cargo run --features embed --bin lore-ingest -- --book jianghu-qixia-zh-1922
   ```
6. 검색 시연 (코드 또는 별도 CLI):
   ```rust
   let results = lore.search("강호의 의리", 5, None).await?;
   ```
   한국어 쿼리·중국어 쿼리 각 1개씩.

**체크포인트 1 보고서** (디렉터에게 제출):
- `git diff --stat` 결과
- 청크 샘플 3개 (raw text 발췌 + meta JSON)
- 검색 결과: 한국어 쿼리 1개 + 중국어 쿼리 1개 (top 5씩 — text·score·corpus_id·edition_id·chunk_id)
- 실측: 임베딩 시간(분), 청크 수, SQLite 크기(MB)
- 의문점 / 디렉터 결정 필요 사항
- Step 4·5 진행 가능 여부 의견

→ Cowork 세션에서 리뷰 후 통과 시 다음 단계.

### Step 4 — 3권 일괄 ingest

```
cargo run --features embed --bin lore-ingest -- --all
```

manifest의 모든 corpus.editions 순회. 진행률 로그 출력. 동일 edition 재실행 시 skip 또는 `--reembed` 플래그 시 재임베딩.

산출물: `data/corpus/lore.sqlite`. 예상 사이즈 100MB 미만.

### Step 5 — MCP 도구 3개 노출 ★체크포인트 2★

`bin/mind-studio` MCP 서버에 등록 (기존 도구 등록 위치 참고).

```
search_lore(
    query: String,
    top_k: u32 = 5,
    corpus_filter: Option<Vec<String>> = None,
    edition_filter: Option<Vec<String>> = None,
) -> Vec<SearchHit>

SearchHit { corpus_id, edition_id, chunk_id, text, score, language, chapter_title? }

list_corpora(
    genre_tag: Option<String> = None,
) -> Vec<CorpusSummary>

get_chunk(
    chunk_id: String,
    before: u32 = 1,
    after: u32 = 1,
) -> ChunkContext { focus, before: Vec<Chunk>, after: Vec<Chunk> }
```

**체크포인트 2 보고서**:
- MCP 도구 호출 데모 (curl 또는 jsonrpc payload 예시)
- 한국어 5쿼리 결과 요약 (각 한 줄: 쿼리 — top hit corpus·chapter·발췌)
- 중국어 5쿼리 결과 요약
- 정성 평가: 검색 품질, cross-lingual 작동 여부
- `data/corpus/lore.sqlite` 최종 사이즈
- 다음 Phase 1 진행 가능 여부 의견

→ Cowork 리뷰 → 통과 시 Phase 0 종료.

## 6. 결정 사항 (변경 시 디렉터 승인)

### 6.1 Manifest 스키마 + 3권 entry

```toml
# data/corpus/manifest.toml

[[corpus]]
id = "shuihuzhuan"
title = "水滸傳 (수호지)"
author_name = "施耐庵 (시내암)"
genre_tags = ["wuxia", "chinese-literature", "novel-classical"]
license = "public-domain"

  [[corpus.editions]]
  id = "shuihuzhuan-zh-zhang"
  language = "zh"
  edition = "張啟疆 註釋"
  source = "wuxia-core/docs/Chinese-Literature/水滸傳 (張啟疆) (z-library.sk, 1lib.sk, z-lib.sk).epub"
  format = "epub"
  license_note = "원전 본문 PD. 張啟疆 註釋 부분 분리 권장. 분리 어려우면 본문만 임베딩."

[[corpus]]
id = "jianghu-qixia-zhuan"
title = "江湖奇俠傳"
author_name = "平江不肖生 (向恺然)"
genre_tags = ["wuxia", "chinese-literature", "novel-modern", "republican-era"]
license = "public-domain"
license_note = "1922 발표, 작가 1957 사망. 글로벌 PD 확정."

  [[corpus.editions]]
  id = "jianghu-qixia-zh-1922"
  language = "zh"
  edition = "1922 원전"
  source = "wuxia-core/docs/Chinese-Literature/江湖奇俠傳 (平江不肖生) (z-library.sk, 1lib.sk, z-lib.sk).epub"
  format = "epub"

[[corpus]]
id = "shushan-jianxia-zhuan"
title = "蜀山劍俠傳"
author_name = "還珠樓主 (李壽民)"
genre_tags = ["wuxia", "xianxia", "chinese-literature", "novel-modern"]
license = "public-domain"
license_note = "1932 발표, 작가 1961 사망. 중국·한국 PD 확정."

  [[corpus.editions]]
  id = "shushan-jianxia-zh-1932"
  language = "zh-Hant"
  edition = "1932 원전 (繁體中文)"
  source = "wuxia-core/docs/Chinese-Literature/蜀山劍俠傳 (繁體中文) (還珠樓主) (z-library.sk, 1lib.sk, z-lib.sk).epub"
  format = "epub"
```

### 6.2 임베딩

- 모델: bge-m3 (이미 `embed` feature에서 사용 중)
- 차원: 1024 (FLOAT32)
- 코사인 거리 (vec0 default)

### 6.3 청킹

- 중국어: ~500자, overlap 200자
- 한국어: ~1000자, overlap 200자 (Phase 0에선 사용 안 함)
- 영어: ~1500자, overlap 300자 (Phase 0에선 사용 안 함)
- 챕터 경계 보존 — 같은 청크 안에 두 챕터 안 섞임
- 청크 메타: `chunk_id`, `corpus_id`, `edition_id`, `language`, `chapter_index?`, `chapter_title?`, `char_offset_in_edition`, `char_offset_in_chapter`

### 6.4 SQLite

- 파일: `data/corpus/lore.sqlite` (gitignore)
- 환경변수: `NPC_MIND_LORE_DB` (없으면 위 default 경로). `NPC_MIND_MEMORY_DB`와 별개.
- 스키마: 기존 `SqliteMemoryStore` 패턴 미러링 — `lore_chunks` 일반 테이블 + `lore_chunks_fts` FTS5(trigram) + `lore_chunks_vec` vec0(FLOAT[1024])
- 마이그레이션: schema_meta 테이블로 버전 관리 (기존 패턴 그대로)

### 6.5 라이브러리

- EPUB 파서: `epub` 크레이트 (대안 가능 — Claude Code가 자동 결정)
- 모두 `embed` feature 안에 optional dep

## 7. Out of Scope (Phase 0)

- OCR (스캔 PDF)
- GPU 가속 (1회성 ingest, CPU OK)
- UI 패널 (Mind Studio worldbuilding 패널 = Phase 3+)
- 39권 전체 인덱싱 (라이선스 검증 후 별도 TASK)
- PDF 파서 (Phase 0 자료 3권 다 EPUB이라 불필요)
- 한국어 어휘 갭 시드 사전 (필요 시 Phase 1+)
- Cross-lingual 정확도 튜닝 (일단 baseline 측정만)

## 8. 코드 위치 가이드

작업 시작 시 아래 위치를 먼저 읽어 패턴 파악:

| 위치 | 무엇을 볼지 |
|---|---|
| `Cargo.toml` `[features]` | `embed` feature 활성화 패턴 |
| `src/adapter/`(embedder 위치) | `OrtEmbedder` 사용법 |
| `src/adapter/`(memory store 위치) | `SqliteMemoryStore` — vec0+FTS5+1024 패턴 |
| `src/ports.rs` | `MemoryStore` 트레잇 — `LoreStore` 만들 때 참고 |
| `src/bin/mind-studio/handlers/` | MCP 도구 등록 패턴 |
| `src/bin/mind-studio/state.rs` (또는 동등) | AppState 확장 — `lore_store: Option<Arc<dyn LoreStore>>` 추가 위치 |

## 9. 리뷰 채널

체크포인트 1·2 보고서를 디렉터(사용자)가 Cowork 세션에 복붙 → Cowork 세션이 검토 → 코멘트·다음 지시 → 사용자가 Claude Code에 전달.

보고서 형식 권장 — 마크다운으로 다음 섹션:
- **Done**: 어떤 단계가 끝났는지 (체크리스트)
- **Diff**: `git diff --stat` 결과 + 핵심 파일 발췌
- **데모 명령**: 디렉터가 직접 돌려볼 수 있는 한 줄
- **결정**: TASK에 안 적힌 미세 결정 (라이브러리 버전 등)
- **막힌 것**: 디렉터 승인이 필요한 사항
- **다음**: 다음 단계로 넘어갈지에 대한 의견

## 10. 시작 체크리스트

Claude Code가 이 TASK 받으면 첫 5분에:

1. `CLAUDE.md` 통독
2. `Cargo.toml` 읽고 `embed` feature 구조 확인
3. `src/adapter/` 안에서 `OrtEmbedder` + `SqliteMemoryStore` 위치 grep
4. `src/bin/mind-studio/handlers/`에서 기존 MCP 도구 등록 패턴 한두 개 읽기
5. 그 위에서 Step 1 시작 (manifest.toml 작성)

이 5분이 끝난 뒤 시작하면 헛걸음 적음.
