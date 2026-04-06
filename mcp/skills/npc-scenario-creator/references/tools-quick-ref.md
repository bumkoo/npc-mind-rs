# MCP Tools Quick Reference (시나리오 생성 관련)

> 전체 34개 도구 중 시나리오 생성에 주로 사용하는 도구만 발췌.

## 소스 텍스트 & 시나리오 생성

| 도구 | 입력 | 반환 |
|---|---|---|
| `list_source_texts` | 없음 | `{ files: [{ name, path, size_kb }] }` |
| `read_source_text` | `{ path, chapter? }` | chapter 지정 시 텍스트, 생략 시 챕터 목록 |
| `create_full_scenario` | `{ save_path, scenario: { scenario, npcs, relationships, objects, scene } }` | `{ status, path, npcs, relationships }` |

## 세계 구축 CRUD

| 도구 | 입력 | 용도 |
|---|---|---|
| `list_npcs` | 없음 | NPC 목록 + HEXACO 전체 |
| `create_npc` | `{ npc: NpcProfile }` | NPC upsert |
| `list_relationships` | 없음 | 관계 목록 |
| `create_relationship` | `{ rel: RelationshipData }` | 관계 upsert |
| `list_objects` / `create_object` | 동일 패턴 | 오브젝트 CRUD |

## 감정 파이프라인 (검증용)

| 도구 | 입력 | 용도 |
|---|---|---|
| `appraise` | `{ npc_id, partner_id, situation }` | 초기 감정 확인 |
| `generate_guide` | `{ req: { npc_id, partner_id } }` | 연기 가이드 재생성 |

## 시나리오 관리

| 도구 | 입력 | 용도 |
|---|---|---|
| `list_scenarios` | 없음 | 시나리오 파일 목록 |
| `load_scenario` | `{ path }` | 시나리오 로드 |
| `save_scenario` | `{ path, save_type }` | 저장 (scenario/result/report/all) |
| `get_scenario_meta` | 없음 | 로드된 시나리오 메타 |

## Scene 관리

| 도구 | 입력 | 용도 |
|---|---|---|
| `start_scene` | `{ req: SceneRequest }` | Scene 시작 + 초기 appraise |
| `get_scene_info` | 없음 | Focus 상태 조회 |
