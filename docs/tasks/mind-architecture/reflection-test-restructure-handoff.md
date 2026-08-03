# Reflection 테스트 재구성 — 인계 문서

> 작성: 2026-08-04 세션. 브랜치 `llm-stage1-remove-timings`.
> 목적: `phase1_real_llm_test` 재구성 작업을 다른 세션에서 이어받아 검증·구현하기 위한 컨텍스트.

---

## 0. 브랜치 상태

`main`에서 분기한 `llm-stage1-remove-timings` 위에 4 커밋. **아직 push 안 됨.**

| 커밋 | 내용 |
|---|---|
| `3014e86` | LLM 정리 1단계 — timings 파이프라인 전량 제거 (-763줄) + 빈 응답 가드 |
| `0fda5c2` | `dialogue_converter_integration.rs` D축 기대값 갱신 (e121610 누락분) |
| `217ef85` | CLAUDE.md — 회귀 확인 정본 명령 + 플래그 근거 |
| `fc2174a` | LLM 정리 2단계 — rig 0.41 이관 + Responses API 전환 |

현재 회귀: **978 passed / 0 failed / 3 ignored (67 묶음)**.

```bash
cargo test --features chat,mind-studio --lib --tests --bins --no-fail-fast
```

> ⚠️ 이 명령이 **정본**이다. `--features chat` 단독은 91개를 놓치고,
> `--lib --tests --bins`를 빼면 깨진 `examples/` 2개 때문에 빌드가 중단되며,
> `--no-fail-fast`를 빼면 첫 실패에서 멈춰 4/67 묶음만 보고 통과로 오인한다.
> (CLAUDE.md 「회귀 확인 명령의 플래그가 전부 필요한 이유」 절 참조)

---

## 1. 발단

`fc2174a`(rig 0.41 + Responses) 검증을 위해 실서버 E2E를 돌렸다.

```bash
NPC_MIND_CHAT_URL=http://127.0.0.1:8081/v1 \
  cargo test --features chat --test phase1_real_llm_test -- --ignored --nocapture
```

테스트 자체는 통과(`ok. 1 passed`)했으나 `⚠️ DRIFT 2/3`이 출력됐다. 원인을 파다가
**이 테스트가 목적 두 개를 겸하고 있고, 그중 하나는 구조적으로 검증이 불가능한
상태**라는 것이 드러났다.

---

## 2. 실측으로 확인한 사실 (추측 아님)

### 2-1. `significance_score`는 LLM이 내는 값이 아니다

[`reflection_service.rs:191`](../../../src/application/reflection_service.rs):

```rust
let significance = compute_significance(turns);   // 결정론적 엔진 계산
...
significance_score: significance,                  // LLM 응답 미사용
```

LLM은 `is_chitchat` / `summary` / `reasoning`만 JSON으로 낸다. LLM 호출이 실패해도
`fallback_result`가 같은 계산값을 그대로 쓴다.

**따라서 "두 실행에서 significance가 같다"는 LLM 동등성의 근거가 되지 못한다.**
(`fc2174a` 커밋 메시지가 이 점을 처음엔 잘못 서술했다가 amend로 정정됨)

### 2-2. `is_chitchat`은 순수 LLM 판정이며, 입력은 대사 텍스트뿐이다

[`format_transcript`](../../../src/application/reflection_service.rs)가 `TurnSnapshot`에서
`user_utterance` / `npc_response` **두 필드만** 뽑는다. `pad_after`·`occ_emotions`·
`beat_changed`는 프롬프트에 들어가지 않는다.

→ 주관(LLM)과 객관(엔진)이 **의도적으로 분리**되어 있고, 게이트에서 `OR`로 묶인다:
`significance >= 0.3 OR !is_chitchat` ([`relationship_policy.rs:317`](../../../src/application/command/policies/relationship_policy.rs))

### 2-3. `phase1_real_llm_test`는 analyzer를 붙이지 않는다

`with_analyzer` grep **0건**. `turn(&sid, utterance, None, None)`으로 `pad_hint`도 `None`.

결과: [`dialogue_orchestrator.rs:453`](../../../src/application/dialogue_orchestrator.rs)에서
`turn_pad`가 매 턴 `Pad::neutral()`로 폴백 → 모든 `pad_after`가 동일 →
`pad_magnitude = 0` → 자극이 없으니 감정도 안 흔들려 `peak_occ`·`diversity`도 낮고
Beat 전환도 안 일어나 `beat_signal = 0`.

**즉 이 테스트의 significance 밴드 단언은 처음부터 통과할 수 없는 상태였다.**

### 2-4. `phase1_bench_test`의 0.000/0.461/0.980과는 비교 대상이 아니다

`phase1_bench_test`는 `TurnSnapshot`을 리터럴로 손수 구성한다 (analyzer·LLM 0건).
`compute_significance` 수식·가중치의 **골든값 회귀 테스트**이며 현재도 정상 통과한다.

두 테스트는 **입력이 다르므로 값이 다른 게 정상**이다. 이전 세션에서 이 둘을
비교하다가 "calibration이 깨졌다"고 오판한 적이 있으니 반복하지 말 것.

### 2-5. `daily-training` 시나리오에 데이터 버그가 있다

[`phase1_real_llm_test.rs`](../../../tests/phase1_real_llm_test.rs) `scenarios()`:

```rust
npc_id: "shu_lien",        // 수련 = 사부 (NPC)
partner_id: "chunxueping", // 춘설병 = 제자 (user 역할)
turns: vec![
    "사부님, 어제 가르쳐주신 검법을 다시 한번 보여주실 수 있나요?",  // 제자 ✅
    "잘 따라하는구나. 이번엔 호흡에 집중해보아라.",                 // 사부 ❌ 역할 뒤바뀜
    "사부님, 감사합니다. 오늘 많이 배웠어요.",                      // 제자 ✅
],
```

`turns`는 **상대(제자)의 발화**여야 하는데 2번이 사부 대사다. 실행 결과 NPC(사부)가
turn 2에서 제자처럼 응답한다("네, 사부님 말씀대로 합니다"). 이 부정합이 생성 대사를
오염시키고, 오염된 transcript가 다시 reflection 입력이 되어 LLM이 "잡담"으로 오판한다.

user 발화는 고정값이므로 **재실행해도 매번 재현**된다.

### 2-6. embed 빌드는 이 환경에서 `cargo clean` 없이 불가

```
1차 (환경변수 없음)  : LNK2005 msvcprt.lib vs libcpmt.lib 충돌, 35초 후 실패
2차 (CFLAGS=/MD CXXFLAGS=/MD): LNK2038 'RuntimeLibrary' 불일치,
                               ort_sys(MD_DynamicRelease) vs esaxx_rs(MT_StaticRelease)
```

환경변수는 **새로 컴파일되는 C 코드에만** 적용되고, `esaxx-rs` 등은 이미 `/MT`로
캐시되어 있다. CLAUDE.md가 말하는 `cargo clean` 필수가 이것. 하지만 clean하면
978개가 돌던 빌드 캐시가 전부 날아가고 재빌드에 수 분이 든다.

**→ reflection 테스트에 embed를 연결하는 것은 비용 대비 실익이 없다고 판단함.**

---

## 3. 커버리지 지도 (이 논의의 결론)

```
사용자 발화
   ↓  analyzer
  PAD                        ← ①  embed_test 담당 (ONNX 필요) ✅
   ↓  Command::ApplyStimulus
 감정 변화 · Beat 전환 판정
   ↓
TurnSnapshot 축적            ← ②  담당 없음 ❌  (진짜 빈 칸)
   ↓  compute_significance
 significance                ← ③  phase1_bench_test 담당 ✅
```

| | 질문 | 담당 | ONNX 필요? |
|---|---|---|---|
| ① | 대사에서 PAD가 제대로 뽑히나 | `embed_test` | 예 |
| **②** | **PAD가 파이프라인을 타고 TurnSnapshot에 실리나** | **없음** | **아니오** |
| ③ | TurnSnapshot으로 점수가 맞게 계산되나 | `phase1_bench_test` | 아니오 |

**②는 ONNX가 필요 없다.** analyzer가 무엇을 돌려주든 그 값이 `ApplyStimulus`를 거쳐
`TurnSnapshot.pad_after` / `beat_changed`에 제대로 실리는지가 관심사이므로,
mock analyzer로 충분하고 **정본 회귀에 편입 가능**하다.

②와 ③의 차이 — ③은 재료를 직접 주입하므로 배선이 끊겨도 통과한다(오늘 실제로 그랬다).
②는 발화만 주고 파이프라인이 재료를 만들게 하므로 중간이 끊기면 잡힌다.

---

## 4. 합의된 작업 4개

### (1) `phase1_real_llm_test` — significance 단언 제거

`ExpectedBand`를 `is_chitchat` 기대값만 갖도록 단순화:

```rust
// 현재
Chitchat     => is_chitchat==true || sig < 0.3
Daily        => is_chitchat==false && 0.3 <= sig < 0.7
Shanshenmiao => is_chitchat==false && sig >= 0.7

// 변경 후
chitchat     => is_chitchat == true
daily        => is_chitchat == false
shanshenmiao => is_chitchat == false
```

significance는 **출력은 유지**하되 `assert` 제거. 왜 이 테스트에서 무의미한지
(§2-3 analyzer 미부착) 주석/로그로 명시해 후속 오해를 막을 것.

### (2) `daily-training` turns[1] 역할 오류 수정

사부 대사를 제자 시점으로 교체. 예: `"이렇게 하는 게 맞나요? 호흡이 자꾸 흐트러집니다."`

수정 후 `is_chitchat=false`로 바뀌는지 확인하면 §2-5 가설이 확정된다.

### (3) ② 배선 검증 통합 테스트 신설 — **정본 회귀 편입**

- mock analyzer (턴마다 다른 PAD 반환) + `MockConversationPort`
  (`tests/common/mock_chat.rs`) + mock `ReflectionRunner`
- `turn_buffers`는 private이므로 **mock `ReflectionRunner`가 받은 `turns`를 캡처**해
  관측하는 방식이 깔끔하다 (Phase 1.5에 Mock ReflectionRunner 선례 있음)
- 참고 선례: `tests/dialogue_converter_integration.rs`의 `ScriptedAnalyzer`
- 단언 예:
  ```rust
  assert!(snapshots[0].pad_after != snapshots[1].pad_after, "턴 사이 PAD 변화");
  assert!(snapshots.iter().any(|s| s.beat_changed), "Beat 전환 기록");
  assert!(significance > 0.3, "실제 흐름에서 유의미한 점수");
  ```
  마지막 줄은 값의 정확성이 아니라 **배선이 죽어있지 않음**을 보는 것.

### (4) [이월] 실 ONNX 기반 significance 측정을 `embed_test`에 추가

`phase1_real_llm_test`에서 빼는 significance 측정의 **제대로 된 버전**을
`embed_test`에 넣는다. `embed_test`는 이미 `--features embed` + 실 ONNX 모델을
쓰므로 자연스러운 자리다. ①+②+③을 실 임베딩으로 관통하는 형태가 된다.

**지금은 하지 않는다** (§2-6 빌드 비용). `embed_test.rs`에 `// TODO:` 주석으로
마커를 남겨 이 문서를 가리킬 것.

---

## 5. 검증 방법

```bash
# 정본 회귀 (978P/0F/3I 유지되어야 함)
cargo test --features chat,mind-studio --lib --tests --bins --no-fail-fast

# 실서버 E2E — llama-server가 127.0.0.1:8081에 떠 있어야 함
NPC_MIND_CHAT_URL=http://127.0.0.1:8081/v1 \
  cargo test --features chat --test phase1_real_llm_test -- --ignored --nocapture

# significance 골든값 (0.000 / 0.461 / 0.980)
cargo test --features chat --test phase1_bench_test -- --nocapture
```

검증 환경 (이 문서 작성 시점):
- llama-server b10223, 모델 `Qwen3.5-9B-The-Defiant-Fable-Uncnr-Heretic-NEO-MAX-Q4_K_M.gguf`
- 서버 `reasoning_format: "none"`, 기본적으로 thinking off
  (단 요청에 `chat_template_kwargs.enable_thinking=true`를 주면 켜짐 — 실측)

---

## 6. 함정 목록 (반복하지 말 것)

1. **`phase1_bench_test`(0.000/0.461/0.980)와 `phase1_real_llm_test`(0.050/0.230/0.390)를
   비교하지 말 것.** 입력이 다른 별개 측정이다.
2. **significance가 같다고 "LLM 동작이 같다"고 결론내지 말 것.** 엔진 결정론 값이다.
3. **`cargo test --features chat`만 돌리고 "통과"라 하지 말 것.** 91개를 놓친다.
4. **`--no-fail-fast` 없이 돌리고 총계를 신뢰하지 말 것.** 첫 실패에서 멈춘다.
5. **`cargo clean`을 가볍게 실행하지 말 것.** embed CRT 문제를 풀려면 필요하지만
   전체 재빌드 비용이 크다.
6. `phase1_real_llm_test`의 `⚠️ DRIFT` 출력은 **테스트 실패가 아니다** (`ok`로 통과).
   밴드 미달을 알리는 경고성 출력이다.

---

## 7. 열린 질문

### ✅ 해결됨 — (2) 수정으로 `daily` 판정이 바뀌는가? → **안 바뀐다**

`turns[1]`을 제자 시점(`"이렇게 하는 게 맞나요? 호흡이 자꾸 흐트러집니다."`)으로
고친 뒤 재실행한 결과:

```
chitchat-passerby      Chitchat     true    ✅
daily-training         Daily        true    ⚠️ DRIFT   ← 여전히 is_chitchat=true
lin-chong-shanshenmiao Shanshenmiao false   ✅
```

생성 대사는 확실히 정상화됐다 — NPC(사부)가 3턴 내내 사부로 일관되게 응답한다
("손가락이 검날에 너무 매몰되지 말아라", "마음이 열려 있으니 배움도 잘 들어오겠지").
그럼에도 LLM 판정은 `is_chitchat=true` 그대로다.

**따라서 §2-5의 데이터 버그는 실재했고 고칠 값어치도 있었지만, `daily` DRIFT의
원인은 아니었다.** 원인은 프롬프트/모델 쪽이다 — 현재 프롬프트가 대사 텍스트만
보고 판단하므로, "사제 간 수련이 관계상 의미 있다"는 판단 근거가 transcript 표면에
드러나지 않는다. 후속 조사 시 이 지점부터 볼 것. (샘플 1회, LLM 샘플링 편차 있음)

`shanshenmiao`는 (1) 적용으로 통과로 바뀌었다 — significance 0.390이 0.7에 못 미쳐
실패하던 것이 단언 제거로 해소됐다. LLM 판정은 원래부터 맞았다.

### 미해결

- (3)의 mock analyzer는 Beat 전환까지는 유발하지 못한다 (`FocusTrigger` 조건이 있는
  Scene이 필요). 현재는 PAD 변화·감정 축적·스냅샷 축적만 검증하고 `beat_signal`
  경로는 미검증으로 남겼다. Focus 시나리오를 붙이면 보강 가능.
- `phase1_real_llm_test`를 E2E 스모크(시나리오 1개, 단언 최소)와 calibration(시나리오
  N개, `is_chitchat`만)으로 완전히 쪼갤지 여부 — 이번 작업 범위에서는 (1)만 하고
  분할은 보류.
