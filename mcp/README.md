# NPC Mind Studio MCP Server

AI Agent(Claude Code, Cline, Roo Code 등)가 Mind Studio HTTP API를 통해 NPC 시나리오를 설계/테스트할 수 있는 MCP 서버입니다.

## 네이티브 SSE 통합 (추천)

`mind-studio` 바이너리에는 MCP SSE(Server-Sent Events) 서버가 내장되어 있습니다. 별도의 파이썬 브릿지 없이 서버 자체 엔드포인트에 직접 연결하여 사용합니다.

### 1. Mind Studio 서버 실행

```bash
# npc-mind-rs 프로젝트 루트에서
cargo run --features mind-studio --bin npc-mind-studio
# 기본 주소: http://127.0.0.1:3000
# MCP 엔드포인트: http://127.0.0.1:3000/mcp/sse
```

### 2. 에이전트 도구 설정 (.mcp.json)

AI Agent가 작동할 프로젝트의 루트에 `.mcp.json` 파일을 생성하거나 업데이트합니다:

```json
{
  "mcpServers": {
    "npc-mind-studio": {
      "url": "http://127.0.0.1:3000/mcp/sse"
    }
  }
}
```

## 제공되는 도구 (23개)

### 세계 구축 (CRUD)

| 도구 | 설명 |
|------|------|
| `list_npcs` | NPC 목록 조회 |
| `create_npc` | NPC 생성/수정 (HEXACO 24 facets) |
| `delete_npc` | NPC 삭제 |
| `list_relationships` | 관계 목록 조회 |
| `create_relationship` | 관계 생성/수정 (closeness/trust/power) |
| `delete_relationship` | 관계 삭제 |
| `list_objects` | 오브젝트 목록 조회 |
| `create_object` | 오브젝트 생성/수정 |
| `delete_object` | 오브젝트 삭제 |

### 감정 파이프라인

| 도구 | 설명 |
|------|------|
| `appraise` | 상황 평가 → OCC 감정 + LLM 프롬프트 생성 |
| `apply_stimulus` | PAD 자극 적용 → 감정 갱신 + Beat 전환 |
| `generate_guide` | 현재 감정으로 프롬프트 재생성 |
| `after_dialogue` | 대화 종료 → 관계 갱신 + 감정 초기화 |

### 대사 분석

| 도구 | 설명 |
|------|------|
| `analyze_utterance` | 대사 텍스트 → PAD 자동 분석 (embed feature 필요) |

### Scene 관리

| 도구 | 설명 |
|------|------|
| `start_scene` | Scene 시작 (Focus/Beat 등록 + 초기 감정 평가) |
| `get_scene_info` | 현재 Scene Focus 상태 조회 |

### 상태 조회

| 도구 | 설명 |
|------|------|
| `get_history` | 턴별 히스토리 조회 (감정 변화 추적/디버깅) |
| `get_situation` | 현재 상황 설정 패널 상태 조회 |
| `update_situation` | 상황 설정 패널 상태 저장 (WebUI 동기화) |
| `get_scenario_meta` | 현재 로드된 시나리오 메타 정보 |

### 시나리오 관리

| 도구 | 설명 |
|------|------|
| `save_scenario` | 현재 상태를 JSON 파일로 저장 |
| `load_scenario` | 시나리오 JSON 로드 |
| `list_scenarios` | 사용 가능한 시나리오 목록 |

## AI Agent 워크플로우 예시

에이전트에게 다음과 같은 흐름으로 작업을 지시할 수 있습니다:

1.  **NPC 생성**: `create_npc`로 실험할 캐릭터들의 성격을 정의합니다.
2.  **관계 설정**: `create_relationship`으로 캐릭터 간의 초기 호감도와 신뢰도를 설정합니다.
3.  **상황 시작**: `start_scene`으로 구체적인 갈등 상황과 Beat 전환 조건(Focus)을 입력합니다.
4.  **반응 검토**: 생성된 `prompt`를 보고 NPC의 심리 상태가 의도한 성격대로 형성되었는지 확인합니다.
5.  **대화 진행**: `apply_stimulus`로 상대의 대사에 따른 감정 변화를 시뮬레이션하고, Beat 전환이 발생하는지 관찰합니다.
6.  **결과 저장**: `save_scenario`를 통해 시뮬레이션 결과를 파일로 기록합니다.

---
*레거시 Python 브릿지 서버 코드는 `mcp/archive/` 폴더로 이동되었습니다.*
