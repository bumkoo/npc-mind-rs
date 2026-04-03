# Integration Guide

`npc-mind`를 외부 프로젝트에 통합하는 단계별 가이드입니다.

---

## 목차

1. [의존성 추가](#1-의존성-추가)
2. [Repository 구현](#2-repository-구현)
3. [NPC 생성](#3-npc-생성)
4. [관계 생성](#4-관계-생성)
5. [서비스 생성](#5-서비스-생성)
6. [상황 평가 (Appraise)](#6-상황-평가-appraise)
7. [대사 자극 적용 (Stimulus)](#7-대사-자극-적용-stimulus)
8. [Scene & Beat 설정](#8-scene--beat-설정)
9. [Scene 종료 및 관계 갱신](#9-scene-종료-및-관계-갱신)
10. [가이드 재생성](#10-가이드-재생성)
11. [전체 흐름 다이어그램](#11-전체-흐름-다이어그램)

---

## 1. 의존성 추가

```toml
[dependencies]
npc-mind = { git = "https://github.com/bumkoo/npc-mind-rs.git" }
```

대사 → PAD 자동 분석이 필요한 경우:

```toml
npc-mind = { git = "https://github.com/bumkoo/npc-mind-rs.git", features = ["embed"] }
```

## 2. Repository 설정

라이브러리에 내장된 `InMemoryRepository`를 사용합니다. Mind Studio에서 저장한 scenario.json을 로드하거나, 프로그래밍 방식으로 데이터를 등록할 수 있습니다.

### 방법 A: Mind Studio JSON 로드 (권장)

Mind Studio에서 NPC, 관계, 오브젝트, Scene을 만들어 저장한 JSON을 한 줄로 로드합니다:

```rust
use npc_mind::InMemoryRepository;

let repo = InMemoryRepository::from_file("data/my_scenario/scenario.json")?;
// NPC, 관계, 오브젝트, Scene 모두 자동 로드
```

### 방법 B: 프로그래밍 방식

```rust
use npc_mind::InMemoryRepository;

let mut repo = InMemoryRepository::new();
repo.add_npc(npc);
repo.add_relationship(rel);
repo.add_object("sword", "명검 천하제일검");
```

### 방법 C: 커스텀 저장소

DB 연동 등 특수한 경우에는 3개의 포트 트레이트를 직접 구현합니다:

```rust
use npc_mind::{NpcWorld, EmotionStore, SceneStore};

impl NpcWorld for MyGameDB { /* ... */ }
impl EmotionStore for MyGameDB { /* ... */ }
impl SceneStore for MyGameDB { /* ... */ }
// → MindRepository 자동 파생
```

## 3. NPC 생성

> Mind Studio JSON 로드(방법 A)를 사용하면 이 단계가 불필요합니다.

`NpcBuilder`로 HEXACO 24 패싯 성격을 정의합니다.

```rust
use npc_mind::domain::personality::{NpcBuilder, Score};

let mu_baek = NpcBuilder::new("mu_baek", "무백")
    .description("야심 없이 의리를 지키는 정직한 검객")
    // Honesty-Humility: 높은 정직성
    .honesty_humility(|h| {
        h.sincerity = Score::new(0.7, "sincerity").unwrap();
        h.fairness = Score::new(0.8, "fairness").unwrap();
        h.greed_avoidance = Score::new(0.6, "greed_avoidance").unwrap();
        h.modesty = Score::new(0.5, "modesty").unwrap();
    })
    // Emotionality: 낮은 감정성 (담대)
    .emotionality(|e| {
        e.fearfulness = Score::new(-0.3, "fearfulness").unwrap();
        e.anxiety = Score::new(0.1, "anxiety").unwrap();
        e.dependence = Score::new(-0.2, "dependence").unwrap();
        e.sentimentality = Score::new(0.4, "sentimentality").unwrap();
    })
    // Extraversion: 중간
    .extraversion(|x| {
        x.social_self_esteem = Score::new(0.3, "social_self_esteem").unwrap();
        x.social_boldness = Score::new(0.2, "social_boldness").unwrap();
        x.sociability = Score::new(-0.1, "sociability").unwrap();
        x.liveliness = Score::new(0.0, "liveliness").unwrap();
    })
    // Agreeableness: 높은 관용
    .agreeableness(|a| {
        a.forgiveness = Score::new(0.4, "forgiveness").unwrap();
        a.gentleness = Score::new(0.3, "gentleness").unwrap();
        a.flexibility = Score::new(0.2, "flexibility").unwrap();
        a.patience = Score::new(0.6, "patience").unwrap();
    })
    // Conscientiousness: 높은 성실성
    .conscientiousness(|c| {
        c.organization = Score::new(0.5, "organization").unwrap();
        c.diligence = Score::new(0.7, "diligence").unwrap();
        c.perfectionism = Score::new(0.3, "perfectionism").unwrap();
        c.prudence = Score::new(0.6, "prudence").unwrap();
    })
    // Openness: 중간
    .openness(|o| {
        o.aesthetic_appreciation = Score::new(0.2, "aesthetic_appreciation").unwrap();
        o.inquisitiveness = Score::new(0.1, "inquisitiveness").unwrap();
        o.creativity = Score::new(0.0, "creativity").unwrap();
        o.unconventionality = Score::new(-0.1, "unconventionality").unwrap();
    })
    .build();

// Repository에 등록
repo.add_npc(mu_baek);
```

> **Tip:** `Score::clamped(value)`를 사용하면 범위 초과 시 자동 클램프됩니다. `Score::neutral()`은 0.0입니다.

## 4. 관계 생성

> Mind Studio JSON 로드(방법 A)를 사용하면 이 단계가 불필요합니다.

```rust
use npc_mind::domain::relationship::{RelationshipBuilder, Score};

let rel = RelationshipBuilder::new("mu_baek", "player")
    .closeness(Score::clamped(0.6))   // 친밀
    .trust(Score::clamped(0.8))       // 높은 신뢰
    .power(Score::clamped(0.0))       // 대등
    .build();

repo.add_relationship(rel);
```

**축 설명:**

| 축 | -1.0 | 0.0 | 1.0 |
|----|------|-----|-----|
| closeness | 적대 | 무관 | 친밀 |
| trust | 불신 | 중립 | 신뢰 |
| power | 하위 | 대등 | 상위 |

## 5. 서비스 생성

```rust
use npc_mind::{InMemoryRepository, FormattedMindService};

// Mind Studio JSON에서 바로 서비스 생성 (가장 간단한 방법)
let repo = InMemoryRepository::from_file("data/my_scenario/scenario.json")?;
let mut service = FormattedMindService::new(repo, "ko")?;

// 또는 프로그래밍 방식으로 구성한 repo 사용
let mut service = FormattedMindService::new(repo, "ko")?;

// 무협 세계관 용어 오버라이드
let overrides = r#"
[emotion]
Anger = "살기(殺氣)"
Gratitude = "은혜"

[tone]
RoughAggressive = "내공이 실린 거친 목소리로"
SuppressedCold = "차갑게 내공을 억누르며"
"#;
let mut service = FormattedMindService::with_overrides(repo, "ko", overrides)?;
```

## 6. 상황 평가 (Appraise)

Scene 진입 시 또는 새로운 상황 발생 시 1회 호출합니다.

```rust
use npc_mind::application::dto::*;

let response = service.appraise(AppraiseRequest {
    npc_id: "mu_baek".into(),
    partner_id: "player".into(),
    situation: SituationInput {
        description: "의형제가 적에게 아군 위치를 밀고했다".into(),
        event: Some(EventInput {
            description: "배신으로 인한 피해".into(),
            desirability_for_self: -0.8,
            other: None,
            prospect: None,
        }),
        action: Some(ActionInput {
            description: "밀고 행위".into(),
            agent_id: Some("jo_ryong".into()),
            praiseworthiness: -0.9,
        }),
        object: None,
    },
}, || {}, || vec![])?;

// LLM에 전달할 프롬프트
println!("{}", response.prompt);

// 감정 상태 확인
for emotion in &response.emotions {
    println!("{}: {:.2}", emotion.emotion_type, emotion.intensity);
}
```

### Focus 조합 패턴

| 패턴 | event | action | object | 생성되는 감정 |
|------|-------|--------|--------|--------------|
| 사건만 | O | - | - | Joy/Distress, Hope/Fear |
| 타인 행동 | - | O (agent_id: Some) | - | Admiration/Reproach → Gratitude/Anger |
| 자기 행동 | - | O (agent_id: None) | - | Pride/Shame → Gratification/Remorse |
| 대상 평가 | - | - | O | Love/Hate |
| 복합 상황 | O | O | O | 위 모두 + 복합 감정 |

## 7. 대사 자극 적용 (Stimulus)

대화 중 매 턴마다 호출합니다. PAD 벡터로 감정 강도를 조정합니다.

```rust
let stimulus_response = service.apply_stimulus(StimulusRequest {
    npc_id: "mu_baek".into(),
    partner_id: "player".into(),
    situation_description: Some("대화 중".into()),
    pleasure: 0.3,     // 긍정적 발화
    arousal: -0.2,      // 차분한 어조
    dominance: 0.1,     // 약간 우위
}, || {}, || vec![])?;

// Beat 전환 체크
if stimulus_response.beat_changed {
    println!("Beat 전환! 새 Focus: {:?}", stimulus_response.active_focus_id);
}

// 갱신된 프롬프트
println!("{}", stimulus_response.prompt);
```

**PAD 축 의미:**

| 축 | -1.0 | 0.0 | 1.0 |
|----|------|-----|-----|
| Pleasure | 불쾌한 발화 | 중립 | 유쾌한 발화 |
| Arousal | 차분/나른 | 중립 | 격앙/긴박 |
| Dominance | 복종적/약한 | 중립 | 지배적/강한 |

## 8. Scene & Beat 설정

Scene은 여러 Beat(감정 전환점)로 구성됩니다. 각 Beat는 Focus로 정의되며, 감정 조건에 따라 자동 전환됩니다.

```rust
let scene_response = service.start_scene(SceneRequest {
    npc_id: "mu_baek".into(),
    partner_id: "player".into(),
    description: "숲속 은신처에서의 대면".into(),
    focuses: vec![
        // Beat 1: 초기 Focus (trigger: None → Initial)
        SceneFocusInput {
            id: "confrontation".into(),
            description: "배신자와의 대면".into(),
            trigger: None,  // Initial — 즉시 적용
            event: Some(EventInput {
                description: "배신의 진상 확인".into(),
                desirability_for_self: -0.7,
                other: None,
                prospect: None,
            }),
            action: Some(ActionInput {
                description: "밀고 행위에 대한 추궁".into(),
                agent_id: Some("jo_ryong".into()),
                praiseworthiness: -0.8,
            }),
            object: None,
        },

        // Beat 2: Anger가 0.7 이상 OR Distress가 0.8 이상일 때 전환
        SceneFocusInput {
            id: "rage_peak".into(),
            description: "분노가 폭발하는 순간".into(),
            trigger: Some(vec![
                // OR 그룹 1: Anger > 0.7
                vec![ConditionInput {
                    emotion: "Anger".into(),
                    above: Some(0.7),
                    below: None,
                    absent: None,
                }],
                // OR 그룹 2: Distress > 0.8
                vec![ConditionInput {
                    emotion: "Distress".into(),
                    above: Some(0.8),
                    below: None,
                    absent: None,
                }],
            ]),
            event: Some(EventInput {
                description: "참았던 분노가 터짐".into(),
                desirability_for_self: -0.9,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        },

        // Beat 3: Anger가 소멸하고 Distress < 0.3일 때 (AND 조건)
        SceneFocusInput {
            id: "resignation".into(),
            description: "체념과 실망".into(),
            trigger: Some(vec![
                vec![
                    // AND: Anger가 없고
                    ConditionInput {
                        emotion: "Anger".into(),
                        absent: Some(true),
                        above: None,
                        below: None,
                    },
                    // AND: Distress < 0.3
                    ConditionInput {
                        emotion: "Distress".into(),
                        below: Some(0.3),
                        above: None,
                        absent: None,
                    },
                ],
            ]),
            event: Some(EventInput {
                description: "의형제에 대한 체념".into(),
                desirability_for_self: -0.3,
                other: None,
                prospect: None,
            }),
            action: None,
            object: None,
        },
    ],
}, || {}, || vec![])?;

println!("등록된 Focus: {}개", scene_response.focus_count);
println!("초기 활성 Focus: {:?}", scene_response.active_focus_id);
```

### Trigger 조건 구조

```
trigger: Option<Vec<Vec<ConditionInput>>>
         │      │    └── AND 그룹 (모든 조건 충족)
         │      └── OR 그룹 (하나라도 충족하면 전환)
         └── None = Initial (첫 번째 Beat)
```

**예시:**

```
// Anger > 0.7 AND Fear < 0.2
trigger: Some(vec![
    vec![
        ConditionInput { emotion: "Anger", above: Some(0.7), .. },
        ConditionInput { emotion: "Fear", below: Some(0.2), .. },
    ],
])

// Anger > 0.7 OR Fear > 0.8
trigger: Some(vec![
    vec![ConditionInput { emotion: "Anger", above: Some(0.7), .. }],
    vec![ConditionInput { emotion: "Fear", above: Some(0.8), .. }],
])
```

### Beat 전환 흐름

```
apply_stimulus() 호출
  → 1. PAD 자극으로 감정 강도 조정 (관성 적용)
  → 2. scene.check_trigger() — 대기 중 Focus의 조건 체크
  → 3. 조건 충족 시:
       a. 현재 Beat 관계 갱신 (after_beat)
       b. 새 Focus로 appraise (새 감정 생성)
       c. 이전 감정 + 새 감정 병합 (merge_from_beat)
  → 4. StimulusResult.beat_changed = true
```

## 9. Scene 종료 및 관계 갱신

### Beat 종료 (감정 유지)

```rust
service.after_beat(AfterDialogueRequest {
    npc_id: "mu_baek".into(),
    partner_id: "player".into(),
    praiseworthiness: Some(0.0),
    significance: Some(0.5),
})?;
```

### Scene(대화) 종료 (감정 초기화)

```rust
let after = service.after_dialogue(AfterDialogueRequest {
    npc_id: "mu_baek".into(),
    partner_id: "player".into(),
    praiseworthiness: Some(0.3),    // 상대 행동을 약간 긍정 평가
    significance: Some(0.7),         // 꽤 중요한 사건
})?;

println!("신뢰 변화: {:.2} → {:.2}", after.before.trust, after.after.trust);
println!("친밀 변화: {:.2} → {:.2}", after.before.closeness, after.after.closeness);
```

**significance 값 가이드:**

| 값 | 의미 | 관계 변동 |
|----|------|-----------|
| 0.0 | 일상 대화 | 미미 |
| 0.3 | 의미 있는 대화 | 소폭 |
| 0.5 | 중요한 사건 | 중간 |
| 0.7 | 큰 사건 | 대폭 |
| 1.0 | 인생을 바꾸는 사건 | 최대 (4배) |

## 10. 가이드 재생성

감정 상태는 유지하되 프롬프트만 다시 생성할 때 사용합니다.

```rust
let guide_response = service.generate_guide(GuideRequest {
    npc_id: "mu_baek".into(),
    partner_id: "player".into(),
    situation_description: Some("새로운 상황 설명".into()),
})?;

println!("{}", guide_response.prompt);
println!("{}", guide_response.json);  // JSON 포맷
```

## 11. 전체 흐름 다이어그램

```
┌─────────────────────────────────────────────────────────────┐
│  1. 준비                                                     │
│     NpcBuilder → Npc                                        │
│     RelationshipBuilder → Relationship                      │
│     Repository에 등록                                        │
│                                                             │
│  2. 서비스 생성                                               │
│     FormattedMindService::new(repo, "ko")                   │
│                                                             │
│  3. Scene 시작                                               │
│     start_scene(focuses) → 초기 Beat appraise               │
│                                                             │
│  4. 대화 루프                                                │
│     ┌──────────────────────────────────────┐                │
│     │  NPC 대사 → LLM(prompt)             │                │
│     │  상대 대사 → PAD 분석                │                │
│     │  apply_stimulus(PAD)                 │                │
│     │  ├── 감정 갱신                       │                │
│     │  ├── Beat 전환 체크                  │                │
│     │  └── 새 prompt 반환                  │                │
│     │  (반복)                              │                │
│     └──────────────────────────────────────┘                │
│                                                             │
│  5. Scene 종료                                               │
│     after_dialogue(significance) → 관계 갱신 + 감정 초기화   │
└─────────────────────────────────────────────────────────────┘
```

---

## 참고

- [API Reference](api-reference.md) — 전체 API 타입 레퍼런스
- [Locale Guide](../locale-guide.md) — TOML 로케일 커스터마이징
- [OCC Emotion Model](../emotion/occ-emotion-model.md) — 22 감정 타입 상세
- [HEXACO Research](../personality/hexaco-research.md) — HEXACO 성격 모델 배경
