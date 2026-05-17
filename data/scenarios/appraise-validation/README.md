# Phase 2.3 Appraise Validation — S1~S4 시나리오 박제

`task-rel-phase2.3-appraise-tuning.md` Stage 2.3-C 산출. **appraise tuning narrative
검증**용 4 시나리오 + `_expected_axes_delta` prose.

## 위치

- 본 디렉토리는 **데이터 박제 + 디자이너 narrative 검토 컨텍스트** 전용.
- 정량 회귀 가드는 `tests/phase2_narrative_test.rs` 에 이미 박제 (tol ±0.5; Phase 2 Stage 5
  진입 시점 코드 산출, S5-D3). 본 디렉토리의 JSON은 그 박제값과 1:1 대응.

## 4 케이스

| 케이스 | 시나리오 | OCC peak | 박제 4축 (trust/affinity/respect/wariness) |
|---|---|---|---|
| **S1** | [S1.json](S1.json) — 임충→노지심 (Admiration+Gratitude) | 0.6 | (64.4, 46.0, 32.0, 0.0) |
| **S2** | [S2.json](S2.json) — 임충→육겸 (Reproach+Hate+Anger, 산신묘 처단) | 0.9+ | (3.8, 3.0, -4.0, 42.5) |
| **S3** | [S3.json](S3.json) — 수련→옥교룡 (Pity+Reproach+Anger) | 0.7 | (25.216, 26.64, -1.76, 32.32) |
| **S4** | [S4.json](S4.json) — 임충→고구 (정성, 시간 분산) | — | qualitative · BLOCKED |

## Phase 2.5 이관 (P-D-8 확정)

- **`axis_modulation`**: src 전체 0건 + `ReflectionResult` 정확히 7필드 (is_chitchat/summary/
  significance_score/declarative_events/partnership_event/turn_count/llm_reasoning).
  `axis_modulation` 신설 = Phase 2.5 LLM 기반 reflection 출력 필드. Phase 2.3 진입 불가
  (구조 의존).
- **시간 분산 (S4)**: axis_modulation 하드 의존 → S4 정량 검증은 Phase 2.5 시간 분산 모델
  활성화 후 1차 케이스로 진입.

## §C P-D-C1 — content-projection 정책 (확인② 결정 항목)

`relationship_memory_handler.rs::dominant_delta` (L63) — 게이트는 **`max@5.0`** FROZEN
(Stage 0 §3-bis). 정책 결정 = MemoryEntry.content 라벨 정책:

| 옵션 | 설명 | trade-off |
|---|---|---|
| **(a) 현행** | dominant 1축만 라벨 | 텍스트 간결 / 다축 동시 유의 변화 *소실* |
| **(b) threshold 초과 전부** | ≥ 5.0 축 전부, magnitude desc 정렬 | 충실한 다축 회상 / 게이트 수학 동치 / 회귀 0 |
| (c) hybrid | dominant + 부차 비율 임계 시만 | 복잡도↑, 불요 |

### S2 anchor 측정 (P-D-C1 (b) 채택 근거)

S2 사후 4축 |Δ|:

| 축 | Δ | 임계값(5.0) 초과? |
|---|---|---|
| trust | **-46.2** | YES (dominant) |
| wariness | **+42.5** | YES |
| affinity | **-37.0** | YES |
| respect | **-24.0** | YES |

- **현행 (a) 라벨**: `[trust Δ=-46.20]`
- **소실되는 정보**: wariness +42.5 (산신묘 결단 후 *경계심 50% 도달*) / affinity -37 (옛
  친구 단절) / respect -24 (인격 격하). 모두 narrative 핵심 신호인데 기억 텍스트에서 사라짐.
- **(b) 라벨**: `[trust Δ=-46.20, wariness Δ=+42.50, affinity Δ=-37.00, respect Δ=-24.00]`
  (magnitude desc).
- **게이트 동치 보존**: `filter(|Δ|≥thr).is_empty() ⟺ max(|Δ|) < thr`. (b)는 *무엇을 기억하는가*
  불변, 라벨만 풍부.

→ **권고 (확인② 입력)**: P-D-C1 **(b) 채택** — S2가 직접 정당화. 회귀 0 (순수 projection).

### S1·S3 비교

| 케이스 | \|Δ\|>5 축 수 | 현행 dominant | (b) 채택 효과 |
|---|---|---|---|
| S1 | 3 (trust 14.4 / respect 12 / affinity 6 — affinity는 정확히 임계값 ≥5) | trust | respect·affinity 보존 |
| S2 | 4 (trust 46.2 / wariness 42.5 / affinity 37 / respect 24) | trust | **핵심** — wariness·affinity·respect 보존 |
| S3 | 3 (trust 14.78 / wariness 12.32 / respect 11.76) | trust | wariness·respect 보존 |

3 케이스 모두 dominant 1축 외에 ≥ 2 축 추가 정보 소실. (b) 일관 권고.

## §B HEXACO 6→24 차등 spread + significance 게이트 (narrative-gated)

`mind_sync.rs:40` — 현 identity flat copy (HEXACO 6 dim을 4 facet에 동일 복사).
`reflection.rs::compute_significance` 가중치 = `peak_occ·0.40 + pad_mag·0.30 + diversity·0.15 + beat·0.15`.

### 권고 (확인② 입력)

- **HEXACO spread**: S1~S3 박제값은 *6 dim flat copy* 가정에서 산출. 차등 spread는 facet별
  세분화 → modifier 계산 분해도↑. **narrative 의존** — 디자이너가 S1~S3 박제값이 *서사 직관*에
  부합하는지 판단 후 진입. 본 Phase 2.3에서는 *현행 유지* 권고 (회귀 가드 보존).
- **significance 가중치**: S1(0.78) / S2(0.92) / S3(0.72) 박제. 3 케이스 모두 outer loop 진입
  threshold (0.3) 통과. 변경 trigger 없음 → **현행 유지** 권고.

## §5.2 디자이너 판단분 — intensity 0.4 잠정

`phase1_daily_training_test.rs::set_intensity(EmotionType::Admiration, 0.4)` 잠정값.

- **0.4 유지**: 일상 가르침의 "어제보다 안정됐다" 톤
- **상향 (0.5~0.6)**: mid 밴드 axes 변동 미세→중강 필요 시
- **하향 (0.3)**: mid 밴드 axes 변동 너무 큼 판정 시

조정 동기: `daily-training.json` `_expected_axes_delta` 문구 + S5-D4 박제값.

권고 (확인② 입력): **0.4 유지** (D3 박제값 0.461 exact 보존 직접 의존). 변경 시 D3 회귀.

## §5.3 디자이너 판단분 — S1~S3 narrative 타당성

| 케이스 | 박제값 (trust/aff/resp/war) | 검토 포인트 |
|---|---|---|
| S1 임충→노지심 | (64.4, 46.0, 32.0, 0.0) | respect +12, trust +14.4. 의리·은혜 갚음. Admiration+Gratitude 효과 합치 적절? |
| S2 임충→육겸 | (3.8, 3.0, -4.0, 42.5) | trust 50→3.8 (거의 0). 옛 친구의 처단 → 완전 단절. wariness +42.5 (50% 도달) — 과한지/부족한지 |
| S3 수련→옥교룡 | (25.216, 26.64, -1.76, 32.32) | trust -14.8 / respect -11.8 / wariness +12.3. 안타까움+책망+분노. 사부의 *체념과 한* 표현 적절? |

어색 시: 입력 emotion intensity 조정 → 코드 재실행 → 새 EXPECTED 박제 → Stage 5 §4 작업 3
표 갱신. 게이트 2 tolerance 완화 **금지** (S5-D4).

## 변경 이력

| 일자 | 변경 |
|---|---|
| 2026-05-16 | 디렉토리 신설 (Phase 2 Stage 6 작업 6) — placeholder. |
| 2026-05-17 | Phase 2.3 Stage 2.3-C — S1~S4 4 시나리오 박제 + P-D-C1 anchor 측정 (S2) + §B/§5.2/§5.3 권고 정리. |
