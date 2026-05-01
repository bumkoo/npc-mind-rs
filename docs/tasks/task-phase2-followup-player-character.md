# Phase 2 Follow-up — Player Character (Q2·B kind="player") 단독 슬라이스

> **For Claude Code.** Phase 2 본 task(`task-phase2-person-vertical-slice.md`)와 분리된 짧은 후속 슬라이스.
> 결정 사항을 임의 변경하지 말 것. 변경이 필요하면 디렉터 승인 요청.
> **선행 조건**: Phase 2 체크포인트 2 통과 (7 Person 등록 완료, mind upsert 동작 확인).

## 1. 목표

Phase 2의 본문 §3.3 / §6.7 에서 결정한 **Q2·B 정책**(별도 PlayerCharacter 도메인 안 만듦, Person.kind="player" 인스턴스 1개로 처리)을 단독으로 검증한다.

스코프는 **칠국춘추 시나리오의 17세 화산파 유일 생존자 1명** — 플레이어 캐릭터의 초기값(이름 미정, 화산파 멸문 직후 명경에게 맡겨진 상태)을 .md로 등록하고, mind upsert가 다른 active NPC와 동일하게 동작하는지 확인.

## 2. 연관 컨텍스트

- `task-phase2-person-vertical-slice.md` §3.3 / §6.7 — Q2·B 결정 + kind="player" 정책
- `phase2-checkpoint2-report.md` — 7 Person 변환 결과 (player 미포함 상태)
- `wuxia-core/docs/characters/character-roster.md` §1 — "플레이어 (이름 미정), 17세, 화산파 유일 생존자"
- `wuxia-core/docs/characters/칠국춘추_플레이어_캐릭터_시트.md` — 플레이어 캐릭터 시트 (실 데이터 출처)

## 3. 제약

### 3.1 ID 결정

- `id: player` (단일 인스턴스 가정 — 멀티 플레이어 미지원 단계). 또는 `id: pc-01`로 향후 멀티플레이어 확장 가능성 보존. **디렉터 결정 필요** — Phase N+ 멀티플레이어 도입 시 후자가 깔끔.
- `name`은 시나리오 시작 시 사용자 입력으로 갱신되는 가변 항목. 본 .md는 "이름 미정"으로 둠 + mind-studio가 사용자 입력 시 `inner.npcs[id].name`만 in-place 갱신하는 단순 흐름 (현재 add_npc HashMap insert가 그대로 동작).

### 3.2 kind 옵션

- `genres/wuxia/forms/person.toml`에 `player` 옵션 이미 등록됨 (Phase 2 체크포인트 1).
- `worldbuilding::mind_sync::person_to_npc`도 `kind="player"`를 mind 적격으로 처리 (단위 테스트 `player_kind_converts` 통과 중).

### 3.3 HEXACO 정책

- 시작값은 화산파 검학 + 17세 청년 + 멸문 직후 트라우마 + 임서운에게 구해진 경험을 반영한 **"기본 무인" 프로필**.
- 시나리오 시작 후 게임 도중 HEXACO 갱신은 mind 시스템의 emotion·관계·기억 변화로 처리되며 .md 자체는 갱신되지 않음 (=mind upsert는 시작 시점만).

권장 시작값 (디렉터 검토 필요):
- H: +0.5 — 화산파의 정파적 정직성 학습
- E: +0.3 — 17세 + 멸문 트라우마 + 명경에게 의존
- X: 0.0 — 청년기 평균
- A: +0.4 — 명경의 가르침에서 자비 학습
- C: +0.5 — 화산파 수련 규율
- O: +0.5 — 청년기 평균 + 새로운 환경(낙양·자유도시 등) 노출 시 상승

## 4. Done Criteria

- [ ] `projects/chilguk-chunchu/world/person/player.md` (또는 `pc-01.md`) 작성 — kind="player" 1행
- [ ] world-load 실행 시 `mind eligible = 8` (기존 7 + 1)
- [ ] e2e 테스트 1-2건 추가 — kind="player" 필터 + mind 변환 검증
- [ ] mind-studio 시작 로그 "Phase 2: 8명의 Person을 mind repository에 자동 등록 완료"
- [ ] 디렉터 수동 검증: `dialogue_start("player", partner="npc-01")` 같은 역방향 호출 (플레이어가 NPC 역할로 등장하는 케이스 없음 — 본 검증은 단순 등록 가능성 확인용)

## 5. 단계별 작업

### Step 1 — ID 결정 + frontmatter 작성

디렉터에게 `player` vs `pc-01` 선택 묻기. 결정 후 단일 .md 작성 (약 80-100 라인, 본문은 시나리오 시작 시점 인물 묘사).

### Step 2 — world-load 재실행 + e2e 추가

- `cargo run --features embed --bin world-load -- --project chilguk-chunchu --reload`
- 결과: persons indexed = 8, mind eligible = 8
- 기존 npc-11 잔여 FK는 그대로 (이 task 범위 외)

신규 e2e 테스트 1-2건:
- `player_kind_loads_and_converts` — kind="player" 필터 + person_to_npc Some
- `player_present_in_mind_eligible_list` — 8명 적격 검증

### Step 3 — 보고서

`docs/tasks/phase2-followup-player-report.md` (약 100 라인) — 결정 사항 + 변환 결과 + Phase 2 본문 §3.3 Q2·B 검증 통과 여부.

## 6. Out of Scope

- 멀티플레이어 (id 충돌·세션 분리 등) — Phase N+
- 플레이어 빌드 시스템 (skill·class·level·equipment) — 게임 플레이 Phase
- 플레이어 이름·외모 사용자 커스터마이징 UI — Phase N+
- 화산파 검학·보심결 등 무공 시스템 — Phase 5+ (Skill 도메인)

## 7. 추정 작업량

- Claude Code 작업 시간: 1-2 시간 (90% .md 작성 + 10% e2e + 보고서)
- 코드 변경 줄 수: 0 — Phase 2 본문에서 일반화 완료. 데이터만 추가.

## 8. 시작 체크리스트

1. 디렉터에게 `id: player` vs `id: pc-01` 선택
2. `wuxia-core/docs/characters/칠국춘추_플레이어_캐릭터_시트.md` 통독
3. .md 작성 + e2e 추가 + 보고서
