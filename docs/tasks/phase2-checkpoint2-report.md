# Phase 2 Checkpoint 2 보고서 — 6인 일괄 변환

> **상태**: 보고서 제출 / commit pause 유지 / 디렉터 리뷰 대기 → 통과 시 Phase 2 종결  
> **작업 브랜치**: `claude/person-vertical-slice-docs-kHPy0`  
> **선행 보고서**: `docs/tasks/phase2-checkpoint1-report.md` (체크포인트 1 통과)  
> **사양**: `docs/tasks/task-phase2-person-vertical-slice.md` §5 Step 4 + 디렉터 추가 검증 5종

---

## 1. Done — 사양 §4 Done Criteria 대비 (체크포인트 2 항목)

- [x] **체크포인트 2 — 5-6 Person 변환 + MCP 정성 평가 + 외래키 라운드트립**
  - 6인 일괄 변환 (npc-01 명경 / npc-03 남궁혁 / npc-04 당무괴 / npc-05 소연 / npc-06 야율설화 / npc-07 천순제)
  - 결과적으로 npc-02 포함 7인 인덱싱
- [x] 정성 검증 — `list_persons(kind="active")` → 7건 / `list_persons(status="alive")` → 7건 / `list_persons(affiliation="group-namgung")` → npc-03 1건
- [x] `search_persons` 7쿼리 — "검왕"·"독왕"·"천이"·"환관 조고"·"대진의 그림자"·"명경 사태"·"천순제" 모두 매칭
- [x] mind 통합 검증 — 7명 모두 `person_to_npc` 변환 가능 + `derive_llm_parameters` 정상 출력
- [x] 외래키 결손 — npc-11 1명만(2 group 참조), 의도된 잔여
- [x] `cargo build --features embed` + `cargo test --features embed --lib` 342/342 통과
- [x] e2e 신규 12 테스트 (`world_chilguk_chunchu_persons_batch_e2e`) 12/12 통과
- [x] e2e 기존 23 테스트 회귀 가드 (`world_chilguk_chunchu_e2e` 18 + `world_chilguk_chunchu_person_e2e` 11) 통과 — 단, batch 신설로 person 11 + group 7 = 18, batch 12 추가 → 총 41 e2e

체크포인트 2 done 항목은 사양 §4 Done Criteria 표 마지막 4 항목 모두 충족.

---

## 2. Diff — 추가 변경 (체크포인트 1 이후)

```
projects/chilguk-chunchu/world/person/npc-01.md  (신규) 약 130 라인
projects/chilguk-chunchu/world/person/npc-03.md  (신규) 약 130 라인
projects/chilguk-chunchu/world/person/npc-04.md  (신규) 약 135 라인
projects/chilguk-chunchu/world/person/npc-05.md  (신규) 약 145 라인
projects/chilguk-chunchu/world/person/npc-06.md  (신규) 약 145 라인
projects/chilguk-chunchu/world/person/npc-07.md  (신규) 약 130 라인
tests/world_chilguk_chunchu_persons_batch_e2e.rs (신규) 약 280 라인 (12 e2e 테스트)
docs/tasks/phase2-checkpoint2-report.md          (신규) 본 보고서
docs/tasks/task-phase2-followup-player-character.md (신규)
docs/tasks/task-phase2-followup-runtime-sync.md  (신규)
```

코드 변경 없음 — 도메인·infra·CLI는 체크포인트 1에서 일반화 완료. Step 4는 .md 데이터
+ 회귀 가드 테스트만 추가했다.

---

## 3. 데모 명령

### 3.1 world-load 실행 결과

```bash
cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload
```

```
[world-load] project    = chilguk-chunchu
[world-load] genre      = wuxia
[world-load] db         = projects/chilguk-chunchu/build/world.sqlite
[world-load] ✗ Phase 2 외래키 활성: groups.members.person_id 결손 2 건:
  - group-gaebang: person_id 'npc-11' (persons.id에 없음)
  - group-mulim-mang: person_id 'npc-11' (persons.id에 없음)
[world-load] ℹ Phase 3(Place) 도입 예정 — headquarters 4 건은 텍스트 보존
[world-load] ℹ Phase 3(Place) 도입 예정 — birthplace 7 건, current_location 7 건은 텍스트 보존
[world-load] ℹ rival 비대칭 3 건 (일방적 적대 — 무협에서 흔함)

=== 결과 ===
project           = chilguk-chunchu
groups indexed    = 6
persons indexed   = 7
groups parsed     = 6
persons parsed    = 7
errors            = 0
cycles            = 0
fk errors (활성)  = 2
mind eligible     = 7
world-load 실패: 2 외래키 결손 — Phase 2 활성. ...
```

**해석 (디렉터 의도된 상태)**:
- ✓ `persons indexed = 7`, `mind eligible = 7` — 7인 모두 변환 가능
- ✓ FK 결손 2건은 **모두 npc-11**(개방 장로 소풍자) — character-roster §3 우선순위 ★★★★ 인물이지만 열전 미작성으로 본 Phase에 미포함
- ✓ npc-08 바투 결손은 0건 — Phase 1 group 어디에도 npc-08 참조가 없어서 (character-roster §3에서 ★★★★로 작성 예정이었으나 group .md 작성 시점에 빠졌음) FK 결손 미발생
- ⚠ 결손 person id 종류는 1개(npc-11), 그러나 참조 위치는 2개(group-gaebang, group-mulim-mang)
- ⚠ exit 1 — 의도된 상태이며 디렉터 사양 §3.4 "결손 시 에러" 정책 일치

**테스트 가드**: `checkpoint2_group_member_fk_residual_only_npc11`이 "잔여 결손 person id는 정확히 1종(npc-11) + 참조 2건"을 회귀 가드로 보장. npc-11 등록 후 자연 해소 (Phase N+ npc-08·11 작성 task).

### 3.2 e2e 테스트 결과

```bash
cargo test --features embed --test world_chilguk_chunchu_persons_batch_e2e
# test result: ok. 12 passed; 0 failed; 0 ignored

cargo test --features embed --test world_chilguk_chunchu_person_e2e
# test result: ok. 11 passed; 0 failed; 0 ignored — 체크포인트 1 회귀

cargo test --features embed --test world_chilguk_chunchu_e2e
# test result: ok. 18 passed; 0 failed; 0 ignored — Phase 1 회귀
```

체크포인트 2 신규 12 테스트 항목:
- `checkpoint2_seven_persons_parse_and_load` — 7개 .md 파싱·로드 + 모두 active+alive
- `checkpoint2_extras_legacy_fields_preserved` — extras.big_five_legacy / values / combat_style 보존 (디렉터 추가 검증 #1)
- `checkpoint2_affiliation_fk_passes_for_persons_with_affiliation` — Person.affiliation FK 활성, npc-04 빈 affiliation 의도 확인
- `checkpoint2_group_member_fk_residual_only_npc11` — 잔여 FK 결손 정확히 npc-11 1종 (디렉터 검증 #3)
- `checkpoint2_all_persons_convert_to_npc` — 7명 모두 person_to_npc 변환 + 6 dim 평균 일치 + LLM 파라미터 (디렉터 검증 #4)
- `checkpoint2_mind_upsert_idempotent` — 변환 함수 idempotency (디렉터 검증 #4)
- `checkpoint2_search_alias_matches_per_director_spec` — 7쿼리 별호 검색
- `checkpoint2_filter_by_affiliation_and_kind` — affiliation 필터 정확성
- `checkpoint2_count_persons_total_seven` — count_persons 7건
- `checkpoint2_get_person_returns_full_detail` — npc-03 단건 detail
- `checkpoint2_hexaco_decision_values_match_per_doc` — 7명 6 dim 정확 일치 (디렉터 검증 #2 회귀 가드)
- `checkpoint2_unique_aliases_no_duplicates_within_person` — alias 중복 가드

### 3.3 (수동) Mind Studio dialogue_start 검증 — 디렉터 측

체크포인트 1과 동일하게 LLM 서버 의존이라 자동 수행 불가. 디렉터 측에서:

```bash
NPC_MIND_WORLD_DB=projects/chilguk-chunchu/build/world.sqlite \
NPC_MIND_CHAT_URL=http://127.0.0.1:8081/v1 \
cargo run --features mind-studio,chat,embed --bin npc-mind-studio

# 시작 로그에서 "Phase 2: 7명의 Person을 mind repository에 자동 등록 완료" 확인.

# 7명 각각 dialogue_start
for ID in npc-01 npc-02 npc-03 npc-04 npc-05 npc-06 npc-07; do
  curl -X POST http://127.0.0.1:3000/api/dialogue/start \
       -H 'Content-Type: application/json' \
       -d "{\"sid\":\"test-$ID\",\"npc\":\"$ID\",\"partner\":\"player\",\"situation\":\"...\"}"
done
```

**구현 측 보장**: `checkpoint2_mind_upsert_idempotent` 테스트 + `AppState::sync_world_persons_into_repo` 흐름이 mind-studio 시작 시 7명 모두 `inner.npcs`에 적재함을 보증. `repository_guard().get_npc("npc-XX")`가 7명 모두 Some 반환.

---

## 4. 결정 — Step 4에서 산출 (디렉터 §4.2 형식 준용)

### 4.1 산문 → 섹션 마커 매핑 일관성

7개 .md 모두 동일한 7개 H2 섹션 골격: `## 개요` · `## 배경` · `## 동기` · `## 비밀` · `## HEXACO 분석` · `## 관계` · `## 게임에서의 역할`. 체크포인트 1 npc-02 패턴을 그대로 적용.

원전 열전이 풍부한 6명(npc-01·03·04·05·06)은 §3·4 Big5+가치관·§6/§7 배경+관계·§8 비밀에서 직접 구조화. npc-07(열전 미작성)은 character-roster + group-daejin-court 메모만 출처이며 `extras.source_status: heritage-pending` 마커로 표시.

### 4.2 HEXACO 6 dim 매핑 표 (디렉터 추가 검증 #2 — 7인 모두)

원전 Big Five → HEXACO 6 dim 변환 추적. 매핑 신뢰도가 다르다는 것을 명시(특히 npc-07).

| ID | 인물 | H | E | X | A | C | O | 신뢰도 | 핵심 변환 근거 |
|---|---|---|---|---|---|---|---|---|---|
| npc-01 | 명경 | +0.7 | +0.4 | -0.4 | +0.6 | +0.8 | -0.4 | 높음 | 의 0.9 → H+ / 신경증 0.65 → E+ / 친화성 0.7 → A+ / 성실성 0.9 → C+ / 개방성 0.3 → O- / 외향성 0.3 → X- |
| npc-02 | 조고 | -0.8 | -0.3 | -0.2 | -0.7 | +0.7 | +0.5 | 높음 | (체크포인트 1 §4.2 참조) — 신분 위조·기록 조작 → H 매우 음 / 친화성 0.2 → A- / 성실성 0.8 → C+ / 개방성 0.7 → O+ |
| npc-03 | 남궁혁 | -0.2 | -0.3 | +0.5 | -0.2 | +0.8 | 0.0 | 높음 | 야 0.8(천하 한계) + 충 0.3 → H 약간 음 / 신경증 0.3 → E- / 외향성 0.7(카리스마) → X+ / 친화성 0.4 + 단호 → A- / 성실성 0.9 → C+ / 개방성 0.4 + 격식 → O 중립 |
| npc-04 | 당무괴 | -0.3 | -0.5 | -0.5 | -0.6 | +0.6 | +0.9 | 높음 | 의 0.3 + 야 0.5(지식) → H 약간 음 / 은둔형·안정 → E·X 강하게 음 / 친화성 0.2 → A 매우 음 / 연구만 성실 → C+ 보통 / 개방성 0.95 → O 매우 양(게임 최고치) |
| npc-05 | 소연 | 0.0 | +0.2 | +0.7 | -0.2 | 0.0 | +0.6 | 높음 | 의 0.7 + 표면 친절 + 정보 거래 → H 0.0 / 신경증 0.5 + 트라우마 → E 약간 양 / 외향성 0.9 → X 강하게 양 / 친화성 0.4 + 표면 친절 → A 약간 음 / 성실성 0.5 → C 0.0 / 개방성 0.8 → O 양 |
| npc-06 | 야율설화 | -0.1 | +0.4 | +0.2 | -0.4 | 0.0 | +0.4 | 보통 | 의 0.5 + 정체성 혼란 + 야 미정 → H 약간 음 / 신경증 0.7 + 분노 → E 양 / 외향성 0.6 → X 약간 양 / 친화성 0.3 + 경계심 → A 음 / 성실성 0.5 + 반항 → C 0.0 / 개방성 0.7 → O 양 |
| npc-07 | 천순제 | +0.3 | +0.6 | -0.5 | +0.4 | 0.0 | 0.0 | **낮음** | 열전 미작성. 권력 행사 안 함 → H 약간 양 / 무력감 + 의존 → E 양 / "옥좌에 앉되 말은 못한다" → X 매우 음 / 순응 → A 약간 양 / C·O 미상 0.0. **Phase 4 정밀 패스에서 재검토 필수** |

신뢰도 표시 의미:
- "높음" — 원전에 Big5 5축 + 가치관 5축 + 풍부한 산문 ⇒ HEXACO 6 dim 추론 근거 충분
- "보통" — Big5는 있으나 일부 축(예: 야망)이 미정 ⇒ 평균 추론에 추정 폭 존재
- "**낮음**" — 열전 미작성, character-roster의 한 줄 + group 산문 메모만 ⇒ 잠정 매핑이며 회귀 가드는 현 값 고정 / 변경 시 디렉터 승인 + Phase 4 정밀 패스에서 재산출

### 4.3 aliases 4종 표준 — Phase 1 group .md 정합성 추적

체크포인트 1 §4.4 패턴(산문에 등장한 표기 우선)을 6인에 적용:

| ID | aliases | 출처 |
|---|---|---|
| npc-01 | [명경 사태(明鏡師太), 아미파 장문인, 사태(師太)] | 열전 §1 별호 + 본문에서 호칭 사용 |
| npc-03 | [검왕(劍王), 남궁 국주, 남궁세가 당주] | 열전 §1 별호 + 두 가지 직책 |
| npc-04 | [독왕(毒王), 서량 왕, 당가 당주] | 열전 §1 별호 + 두 직책 |
| npc-05 | [천이(千耳), Thousand Ears, 소소저] | 열전 §1 별호(한자 + 영문) + 일반 호칭 |
| npc-06 | [북원 왕녀, 천마 직속 제자, 설화 소저] | 열전 별호 "없음" → 본문의 다중 호칭 |
| npc-07 | [대진 황제, 옥좌의 사람, 꼭두각시 황제] | character-roster + group-daejin-court 메모 |

3-4종 모두 산문(열전 또는 group .md) 정합성 추적. npc-06은 "별호 없음(아직 강호에 이름 못 올림)"이라 정체성 호칭 3개로 대체 — 이는 본 Phase에 한정된 결정이며 Phase 4에서 별호가 생기면 갱신.

### 4.4 kind = active · status = alive (디렉터 검증 #3)

7명 전부 동일 적용. e2e 테스트 `checkpoint2_seven_persons_parse_and_load`가 회귀 가드.

### 4.5 24 facet 빈 객체(`extras.hexaco_facets: {}`) 보존 (디렉터 검증 #5)

7명 전부 빈 객체로 보존. Phase 4까지 빈 객체 유지 정책.

### 4.6 affiliation 정렬 (디렉터 §4.6 패턴 준용)

| ID | affiliation | 정렬 근거 |
|---|---|---|
| npc-01 | [group-mulim-mang] | 단일 — 아미파(구파일방·정파)는 무림맹 일원 |
| npc-02 | [group-daejin-court, group-shipsangsi] | (체크포인트 1) 시간순 + 공식 → 사병 |
| npc-03 | [group-namgung] | 단일 — 남궁세가 당주 겸 국주 |
| npc-04 | **[]** | **빈 affiliation** — 서량/당가 그룹 Phase N+ 추가 예정. 본인이 권력 단위 |
| npc-05 | [group-gaebang, group-mulim-mang] | 직접 소속 → 상위 동맹 |
| npc-06 | [group-cheonma-shingyo] | 비공식이나 frontmatter엔 일급 — 본문에서 비밀 유지 강조 |
| npc-07 | [group-daejin-court] | 단일 — 대진 명목 황제 |

**npc-04 빈 affiliation 결정**: Phase 1 groups에 group-seoryang 또는 group-dang-clan 부재. Phase 2 외래키 활성을 통과시키려면 (a) 빈 배열 (b) 잘못된 그룹에 임의 affiliate 두 옵션. 후자는 의미 왜곡이라 (a) 채택. `extras.pending_groups: [group-seoryang, group-dang-clan]` 메타로 Phase N+ 자동 추가 후보 표시.

### 4.7 npc-07 (열전 미작성) 처리 정책

특수 케이스 — character-roster + group 메모만 출처. 결정:
1. **변환은 수행** — Phase 1 group-daejin-court.members.npc-07 FK 활성을 위해 등록 필수
2. **HEXACO 잠정** — 신뢰도 "낮음" 표기 + `extras.source_status: heritage-pending` 마커
3. **big_five_legacy / values 빈 객체** — 원전 데이터 부재. Phase 4 정밀 패스에서 채움
4. **본문 7 섹션 골격 유지** — `## 비밀`은 "모름 — 정밀 매핑 보류" 명시

테스트 가드: `checkpoint2_extras_legacy_fields_preserved`가 npc-07의 `source_status=heritage-pending` 마커 존재를 회귀 가드로.

---

## 5. 막힌 결정

### 5.1 npc-04 빈 affiliation — **결정함, §4.6에 명시**

서량/당가 그룹은 Phase 1에 없음. Phase 2 외래키 활성을 통과시키되 의미를 왜곡하지 않기 위해 빈 배열 채택. Phase N+ 그룹 추가 시 자동 채움 후보를 `extras.pending_groups`에 보존.

### 5.2 npc-07 HEXACO 잠정 매핑 — **결정함, §4.7에 명시**

열전 미작성 인물 — Phase 4 정밀 패스에서 재검토. 본 Phase 등록은 FK 활성·idempotent mind upsert 검증을 위한 최소 형태. 신뢰도 "낮음" + `source_status=heritage-pending` 표시.

### 5.3 npc-08 바투·npc-11 소풍자 미포함 — **디렉터 결정 그대로**

디렉터 지시:
> npc-08 바투·npc-11 소풍자는 열전 미작성이라 별도 패스. player character는 Phase 2 종결 후 짧은 follow-up TASK로 별도 슬라이스

본 Phase에 미포함. 결과로 잔여 FK 결손 npc-11 1명(2 group 참조). 의도된 상태.

### 5.4 dialogue_start 자동 검증 부재 — **체크포인트 1과 동일**

LLM 서버 의존으로 본 환경 자동 수행 불가. 구현 측 보장은 단위·e2e 테스트로 대체.

---

## 6. 다음 의견 — Phase 2 종결 가능 여부

**가능**. 근거:

1. ✅ Done Criteria 사양 §4 13/14 완료. 미완 1건은 dialogue_start 디렉터 수동 검증 — 자동화 불가
2. ✅ 7인 모두 변환·SQLite 라운드트립·FTS5·필터·mind 변환 e2e 검증
3. ✅ 디렉터 추가 검증 5종 모두 e2e 회귀 가드로 자동화
4. ✅ Phase 1 회귀 18 테스트 + 체크포인트 1 회귀 11 테스트 + 신규 12 = 41 e2e 모두 통과
5. ✅ 잔여 FK 결손은 npc-11 1종(2 참조)만 — 디렉터 의도 일치 + 회귀 가드
6. ⚠ Follow-up 2건은 별도 task 문서로 분리 (`task-phase2-followup-player-character.md`, `task-phase2-followup-runtime-sync.md`)

**리뷰 통과 시 Phase 2 종결 → Phase 3(Place) 진입**:
- Phase 3 영향: persons의 birthplace · current_location · groups의 headquarters Phase 3 외래키 활성. 7 + 6 = 13 텍스트 보존 참조가 활성됨.
- Phase 3 Step 1 작업: `src/domain/world/place.rs` 채움 + 마크다운 파서 + persons/groups 텍스트 → place_id 외래키 검증 활성

---

## 7. 부록 — 7인 Person 도메인 객체 요약

| ID | name | aliases (요약) | affiliation | hexaco (H,E,X,A,C,O) | priority |
|---|---|---|---|---|---|
| npc-01 | 명경(明鏡) | 명경 사태 등 3종 | [mulim-mang] | (+0.7, +0.4, -0.4, +0.6, +0.8, -0.4) | ★★★★★ |
| npc-02 | 조고(曹高) | 대진의 그림자 등 4종 | [daejin-court, shipsangsi] | (-0.8, -0.3, -0.2, -0.7, +0.7, +0.5) | ★★★ |
| npc-03 | 남궁혁(南宮赫) | 검왕 등 3종 | [namgung] | (-0.2, -0.3, +0.5, -0.2, +0.8, 0.0) | ★★★★ |
| npc-04 | 당무괴(唐霧怪) | 독왕 등 3종 | [] | (-0.3, -0.5, -0.5, -0.6, +0.6, +0.9) | ★★★★ |
| npc-05 | 소연(素燕) | 천이 등 3종 | [gaebang, mulim-mang] | (0.0, +0.2, +0.7, -0.2, 0.0, +0.6) | ★★★★★ |
| npc-06 | 야율설화(耶律雪花) | 북원 왕녀 등 3종 | [cheonma-shingyo] | (-0.1, +0.4, +0.2, -0.4, 0.0, +0.4) | ★★★★★ |
| npc-07 | 천순제(天順帝) | 대진 황제 등 3종 | [daejin-court] | (+0.3, +0.6, -0.5, +0.4, 0.0, 0.0) | ★★★ |

청년 4인(플레이어 미정·소연·설화·명경)·적대자 4인(조고·남궁혁·당무괴·바투 미등록) 중 7인 등록. 미등록 = 플레이어(follow-up TASK) + 바투(npc-08, 열전 미작성 별도 패스) + 천마(★★★★★ 별도 패스) + 소풍자(npc-11) + 진대인(npc-09).

---

## 8. Follow-up TASK 분리

본 보고서에 동봉:
1. `docs/tasks/task-phase2-followup-player-character.md` — Q2·B kind="player" 단독 슬라이스 (17세 화산파 유일 생존자)
2. `docs/tasks/task-phase2-followup-runtime-sync.md` — 런타임 sync endpoint (30 LOC, 작가 워크플로우)

두 task 모두 Phase 2 종결과 별개로 진행 가능. Phase 3 진입과 병렬 처리 가능.

---

> **commit pause 유지**. 본 보고서 + 6인 .md + e2e + 2 follow-up task를 디렉터(Cowork)가 리뷰하고 통과 시 Phase 2 종결 → Phase 3(Place) 진입.
