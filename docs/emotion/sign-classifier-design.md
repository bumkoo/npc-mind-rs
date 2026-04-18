# 부호 축 분류기 설계 — Listener-perspective 변환 (임베딩 기반)

**상태**: Phase 1 완료 (baseline 확정) · Phase 2 대기
**날짜**: 2026-04-18
**작성자**: Bekay + Claude
**관련 문서**: [`adr-pad-v2-redesign.md`](adr-pad-v2-redesign.md) (LLM 기반 PAD 추출, 장기 방향)

## Phase 1 완료 요약 (2026-04-18)

- 프로토타입 v2: sign_keep 14개, sign_invert 14개
- 벤치: 전체 81% (21/26), easy 100%, medium 80%, hard 50%
- Baseline: [`../../data/listener_perspective/results/baseline.md`](../../data/listener_perspective/results/baseline.md)
- 잔여 실패: 019/020 (Phase 후속 sarcasm 보강), 022/023 (Phase 3 정규식), 024 (체념 — 설계 범위 밖)

---


## 1. 배경

현재 `PadAnalyzer`는 발화를 **화자 톤(speaker tone)** 으로 PAD 추출한다.
그러나 `apply_stimulus`는 그 PAD를 **청자가 받는 자극**으로 그대로 사용한다.

이 둘은 일반적으로 다르다:

- 진심어린 사과 → 화자 P−, 청자 P+ (부호 반전)
- 빈정거림 → 화자 P+, 청자 P− (부호 반전 + 증폭)
- 위협 → 화자 P−, 청자 P− 이지만 청자가 더 크게 느낌 (부호 유지 + 증폭)

화자 톤을 그대로 청자 자극에 꽂으면 감정 변동이 뒤집히거나 과소/과대평가된다.

이 문서는 **임베딩 기반**으로 화자→청자 PAD를 변환하는 첫 구현 설계를 다룬다.
LLM 기반 추출(ADR v2)은 장기 방향이며, 이 문서는 그 fallback 경로를 먼저 정교화하는 실험이다.

---

## 2. 결정

### 2.1 변환의 본질을 "축 분류 문제"로 정의

Listener-perspective 변환은 두 축으로 분해된다:

| 축 | 목적 | 값 |
|----|------|-----|
| **부호 축 (sign)** | 청자 P 부호가 화자와 같은가 반대인가 | `keep` / `invert` |
| **강도 축 (intensity)** | 청자가 체감하는 강도가 증폭되는가 | `strong` / `normal` |

두 축의 조합으로 4그룹 × 5 변환 패턴이 나온다 (상세 §3.1).

**이 분류는 P축에만 적용된다.** A축과 D축의 변환식은 부호 축 결과로부터 파생된다 (§3.1.1 참조).

**이 문서의 Phase 1은 부호 축만 다룬다.** 강도 축은 부호 축 검증 이후.

### 2.2 분류 방법

**임베딩 기반 k-NN top-k 분류.** 상세는 §3.2.

정규식 프리필터(한국어 형식 마커 기반)와 fallback 규칙은 Phase 1 결과 이후 추가 검토.

### 2.3 검증 방법

정적 벤치마크. 라벨링된 테스트 케이스 TOML로 분류 정확도 측정.
벤치 결과는 Markdown 리포트로 날짜별 저장, 반복 튜닝에 활용 (§4).

---

## 3. 상세 설계

### 3.1 4그룹 변환 패턴

화자 PAD를 청자 PAD로 변환할 때, 4그룹이 다음과 같이 작동한다.

| 그룹 | 부호 | 강도 | 대표 화행 | P 변환식 (예시) |
|------|-----|------|----------|----------------|
| `keep_normal` | 유지 | 보통 | 감사, 칭찬, 중립 단언 | `P_L = P_S × 1.0` |
| `keep_strong` | 유지 | 강함 | 비난, 위협 | `P_L = P_S × 1.3` |
| `invert_normal` | 반전 | 감쇄 | 사과, 간청, 위로 | `P_L = −P_S × 0.4` |
| `invert_strong` | 반전 | 증폭 | 빈정거림, 조롱 | `P_L = −P_S × 1.2` |

**주의 — 변환식의 구체 계수는 Phase 2 대상.**
Phase 1은 그룹 **분류**만 검증하며, 실제 P 변환값은 측정하지 않는다.

#### 3.1.1 P축 중심 설계의 근거

부호 축 분류기는 **P축 변환만을 직접 결정**한다. A축과 D축은 별도의 부호 분류기가 필요하지 않다.

**P축 — 부호 축 필수**
화자와 청자의 P 부호가 뒤집히는 케이스가 빈번하다 (사과, 빈정, 위로).
`pad_dot` 내적에 직접 관여하므로 부호가 틀리면 감정 변동 방향 자체가 뒤집힌다.
다른 어떤 레이어(성격, 관계)로도 복구 불가능한 결정적 오류.

**A축 — 대부분 identity 변환**
화자와 청자의 각성 방향은 일반적으로 같다 — 흥분된 발화는 청자도 긴장시키고, 차분한 발화는 청자도 차분해진다.
**부호 반전 케이스가 실질적으로 없다.**
예외(차분한 위협 등 화자 A 낮은데 청자 A 높음)는 부호 문제가 아니라 **강도 조절 문제**로, 그룹별 A 계수 보정으로 처리 가능.

**D축 — 부호 반전의 실효성 없음**
화자와 청자의 D 부호는 자연스럽게 반대(화자 명령 D+ / 청자 위축 D−).
그러나 현재 `pad_dot` 공식은 D축을 내적 항이 아닌 **격차 스케일러**(`1.0 + |ΔD| × 0.3`)로 사용한다.
격차 절대값만 의미를 가지므로 **D 부호를 뒤집어도 스케일러 결과는 동일**하다.
참조: [`pad-stimulus-design-decisions.md`](pad-stimulus-design-decisions.md)

**결론**
- Phase 1 부호 축 분류기는 **P축 전용**
- A/D축은 Phase 2의 그룹별 변환식 계수로 흡수 (예: `keep_strong` 그룹이면 A × 1.3, `invert_normal` 그룹이면 A × 0.5)
- 하나의 부호 축 분류 결과로 P/A/D 전 축 변환식이 모두 파생됨

#### 3.1.2 축별 변환식 구조 (Phase 2 예정)

| 축 | 부호 축 분류기 필요? | 변환 방식 |
|----|--------------------|---------|
| P | **필수** | 부호 축 라벨로 부호 결정 + 그룹별 스케일 |
| A | 불필요 | 그룹별 스케일 계수만 (identity 기본) |
| D | 불필요 | 그룹별 \|D\| 스케일 조정 (부호 유지) |

### 3.2 부호 축 분류기 — k-NN top-k

**입력**: 발화 텍스트 (화자 발화)

**초기화 시 1회**:
1. `sign_keep.toml` 프로토타입 텍스트들을 임베딩
2. `sign_invert.toml` 프로토타입 텍스트들을 임베딩
3. 벡터로 메모리 상주

**런타임 분류 (발화당 1회)**:
1. 발화 텍스트를 임베딩
2. `keep` 프로토타입 각각과 cosine 유사도 계산 → top-k 평균 = `keep_score`
3. `invert` 프로토타입 각각과 cosine 유사도 계산 → top-k 평균 = `invert_score`
4. 점수가 높은 쪽을 predicted_sign으로 판정
5. `margin = |keep_score − invert_score|` 를 신뢰도 지표로 기록

**k 값**: `k=3`을 시작값으로 사용.
프로토타입이 10개라면 상위 30%만 반영 → 단일 최대값의 노이즈와 전체 평균의 희석을 동시에 피함.

**centroid 평균이 아닌 이유**:
같은 그룹 내에서도 subtype(gratitude/praise/criticism/threat/...)이 다르면 임베딩 위치가 흩어짐.
centroid는 흩어진 점들의 중심이 되어 변별력이 희석되지만, top-k는 **가장 가까운 몇 개의 프로토타입**만 본다.

### 3.3 프로토타입 큐레이션 5원칙

1. **명확한 사례만.** 경계/혼합 발화는 프로토타입이 아니라 테스트 케이스로.
2. **그룹 내 다양성.** subtype을 균형 있게 배치. 같은 subtype에 몰리면 임베딩 공간이 좁아짐.
3. **톤 통일.** 1인칭 대사, 무협체로 시작. 기존 PAD 앵커와 동일한 원칙.
4. **수집 가능성 보장.** `source` 필드 기록, `version` 증분 관리, 실패 케이스 흡수 루프.
5. **형식 대칭 배치.** 같은 문장 형식이 두 그룹에 공존할 수 있는 경우, 양쪽에 의도적으로 프로토타입을 배치.

#### 3.3.1 형식 대칭이 필요한 케이스

**질문 형식**
비난·위협도 질문형으로 발화 가능 — "네놈이 감히 나를 농락하느냐!", "이 자리에서 죽고 싶으냐?"
감사·반가움도 질문형 — "그간 무고하셨소?", "오랜만이오, 반갑소!"
→ `sign_keep`에 **질문형 비난/위협** 프로토타입 최소 1~2개 포함.
누락 시 "제정신이냐?" 같은 질문형 비난이 감사형 질문 프로토타입과 혼동될 위험.

**반복 강조 형식**
진심 칭찬 — "잘했소, 참으로 잘했소"
빈정 조롱 — "그래, 잘났다 잘났어"
→ `sign_keep`에 **진심 반복 강조** 프로토타입, `sign_invert`에 **빈정 반복 강조** 프로토타입을 대칭 배치.
빈정 쪽에는 "그래," 같은 시작 마커를 의도적으로 포함시켜 분리 보조.

**체념 형식**
진심 수용 — "좋다, 그대 말이 옳다"
분노 후 내던짐 — "좋다, 네 뜻대로 해라 (관계 파탄)"
→ 체념 형식은 맥락 없이 결정 불가. **프로토타입에서 배제, 테스트 케이스로만** 처리.

### 3.4 Phase 1 스코프에서 제외한 것

- **강도 축 분류기** — 부호 축 안정화 후
- **한국어 형식 마커 정규식 프리필터** — k-NN 단독 정확도 측정 후 추가
- **현대어 register** — 무협체로 시작. 플레이어 발화는 후속 확장
- **fallback 임계값 튜닝** — 초기 벤치 결과로 임계값 경험적 결정
- **프로덕션 port 통합** — 벤치 단독으로 검증 완료 후

---

## 4. 파일 구조

### 4.1 디렉토리 레이아웃

```
data/
└── listener_perspective/
    ├── prototypes/
    │   ├── sign_keep.toml       # 부호 유지 프로토타입
    │   └── sign_invert.toml     # 부호 반전 프로토타입
    ├── testcases/
    │   └── sign_benchmark.toml  # 라벨링된 테스트 케이스
    └── results/
        ├── baseline.md          # 기준선 (커밋)
        └── YYYY-MM-DD_runNN.md  # 런별 로그 (gitignore)

tests/
└── sign_classifier_bench.rs     # 벤치 러너

docs/emotion/
└── sign-classifier-design.md    # 이 문서
```

### 4.2 `.gitignore`

```
data/listener_perspective/results/*.md
!data/listener_perspective/results/baseline.md
```

### 4.3 프로토타입 TOML 스키마

```toml
[meta]
language = "ko"
register = "wuxia"           # Phase 1: 무협체만
version = "1"
group = "sign_keep"          # 또는 "sign_invert"
last_updated = "2026-04-18"

[prototypes]
items = [
    { text = "...", subtype = "gratitude", source = "created_by_bekay" },
    # ...
]
```

**필드 설명**:
- `text` — 발화 원문 (1인칭 대사, 무협체)
- `subtype` — 화행 세부 유형 (gratitude/praise/criticism/threat/apology/plea/condolence/sarcasm/assertion)
- `source` — 출처 추적 (`created_by_bekay` / `v2_patch` / `scenario:path/to/turn` / `failed_case:run_id#case_id`)

**규모 지침**:
- 그룹당 초기 8~12개
- subtype당 2~3개 균형
- `sign_keep` ∩ `sign_invert` = ∅ (중복 금지)

### 4.4 테스트 케이스 TOML 스키마

```toml
[meta]
language = "ko"
register = "wuxia"
version = "1"
last_updated = "2026-04-18"

[[case]]
id = "001"
utterance = "..."
label = "진심 사과"
expected_sign = "invert"                # "keep" | "invert"
speaker_p_sign = "negative"             # 화자 톤의 P 부호 (Phase 2 검증용)
listener_p_sign = "positive"            # 청자 체감 P 부호 (Phase 2 검증용)
difficulty = "easy"                     # "easy" | "medium" | "hard"
subtype = "apology"
notes = "사과의 전형. 프로토타입 near-duplicate 검증."
```

**hard 난이도 필수 커버리지**:
- **반어법 / 빈정** — "그래, 잘났다 잘났어"
- **복합 절 / 마커와 본문 충돌** — "미안하지만 안 돼"
- **체념 표현** — "좋다, 네 뜻대로 해라"
- **짧은 감탄사** — "허, 참..."

hard 목표 정확도(70%)는 이 케이스들을 전부 맞추라는 뜻이 아니며, 오분류 패턴 관찰로 Phase 3 설계 근거를 수집하는 용도.

---

## 5. 벤치 결과 포맷

### 5.1 YAML front-matter

```yaml
---
run_id: "2026-04-18_run01"
prototype_keep_version: "1"
prototype_invert_version: "1"
benchmark_version: "1"
classifier: "knn-top3"
overall_accuracy: 0.83
---
```

### 5.2 본문 섹션

**요약** — 전체/난이도별/부호별/subtype별 통과율
**실패 케이스 상세** — `id | 난이도 | 발화 | 기대 | 예측 | 점수차 | 노트`
**점수차 분포** — margin 4구간 bucket + 구간별 통과율

### 5.3 목표 정확도

| 카테고리 | 목표 |
|---|---|
| 전체 | 80% 이상 |
| easy | 95% 이상 |
| hard | 70% 이상 |

---

## 6. 워크플로우

```
1. cargo test --features embed --test sign_classifier_bench -- --nocapture
   → results/YYYY-MM-DD_runNN.md 생성

2. 실패 케이스 분석:
   - 점수차 작음 → 프로토타입 추가 → version++
   - 점수차 큼   → 프로토타입 설계 재검토

3. 재실행 → runNN+1 리포트 생성

4. git diff baseline.md YYYY-MM-DD_runNN.md

5. 목표 달성 시 baseline.md 갱신
```

---

## 7. Phase 로드맵

| Phase | 작업 | 상태 |
|-------|------|------|
| **P1** | 부호 축 k-NN 분류기 + 벤치 구조 | **✅ 완료 (2026-04-18, 81%)** |
| **P2** | P축 변환식 계수 튜닝 | 대기 (`listener_p_sign` 라벨 활용) |
| **P3** | 정규식 프리필터 추가 | 후속 (복합 절 022/023 해결 목표) |
| **P4** | 강도 축 분류기 | 후속 (4그룹 완성, A/D축 변환) |
| **P5** | 현대어 register 추가 | 후속 (플레이어 발화 대응) |
| **P6** | Relationship modulation | 후속 (trust/closeness 변조 레이어) |
| **P7** | 프로덕션 port 통합 | 후속 (`ListenerPerspectiveConverter` trait) |

---

## 8. 트레이드오프

### 채택 이유

- **임베딩 기반**: LLM 호출 없이 로컬 결정론적 처리
- **k-NN top-k**: 프로토타입 다양성 유지. 학습 없이 데이터 추가만으로 개선
- **부호 축 단독 Phase 1**: 한 번에 한 변수. 디버깅 경로 단순화

### 리스크

| 리스크 | 완화 |
|--------|------|
| 빈정거림 등 의도 층위 발화 오분류 | Phase 3 정규식 프리필터 |
| 무협체 한정 — 현대어 미지원 | Phase 5 |
| 프로토타입 편향 | version 관리 + 실패 흡수 루프 |
| BGE-M3의 D축 76% 천장 | Phase 1은 P축만 대상 |

### 열린 질문

1. **k 값 최적**: top-3 시작값. top-1/top-5 비교 필요 가능.
2. **프로토타입 중복 검출 자동화**: 두 TOML 간 텍스트 충돌 컴파일 타임 검출?
3. **margin 임계값**: fallback 발동 기준. v2 결과 margin 분포 참조 (0.20 이상 0건, 0.05 미만 18/26).
4. **관계 전환적 발화의 강도 판정**: "사랑합니다" 류. 부호는 `keep` 명확하나 강도 판정 어려움. Phase 4 진입 시 재검토.
5. **프로토타입 버전 관리 주기**: baseline 재측정 부담과 흡수 루프 속도의 트레이드오프.

---

## 9. Phase 1 실측 결과 (v1 → v2)

| 지표 | v1 | v2 | Δ |
|---|---|---|---|
| 전체 | 65% | **81%** | +16 |
| easy | 80% | **100%** | +20 |
| medium | 70% | 80% | +10 |
| hard | 33% | 50% | +17 |

**v2 패치 내역** (10개 추가):
- `sign_keep`: gratitude +2, praise +2, criticism +2
- `sign_invert`: plea +2, sarcasm +2

**잔여 실패 5건**: 019, 020 (medium sarcasm), 022, 023, 024 (hard — 설계상 예상)

---

## 10. 참고

- v1 PAD 설계: [`pad-stimulus-design-decisions.md`](pad-stimulus-design-decisions.md)
- LLM 기반 PAD (장기 방향): [`adr-pad-v2-redesign.md`](adr-pad-v2-redesign.md)
- 기존 PAD 벤치마크: [`pad-anchor-score-matrix.md`](pad-anchor-score-matrix.md)
- Baseline: [`../../data/listener_perspective/results/baseline.md`](../../data/listener_perspective/results/baseline.md)
- `stimulus_absorb_rate` 구현: `src/domain/personality.rs`
- `StimulusEngine::apply_stimulus`: `src/domain/emotion/stimulus.rs`
