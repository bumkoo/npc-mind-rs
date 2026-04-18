# Phase 7 설계 — Listener-perspective Converter 프로덕션 통합

**상태**: 설계 중 (2026-04-19)
**관련 문서**: [`sign-classifier-design.md`](./sign-classifier-design.md) §3.1~§3.7
**전제**: Phase 1 (sign 81%) + Phase 3 (prefilter 96%) + Phase 4 (magnitude 81%) 완료

## 1. 목적

tests/ 에 격리되어 있는 listener-perspective 변환 엔진 구성요소를 `src/domain/listener_perspective/` 모듈로 승격하고, 프로덕션 `MindService`가 `apply_stimulus` 이전 단계에서 자연스럽게 사용할 수 있도록 `ListenerPerspectiveConverter` trait을 설계한다.

## 2. 현재 상태 (tests/ 내 격리)

| 구성요소 | 위치 | 역할 |
|---|---|---|
| Prefilter 엔진 | `tests/common/prefilter.rs` | 4 카테고리 정규식 매칭 |
| Prefilter 패턴 | `data/listener_perspective/prefilter/patterns.toml` | 카테고리 정의 |
| Sign 프로토타입 | `data/listener_perspective/prototypes/sign_{keep,invert}.toml` | k-NN 부호 분류 |
| Magnitude 프로토타입 | `data/listener_perspective/prototypes/magnitude_{weak,normal,strong}.toml` | k-NN 강도 분류 |
| Sign 분류기 | `tests/sign_classifier_bench.rs` (벤치 내부) | k-NN top-3 |
| Magnitude 분류기 | `tests/magnitude_classifier_bench.rs` (벤치 내부) | k-NN top-3 |
| 변환식 | `tests/magnitude_bench.rs` (벤치 내부) | sign × coef × P_S |

**문제**: 분류기 로직이 `#[cfg(test)]` 스코프 안에 있어 `MindService` 가 호출 불가.

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

### Step 1 — `src/domain/listener_perspective/` 모듈 생성

1. `types.rs`: Sign, Magnitude enum 이관
2. `prefilter.rs`: `tests/common/prefilter.rs` 로직 → 도메인 이관
3. 단위 테스트: `tests/prefilter_unit.rs` 도메인 테스트로 승격

**회귀 감시**: `tests/magnitude_bench.rs` 와 `tests/sign_classifier_bench.rs` 가 여전히 통과해야 함.

### Step 2 — 분류기 이관

1. `sign_classifier.rs`: 현재 bench 내부 로직 → 도메인 이관
2. `magnitude_classifier.rs`: 동일
3. `magnitude_coef.rs`: 계수 테이블 + bin 경계
4. 기존 bench 는 이관된 도메인 모듈 import 하도록 수정

**회귀 감시**: 벤치 수치 그대로 유지 (sign 81%, magnitude 81%, prefilter 96%)

### Step 3 — Converter trait 및 EmbeddedConverter 구현

1. `converter.rs` 신규 작성
2. Prefilter/Sign/Magnitude 조합 로직 구현
3. 신규 통합 벤치: `tests/listener_perspective_integration_bench.rs`
   - 26 케이스 × (speaker_pad + 기대 listener_pad)
   - 현재 Phase 3 bench 결과 96% 가 Phase 7 통합 후에도 재현되는지

### Step 4 — MindService 통합

1. `application/mind_service.rs`: `ListenerPerspectiveConverter` 의존성 추가
2. `apply_stimulus` 호출 전 `convert()` 호출 삽입
3. feature flag `listener_perspective` 으로 선택적 활성화 (옵트인)
4. 기존 scenario 테스트 회귀 없음 확인

### Step 5 — 기본 활성화 및 문서화

1. feature flag 기본 on 전환
2. 마이그레이션 가이드 작성
3. userMemories / 설계 문서 갱신

## 5. 주요 설계 결정

### 5.1 Embedder 소유 구조

`EmbeddedConverter` 는 `Box<dyn TextEmbedder + Send>` 를 **이관 혹은 공유**? 

- **옵션 A (소유)**: Converter가 자체 Embedder 인스턴스 보유. PadAnalyzer와 별도 인스턴스 → 메모리·모델 로드 2배
- **옵션 B (공유 Arc<Mutex<_>>)**: PadAnalyzer와 Converter가 같은 Embedder 공유. 동시성 주의
- **옵션 C (프리컴퓨트만 공유)**: 초기화 시 프로토타입 임베딩을 Vec<Vec<f32>> 로 생성하고 Converter 는 임베딩 캐시만 보유. 발화 임베딩은 매번 Embedder 호출.

**권장: C**. 이유:
- 프로토타입은 초기화 후 불변 → 캐시로 충분
- 발화당 Embedder 1회 호출은 PadAnalyzer 와 동일 비용
- PadAnalyzer가 이미 가지고 있는 Embedder 를 재사용하면 모델 로드 1회

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

### Q1 — Embedder 공유 메커니즘
PadAnalyzer 와 Converter 가 같은 Embedder 인스턴스 공유 시 동시성 처리:
- Mutex — 단순하지만 멀티 스레드 성능 저하
- Per-thread clone — 모델 중복 로드
- **Rwlock** + RwLockReadGuard — 분석은 read-only 라면 적합
실측 후 결정.

### Q2 — 현대어 register 감지
Phase 5/6 예정이지만, Phase 7 에서 feature 플레이스홀더만 우선 정의?
- ConvertMeta 에 `register: RegisterHint` 필드 추가
- 현재는 항상 `RegisterHint::Wuxia` 반환
- Phase 6 에서 실제 감지 로직 구현 시 필드 활용

### Q3 — 벤치 스크립트 이관
이관 후 `tests/magnitude_bench.rs` 는 어떻게 되는가?
- Phase 7 통합 bench 로 대체?
- 또는 **레거시 bench 로 유지** (Phase 3/4 회귀 감시용)?
- **권장**: 레거시 유지, 신규 integration bench 병행

### Q4 — 에러 처리
Converter 실패 시 apply_stimulus 어떻게?
- Fallback: `listener_pad = speaker_pad` (변환 실패 시 화자 PAD 그대로)
- 또는 에러 전파하여 stimulus 자체 skip
- **권장**: Fallback. 감정 시스템이 깨져도 서비스가 죽으면 안 됨

## 8. 예상 작업량 추정

| Step | 예상 소요 | 난이도 |
|---|---|---|
| 1. prefilter 이관 | 0.5일 | 낮음 — 순수 로직 이동 |
| 2. 분류기 이관 | 1일 | 중간 — 배열/벡터 타입 재정의 |
| 3. Converter trait + 통합 bench | 1.5일 | 중상 — 설계 결정 많음 |
| 4. MindService 통합 | 1일 | 중간 — feature flag + 기존 테스트 |
| 5. 기본 활성화 + 문서 | 0.5일 | 낮음 |
| **합계** | **4~5일** | — |

## 9. 참고

- Phase 1~4 설계 및 벤치 결과: [`sign-classifier-design.md`](./sign-classifier-design.md)
- 현재 baseline: [`../../data/listener_perspective/results/baseline_magnitude_classifier.md`](../../data/listener_perspective/results/baseline_magnitude_classifier.md)
- TextEmbedder 포트: `src/ports.rs:105-114`
- UtteranceAnalyzer 포트: `src/ports.rs:155-162`
- PadAnalyzer 구현: `src/domain/pad.rs:194-`
