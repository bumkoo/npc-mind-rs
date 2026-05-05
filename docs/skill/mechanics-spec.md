# Skill 도메인 메카닉 Spec

> **버전**: v1.0 (초안)
> **작성일**: 2026-05-04
> **상태**: Phase 6 진입 전 정책 결정 — 구현 시작 전 검토 필요
> **선결 docs**:
> - 도구 결: [00-roadmap.md](../tasks/00-roadmap.md) §1 (3계층 분리)
> - 장르 결 시드: `wuxia-core/docs/architecture/{growth,relationship}-mechanic.md`
> - 시간 도메인: `wuxia-core/src/shared/time.rs` (Watch v1.1)

---

## §1. 목적과 경계

### 1.1 무엇을 위한 문서인가

Phase 6 — Skill vertical slice 진입 *전*, 컨텐츠 작성 중 슬롯 빈틈으로 인한 도메인 재수정을 막기 위한 *룰의 윤곽*을 잠그는 문서. **구현(Phase 7+) 아닌 spec**. L2 깊이 — 자연어 정책 + 작동 매트릭스, 수식·의사코드 없음.

### 1.2 도구 결 vs 장르 결 분리

7개 도메인이 굳혀 온 컨벤션 그대로 적용, 새 결정 0:

| 결 | 위치 | Skill에서 책임 |
|---|---|---|
| 도구 | `src/domain/world/skill.rs` | struct 슬롯, 외래키 매트릭스, prerequisites DAG, school 외래키 검증 |
| 장르 | `genres/wuxia/skill/` (Phase 6 신설) | kind 식별자, Aura 4종, Alignment, 주화입마 trigger, HEXACO·Watch 정합 |
| 사용자 | `projects/chilguk-chunchu/world/skill/*.md` | 인스턴스 (남궁가 검법, 매화검법 등) |

`Skill.kind`는 free-form `String` (7도메인 컨벤션). 장르가 "권장 식별자 목록" 제공.

---

## §2. 9축 결합 매트릭스 — 한 표

| # | 교차 축 | 시드 위치 (이미 정의) | Phase 6 | Phase 7+ |
|---|---|---|---|---|
| 1 | HEXACO ↔ Skill | growth-mechanic §1 (부산물) | C/O/H 정책 + L2 매트릭스 (§5) | 계산 함수, 주화입마 이벤트 |
| 2 | Prerequisites (DAG) | 본 문서 §3 | 외래키 활성 + DAG 무순환 | 습득 시 prereq 검증 |
| 3 | Item(비급) ↔ Skill | — | `extras.required_item` 텍스트만 | Item 도메인 + 외래키 |
| 4 | 문파(Group) ↔ Skill | — | `school` 외래키 + sect 정합 | 충돌·시너지 계산 |
| 5 | Place ↔ Skill | growth-mechanic §3, dev-plan §3.4 | **인용만** (재정의 금지) | 수련 보너스 계산 |
| 6 | Watch/Season ↔ Aura | shared/time.rs 코드 주석 | Aura 4종 + Watch 매트릭스 (§4.2) | 시진별 위력 계산 |
| 7 | 혈맥 ↔ Skill | — | `extras.bloodline_requirement` 텍스트 | Person.extras 결합 |
| 8 | Lineage(사부-제자) | relationship-mechanic §2.3 | `extras.lineage_chain` 텍스트 | Relationship × Skill |
| 9 | 주화입마 trigger | growth-mechanic §5.3 (역방향) | trigger 조건 (§6) | 이벤트 발화 |

**이미 정의되어 인용만 — Phase 6에서 *재작성 금지***:
- 기연 5조건 (場·人·物·心·時) → growth-mechanic §5
- 부산물 능력치 매핑 (무공 유형 → 주 관련 StatType +1) → growth-mechanic §1
- 무공 경지 4단계 (입문/숙련/통달/화경) → growth-mechanic §5.7
- Master-Disciple 효과 ("기연 조건②, 성장 보너스") → relationship-mechanic §2.3

---

## §3. 도구 결 — Skill 도메인 슬롯 카탈로그

### 3.1 struct 정의 (Phase 6 코드)

```rust
// src/domain/world/skill.rs
pub struct Skill {
    pub id: SkillId,
    pub name: String,
    pub aliases: Vec<String>,

    /// 장르가 채움. 권장 식별자는 `genres/wuxia/skill/kinds.toml`.
    pub kind: String,

    pub temporal: SkillTemporal,         // creation_year_relative, status

    // 외래키 매트릭스 (Phase 6 검증 게이트)
    pub school: Option<GroupId>,         // 사문
    pub founder: Option<PersonId>,       // 창시자 (historical)
    pub current_masters: Vec<PersonId>,  // 현 전승자
    pub prerequisites: Vec<SkillId>,     // 선결 무공 (DAG)
    pub related_event: Option<EventId>,  // 창시·봉인·실전

    pub body: String,                    // 마크다운 본문 (구결·서술)

    #[serde(default)]
    pub extras: HashMap<String, JsonValue>,
}

pub enum SkillStatus { Active, Sealed, Lost, Forbidden }
```

### 3.2 외래키 활성 시점

| 외래키 | Phase 6 | Phase 7+ |
|---|---|---|
| `school: GroupId` | ✅ 활성 + sect 검증 | — |
| `founder: PersonId` | ✅ 활성 | — |
| `current_masters: Vec<PersonId>` | ✅ 활성 | — |
| `prerequisites: Vec<SkillId>` | ✅ 활성 + DAG 무순환 | — |
| `related_event: EventId` | ✅ 활성 (5a 양방향 패턴) | — |
| `extras.required_item` | 텍스트만 | Item 도입 시 외래키 활성 |
| `extras.bloodline_requirement` | 텍스트만 | Person.extras 결합 |
| `extras.lineage_chain` | 텍스트만 | Relationship × Skill |

### 3.3 Prerequisites DAG 정책

- 자기 자신 참조 금지 (`A → A` reject)
- 순환 금지 (`A → B → A` reject) — 토폴로지 정렬 검증
- 전이 닫힘은 *암묵* (A → B, B → C이면 A는 C도 prereq) — 인스턴스에 명시 안 함
- 양방향 역참조 — `Skill.dependents`는 빌드 시 자동 생성 (5a related_events 패턴)


---

## §4. 장르 결 — Aura 4종 도입

### 4.1 정의

Alignment(정/사/중립)와 *직교*하는 추가 차원. 무공의 *기운 색채*를 표현 — 양강·음유·마기·불문.

| Aura | 한자 | 색채 | 정전 시드 |
|---|---|---|---|
| `yang-hard` | 陽剛 | 강맹·외향·정공 | 소림 권법, 남궁가 검법 |
| `yin-soft` | 陰柔 | 유려·내향·변화 | 화산 매화검법, 아미파 의술 |
| `demon-qi` | 魔氣 | 파괴·통제·잠식 | 천마신공, 혈교 제 무공 |
| `buddha-pure` | 佛門 | 자비·정화·항마 | 소림 반야공, 달마역근경 |

`Skill.extras["aura"]` 슬롯에 위 4값 중 하나. 미지정 시 `neutral` (대다수 일반 기술).

### 4.2 Aura × Watch 정합 매트릭스 (L2)

`shared/time.rs` 코드 주석의 시진별 보너스를 Aura로 매핑·확장. **작동하는 칸만 명시** — 무관한 칸은 빈 칸(neutral).

| Watch (시진) | yang-hard | yin-soft | demon-qi | buddha-pure |
|---|---|---|---|---|
| Dawn (인묘 03~07) | — | **위력 ↑** | — | **위력 ↑↑** |
| Morning (진사 07~11) | — | — | — | — |
| Midday (오미 11~15) | **위력 ↑↑** | 위력 ↓ | 위력 ↓ | — |
| Afternoon (신유 15~19) | — | — | — | — |
| Evening (술해 19~23) | — | 위력 ↑ | — | — |
| Night (자축 23~03) | 위력 ↓ | **위력 ↑** | **위력 ↑↑** | 위력 ↓ |

**근거**:
- yang-hard ↔ Midday: 양기 절정 → 외공·검법 위력 ↑ (shared/time.rs 시드 그대로)
- yin-soft ↔ Dawn/Night: 음기 시간 → 도가 무공 적합
- demon-qi ↔ Night: 자시 사파 활동 (shared/time.rs 시드)
- buddha-pure ↔ Dawn: 새벽 예불·내공 (shared/time.rs 시드 + 항마 결)

### 4.3 Aura × Alignment 직교 매트릭스

두 차원이 *상관은 있으나 독립*임을 명시:

| | yang-hard | yin-soft | demon-qi | buddha-pure |
|---|---|---|---|---|
| **Orthodox(정)** | 남궁가 검법 | 매화검법 | 비고① | 소림 반야공 |
| **Heterodox(사)** | 비고② | 비고③ | 천마신공 | 비고④ |
| **Neutral(중립)** | 산적 도법 | 강호 검법 | 야인 마공 | — |

- 비고① — 정파+마기: 정파 무공이 주화입마로 변질. 매우 드묾, 서사 장치
- 비고② — 사파+양강: 외문 사파 (예: 흑풍채 권법). 가능
- 비고③ — 사파+음유: 백사문 같은 음유 사파. 가능
- 비고④ — 사파+불문: 모순. 불가 (검증 reject)

→ 직교성 검증: **Aura/Alignment 매트릭스 12칸 중 1칸(④)만 reject**, 나머지는 모두 작가가 사용 가능.


---

## §5. HEXACO ↔ Skill 정책 (L2)

### 5.1 정책 진술 (자연어)

핵심 3축 — C(성실성), O(개방성), H(정직-겸손). 나머지 3축(E·X·A)은 *부수적*. 첨부 외부 자료가 짚은 결합을 칠국춘추 메카닉과 정합시킴.

- **C(성실성) 높음** → 모든 무공 *완숙도* ↑. 수련 효율 보너스, 화경 진입 임계 ↓ (※ 노화 단계 계수와 별도, growth-mechanic §3.1)
- **O(개방성) 높음** → 비급(`extras.required_item`) 해독 가능, 기연 5조건 ⑤"시기"의 *깨달음 판정* ↑ (growth-mechanic §5.1 인용)
- **H(정직-겸손) 높음** → Orthodox 무공 친화, 마기(`demon-qi`) 무공 *학습 거부* (수련 자체 진입 불가)
- **H(정직-겸손) 낮음** → 사파 무공 학습 가능, 마기 무공 진입 *허용* (단 §6 주화입마 위험 동반)
- **E(정서성) 높음 × demon-qi** → 주화입마 발생 확률 추가 (감정 통제 부족)
- **A(우호성) 낮음 × Heterodox** → 강호 적대 관계 가속 (Skill 학습 자체엔 영향 없음, 서사 부산물)
- **X(외향성)** → 직접 영향 없음 (Skill 메카닉 차원에서)

### 5.2 L2 매트릭스 — 작동 칸만

| HEXACO 축 | yang-hard | yin-soft | demon-qi | buddha-pure | neutral |
|---|---|---|---|---|---|
| **H 높음** (≥+0.3) | 친화 | 친화 | **거부** | **친화 ↑↑** | — |
| **H 낮음** (≤−0.3) | — | — | 친화 | **거부** | — |
| **C 높음** (≥+0.3) | 완숙도 ↑ | 완숙도 ↑ | 완숙도 ↑ | 완숙도 ↑ | 완숙도 ↑ |
| **O 높음** (≥+0.3) | 깨달음 ↑ | 깨달음 ↑↑ | — | 깨달음 ↑ | 깨달음 ↑ |
| **E 높음** (≥+0.3) | — | — | **주화입마 ↑** | — | — |

**거부**(reject) = 학습 시도 자체가 실패, `learn_skill` 이벤트 발화 안 됨.
**완숙도 ↑** = growth-mechanic §3.1의 `growth_multiplier` 추가 보정 (Phase 7+ 구현).

### 5.3 기연 5조건 ④"감정" Aura 매핑 (인용 + 확장)

기연 5조건은 *재정의 안 함* (growth-mechanic §5). Aura가 도입됨에 따라 ④"감정" 칸의 매핑만 명시:

| 감정 상태 | 적합 Aura |
|---|---|
| 결의·집중 | yang-hard |
| 평정·고요 | yin-soft |
| 자비·연민 | buddha-pure |
| 분노·집착 | demon-qi |
| 슬픔·상실 | yin-soft (도가) 혹은 demon-qi (마교 — 분기) |

→ Phase 6에서 사용자(작가)가 무공 인스턴스의 `extras.suitable_emotion`을 채울 때 이 표 참조.


---

## §6. 주화입마(走火入魔) Trigger — Phase 6 spec, Phase 7+ 발화

### 6.1 정책 진술

기연 역방향 5조건의 *최고 등급*(growth-mechanic §5.3 — "주화입마 ×7.0 페널티"). Aura·Alignment·H의 *불일치 조합*이 역방향 조건①·②·③를 자동 가산.

### 6.2 자동 가산 trigger (L2)

| 조합 | 역방향 가산 | 근거 |
|---|---|---|
| H 높음 + demon-qi 학습 시도 | reject (학습 자체 차단) | §5.2 |
| H 낮음 + buddha-pure 학습 시도 | reject | §5.2 |
| Orthodox 무공 + demon-qi aura 동시 보유 | +1 역방향 | §4.3 비고① |
| E 높음 + demon-qi 연마 | +1 역방향 | §5.1 |
| Heterodox 무공 + Master-Disciple 정파 사부 | +1 역방향 | relationship-mechanic §2.3 결합 |

### 6.3 Phase 6 산출물

`Skill.extras.zhuhwa_trigger`(선택)에 텍스트 정책 명시:
```yaml
extras:
  zhuhwa_trigger:
    auto_reverse_when: "alignment=orthodox AND aura=demon-qi"
    note: "정파 무공이 마기로 변질 시 자동 역방향 +1"
```

Phase 7+ 구현은 기연 판정 함수에 통합.

---

## §7. Phase 7+ 텍스트만 (외래키 미활성)

§2 표에 라벨된 3축. Skill 인스턴스의 `extras`에 텍스트로만 보존, 외래키 검증 없음.

| extras key | 텍스트 형식 | Phase 7+ 활성 시 변환 |
|---|---|---|
| `required_item` | `"비급-매화보전"` (자유 텍스트) | `Vec<ItemId>` 외래키 |
| `bloodline_requirement` | `"hwasan-lineage"` | `Person.extras.bloodline` 결합 |
| `lineage_chain` | `"자양진인 → … → 임서운 → player"` | Relationship × Skill 도메인 |
| `suitable_emotion` | `"평정"` (§5.3 표 값) | 감정 상태 enum |
| `place_synergy` | `[{place, bonus}]` 배열 | growth-mechanic §3 활성 |

**원칙**: Phase 5a `era_id` 텍스트 → Phase 5b 외래키 패턴 그대로. 텍스트 시점에 *값의 자유도*를 유지하다가, 외래키 활성 시 검증 게이트 통과한 값만 살아남음.

---

## §8. 검증 게이트 — 체크포인트 통과 조건

### 8.1 체크포인트 1 — 남궁가 검법 (표준 케이스)

| 검증 항목 | 통과 조건 |
|---|---|
| 외래키 결손 | 0건 (school·founder·current_masters 모두 존재) |
| school 정합 | `group-namgung`이 sect 계열인지 |
| Aura × Alignment | `yang-hard × orthodox` — §4.3 매트릭스의 *작동 칸* |
| HEXACO 매트릭스 | C/O 슬롯 채움, H 슬롯은 비워도 됨 (yang-hard라 거부 trigger 없음) |
| Watch 시너지 | `time_aura_peak: [Midday]` (§4.2 yang-hard 작동 칸) |
| prerequisites | 빈 배열 허용 (창시자 무공) |
| MCP 도구 | `list_skills` / `get_skill` / `search_skills` 작동 |

### 8.2 체크포인트 2 — 매화검법 (시간성·lost 케이스)

| 검증 항목 | 통과 조건 |
|---|---|
| `status: lost` | enum 정합 |
| `related_event: event-hwasan-fall` | 양방향 — Event 쪽에서도 역참조 가능 |
| `prerequisites: [화산기본내공]` | DAG 무순환, stub 인스턴스 동반 (5c.1 npc-11 패턴) |
| Aura × Alignment | `yin-soft × orthodox` — §4.3 작동 칸 |
| Watch 시너지 | `time_aura_peak: [Dawn, Night]` (§4.2 yin-soft) |
| `current_masters` | `[npc-im-seoun, player]` — 멸문 후 잔존 |
| Lore RAG 시연 | `body` 마크다운에 `search_lore` 인용 1건 이상 |
| 주화입마 trigger | 정파 음유 → demon-qi 변질 trigger 텍스트 명시 |

---

## §9. 결정 필요 — 사용자 확인 항목

작성 중 새로 surface된 결정 후보. Phase 6 진입 *전* 잠가야 함:

1. **`yin-soft × Heterodox` 비고③** (§4.3) — 현재 "가능"으로 둠. 칠국춘추에 실제 인스턴스 시드가 있는지 (예: 백사문 같은 음유 사파 그룹). 없다면 *가능하나 미사용*으로 표시.
2. **H 임계값 ±0.3** (§5.2) — wuxia-core 다른 docs에서 HEXACO 수치 임계가 어떻게 잡혔는지 확인 필요. `npc-temperament-values-detail.md`와 정합?
3. **Master-Disciple 결합 가산** (§6.2 마지막 행) — relationship-mechanic §2.3은 *보너스*만 명시. 사부-제자 *불일치*가 페널티가 되는 룰은 본 문서에서 처음 도입. 사용자 의도 확인 필요.
4. **`extras.suitable_emotion`을 슬롯으로 둘지** (§5.3) — 기연 5조건 ④"감정"은 *행위 시* 평가되는 것이라, Skill 정의 시 미리 적어두는 게 맞는지. 텍스트 hint로만 둘지.

---

## §10. 변경 이력

| 버전 | 날짜 | 변경 |
|---|---|---|
| v1.0 (초안) | 2026-05-04 | 최초 작성. 9축 매트릭스, Aura 4종 도입, HEXACO L2 매트릭스, 주화입마 trigger spec. |
