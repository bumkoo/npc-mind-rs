# ⑦층 성찰·계획 시스템 (Tier 1~4)

> **버전**: v1.1 | **최종 수정**: 2026-02-28T01:30:00Z
> **역할**: NPC 성찰·계획 메카닉의 상세 설계
> **원본**: [npc-psychology-architecture.md](npc-psychology-architecture.md) §9에서 분리
> 📎 신조 변화 메카닉: [npc-psychology-architecture.md](npc-psychology-architecture.md) §10
> 📎 Generative Agents 참조: [generative-agents.md](../reference/generative-agents.md)
> 📎 OCC 감정 상세: [occ-emotion-detail.md](occ-emotion-detail.md)

---

## 1. 개요

> "돌아보고, 의미를 찾고, 다음을 계획한다"
> Stanford Generative Agents의 Reflection + Planning을 무협 맥락으로 확장

성찰은 ⑦층 메타 층으로, ①~⑥ 모든 층에 역방향 영향을 준다.
깊이에 따라 4단계(Tier)로 나뉘며, 깊을수록 비용이 높고 빈도는 낮지만 영향 범위가 넓다.

---

## 2. 4단계 성찰 계층

```
  Tier 1 ─ 순간 반응      🔧 코드      매 이벤트
  Tier 2 ─ 일상 성찰      🧠 LLM      하루 끝 (취침/명상)
  Tier 3 ─ 전환점 성찰    🧠 LLM      중대 사건 직후
  Tier 4 ─ 인생 성찰      🧠 LLM      장기 누적/고비

     깊이 ↑   비용 ↑   빈도 ↓   영향 범위 ↑
```

---

## 3. Tier별 변화 범위

```
  ┌────────┬───────────┬───────────┬───────────┬──────────┐
  │        │  Tier 1   │  Tier 2   │  Tier 3   │  Tier 4  │
  │        │ 순간반응  │ 일상성찰  │ 전환점    │ 인생성찰 │
  │        │ 🔧코드   │ 🧠LLM    │ 🧠LLM    │ 🧠LLM   │
  │        │ 매사건   │ 하루끝   │ 중대사건  │ 고비     │
  ├────────┼───────────┼───────────┼───────────┼──────────┤
  │①성격   │     -     │     -     │     -     │ ±5      │
  │(HEXACO)│           │           │           │ 최대2요인│
  ├────────┼───────────┼───────────┼───────────┼──────────┤
  │②3축강도│  ±5      │  ±10     │  ±20     │  ±30    │
  ├────────┼───────────┼───────────┼───────────┼──────────┤
  │②신조   │     -     │  뉘앙스   │  전환가능 │ 전환가능 │
  │(방향)  │           │  미세조정 │  (조건부) │ (큰변화) │
  ├────────┼───────────┼───────────┼───────────┼──────────┤
  │③5가치  │  ±5      │  ±10     │  ±20     │  ±20    │
  ├────────┼───────────┼───────────┼───────────┼──────────┤
  │④감정   │  발생     │  잔여정리 │  극단감정 │     -    │
  ├────────┼───────────┼───────────┼───────────┼──────────┤
  │⑤기분   │  미세변화 │  리셋     │  급변     │  리셋    │
  ├────────┼───────────┼───────────┼───────────┼──────────┤
  │⑥행동   │  범주결정 │  계획수립 │  목표변경 │ 인생방향 │
  │        │           │           │  진영재고 │ 전환     │
  ├────────┼───────────┼───────────┼───────────┼──────────┤
  │기억    │  기록     │  중요도   │  의미     │ 인생     │
  │        │           │  재평가   │  재해석   │ 재평가   │
  ├────────┼───────────┼───────────┼───────────┼──────────┤
  │관계    │  미세변화 │  재평가   │  급변     │ 재정립   │
  └────────┴───────────┴───────────┴───────────┴──────────┘
```

---

## 4. Tier 1 — 순간 반응

```
  트리거:  매 게임 이벤트 발생 시
  처리:    🔧 코드 (결정적, <1ms)
  빈도:    매 이벤트

  하는 일:
  · OCC 감정 평가 (5가치 × 사건 강도)
  · PAD 기분 미세 변화
  · Utility AI 행동 범주 결정
  · 기억 스트림에 관찰 기록
  · 3축 강도 ±5, 5가치 ±5 (미세)
  · 관계값 미세 변화

  하지 않는 일:
  · 성격 변화 (불가)
  · 신조 변화 (불가)
  · 목표 변경 (불가)
  · LLM 호출 (없음)
```

---

## 5. Tier 2 — 일상 성찰

```
  트리거:  게임 내 하루가 끝날 때 (취침, 명상, 수련 후 휴식)
  처리:    🧠 LLM
  빈도:    게임 내 하루 1회

  결정하는 것:
  1. 내일 계획 (Plan)
  2. 관계 재평가 (Relationship)
  3. 기억 중요도 재평가 (Memory Revaluation)
  4. 기분 리셋 (Mood Reset)
  5. 3축 강도 미세 조정 (±10)
  6. 5가치 미세 조정 (±10)
  7. 신조 뉘앙스 미세 변화 (텍스트 수정)
  8. 대안 후보 접촉 기록 (다른 신조 NPC와 접촉했다면)
```

### 5.1 Tier 2 LLM 프롬프트 구조 (설계)

```
  입력:
  ┌──────────────────────────────────────────────┐
  │ 오늘 있었던 일 (기억 스트림에서 상위 N개)      │
  │ 현재 감정 상태 (OCC 잔여)                      │
  │ 현재 기분 (PAD)                               │
  │ 현재 관계 목록 (변동이 있었던 것만)             │
  │ 현재 목표                                     │
  │ 현재 3축 + 신조                                │
  │ 현재 5가치                                    │
  └──────────────────────────────────────────────┘

  출력 (JSON):
  ┌──────────────────────────────────────────────┐
  │ tomorrow_plan: String                         │
  │ relationship_changes: [{npc, delta, reason}]  │
  │ memory_revaluations: [{id, new_importance}]   │
  │ mood_reset: {P, A, D}                         │
  │ axis_deltas: {trust, rightness, want}         │
  │ value_deltas: {loyalty, ..., ambition}        │
  │ creed_nuance: Option<String>                  │
  │ candidate_contact: Option<CreedCandidate>     │
  └──────────────────────────────────────────────┘
```

---

## 6. Tier 3 — 전환점 성찰

```
  트리거 (코드가 판단):
  · importance >= 9.0 인 사건 발생
  · PAD 극단값 (|pleasure| > 0.8)
  · 관계값 급변 (|delta| > 0.4)
  · 목표 실패/완료

  무협 RPG 예시:
  · 사부의 죽음, 비급 획득/상실
  · 배신 발각, 사랑 고백/이별
  · 무공 돌파/폐인

  결정하는 것 (Tier 2의 모든 것 + 추가):
  5. 목표 변경/추가/포기 (Goal Shift)
  6. 가치관 흔들림 (Value Conflict)
  7. 새로운 결심 (Resolution)
  8. 진영/소속 재고 (Faction Loyalty)
  9. 신조 전환 가능 (대안 후보가 있을 때만)
```

### 6.1 Tier 3 트리거 판단 (코드 설계)

```rust
fn should_trigger_tier3(event: &GameEvent, npc: &NpcPsyche) -> bool {
    let importance = event.importance;
    let pad_extreme = npc.mood.pleasure.abs() > 0.8
        || npc.mood.arousal.abs() > 0.8;
    let relationship_shift = event.relationship_deltas
        .iter()
        .any(|d| d.abs() > 0.4);
    let goal_resolved = event.resolves_goal.is_some();

    importance >= 9.0 || pad_extreme || relationship_shift || goal_resolved
}
```

---

## 7. Tier 4 — 인생 성찰

```
  트리거:
  · Tier 3급 사건이 짧은 기간에 3회 이상 누적
  · 극단적 가치 충돌 (옳음과 행동이 장기간 불일치)
  · 게임 내 중요 시점 (10년 경과, 은퇴 직전 등)

  결정하는 것 (Tier 3의 모든 것 + 추가):
  · 성격 변화 (±5, 최대 2요인)
  · 인생 방향 전환
  · 새 신념/좌우명 생성
  · 3축 신조 전환 (가장 큰 변화)
  · 형성기억에 이 전환 자체가 새 항목으로 추가됨
```

### 7.1 Tier 4 트리거 판단 (코드 설계)

```rust
fn should_trigger_tier4(npc: &NpcPsyche, history: &ReflectionHistory) -> bool {
    // 조건 1: 최근 Tier3가 짧은 기간에 3회 이상
    let recent_tier3_count = history.tier3_events
        .iter()
        .filter(|e| e.game_time > current_time - TIER4_WINDOW)
        .count();

    // 조건 2: 3축 강도와 실제 행동이 장기간 불일치
    let value_action_conflict = npc.detect_prolonged_conflict();

    // 조건 3: 게임 내 중요 시점
    let milestone = npc.age_milestone() || npc.career_milestone();

    recent_tier3_count >= 3 || value_action_conflict || milestone
}
```

### 7.2 성격 변화 제한 (Tier 4 전용)

```
  ┌──────────────────────────────────────────────────────┐
  │  1. Tier 1~3에서: 성격 변화 불가 (delta = 0)         │
  │  2. Tier 4에서: ±5 제한, 6요인 중 최대 2개만           │
  │  3. 나이 drift: 없음 (6요인 전부)                    │
  │  4. 어떤 경우에도 0~100 범위 clamp                   │
  └──────────────────────────────────────────────────────┘

  핵심 원칙: 사람은 나이로 변하지 않는다.
  겪은 일과 내린 선택으로만 변한다.
```

---

## 8. Tier 간 누적 관계

```
  Tier 1 (매 이벤트)
    │  감정/기분 누적
    ▼
  Tier 2 (하루 끝)
    │  일상 경험 정리 → 미세 변화 누적
    ▼
  Tier 3 (중대 사건)
    │  전환점 → 목표/가치관 흔들림
    │  짧은 기간에 3회 누적 시 ▼
    ▼
  Tier 4 (인생 고비)
    │  성격까지 변화 가능
    │  인생 방향 전환
    ▼
  ①~⑥ 전 층에 역방향 영향

  핵심: 작은 변화가 쌓여 큰 변화를 만든다.
  갑자기 변하는 사람은 없다. 갑자기 변한 것처럼 보일 뿐이다.
```

---

## 9. 코드/LLM 역할 분담 (성찰 영역)

```
  🔧 코드가 담당:
  · Tier 1 전체 (감정 계산, 기분 갱신, 행동 범주)
  · Tier 2~4 트리거 판단 (importance 합산, 극단값 감지)
  · 변화 범위 제한 (clamp) — Tier별 ± 범위 강제
  · 대안 후보 존재 여부 확인 (신조 전환 전제조건)

  🧠 LLM이 담당:
  · Tier 2~4 성찰 내용 생성
  · 계획 내용 작성
  · 관계 재평가의 "이유"와 "라벨"
  · 기억 중요도 재평가 (과거 사건의 의미 변화)
  · 목표 추가/변경/포기의 "내용"
  · 가치관 충돌의 구체적 내용
  · 신조 뉘앙스 변화, 전환 판단
  · 성격 변화의 "방향"과 "이유"
```

---

## 10. 안전장치 — LLM 출력 검증

LLM이 제안한 변화값을 코드가 Tier별 범위로 clamp한다.

```rust
fn apply_reflection_result(
    psyche: &mut NpcPsyche,
    result: &ReflectionResult,
    tier: ReflectionTier,
) {
    let max_axis_delta = match tier {
        Tier1 => 5.0,
        Tier2 => 10.0,
        Tier3 => 20.0,
        Tier4 => 30.0,
    };
    let max_value_delta = match tier {
        Tier1 => 5.0,
        Tier2 => 10.0,
        Tier3 => 20.0,
        Tier4 => 20.0,
    };
    let max_personality_delta = match tier {
        Tier4 => 5.0,
        _ => 0.0,  // Tier 1~3에서 성격 변화 불가
    };

    // 3축 강도 clamp
    for (intensity, delta) in axes {
        *intensity += delta.clamp(-max_axis_delta, max_axis_delta);
        *intensity = intensity.clamp(0.0, 100.0);
    }

    // 5가치 clamp
    for (value, delta) in values {
        *value += delta.clamp(-max_value_delta, max_value_delta);
        *value = value.clamp(0.0, 100.0);
    }

    // 성격 clamp (Tier4에서만, 최대 2요인)
    if max_personality_delta > 0.0 {
        let mut changed_count = 0;
        for (t, delta) in traits {
            if delta.abs() > 0.1 && changed_count < 2 {
                *t += delta.clamp(-max_personality_delta, max_personality_delta);
                *t = t.clamp(0.0, 100.0);
                changed_count += 1;
            }
        }
    }
}
```

---

## 변경 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| v1.1 | 2026-02-28T01:30:00Z | 전체 측정 스케일 0.0~1.0 → 0~100 변환. Tier별 변화 범위표, 안전장치 코드, 성격 변화 제한 규칙 일괄 적용. PAD 임계값(-1.0~+1.0)은 유지. |
| v1.0 | 2026-02-28T00:00:00Z | 최초 작성. npc-psychology-architecture.md v2.1의 §9(성찰·계획) 분리. Tier 1~4 상세 설계, Tier별 변화 범위표, 트리거 판단 코드, LLM 프롬프트 구조, 안전장치 코드 포함. Tier 1 상세(원본에 없던 내용) 및 Tier 2 프롬프트 구조, Tier 간 누적 관계도 추가. |
