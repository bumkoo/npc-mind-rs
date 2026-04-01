"""
NPC Mind Studio MCP Server

Mind Studio HTTP API를 MCP 도구로 노출하여
AI Agent(Claude Code)가 NPC 시나리오를 설계/테스트할 수 있게 합니다.

사전 조건: Mind Studio 서버가 실행 중이어야 합니다.
  cargo run --features mind-studio --bin npc-mind-studio

사용법:
  다른 프로젝트의 .mcp.json에 이 서버를 등록하세요.
  자세한 설정은 mcp/README.md를 참고하세요.
"""

import os
import json
import httpx
from mcp.server.fastmcp import FastMCP

BASE_URL = os.environ.get("MIND_STUDIO_URL", "http://localhost:3000")

mcp = FastMCP(
    "mind-studio",
    instructions="""NPC Mind Studio — NPC 심리 엔진 시뮬레이터.
HEXACO 성격 모델 기반으로 NPC를 생성하고, OCC 감정 이론으로 상황별 감정을 평가하여
LLM 연기 프롬프트를 생성합니다.

일반적인 워크플로우:
1. create_npc → NPC 생성 (HEXACO 24 facets 설정)
2. create_relationship → NPC 간 관계 설정 (closeness/trust/power)
3. start_scene → Scene 시작 (Focus/Beat 등록)
4. appraise → 초기 상황 평가 → 감정 + 프롬프트 확인
5. apply_stimulus → 대사별 PAD 자극 적용 → 감정 변화 + Beat 전환
6. after_dialogue → 대화 종료 → 관계 갱신
7. save_scenario → 시나리오 JSON 저장

HEXACO 6차원 (각 4 facets, -1.0~1.0):
- H(정직-겸손): sincerity, fairness, greed_avoidance, modesty
- E(정서성): fearfulness, anxiety, dependence, sentimentality
- X(외향성): social_self_esteem, social_boldness, sociability, liveliness
- A(원만성): forgiveness, gentleness, flexibility, patience
- C(성실성): organization, diligence, perfectionism, prudence
- O(개방성): aesthetic_appreciation, inquisitiveness, creativity, unconventionality
""",
)


async def _request(method: str, path: str, body: dict | None = None) -> str:
    """Mind Studio HTTP API 호출 헬퍼."""
    async with httpx.AsyncClient(base_url=BASE_URL, timeout=30.0) as client:
        if method == "GET":
            resp = await client.get(path)
        elif method == "POST":
            resp = await client.post(path, json=body)
        elif method == "PUT":
            resp = await client.put(path, json=body)
        elif method == "DELETE":
            resp = await client.delete(path)
        else:
            return f"Unsupported method: {method}"

        if resp.status_code >= 400:
            return f"Error {resp.status_code}: {resp.text}"

        text = resp.text.strip()
        if not text:
            return "OK"
        # JSON이면 보기 좋게 포맷팅
        try:
            return json.dumps(json.loads(text), ensure_ascii=False, indent=2)
        except json.JSONDecodeError:
            return text


# =========================================================================
# CRUD — NPC
# =========================================================================


@mcp.tool()
async def list_npcs() -> str:
    """등록된 모든 NPC 목록을 조회합니다."""
    return await _request("GET", "/api/npcs")


@mcp.tool()
async def create_npc(
    id: str,
    name: str,
    description: str = "",
    # H: 정직-겸손
    sincerity: float = 0.0,
    fairness: float = 0.0,
    greed_avoidance: float = 0.0,
    modesty: float = 0.0,
    # E: 정서성
    fearfulness: float = 0.0,
    anxiety: float = 0.0,
    dependence: float = 0.0,
    sentimentality: float = 0.0,
    # X: 외향성
    social_self_esteem: float = 0.0,
    social_boldness: float = 0.0,
    sociability: float = 0.0,
    liveliness: float = 0.0,
    # A: 원만성
    forgiveness: float = 0.0,
    gentleness: float = 0.0,
    flexibility: float = 0.0,
    patience: float = 0.0,
    # C: 성실성
    organization: float = 0.0,
    diligence: float = 0.0,
    perfectionism: float = 0.0,
    prudence: float = 0.0,
    # O: 개방성
    aesthetic_appreciation: float = 0.0,
    inquisitiveness: float = 0.0,
    creativity: float = 0.0,
    unconventionality: float = 0.0,
) -> str:
    """NPC를 생성하거나 수정합니다.

    HEXACO 24 facets는 각각 -1.0 ~ 1.0 범위입니다.
    생략된 facet은 0.0(중립)으로 설정됩니다.
    """
    body = {k: v for k, v in locals().items()}
    return await _request("POST", "/api/npcs", body)


@mcp.tool()
async def delete_npc(id: str) -> str:
    """NPC를 삭제합니다."""
    return await _request("DELETE", f"/api/npcs/{id}")


# =========================================================================
# CRUD — Relationship
# =========================================================================


@mcp.tool()
async def list_relationships() -> str:
    """등록된 모든 관계를 조회합니다."""
    return await _request("GET", "/api/relationships")


@mcp.tool()
async def create_relationship(
    owner_id: str,
    target_id: str,
    closeness: float = 0.0,
    trust: float = 0.0,
    power: float = 0.0,
) -> str:
    """NPC 간 관계를 생성하거나 수정합니다.

    - closeness: -1.0(적대) ~ 1.0(친밀)
    - trust: -1.0(불신) ~ 1.0(신뢰)
    - power: -1.0(하위) ~ 1.0(상위) — owner 기준
    """
    return await _request(
        "POST",
        "/api/relationships",
        {
            "owner_id": owner_id,
            "target_id": target_id,
            "closeness": closeness,
            "trust": trust,
            "power": power,
        },
    )


# =========================================================================
# CRUD — Object
# =========================================================================


@mcp.tool()
async def list_objects() -> str:
    """등록된 모든 오브젝트를 조회합니다."""
    return await _request("GET", "/api/objects")


@mcp.tool()
async def create_object(
    id: str,
    description: str,
    category: str | None = None,
) -> str:
    """오브젝트(사물/장소/특성)를 생성하거나 수정합니다."""
    body: dict = {"id": id, "description": description}
    if category is not None:
        body["category"] = category
    return await _request("POST", "/api/objects", body)


# =========================================================================
# Emotion Pipeline
# =========================================================================


@mcp.tool()
async def appraise(
    npc_id: str,
    partner_id: str,
    description: str,
    event_description: str | None = None,
    desirability_for_self: float | None = None,
    other_target_id: str | None = None,
    other_desirability: float | None = None,
    prospect: str | None = None,
    action_description: str | None = None,
    agent_id: str | None = None,
    praiseworthiness: float | None = None,
    object_target_id: str | None = None,
    appealingness: float | None = None,
) -> str:
    """상황을 평가하여 OCC 감정을 생성하고 LLM 연기 프롬프트를 반환합니다.

    event, action, object 중 하나 이상을 지정해야 합니다.

    - event: 사건 (desirability_for_self -1.0~1.0, prospect: anticipation/hope_fulfilled/hope_unfulfilled/fear_unrealized/fear_confirmed)
    - action: 행위 (agent_id: None=자기, Some=타인, praiseworthiness -1.0~1.0)
    - object: 대상 (appealingness -1.0~1.0)

    반환값에 prompt 필드가 LLM에 전달할 연기 지시 프롬프트입니다.
    """
    situation: dict = {"description": description}

    if event_description is not None and desirability_for_self is not None:
        event: dict = {
            "description": event_description,
            "desirability_for_self": desirability_for_self,
        }
        if other_target_id is not None and other_desirability is not None:
            event["other"] = {
                "target_id": other_target_id,
                "desirability": other_desirability,
            }
        if prospect is not None:
            event["prospect"] = prospect
        situation["event"] = event

    if action_description is not None and praiseworthiness is not None:
        situation["action"] = {
            "description": action_description,
            "agent_id": agent_id,
            "praiseworthiness": praiseworthiness,
        }

    if object_target_id is not None and appealingness is not None:
        situation["object"] = {
            "target_id": object_target_id,
            "appealingness": appealingness,
        }

    return await _request(
        "POST",
        "/api/appraise",
        {
            "npc_id": npc_id,
            "partner_id": partner_id,
            "situation": situation,
        },
    )


@mcp.tool()
async def apply_stimulus(
    npc_id: str,
    partner_id: str,
    pleasure: float,
    arousal: float,
    dominance: float,
    situation_description: str | None = None,
) -> str:
    """대사의 PAD 자극을 적용하여 감정을 갱신합니다.

    - pleasure: -1.0(불쾌) ~ 1.0(유쾌)
    - arousal: -1.0(차분) ~ 1.0(격앙)
    - dominance: -1.0(복종) ~ 1.0(지배)

    Scene이 활성화된 경우, 감정 조건 충족 시 Beat가 자동 전환됩니다.
    반환값의 beat_changed가 true이면 새 Focus로 전환된 것입니다.
    """
    body: dict = {
        "npc_id": npc_id,
        "partner_id": partner_id,
        "pleasure": pleasure,
        "arousal": arousal,
        "dominance": dominance,
    }
    if situation_description is not None:
        body["situation_description"] = situation_description
    return await _request("POST", "/api/stimulus", body)


@mcp.tool()
async def generate_guide(
    npc_id: str,
    partner_id: str,
    situation_description: str | None = None,
) -> str:
    """현재 감정 상태에서 LLM 연기 가이드를 재생성합니다.

    감정 상태는 유지한 채 프롬프트만 다시 생성할 때 사용합니다.
    """
    body: dict = {"npc_id": npc_id, "partner_id": partner_id}
    if situation_description is not None:
        body["situation_description"] = situation_description
    return await _request("POST", "/api/guide", body)


@mcp.tool()
async def after_dialogue(
    npc_id: str,
    partner_id: str,
    praiseworthiness: float | None = None,
    significance: float | None = None,
) -> str:
    """대화(Scene) 종료 후 관계를 갱신하고 감정을 초기화합니다.

    - praiseworthiness: 상대 행동 평가 (-1.0~1.0)
    - significance: 상황 중요도 (0.0=일상 ~ 1.0=인생을 바꾸는 사건)

    반환값에 before/after 관계 수치가 포함됩니다.
    """
    body: dict = {"npc_id": npc_id, "partner_id": partner_id}
    if praiseworthiness is not None:
        body["praiseworthiness"] = praiseworthiness
    if significance is not None:
        body["significance"] = significance
    return await _request("POST", "/api/after-dialogue", body)


# =========================================================================
# Scene Management
# =========================================================================


@mcp.tool()
async def start_scene(
    npc_id: str,
    partner_id: str,
    description: str,
    focuses_json: str,
) -> str:
    """Scene을 시작하고 Focus(Beat) 옵션 목록을 등록합니다.

    focuses_json은 Focus 배열의 JSON 문자열입니다:
    [
      {
        "id": "focus_id",
        "description": "설명",
        "trigger": null,  // null=Initial(즉시), [[{"emotion":"Fear","above":0.6}]]=조건부
        "event": {"description":"...", "desirability_for_self": -0.7},
        "action": {"description":"...", "agent_id":"npc_id", "praiseworthiness": -0.8},
        "object": null
      }
    ]

    trigger 구조: OR[ AND[condition, ...], AND[...] ]
    - null → Initial Focus (처음에 적용)
    - [[{"emotion":"Anger","above":0.7}]] → Anger > 0.7일 때 전환
    - [[{"emotion":"Fear","absent":true},{"emotion":"Distress","below":0.3}]] → AND 조건
    """
    focuses = json.loads(focuses_json)
    return await _request(
        "POST",
        "/api/scene",
        {
            "npc_id": npc_id,
            "partner_id": partner_id,
            "description": description,
            "focuses": focuses,
        },
    )


@mcp.tool()
async def get_scene_info() -> str:
    """현재 Scene의 Focus 상태를 조회합니다."""
    return await _request("GET", "/api/scene-info")


# =========================================================================
# Scenario Persistence
# =========================================================================


@mcp.tool()
async def save_scenario(path: str) -> str:
    """현재 상태를 JSON 파일로 저장합니다.

    path 예시: "data/my_scenario/scenario.json"
    """
    return await _request("POST", "/api/save", {"path": path})


@mcp.tool()
async def load_scenario(path: str) -> str:
    """저장된 시나리오 JSON을 로드합니다.

    Scene 필드가 있으면 Focus/Beat도 자동 복원됩니다.
    """
    return await _request("POST", "/api/load", {"path": path})


@mcp.tool()
async def list_scenarios() -> str:
    """data/ 폴더에서 사용 가능한 시나리오 목록을 조회합니다."""
    return await _request("GET", "/api/scenarios")


# =========================================================================
# Entry Point
# =========================================================================

if __name__ == "__main__":
    mcp.run()
