# 기억 중요도 라벨 설계서 (Memory Importance Label Design)

**버전:** v1.3  
**수정일:** 2026-02-22 05:30:00

---

## 1. 설계 배경

### 1.1 현재 시스템 문제점

기억 중요도가 시스템 내부에서 세 가지 역할을 수행하지만, 각 단계에 공백이 있다.

| 단계 | 현재 상태 | 문제 |
|------|-----------|------|
| 생성 시 (MemoryEntry::new) | importance를 호출자가 수동 부여 (기본 7.0) | 맥락 기반 평가 없음 |
| 검색 시 (retrieval_score) | importance/10.0을 점수에 반영 | 정상 작동 |
| 프롬프트 삽입 시 (format_memories_for_prompt) | `[중요도: 7]` 숫자 표시 | LLM이 메타 태그를 대사에 노출할 위험 |

### 1.2 목표

- 대화 종료 시 LLM이 중요도를 자동 산정하도록 한다
- 프롬프트에 삽입되는 기억에 **자기설명적 라벨**을 붙여 LLM의 반응 품질을 높인다
- 내부 정밀도(f32, 1.0~10.0)를 유지하면서 LLM에게는 5단계 자연어로 전달한다

---

## 2. 중요도 산정 시점 결정

### 2.1 기억 생명주기별 시점

| 시점 | 설명 | 중요도 필요? | 방법 |
|------|------|:---:|------|
| ① 미시요약 (Compress) | 대화 중 ctx 압축용 요약 | ❌ | 기술적 장치이므로 불필요 |
| ② 대화 종료 요약 (session.end) | 영구 기억(Observation)으로 저장 | ✅ | LLM 1회 호출로 요약+중요도 동시 산정 |
| ③ Observation 생성 (MemoryEntry::new) | 기억 저장 | ✅ | ②의 결과를 사용 |
| ④ 기억 검색 (rank_memories) | retrieval_score 계산 | - | 기존 importance/10.0 유지 |
| ⑤ 프롬프트 삽입 (format_memories) | NPC 대화 생성용 | ✅ | 5단계 자기설명적 라벨로 변환 |
| ⑥ Tier 2 일상 성찰 (하루 끝) | 기존 기억 재평가 | ✅ | LLM 재평가 (향후) |
| ⑦ Tier 3 전환점 (중대 사건) | 기존 기억 재해석 | ✅ | LLM 재평가 (향후) |

### 2.2 핵심 결정

- **미시요약(①):** 중요도 산정 불필요 → 요약 품질에만 집중
- **최종 Observation(②):** LLM 1회 호출로 요약+중요도 동시 산정 (추가 비용 0)
- **파싱 실패 안전장치:** 기본값 5.0

---

## 3. 자기설명적 라벨 5단계

### 3.1 설계 원칙

라벨은 **NPC의 행동을 직접 지시**한다. 4B 모델은 추론 단계를 줄일수록 성능이 높다.

기존 방식의 문제:
- `[중요도: 7]` → 숫자가 대사에 노출될 위험
- `[떠올리면 지금도 감정이 올라오는 기억]` → 자기설명적이지만, 4B가 "그래서 어떻게 말해야 하지?"를 추론해야 함

v1.3 해결 (4B 최적화):
- 라벨에서 **상태 설명을 제거**하고 **행동 지시만** 남김
- "얼버무려라", "말을 끊거나 침묵해라" 등 직접 명령형
- 4B의 추론 부담 제거: "무엇" → "어떻게"의 추론 단계 0

### 3.2 5단계 라벨 정의

| 중요도 | 라벨 (ko) | 라벨 (en) | 행동 의도 |
|:---:|---|---|---|
| 1.0~2.9 | 얼버무려라 | mumble vaguely | 기억 불확실, 모호하게 |
| 3.0~4.9 | 사실만 담담히 말해라 | state facts only | 감정 없이 정보 전달 |
| 5.0~6.9 | 회상하듯 말해라 | speak as if reminiscing | 과거 감정 회상 |
| 7.0~8.9 | 말투가 흔들려라 | let speech waver | 현재 감정 영향 |
| 9.0~10.0 | 말을 끊거나 침묵해라 | go silent or cut short | 압도/각인 |

### 3.3 인접 레벨 구분 근거

| 구간 | 구분 축 | 예시 대사 차이 |
|------|---------|---------------|
| 1단계 vs 2단계 | "모른다" vs "안다" | "뭔가 있었던 것 같기도..." vs "그래, 네가 약재 물어봤잖아." |
| 2단계 vs 3단계 | "사실만" vs "감정도" | "서문 쪽 움직임 얘기했지." vs "처음 봤을 때 좀 경계했었어." |
| 3단계 vs 4단계 | "과거 감정" vs "현재 감정" | "그때 고마웠어." vs "...그 얘기 하니까 또 그러네." |
| 4단계 vs 5단계 | "감정 표현" vs "감정에 압도" | "지금 생각해도 화가 나." vs "............그 얘긴 하지 마." |

### 3.4 라벨이 행동 지시인 이유 (v1.3)

v1.2까지는 "자기설명적 라벨"이었다 ("떠올리면 지금도 감정이 올라오는 기억").
v1.3에서 4B 모델 최적화를 위해 **행동 지시만** 남겼다.

- 12B: 상태 설명 → 행동 추론 가능 ("감정이 올라오는 기억" → "말투가 흔들려야겠다")
- 4B: 상태 설명 → 행동 추론 **불가** → 행동을 직접 지시해야 함

라벨과 기억 내용 사이 거리가 0이므로 (바로 옆), 4B도 즉시 적용할 수 있다.

---

## 4. 프롬프트 삽입 형태

### 4.1 최종 형태

```
[관련 기억]
- (12년 전) 혈교 습격으로 형제들을 잃었다.
  [말을 끊거나 침묵해라]
- (어제) 플레이어가 위기에서 도와줬다.
  [말투가 흔들려라]
- (3일 전) 서문에서 수상한 사내를 보았다.
  [사실만 담담히 말해라]
```

### 4.2 토큰 비용

| 항목 | 토큰 |
|------|:---:|
| 도입 문구 | 0 (불필요) |
| 행동 지시 | 0 (라벨에 내장) |
| 기억 1개당 라벨 추가 | ~10 |
| 기억 5개 총 비용 | ~50 |
| ctx 8192 대비 | ~0.6% |

---

## 5. prompt_config.toml 설계

```toml
# ═══════════════════════════════════════════
# 기억 중요도 라벨 (자기설명적)
# ═══════════════════════════════════════════

# 중요도 → 라벨 매핑 경계값 (이상)
[memory_labels.thresholds]
level_1 = 1.0    # 1.0 ~ 2.9
level_2 = 3.0    # 3.0 ~ 4.9
level_3 = 5.0    # 5.0 ~ 6.9
level_4 = 7.0    # 7.0 ~ 8.9
level_5 = 9.0    # 9.0 ~ 10.0

# 프롬프트용 5단계 라벨 (한국어) — 행동 지시형
[memory_labels.ko]
level_1 = "얼버무려라"
level_2 = "사실만 담담히 말해라"
level_3 = "회상하듯 말해라"
level_4 = "말투가 흔들려라"
level_5 = "말을 끊거나 침묵해라"

# 프롬프트용 5단계 라벨 (영어) — 행동 지시형
[memory_labels.en]
level_1 = "mumble vaguely"
level_2 = "state facts only"
level_3 = "speak as if reminiscing"
level_4 = "let speech waver"
level_5 = "go silent or cut short"

# UI용 10단계 라벨 (향후 구현)
[memory_labels_ui.ko]
level_1 = "흐릿한 기억"
level_2 = "희미한 기억"
level_3 = "어렴풋한 기억"
level_4 = "남아있는 기억"
level_5 = "기억나는 일"
level_6 = "마음에 남은 기억"
level_7 = "깊이 남은 기억"
level_8 = "강렬한 기억"
level_9 = "뼈에 새긴 기억"
level_10 = "잊을 수 없는 기억"

[memory_labels_ui.en]
level_1 = "hazy memory"
level_2 = "faint memory"
level_3 = "dim memory"
level_4 = "remaining memory"
level_5 = "remembered event"
level_6 = "heartfelt memory"
level_7 = "deep memory"
level_8 = "intense memory"
level_9 = "searing memory"
level_10 = "unforgettable memory"
```

---

## 6. ObservationDraft 구조체 설계

### 6.1 현재: session.end() 반환값

```rust
pub fn end(&mut self) -> Result<String, LlmError>
// 요약 텍스트만 반환. 중요도 정보 없음.
```

### 6.2 개선: ObservationDraft 반환

```rust
/// 대화 종료 시 생성되는 관찰 초안.
/// MemoryEntry 생성의 중간 산출물.
pub struct ObservationDraft {
    /// LLM이 생성한 요약 텍스트
    pub summary: String,
    /// LLM이 평가한 중요도 (1.0~10.0, 파싱 실패 시 5.0)
    pub importance: f32,
    /// 총 대화 턴 수
    pub turn_count: usize,
    /// 압축(미시요약)이 발생했는지
    pub had_compression: bool,
}

pub fn end(&mut self) -> Result<ObservationDraft, LlmError>
```

### 6.3 ObservationDraft → MemoryEntry 변환 흐름

```rust
let draft = session.end()?;

let entry = MemoryEntry::new(
    next_id(),
    npc_character_id,
    draft.summary,        // ← 요약 텍스트
    draft.importance,     // ← LLM이 평가한 중요도
    MemoryType::Observation,
    current_game_time,
    extract_keywords(&draft.summary),
);

memory_repo.save(entry)?;
```

---

## 7. 요약+중요도 통합 프롬프트 설계

### 7.1 LLM 호출 전략

1회 호출로 요약과 중요도를 동시에 산정한다. (RTX 2070S, 7tok/s 환경에서 2회 호출은 비현실적)

### 7.2 요약 요청 프롬프트 (안)

```
너는 대화 요약 도우미다.
1. 아래 대화의 핵심 내용을 2~3문장 한국어로 요약해라.
2. 마지막 줄에 이 대화의 중요도를 1~10으로 평가해라.

중요도 기준:
1~3: 일상 잡담 (날씨, 인사, 음식)
4~6: 정보 교환 (위치, 인물, 소문)
7~8: 관계 변화 (호감, 갈등, 약속)
9~10: 극적 사건 (배신, 죽음, 비급, 정체 노출)

형식:
[요약 내용]
[importance: N]
```

### 7.3 파싱 로직 (의사코드)

```rust
fn parse_summary_with_importance(response: &str) -> (String, f32) {
    // 마지막 줄에서 [importance: N] 패턴 추출
    // 정규식: \[importance:\s*(\d+(?:\.\d+)?)\]
    
    if let Some(n) = extract_importance(response) {
        let summary = remove_importance_line(response);
        (summary, n.clamp(1.0, 10.0))
    } else {
        // 파싱 실패 → 기본값 5.0
        (response.to_string(), 5.0)
    }
}
```

---

## 8. 코드 변경 포인트

| 순번 | 파일 | 변경 내용 | 단계 |
|:---:|------|-----------|:---:|
| 1 | `session.rs` :: `build_summary_request()` | 요약 프롬프트에 중요도 평가 지시 추가 | A |
| 2 | `session.rs` :: `request_summary()` | 응답에서 `[importance: N]` 파싱, `(String, f32)` 반환 | A |
| 3 | `session.rs` :: `end()` | 반환 타입을 `ObservationDraft`로 변경 | A |
| 4 | `conversation/mod.rs` 또는 신규 파일 | `ObservationDraft` 구조체 정의 | A |
| 5 | `template.rs` :: `format_memories_for_prompt()` | 숫자 `[중요도: N]` → 자기설명적 라벨로 변환 | B |
| 6 | `prompt_config.toml` | `memory_labels` 섹션 추가 | B |
| 7 | `i18n` 관련 | 라벨의 ko/en 지원 | B |

### 구현 순서

- **Stage A (중요도 산정):** 변경 포인트 1~4
- **Stage B (라벨 표현):** 변경 포인트 5~7
- Stage A와 B는 독립적으로 구현 가능하나, A → B 순서가 자연스러움

---

## 9. OCC 감정 기반 자동 중요도 (향후 — Stage C)

### 9.1 개요

OCC 감정 시스템 구현 후, 감정 강도로 중요도를 자동 산정하는 경로.

### 9.2 공식

```
importance = base(3.0) + Σ(emotion.intensity × relevant_value)
```

### 9.3 예시

소연이 "조고의 배신"을 경험:
- base = 3.0
- 분노(0.55) × 의(0.8) = +4.4
- importance = 7.4

### 9.4 관련 OCC_TODO 마커

| 마커 | 내용 |
|------|------|
| OCC_TODO① | importance auto-calculation from emotion intensity |
| OCC_TODO② | PAD mood-congruent memory bias |
| OCC_TODO③ | 5-value relevance amplification |
| OCC_TODO④ | Add valence field to MemoryEntry |

---

## 10. 변경 이력

| 버전 | 변경일시 | 변경 내역 |
|:---:|----------|-----------|
| v1.0 | 2026-02-21 22:30:00 | 초안 작성. 5단계 자기설명적 라벨 확정, ObservationDraft 구조체 설계, 프롬프트 형태 확정 |
| v1.1 | 2026-02-22 00:15:00 | Stage A 구현 완료. ObservationDraft, build_final_summary_request(), parse_summary_with_importance() 추가. end() 반환 타입 변경. 테스트 9개 추가 + 4개 수정, 전체 통과 |
| v1.2 | 2026-02-22 02:30:00 | Stage B 구현 완료. MemoryLabelsConfig 구조체 + importance_to_label() 추가. prompt_config.toml에 memory_labels 섹션 추가. format_memories_for_prompt()에서 숫자 → 자기설명적 라벨 변환. 테스트 6개 추가 + 6개 수정, 전체 통과 |
| v1.3 | 2026-02-22 05:30:00 | 4B 모델 최적화: 자기설명적 라벨 → 행동 지시형 라벨로 전환. 상태 설명 제거, "~해라" 명령형만 남김. prompt_config.toml ko/en 10줄, template.rs 테스트/주석 12곳, 설계문서 §3.1~§5 반영. 코드 로직 변경 0줄 (TOML only) |
