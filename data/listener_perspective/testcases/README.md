# 테스트 케이스 (채점 정답지)

벤치 실행 시 분류기에 입력되는 발화와 기대 라벨 집합.
`expected_sign`을 기준으로 채점하여 정확도를 산출.

## 파일

- `sign_benchmark.toml` — 부호 축 분류기 벤치마크용

## 필드 구조

| 필드 | 용도 |
|---|---|
| `id` | 실패 케이스 추적용 고유 식별자 |
| `utterance` | 분류기 입력 발화 |
| `expected_sign` | 정답 (`keep` or `invert`) — 채점 기준 |
| `speaker_p_sign` | 화자 톤의 P 부호 (Phase 2 검증용) |
| `listener_p_sign` | 청자 체감 P 부호 (Phase 2 검증용) |
| `difficulty` | `easy` / `medium` / `hard` |
| `subtype` | 커버리지 점검용 |
| `notes` | 복기/디버깅 단서 |

## 규모 지침

- 총 24~30개
- 난이도 배분: easy 40% / medium 40% / hard 20%
- 부호 균형: keep/invert 각 난이도에서 반반
- 모든 subtype이 최소 1번 등장

## 주의

**`expected_*` 라벨 임의 변경 금지.** 변경 시 Bekay 확인 필수.
(PAD 벤치마크와 동일 원칙)
