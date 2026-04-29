# 무협 RPG 개발 계획 v2.0

> **원칙**: 동작하는 가장 작은 것부터. 테스트 통과 → 다음 단계.  
> **각 스텝**: 코드 구조 리뷰 → 구현 → 테스트 → 다음  
> **기준**: 모든 스텝은 `cargo test` 통과 상태를 유지  
> **아키텍처**: Hexagonal Architecture + DDD, Pure Logic Lib + Bevy Plugin + Hybrid Component  
> **관련 문서**: domain-analysis.md, architecture-decision.md, growth-mechanic-decisions-v1.md  

---

## Phase 0: 프로젝트 뼈대 ✅ 완료

### Iteration 0.1 — Cargo Workspace 구성 ✅
> 두 개의 crate를 가진 워크스페이스를 만든다 → v1.0에서 6개 crate로 확장

```
wuxia-rpg/                         [v1.0 현재 구조]
├── Cargo.toml              (workspace)
├── crates/
│   ├── wuxia-core/         (순수 Rust, 외부 의존 없음)  ✅ 활성
│   ├── wuxia-data/         (데이터 로딩, toml/json)    ✅ 활성
│   ├── wuxia-llm/          (LLM 어댑터)              📋 Phase 4
│   ├── wuxia-memory/       (벡터DB 어댑터)           📋 Phase 4
│   ├── wuxia-game/         (Bevy Plugin lib + 개발 main) 📋 Phase 5
│   └── wuxia-app/          (최종 조립 바이너리)        📋 Phase 5
```

- [x] workspace Cargo.toml 작성
- [x] wuxia-core: 순수 도메인 lib crate (serde만)
- [x] wuxia-data: 데이터 로딩 crate (cfg: toml/json)
- [x] wuxia-llm: LLM 어댑터 빈대 (feature: live-llm)
- [x] wuxia-memory: 벡터DB 어댑터 빈대 (feature: live-db)
- [x] wuxia-game: Bevy lib + 개발용 main
- [x] wuxia-app: 최종 조립 placeholder
- [x] wuxia-game → wuxia-core 의존성 추가
- [x] wuxia-core에 Bevy 의존성이 **없음**을 확인
- **테스트**: `cargo build --workspace` 성공 ✅

---

## Phase 1: 기반 도메인 `[wuxia-core]` ✅ 완료

> Phase 1~4는 전부 wuxia-core 안에서 진행한다.
> Bevy를 모르는 순수 Rust. `cargo test -p wuxia-core`로 검증.

### Iteration 1.1 — 공유 타입 (Shared Kernel) ✅
> 모든 도메인이 쓰는 기본 벽돌

- [x] `shared/` 모듈 생성
- [x] CharacterId ID 타입 (generic TypedId 패턴)
- [x] GameTime (연/월/일) + 날짜 연산 (360일/년, 30일/월)
- [x] DomainEvent enum
- [x] DomainError 타입
- **테스트**: ID 생성/비교, GameTime 연산 ✅

### Iteration 1.2 — 캐릭터 도메인 (최소) ✅
> 세상에서 가장 작은 캐릭터: 이름과 나이만 있다

- [x] `character/` 모듈 생성
- [x] Character struct: id, name, courtesy_name, age, gender, role
- [x] CharacterRole enum (Player/Npc/Companion)
- [x] `age_one_year(&mut self) -> Vec<CharacterEvent>` 메서드
- [x] LifeStage: 청년(~32)/장년(33~54)/중년(55~68)/노년(69~) — [v0.9.1 연령 조정]
- **테스트**: 캐릭터 생성, 나이 먹기, 생애 단계 전환 확인 ✅

### Iteration 1.3 — 시간 도메인 (최소) ✅
> 하루가 지나고, 1년이 지난다

- [x] `time/` 모듈 생성
- [x] GameClock struct: 현재 GameTime, tick()
- [x] tick() → TimeEvent::DayPassed 이벤트 반환
- [x] 계절 계산 (월 기반: 봄1~3/여름4~6/가을7~9/겨울10~12)
- [x] YearPassed 이벤트 발행 조건 (12월 30일)
- **테스트**: 360일 tick → YearPassed 1회, 계절 전환 4회 확인 ✅

### Iteration 1.4 — 시간 → 캐릭터 연결 (Application Service) ✅
> 시간이 흐르면 캐릭터가 나이를 먹는다

- [x] `application/` 모듈 생성
- [x] `TimeCharacterService`: YearPassed → 캐릭터 age +1 처리
- [x] 여러 캐릭터를 순회하며 age_one_year 호출
- [x] 발생한 DomainEvent 수집/반환
- **테스트**: 캐릭터 3명 생성 → 1년 경과 → 3명 모두 나이 +1 ✅

**Phase 1 완료 시점: 80 tests 통과**

---

## Phase 1-R: 시간대 리팩터링 `[wuxia-core]` — [v1.1 신설] ✅ 완료

> 1 tick = 1일에서 1 tick = 1시간대(경, Watch)로 변경한다.
> "묘시까지 수련한다" 같은 무협적 시간 표현을 지원한다.
> 기존 DayPassed 구독자는 영향 없도록 호환성을 유지한다.

### Iteration 1.3-R — 시간대(Watch/經) 시스템 ✅ 완료
> 하루가 6시간대로 나뉜다. 십이시진(十二時辰)을 2개씩 묶는다.

**Step 1: Watch enum + GameTime 확장 (shared/time.rs)**
- [x] Watch enum 신설: Dawn/Morning/Midday/Afternoon/Evening/Night
  - Dawn(黎明: 인묘시 03~07) → Morning(午前: 진사시 07~11) → Midday(正午: 오미시 11~15) → Afternoon(午後: 신유시 15~19) → Evening(黃昏: 술해시 19~23) → Night(深夜: 자축시 23~03)
- [x] Watch에 Translatable, Display, Serde, Copy, PartialEq, Eq, Ord 구현
- [x] Watch::next() → 다음 시간대 (Night → Dawn)
- [x] Watch::is_last() → Night인지 (하루의 마지막)
- [x] Watch::index() → 0~5 (순서값, total 계산용)
- [x] Watch::WATCHES_PER_DAY = 6 상수
- [x] GameTime에 watch: Watch 필드 추가
- [x] GameTime::new(year, month, day) → 기본값 Dawn (하위 호환)
- [x] GameTime::with_watch(year, month, day, watch) → 정밀 생성자
- [x] GameTime::watch() getter
- [x] GameTime::to_total_watches() / from_total_watches() (총 시간대 수)
- [x] GameTime::next_watch() → 다음 시간대 (Night→Dawn이면 날짜도 +1)
- [x] GameTime Ord 비교에 watch 포함
- [x] GameTime Display에 watch 포함: "Y1200-M03-D15 새벽(黎明) (Spring)"
- [x] 기존 next_day(), advance_days(), days_between() 등은 watch=Dawn 기준 유지
- [x] WATCHES_PER_DAY, WATCHES_PER_YEAR 상수 추가
- **테스트**: Watch 순환, GameTime 생성 (기존 호환), next_watch, total_watches 변환, Ord 비교 (시간대 포함) ✅

**Step 2: TimeEvent 확장 + GameClock 리팩터링 (time/)**
- [x] TimeEvent::WatchChanged { new_watch: Watch, date: GameTime } 추가
- [x] GameClock::tick() 변경: 1일 전진 → 1시간대 전진
  - WatchChanged 항상 발행
  - DayPassed는 Night→Dawn 전환 시에만 (6 tick마다)
  - SeasonChanged, YearPassed는 DayPassed 발생 시에만 체크
  - 이벤트 순서: WatchChanged → DayPassed → SeasonChanged → YearPassed
- [x] GameClock::tick_until(target: Watch) → 목표 시간대까지 전진
- [x] GameClock::tick_day() → 다음 날 Dawn까지 전진 (기존 tick() 호환)
- [x] GameClock::tick_days(n) 유지 (내부적으로 tick_day() × n)
- [x] GameClock::tick_watches(n) → n 시간대 전진
- **테스트**: tick 1회 → WatchChanged만. 6 tick → DayPassed 1회. 2160 tick → YearPassed 1회. tick_until(Morning) → Dawn에서 1 tick. tick_until(Dawn) → Dawn에서 6 tick (다음 날). tick_day() → 기존과 동일 동작. ✅

**Step 3: 기존 테스트 마이그레이션**
- [x] clock.rs 테스트: tick() → tick_day()로 변경 (360일 → 360 tick_day 또는 2160 tick)
- [x] shared/event.rs 테스트: GameTime 비교에 watch 영향 확인
- [x] application/time_character.rs: DayPassed 구독 로직 변경 없음 확인
- [x] 기존 테스트 전부 통과 확인
- **테스트**: 전체 `cargo test -p wuxia-core` 통과 ✅

**Step 4: i18n 로케일 추가**
- [x] ko.toml / en.toml에 watch 번역 키 추가
  - watch.dawn = "새벽(黎明)" / "Dawn"
  - watch.morning = "오전(午前)" / "Morning"
  - watch.midday = "한낮(正午)" / "Midday"
  - watch.afternoon = "오후(午後)" / "Afternoon"
  - watch.evening = "저녁(黃昏)" / "Evening"
  - watch.night = "심야(深夜)" / "Night"
- **테스트**: Watch Translatable 번역 확인 ✅

---

## Phase 2: 캐릭터의 내면 세계 `[wuxia-core]`

### Iteration 2.1 — 성장 도메인 (능력치) + DomainEvent 리팩터링 ✅ [v0.9 업데이트]
> 캐릭터에게 힘과 지혜가 생긴다

**Step 1: DomainEvent 리팩터링** (v0.9 추가 작업)
- [x] 단일 DomainEvent enum → 도메인별 enum + wrapper 패턴으로 변경
- [x] TimeEvent enum: DayPassed/SeasonChanged/YearPassed
- [x] CharacterEvent enum: Aged/LifeStageChanged
- [x] DomainEvent wrapper: Time(TimeEvent) / Character(CharacterEvent)
- [x] From trait 구현, 기존 코드 .into() 변환
- [x] 기존 테스트 전부 match 패턴 업데이트

**Step 2: GrowthProfile 구현**
- [x] `growth/` 모듈 생성
- [x] StatType enum (9개): InnerPower/Wisdom/Strategy/Vitality/Agility/Strength/Willpower/Endurance/Empathy
- [x] StatCategory enum (3개): Intellectual/Physical/Emotional
- [x] StatBlock struct: 9개 능력치 묶음 (Serde 호환, JSON/TOML 로딩 가능)
- [x] GrowthProfile struct (Aggregate Root): CharacterId 연결, 0~100 clamp
- [x] `new_default()` / `new_with_stats(StatBlock)` 생성자
- [x] `combat_power()`: 무력 + 경공 + 내공
- [x] `category_total(StatCategory)`: 범주별 합계
- **테스트**: 프로필 생성, 능력치 조회, 전투력 계산, clamp 검증, 무협 시나리오(청년무인 vs 노현자) ✅

**Iteration 2.1 완료 시점: 135 unit tests + 16 doc-tests = 151 tests 통과**

### Iteration 2.2 — 성장: 단련과 쇠퇴 ✅ [v1.0 용어 갱신]
> 청년은 빨리 크고, 노인은 체력이 줄어든다

**Step 1: LifeStage 연령 경계 조정**
- [x] 청년(~32) / 장년(33~54) / 중년(55~68) / 노년(69~) 으로 조정
- [x] 기존 테스트 전부 경계값 갱신 (character/model.rs, time_character.rs)

**Step 2: 성장 이벤트 & 변화 기록**
- [x] `growth/event.rs` 신규: StatChange(stat, delta:i32, source), ChangeSource, GrowthEvent
- [x] DomainEvent::Growth(GrowthEvent) variant 추가 + From impl
- [x] GrowthEvent: Trained { changes }, YearlyAgingApplied { life_stage, changes }

**Step 3: 단련/노화 규칙 (규칙-Aggregate 분리)**
- [x] `growth/training.rs` 신규 — 순수 함수만 (규칙서 역할)
- [x] LifeStage별 성장 계수 (청년: 1.5x, 장년: 1.0x, 중년: 0.7x, 노년: 0.3x)
- [x] `calculate_training(stat, intensity, life_stage) -> StatChange`
- [x] `calculate_yearly_aging(life_stage) -> Vec<StatChange>` 노화 테이블
  - 청년: 체력/경공/무력 +1
  - 장년: 내공/지혜/책략 +1
  - 중년: 체력-1, 경공-1, 내공+1, 지혜+2, 책략+1, 의지+1, 공감+1
  - 노년: 체력-2, 경공-2, 무력-1, 지혜+1, 책략+1, 인내-1

**Step 4: GrowthProfile 명령 메서드**
- [x] `train(&mut self, stat, intensity, life_stage) -> GrowthEvent`
- [x] `apply_yearly_aging(&mut self, life_stage) -> GrowthEvent`
- [x] `apply_stat_change()` 내부 헬퍼: saturating_add/sub + clamp(0~100)
- **테스트**: 같은 수련량 청년 > 노년 성장. 20년 경과 후 체력 감소. 0 이하/100 초과 방지. ✅

**Iteration 2.2 완료 시점: 164 unit tests + 25 doc-tests = 189 tests 통과**

### Iteration 2.3 — 성장: 무공 + 이름 리팩터링 ✅ [v1.0 대폭 확장, v1.1 완료]
> 독고구검을 배운다. 단련(鍛鍊)과 연마(練磨)가 구분된다.

**Step 1: 기존 함수 이름 변경** [v1.0 코드 변경 계획]
- [x] `calculate_training()` → `calculate_stat_training()`
- [x] `train()` → `train_stat()`
- [x] `ChangeSource::Training` → `ChangeSource::StatTraining`
- [x] 기존 테스트 전부 이름 갱신

**Step 2: MartialArt + 경지(Mastery) 체계** [v1.0 무공 경지]
- [x] MartialArt struct: id, 이름, art_type(내공/외공/병기/경공/암기), 기본위력
- [x] MartialArtType enum: InternalArt/ExternalArt/WeaponArt/LightArt/HiddenWeaponArt
- [x] MasteryLevel enum: 입문(0~24)/숙련(25~49)/통달(50~74)/화경(75~100)
- [x] `MasteryLevel::from_proficiency(u32) -> MasteryLevel`
- [x] 주 관련 능력치 매핑 (MartialArtType → Vec<StatType>) [부산물용]

**Step 3: MartialArtProficiency + 연마** [v1.0 연마 메카닉]
- [x] MartialArtProficiency struct: martial_art_id, proficiency(0~100), mastery_level
- [x] `learn_art(&mut self, art) -> Result<GrowthEvent>`
- [x] `calculate_art_training(art_type, intensity, life_stage) -> ArtTrainingResult` [부산물 포함]
- [x] `train_art(&mut self, art_id, intensity, life_stage) -> GrowthEvent` [v1.0 용어]
- [x] ChangeSource::ArtPractice variant 추가
- [x] 부산물 효과: 연마 시 주 관련 능력치 소폭 (본업 대비 약 25%)

**Step 4: 전투력 계산 확장**
- [x] `calculate_effective_power(base_power, proficiency, related_stat_avg) -> u32` 실전위력 계산
- [x] `art_effective_power()`, `best_art_power()` GrowthProfile 메서드
- **테스트**:
  - 이름 변경 후 기존 189 tests 통과 확인 ✅
  - 무공 습득, 연마 숙련도 증가, 경지 전환(입문→숙련) ✅
  - 연마 시 부산물 능력치 상승 확인 ✅
  - 단련 vs 연마 비교: 단련=능력치 집중, 연마=숙련도+소폭능력치 ✅
  - 실전위력 계산 ✅

**Iteration 2.3 완료 시점: 265 unit tests + 37 doc-tests = 302 tests 통과**

### Iteration 2.3A — 캐릭터: 피로/부상 시스템 ✅ 완료 [v1.0 신설, v1.2 완료]
> 수련하면 피곤해지고, 무리하면 다친다

**Step 1: 피로 시스템** ✅
- [x] Character struct에 `fatigue: u32` (0~100) 필드 추가 (#[serde(default)])
- [x] FatigueLevel enum: Fresh(0~20)/Mild(21~40)/Moderate(41~60)/Severe(61~80)/Exhausted(81~100)
- [x] `FatigueLevel::from_fatigue(u32) -> FatigueLevel`
- [x] `add_fatigue(&mut self, amount: u32) -> Vec<DomainEvent>`
- [x] `recover_fatigue(&mut self, amount: u32) -> Vec<DomainEvent>`
- [x] `daily_rest_recovery(&mut self) -> Vec<DomainEvent>` (수면 -5/밤)
- [x] CharacterEvent::FatigueChanged { character_id, new_fatigue, fatigue_level }

**Step 2: 부상 시스템** ✅
- [x] Injury struct: injury_type, severity, remaining_days (Value Object)
- [x] InjuryType enum: Bruise(타박)/Strain(근육손상)/Fracture(골절)/QiDeviation(주화입마)
- [x] InjurySeverity enum: Minor(3일)/Major(7일)/Critical(15일)
- [x] Character struct에 `injury: Option<Injury>` 필드 추가 (#[serde(default)])
- [x] `injure(&mut self, type, severity) -> Vec<DomainEvent>`
- [x] `heal_daily(&mut self) -> Vec<DomainEvent>` (부상 회복 1일 진행)
- [x] `treat_injury(&mut self, days) -> Vec<DomainEvent>` (치료 가속)
- [x] `can_train() -> bool` (탈진 OR 심각 부상 시 false)
- [x] CharacterEvent::Injured / InjuryHealed
- [x] i18n: injury_type(4) + injury_severity(3) ko/en 번역

**Step 3: 수련 피로 공식** ✅
- [x] `calculate_fatigue_from_training(intensity, fatigue_level) -> u32`
  - 피로 배율: Fresh(×1.0) / Mild(×1.2) / Moderate(×1.5) / Severe(×2.0)
  - 최소 1 (intensity > 0일 때)

**Step 4: 부상 확률 판정** ✅
- [x] `calculate_injury_chance(intensity, fatigue_level, over_max) -> f32`
  - 피로 기반: Fresh(0%)/Mild(5%)/Moderate(10%)/Severe(20%)
  - 고강도 보너스: intensity 7+(+10%), 9+(+15%)
  - 한계초과: over_max(+10%)
  - 상한: 80%
- **테스트**: 피로 누적/회복, 부상 발생/회복/치료, can_train 검증, i18n ✅

**Iteration 2.3A 완료 시점: 380 unit tests + 47 doc-tests = 427 tests 통과**

### Iteration 2.3B — 성장: 최대 강도 메카닉 ✅ 완료 [v1.0 신설, v1.2 완료]
> 능력치와 피로가 수련 강도의 상한선을 결정한다

- [x] `calculate_max_intensity(vitality, endurance, willpower, fatigue_level) -> Option<u32>` 순수 함수
  - 기본 = (체력 + 인내) / 20
  - 의지 보너스 = 의지 70 이상이면 +1
  - 피로 페널티: Fresh(0) / Mild(1) / Moderate(2) / Severe(3) / Exhausted → None
  - 결과 clamp 1..=10
- [x] `calculate_base_max_intensity(vitality, endurance, fatigue_level) -> Option<u32>` (over-limit 감지용)
  - full - base = 의지 보너스 부분 → over_limit 판정
- [x] 의지 보너스로 초과 수련 시 부상 확률 +10% 추가 규칙 (calculate_injury_chance에 반영)
- **테스트**: 17개 — 경계값, 피로 페널티, Exhausted=None, clamp, over-limit 감지 ✅

**Iteration 2.3B 완료 시점: 397 unit tests + 47 doc-tests = 444 tests 통과**

### Iteration 2.3C — 성장: 수련-피로 연결 (Application Service) ✅ 완료 [v1.0 신설, v1.2 완료]
> 수련하면 성장하지만 피로가 쌓인다

- [x] `TrainingService` (Application Service): 단련/연마 실행 시 피로/부상 처리 통합
  1. can_train() → 탈진/부상 확인
  2. calculate_max_intensity() → 최대 강도 제한
  3. over_limit 판정 → 의지 보너스 초과 감지
  4. 부상 페널티 → effective_intensity 계산
  5. train_stat()/train_art() → 성장 적용
  6. add_fatigue() → 피로 누적
  7. 부상 판정 → injury_chance > 0.10 && intensity ≥ 7
- [x] `TrainingError` enum: Exhausted / InjuryPreventsTraining / IntensityTooHigh / ArtNotLearned
- [x] `TrainingOutcome` struct: requested/effective_intensity, fatigue_gained, injury_occurred, over_limit
- [x] 부상 시 강도 차감: Bruise(-1), Strain(-2)
- **테스트**: 25 unit + 1 doc-test — 전체 수련 흐름, 에러 조건, 시나리오 ✅

**Iteration 2.3C 완료 시점: 422 unit tests + 50 doc-tests = 472 tests 통과**

**Phase 2.3 전체 완료 요약:**
```
2.3   MartialArt + 이름 리팩터링    ✅  302 tests
2.3A  피로 + 부상 시스템          ✅  427 tests (+125)
2.3B  최대 강도 메카닉            ✅  444 tests (+17)
2.3C  TrainingService 통합       ✅  472 tests (+28)
```

### Iteration 2.4 — 심리 도메인: 3축 가치관 + 5가치 ✅ 완료 — [v1.5 3 Aggregate 반영, v2.0 완료]
> 이 사람은 어떤 존재이며(敍), 상황에서 어떻게 판단하는가(判)
> 📎 설계 상세: [npc-psychology-architecture.md](npc-psychology-architecture.md) §3~§5
> 📎 구현 상세: [psychology-implementation-plan.md](psychology-implementation-plan.md)

- [x] `psychology/` 모듈 생성
- [x] `psychology/three_axis.rs` — ThreeAxisValues struct (**②층 敍**) [v1.5 신규]
  - 믿음(信) / 옳음(正) / 바람(願) 각 축:
    - intensity: f32 (0.0~1.0, 강도)
    - creed: String (신조 — "도의를 지킨다", "힘이 정의다" 등)
    - formation_memories: Vec<MemoryId> (형성기억)
    - creed_candidates: Vec<CreedCandidate> (대안후보)
  - **LLM 프롬프트 전용** — OCC 수식에 직접 투입되지 않음
- [x] CreedCandidate struct: text, source, exposure_count, resonance
- [x] `psychology/values.rs` — PracticalValues struct (**③층 判**) [v2.0 명칭: WuxiaValues → PracticalValues]
  - 충(忠) / 의(義) / 효(孝) / 복수(復) / 야망(野) (각 f32, 0.0~100.0)
  - **OCC 감정 평가 수식의 직접 입력** — 코드가 <1ms에 계산
- [x] CharacterId 연결 (양쪽 모두)
- [x] `alignment() -> Alignment` (5가치 기반 정파/사파/중립 계산)
- [x] `betrayal_potential(&self) -> f32` (야망 높고 충 낮으면 높음)
- [x] 프리셋 6인: 명경, 조고, 소연, 야율설화, 진야림, 남궁현 (3축+신조+5가치)
- [x] PsychologyEvent enum 생성 (shared/event.rs에 wrapper 추가)
- [x] **3축↔5가치 해석 관계 검증 테스트**
- **테스트**: 3축 생성, 5가치 생성, 프리셋 6인 비교, 불일치 케이스 검증, 배반 가능성 ✅

### Iteration 2.5 — 심리: 성격 (HexacoPersonality) ✅ 완료 — [v2.0 BigFive→HEXACO 6요소]
> 이 사람은 원래 어떤 사람인가 (性, ①층)

- [x] `psychology/personality.rs` — HexacoPersonality struct (**①층 性**) [v2.0: BigFive 5요소 → HEXACO 6요소]
  - Honesty-Humility / Emotionality / eXtraversion / Agreeableness / Conscientiousness / Openness (각 u32, 0~100)
- [x] CharacterId 연결
- [x] `hexaco_emotion_filter()` — HEXACO 성격 기반 감정 강도 필터링 (filter.rs)
- [x] 프리셋 6인: 명경, 조고, 소연, 야율설화, 진야림, 남궁현
- **테스트**: 프리셋 비교, 감정 필터 계수 차이, 경계값 검증 ✅

### Iteration 2.6 — 심리: 감정 (OCC 22종 + PAD) ✅ 완료 [v2.0 완료]
> 기쁘고, 분노하고, 슬퍼한다.
> 📎 OCC 22종 상세: [occ-emotion-detail.md](occ-emotion-detail.md)

- [x] EmotionType enum: 22종 (전체 구현 — Joy, Distress, Hope, Fear, Anger, Gratitude, Pride, Shame, Admiration, Reproach, Love, Hate, HappyFor, SorryFor, Resentment, Gloating, Satisfaction, FearsConfirmed, Relief, Disappointment, Gratification, Remorse)
- [x] ActiveEmotion struct: emotion_type, intensity, remaining_turns
- [x] PadState struct: Pleasure/Arousal/Dominance (각 f32, -1.0~1.0)
- [x] `decay_emotion()` — 턴 기반 감쇠 (decay.rs)
- [x] `hexaco_emotion_filter()` — HEXACO 성격 기반 감정 강도 필터 (filter.rs)
- [x] EmotionCategory enum: WellBeing, Fortune, Attribution, Attraction, Compound
- [x] Valence enum: Positive, Negative
- **테스트**: 감정 생성, 감쇠, PAD 경계값, HEXACO 필터, 카테고리 분류 ✅
- **참고**: 수련 효율 계수(emotion_training_modifier), 무공 적합도(emotion_art_affinity)는 Phase 5 통합 시 구현 예정

### Iteration 2.7 — 심리: OCC 인지 평가 ✅ 완료 [v2.0 완료]
> 사건을 평가하여 감정을 생성한다
> 📎 OCC 평가 흐름: [npc-psychology-architecture.md](npc-psychology-architecture.md) §6

- [x] OccStimulus struct: stimulus_type (Event/Action/Object), desirability, praiseworthiness, appealingness
- [x] OccAppraisal struct: 감정 타입 + 강도 후보 목록
- [x] `appraise_to_emotions(stimulus, values, personality) -> Vec<ActiveEmotion>` (순수 함수)
  - values=PracticalValues(③층 判) — 수식 직접 입력 [v2.0 명칭: WuxiaValues → PracticalValues]
  - personality=HexacoPersonality(①층) — 필터 역할 [v2.0 명칭: BigFive → HEXACO]
- [x] **핵심 공식 구현**: OCC 감정 유형별 평가 로직 (appraisal.rs)
- **테스트**: 배신 목격 → 의리↑ 캐릭터 분노 > 야망↑ 캐릭터, 5가치별 감정 차등 검증 ✅

**Iteration 2.4~2.7 완료 시점 (심리 도메인 전체):**
```
2.4   3축가치관 + 5가치 + 모듈 골격   ✅
2.5   HexacoPersonality (6요소)      ✅
2.6   OCC 감정 22종 + PAD            ✅
2.7   OCC 인지 평가                  ✅
심리 도메인 전체: 207 tests (wuxia-core psychology/)
```

### Iteration 2.8 — 기억 도메인: 기억 타입 + 저장 ✅ 완료 [v2.0 Sprint 2에서 구현]
> 겪은 일을 기억한다

- [x] MemoryType enum: 관찰(Observation)/반성(Reflection)/계획(Plan)
- [x] MemoryEntry struct: id, character_id, content, game_time, importance(1~10), memory_type, keywords, source_ids, reflection_tier, lang
- [x] MemoryRepository trait (Outbound Port) — save, find_recent, find_by_id, search, update_importance, count
- [x] EmbeddingPort trait (Outbound Port) — embed, embed_batch, embed_document, dimension, model_name
- [x] InMemoryRepository 구현 (wuxia-memory)
- **테스트**: 기억 추가, 시간순 조회, 유형별 필터 ✅

### Iteration 2.9 — 기억 도메인: 기억 검색 + 회상 ✅ 완료 [v2.0 Sprint 2에서 구현]
> 관련된 기억을 떠올린다

- [x] `retrieval_score()` — recency + importance + relevance (configurable via RetrievalWeights)
- [x] `recall_memories()` — 오케스트레이터 함수 (service.rs)
- [x] `store_memory()`, `recall_and_emit()`, `update_importance()` — 서비스 함수
- [x] ScoredMemory, RankedMemory — 점수 기반 기억 타입
- [x] EmotionalBias — 감정 편향 기반 검색 가중치 조정
- [x] RetrievalWeights — 검색 가중치 커스터마이징 (recency, importance, relevance)
- **테스트**: 최근+중요+관련 기억 상위 확인. 오래된 기억 점수 감소 확인. 감정 편향 검증 ✅

**Phase 2 완료 시점: 908 unit + 94 doc = 1,002 tests (wuxia-core)**

---

## Phase 3: 인간관계와 세계 `[wuxia-core]`

### Iteration 3.1 — 관계 도메인 (2축 모델) ✅ 완료 [v2.0: 3축→2축, Sprint 3에서 구현]
> 두 사람 사이에 관계가 생긴다

- [x] `relationship/` 모듈 생성 (13개 파일: types, level, trust_level, relationship_type, event, port, effect, chronicle, description, sentiment, extreme_anchors, sentiment_judge, delta)
- [x] Relationship struct: source_id, target_id, 호감도(affinity -100~+100)/신뢰도(trust 0~100) — **2축 모델** [v2.0 변경: 원안의 적대도 축 제거]
- [x] RelationshipType enum (8종): MasterDisciple/Siblings/Rivals/Allies/FamilyBond/Lovers/SwornSiblings/Enemies
- [x] RelationshipLevel enum (8단계): Enemy/Hostile/Wary/Stranger/Acquaintance/Friendly/Close/Intimate
- [x] TrustLevel enum: Wary/Cautious/Neutral/Trusting/DeepTrust
- [x] ConversationEffect + apply_conversation_effect() — 대화 기반 관계 변화
- [x] RelationshipDescriptions — 로컬라이즈된 관계 설명 (descriptions.toml)
- [x] RelationshipRepository trait (Outbound Port) — save, find_by_pair, find_by_source, find_by_target
- [x] ChronicleRepository trait (Outbound Port) — append, find_by_pair, find_by_session, find_by_change_type, count
- [x] RelationshipChronicle struct — 관계 변화 이력 기록 (13 fields)
- **테스트**: 관계 생성, 레벨 전환, 신뢰 변화, 대화 효과, 직렬화 ✅ (114 tests in relationship/)

### Iteration 3.2 — 관계: 감정 판정 + 영속 ✅ 완료 [v2.0: Sprint 3에서 구현]
> 대화하면 관계가 변한다 — 2단계 하이브리드 감정 판정

- [x] ExtremeAnchorSet — 극단 앵커 임베딩 기반 감정 분류 (sentiment.rs)
- [x] SentimentJudgment, SentimentDirection — LLM 판정 결과 타입
- [x] TurnCounter — 정기 LLM 판정 턴 카운터
- [x] DeltaSource enum — 호감도 변화 원인 추적 (Embedding/LlmJudgment/Manual)
- [x] judgment_to_delta() — 판정 결과 → 호감도 변화량 변환
- [x] ChangeType enum (5종): Affinity/Trust/LevelChanged/TypeChanged/BondBroken
- [x] CauseSource enum (5종): Conversation/Action/Event/TimePassage/ThirdParty
- **테스트**: 극단 앵커 판정, 판정→델타 변환, 연대기 기록 ✅
- **참고**: 피로 회복 계수, 간호 상호작용 등은 Phase 5 Bevy 통합 시 구현 예정

### Iteration 3.3 — 사물 도메인 [v1.0 피로/기연 연동 추가]
> 세계에 물건이 존재한다

- [ ] `item/` 모듈 생성
- [ ] Item struct: id, 이름, item_type, 무게, 희귀도(1~5), 설명
- [ ] ItemType enum: 무기/의복/음식/서적/약물/비급/잡화
- [ ] WeaponStats: 공격력, 무기유형(검/도/창/봉/암기)
- [ ] BookContent: 무공 학습 가능 여부, 관련 martial_art_id
- [ ] **피로 회복 아이템** [v1.0 신설]:
  - FatigueRecoveryItem trait 또는 속성: 피로 회복량, 즉시/지속 구분
  - 일반 영약(-15 즉시), 상급 영약(-30 즉시), 독주(-10 즉시 + 숙취), 보양식(-10/일×3일)
- [ ] **기연 아이템** [v1.0 신설]:
  - FortuneItem trait 또는 속성: 기연 조건③ 충족 여부
  - 비급 = 연마 기연 조건③, 영약 = 단련 기연 조건③
- **테스트**: 아이템 생성, 종류별 조회, 무기 스탯, 피로 회복량 계산, 기연 조건③ 판정

### Iteration 3.4 — 공간 도메인 [v1.0 수련/회복 연동 추가]
> 세계에 장소가 존재한다

- [ ] `space/` 모듈 생성
- [ ] Location struct: id, 이름, location_type, 좌표(x, y)
- [ ] LocationType enum: 수도/도시/마을/산/강/사찰/동굴/객잔/전장/온천/약초밭
- [ ] TerrainType enum: 평지/산악/수변/사막/숲
- [ ] `distance(a, b) -> f32` 두 장소 간 거리
- [ ] `travel_time(distance, agility) -> GameTimeDelta` 이동 시간
- [ ] **수련 보너스 장소** [v1.0 신설]: 기연 조건① 충족
  - `training_bonus(location: &Location, stat_or_art_type) -> Option<f32>`
  - 무당산→내공 보너스, 화산→검법 보너스, 소림사→권법/내공 보너스
- [ ] **피로 회복 장소** [v1.0 신설]:
  - `fatigue_recovery_rate(location: &Location) -> i32`
  - 온천(-20/일), 약초밭(-15/일), 사찰/도관(-15/일), 자기 문파(-12/일), 객잔(-10/일), 야외 노숙(-3/일)
- **테스트**: 장소 생성, 거리 계산, 이동 시간 추정, 수련 보너스 확인, 피로 회복률 확인

### Iteration 3.5 — 세계관 도메인 (국가)
> 7개 국가가 존재한다

- [ ] `world/` 모듈 생성
- [ ] Nation struct: id, 이름, government_type(왕조/공화국)
- [ ] WuxiaWorld struct: nations Vec (최대 7개 제약)
- [ ] `add_nation() -> Result` (7개 초과 시 에러, 왕조 최대 4, 공화국 최대 3)
- **테스트**: 7개 생성 성공, 8번째 실패, 왕조 5번째 실패

### Iteration 3.6 — 세계관: 무림 조직
> 정파 구파일방과 사파 3파벌

- [ ] Sect struct: id, 이름, alignment(정파/사파), 본거지(LocationId)
- [ ] MurimWorld struct: orthodox_sects(최대 10), dark_factions(최대 3)
- [ ] Membership struct: character_id, sect_id, rank, join_date
- [ ] `join_sect()` / `leave_sect()` / `promote()` 메서드
- **테스트**: 구파일방 10개 생성, 11번째 실패. 가입/탈퇴/승급.

### Iteration 3.7 — 세계관: 상인 조직
> 상방이 교역한다

- [ ] MerchantHouse struct: id, 이름, specialty(주력거래품), wealth
- [ ] CommerceSystem struct: currency_name("냥"), merchant_houses Vec
- **테스트**: 상방 생성, 조회

---

## Phase 3-A: LLM · 메모리 · 품질 어댑터 ✅ 완료 [v2.0 신설 — Sprint 1~3에서 구현]

> dev-plan 원안에 없었으나 Sprint 1~3에서 구현된 어댑터 계층.
> wuxia-core의 포트 트레잇을 실제 외부 시스템에 연결한다.
> 📎 상세: sprint1-progress.md (소연이 말한다), sprint2-progress.md (소연이 기억한다), sprint3-progress.md (소연이 영원히 기억한다)

### Sprint 1 — LLM 어댑터 (wuxia-llm) ✅ 완료
- [x] LlmPort trait 구현: MockLlm (테스트용), LlamaCppAdapter (프로덕션 — feature `live-llm`)
- [x] 프롬프트 조립: XML 2-layer builder, CharacterPromptData, PromptContext
- [x] 응답 파서: LLM 응답 파싱 (parser.rs, text_utils.rs)
- [x] 소연 캐릭터 프롬프트 팩토리 (fixtures.rs)
- **테스트**: 340 tests (319 unit + 21 doc) in wuxia-llm

### Sprint 2 — 메모리 어댑터 (wuxia-memory) ✅ 완료
- [x] InMemoryRepository — MemoryRepository 구현 (테스트용)
- [x] LanceDbRepository — LanceDB 기반 벡터 검색 (feature `live-db`)
- [x] EmbeddingPort 구현: MockEmbedding (해시 기반), LlamaCppEmbedding (BGE-M3 1024-dim)
- [x] EmbeddingConfig — TOML 프로파일 기반 설정 (gemma/bge-m3)
- **테스트**: 97 tests (91 unit + 6 doc) in wuxia-memory

### Sprint 3-A — 대화 관리 + 컨텍스트 ✅ 완료
- [x] ChatSession — 전체 대화 루프 오케스트레이터 (LLM 호출, 기억 주입, 관계 추적, 감정 파이프라인)
- [x] ConversationManager — 컨텍스트 윈도우 관리, 압축 결정
- [x] ContextProvider trait — Null/Static/Live 구현
- [x] LiveContextProvider (wuxia-app) — 도메인 rank_memories() + 어댑터 format_memories_for_prompt()

### Sprint 3-B — 감정 판정 파이프라인 ✅ 완료
- [x] SentimentPipeline — 2단계 하이브리드 (극단 앵커 트리거 + 정기 LLM 판정)
- [x] SentimentJudge trait + LlmSentimentJudge + MockSentimentJudge
- [x] JSON 판정 파서 (이중 i64/str score 파싱)
- 📎 상세: [embedding-sentiment-plan.md](embedding-sentiment-plan.md)

### Sprint 3-C — 관계 영속 어댑터 ✅ 완료
- [x] InMemoryRelRepo / JsonFileRelRepo — RelationshipRepository 구현
- [x] InMemoryChronicleRepo / JsonlChronicleRepo — ChronicleRepository 구현
- [x] 관계 저장: relationships.json (전체 상태), relationship_chronicles.jsonl (변화 이력)
- 📎 상세: [relationship-persistence-plan.md](relationship-persistence-plan.md)

### Sprint 3-D — 대화 품질 벤치마크 ✅ 완료
- [x] TOML 기반 테스트 시나리오 (scenario.rs)
- [x] 6가지 자동 측정 메트릭 (metrics.rs)
- [x] JudgePort trait + MockJudge / ClaudeJudge / OpenAiJudge (feature-gated)
- [x] FullBenchReport + ComparisonReport — A/B 테스트 비교
- [x] TurnTrace / SessionTrace / TimingTrace / MemoryHit — 턴별 추적
- [x] Terminal replay (replay.rs) — --replay / --detailed 플래그
- 📎 상세: [conversation-quality-test-plan.md](conversation-quality-test-plan.md), [test-improvement-plan.md](test-improvement-plan.md)

### wuxia-data ✅ 완료
- [x] TOML/JSON 로더, PromptConfig, RelationshipDescriptions, ExtremeAnchorsData, SentimentJudgeData
- **테스트**: 16 tests (13 unit + 3 doc)

### wuxia-app (조립 계층) ✅ 완료
- [x] LiveContextProvider — 도메인 타입 → 어댑터 타입 변환 (RankedMemory → MemoryView)
- [x] soyeon_chat_v2 예제 — 인터랙티브 NPC 데모 (feature `live-demo`)
- [x] conversation_bench 예제 — 품질 벤치마크 러너 (feature `quality-bench`)
- **테스트**: 8 tests

---

## Phase 4: 동적 시스템 `[wuxia-core]`

### Iteration 4.1 — 경제 도메인 (기본)
> 물건을 사고판다

- [ ] `economy/` 모듈 생성
- [ ] Wallet struct: character_id, balance(냥)
- [ ] PriceRegistry: item_id → base_price 매핑
- [ ] `buy(wallet, item, price) -> Result<Transaction>`
- [ ] `sell(wallet, item, price) -> Result<Transaction>`
- **테스트**: 구매 → 잔고 감소 + 성공. 잔고 부족 → 실패.

### Iteration 4.2 — 전투 도메인 (기본) [v1.0 피로/경지 연동]
> 간단한 전투 판정

- [ ] `combat/` 모듈 생성
- [ ] CombatPower: 공격력/방어력/민첩성 계산 (GrowthProfile + MartialArt + WeaponStats)
- [ ] `resolve_duel(a: CombatPower, b: CombatPower) -> DuelResult`
- [ ] DuelResult: 승자, 패자 부상 정도, 발생 이벤트
- [ ] 부상 → Character::injure() 호출 [v1.0 캐릭터 도메인 연동]
- [ ] **무공 경지 전투 보너스** [v1.0 신설]: 숙련→+5%, 통달→+15%, 화경→+30%
- [ ] **피로 전투력 감소** [v1.0 신설]: 피로 높으면 전투력 감소
  - 양호(0~20): 감소 없음
  - 경미(21~40): -5%
  - 보통(41~60): -15%
  - 심각(61~80): -30%
  - 탈진(81~100): 전투 불가
- [ ] **전투 피로 누적** [v1.0 신설]: 전투 후 피로 += 기본피로 × (100 - 체력) / 100
- **테스트**: 강자 > 약자 승률 80%+. 부상 시 능력치 감소. 피로 높으면 전투력↓. 경지 보너스 확인.

### Iteration 4.3 — 전투: 감정과 지형 영향
> 분노하면 더 세게 치고, 산에서는 경공이 유리하다

- [ ] `emotion_combat_modifier(pad: &PADState) -> CombatModifier`
  - 분노(P↓A↑): 공격↑방어↓
  - 공포(P↓D↓): 공격↓도주↑
  - 자부(P↑D↑): 전체↑
- [ ] `terrain_combat_modifier(terrain: TerrainType) -> CombatModifier`
  - 산악: 경공↑, 병기↓
  - 수변: 경공↓, 내공↑
- **테스트**: 동일 능력치, 분노 시 공격력 15%+ 증가. 산악 경공 20%+ 보정.

### Iteration 4.4 — 서사 도메인 (기본) [v1.0 기연 이벤트 추가]
> 이벤트가 발생하고 퀘스트가 진행된다

- [ ] `narrative/` 모듈 생성
- [ ] StoryCondition enum: 관계임계/감정임계/시간경과/장소도달/아이템보유
- [ ] StoryEvent struct: id, conditions, outcomes(Vec<DomainEvent>)
- [ ] Quest struct: id, 목표, 상태(미시작/진행/완료/실패)
- [ ] `check_conditions(world_state) -> Vec<TriggeredEvent>`
- [ ] **기연 이벤트 연동** [v1.0 신설]:
  - GrowthEvent::FortuneTriggered 수신 → 서사적 이벤트 생성
  - 기연 등급 4+ → 특별 스토리 이벤트 트리거
- **테스트**: 조건 충족 → 이벤트 발생. 퀘스트 상태 변경. 기연 → 서사 이벤트.

### Iteration 4.5 — 성장: 기연(奇緣) 시스템 [v1.0 신설]
> 장소 + 사람 + 아이템 + 감정 + 시기 — 성장이 폭발하는 순간

**Step 1: 기연 조건 판정**
- [ ] FortuneCondition struct: location, companion, item, emotion, timing
- [ ] `evaluate_fortune_conditions(conditions) -> (u32, u32)` (순방향 수, 역방향 수) 순수 함수
- [ ] `determine_fortune_grade(forward_count, backward_count) -> FortuneGrade`
- [ ] FortuneGrade enum: 일상/호기/전기/기연/천재기연/불운/위기/액운/대화/주화입마

**Step 2: 기연 가중치 적용**
- [ ] `fortune_multiplier(grade: FortuneGrade) -> f32`
  - 순방향: 1.0/1.5/2.5/4.0/7.0
  - 역방향: 1.0/1.5/2.5/4.0/7.0
- [ ] 단련 기연 효과: 추가 능력치, 피로 회복, 상한 돌파(100→105)
- [ ] 연마 기연 효과: 경지 돌파, 비밀 특성 발견, 영구 보너스
- [ ] GrowthEvent::FortuneTriggered { grade, effects }

**Step 3: train_stat / train_art에 기연 판정 통합**
- [ ] TrainingService에 기연 판정 단계 추가
  1. 최대 강도 계산
  2. 5가지 조건 수집 (공간/관계/사물/심리/시간 도메인 조회)
  3. 기연 등급 결정
  4. 가중치 적용하여 성장량 계산
  5. 기연 등급별 추가 효과 적용
  6. 피로/부상 처리
  7. 이벤트 수집/반환
- **테스트**:
  - 조건 0개 → 일상(×1.0)
  - 조건 3개 순방향 → 전기(×2.5) 성장량 확인
  - 조건 5개 순방향 → 천재기연(×7.0) + 상한 돌파
  - 역방향 조건 → 재앙 판정
  - 순역 혼합 → 상쇄 확인

### Iteration 4.6 — 피로 회복 통합 (Application Service) [v1.0 신설]
> 장소 + 사람 + 아이템의 피로 회복이 중첩된다

- [ ] `FatigueRecoveryService` (Application Service):
  1. 장소 피로 회복 (공간 도메인)
  2. 사람 피로 회복 (관계 도메인)
  3. 아이템 피로 회복 (사물 도메인)
  4. 기본 회복 (수면 -5)
  5. 합산 적용
- [ ] 중첩 예시:
  - 최악: 야외 노숙(-3) + 적대자(+5) = +2/일 (피로 증가)
  - 보통: 객잔(-10) + 수면(-5) = -15/일
  - 최상: 온천(-20) + 연인(-20) + 상급 영약(-30) = -70 즉시
- **테스트**: 장소+사람+아이템 중첩 회복. 최악/보통/최상 시나리오.

### Iteration 4.7 — wuxia-core 통합 테스트
> Phase 1~4 전체가 함께 동작하는지 확인

- [ ] 통합 테스트: "수련하고 성장하고 피로가 쌓인다"
  - 캐릭터 생성 → 성장 프로필 → 무공 습득 → 단련 3일 → 피로 누적 → 휴식 → 회복
- [ ] 통합 테스트: "연마하다 기연을 만난다"
  - 보너스 장소 + 스승 + 비급 + 결의 → 전기(3조건) → ×2.5 성장
- [ ] 통합 테스트: "시간이 흐르며 세계가 변한다"
  - 5년 tick → 나이 +5, 쇠퇴/성장, 관계 변화, 피로 자연회복
- [ ] 통합 테스트: "제자가 무리해서 다친다"
  - 높은 강도 수련 → 피로 80+ → 부상 → 간호 → 관계 발전
- **테스트**: `cargo test -p wuxia-core --test integration` 전체 통과

---

## Phase 5: Bevy 연결 `[wuxia-game]`

> wuxia-core를 Bevy Plugin/Component/Resource로 연결한다.
> 하이브리드 Component 전략 적용.

### Iteration 5.1 — Bevy 기본 앱 + TimePlugin
> 가장 작은 Bevy 앱: 시간이 흐른다

- [ ] main.rs: App::new() + DefaultPlugins
- [ ] **TimePlugin** 구현
  - Resource: `GameClockRes(wuxia_core::time::GameClock)`
  - Bevy Event: `DayPassed`, `SeasonChanged`, `YearPassed`
  - System: `tick_system` (키 입력으로 시간 진행)
- [ ] 화면에 현재 날짜/계절 텍스트 표시
- **테스트**: 앱 실행 → 시간 진행 → 날짜 변경 확인

### Iteration 5.2 — CharacterPlugin + Hybrid Component [v1.0 피로/부상 포함]
> 캐릭터가 Bevy 세계에 존재한다

- [ ] **통째 Component**: `CharacterIdentity(Character)` — 피로/부상 포함
- [ ] **분리 Component**: `Position { x, y }`, `SpriteInfo { ... }`
- [ ] **CharacterPlugin** 구현
  - Event: `CharacterAgedEvent`, `LifeStageChangedEvent`, `FatigueChangedEvent`, `InjuredEvent`
  - System: `aging_system` (YearPassed → Character::age_one_year 호출)
  - System: `daily_recovery_system` (DayPassed → 피로 자연회복 + 부상 진행) [v1.0]
  - System: `spawn_character_system`
- **테스트**: 1년 경과 → 캐릭터 나이 +1, 하루 경과 → 피로 -5

### Iteration 5.3 — GrowthPlugin + PsychologyPlugin [v1.0 업데이트]
> 캐릭터 내면이 Bevy에 연결된다

- [ ] **통째 Component**: `Growth(GrowthProfile)`
- [ ] **통째 Component**: `Personality(BigFivePersonality)`, `ThreeAxis(ThreeAxisValues)`, `Values(WuxiaValues)` [v1.5 3 Aggregate]
- [ ] **GrowthPlugin**: yearly_growth_system, training_system, **fortune_system** [v1.0]
- [ ] **PsychologyPlugin**: emotion_decay_system, occ_evaluation_system, **reflection_trigger_system** [v1.5]
- **테스트**: 수련 → 능력치 증가 + 피로. 사건 → 감정 변화. 기연 판정. Plugin 간 이벤트.

### Iteration 5.4 — LLM Resource (Non-Send)
> LLM에게 물어보고 답을 받는다

- [ ] Resource: `LlmService(Box<dyn LlmPort>)`
- [ ] MockLlmAdapter (테스트용)
- [ ] LlamaCppAdapter (프로덕션용: llama-cpp-2)
- [ ] main.rs에서 선택적 주입
- [ ] 프롬프트 템플릿: 캐릭터 상태 (능력치+**3축+신조**+감정+**피로/부상**) → 프롬프트 조립 [v1.5 갱신]
- **테스트**: MockLlm으로 프롬프트 조립 → 응답 파싱 확인

### Iteration 5.5 — LLM 비동기 처리
> LLM 호출이 게임을 멈추지 않는다 (성능 1순위)

- [ ] `AsyncComputeTaskPool` 활용 LLM 비동기 호출
- [ ] `LlmTask` Component: 비동기 작업 상태 추적
- [ ] 요청 → 대기(NPC "생각 중...") → 완료 → 결과 반영
- [ ] 동시 요청 큐 관리 (최대 N개)
- **테스트**: LLM 호출 중 게임 루프 안 멈춤

### Iteration 5.6 — Persistence Resource (LanceDB)
> 기억을 LanceDB에 저장하고 벡터 검색한다

- [ ] Resource: `MemoryDb(Box<dyn MemoryRepository>)`
- [ ] LanceDbAdapter 구현 (MemoryRepository)
- [ ] 벡터 임베딩으로 기억 검색 (relevance 대체)
- **테스트**: 기억 저장 → 벡터 검색 조회. InMemory 테스트와 동일 케이스 통과.

### Iteration 5.7 — 나머지 Plugin 연결
> 관계, 세계관, 공간, 사물, 경제, 전투, 서사

- [ ] RelationshipPlugin: 관계 변화 + **피로 회복 계수 제공** [v1.0]
- [ ] SpacePlugin: 장소 + **수련 보너스/피로 회복 장소** [v1.0]
- [ ] ItemPlugin: 사물 + **피로 회복 아이템** [v1.0]
- [ ] WorldPlugin, EconomyPlugin, CombatPlugin, NarrativePlugin
- **테스트**: 각 Plugin 이벤트 수신/발행 확인

### Iteration 5.8 — Plugin 간 이벤트 흐름 통합 테스트 [v1.0 갱신]
> 모든 Plugin이 이벤트로 연결되어 동작한다

```
TimePlugin ──YearPassed──▶ CharacterPlugin (나이+1)
                           GrowthPlugin (쇠퇴/성장)
                           PsychologyPlugin (성격변화)
                           RelationshipPlugin (소원해짐)

TimePlugin ──DayPassed──▶ CharacterPlugin (피로 자연회복, 부상 진행) [v1.0]

PsychologyPlugin ──EmotionChanged──▶ GrowthPlugin (수련 효율 계수 변경) [v1.0]
                                     NarrativePlugin (스토리 분기)
                                     CombatPlugin (전투 감정 보정)

GrowthPlugin ──FortuneTriggered──▶ NarrativePlugin (기연 이벤트 생성) [v1.0]
                                   CharacterPlugin (피로/부상 회복) [v1.0]
```

- [ ] Bevy MinimalPlugins 헤드리스 통합 테스트
- **테스트**: `cargo test -p wuxia-game --test integration`

---

## Phase 6: 통합 시나리오 `[전체]`

### Iteration 6.1 — "제자의 배반" 시나리오 [v1.0 피로/기연 포함]
> 모든 도메인이 함께 동작하는 시나리오

- [ ] 스승(55세) + 제자(18세) 캐릭터 생성
- [ ] 화산파 입문, 사제 관계 형성
- [ ] **단련/연마 3년** → 능력치 상승 + 피로 순환 [v1.0]
- [ ] **기연 발생**: 사과절벽(장소) + 스승(사람) + 석벽 검보(아이템) → 전기(3조건) [v1.0]
- [ ] 비밀 발견 → 갈등 누적
- [ ] 가치관 변화 → 탈문 결정 → 전투 (피로 영향 포함) → 결과 기억
- **테스트**: 전체 흐름 E2E 확인

### Iteration 6.2 — "강호 방랑" 시나리오 [v1.0 피로/기연 포함]
> 캐릭터가 세계를 돌아다니며 성장한다

- [ ] 여러 장소 이동 (여행 피로 누적) → NPC 만남 → 전투 → 비급 습득
- [ ] **피로 순환**: 수련→피로↑→온천 휴식(장소 회복)→관계 발전→수련 [v1.0]
- [ ] **기연**: 무당산(장소)+은둔고수(사람)+내공 비급(아이템) → 기연(4조건) → 피로 완전 회복 [v1.0]
- [ ] 감정 변화 → 기억 → 반성 (LLM)
- [ ] 상점 거래
- **테스트**: 30일 시뮬레이션 → 성장/관계/기억/피로 변화 확인

---

## Phase 7: 게임 UI `[wuxia-game]` (향후)

> Phase 6 완료 후 세부 계획 수립

- [ ] Pixel Art 스프라이트 로딩
- [ ] 타일맵 세계 표현
- [ ] 캐릭터 이동 조작
- [ ] 대화 UI, 전투 UI, 인벤토리 UI
- [ ] 캐릭터 상태 화면 (**피로/부상 표시 포함**) [v1.0]
- [ ] **수련 UI**: 단련/연마 선택, 강도 조절, 최대 강도 표시 [v1.0]

---

## 진행 규칙

1. **한 턴에 한 이터레이션만** 진행한다
2. 코드 구조를 **먼저 리뷰** 받고 구현한다
3. **`cargo test` 통과**해야 다음으로 간다
4. 이전 이터레이션의 테스트가 **깨지면 안 된다**
5. 불확실하면 **물어보고** 진행한다
6. Phase 1~4는 `cargo test -p wuxia-core` (Bevy 없이)
7. Phase 5~6은 `cargo test -p wuxia-game` (Bevy 포함)
8. **wuxia-core에 `bevy` 의존성이 추가되면 안 된다**

---

## 이터레이션 요약

| Phase | crate | 이터레이션 | 내용 | 상태 |
|-------|-------|-----------|------|------|
| 0 | workspace | 1 | 프로젝트 뼈대 | ✅ 완료 |
| 1 | wuxia-core | 4 | 기반: 공유타입, 캐릭터, 시간, 연결 | ✅ 완료 |
| 1-R | wuxia-core | 1 | 시간대(Watch/經) 시스템: 1tick=1시간대, 십이시진 매핑 | ✅ 완료 |
| 2 | wuxia-core | 12 | 내면: 성장(단련/연마/무공경지), 피로/부상, 최대강도, 수련-피로연결, 심리(HEXACO/3축/5가치/OCC22종/PAD/인지평가), 기억(타입+검색+회상) | ✅ 완료 (1,002 tests) |
| 3.1~3.2 | wuxia-core | 2 | 관계: 2축모델(호감도/신뢰도), 감정판정, 연대기, 영속 포트 | ✅ 완료 (114 tests) |
| **3-A** | **어댑터 전체** | **Sprint 1~3** | **LLM어댑터, 메모리어댑터, 대화관리, 감정판정파이프라인, 관계영속, 품질벤치** | **✅ 완료 (wuxia-llm 340, wuxia-memory 97, wuxia-data 16, wuxia-app 8 tests)** |
| 3.3~3.7 | wuxia-core | 5 | 세계: 사물(피로/기연), 공간(수련/회복보너스), 세계관(국가/무림/상인) | 📋 미착수 |
| 4 | wuxia-core | 7 | 동적: 경제, 전투(피로/경지), 서사(기연이벤트), **기연시스템**, **피로회복통합**, 통합테스트 | 📋 미착수 |
| 5 | wuxia-game | 8 | Bevy: Plugin, Component, Resource, 비동기LLM | 📋 미착수 |
| 6 | 전체 | 2 | 시나리오: 제자의 배반(기연), 강호 방랑(피로순환) | 📋 미착수 |
| 7 | wuxia-game | 미정 | UI: 스프라이트, 타일맵, 대화, 전투, 수련UI | 📋 미착수 |
| **합계** | | **42+** | **완료: Phase 0~3-A (도메인 + 어댑터), 총 ~1,463 tests** | |

---

## 설계 결정 기록

### ADR-001: DomainEvent 리팩터링 (Iteration 2.1)
- **결정**: 단일 enum → 도메인별 enum + DomainEvent wrapper
- **근거**: 도메인 독립성, 확장 용이성
- **대안**: 완전 분리 (도메인별 별도 이벤트 버스) → 기존 코드 변경 과다로 기각

### ADR-002: 성장 도메인 StatBlock 생성 패턴 (Iteration 2.1)
- **결정**: StatBlock struct (이름 있는 필드)
- **근거**: 필드명이 문서 역할, Serde 호환, 파라미터 순서 오류 방지
- **대안**: Builder 패턴 → 간단한 구조에 과잉, 9개 매개변수 → 순서 오류 위험

### ADR-003: ~~심리 도메인 2 Aggregate 분리~~ (v0.9) — [v1.5 대체됨 → ADR-012]
- **결정**: ~~psychology/ 안에 BigFivePersonality + WuxiaValues 두 개의 독립 Aggregate~~
- **근거**: 성격과 가치관은 독립적으로 변함 (배신→가치관만, 은둔→성격만)
- **대안**: 완전 별도 도메인 → Application Service 연결은 동일, 폴더만 늘어남
- **폐기 사유**: 3축 가치관(LLM 서사)과 5가치(OCC 수식)가 사용처/변화속도가 다르므로 3 Aggregate로 확장 필요

### ADR-004: ~~能/性/信 프레임워크~~ (v0.9) — [v1.5 확장됨 → ADR-013]
- **결정**: ~~성장(能-능력), 심리/성격(性-성향), 심리/가치관(信-신념) 3원 분류~~
- **근거**: "할 수 있는가/원래 그런가/믿는가" — 세 질문이 서로 다른 도메인 경계를 형성
- **검증**: 츤데레 = 공감(能)↑ + 친화성(性)↓ → 두 도메인 독립 작동 증명
- **폐기 사유**: 信이 서사(敍, 3축)와 판단(判, 5가치) 두 층으로 분화 → 4원 분류로 확장

### ADR-005: 규칙-Aggregate 분리 — training.rs (Iteration 2.2)
- **결정**: 성장 계수/노화 테이블을 training.rs(순수 함수)로 분리, GrowthProfile(Aggregate)에서 호출
- **근거**: 밸런싱 수치 변경 시 Aggregate 로직 불변. 향후 TOML/JSON 외부화 용이. 순수 함수라 테스트 간단.
- **대안**: Aggregate 내부에 규칙 직접 구현 → 간단하지만 수치와 로직이 혼재

### ADR-006: StatChange의 delta를 i32로 (Iteration 2.2)
- **결정**: 능력치(u32)의 변화량을 i32 하나로 표현 (+성장, -쇠퇴)
- **근거**: 성장과 쇠퇴를 Vec<StatChange> 하나로 이벤트에 담을 수 있음. increase/decrease 별도 메서드보다 이벤트 표현이 깔끔.
- **안전장치**: apply_stat_change()에서 saturating_add/sub + clamp(0~100)

### ADR-007: 피로/부상을 캐릭터 도메인에 배치 (v1.0 신설)
- **결정**: fatigue(u32)와 injury(Option<Injury>)를 Character struct에 배치
- **근거**: 피로는 수련, 전투, 여행, 탐험 등 모든 활동에서 쌓이므로 특정 도메인 소유가 아닌 캐릭터의 범용 속성
- **대안**: 성장 도메인 소유 → 전투/여행 피로를 성장 도메인이 처리하면 의미 불일치

### ADR-008: 단련/연마 용어 확정 (v1.0 신설)
- **결정**: train → train_stat(단련), train_art(연마)로 분리. 영문: Conditioning / Practice
- **근거**: 능력치 집중 성장과 무공 숙련도 성장은 결과와 비용이 다른 별도 행위. 코드와 게임 내 용어 일치.
- **대안**: 통합 train() 하나로 처리 → 단련/연마의 서로 다른 효과(부산물 등) 표현이 복잡

### ADR-009: 기연 시스템 5조건 설계 (v1.0 신설)
- **결정**: 장소(場)/사람(人)/아이템(物)/감정(心)/시기(時) 5가지 조건으로 기연 판정
- **근거**: 무협 소설에서 성장의 전환점에는 항상 장소, 인물, 도구, 마음 상태, 때가 복합적으로 작용. 조건 수에 따라 기연 등급이 결정되어 게임 밸런스와 서사 깊이를 동시 달성.
- **대안**: 확률 기반 랜덤 기연 → 조건 없이 랜덤이면 성장-탐험-관계 순환 동기 부여 약함

### ADR-010: 최대 강도 메카닉 (v1.0 신설)
- **결정**: 수련 강도 상한 = (체력+인내)/20 + 의지보너스 - 피로. 의지 초과분은 부상 확률 증가.
- **근거**: 무한 수련 방지 + 피로 순환의 핵심 동력. 의지로 한계를 넘는 것은 무협적(위험하지만 가능).
- **대안**: 피로만으로 제한 → 의지/인내/체력의 역할이 없어져 캐릭터 차별화 약화

### ADR-011: 시간대(Watch/經) 시스템 도입 (v1.1 신설)
- **결정**: 1 tick을 1일에서 1시간대(Watch)로 세분화. 하루 = 6시간대, 십이시진 2개씩 묶음.
- **Watch 정의**: Dawn(黎明/인묘시) → Morning(午前/진사시) → Midday(正午/오미시) → Afternoon(午後/신유시) → Evening(黃昏/술해시) → Night(深夜/자축시)
- **하루 시작**: Dawn(새벽). Night→Dawn 전환 시 날짜 +1.
- **이벤트 체계**: tick() → WatchChanged(항상) → DayPassed(Night→Dawn) → SeasonChanged(조건) → YearPassed(조건)
- **행동 방식**: "묘시까지 수련한다" = tick_until(Watch::Morning). 매 시간대 결정이 아닌 "언제까지" 방식.
- **근거**: 무협 소설의 시간 표현("자시에 만나자")을 자연스럽게 지원. 시간대별 수련 보너스로 전략적 선택 추가(새벽 내공↑, 한낮 외공↑, 심야 사파무공↑). 기연 5조건 중 '시기(時)' 조건의 기반. DayPassed 구독자는 변경 불요.
- **대안A**: 4시간대(아침/낮/저녁/밤) → 단순하지만 시간대 보너스 다양성 부족.
- **대안B**: 12시간대(십이시진 그대로) → 플레이어 결정이 너무 많음, 게임플레이 리듬 저해.
- **호환성**: GameTime::new() 기본값 Dawn, 기존 next_day()/advance_days() 유지, tick_day()로 기존 tick() 동작 대체.

### ADR-012: 심리 도메인 3 Aggregate 분리 (v1.5 신설, ADR-003 대체) — [v2.0 명칭 갱신]
- **결정**: psychology/ 안에 HexacoPersonality + ThreeAxisValues + PracticalValues **세 개**의 독립 타입 [v2.0: BigFive→HEXACO, WuxiaValues→PracticalValues]
- **근거**:
  - 3축(②층)은 LLM 서사용(강도 + 신조 텍스트), 5가치(③층)는 OCC 수식용(수치만) → 사용처가 다름
  - 3축은 Tier 2~4에서 천천히, 5가치는 Tier 1~3에서 중간 속도로 변함 → 변화 속도가 다름
  - 같은 옳음 0.9라도 신조에 따라 5가치 분포가 정반대 → 해석 관계(자동계산 불가)
  - 성찰(⑦층)이 양방향 번역기 역할: 하향 압력(3축→5가치) + 상향 신호(5가치→3축)
- **대안**: 3축+5가치를 하나의 Aggregate로 통합 → 변화 속도/사용처가 다르므로 기각
- **이전**: ADR-003 (2 Aggregate)을 대체함
- 📎 상세: [npc-psychology-architecture.md](npc-psychology-architecture.md) §3~§5

### ADR-013: 能/性/敍/判 프레임워크 (v1.5 신설, ADR-004 확장)
- **결정**: 능력(能)/성향(性)/서사(敍)/판단(判) **4원 분류**
  - 能: "할 수 있는가?" → 성장 도메인 (의지, 공감, 내공)
  - 性: "원래 그런 사람인가?" → 심리/성격 ①층 (친화성, 외향성)
  - 敍: "어떤 존재인가?" → 심리/3축 ②층 (믿음, 옳음, 바람 + 신조)
  - 判: "상황에서 어떻게 판단하는가?" → 심리/5가치 ③층 (충, 의, 효, 복수, 야망)
- **근거**: 기존 信이 "서사적 정체성(3축+신조)"과 "상황적 판단 계수(5가치)"로 분화. 명경과 조고가 같은 옳음 0.9인데 정반대 인물인 이유가 信 하나로는 설명 불가.
- **검증**: 명경 옳음 0.9 "도의를 지킨다" → 충↑의↑ vs 조고 옳음 0.9 "힘이 정의다" → 충↓야망↑
- **이전**: ADR-004 (能/性/信 3원)을 확장함
- 📎 상세: [domain-analysis.md](domain-analysis.md) §2.2

---

## 변경 이력

| 버전 | 날짜 | 변경 내용 |
|------|------|-----------|
| v0.8 초안 | 2025-02-06 | 6 Phase, 25 이터레이션 |
| v0.8 수정 | 2025-02-06 | 아키텍처 결정 반영: Phase 0 추가, crate 분리, Phase 5 확장, 36+ 이터레이션 |
| v0.9 | 2025-02-06 | Phase 0~1 완료 표시. Iteration 2.1 완료 (DomainEvent 리팩터링 + GrowthProfile). 심리 도메인 2 Aggregate 구조 반영 (2.4/2.5). 能/性/信 프레임워크 도입. ADR 4건 추가. 테스트 현황: 151 tests. |
| v0.9.1 | 2025-02-09 | Iteration 2.2 완료 (수련/쇠퇴). LifeStage 연령 조정(33/55/69). growth/event.rs, growth/training.rs 신규. 규칙-Aggregate 분리 패턴 도입. ADR 2건 추가 (005, 006). 테스트 현황: 189 tests. |
| **v1.0** | **2026-02-11** | **도메인 분석 v1.0 전면 반영. Iteration 2.3 대폭 확장 (이름 리팩터링 + 무공 경지 + 연마 + 부산물). Iteration 2.3A~2.3C 신설 (피로/부상, 최대 강도, 수련-피로 연결). Iteration 4.5~4.6 신설 (기연 시스템, 피로 회복 통합). Phase 3~4 전체 도메인에 피로/기연 연동 반영. Phase 5~6 피로/기연 포함 갱신. ADR 4건 추가 (007~010). 41+ 이터레이션.** |
| **v1.1** | **2026-02-12** | **Phase 1-R 신설: 시간대(Watch/經) 시스템. 십이시진 2개 묶음 × 6시간대, "묘시까지 수련" 방식. Iteration 2.3 완료 (265 unit + 37 doc = 302 tests). ADR-011 추가 (Watch 시스템). 42+ 이터레이션.** |
| **v1.2** | **2026-02-15** | **Iteration 2.3A~2.3C 완료. 피로/부상 시스템(FatigueLevel, Injury 4종×3등급), 최대 강도 메카닉(calculate_max_intensity, over-limit), TrainingService 7단계 통합(TrainingError, TrainingOutcome). 472 tests (422 unit + 50 doc).** |
| **v1.3** | **2026-02-19** | **심리 도메인 7층 아키텍처 반영. ADR-003 폐기→ADR-012 (2 Aggregate→3 Aggregate: 성격+3축가치관+5가치). ADR-004 폐기→ADR-013 (能/性/信→能/性/敍/判 4원 프레임워크). Iteration 2.4 전면 개편 (3축+CreedCandidate+5가치 역할 재정의). Iteration 2.5~2.7 v1.5 반영 (OCC 용어, 3 Aggregate 참조). Iteration 5.3 PsychologyPlugin 3 Aggregate Component. Iteration 5.4 프롬프트에 3축+신조 포함. Phase 2 요약 갱신.** |
| **v1.4** | **2026-02-19** | **3축 용어 변경: 정(情)/소신(節)/갈망(欲) → 믿음(信)/옳음(正)/바람(願). Iteration 2.4 필드 설명, 3축↔5가치 검증 테스트 예시, ADR-012/013 내 용어 일괄 교체. NPC 심리 아키텍처 v1.1·도메인 분석 v1.6·GDD v0.6과 용어 정렬.** |
| **v2.0** | **2026-03-03** | **현행화 전면 갱신. Phase 1-R(Watch) 완료 표시. Iteration 2.4~2.7(심리) 완료 — BigFive→HEXACO(6요소), WuxiaValues→PracticalValues 명칭 반영. Iteration 2.8~2.9(기억) 완료. Iteration 3.1~3.2(관계) 완료 — 3축→2축 모델 반영. Phase 3-A 신설 — Sprint 1~3 어댑터 작업 반영(LLM/Memory/Sentiment/Quality/Persistence). ADR-012 명칭 갱신. 이터레이션 요약표 전면 갱신. wuxia-core 1,002 tests, 전체 ~1,463 tests.** |
