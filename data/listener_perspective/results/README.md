# 벤치 결과 로그

벤치 실행마다 자동 생성되는 Markdown 리포트.

## 파일 규칙

- `baseline.md` — **커밋됨**. 현재 기준선
- `YYYY-MM-DD_runNN.md` — **gitignore**. 로컬 실험용 런별 로그

## 파일 구조

각 리포트는 YAML front-matter + 3섹션 본문:

```
---
run_id, 버전, overall_accuracy
---

## 요약 (난이도별 / 부호별 / subtype별)
## 실패 케이스 상세
## 점수차 분포
```

## 튜닝 워크플로우

```
1. cargo test --features embed --test sign_classifier_bench -- --nocapture
   → 새 YYYY-MM-DD_runNN.md 생성

2. 실패 케이스 분석:
   - 점수차 < 0.05 → 프로토타입 한두 개 추가해 개선 시도
   - 점수차 > 0.10 → 프로토타입 구조 재검토

3. 재실행 → runNN+1 생성

4. git diff baseline.md YYYY-MM-DD_runNN.md
   → front-matter 정확도 변화 확인

5. 목표 달성 시 baseline.md 덮어쓰기 + 커밋
```

## 목표 정확도 (Phase 1)

| 카테고리 | 목표 |
|---|---|
| 전체 | 80% 이상 |
| easy | 95% 이상 |
| hard | 70% 이상 (시작값, 벤치 결과 보고 재조정) |

실패해야 하는 케이스도 포함됨 (반어법, 체념 표현 등) — hard 목표는 이들을 전부 맞추라는 뜻이 아님.
