# Session 002: Ch.8 잭슨 섬 — 이슈 수정 후 재실행
- 날짜: 2026-03-29
- NPC 주체: Jim (짐의 관점)
- 대화 상대: Huck Finn
- 비교 대상: session_001

## 수정 사항 요약

| 이슈 | 내용 | 수정 |
|------|------|------|
| 1 | Gratitude 누락 | HopeFulfilled/FearConfirmed 시 Joy/Distress fall-through |
| 2 | 관계 변동 과소 | significance 파라미터 추가 (배율 = 1 + sig × 3) |
| 4 | power 라벨 오류 | 3단계→5단계 확장 + 행동 지시 포함 |

## 대화 턴 비교 (Session 001 → 002)

| Turn | 장면 | S001 감정 | S002 감정 | 변화 |
|------|------|----------|----------|------|
| 1 | 유령 공포 | Distress 0.650 | Distress 0.650 | 동일 |
| 2 | 안도 (FearUnrealized) | Relief 0.858 | Relief 0.858 | 동일 |
| 3 | 도주 고백 | Fear 0.676, Shame 0.268 | Fear 0.676, Shame 0.268 | 동일 |
| 4 | 헉의 맹세 (HopeFulfilled) | Satisfaction 0.858, Admiration 1.000 | Satisfaction 0.858, **Joy 0.914**, Admiration 1.000, **Gratitude 0.957** | 🆕 Joy+Gratitude |

## 관계 변동 비교

| | S001 (sig=0.0) before | S001 after | S002 (sig=0.8) after | 배율 |
|--|-----------|--------|---------|------|
| closeness | 0.100 | 0.146 (+0.046) | 0.258 (+0.158) | 3.4× |
| trust | 0.100 | 0.180 (+0.080) | 0.372 (+0.272) | 3.4× |
| power | -0.300 | -0.300 | -0.300 | — |

## 이슈별 검증 결과

### 이슈 1: Gratitude 생성 ✅ 해결
- HopeFulfilled에서 Satisfaction + Joy가 동시 생성됨
- Joy(0.914) + Admiration(1.000) → Gratitude(0.957) 정상 생성
- FearUnrealized(Turn 2)에서는 Relief만 생성, Joy/Distress 없음 → 정상

### 이슈 2: 관계 변동 ✅ 해결
- significance=0.8 적용 시 변동 폭 3.4배 증가
- trust 0.100→0.372: "인생을 건 약속" 장면에 적절한 수준
- closeness 0.100→0.258: 급격한 친밀도 상승도 자연스러움

### 이슈 3: 턴 간 감정 누적 ⏳ 미착수
- 의도된 설계 제약 (stateless engine)
- stimulus 워크플로우로 우회 가능

### 이슈 4: power 라벨 ✅ 해결
- power=-0.3 → "하위자 — 가르치는 어조, 지시와 훈계가 자연스러움"
- Session 001에서 "대등한 관계"로 표시되던 오류 수정됨
- 5단계 분류 + 행동 지시로 4B 모델의 연기 구분 가능성 향상

## 남아있는 관찰

1. **Turn 4 Joy 강도 0.914**: Joy의 weight(desirability_self_weight)가 Satisfaction과 별도로 적용되어 독립적인 강도를 가짐. Satisfaction(0.858)과 Joy(0.914)가 공존하는 것이 OCC 이론상 맞는지 추후 검토 필요.

2. **Gratitude 강도 0.957**: (Admiration 1.0 + Joy 0.914) / 2 = 0.957. 매우 강한 감사로, 이 장면의 서사적 무게와 일치.

3. **프롬프트 품질 개선**: "상하 관계: 하위자 — 가르치는 어조..." 행동 지시가 프롬프트에 포함됨. 4B LLM 테스트로 실제 연기 차이 확인 필요.

## 다음 단계

- Phase 1 기초 검증 잔여: Ch.15 안개 장면 (신뢰 파열/회복 테스트)
- 이슈 3 대응: stimulus 워크플로우로 턴 간 감정 연결 시도
- 4B LLM으로 power 라벨 행동 지시 효과 검증
