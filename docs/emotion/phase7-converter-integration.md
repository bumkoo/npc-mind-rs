# Phase 7 설계 — Listener-perspective Converter 프로덕션 통합

**상태**: Step 1-3 완료 (2026-04-19), Step 4 (MindService 통합) 대기
**관련 문서**: [`sign-classifier-design.md`](./sign-classifier-design.md) §3.1~§3.7
**전제**: Phase 1 (sign 81%) + Phase 3 prefilter (magnitude_bench 85%) + Phase 4 (magnitude classifier 77%) 완료
**통합 벤치 baseline**: **88% (23/26)** — [`baseline_converter.md`](../../data/listener_perspective/results/baseline_converter.md)

## 1. 목적

tests/ 에 격리되어 있는 listener-perspective 변환 엔진 구성요소를 `src/domain/listener_perspective/` 모듈로 승격하고, 프로덕션 `MindService`가 `apply_stimulus` 이전 단계에서 자연스럽게 사용할 수 있도록 `ListenerPerspectiveConverter` trait을 설계한다.

## 2. 이관 완료 상태 (Step 1-3, 2026-04-19)

| 구성요소 | 위치 | 역할 | 상태 |
|---|---|---|---|
| Prefilter 엔진 | `src/domain/listener_perspective/prefilter.rs` | 4 카테고리 정규식 매칭 | ✅ Step 1 |
| Prefilter 패턴 | `data/listener_perspective/prefilter/patterns.toml` | 카테고리 정의 | 변경 없음 |
| Sign/Magnitude 프로토타입 | `data/listener_perspective/prototypes/` | k-NN 분류용 | 변경 없음 |
| 공통 프로토타입 로더 | `src/domain/listener_perspective/prototype.rs` | TOML → PrototypeSet (group 검증) | ✅ Step 2 |
| k-NN 수학 | `src/domain/listener_perspective/classifier.rs` | cosine_sim + top_k_mean_sorted | ✅ Step 2 |
| Sign 분류기 | `src/domain/listener_perspective/sign_classifier.rs` | 2-way k-NN top-3 | ✅ Step 2 |
| Magnitude 분류기 | `src/domain/listener_perspective/magnitude_classifier.rs` | 3-way k-NN top-3 | ✅ Step 2 |
| 계수 테이블 | `src/domain/listener_perspective/magnitude_coef.rs` | MagnitudeCoefTable + MagnitudeBinThresholds | ✅ Step 3 |
| Converter trait + 기본 구현 | `src/domain/listener_perspective/converter.rs` | prefilter + sign + magnitude 조합 | ✅ Step 3 |
| 통합 벤치 | `tests/listener_perspective_integration_bench.rs` | 엔드투엔드 회귀 감시 (88%) | ✅ Step 3 |

**feature flag**: `listener_perspective` (default off, 회귀 방어). 도메인 단위 테스트 **39개** 전수 통과.

**회귀 감시**: 기존 4개 벤치 수치 유지 확인 완료 (sign 81%, magnitude classifier 77%, magnitude 변환식 85%, prefilter_unit 10/10).

**남은 작업**: Step 4 (MindService 통합) + Step 5 (feature flag 기본 on + tests/common/prefilter.rs 제거).

## 3. 타겟 구조

```
src/domain/listener_perspective/
├── mod.rs              # pub 재수출, 공용 타입
├── converter.rs        # ListenerPerspectiveConverter trait + 기본 구현
├── prefilter.rs        # Prefilter 엔진 (tests/common에서 이관)
├── sign_classifier.rs  # k-NN sign 분류기
├── magnitude_classifier.rs  # k-NN magnitude 분류기
├── magnitude_coef.rs   # Calibration 계수 테이블 (§3.1.2)
└── types.rs            # Sign / Magnitude / PrototypeSet / ConvertResult 등
```

### 3.1 핵심 trait — `ListenerPerspectiveConverter`

```rust
use crate::domain::pad::Pad;

/// 청자 관점 PAD 변환기
///
/// 화자 PAD(PadAnalyzer 결과)를 받아 청자가 체감하는 PAD로 변환한다.
pub trait ListenerPerspectiveConverter: Send + Sync {
    /// 발화 텍스트와 화자 PAD → 청자 PAD
    ///
    /// 내부 동작(예시):
    /// 1. Prefilter 매칭 시도 → hit면 (sign, magnitude, p_s_default) 직접 결정
    /// 2. miss면 sign k-NN → magnitude k-NN
    /// 3. 변환식: P_L = sign × coef[magnitude] × P_S
    fn convert(
        &self,
        utterance: &str,
        speaker_pad: &Pad,
    ) -> Result<ConvertResult, ConvertError>;
}

pub struct ConvertResult {
    pub listener_pad: Pad,
    pub meta: ConvertMeta,
}

pub struct ConvertMeta {
    pub path: ConvertPath,         // Prefilter { category } | Classifier { sign, magnitude }
    pub sign: Sign,
    pub magnitude: Magnitude,
    pub margin: f32,               // 분류기 신뢰도 (classifier 경로 시)
}
```

### 3.2 기본 구현 — `EmbeddedConverter`

```rust
pub struct EmbeddedConverter {
    prefilter: Prefilter,
    sign_classifier: SignClassifier,       // k-NN top-3, 2-way
    magnitude_classifier: MagnitudeClassifier,  // k-NN top-3, 3-way
    coef_table: MagnitudeCoefTable,        // weak=0.5, normal=1.0, strong=1.5
}

impl EmbeddedConverter {
    pub fn new(
        embedder: Box<dyn TextEmbedder + Send>,
        prefilter_path: &Path,
        sign_keep_path: &Path,
        sign_invert_path: &Path,
        magnitude_weak_path: &Path,
        magnitude_normal_path: &Path,
        magnitude_strong_path: &Path,
    ) -> Result<Self, ConvertError>;
}

impl ListenerPerspectiveConverter for EmbeddedConverter {
    fn convert(&self, utterance: &str, speaker_pad: &Pad) -> Result<ConvertResult, ConvertError> {
        // 1. Prefilter 우선 시도
        if let Some(hit) = self.prefilter.classify(utterance) {
            return Ok(self.apply_prefilter(hit, speaker_pad));
        }
        // 2. k-NN 분류
        let sign = self.sign_classifier.classify(utterance)?;
        let magnitude = self.magnitude_classifier.classify(utterance)?;
        Ok(self.apply_classifier(sign, magnitude, speaker_pad))
    }
}
```

### 3.3 `apply_stimulus` 파이프라인 변경

**Before (현재)**:
```
speaker_utterance → PadAnalyzer → speaker_pad → apply_stimulus(speaker_pad)
```

**After (Phase 7)**:
```
speaker_utterance → PadAnalyzer → speaker_pad
                → ListenerPerspectiveConverter → listener_pad → apply_stimulus(listener_pad)
```

## 4. 이관 순서 (Stepped Migration)

### Step 1 — `src/domain/listener_perspective/` 모듈 생성 ✅ 완료

1. `types.rs`: Sign, Magnitude enum 이관
2. `prefilter.rs`: `tests/common/prefilter.rs` 로직 → 도메인 이관
3. 단위 테스트: 도메인 내 `#[cfg(test)]` 작성 (types 5, prefilter 6 = 11개)

**회귀 감시 결과**: 기존 4개 벤치 모두 수치 유지 ✅

### Step 2 — 분류기 이관 ✅ 완료

1. `prototype.rs`: 공통 TOML 로더 신설 (group 검증, 빈 items 거부)
2. `classifier.rs`: 순수 수학 함수 (cosine_sim, top_k_mean_sorted)
3. `sign_classifier.rs`: 2-way k-NN, Embedder 주입 + 프로토 임베딩 내장
4. `magnitude_classifier.rs`: 3-way k-NN, 동점 시 weak 우선
5. `ListenerPerspectiveError` 5 variant 추가 (Prototype*, Embed, EmptyPrototypes)
6. 도메인 단위 테스트 +22개 (누적 33개)

**회귀 감시 결과**: sign 81% / magnitude classifier 77% / magnitude 85% / prefilter 10/10 유지 ✅

### Step 3 — Converter trait 및 EmbeddedConverter 구현 ✅ 완료 (88%)

1. `magnitude_coef.rs`: MagnitudeCoefTable + MagnitudeBinThresholds
2. `converter.rs`: ListenerPerspectiveConverter trait + EmbeddedConverter
3. ConvertResult/ConvertMeta/ConvertPath: 경로(Prefilter|Classifier) + margin 보존
4. `with_coef_table()` builder (주입 가능 계수 테이블)
5. `convert_from_text()` 편의 메서드 (내부 embedding)
6. 도메인 단위 테스트 +6개 (누적 39개)
7. 신규 통합 벤치: `tests/listener_perspective_integration_bench.rs`
   - A안 `result.meta.magnitude == expected` 주 판정 + C안 `bin(|P_L|)` 병기

**결과**: 88% (23/26) — 기존 magnitude_bench 85% 대비 **+3%p**. 
Prefilter 7/7 (100%) + Classifier 16/19 (84%). meta vs bin 불일치 5건이 이중 판정 구조의 디버깅 가치 실증.

baseline: [`baseline_converter.md`](../../data/listener_perspective/results/baseline_converter.md)

### Step 4 — MindService 통합 ⏳ 대기

1. `application/mind_service.rs`: `ListenerPerspectiveConverter` 의존성 추가
2. `apply_stimulus` 호출 전 `convert()` 호출 삽입
3. feature flag `listener_perspective` 으로 선택적 활성화 (옵트인)
4. 기존 scenario 테스트 회귀 없음 확인

### Step 5 — 기본 활성화 및 문서화 ⏳ 대기

1. feature flag 기본 on 전환
2. 마이그레이션 가이드 작성
3. userMemories / 설계 문서 갱신

## 5. 주요 설계 결정

### 5.1 Embedder 소유 구조 ✅ 옵션 C 채택 (Step 2)

`EmbeddedConverter` 는 `Box<dyn TextEmbedder + Send>` 를 **이관 혹은 공유**? 

- **옵션 A (소유)**: Converter가 자체 Embedder 인스턴스 보유. PadAnalyzer와 별도 인스턴스 → 메모리·모델 로드 2배
- **옵션 B (공유 Arc<Mutex<_>>)**: PadAnalyzer와 Converter가 같은 Embedder 공유. 동시성 주의
- **옵션 C ✅ 채택**: 초기화 시 프로토타입 임베딩을 Vec<Vec<f32>> 로 생성하고 Converter 는 임베딩 캐시만 보유. 발화 임베딩은 호출자가 `&[f32]` 로 넘김.

**최종 결정 (2026-04-19, Step 2 구현 시 확정)**:
- SignClassifier / MagnitudeClassifier `new(embedder: &mut dyn TextEmbedder, …)` 에서 프로토타입 임베딩만 계산 → 내부 Vec<Vec<f32>> 보유
- 런타임 `classify(utterance_embedding: &[f32])` 는 TextEmbedder 의존 없음 → Send + Sync 쉬움
- `EmbeddedConverter::convert(utterance, speaker_pad, utterance_embedding)` 동일 패턴 — 호출자가 PadAnalyzer 결과 임베딩을 재사용 가능
- 편의 메서드 `convert_from_text(..., &mut dyn TextEmbedder)` 도 제공 (내부 embedding)

### 5.2 Embedder 호출 횟수 최적화

발화당 필요한 임베딩: **1회** (발화 텍스트만)
- Prefilter: 정규식, 임베딩 불필요
- Sign k-NN: 발화 임베딩 × 28개 프로토타입 코사인
- Magnitude k-NN: 동일 발화 임베딩 × 38개 프로토타입 코사인

**구현 시 주의**: `convert()` 내부에서 Embedder 호출 1회만 수행하도록 구조.

### 5.3 Prefilter 우선순위의 의미

Prefilter hit 시 k-NN 생략. 이유:
- Prefilter 정확도 100% (Phase 3 검증)
- k-NN 호출 생략으로 레이턴시 감소
- 단, hit 시에도 **margin 메타는 null** 로 반환

### 5.4 실패 경로 설계

분류기가 양쪽 점수 모두 threshold 미만인 경우?
- Phase 1 bench는 threshold 없이 "상대적 최대값" 채택
- Phase 7 에서도 동일 — fallback 없음, 무조건 최대 점수 카테고리 선택
- 단, `margin < 0.02` 이면 로그 warning (Phase 1 근접 실패 패턴 관찰용)

### 5.5 PadAnalyzer 와의 관계

`PadAnalyzer::analyze(utterance) → speaker_pad` 는 **변경 없음**.
`ListenerPerspectiveConverter::convert(utterance, speaker_pad) → listener_pad` 는 **새 단계**.

두 단계 분리 이유:
- `speaker_pad` 는 guide 생성 / 대화 로그에 여전히 유용
- `listener_pad` 는 `apply_stimulus` 직전에만 필요
- 테스트 가능성: 두 단계 독립 측정

### 5.6 설정 로딩 규약

모든 프로토타입/패턴 TOML 경로는 `ListenerPerspectiveConfig` 구조로 주입:

```rust
pub struct ListenerPerspectiveConfig {
    pub prefilter_path: PathBuf,
    pub sign_keep_path: PathBuf,
    pub sign_invert_path: PathBuf,
    pub magnitude_weak_path: PathBuf,
    pub magnitude_normal_path: PathBuf,
    pub magnitude_strong_path: PathBuf,
    pub coef_table: MagnitudeCoefTable,
}

impl Default for ListenerPerspectiveConfig {
    fn default() -> Self {
        // data/listener_perspective/... 기본 경로
    }
}
```

## 6. 회귀 감시 전략

Phase 7 Step별 종료 조건:

| Step | 회귀 감시 |
|---|---|
| 1 | prefilter 단위 테스트 10/10 통과 유지 |
| 2 | sign_classifier_bench 81% 유지, magnitude_classifier_bench 81% 유지 |
| 3 | 신규 integration bench — Phase 3 magnitude_bench (96%) 재현 |
| 4 | scenario 기반 통합 테스트 — 기존 시나리오 결과 변동 없음 |
| 5 | feature flag 기본 on — 프로덕션 호출 무회귀 |

**핵심 원칙**: 이관 과정에서 벤치 수치가 떨어지면 즉시 중단, 원인 분석.

## 7. 열린 질문

### Q1 — Embedder 공유 메커니즘 ✅ 답변됨 (Step 2)

**옵션 C 채택**. 분류기들은 초기화 시에만 Embedder 를 받아 프로토타입 임베딩을 내부에 보유. 런타임에는 TextEmbedder 의존 없음. 발화 임베딩은 호출자가 계산해 `&[f32]` 로 주입해 PadAnalyzer 결과와 공유 가능. Send + Sync 자연스러움.

### Q2 — 현대어 register 감지
Phase 5/6 예정이지만, Phase 7 에서 feature 플레이스홀더만 우선 정의?
- ConvertMeta 에 `register: RegisterHint` 필드 추가
- 현재는 항상 `RegisterHint::Wuxia` 반환
- Phase 6 에서 실제 감지 로직 구현 시 필드 활용

### Q3 — 벤치 스크립트 이관 ✅ 답변됨 (Step 3)

**레거시 유지 + 신규 integration bench 병행** 채택.
- `sign_classifier_bench.rs` / `magnitude_classifier_bench.rs` / `magnitude_bench.rs` 그대로 유지 — Phase 1·3·4 회귀 감시용
- 신규 `listener_perspective_integration_bench.rs` 는 Converter 엔드투엔드 회귀 감시 (88%)
- `tests/common/prefilter.rs` 도 이중 구현으로 유지 — Step 4/5 완료 후 제거 예정

### Q4 — 에러 처리
Converter 실패 시 apply_stimulus 어떻게?
- Fallback: `listener_pad = speaker_pad` (변환 실패 시 화자 PAD 그대로)
- 또는 에러 전파하여 stimulus 자체 skip
- **권장**: Fallback. 감정 시스템이 깨져도 서비스가 죽으면 안 됨

## 8. 작업량 추적

| Step | 계획 | 실제 (2026-04-19) | 난이도 |
|---|---|---|---|
| 1. prefilter 이관 | 0.5일 | **완료** | 낮음 — 순수 로직 이동 |
| 2. 분류기 이관 | 1일 | **완료** | 중간 — thiserror source 메타 버그, Debug derive 이슈 두 라운드 |
| 3. Converter trait + 통합 bench | 1.5일 | **완료 (88%)** | 중상 — A안/B안/C안 결정, bin 병기 실측 |
| 4. MindService 통합 | 1일 | 미착수 | 중간 — application 계층 침습 |
| 5. 기본 활성화 + 문서 | 0.5일 | 미착수 | 낮음 |
| **Step 1-3 합계** | **3일** | **단일 세션 (~수 시간)** | — |

예상보다 짧게 걸린 이유:
- 명확한 설계 문서 (이 문서 + sign-classifier-design.md §3.1·3.7) 사전 정리 효과
- Q1/Q2/Q3 3개 설계 결정을 사전 확정 후 작업 시작
- 회귀 감시 벤치가 이미 4개 존재해 안전망 역할

## 9. 참고

- Phase 1~4 설계 및 벤치 결과: [`sign-classifier-design.md`](./sign-classifier-design.md)
- 현재 baseline: [`../../data/listener_perspective/results/baseline_magnitude_classifier.md`](../../data/listener_perspective/results/baseline_magnitude_classifier.md)
- TextEmbedder 포트: `src/ports.rs:105-114`
- UtteranceAnalyzer 포트: `src/ports.rs:155-162`
- PadAnalyzer 구현: `src/domain/pad.rs:194-`
