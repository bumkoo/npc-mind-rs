# Phase 2.3 — Appraise Tuning + ±100 Native 전환 (FROZEN SPEC)

> **상태**: 🟢 **FROZEN → 종결** (2026-05-17 확인② 실측 검증 완료 — G1·G2·G3 PASS, 회귀 0, G4 정정 완료) — 2026-05-17 확인① 승인
> **상위**: [`PHASE2.3-KICKOFF.md`](PHASE2.3-KICKOFF.md) v1.2 §1·§5 인계 — 본 spec이 정본 (KICKOFF 재작성 안 함, P-D-1)
> **선행**: Phase 2 종결 (git `9339909`, PR #93). 진입 HEAD `f7ea858` (=종결 + doc 2커밋, 코드 변경 0)
> **설계 근거**: 모든 결정(P-D-1~9 / C-1~6)은 Stage 0 사실조사(grep + git + cargo 실측)로 검증됨 — 추론 0
> **협업**: Claude Code 자율 실행. 중간 확인 = 확인② 1회(2.3-C 후). git 커밋 = Claude 직접 (2026-05-17 Bekay 지시로 변경 — 기존 "Bekay 직접/메시지 텍스트만" 폐기).

---

## §0. 본 문서의 성격

- KICKOFF v1.2 §0 초안(헤더+골격)을 **정식 FROZEN spec으로 승격** — DRAFT 폐기, 본 문서가 정본.
- Stage 0 사실조사가 KICKOFF v1.2 대비 **정정 3건 + dead field 1건 + 신규 소작업 1건** 도출 → 본 spec이 정본 (§0.5).
- 확인⓪(Q1~Q4) + 확인①(A1~A4) 승인 완료. P-D-1~9 **변경 금지** — 의문 시 중단·확인, 임의 재해석 금지.

---

## §0.5. Stage 0 사실조사 정정 (P-D-1 — KICKOFF v1.2 대비 정정 박제)

> KICKOFF v1.2 §1-C/§1-E는 **재작성하지 않는다**(정본 박제 원칙). 아래가 정정 정본. KICKOFF엔 본 절 포인터 1줄만 부기.

| # | KICKOFF v1.2 서술 | Stage 0 실측 (정정) | 근거 |
|---|---|---|---|
| ① §E | "Stage 4 미처리 확정, state.rs:666~671 커스텀 Deserialize **잔존**" | Stage 4(`81777e0`)가 커스텀 Deserialize impl + 5테스트 **이미 제거 완료**. Phase 2 종결 전부터 plain `#[derive]`. Stage 6 S6-D5가 고아 doc 주석(state.rs:664-668)을 impl로 오독 → 옳았던 Stage 4 회고를 잘못 "정정"한 것 | `git show 9339909` / `git log -S` / src grep `impl..Deserialize..RelationshipData`=0 / 5테스트명=0 |
| ② §C | "4축 **합산**(\|Δt\|+\|Δa\|+\|Δr\|+\|Δw\|) → 동시 변동 over-trigger. 옵션 a/b/c 결정 필요" | `dominant_delta`(relationship_memory_handler.rs:63)는 4축 \|Δ\| 중 **최댓값 1개** fold 반환. 합산 아님 = §1-C 옵션(a) 이미 구현. "합산 over-trigger"는 코드에 부재 | 함수 본체 fold 실측 / 호출부 L155-167 |
| ③ §A | "situation.rs closeness = 함수 인자명 별 의미 가능 / worldbuilding 장소 인접성 별 의미 가능 — 분해 확인 필요" | situation.rs 3건 전부 `///` 주석(RelationshipModifiers 필드 doc), 비주석 사용 0. worldbuilding closeness/power src 0건. **별 의미 없음** — "별 의미 분리" 카테고리 공집합 | situation.rs 비주석 grep=0 / worldbuilding grep=0 / 필드 실제 affinity_norm 기반 |

**공통 패턴**: 3축→4축 마이그레이션이 코드는 옮겼으나 주석/prose 미동반 = 체계적 부채. ①이 실제로 Stage 6 §E 오독을 유발 → P-D-9 sweep로 대응.

---

## §1. 확정 결정 (P-D-1 ~ P-D-9) — 변경 금지

| ID | 결정 | 근거 (Stage 0 실측) |
|---|---|---|
| **P-D-1** | (Q1) 불일치 3건을 본 §0.5에 정정 박제. KICKOFF v1.2 §1-C/§1-E **재작성 금지**, spec 포인터 1줄만 부기. | §0.5 표 |
| **P-D-2** | (Q2) §A = **값 동치 리팩터**(동작 불변). `mod.rs:172-173` ÷100 제거 + weight 1/100 재조정. rescale 표 ↓. 의미 튜닝은 전부 §B/§C(2.3-C)로 격리. | tuning.rs L72-75 실값 |
| **P-D-3** | (A2 개정) `closeness_update_rate` dead field **완전 제거**(rename 아님). `deny_unknown_fields`로 rename 호환성 이득 0. 제거 범위 ↓. **L154는 삭제 아닌 치환**. | caller 0 grep / tuning.rs:172 `deny_unknown_fields` / 활성 config 키 0 |
| **P-D-4** | §A presentation 4축. `snapshot.rs:229 closeness_level→affinity_level` + `respect_level`/`wariness_level` 신설. `:233 power_level` **폐기(B-D4)**. `presentation/{locale,formatter}.rs` 라벨 4축. `adapter/memory_repository.rs` = 테스트 doc, 의미 0. | snapshot.rs:229/233 실측 |
| **P-D-5** | (Q3) §C: 게이트 `max@5.0`(`dominant_delta` L63) **FROZEN**. content-projection = **P-D-C1 미결 항목**((b) 잠정, S1~S4 + ±N + 디자이너 확인②). KICKOFF §1-C 3옵션 종결. | dominant_delta fold 실측 |
| **P-D-6** | §D W1: 가드 1·2 quantitative literal `0.286→28.6` / `0.158→15.8`(asserted 실값 bit-동일, direction 가드 scale-invariant 불변). 가드 3(`admiration_no_leak`) **GREEN 유지·제거 금지**(respect→modifier 미연결, Q4 §B 범위 밖). | mapping.rs L800/827/851 본체 |
| **P-D-7** | §E Stage 4 완료 확정. 잔여 = ⑴ stale 주석 `state.rs:664-668` 삭제 ⑵ `appraise-validation/README` axis_modulation 언급 정정 ⑶ `_discarded-v0.6`/`scenarios.backup-v0.6` 영구 폐기(A4, 2.3-B). | git / 활성 scenario clean / README 실측 |
| **P-D-8** | (Q4) `axis_modulation`(src 0 / ReflectionResult 7필드 부재) + S4 시간분산(axis_mod 하드의존) = **Phase 2.5 확정 이관**. §B Phase 2.3 = HEXACO 6→24 차등 spread + significance 게이트만(둘 다 narrative-gated). 디자이너 판단(intensity 0.4/S1~S3) = P-D-C1 동일 확인② 생애주기. | axis_modulation grep=0 / ReflectionResult 7필드 / mind_sync.rs:40 / reflection.rs:100 |
| **P-D-9** | (A3) **comment-drift sweep** 신규 소작업(2.3-B). 옛 3축 어휘 주석/prose 일소. 실측 4건: state.rs:664-668 / situation.rs:36,40,42 / tuning.rs 주석 / lin-chong-shanshenmiao.json `_purpose`. 코드 동작 0. | §0.5 공통 패턴 |

### P-D-2 rescale 표 (값 동치 — 출력 0-델타)

| const (tuning.rs) | 현재 (±1.0 가정) | §A 후 (±100 native) |
|---|---|---|
| `REL_CLOSENESS_INTENSITY_WEIGHT` (L72) | 0.5 | **0.005** |
| `REL_CLOSENESS_EMPATHY_WEIGHT` (L74) | 0.3 | **0.003** |
| `REL_CLOSENESS_HOSTILITY_WEIGHT` (L75) | 0.3 | **0.003** |
| `REL_TRUST_EMOTION_WEIGHT` (L73) | 0.3 | **0.003** |

수식 항등: `(affinity/100) × w ≡ affinity × (w/100)`. `telling_ingestion:80` = `(t+100)/200` 동치식. `snapshot:316-317 from_score` ±100(threshold const 동반). 동반 rename `rel_closeness_*_weight → rel_affinity_*_weight`(logical 정렬).

### P-D-3 제거 범위 (전부 tuning.rs 단일)

- 삭제: `CLOSENESS_UPDATE_RATE`(L57) / 필드 `closeness_update_rate`(L190) / Default 할당(L270) / doc(L335) / 런타임 invariant(L366-367 `< trust_update_rate`, L372-373 `in (0,1)`) / 컴파일 invariant L133(`< TRUST_UPDATE_RATE`)
- **치환(삭제 아님)**: 컴파일 invariant L154 `MEMORY_RELATIONSHIP_DELTA_THRESHOLD >= CLOSENESS_UPDATE_RATE` → `MEMORY_RELATIONSHIP_DELTA_THRESHOLD > 0.0` (live 임계값 가드를 dead 제거하며 몰래 약화 금지)

---

## §2. C 위험 (Phase 2.3 = KICKOFF "중간 위험" phase)

| ID | 위험 | 봉쇄 |
|---|---|---|
| **C-1** | 통로 A 회귀 — §A가 base_delta 흔듦 | Q2 값 동치 → 0-델타. 부동소수 재정렬 미세오차는 tolerance `1e-3`(W1 기존 패턴). 게이트 = S5-D5 정본(failed=0 ∧ 회귀0 ∧ D3 3밴드 exact ∧ 증감 설명가능) |
| **C-2** | W1 깨짐 회귀 오판 | P-D-6 깨짐/안깨짐 표 박제. 1·2=설계 신호(literal), 3=안 깨짐(범위 밖). exact assert 강화 금지 |
| **C-3** | 무한 튜닝(§B/§C/narrative) | S1~S4 박제 + 정량 게이트 `4축 EXPECTED ±5(±100) ∧ D3 3밴드 exact ∧ W1 재조정후 PASS`. tolerance 완화 금지(S5-D4 승계) |
| **C-4** | dead field 제거 부작용 | invariant 동반 누락 시 컴파일 에러(빌드가 잡음=안전). `deny_unknown_fields`로 옛 키 config 하드 에러 — 활성 config 키 0 확정이라 영향 0. backup/discarded 비활성(2.3-B서 폐기) |
| **C-5** | doc-drift 재오독 재발 | P-D-9 sweep 필수. 미실행 시 Phase 2.5 인계자 ①형 재오독(Stage 6 전례) |
| **C-6** | §A 비원자성 | rename+weight rescale+W1 literal+÷100 제거+dead 제거는 **2.3-A 한 커밋 원자** 필수(split 시 중간 빌드/W1 깨짐 — stream-migration "stages together" 선례 동형) |

---

## §3. D baseline (S5-D5 정본 고정)

**측정 명령**: `cargo test --lib --tests --bins` (S5-D5/S6-D1 정본, 변경 금지)

| 항목 | 진입 박제값 (Stage 0 실측) | 비고 |
|---|---|---|
| failed | **0** | log: `baselines/phase2.3-entry-2026-05-17-0852.log` |
| passed | **843** | KICKOFF §3 정본과 정확 일치 |
| ignored | **2** | 동일 |
| result 묶음 | **65** | 동일 |
| git HEAD | `f7ea858` | =Phase2 종결 9339909 + doc 2커밋, 코드 변경 0 |

**정본 정의**: `failed=0 ∧ Stage 0 대비 회귀 0 ∧ D3 narrative 3밴드 exact 보존 ∧ 증감분 전부 설명가능(미스터리 증감=회귀 트리거)`. 절대수 baseline 부적합(S5-D5/S6-D1 승계).

---

## §4. 작업 분해 — 3 Stage (Claude Code 실행 단위)

게이트 성격 분리(Q2 원칙의 Stage 적용). 각 Stage 게이트 실패 시 다음 진행 금지·보고.

### Stage 2.3-A — §A 값 동치 리팩터 (P-D-2/3/4/6) · 원자 1커밋(C-6)

1. `mod.rs:172-173` ÷100 제거 + `tuning.rs` weight 4개 1/100 재조정(P-D-2 표)
2. `closeness_update_rate` dead field 완전 제거(P-D-3 범위, L154 치환)
3. `closeness_* → affinity_*` rename (tuning const/field + `rel_closeness_*_weight→rel_affinity_*_weight`)
4. `telling_ingestion:80` = `(t+100)/200` 동치식. `snapshot:316-317 from_score` ±100 native(threshold const 동반)
5. presentation: `closeness_level→affinity_level` + `respect_level/wariness_level` 신설, `power_level` 폐기(B-D4), locale/formatter 4축 라벨
6. W1 가드 1·2 literal `0.286→28.6` / `0.158→15.8` 재조정. 가드 3 **건드리지 말 것**(GREEN 유지)

**게이트 G1**: `cargo test --lib --tests --bins` → 843 P 유지 ∧ failed=0 ∧ D3 3밴드 exact ∧ W1 가드 1·2 재조정후 PASS ∧ 가드 3 GREEN ∧ 미스터리 증감 0. **원자 1커밋**(메시지 텍스트 Bekay에 제공).

### Stage 2.3-B — 정리 (P-D-9 + P-D-7) · 코드 동작 0

1. comment-drift sweep: state.rs:664-668(고아 Deserialize doc 삭제) / situation.rs:36,40,42("closeness 기반"→"affinity 기반") / tuning.rs 잔여 옛어휘 주석 / lin-chong-shanshenmiao.json `_purpose` "closeness" 정정
2. `appraise-validation/README` axis_modulation 언급 정정(Q4 정합 — Phase 2.5 이관 명시)
3. `_discarded-v0.6/` + `scenarios.backup-v0.6/` 영구 삭제 — **삭제 전 활성 코드/테스트/스크립트/.gitignore 참조 0 확인**(추론 금지)

**게이트 G2**: 옛 3축 어휘 grep 잔존 0 ∧ §E stale 주석 제거 확인 ∧ 빌드 PASS ∧ 삭제 후 `cargo test --lib --tests --bins` 843 P 불변(코드 동작 0). 1커밋.

### Stage 2.3-C — narrative 검증 + 디자이너 (P-D-5/P-D-8) → 확인②

1. `data/scenarios/appraise-validation/` S1~S4 시나리오 JSON 박제 + `_expected_axes_delta` prose
2. P-D-C1 결정: S1~S4 실행 → dominant-1축이 threshold 초과 타 축 소실시키는 케이스 측정(객관)
3. §B: HEXACO 6→24 차등 spread 검토(`mind_sync.rs:40`) + significance 게이트(`reflection.rs:100`) 조정 여부 — narrative 의존
4. 디자이너 판단분: intensity 0.4(§5.2) / S1~S3 narrative 타당성(§5.3) 실측 출력 캡처

**게이트 G3**: C-3 정량 게이트(4축 EXPECTED ±5 ∧ D3 3밴드 exact ∧ W1 PASS) ∧ P-D-C1/intensity/S1~S3 실측+권고 **확인② HTML 보고** → Bekay 종결/정정 판단.

**순서**: 2.3-A → 2.3-B → 2.3-C. 2.3-A G1 실패 시 2.3-B/C 진행 금지.

---

## §5. 종결 게이트 (G1~G4)

- ☐ **G1 (2.3-A)** — §A 0-델타: 843 P ∧ failed=0 ∧ D3 3밴드 exact ∧ W1 1·2 재조정후 PASS ∧ W1 3 GREEN ∧ 미스터리 증감 0 ∧ 원자 1커밋
- ☐ **G2 (2.3-B)** — 옛 3축 어휘 grep 잔존 0 ∧ §E stale 주석 제거 ∧ `_discarded`/`backup` 삭제(참조 0 확인 후) ∧ 빌드·테스트 PASS ∧ 코드 동작 0
- ☐ **G3 (2.3-C)** — S1~S4 박제 ∧ C-3 정량 게이트 통과 ∧ P-D-C1/intensity/S1~S3 확인② HTML 보고·Bekay 판단 완료
- ☐ **G4** — KICKOFF v1.2 §0.5 포인터 1줄 부기 ∧ 본 spec FROZEN 유지 ∧ 외부문서(`00-roadmap.md`/`CLAUDE.md`) Phase 2.3 행 동기화

**Phase 2.3 종결 선언**: G1~G4 전부 ☑ + 확인② Bekay 종결 판단 → Phase 2.3 완료. 다음 = Phase 2.5(LLM 자동화 — axis_modulation/S4 시간분산).

---

## §6. Claude Code 인계 주의

- **P-D-1~9 / C-1~6 변경 금지.** 의문 시 중단·확인, 임의 재해석 금지.
- **2.3-A는 원자 1커밋**(C-6) — rename+rescale+W1 literal+÷100 제거+dead 제거를 쪼개면 중간 빌드/W1 깨짐.
- **W1 가드 3(`admiration_no_leak`) 건드리지 말 것** — Q4 §B 범위 밖. 깨지면 회귀(보고).
- **W1 가드 1·2는 깨지는 게 정상**(literal 재조정 = 설계 신호). 회귀로 오판 금지(C-2).
- 디자이너 git 직접 — Claude Code는 **commit 메시지 텍스트만 제공, git 명령 실행 금지**.
- 의미 튜닝(§B/§C)은 2.3-C 전까지 **금지** — 2.3-A/B는 동작 0 (Q2 분리).
- `_discarded`/`backup` 삭제 전 활성 참조 0 **실측 확인**(추론 금지 — Stage 0 교훈).
- 각 Stage 게이트 실패 시 다음 진행 금지·원인 보고. 확인②는 2.3-C 후 HTML 1회.

---

## §7. 변경 이력

| 버전 | 날짜 | 변경 |
|---|---|---|
| DRAFT | 2026-05-16 | Phase 2 Stage 6 작업6 — 헤더+골격 초안 |
| **1.0 FROZEN** | **2026-05-17** | 확인⓪(Q1~Q4)+확인①(A1~A4) 승인. Stage 0 사실조사로 본체 작성: §0.5 정정 3건(KICKOFF v1.2 §1-C/§1-E 정정) + P-D-1~9 + C-1~6 + D baseline(843P/0F/2I/65묶음) + 3 Stage 분해(2.3-A 원자/B 정리/C narrative) + 종결 게이트 G1~G4. KICKOFF 재작성 안 함(포인터만, P-D-1). DRAFT 폐기. |
