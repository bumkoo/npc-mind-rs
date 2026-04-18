# Listener-perspective 변환 실험 데이터

화자 발화의 PAD 톤을 **청자가 체감하는 PAD 자극**으로 변환하기 위한 실험 데이터.

## 설계 문서

[`docs/emotion/sign-classifier-design.md`](../../docs/emotion/sign-classifier-design.md)

## 디렉토리 구조

```
listener_perspective/
├── prototypes/       # 분류기 기준점 — 그룹별 대표 발화
│   ├── sign_keep.toml      # 부호 유지 (감사, 칭찬, 비난, 위협)
│   └── sign_invert.toml    # 부호 반전 (사과, 간청, 위로, 빈정)
├── testcases/        # 채점용 라벨링된 테스트 케이스
│   └── sign_benchmark.toml
└── results/          # 벤치 실행 결과 로그
    ├── baseline.md              # 기준선 (커밋됨)
    └── YYYY-MM-DD_runNN.md      # 런별 로그 (gitignore)
```

## 파일 역할

| 파일 | 역할 | 언제 쓰이나 |
|---|---|---|
| `prototypes/sign_keep.toml` | 분류 기준점 | 분류기 초기화 시 로드 |
| `prototypes/sign_invert.toml` | 분류 기준점 | 분류기 초기화 시 로드 |
| `testcases/sign_benchmark.toml` | 채점 정답지 | 벤치 실행 시만 로드 |
| `results/*.md` | 실행 결과 아카이브 | 튜닝 근거 추적용 |

## 중요 원칙

- `prototypes/sign_keep.toml` ∩ `sign_invert.toml` = ∅ (중복 금지)
- 프로토타입과 테스트 케이스 발화 중복 금지 (데이터 누수 방지)
- `expected_*` 라벨 변경 시 Bekay 확인 필수

## 벤치 실행

```bash
cargo test --features embed --test sign_classifier_bench -- --nocapture
```

결과는 `results/YYYY-MM-DD_runNN.md` 로 자동 생성됨.
