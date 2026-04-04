# NPC Mind Engine — 협업 워크플로우 가이드

## 개요

이 문서는 Bekay와 Claude가 NPC 심리 엔진을 **반복적으로 개선**하기 위한 협업 루프를 정의한다.
핵심 도구는 Mind Studio (http://127.0.0.1:3000)이며, Claude는 **MCP(Model Context Protocol) 도구**를 통해 시나리오 로드, 감정 평가, 자극 적용, 보고서 작성까지 자율적으로 수행한다.

> **개선 루프의 모든 단계는 MCP 도구 기반으로 실행됩니다.**
> Claude가 MCP 도구를 직접 호출하여 시나리오를 로드하고, 감정을 평가하고, 결과를 저장합니다.
> Bekay는 브라우저 WebUI를 통해 실시간으로 동일한 상태를 확인하며 협업합니다.

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

## 개선 루프 (Improvement Loop) — MCP 도구 기반

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  ① 장면 선택          "허클베리핀 Ch.8 잭슨 섬 첫 만남"      │
│       │                                                     │
│       ▼                                                     │
│  ② 인물 프로파일 생성   create_npc, create_relationship      │
│       │                                                     │
│       ▼                                                     │
│  ③ 감정 평가 실행       load_scenario → appraise             │
│       │                 → analyze_utterance → apply_stimulus │
│       ▼                                                     │
│  ④ 결과 검증            get_history, get_test_report         │
│       │                 + update_test_report (마크다운)       │
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
│  ⑥ 저장 + 다음 장면     save_scenario → 다음 장면으로 이동   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 각 단계 상세 (MCP 도구 매핑)

### ① 장면 선택
- `list_scenarios` — 사용 가능한 시나리오 목록 조회
- 새 장면이면 scenario.json 작성 후 `load_scenario`

### ② 인물 프로파일 생성
**누가**: Claude (초안) → Bekay (검토/조정)
**MCP 도구**: `create_npc`, `create_relationship`

### ③ 감정 평가 실행
**누가**: Claude (MCP 도구 호출) + Bekay (브라우저에서 실시간 확인)
**산출물**: 감정 상태 + 프롬프트 + 상세 Trace 로그

**MCP 워크플로우:**
1. `load_scenario` — 시나리오 로드
2. `appraise` — 초기 상황 판단 및 감정 생성
3. `analyze_utterance` — 대사 → PAD 자동 분석 (embed feature)
4. `apply_stimulus` — PAD 자극 적용 → 감정 변동 + Beat 전환 체크
5. 3~4를 대사마다 반복

### ④ 결과 검증 및 보고서 작성
**MCP 도구**: `get_history`, `get_test_report`, `update_test_report`
- `get_history` — 전체 턴별 히스토리 (trace + input_pad 포함) 조회
- `update_test_report` — AI 분석 결과를 마크다운 보고서로 기록
- 보고서는 시나리오와 함께 저장되어 추후 분석 근거로 활용

### ⑤ 개선점 식별 및 리팩토링
**안심 리팩토링 원칙**:
- **유닛 테스트**: 도메인 로직 수정 시 `cargo test`로 회귀 테스트 수행
- **통합 테스트**: `handler_tests.rs`를 활용하여 대화 분석 파이프라인의 무결성을 검증
- **아키텍처 준수**: DTO가 저장소에 의존하지 않도록 `SituationService`를 경유하는지 확인

### ⑥ 저장
**MCP 도구**: `save_scenario`
- `save_scenario(path, save_type="scenario")` — 시나리오 원본 저장
- `save_scenario(path)` — 결과 포함 전체 저장

---

## MCP Server 연결

Mind Studio는 Rust 네이티브 SSE 기반 MCP 서버(`/mcp/sse`)를 내장하고 있다.
MCP 프로토콜 핸드셰이크(`initialize`, `notifications/*`, `ping`)를 지원하여
별도의 Python 브릿지 없이 직접 연결된다.

### 클라이언트별 설정

**Claude Code** (`.mcp.json` — 프로젝트 루트):
```json
{
  "mcpServers": {
    "npc-mind-studio": {
      "url": "http://127.0.0.1:3000/mcp/sse"
    }
  }
}
```

**Claude Desktop** (`claude_desktop_config.json`):
Claude Desktop은 stdio 트랜스포트만 네이티브 지원하므로 `mcp-remote` 브릿지를 사용한다.
```json
{
  "mcpServers": {
    "npc-mind-studio": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "http://127.0.0.1:3000/mcp/sse"]
    }
  }
}
```
- 설정 파일 위치: `%APPDATA%\Claude\claude_desktop_config.json`
- 변경 후 Claude Desktop **완전 종료 후 재시작** 필요 (새 대화에서 도구 인식)

### 사전 조건
- Mind Studio 서버 실행 중: `cargo run --features mind-studio,embed --bin npc-mind-studio`
- `--features embed` 필수: `analyze_utterance` 도구에 BGE-M3 임베딩 사용

### 주요 MCP 도구 ↔ 내부 서비스 매핑 (25개)

| MCP 도구 | 내부 처리 주체 | 역할 |
|----------|----------------|------|
| `appraise` | `SituationService` | 상황 DTO → 도메인 변환 후 감정 평가 |
| `apply_stimulus` | `SceneService` | PAD 자극 적용 및 Beat 전환 트리거 체크 |
| `analyze_utterance` | `PadAnalyzer` | 대사 → PAD 자동 분석 (embed feature) |
| `after_dialogue` | `RelationshipService` | 대화 종료 후 관계 수치 최종 갱신 |
| `load_scenario` | `StateInner` | 시나리오 JSON 로드 (NPC/관계/Scene 복원) |
| `save_scenario` | `StateInner` | 현재 상태를 JSON으로 저장 |
| `get_history` | `TurnRecord` (State) | trace 및 input_pad를 포함한 전체 히스토리 |
| `get_test_report` | `State` | 테스트 분석 보고서 조회 |
| `update_test_report` | `State` | AI 분석 결과를 마크다운 보고서로 작성 |
| `create_npc` | `State` | NPC 생성/수정 (HEXACO 24 facets) |
| `create_relationship` | `State` | 관계 생성/수정 (closeness/trust/power) |

### MCP Agent 워크플로우 예시

```
1. load_scenario(path="wuxia_confession/session_001")
   # 시나리오 및 관련 NPC/관계 데이터 로드

2. appraise(npc_id="shu_lien", partner_id="mu_baek", situation={...})
   # 초기 감정 평가 + LLM 프롬프트 생성

3. analyze_utterance(utterance="수련, 나는 그대를 사랑하오.")
   # → PAD 수치 자동 분석

4. apply_stimulus(req={npc_id, partner_id, pleasure, arousal, dominance, ...})
   # → 감정 갱신 + Beat 전환 체크

5. get_history()
   # → 전체 턴 히스토리 확인

6. update_test_report(content="# 테스트 결과 분석\n\n- 수련의 Distress가 ...")
   # → 분석 보고서 기록

7. save_scenario(path="wuxia_confession/session_001_result")
   # → 보고서 포함 전체 결과 저장
```

---

## 프로젝트 발전 로드맵 (현행화)

### Phase 1 & 2: 기초 및 정밀도 (완료)
- HEXACO-OCC 매핑 및 Scene/Beat 자동 전환 시스템 구축 완료.
- **[2026-04]** 애플리케이션 계층 분리 (Mind/Situation/Relationship/Scene) 완료.

### Phase 3: 데이터 무결성 및 가시성 (완료)
- 대화 중 PAD 분석 결과 및 상세 Trace 로그의 완벽한 보존 및 UI 연동.
- 통합 테스트(`handler_tests.rs`) 강화를 통한 리팩토링 안정성 확보.
- **[2026-04-04]** Rust 네이티브 MCP 서버 프로토콜 수정 — `initialize`/`notifications`/`ping` 핸드셰이크 지원.
- **[2026-04-04]** Claude Desktop MCP 연동 검증 완료 (`mcp-remote` 브릿지 경유).

### Phase 4: 가이드 품질 및 청자 변환 (진행 예정)
- 청자 관점 PAD 자동 변환 알고리즘 설계.
- Beat trigger 임계값 완화 (Anger<0.7, Distress<0.6).
- stimulus 상수 튜닝 (MIN_INERTIA 0.30→0.20~0.25, IMPACT_RATE 0.5→0.6).
- 2단계 비선형 PAD 스케일링 (출력 범위 ±0.2 → ±1.0).
- LLM 프롬프트 세밀화.
