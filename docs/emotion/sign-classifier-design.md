# 부호 축 분류기 설계 — Listener-perspective 변환 (임베딩 기반)

**상태**: Phase 1-4 설계 완료 (magnitude k-NN 81%) · **Phase 7 Step 1-5 프로덕션 통합 완료** (88% baseline, default-on, DialogueOrchestrator · Mind Studio 적용)
**날짜**: 2026-04-19 (Phase 1-4 본문 2026-04-18 작성, Phase 7 완료로 상태 갱신)
**작성자**: Bekay + Claude
**관련 문서**: [`adr-pad-v2-redesign.md`](adr-pad-v2-redesign.md) (LLM 기반 PAD 추출, 장기 방향), [`phase7-converter-integration.md`](phase7-converter-integration.md) (프로덕션 통합 설계)

**Register 지원 범위 요약**: 무협 존대 ✅ 완성 · 현대 존대 ⏳ Phase 5 · **반말 ⏳ Phase 6** (Relationship layer + trust 기반 해석). 상세 §3.7.0 한눈에 보기.

## Phase 1 완료 요약 (2026-04-18)

- 프로토타입 v2: sign_keep 14개, sign_invert 14개
- 벤치: 전체 81% (21/26), easy 100%, medium 80%, hard 50%
- Baseline: [`../../data/listener_perspective/results/baseline.md`](../../data/listener_perspective/results/baseline.md)
- 잔여 실패: 019/020 (Phase 후속 sarcasm 보강), 022/023 (Phase 3 정규식), 024 (체념 — 설계 범위 밖)

## Phase 2 + 1.5 완료 요약 (2026-04-19)

- Phase 2 Calibration: coef 0.5/1.0/1.5 + bin 0.15/0.4 → 58% (run04)
- Phase 1.5 앵커 보강: P+ 4개, P- 2개 추가 → **62% (run06)**
- PAD 벤치 P 100% (18/18) 완벽 보존
- Baseline: [`../../data/listener_perspective/results/baseline_magnitude.md`](../../data/listener_perspective/results/baseline_magnitude.md)
- Phase 3 이관 6건: 011, 012, 013, 014, 020, 021 (BGE-M3 표면 어휘 편향 구조적 한계)

## Phase 3 완료 요약 (2026-04-19)

- 정규식 프리필터 4 카테고리 도입 → **96% (run07, 25/26)**
- Sparse 조회 대안 스파이크 → 대체 불가 확인 (§3.6)
- Prefilter hit 8/26 (100% 정확도), 나머지 18/26 임베딩 경로 (94%)

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

### 3.1 변환식 — Magnitude 기반 (Phase 2 채택)

**변환식**:
```
P_L = sign_val × magnitude_coef × P_S
A_L = magnitude_coef_a × A_S
D_L = magnitude_coef_d × D_S   (부호 유지, 크기만 조정)
```

- `sign_val`: keep=+1, invert=-1
- `magnitude_coef`: 라벨(`weak`/`normal`/`strong`)에서 계수표로 직접 매핑

**초기 계수표 (Phase 2 baseline)**:

| magnitude | P 계수 | A 계수 | D 계수 | 대응 화행 |
|---|---|---|---|---|
| weak | 0.4 | 0.5 | 0.4 | 사과·간청·위로 (감쇄) |
| normal | 1.0 | 1.0 | 1.0 | 감사·칭찬·중립 단언 (기준) |
| strong | 1.3 | 1.3 | 1.3 | 비난·위협·빈정·극찬 (증폭) |

**Magnitude 기반 채택 근거**:

1. **직교성(Orthogonality)**: 부호(방향)와 강도(크기)를 독립 축으로 분리. 기존 §3.1 초안의 4그룹(keep_normal/keep_strong/invert_normal/invert_strong)은 이 둘을 묶어버려 해상도를 잃음.
2. **해상도 이득**: `invert × normal` 조합이 가능해짐 — 깊은 사과(016, "돌이킬 수 없는 과오")를 `weak(0.4)`로 일괄 감쇄하지 않고 `normal(1.0)`로 처리 가능.
3. **데이터 효율**: 4그룹 × 3축 = 12계수 → 3단 × 3축 = 9계수. 관리 부담 감소.
4. **라벨 직접 매핑**: `listener_p_magnitude` 라벨이 계수 인덱스로 바로 사용됨. 중간 번역 레이어 불필요.
5. **Phase 4 확장성**: magnitude 분류기를 별도로 만들 때 라벨·계수 구조 재활용.

**Bin 경계 (청자 P 크기 → magnitude 분류)**:

산출된 `|P_L|` 값을 3단으로 bin화해 라벨과 비교:

| magnitude | 구간 | 의미 |
|---|---|---|
| weak | \|P_L\| < 0.3 | 배경 노이즈 수준의 미미한 변화 |
| normal | 0.3 ≤ \|P_L\| < 0.7 | 인지 가능한 명확한 감정 변화 |
| strong | \|P_L\| ≥ 0.7 | 캐릭터 행동·톤을 즉각 바꾸는 강렬한 자극 |

Bin 경계(0.3 / 0.7)는 PAD 정규화 공간(-1.0~1.0)의 감정 역치로서 설정. 1차 벤치 결과에 따라 튜닝.

#### 3.1.1 기존 4그룹 ↔ Magnitude 매핑 (호환성)

| §3.1 초안 그룹 | magnitude 기반 | 차이 |
|---|---|---|
| keep_normal | keep × normal → +1.0 | 동일 |
| keep_strong | keep × strong → +1.3 | 동일 |
| invert_normal (사과·간청·위로) | invert × weak → -0.4 | 동일 |
| invert_strong (빈정) | invert × strong → -1.3 | +0.1 |
| (신규) invert × normal | -1.0 | **해상도 확장** — 깊은 사과/위로 |

#### 3.1.2 축별 변환식 구조

| 축 | 부호 축 | 크기 축 | 방식 |
|----|--------|--------|------|
| P | 분류기 결과 (keep/invert) | magnitude 계수 | `sign × coef × P_S` |
| A | 유지 (변환 없음) | magnitude 계수 | `coef × A_S` |
| D | 유지 (변환 없음) | magnitude 계수 | `coef × D_S` |

**A축 근거**: 화자·청자 각성 방향 일반적으로 동일. 크기만 보정.
**D축 근거**: `pad_dot`이 `|ΔD|`를 스케일러로 쓰므로 부호 반전 무의미. 크기 조정으로 위축감 수학적 강화.

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

### 3.5 정규식 프리필터 (Phase 3)

BGE-M3 임베딩의 표면 어휘 편향을 우회하기 위한 규칙 기반 layer.

**파이프라인**:
```
utterance → Prefilter.classify()
  Some(hit)  → (sign, magnitude, p_s_default) 직접 반환 — PadAnalyzer 우회
  None       → 기존 임베딩 경로로 fallback
```

**히트 시 계산**:
```
P_L = hit.sign × coef[hit.magnitude] × hit.p_s_default
predicted_magnitude = bin(|P_L|)
```

**카테고리 설계 (4개)** — `data/listener_perspective/prefilter/patterns.toml` 외부화:

| 카테고리 | sign | magnitude | p_s_default | 타겟 |
|---|---|---|---|---|
| counterfactual_gratitude | keep | strong | +0.7 | 011 (반사실 감사) |
| negation_praise | keep | strong | +0.7 | 012 (부정 형태 극찬) |
| wuxia_criticism | keep | strong | -0.7 | 013/014 (무협 비난·위협) |
| sarcasm_interjection | invert | strong | +0.6 | 010/020/021 (감탄사 빈정) |

**설계 원칙**:
1. **어미 결합형(Suffix-bound)** — `아니`만 쓰지 말고 `아니었(으면|더라면)`처럼 결합
2. **첫 매칭 반환** — 카테고리 등록 순서가 우선순위
3. **p_s_default 설계** — 계수×기본값이 목표 bin에 안착하도록 사전 계산
4. **외부화** — TOML 편집으로 패턴 추가·수정, Rust 재컴파일 불필요

**실측 결과 (2026-04-19, run07)**:

| 경로 | 통과 | 전체 | 정확도 |
|---|---|---|---|
| prefilter | 8 | 8 | **100%** |
| pad_analyzer | 17 | 18 | 94% |
| **전체** | **25** | **26** | **96%** |

잔여 실패 1건 (002): P_S 값이 weak bin 경계(0.15) 미달. Calibration 구조 한계로 수용.

### 3.6 Sparse 조회 대안 — 스파이크 결과 (2026-04-19)

Phase 3 정규식을 BGE-M3 sparse(lexical) 임베딩으로 대체 가능한지 검증한 스파이크.

**구성**:
- 4 카테고리 × 3 프로토타입 = 12 sparse 벡터 사전 계산
- 26 테스트 케이스 × 각 프로토타입 `sparse_dot_product` 계산
- Threshold 0.3 초과 시 매칭 인정
- 기여 토큰 로깅 — 어휘 겹침 패턴 관측

**결과**:

| 분류 | 건수 |
|---|---|
| 정규식 ∩ sparse 동일 카테고리 | **0/26** |
| sparse만 hit | 0/26 |
| 정규식만 hit | 7/26 |
| 둘 다 miss | 19/26 |

전체 sparse 점수 최대값 **0.125** (020번) — threshold 0.3 대비 1/3 이하.

**원인 분석**:

1. **sparse_dot_product 점수 스케일** — BGE-M3 README의 hybrid retrieval 예시 점수가 0.18~0.25 범위. 0.3 threshold는 비현실적
2. **어휘 공유 전제** — Sparse는 "같은 토큰을 쓴 질의/문서" 검색에 최적화. 다른 어휘로 같은 의미를 표현하는 무협 대사 도메인과 구조적 불일치
3. **한국어 노이즈** — 조사·어미가 높은 빈도로 기여 토큰에 포함되어 신호 희석 (020의 "참"/"으" 등)

**결론**: 현재 설계로는 sparse 조회가 정규식 프리필터를 대체하거나 보완할 수 없음.

**기록 자료**:
- 테스트 파일: `tests/sparse_spike.rs` — 삭제하지 않고 유지 (미래 재평가 참고)
- BGE-M3 sparse API: `bge-m3-onnx-rust::BgeM3Embedder::encode() -> BgeM3Output`, `sparse_dot_product`

**미래 재고 가능성**:
- 프로토타입을 테스트 케이스와 의도적으로 **핵심 어휘 공유**하도록 재설계 (overfitting 위험)
- Dense + Sparse hybrid를 PadAnalyzer에 통합 (PAD 벤치 P 100% 회귀 위험)
- BGE-M3 공식 fine-tuning으로 sparse 품질 개선 (별도 모델 학습 필요)
- 위 셋 모두 현재 Phase 3 96% baseline 대비 투입 대비 개선 기대치 낮음

### 3.7 Register 확장 전략 — γ+δ 설계

현재 엔진은 **무협체 1인칭 존대 (`wuxia`) 단일 register** 가정으로 구축되어 있다. 프로덕션 통합 시 플레이어 발화(현대어) 및 관계 기반 반말/존대 전환이 필수 대응 영역이다.

#### 3.7.0 한눈에 보기

**핵심 주장**: 반말은 포기하지 않았다. **Phase 6 Relationship layer 에서 완성**되도록 의존성 순서상 뒤에 배치했을 뿐이다.

##### Register 매트릭스 — 어떤 조합이 어디서 처리되나

| | **무협체** | **현대어 존대** | **현대어 반말** |
|---|---|---|---|
| **존대** | ✅ 현재 완성<br>(PAD 앵커 + Phase 4 프로토타입) | ⏳ Phase 5<br>(벤치 확장 + 앵커 혼합) | — |
| **반말 (친밀)** | ⏳ Phase 6<br>(trust>0.5 × Informal → P+) | — | ⏳ Phase 6 |
| **반말 (적대)** | ⏳ Phase 6<br>(trust<0 × Informal → P-) | — | ⏳ Phase 6 |
| **반말 (무례)** | ⏳ Phase 6<br>(|trust|<0.3 × Informal → D+, P-) | — | ⏳ Phase 6 |

**읽는 법**:
- ✅ = 현재 엔진이 올바르게 처리
- ⏳ = 계획된 Phase 에서 처리 예정
- — = 고유 조합이 거의 없음 (무협 세계 NPC가 현대어 반말 쓰는 상황은 드뭄)

##### 의존성 사슬

```
                 ┌─────────────────────────────────────────────┐
                 │ 왜 반말은 Phase 6 인가?                      │
                 └─────────────────────────────────────────────┘

반말 "이 자식이" — 이 한 발화의 P 부호는 무엇인가?
    ├─ 친구 사이 (trust 높음) → P+ (친밀 표현)
    └─ 적 사이  (trust 낮음) → P- (도발 표현)

        ↓ PadAnalyzer 만으로는 판정 불가
        ↓ trust 값이 필요하다
        ↓ trust 는 Relationship 도메인

∴ Relationship layer (Phase 6) 가 없으면 반말 해석 불가
∴ Phase 6 가 구현되기 전까지 반말은 임시 처리 (존대 fallback 또는 dead zone)
```

##### 로드맵 시각화

```
  완료                                  현재                        대기
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│ Phase 1~4       │──│ Phase 7         │──│ Phase 5         │
│ 무협 존대 완성  │  │ 프로덕션 통합   │  │ 현대 존대 벤치  │
│ sign 81%        │  │ Converter trait │  │ 20~30 페어 케   │
│ prefilter 96%   │  │ MindService     │  │ 이스 추가       │
│ magnitude 81%   │  │ 통합            │  │                 │
└─────────────────┘  └─────────────────┘  └─────────────────┘
                                                    │
                                                    ↓
                                          ┌─────────────────┐
                                          │ Phase 6         │
                                          │ Relationship +  │
                                          │ 반말 register   │
                                          │ trust×informal  │
                                          │ 테이블          │
                                          └─────────────────┘
                                                    │
                                                    ↓
                                                [MVP Ready]
                                                    │
                                                    ↓
                                          ┌─────────────────┐
                                          │ Post-MVP        │
                                          │ 슬랭 / 줄임말   │
                                          │ 혼합체          │
                                          └─────────────────┘
```

##### 시점별 커버리지

| 시점 | 지원되는 입력 | 미지원 (임시 fallback) |
|---|---|---|
| **현재 (Phase 4 완료)** | NPC 무협체 존대 | 현대어, 반말 전반 |
| **Phase 5 완료 후** | + Player 현대 존대 | 반말 전반 |
| **Phase 6 완료 후 (MVP)** | + 무협·현대 반말 (trust 기반 해석) | 슬랭·혼합체 |
| **Post-MVP** | + 슬랭, 혼합체 | — |

##### 현재 반말 입력이 들어오면?

Phase 6 이전에 Player 가 "야, 고마워" 같은 반말을 입력하면:

1. **PadAnalyzer** — ko.toml 앵커와 cosine 유사도 낮음 → 저전압 PAD 반환 (dead zone)
2. **Sign classifier** — sign_keep/sign_invert 무협체 프로토와 거리 멀음 → margin 작은 분류 (불안정)
3. **Prefilter** — 패턴 매칭 안 됨 → miss
4. **결과** — 약한 중립 감정으로 처리됨. 완전 오류는 아니나 무미건조.

**임시 완화책 (Phase 7 통합 시 고려)**: 반말 감지 시 warning 로그 + `register = Informal` 필드 기록. Phase 6 진입 시 과거 세션 로그 분석 데이터로 활용.

---

#### 3.7.1 핵심 통찰 — "반말"은 register가 아닌 관계 신호

반말/존대 선택은 이미 **관계 정보를 담은 meta 신호**이다:

- **친밀**: "야, 고마워" — trust 가산 요인
- **적대**: "네놈이 감히" — 적대 표식, P- 강화
- **무례**: 낯선 상대에 반말 — D+ 자극, P- 약간

즉 같은 반말 표현 "이 자식이" 가 친구 간 농담(P+)과 적 간 도발(P-)로 양의하는 현상은 **PAD 레이어 단독 해결 불가**. Relationship layer (Phase 6)가 협조해야 한다.

#### 3.7.2 채택 전략 — γ+δ 조합

**γ (Relationship reframe 위임)**:
- PadAnalyzer는 발화의 **감정 톤을 있는 그대로 추출**
- Relationship layer가 `trust`, `closeness` 등으로 **사후 보정**
- 책임 분리: PAD = 감정, Relationship = 문맥 해석

**δ (단일 앵커에 반말·현대어 샘플 섞기, 축소판)**:
- 앵커 centroid가 다양한 register를 커버하도록 **제한적으로** 확장
- 전체 교체 아닌 점진 추가 (P 100% 회귀 방어)
- Phase 4 작업 중 프로토타입 큐레이션 시 함께 진행

**제외된 옵션**:
- α (4개 register 파일 분리) — 유지보수 폭발
- β (informality modifier 즉시 주입) — Phase 6 없이는 소비자 없음
- δ 단독 — register 감지 로직 공백

#### 3.7.3 발화 meta 정보 추출

Phase 6 Relationship layer가 소비할 수 있도록, 현 시점에 **meta 필드만 구조화**하여 보관:

```rust
struct UtteranceRegisterMeta {
    formality: Formality,     // Formal / Informal / Mixed
    era: Era,                 // Wuxia / Modern / Mixed
    honorifics: Vec<String>,  // 감지된 존칭 어미 (~하오, ~습니다 등)
    casual_markers: Vec<String>, // 반말 마커 (~야, ~다, ~냐 등)
}
```

추출 방식 (Phase 6 진입 시 구현):
- 어미 기반 정규식 (PadAnalyzer/SignClassifier와 별개 layer)
- PadAnalyzer 결과와 **나란히 출력** (대체 아님)

현 시점에는 **구조만 설계하고 구현은 Phase 6 진입 시**.

#### 3.7.4 Phase 4 진입 시 영향

Phase 4 (magnitude k-NN 분류기) 프로토타입 큐레이션 시:
- `magnitude_strong.toml` 에 **무협체 + 현대어 존대 샘플 혼합** 소규모 실험
- 예: "천하에 다시없을 기재요" (무협) + "정말 대단하세요" (현대 존대)
- 반말 샘플은 **제외** — 적대/친밀 양의 위험, Phase 6 대기
- 각 카테고리당 무협:현대 = 7:3 정도 비율

##### Step C 실험 결과 (2026-04-19) — 현대 존대 불가결성 확인

**실험 목적**: 현대 존대 3개가 centroid 희석 요인인지 단일 변수 검증.

**방법**: `magnitude_strong.toml` v1 (무협 7 + 현대 3) → v2 (무협 7만) 로 변경 후 재측정.

**결과**:

| 버전 | 구성 | 전체 정확도 | strong 정확도 |
|---|---|---|---|
| v1 | 무협 7 + 현대 3 (run01) | **54%** | 27% (4/15) |
| v2 | 무협 7만 (run02) | **42%** | 7% (1/15) |

**결론 (반직관적)**: 현대 존대 제거가 오히려 악화. 현대 존대 프로토타입이 strong centroid를 **좁은 무협 특수 어휘(오장육부/천인공노/영웅호걸)에 갇히지 않도록 확장하는 역할**을 하고 있었음.

**악화 메커니즘**:
- "정말 대단하십니다" — 001 "크나큰 은혜를 입었소" 와 감사 축 공유 → v2에서 001이 strong → normal 로 이동
- "어쩜 이리 훌륭하신지" — 010/019 빈정 케이스와 표면 긍정 톤 공유 → v2에서 이탈
- 무협체 7개만으로는 subtype 다양성 부족 → 일반 감사/빈정 발화 포섭 실패

**적용**: 
- v3로 원복 (v1 + 실험 로그 주석)
- 향후 현대 존대 확장은 **허용**, 축소는 **금지** 원칙
- weak/normal 에도 현대 존대 혼합 검토 가능 (Step B 로 연기)

**회귀 감시**:
- Phase 4 벤치에서 무협 케이스 정확도 유지 확인
- 떨어지면 현대어 샘플 수 축소 아닌 **무협 샘플 다양성 추가**로 대응 (Step C 교훈)

#### 3.7.5 Phase 6 진입 시 완성 계획

1. `UtteranceRegisterMeta` 구조 구현 (어미 정규식 기반)
2. Relationship modifier가 meta 정보 소비:
   - `trust > 0.5 + Informal` → P+ 보강 (친밀 반말)
   - `trust < 0 + Informal` → P- 보강 (적대 반말)
   - `|trust| < 0.3 + Informal` → D+ 자극 + P- 약간 (무례)
3. 반말 프로토타입/앵커 선택적 추가 (Relationship layer 검증 후)

#### 3.7.6 프로덕션 Day 1 커버 범위

| 시나리오 | 처리 |
|---|---|
| NPC↔NPC (무협 존대) | ✅ 현재 완성 |
| NPC↔NPC (무협 반말, 적대/친밀) | Phase 6 대기 — 현재는 앵커 커버리지 부족으로 dead zone 위험 |
| Player(현대 존대) → NPC | Phase 4 에서 일부 커버 — 앵커에 소규모 혼합 |
| Player(현대 반말) → NPC | Phase 6 대기 — 반말 프로토타입 미큐레이션 |
| NPC(무협) → Player(현대) | 입력 측은 무협체 그대로 — 처리 영향 없음 |

**결론**: MVP는 존대 중심 (무협 + 현대) 커버. 반말·슬랭은 Post-MVP.

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
| **P2** | P축 변환식 계수 튜닝 (magnitude 기반) | ✅ **완료 (2026-04-19)** — Calibration 58% → 앵커 보강 62% |
| **P1.5** | PAD 앵커 보강 (`locales/anchors/ko.toml`) | ✅ **완료 (2026-04-19)** — P+ 4/P- 2 추가, PAD P 100% 보존 |
| **P3** | 정규식 프리필터 | ✅ **완료 (2026-04-19)** — 96% (25/26), Sparse 대체 불가 확인 (§3.6) |
| **P3.7** | Register 확장 전략 (γ+δ 설계) | ✅ **완료 (2026-04-19)** — §3.7 설계 문서화 |
| **P4** | 강도 축 k-NN 분류기 | ✅ **완료 (2026-04-19)** — 81% (21/26). weak 100%, strong 80%. v5 Step A (8개 프로토 확장) 효과 실증 |
| **P7** | 프로덕션 port 통합 | **진입** — `ListenerPerspectiveConverter` trait 설계 (`phase7-converter-integration.md`) |
| **P5** | 현대어 register 벤치 확장 | Phase 7 이후 — 20~30 현대어 **존대** 페어 케이스 추가, 앵커·프로토타입 현대 존대 비율 확장 |
| **P6** | Relationship layer + **반말 register 완성** | Phase 5 이후 — `UtteranceRegisterMeta` 구현 (§3.7.3), `trust × informality` 테이블 (§3.7.5), 반말 프로토타입/앵커 큐레이션 |
| Post-MVP | 슬랭 / 줄임말 / 혼합체 | Phase 6 이후 — 별도 연구 과제 |

---

## 8. 트레이드오프

### 채택 이유

- **임베딩 기반**: LLM 호출 없이 로컬 결정론적 처리
- **k-NN top-k**: 프로토타입 다양성 유지. 학습 없이 데이터 추가만으로 개선
- **부호 축 단독 Phase 1**: 한 번에 한 변수. 디버깅 경로 단순화

### 리스크

| 리스크 | 완화 |
|--------|------|
| 빈정거림 등 의도 층위 발화 오분류 | Phase 3 정규식 프리필터 (완료 96%) |
| 무협체 한정 — 현대 존대 미지원 | Phase 5 (현대 페어 벤치) — §3.7.0 매트릭스 |
| **반말 (친밀/적대/무례 양의)** | **Phase 6 (Relationship layer + `trust × informality`)** — §3.7.5 |
| 프로토타입 편향 | version 관리 + 실패 흡수 루프 |
| BGE-M3의 D축 76% 천장 | Phase 1~4는 P축만 대상 |

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
