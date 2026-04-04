# NPC Mind Studio MCP Server

AI Agent(Claude Code, Claude Desktop, Cline 등)가 Mind Studio를 통해 NPC 시나리오를 설계/테스트할 수 있는 MCP 서버입니다.

## 네이티브 SSE 통합

`mind-studio` 바이너리에는 MCP SSE(Server-Sent Events) 서버가 내장되어 있습니다.
MCP 프로토콜 핸드셰이크(`initialize`, `notifications/*`, `ping`)를 지원하여
별도의 Python 브릿지 없이 직접 연결됩니다.

### 1. Mind Studio 서버 실행

```bash
cargo run --features mind-studio,embed --bin npc-mind-studio
# 기본 주소: http://127.0.0.1:3000
# MCP 엔드포인트: http://127.0.0.1:3000/mcp/sse
# --features embed 필수: analyze_utterance 도구 활성화
```

### 2. 에이전트 도구 설정

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

**Claude Desktop** (`%APPDATA%\Claude\claude_desktop_config.json`):

Claude Desktop은 stdio 트랜스포트만 네이티브 지원하므로 `mcp-remote` 브릿지를 사용합니다.
변경 후 앱 **완전 종료 후 재시작** 필요 (새 대화에서 도구 인식).

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

## 제공되는 도구 (28개)

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
| `analyze_utterance` | 대사 텍스트 → PAD 자동 분석 (embed feature) |
| `generate_guide` | 현재 감정으로 프롬프트 재생성 |
| `after_dialogue` | 대화 종료 → 관계 갱신 + 감정 초기화 |

### Scene 관리
| 도구 | 설명 |
|------|------|
| `start_scene` | Scene 시작: Focus 옵션 등록 + 초기 appraise |
| `get_scene_info` | 현재 Scene Focus 상태 조회 |

### 상태 및 시나리오 관리
| 도구 | 설명 |
|------|------|
| `get_history` | 턴별 히스토리 조회 (trace + input_pad 포함) |
| `get_situation` | 현재 상황 설정 패널 상태 조회 |
| `update_situation` | 상황 설정 패널 상태 저장 |
| `get_test_report` | 테스트 분석 보고서 조회 |
| `update_test_report` | AI 분석 결과를 마크다운 보고서로 작성 |
| `list_scenarios` | 사용 가능한 시나리오 목록 |
| `get_scenario_meta` | 현재 로드된 시나리오 메타 정보 |
| `save_scenario` | 현재 상태를 JSON 파일로 저장 |
| `load_scenario` | 시나리오 JSON 로드 (data/ 자동 보정) |
| `get_save_dir` | 결과 저장 디렉토리 경로 조회 |
| `load_result` | 테스트 결과 로드 (턴 히스토리 포함) |

### 기타
| 도구 | 설명 |
|------|------|
| `get_npc_llm_config` | NPC 성격 기반 LLM 파라미터 (temperature, top_p) |

---
*레거시 Python 브릿지 서버 코드는 `mcp/archive/` 폴더로 이동되었습니다.*
