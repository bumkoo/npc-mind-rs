# NPC Mind Engine — 테스트 레포트

## 테스트 정보
- **도서**: The Adventures of Huckleberry Finn (Mark Twain)
- **장면**: Chapter XV — 안개 속 거짓말과 Trash 연설
- **세션**: session_002 (PAD embed 분석 + A+ 앵커 개선 후)
- **날짜**: 2026-04-02
- **엔진 버전**: npc-mind 0.1.0
- **테스트 방식**: Mind Studio API + analyze-utterance (embed feature)
- **PAD 앵커**: ko.toml v2 (A+ 14개 = 전투 10 + 감정적 고조 4)

---

## 1. 장면 개요

안개 속에서 헉과 짐이 이별 후 재회하는 장면.
짐은 안개 속에서 헉이 죽은 줄 알고 밤새 울며 찾아다녔다.
뗏목에 돌아온 헉은 "그건 다 꿈이었다"고 거짓말한다.
짐이 뗏목 위 잔해물(trash)을 발견하고 거짓말을 깨닫자, 유명한 Trash 연설을 한다.
헉은 결국 짐에게 진심으로 사과한다.

### 테스트 목적
- Scene Focus 자동 전환 (betrayal → apology) 검증
- analyze-utterance (embed) PAD 자동 분석 품질 평가
- A+ 앵커 개선 효과 측정 (감정적 고조 표현 4개 추가)

---

## 2. 테스트 워크플로우

1. `POST /api/load` — session_001 시나리오 로드 (Scene Focus 자동 등록 + Initial appraise)
2. 6턴 stimulus — 대사별 `POST /api/analyze-utterance` → PAD 산출 → `POST /api/stimulus`
3. `POST /api/after-dialogue` — 관계 갱신 (praiseworthiness=-0.3, significance=0.7)
4. 결과 저장 — session_002

---

## 3. Scene Focus 설정

| Focus | Trigger | Event | Action |
|-------|---------|-------|--------|
| betrayal (Initial) | 즉시 | 짐이 걱정하며 울었는데 헉은 장난감으로 삼아 거짓말로 속임 (desir: -0.8) | 헉이 진심을 이용해 기만 (praise: -0.8) |
| apology (조건부) | Anger < 0.4 AND Distress < 0.3 | 헉이 자존심을 꺾고 사과 (desir: 0.7) | 자존심 꺾고 진심으로 사과 (praise: 0.7) |

---

## 4. 대사별 PAD 분석 + 감정 변동

### Initial appraise (betrayal Focus)
| 감정 | 강도 | context |
|------|------|---------|
| Distress | 0.806 | 짐이 걱정하며 울었는데 헉은 그걸 장난감으로 삼아 거짓말로 속였다 |
| Reproach | 1.000 | 헉이 짐의 진심어린 걱정을 이용해 거짓말로 기만함 |
| Anger | 0.903 | 거짓말 발각 — 헉이 짐의 진심을 이용해 거짓말로 기만함 |

### Turn 1: Huck — 태연한 귀환
- **대사**: "짐, 나야. 안개가 걷혔으니까 돌아왔지. 뭐 걱정이라도 했어?"
- **PAD**: P=+0.324, A=-0.246, D=0.000
- **PAD 평가**: P+ 태연한 톤 ✅, A- 차분함 ✅, D 중립 (약간 아쉬움)
- **감정 후**: Distress 0.633, Reproach 0.988, Anger 0.774
- **효과**: P+ 자극이 Distress를 소폭 감소시킴 (-0.173)

### Turn 2: Jim — 울며 걱정한 마음
- **대사**: "헉! 네가 살아있다니! 난 네가 죽은 줄 알고 밤새 울며 찾아다녔단 말이다!"
- **PAD**: P=-0.079, A=+0.355, D=0.000
- **PAD 평가**: P 약한 부정 (상처 톤) ✅, A+ 감정적 고조 ✅ (이전 0.261→0.355, 앵커 개선 효과), D 중립
- **감정 후**: Distress 0.627, Reproach 0.986, Anger 0.804
- **효과**: A+ 자극이 Anger를 소폭 증폭 (+0.030)

### Turn 3: Huck — 뻔뻔한 거짓말
- **대사**: "무슨 소리야, 난 여기 줄곧 있었는데. 네가 꿈을 꾼 거 아냐?"
- **PAD**: P=+0.140, A=+0.065, D=+0.131
- **PAD 평가**: P+ 태연함 ✅, A 거의 중립 ✅, D+ 약간 지배적 ✅
- **감정 후**: Distress 0.612, Reproach 0.978, Anger 0.799

### Turn 4: Jim — 잔해 발견, 거짓말 깨달음
- **대사**: "그럼 저 나뭇잎이랑 쓰레기는 뭐냐? 이것도 꿈이냐?"
- **PAD**: P=-0.233, A=+0.105, D=0.000
- **PAD 평가**: P- 불쾌감 ✅, A+ 약간 고조 (이전 A=0.000→0.105, dead zone 탈출!) ✅
- **감정 후**: Distress 0.624, Reproach 0.985, Anger 0.820
- **효과**: P- 자극이 Anger 증폭 (+0.021)

### Turn 5: Jim — Trash Speech
- **대사**: "짐이 물에 빠져 죽을 뻔했을 때, 네 생각에 가슴이 터질 것 같았다..."
- **PAD**: P=-0.304, A=+0.215, D=0.000
- **PAD 평가**: P- 상처/분노 ✅, A+ 감정적 고조 ✅ (핵심 개선! 이전 0.077→0.215), D 중립 (여전히 아쉬움)
- **감정 후**: Distress 0.636, Reproach 0.994, Anger 0.854
- **효과**: P-A+ 자극이 Anger를 확실히 증폭 (+0.034)

### Turn 6: Huck — 사과
- **대사**: "... 짐, 미안해. 내가 잘못했어. 다시는 그러지 않을게."
- **PAD**: P=-0.339, A=+0.158, D=0.000
- **PAD 평가**: P- 후회/자책 톤 (화자 감정 = 올바름), A+ 약간의 감정적 고조 ✅
- **감정 후**: Distress 0.652, Reproach 1.000, Anger 0.886
- **핵심 문제**: 사과 대사가 P-로 분석되어 짐의 고통을 오히려 증폭시킴 (설계 결정: 화자 톤 유지)

---

## 5. Beat 전환 결과

**전환 없음** — Anger가 최종 0.886으로, threshold 0.4를 훨씬 초과한 상태로 유지.

원인 분석:
1. T6 사과 대사의 P=-0.339가 부정 자극으로 작용 → Anger 감소 대신 증폭
2. stimulus만으로는 Initial appraise 수준의 강한 감정(Anger 0.903)을 0.4 이하로 끌어내리기 어려움
3. 관성 공식상 intensity가 높을수록 변동 폭이 작음 (max(1-0.886, 0.3) = 0.3)

**필요 조치**: 사과 대사 적용 시 사용자가 P를 +0.5 정도로 수동 보정하면 Beat 전환 가능 (설계 결정에 따른 운영 방법)

---

## 6. 관계 변동

| | before | after | 변동 | 평가 |
|--|--------|-------|------|------|
| closeness | 0.550 | 0.419 | -0.131 | 적절 — 배신에 의한 친밀도 하락 |
| trust | 0.600 | 0.507 | -0.093 | 적절 — 거짓말에 의한 신뢰 하락 |
| power | -0.300 | -0.300 | 0 | 정상 — 대화로 변경 안 됨 |

관계 변동은 Ch.8 session_001 이슈(변동 너무 작음)에 비해 significance=0.7 적용으로 합리적 수준.

---

## 7. 감정 아크 요약

```
Initial (betrayal)  ■■■■■■■■■░  Anger 0.903  Reproach 1.000  Distress 0.806
Turn 1 (태연)       ■■■■■■■■░░  Anger 0.774  Reproach 0.988  Distress 0.633  ↓ P+ 자극
Turn 2 (울며)       ■■■■■■■■░░  Anger 0.804  Reproach 0.986  Distress 0.627  ↑ A+ 자극
Turn 3 (거짓말)     ■■■■■■■■░░  Anger 0.799  Reproach 0.978  Distress 0.612  ~ 약한 자극
Turn 4 (발견)       ■■■■■■■■░░  Anger 0.820  Reproach 0.985  Distress 0.624  ↑ P- 자극
Turn 5 (Trash)      ■■■■■■■■■░  Anger 0.854  Reproach 0.994  Distress 0.636  ↑ P-A+ 자극
Turn 6 (사과)       ■■■■■■■■■░  Anger 0.886  Reproach 1.000  Distress 0.652  ↑ P- 화자 톤
```

감정 흐름: 강한 분노 유지 → 약간의 등락 → Trash Speech에서 재점화 → 사과가 오히려 증폭
Beat 전환 threshold(0.4)에 도달 못함.

---

## 8. PAD 분석기 평가 (A+ 앵커 개선 효과)

| 턴 | A (before, v1) | A (after, v2) | 변화 |
|---|---|---|---|
| T2 Jim 울며 | +0.261 | +0.355 | +36% ↑ |
| T3 Huck 거짓말 | 0.000 | +0.065 | dead zone 탈출 |
| T4 Jim 발견 | 0.000 | +0.105 | dead zone 탈출 |
| T5 Trash Speech | +0.077 | +0.215 | +179% ↑↑ |
| T6 Huck 사과 | 0.000 | +0.158 | dead zone 탈출 |

A+ 앵커 4개 추가로 감정적 고조 대사의 Arousal 감지가 크게 개선됨.

---

## 9. 프롬프트 출력 검증

### Turn 5 프롬프트 (Trash Speech 시점) — 핵심 프롬프트

**감정 구성**: 비난(극도로 강한, 지배) + 분노(극도로 강한) + 고통(강한)
**어조**: 냉소적이고 비판적인 어조 ✅ (Trash Speech에 적합)
**태도**: 불만을 억누르지만 불편함이 드러나는 태도 — △ 원작에서 짐은 억누르기보다 직접적으로 표현
**행동 경향**: 분노를 억누르고 계획적으로 대응 — △ 원작은 직접적 비난
**금지 사항**: 농담 ❌ + 호의적 태도 ❌ + 거짓말 ❌ ✅

프롬프트 방향은 전체적으로 맞지만, "억누르다"보다 "직접적으로 분노를 표현한다"가 Trash Speech에 더 적합.
→ directive.rs의 Anger+높은 sincerity 분기에서 "직접적 표현" 방향 강화 검토.

---

## 10. 종합 평가

### 잘 동작한 것

1. **Scene Focus 초기화 정상**: betrayal Focus 자동 appraise → 3개 감정 정확 생성
2. **analyze-utterance 연동 성공**: 6턴 전부 PAD 자동 산출 → stimulus 적용
3. **A+ 앵커 개선 효과 확인**: Trash Speech A값 +179%, dead zone 탈출 3건
4. **관계 변동 합리적**: closeness -0.131, trust -0.093 (significance=0.7 반영)
5. **프롬프트 품질 양호**: 감정 context, 금지 사항, 관계 라벨 모두 정상

### 개선 필요 사항

| # | 이슈 | 우선순위 | 설명 |
|---|------|---------|------|
| 1 | Beat 전환 불가 | 높음 | Anger 0.886→0.4 불가능. 사과 대사의 P- 문제 (설계 결정: 수동 보정 필요) |
| 2 | D축 전부 0 | 중간 | 6턴 전부 D=0.000. D축 앵커 변별력 근본적 한계 |
| 3 | 억제형 Anger 분기 | 낮음 | Trash Speech에서 "억누르고 계획적" → "직접적 분노 표현"이 더 적합 |

---

## 11. 파일 목록

| 파일 | 내용 |
|------|------|
| scenario.json | NPC + 관계 + turn_history(7턴) + scene(Focus 2개) |
| test_report.md | 본 레포트 |
| evaluation.md | 간략 평가 + 개선 이력 |
| turn1_huck_casual.txt | Turn 1 프롬프트 (헉 태연한 귀환) |
| turn2_jim_crying.txt | Turn 2 프롬프트 (짐 울며 걱정) |
| turn3_huck_lie.txt | Turn 3 프롬프트 (헉 뻔뻔한 거짓말) |
| turn4_jim_trash_discovery.txt | Turn 4 프롬프트 (짐 잔해 발견) |
| turn5_trash_speech.txt | Turn 5 프롬프트 (Trash 연설) |
| turn6_huck_apology.txt | Turn 6 프롬프트 (헉 사과) |
