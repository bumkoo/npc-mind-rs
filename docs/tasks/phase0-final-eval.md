# Phase 0 Lore RAG — 최종 평가 (Final Evaluation)

> 대상 Task: [`task-phase0-lore-rag-bootstrap.md`](task-phase0-lore-rag-bootstrap.md)
> 인프라 보고서: [`phase0-lore-rag-bootstrap-report1.md`](phase0-lore-rag-bootstrap-report1.md)
> 브랜치: `claude/bootstrap-lore-rag-HKyAW`
> 평가 완료일: 2026-04-29
> 결과: **체크포인트 1·2 통과 — Phase 0 종료**

---

## 1. 결정 확정 (디렉터 승인)

### B3 — 水滸傳 (張啟疆 註釋) 본문/주석 분리 정책
**채택: 옵션 (b) — 분리 어려움 인정.**
- 추가 작업 불필요. 註釋이 본문에 일부 섞여 들어가는 것을 허용한다.
- `manifest.toml`의 `license_note`(`"원전 본문 PD. 張啟疆 註釋 부분 분리 권장. 분리 어려우면 본문만 임베딩."`)로 라이선스적 표시는 충분.
- 정성 평가에서 註釋 혼입이 검색 품질에 유의미한 노이즈를 주지 않음을 확인.

### D2 — HTML→text 마크업 잔여 점검
**통과.** 자체 상태 머신으로 충분. `scraper`/`html5ever` 의존성 도입 안 함.

---

## 2. 정성·정량 평가 요약 (Cowork 세션 측정 결과)

> 상세 쿼리·청크 텍스트는 디렉터 측 Cowork 세션 로그에 보존.
> 본 문서는 상위 결과만 요약.

### 체크포인트 1 — Step 3 단권 ingest (江湖奇俠傳 + 후속 全 3권)

- 江湖奇俠傳(3.4MB) 단권 ingest 정상 통과
- 청킹·임베딩·SQLite 라운드트립 검증 완료
- 한국어 1쿼리 + 중국어 1쿼리 검색 결과 합리적

### 체크포인트 2 — Step 5 MCP 도구 정성 평가 (10 쿼리)

| 항목 | 결과 |
|---|---|
| 한국어(KO) 쿼리 5개 | 통과 — cross-lingual 매칭 정상, 의도된 corpus에서 관련 청크 회수 |
| 중국어(ZH) 쿼리 5개 | 통과 — 같은 언어 매칭 자연스러움 |
| `shuihuzhuan` 단독 corpus_filter 검증 | 통과 — 다른 corpus 청크 누설 없음 |
| `get_chunk` 문맥 확장 | 통과 — focus 청크 ± 인접 청크가 같은 edition·논리적 순서 유지 |
| `list_corpora` indexed_chunks | 통과 — 3 corpus 모두 카운트 노출 |

### 정량 (관측치)

- `SearchHit.score` 관측 범위: **0.45 ~ 0.62** (cross-lingual KO↔ZH)
- 해석: cosine similarity 직접값 (sqlite-vec `distance_metric=cosine` → `1.0 - distance` = cos_sim)
- bge-m3 cross-lingual 정상 범위(같은 의미 다른 언어 0.4~0.7)와 일치 → **정규화 정상**
- 자세한 cosine 정규화 검증 결과는 §3 참조.

### 정성 — Cross-lingual 작동 여부
한국어 쿼리가 중국어 본문 청크를 의미 기반으로 회수함을 다수 케이스에서 확인.
Phase 0의 핵심 가설("bge-m3 단일 모델로 한·중 동일 RAG에서 검색 가능")이 검증됨.

---

## 3. Cleanup TASK (Phase 1 진입 전 동일 PR로 처리)

### 3.1 Ingest 노이즈 청크 필터

**구현 위치:** `src/lore/ingest.rs`

**필터 규칙 (확정):**
1. **챕터 제목 화이트리스트 → 챕터 통째로 skip**
   - `Cover` · `封面` · `目錄` · `目次` · `目录` · `Table of Contents` · `Contents`
   - `is_noise_chapter_title(title)` 함수로 분리, trim 후 정확 일치
2. **청크 텍스트 길이 < `MIN_CHUNK_CHARS`(50자, Unicode scalar) → 청크 skip**
   - `chunk_chapter` 내부에서 슬라이딩 윈도우의 각 청크 길이 검증
   - 마지막 짧은 꼬리(이전 청크의 overlap에 이미 포함된 영역)와 빈 페이지 잔여를 함께 처리

**불변식:**
- noise 챕터를 skip해도 `char_offset_in_edition` 누적은 보존 → 본문 챕터의 절대 offset이 변하지 않아 기존 인덱스의 `get_chunk` 인접 검색에 영향 없음.

**단위 테스트:** `lore::ingest::tests::noise_filter_skips_toc_and_short_chunks`
- ToC 7개 변종 + trim + 한자 본문 비-노이즈 검증
- 4-챕터 시나리오 (목차 / 본문100자 / 본문30자 / 본문200자) → 챕터 2,4만 살아남고 offset이 누적 길이(40·170)로 정확히 매겨짐 확인
- 살아남은 모든 청크가 `MIN_CHUNK_CHARS` 이상

**기존 테스트 영향:**
- `chunks_respect_chapter_boundaries`: 250자 챕터 → 100/100/90/10 청크 중 마지막 10자 청크가 필터됨. `assert!(recs.len() >= 3)`은 그대로 통과.
- `edition_offsets_increment_across_chapters`: 50/30자 챕터를 100/80자로 수정 (50자 미만이 필터되는 새 정책에 맞춤). 의미 동일.

**README/CLAUDE.md 안내 한 줄(추가 완료):**
> 청킹·필터 정책이 바뀌면 (예: Phase 0 cleanup으로 ToC 챕터 + 50자 미만 청크 noise 필터 추가) 기존 인덱스를 재생성하기 위해 `--reembed`를 1회 실행:
> `cargo run --features embed --bin lore-ingest -- --all --reembed`

### 3.2 SearchHit.score 정규화 검증

**검증 결과:** **정규화 정상 — 코드 변경 없음, 주석만 추가.**

**검증 근거:**
- sqlite-vec 0.1.x의 `distance_metric=cosine`은 cosine distance ∈ `[0, 2]`을 반환.
  내부에서 두 벡터의 norm으로 분모를 나누므로 입력 정규화 여부와 무관하게 안전.
- `score = 1.0 - distance` ⇒ `cos_sim ∈ [-1, 1]`.
- bge-m3 dense 출력은 BAAI 모델 자체의 마지막 layer normalization을 거친 unit vector에 가까움 (bge-m3-onnx-rust는 별도 정규화 없이 `dense_vecs` 텐서를 그대로 반환).
- 관측 score 0.45~0.62 = cosine similarity 0.45~0.62. cross-lingual relevant match의 정상 범위.

**조치:** `src/lore/store.rs::SqliteLoreStore::search`의 `score: 1.0 - distance` 라인 위에 위 검증 결과를 6줄 주석으로 박제. 향후 매트릭 변경(예: `distance_metric=l2`) 시 회귀를 잡을 단서.

---

## 4. 완료 후 상태

### 코드 검증
```
cargo test --lib lore::ingest
  → 4 passed (chunk_config / boundaries / edition_offsets / noise_filter)

cargo test --lib lore::
  → 7 passed (corpus×3 + ingest×4)  [embed 미활성에서도 동작]

cargo test --features embed lore::
  → 11 passed (corpus×3 + ingest×4 + html_to_text×3 + sqlite×1)

cargo build --features mind-studio,embed,chat
  → ok (1 pre-existing warning, 본 변경과 무관)
```

### Done Criteria 최종 (Task §4)

- [x] `data/corpus/manifest.toml`에 3권 등록
- [x] `src/lore/{mod,corpus,ingest,query,store}.rs` 모듈 컴파일
- [x] `Cargo.toml`에 EPUB 파서 deps + embed 게이팅
- [x] **1권 ingest 시연 (Step 3) — Cowork 리뷰 통과**
- [x] **3권 일괄 ingest 통과 (Step 4)**
- [x] `bin/mind-studio` MCP 도구 3종 등록
- [x] **한국어 5쿼리·중국어 5쿼리 정성 확인 (Step 5)**
- [x] `.gitignore` 갱신
- [x] `cargo build --features embed` + `cargo test --features embed` 통과

### 변경 요약 (Phase 0 전체)

| 영역 | 신규 / 수정 |
|---|---|
| Manifest | `data/corpus/manifest.toml` (3 corpus, 3 editions) |
| Lore 모듈 | `src/lore/{mod,corpus,query,store,ingest}.rs` (5 파일) |
| CLI | `src/bin/lore_ingest.rs` |
| Mind Studio | `state.rs` `with_lore` 빌더, `main.rs` 자동 부착, `mcp_server.rs` 3 도구 |
| Cargo | `epub = "2.1"` optional dep + `[[bin]] lore-ingest` |
| .gitignore | corpus EPUB/PDF + `lore.sqlite{,-shm,-wal}` |
| 문서 | `README.md` Lore RAG 섹션, `CLAUDE.md` 환경변수+섹션, `docs/tasks/phase0-*.md` 보고서·평가 |

### 미해결 / 의도된 OoS

| 항목 | 상태 |
|---|---|
| FTS5 키워드 검색 메서드 (`search_by_keyword`) | 인덱스만 존재, 검색 메서드 미노출. **Phase 1+에서 RRF 하이브리드와 함께 추가 예정.** |
| PDF 파서 (`Edition.format` 디스패치) | task §7 OoS — 39권 확장 시 별도 TASK |
| OCR / GPU / Mind Studio worldbuilding UI | task §7 OoS |
| `--reembed` 시 chunking 파라미터 변경에 의한 고아 청크 청소 | 현 단계 미구현, Phase 1+ 정책 변경 시 재검토 |

---

## 5. Phase 1 진입 의견

**Phase 0는 종료 가능.** cleanup이 본 PR에 포함되었고 정성/정량 평가가 통과되었습니다.

### 다음 단계 권장

1. **본 PR(Phase 0 cleanup)을 main에 머지.**
2. **Phase 1 TASK 작성** (`docs/tasks/task-phase1-...md`)
   - Phase 0의 §10 협업 워크플로우와 동일하게 자급자족 형식
   - 우선 검토 후보 (디렉터 의사결정 필요):
     - **(a) Worldbuilding 9 카테고리 데이터 모델** — Place·Person·Group·Item·Skill·Knowledge·Lore·Event·Era 도메인 타입 + scenario 통합
     - **(b) RAG 검색 품질 향상** — FTS5 키워드 노출 + RRF 하이브리드 + 한국어 어휘 갭 시드 사전
     - **(c) Mind Studio worldbuilding 패널** — 검색·등록 UI (Phase 3로 미루는 것이 task 명세와 일치)

### Cowork 세션 후속 항목 (체크포인트 2 정성 평가 회신 시 발견되면 별도 보고)

- 註釋 혼입 케이스에서 검색 품질 영향이 향후 발견될 경우, 별도 청소 TASK 트리거 가능 (옵션 (a) 본문/주석 분리로 회귀)
- Cross-lingual 정확도 튜닝(task §7 OoS)이 Phase 1+ 어느 단계에서 실제로 필요한지의 신호 수집

---

## 부록 — 본 PR(Phase 0 cleanup) 변경 요약

```
src/lore/ingest.rs       — MIN_CHUNK_CHARS, NOISE_CHAPTER_TITLES, is_noise_chapter_title,
                            chunk_chapter/chunk_edition 필터 적용,
                            기존 테스트 1개 데이터 수정 + 신규 테스트 1개
src/lore/store.rs        — search() score 산출 위에 정규화 검증 주석 6줄
README.md                — Lore RAG 섹션 (자료 다운 + --reembed 안내)
CLAUDE.md                — 환경변수 2개 + Lore RAG 섹션 (자료 다운 + --reembed 안내)
docs/tasks/phase0-final-eval.md   — 본 문서
```

**보고 끝.** Phase 1 TASK 작성으로 이동.
