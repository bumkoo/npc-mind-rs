# Session 002: Ch.15 안개 — 거짓말 발각과 Trash 연설
- 날짜: 2026-04-02
- NPC 주체: Jim (짐의 관점)
- 대화 상대: Huck Finn
- 테스트 방식: analyze-utterance (embed) → stimulus 6턴
- PAD 앵커: ko.toml v2 (A+ 14개)

## 대화 턴 요약

| Turn | 화자 | 대사 요약 | PAD (P/A/D) | Jim 감정 변화 |
|------|------|----------|-------------|-------------|
| Init | - | betrayal Focus appraise | - | Anger 0.903, Reproach 1.000, Distress 0.806 |
| 1 | Huck | 태연한 귀환 | +0.32/-0.25/0 | Anger 0.774 ↓ |
| 2 | Jim | 울며 걱정 토로 | -0.08/+0.36/0 | Anger 0.804 ↑ |
| 3 | Huck | 뻔뻔한 거짓말 | +0.14/+0.07/+0.13 | Anger 0.799 ~ |
| 4 | Jim | 잔해 발견, 거짓말 깨달음 | -0.23/+0.11/0 | Anger 0.820 ↑ |
| 5 | Jim | Trash Speech | -0.30/+0.22/0 | Anger 0.854 ↑ |
| 6 | Huck | 사과 | -0.34/+0.16/0 | Anger 0.886 ↑ |

## 관계 변동 (Jim → Huck)

| | before | after | 변동 |
|--|--------|-------|------|
| closeness | 0.550 | 0.419 | -0.131 |
| trust | 0.600 | 0.507 | -0.093 |
| power | -0.300 | -0.300 | 0 |

## 평가: 잘 동작한 것

1. Scene Focus 초기화 + betrayal appraise 정상 (Anger/Reproach/Distress 3개)
2. analyze-utterance → stimulus 연동 파이프라인 정상 작동 (6턴 전부)
3. A+ 앵커 개선 효과 확인: Trash Speech A=0.077→0.215 (+179%)
4. dead zone 탈출 3건 (T3, T4, T6 모두 A=0→유의미한 값)
5. 관계 변동 합리적 (significance=0.7 반영)

## 평가: 개선 필요

### 이슈 1: Beat 전환 불가 (높음)
Anger가 최종 0.886으로, apology Focus trigger(Anger<0.4) 미충족.
근본 원인: 사과 대사 P=-0.339 (화자의 후회 톤)가 부정 자극으로 작용.
설계 결정: 대사 PAD = 화자 감정 톤. 청자 관점은 수동 보정.
→ session_003에서 T6 PAD를 수동 보정(P=+0.5, A=-0.2)하여 Beat 전환 검증 필요.

### 이슈 2: D축 전부 0 (중간)
6턴 전부 D=0.000. BGE-M3 임베딩의 D축 변별 한계.
벤치마크 D축 정확도 82%이지만, 실전 대사에서는 dead zone(0.02) 이하만 나옴.
→ D축 개선은 모델 한계로 우선순위 보류.

### 이슈 3: Anger 분기 "억제형" (낮음)
Trash Speech에서 "억누르고 계획적으로 대응" → 원작은 직접적 분노 표현.
Jim의 patience(0.7) + forgiveness(0.8)가 억제 분기로 유도.
→ directive.rs에서 Anger 강도가 극도로 강할 때 sincerity가 높으면 직접 표현 분기 검토.

## 개선 이력

| 날짜 | 변경 | 효과 |
|------|------|------|
| 2026-04-02 | A+ 앵커 감정적 고조 4개 추가 (v2) | Trash Speech A +179%, dead zone 탈출 3건 |
| 2026-04-02 | 설계 결정: 대사 PAD = 화자 톤 | 사과/위로 대사는 수동 보정 운영 방침 확정 |

## 다음 세션 방향

1. **session_003**: T6 사과 대사 PAD 수동 보정 → Beat 전환 검증
2. **Anger 분기 검토**: 극도로 강한 Anger + 높은 sincerity → 직접 표현 분기
3. **무협 도메인 대사**: 청강만리/와호장룡 스타일 대사로 PAD 앵커 확장 테스트
