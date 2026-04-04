# NPC Mind Engine — 협업 워크플로우 가이드

## 개요

이 문서는 Bekay와 Claude가 NPC 심리 엔진을 **반복적으로 개선**하기 위한 협업 루프를 정의한다.
핵심 도구는 Mind Studio (http://127.0.0.1:3000)이며, Claude는 **SSE 방식의 네이티브 MCP 도구**를 사용하여 Bekay와 실시간으로 데이터를 공유하며 협업한다.

---

## 용어 정의

> 용어(Scene, Beat, Utterance)의 정의와 엔진 호출 매핑은 [CLAUDE.md 용어 정의](../../CLAUDE.md#용어-정의) 참조.

**구조 관계:**
```
도서 (허클베리 핀)
 └── 챕터 (Ch.15)
      └── Scene (안개/Trash)    ← 하나의 대화 단위
           ├── Beat 1          ← 감정 전환 비트, appraise 1회
           │    ├── Utterance   ← 대사, stimulus 입력 (분석된 PAD 기록됨)
           │    └── Utterance
           ├── Beat 2          ← 자동 전환 시 Trace 로그 생성
           │    └── Utterance
           └── Scene 종료      ← after_dialogue (관계 갱신)
```

---

## 개선 루프 (Improvement Loop)

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  ① 장면 선택          "허클베리핀 Ch.8 잭슨 섬 첫 만남"      │
│       │                                                     │
│       ▼                                                     │
│  ② 인물 프로파일 생성   HEXACO 24 facet 설계 + 관계 설정     │
│       │                                                     │
│       ▼                                                     │
│  ③ 감정 평가 실행       상황 설정 → 감정 결과 + 프롬프트      │
│       │                                                     │
│       ▼                                                     │
│  ④ 결과 검증            감정 타당성, 프롬프트 품질, Trace 확인│
│       │                 + 테스트 보고서 작성 (마크다운)       │
│       ▼                                                     │
│  ⑤ 개선점 식별          무엇을 고쳐야 하는가?                │
│       │                                                     │
│       ├── 프로파일 문제  → ② 로 복귀                         │
│       ├── 가중치 문제    → 엔진 코드 수정 → ③ 재실행         │
│       ├── PAD 앵커 문제  → 앵커 문장 개선 → ③ 재실행         │
│       ├── 가이드 문제    → directive 로직 수정 → ③ 재실행    │
│       ├── 도구 문제      → Mind Studio 기능 개선 → ③ 재실행  │
│       └── 만족          → ⑥ 저장                            │
│                                                             │
│       ▼                                                     │
│  ⑥ 저장 + 다음 장면     session 저장 → 다음 장면으로 이동    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 각 단계 상세

### ① 장면 선택
(기존과 동일)

### ② 인물 프로파일 생성
**누가**: Claude (초안) → Bekay (검토/조정)
**도구**: `create_npc`, `create_relationship` MCP 도구 사용

### ③ 감정 평가 실행
**누가**: Claude (MCP 도구 호출) + Bekay (브라우저 조작)
**산출물**: 감정 상태 + 프롬프트 + **상세 Trace 로그**

**대사 PAD 측정 및 기록**:
- **자동 분석 (`--features embed`)**: 사용자의 대사가 BGE-M3 모델로 임베딩 분석되어 PAD 수치가 자동 산출됩니다.
- **데이터 흐름**: 분석된 PAD 값은 `input_pad` 필드에 담겨 히스토리에 영구 보존되며, UI 슬라이더에 즉시 반영됩니다.

### ④ 결과 검증 및 보고서 작성
**Trace 및 Report 탭 활용**: 
- `Trace` 탭을 통해 엔진의 상세 계산 과정을 검토합니다. (예: `→ Joy: base_val=0.5, result=0.15 [맥락]`)
- **테스트 보고서 (NEW)**: AI가 테스트 결과를 마크다운으로 정리하여 `Report` 탭에 기록합니다. 
- 이 보고서는 시나리오와 함께 저장되어 추후 분석 근거로 활용됩니다.

### ⑤ 개선점 식별 및 리팩토링
**안심 리팩토링 원칙**:
- **유닛 테스트**: 도메인 로직 수정 시 `cargo test`로 회귀 테스트 수행.
- **통합 테스트**: `handler_tests.rs`를 활용하여 대화 분석 파이프라인의 무결성을 검증.
- **아키텍처 준수**: DTO가 저장소에 의존하지 않도록 `SituationService`를 경유하는지 확인.

---

## MCP Server (AI Agent 연동)

Claude Code 등 AI Agent가 Mind Studio를 자율적으로 사용할 때는 **SSE(Server-Sent Events)** 방식을 통해 실시간으로 연결합니다.

### 설정 (.mcp.json)

별도의 파이썬 브릿지 없이 서버 자체 엔드포인트에 직접 연결합니다.

```json
{
  "mcpServers": {
    "npc-mind-studio": {
      "url": "http://127.0.0.1:3000/mcp/sse"
    }
  }
}
```

### 주요 MCP 도구 ↔ 내부 서비스 매핑

| MCP 도구 | 내부 처리 주체 | 역할 |
|----------|----------------|------|
| `appraise` | `SituationService` | 상황 DTO → 도메인 변환 후 평가 |
| `apply_stimulus` | `SceneService` | 자극 적용 및 Beat 전환 트리거 체크 |
| `after_dialogue` | `RelationshipService` | 대화 종료 후 관계 수치 최종 갱신 |
| `get_test_report` | `State` | 현재 테스트 분석 보고서 조회 |
| `update_test_report` | `State` | AI 분석 결과를 마크다운 보고서로 작성 |
| `get_history` | `TurnRecord` (State) | `trace` 및 `input_pad`를 포함한 전체 히스토리 로드 |

### MCP Agent 워크플로우 예시

```
1. load_scenario(path="huckleberry_finn/session_001")
   # 시나리오 및 관련 NPC/관계 데이터 로드

2. analyze_utterance(utterance="정말 실망이야!")
   # → 분석된 PAD 수치 확인

3. apply_stimulus(utterance="정말 실망이야!", ...)
   # → 감정 갱신 및 히스토리에 input_pad 기록 확인

4. update_test_report(content="# 테스트 결과 분석\n\n- 헉의 죄책감이 의도대로 상승함...")
   # → 테스트 분석 내용 기록

5. save_scenario(path="huckleberry_finn/session_001_result")
   # → 보고서를 포함한 전체 결과 저장
```

---

## 프로젝트 발전 로드맵 (현행화)

### Phase 1 & 2: 기초 및 정밀도 (완료)
- HEXACO-OCC 매핑 및 Scene/Beat 자동 전환 시스템 구축 완료.
- **[2026-04]** 애플리케이션 계층 분리 (Mind/Situation/Relationship/Scene) 완료.

### Phase 3: 데이터 무결성 및 가시성 (진행 중)
- **[진행 중]** 대화 중 PAD 분석 결과 및 상세 Trace 로그의 완벽한 보존 및 UI 연동.
- **[진행 중]** 통합 테스트(`handler_tests.rs`) 강화를 통한 리팩토링 안정성 확보.

### Phase 4: 가이드 품질 및 청자 변환
- LLM 프롬프트 세밀화 및 화자 톤 → 청자 자극 변환 알고리즘 설계.
