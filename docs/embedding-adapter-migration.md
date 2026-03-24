# 임베딩 라이브러리 비교: fastembed vs bge-m3-onnx-rust

> 작성일: 2026-03-25  
> 프로젝트: npc-mind-rs (NPC 심리 엔진)

---

## 1. 개요

NPC 심리 엔진에서 플레이어 대사를 PAD 좌표로 변환하기 위해 텍스트 임베딩이 필요하다.
두 가지 방법을 비교하고 bge-m3-onnx-rust를 선택한 근거를 정리한다.

| 항목 | fastembed (crates.io) | bge-m3-onnx-rust (자체 구축) |
|------|----------------------|---------------------------|
| 저장소 | github.com/Anush008/fastembed-rs | C:\Users\bumko\projects\bge-m3-onnx-rust |
| 접근 | crates.io v5.13.0 | 로컬 path 의존성 |
| 핵심 의존성 | ort + tokenizers + hf-hub + reqwest + image | ort + tokenizers |
| 모델 | bge-m3 FP32 원본 | bge-m3 INT8 양자화 (gpahal/bge-m3-onnx-int8) |

---

## 2. 의존성 비교

### 2.1 크레이트 수

```
npc-mind 기본 (임베딩 없음):   40 크레이트
+ fastembed:                   605 크레이트 (+565)
+ bge-m3-onnx-rust:            241 크레이트 (+201)
```

bge-m3-onnx-rust가 fastembed 대비 **크레이트 수 60% 감소**.

### 2.2 fastembed가 끌고 오는 불필요한 의존성

우리가 fastembed에서 실제로 사용한 API는 2개뿐이다:

```rust
// 1. 모델 초기화 (1회)
TextEmbedding::try_new(InitOptions::new(EmbeddingModel::BGEM3))

// 2. 텍스트 → 벡터 (매 호출)
model.embed(vec!["텍스트"], None) → Vec<Vec<f32>>
```

이 2개를 위해 fastembed가 끌고 오는 의존성:

| 의존성 체인 | 역할 | 우리에게 필요? |
|------------|------|---------------|
| ort (ONNX Runtime) | 모델 추론 | ✅ 핵심 |
| tokenizers | 토큰화 | ✅ 핵심 |
| hf-hub → reqwest → hyper → tokio → h2 | 모델 자동 다운로드 HTTP 스택 | ❌ 로컬 모델 사용 |
| image → rav1e, ravif, png, jpeg, webp | 멀티모달 이미지 처리 | ❌ 텍스트만 사용 |
| indicatif, console | 다운로드 진행바 UI | ❌ 불필요 |
| ndarray | N차원 배열 | △ 간접 사용 |

605개 중 필요한 것은 ort + tokenizers 관련 ~80개뿐이었다.

### 2.3 bge-m3-onnx-rust의 의존성

```toml
# bge-m3-onnx-rust/Cargo.toml
[dependencies]
ort = { version = "2.0.0-rc.12", features = ["ndarray"] }
tokenizers = "0.21.0"
ndarray = "0.16.1"
anyhow = "1.0.95"
serde = { version = "1.0.217", features = ["derive"] }
serde_json = "1.0.138"
tokio = { version = "1.43.0", features = ["full"] }
tracing = "0.1"
```

ort + tokenizers만 직접 의존하고, 모델 다운로드/이미지 처리 체인이 없다.

---

## 3. 모델 비교

| 항목 | fastembed | bge-m3-onnx-rust |
|------|-----------|------------------|
| 모델 파일 | bge-m3 FP32 원본 | bge-m3 INT8 양자화 |
| 파일 크기 | ~1.2GB | **~570MB (52% 감소)** |
| 모델 출처 | HuggingFace 자동 다운로드 | gpahal/bge-m3-onnx-int8 (로컬 배치) |
| 모델 위치 | ~/.fastembed_cache/ | projects/models/bge-m3/ |
| 출력 | Dense 벡터만 | Dense + Sparse + ColBERT |
| 벡터 차원 | 1024 | 1024 |

---

## 4. PAD 출력 비교 (동일 대사, 동일 앵커)

동일한 한국어 앵커 텍스트(3축×양극단×3변형)를 사용하여
같은 대사에 대한 PAD 값을 비교했다.

### 4.1 개별 대사 결과

| 대사 | 축 | fastembed (FP32) | ort (INT8) | 차이 |
|------|-----|-----------------|------------|------|
| "네 이놈, 죽고 싶으냐!" | P | -0.109 | -0.118 | 0.009 |
| "네 이놈, 죽고 싶으냐!" | A | +0.176 | +0.172 | 0.004 |
| "네 이놈, 죽고 싶으냐!" | D | +0.031 | +0.029 | 0.002 |
| "은혜를 잊지 않겠습니다" | P | +0.137 | +0.131 | 0.006 |
| "은혜를 잊지 않겠습니다" | A | +0.045 | +0.057 | 0.012 |
| "은혜를 잊지 않겠습니다" | D | +0.006 | +0.005 | 0.001 |
| "당장 목을 치겠다! 칼을 뽑아라!" | P | -0.094 | -0.089 | 0.005 |
| "당장 목을 치겠다! 칼을 뽑아라!" | A | +0.244 | +0.238 | 0.006 |
| "당장 목을 치겠다! 칼을 뽑아라!" | D | +0.063 | +0.066 | 0.003 |

### 4.2 비교 결과 요약

- 모든 축에서 **방향(부호)이 완전 동일**
- 절대값 차이: 최대 0.012, 평균 0.005
- INT8 양자화에 의한 정밀도 손실이 PAD 추출에 영향 없음
- 상대 순서(도발 P < 0, 감사 P > 0, 위협 A > 차분 A 등) 완전 보존

### 4.3 상대 비교 테스트 (양쪽 모두 통과)

| 테스트 | 검증 내용 | fastembed | ort |
|--------|----------|-----------|-----|
| 도발 P < 0 | 불쾌한 대사의 Pleasure 음수 | ✅ -0.109 | ✅ -0.118 |
| 감사 P > 0 | 긍정 대사의 Pleasure 양수 | ✅ +0.137 | ✅ +0.131 |
| 위협 A > 0 | 격앙된 대사의 Arousal 양수 | ✅ +0.244 | ✅ +0.238 |
| 차분 A < 위협 A | 차분한 대사가 위협보다 각성 낮음 | ✅ | ✅ |
| 복종 D < 명령 D | 복종 대사가 명령보다 지배감 낮음 | ✅ | ✅ |
| 도발 → Anger 증폭 | apply_stimulus 전체 흐름 | ✅ | ✅ |

---

## 5. 빌드 시 이슈

### 5.1 ort 버전 충돌 (fastembed + bge-m3-onnx-rust 공존 불가)

처음에는 feature flag로 양쪽 모두 지원하려 했으나 빌드 실패:

```
[features]
embed-fastembed = ["fastembed", "anyhow"]
embed-ort = ["dep:bge-m3-onnx-rust"]
```

fastembed v5가 `ort = "=2.0.0-rc.10"`을 고정 의존하고,
bge-m3-onnx-rust는 `ort = "2.0.0-rc.12"`를 사용한다.
Cargo가 두 버전을 동시에 해결할 수 없어서 빌드 실패:

```
error: failed to select a version for `ort`.
    ... required by package `fastembed v5.0.1`
versions that meet the requirements `=2.0.0-rc.10` are: 2.0.0-rc.10
all possible versions conflict with previously selected packages.
  previously selected package `ort v2.0.0-rc.12`
    ... which satisfies dependency `ort = "^2.0.0-rc.12"` of package
        `bge-m3-onnx-rust v0.1.0`
failed to select a version for `ort` which could resolve this conflict
```

**해결**: fastembed를 완전히 제거하고 bge-m3-onnx-rust 단일 의존으로 전환.

### 5.2 Windows CRT 링킹 충돌

bge-m3-onnx-rust는 ort를 정적 링크하는데, ort prebuilt 바이너리가
`/MD`(동적 CRT)로 빌드되어 있다. Rust MSVC 기본값은 `/MT`(정적 CRT)이므로
혼용 시 링커 오류가 발생한다:

```
error LNK2038: 'RuntimeLibrary'에 대해 불일치가 검색되었습니다.
'MD_DynamicRelease' 값이 'MT_StaticRelease' 값과 일치하지 않습니다.
```

**해결**: `.cargo/config.toml`을 추가하여 CRT를 동적으로 통일:

```toml
# .cargo/config.toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=-crt-static"]

[env]
CFLAGS = "/MD"
CXXFLAGS = "/MD"
```

- `target-feature=-crt-static`: Rust 컴파일러가 동적 CRT 사용
- `CFLAGS="/MD"`: cc 크레이트가 C 소스 컴파일 시 `/MD` 전달 (onig_sys 등)
- `CXXFLAGS="/MD"`: cc 크레이트가 C++ 소스 컴파일 시 `/MD` 전달 (esaxx_rs 등)

CRT 설정 변경 후에는 반드시 `cargo clean` 필요 (이전 빌드 캐시와 충돌).

### 5.3 PadAnalyzer의 Send 바운드

테스트에서 `OnceLock<Mutex<PadAnalyzer>>`로 모델을 공유하려 하자
`dyn TextEmbedder`에 `Send`가 없어서 컴파일 실패:

```
error[E0277]: `(dyn TextEmbedder + 'static)` cannot be sent between threads safely
```

**해결**: `Box<dyn TextEmbedder>` → `Box<dyn TextEmbedder + Send>`로 변경.

---

## 6. 최종 비교 요약

| 항목 | fastembed | bge-m3-onnx-rust |
|------|-----------|------------------|
| 크레이트 수 | 605 | **241 (60% 감소)** |
| 모델 크기 | ~1.2GB (FP32) | **~570MB (INT8, 52% 감소)** |
| 모델 배치 | 런타임 자동 다운로드 | **로컬 파일 (오프라인 가능)** |
| ort 버전 | =2.0.0-rc.10 (고정) | 2.0.0-rc.12 |
| PAD 방향 일치 | 기준 | **100% 일치** |
| PAD 수치 차이 | 기준 | **평균 0.005 (무시 가능)** |
| 빌드 시간 (clean) | ~90초 | ~45초 |
| 추가 빌드 설정 | 없음 | .cargo/config.toml 필요 (CRT) |
| 게임 배포 | 모델 다운로드 로직 필요 | **에셋에 포함 가능** |
| 확장성 | Dense만 | Dense + Sparse + ColBERT |

### 선택 근거

1. 의존성 60% 감소로 빌드 속도와 바이너리 크기 개선
2. INT8 양자화 모델로 메모리/디스크 절반 절감, PAD 품질 동일
3. 로컬 모델 파일로 오프라인 빌드/게임 에셋 포함 가능
4. fastembed의 ort 버전 고정(rc.10)이 bge-m3-onnx-rust(rc.12)와 충돌하여 공존 불가
5. Sparse/ColBERT 출력을 향후 RAG/검색에 활용 가능
