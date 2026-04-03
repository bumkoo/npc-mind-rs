# 언어 설정 가이드 (Locale Guide)

NPC Mind Engine의 LLM 연기 프롬프트는 로케일 시스템을 통해 다국어를 지원합니다.
이 문서는 **라이브러리 개발자**와 **라이브러리 사용자** 각각의 관점에서
언어 설정을 다루는 방법을 설명합니다.

---

## 아키텍처 개요

```
Domain (ActingGuide)         Presentation (Formatter)
──────────────────           ────────────────────────
Tone::SuppressedCold      → "억누른 분노가 느껴지는 차가운 어조" (ko)
                          → "A cold tone with suppressed anger" (en)
RelationshipLevel::VeryHigh → "매우 깊은" (ko)
                          → "Very close" (en)
```

- **Domain**: "무엇을" 결정 (5단계 분류, 어떤 감정이 유의미한가 등)
- **Presentation**: "어떻게 표현할지" 담당 (한국어/영어/커스텀 텍스트)

이 분리 덕분에 도메인 로직 변경 없이 텍스트만 교체할 수 있습니다.

---

## 서비스 선택

| 서비스 | 반환 타입 | 용도 |
|--------|-----------|------|
| `MindService` | `AppraiseResult` (ActingGuide 포함) | 도메인 데이터만 필요할 때 |
| `FormattedMindService` | `AppraiseResponse` (prompt: String 포함) | 포맷팅된 프롬프트가 필요할 때 |

```rust
// 도메인 데이터만 사용 — 자체 템플릿 엔진으로 렌더링하는 경우
let mut service = MindService::new(repo);
let result = service.appraise(req, || {}, || vec![])?;
let guide: &ActingGuide = &result.guide;
// guide.emotion.dominant, guide.directive.tone 등 직접 접근

// 포맷팅된 프롬프트 사용 — LLM에 바로 전달하는 경우
let mut service = FormattedMindService::new(repo, "ko")?;
let response = service.appraise(req, || {}, || vec![])?;
let prompt: &str = &response.prompt;  // "[NPC: 무백]\n[성격]\n..."
```

---

## 라이브러리 사용자 가이드

### 1. 빌트인 언어 사용 (가장 간단)

```rust
use npc_mind::FormattedMindService;

// 한국어
let mut service = FormattedMindService::new(repo, "ko")?;

// 영어
let mut service = FormattedMindService::new(repo, "en")?;
```

현재 빌트인 지원 언어: `"ko"` (한국어), `"en"` (영어)

### 2. 빌트인 텍스트 부분 수정

빌트인 로케일을 기반으로 일부 텍스트만 교체합니다.
TOML 파일에 변경하고 싶은 키만 작성하면 됩니다.

```rust
let overrides = r#"
[emotion]
Anger = "살기"
Joy = "환희"
Distress = "비통"

[tone]
RoughAggressive = "광폭한 어조로 내공이 실린 목소리"

[template]
section_npc = "[인물: {name}]"
"#;

let mut service = FormattedMindService::with_overrides(repo, "ko", overrides)?;
// → "ko" 빌트인의 나머지 키는 유지, 위 키만 교체
```

**활용 예시:**
- 무협 세계관: 감정 명칭을 무협풍으로 변경
- 판타지 세계관: 관계 수준 라벨을 세계관에 맞게 조정
- 프롬프트 구조 변경: 섹션 헤더나 포맷 패턴 수정

### 3. 완전한 신규 언어 추가

빌트인에 없는 언어(예: 일본어)를 지원하려면
전체 키가 포함된 TOML 파일을 작성합니다.

```rust
let ja_toml = std::fs::read_to_string("locales/ja.toml")?;
let mut service = FormattedMindService::with_custom_locale(repo, &ja_toml)?;
```

TOML 파일 구조는 아래 [TOML 구조 레퍼런스](#toml-구조-레퍼런스)를 참고하세요.

### 4. GuideFormatter 트레이트 직접 구현

TOML 기반 포맷팅이 아닌 완전히 다른 출력 형식이 필요한 경우:

```rust
use npc_mind::{GuideFormatter, FormattedMindService};
use npc_mind::domain::guide::ActingGuide;

struct MyGameFormatter;

impl GuideFormatter for MyGameFormatter {
    fn format_prompt(&self, guide: &ActingGuide) -> String {
        // ActingGuide의 구조화된 데이터를 자유롭게 가공
        let emotion = guide.emotion.dominant.as_ref()
            .map(|e| format!("{:?}", e.emotion_type))
            .unwrap_or("Calm".into());
        let tone = format!("{:?}", guide.directive.tone);

        format!(
            "You are {}. You feel {}. Speak with a {} tone.",
            guide.npc_name, emotion, tone
        )
    }

    fn format_json(&self, guide: &ActingGuide) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(guide)
    }
}

let mut service = FormattedMindService::with_formatter(repo, MyGameFormatter);
```

### 5. 도메인 데이터에서 직접 포맷팅

`MindService`로 도메인 결과를 받은 뒤, 나중에 포맷팅할 수도 있습니다.

```rust
use npc_mind::{MindService, LocaleFormatter};

let mut service = MindService::new(repo);
let result = service.appraise(req, || {}, || vec![])?;

// ActingGuide 데이터 직접 활용
println!("NPC: {}", result.guide.npc_name);
println!("Dominant: {:?}", result.guide.emotion.dominant);
println!("Tone: {:?}", result.guide.directive.tone);

// 필요 시 나중에 포맷팅
let formatter = LocaleFormatter::from_toml(my_toml)?;
let response = result.format(&formatter);
println!("{}", response.prompt);
```

---

## 라이브러리 개발자 가이드

### 빌트인 언어 추가

예: 중국어(`zh`) 추가

**1단계: TOML 파일 작성**

`locales/zh.toml`을 생성합니다. `locales/ko.toml`과 동일한 구조로 작성합니다.

```toml
[meta]
language = "zh"
name = "中文"

[intensity]
extreme = "极强"
strong = "强烈"
noticeable = "明显"
weak = "微弱"
faint = "极微"

[emotion]
Joy = "喜悦"
Anger = "愤怒"
# ... 22개 감정 유형 전체
```

**2단계: 레지스트리 등록**

`src/presentation/mod.rs`:

```rust
const BUILTIN_KO: &str = include_str!("../../locales/ko.toml");
const BUILTIN_EN: &str = include_str!("../../locales/en.toml");
const BUILTIN_ZH: &str = include_str!("../../locales/zh.toml");  // 추가

pub fn builtin_toml(lang: &str) -> Option<&'static str> {
    match lang {
        "ko" => Some(BUILTIN_KO),
        "en" => Some(BUILTIN_EN),
        "zh" => Some(BUILTIN_ZH),  // 추가
        _ => None,
    }
}
```

이후 `FormattedMindService::new(repo, "zh")`로 사용 가능합니다.

### 새로운 enum variant 추가 시

도메인에 새 감정이나 어조가 추가되면:

1. `src/domain/` 에서 enum variant 추가
2. `src/presentation/locale.rs`의 `impl_variant_name!` 매크로에 variant 등록
3. **모든 빌트인 TOML 파일**에 해당 키 추가 (`locales/ko.toml`, `locales/en.toml` 등)

```rust
// locale.rs — 새 Tone variant 등록
impl_variant_name!(Tone, {
    SuppressedCold, RoughAggressive, /* ... */
    NewToneVariant,  // 추가
});
```

```toml
# ko.toml
[tone]
NewToneVariant = "새로운 어조 설명"

# en.toml
[tone]
NewToneVariant = "Description of new tone"
```

---

## TOML 구조 레퍼런스

커스텀 TOML 파일 작성 시 아래 섹션이 모두 필요합니다.
(`with_overrides` 사용 시에는 변경할 섹션만 작성)

| 섹션 | 키 수 | 설명 |
|------|-------|------|
| `[meta]` | 2 | `language` (코드), `name` (표시명) |
| `[intensity]` | 5 | 감정 강도 라벨 (extreme ~ faint) |
| `[mood]` | 5 | 전체 분위기 라벨 (very_positive ~ very_negative) |
| `[emotion]` | 22 | OCC 감정 유형명 (Joy, Distress, ..., Love, Hate) |
| `[tone]` | 18 | 어조 설명 (SuppressedCold, RoughAggressive, ...) |
| `[attitude]` | 7 | 태도 설명 (HostileAggressive, ...) |
| `[behavioral_tendency]` | 8 | 행동 경향 설명 (ImmediateConfrontation, ...) |
| `[restriction]` | 5 | 금지 사항 설명 (NoHumorOrLightTone, ...) |
| `[personality_trait]` | 12 | 성격 특성 설명 (HonestAndModest, ...) |
| `[speech_style]` | 12 | 말투 스타일 설명 (FrankAndUnadorned, ...) |
| `[closeness_level]` | 5 | 친밀도 수준 (VeryHigh ~ VeryLow) |
| `[trust_level]` | 5 | 신뢰도 수준 (VeryHigh ~ VeryLow) |
| `[power_level]` | 5 | 상하 관계 수준 (VeryHigh ~ VeryLow) |
| `[fallback]` | 2 | 특성/말투 없을 때 폴백 문구 |
| `[template]` | 20 | 프롬프트 섹션 헤더 및 포맷 패턴 (`dominant_label` 포함) |

### template 플레이스홀더

template 섹션의 값에는 `{name}` 형태의 플레이스홀더를 사용합니다:

```toml
[template]
section_npc = "[NPC: {name}]"           # {name} → NPC 이름
overall_mood = "Overall mood: {mood}"   # {mood} → 분위기 라벨
directive_tone = "Tone: {tone}"         # {tone} → 어조 라벨
directive_attitude = "Attitude: {attitude}"
directive_behavior = "Behavior: {behavior}"
restriction_item = "- {restriction}"    # {restriction} → 금지 사항 라벨
dominant_label = "dominant"            # 지배 감정 라벨 (ko: "지배")
relationship_closeness = "Closeness: {level}"
relationship_trust = "Trust: {level}"
relationship_power = "Power dynamic: {level}"
```

---

## 전체 API 요약

```
FormattedMindService
├── ::new(repo, "ko")                          빌트인 언어 (기본 엔진)
├── ::with_overrides(repo, "ko", override_toml) 빌트인 + 부분 교체
├── ::with_custom_locale(repo, full_toml)       완전 커스텀 TOML
└── ::with_formatter(repo, impl GuideFormatter) 트레이트 직접 구현

MindService
├── ::new(repo)                                 기본 엔진 (포맷팅 없음)
├── ::with_engines(repo, appraiser, stimulus)   커스텀 엔진 주입
└── result.format(&formatter)                   필요 시 나중에 포맷팅
```

참고: `MindService`와 `FormattedMindService` 모두 감정 평가 엔진(`Appraiser`)과
자극 처리 엔진(`StimulusProcessor`)을 제네릭으로 받으며, 기본값으로
`AppraisalEngine`과 `StimulusEngine`이 사용됩니다.
