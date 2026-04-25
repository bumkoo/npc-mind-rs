# Memory Step B — Injection & Framing

**상태**: ✅ 2026-04 · **커밋**: `43cef24` (구현) + `17f0cd7` (리뷰 대응) + `2ba2766` (문서)
**브랜치**: `claude/reference-memory-docs-UA06H`
**설계 정본**: [`../memory/03-implementation-design.md`](../memory/03-implementation-design.md) §10 · §13 Step B

## 핵심

`DialogueAgent`가 LLM 시스템 프롬프트에 "떠오르는 기억" 블록을 prepend.
Source별 라벨(`[겪음]`/`[목격]`/`[전해 들음]`/`[강호에 떠도는 소문]`)로 신뢰도 위계 시각화.

- `MemoryFramer` trait (`ports.rs`) + `LocaleMemoryFramer` 기본 구현 (`presentation/memory_formatter.rs`)
- `[memory.framing]` locale 섹션 (ko/en) — header/footer + 4 source 라벨
- `DialogueAgent::with_memory(store, framer)` opt-in builder
- `inject_memory_push(npc, query, pad)` 내부 메서드 — 검색 → Ranker → record_recall → Framer 블록
- `start_session` (1회) + `BeatTransitioned` 발생 시 hook
- `MemoryStore::search_by_meaning` / `_keyword` / `get_recent` 3종 `#[deprecated(since="0.4.0")]` 마킹

## 사용자 승인 결정

- **범위**: Core only — Mind Studio 미리보기 UI는 Step E
- **재주입 시점**: `start_session` 1회 + `BeatTransitioned`만 (매 turn 옵션은 Step F)

## 변경 파일

```
src/ports.rs                              # MemoryFramer trait + 3 메서드 deprecation
src/presentation/memory_formatter.rs      # 신규 LocaleMemoryFramer (≈230줄)
src/presentation/mod.rs                   # pub mod memory_formatter
locales/ko.toml · locales/en.toml         # [memory.framing] + [memory.framing.block]
src/application/dialogue_agent.rs         # memory_* 필드 + with_memory + inject_memory_push + 훅
tests/memory_injection_test.rs            # 신규 (3 테스트)
tests/memory_test.rs · sqlite_memory_test.rs  # #![allow(deprecated)] 추가
```

## 동작 요약

```
start_session(npc, partner, situation?)
  → query = situation.description ?? partner_id
  → inject_memory_push(npc, query, pad=None)
       → analyzer.embed(query)?  → Some(emb)
       → store.search(NpcAllowed + exclude_superseded + min_retention=0.10
                      + limit = TOP_K * 3)
       → Ranker(Source priority filter + 5-factor score, limit=TOP_K=5)
       → record_recall(id, now_ms) for each
       → framer.frame_block(entries, "ko")
  → format!("{memory_block}{appraise_prompt}")
  → chat.start_session(...)

turn(utterance, pad_hint?, ...)
  → ApplyStimulus dispatch_v2
  → if BeatTransitioned in events:
       inject_memory_push(npc, utterance, listener_pad)
       chat.update_system_prompt(format!("{memory_block}{new_prompt}"))
```

## 검증

- 7 단위 테스트 (`LocaleMemoryFramer`): ko/en source variants · block empty/assemble · fallback 경로
- 3 통합 테스트 (`memory_injection_test`): start_session prepend / 미부착 no-op / Beat 전환 재주입
- 전체 500+ 테스트 green (default · chat · mind-studio 조합)

## 리뷰 후속 수정 (`17f0cd7`)

- **[HIGH]** `Candidate.embedding`을 쿼리 임베딩 복사 → `None`으로 변경.
  기존엔 모든 후보가 동일 클러스터로 묶여 source-priority 필터가 하위 source를 부당 드롭.
  `None`이면 topic-less 후보가 단독 클러스터로 처리되어 2단계 점수에서 정상 경쟁.
- **[MEDIUM]** Beat trigger 테스트의 `beat_changed=false → return pass` silent skip 제거.
  이제 trigger 미충족 시 `assert!` 실패로 회귀 가시화.
- **[LOW]** `LocaleMemoryFramer::new()`에서 locale TOML 파싱 실패 시 `tracing::warn!` 로그.

## 잔여 / 알려진 제약

- **SQLite `search`의 vec0 통합 부재** — `MemoryQuery.embedding` 제공해도 `relevance_score=1.0` 하드코딩 (Step A 시점부터 누적). semantic 검증은 `InMemoryStore`에서만. SQLite 백엔드 사용 시 Ranker는 retention × source × emotion × recency만으로 구분. → 후속 작업 (`SqliteMemoryStore::search` 리팩터)
- **`record_recall` 세션 내 중복 증가** — 같은 세션의 여러 BeatTransitioned에서 동일 Top-K가 선택되면 `recall_count`가 중복 증가. → Step C/D에서 명시적 Command 경로 도입 시 dedup
- **Mind Studio 프롬프트 미리보기 UI** → Step E
- **Pull 경로 (`recall_memory` tool) · 매 turn 재주입** → Step F (Phase 5 StoryAgent와 묶음)
- **구 `MemoryStore` 메서드 완전 제거** → Step D 이후
- **엔트리 자체 임베딩 전달** — `MemoryResult` 스키마 확장 필요 (Step C/D 범위)

## 회귀 감시 포인트

- `tests/memory_injection_test.rs::injection_reapplied_on_beat_transition` — 강한 부정 PAD + Joy absent trigger 조합. 엔진 튜닝 변동 시 가장 먼저 깨짐
- `LocaleMemoryFramer` 단위 테스트 7개 — locale TOML 파싱 / fallback / 4 source 라벨
- 기존 `dialogue_agent_test.rs` 무영향 (with_memory 미호출 → no-op 경로)
