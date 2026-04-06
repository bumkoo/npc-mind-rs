#!/usr/bin/env python3
"""Grade scenario JSON files against assertions."""
import json
import sys
import os

def grade_scenario(path):
    """Grade a scenario JSON against quality assertions."""
    try:
        with open(path, 'r', encoding='utf-8') as f:
            data = json.load(f)
    except Exception as e:
        return [{"text": f"File load failed: {e}", "passed": False, "evidence": str(e)}]

    results = []

    # 1. Silver HEXACO not all zeros
    silver = data.get("npcs", {}).get("silver", {})
    facets = ["sincerity", "fairness", "greed_avoidance", "modesty", "fearfulness",
              "anxiety", "dependence", "sentimentality", "social_self_esteem",
              "social_boldness", "sociability", "liveliness", "forgiveness",
              "gentleness", "flexibility", "patience", "organization", "diligence",
              "perfectionism", "prudence", "aesthetic_appreciation", "inquisitiveness",
              "creativity", "unconventionality"]

    non_zero = sum(1 for f in facets if silver.get(f, 0) != 0)
    results.append({
        "text": "Silver HEXACO facets are properly defined (not all zero)",
        "passed": non_zero >= 20,
        "evidence": f"{non_zero}/24 facets are non-zero"
    })

    # 2. Jim NPC exists
    jim = data.get("npcs", {}).get("jim", {})
    jim_exists = bool(jim.get("id"))
    results.append({
        "text": "Jim NPC is included in scenario",
        "passed": jim_exists,
        "evidence": f"jim id: {jim.get('id', 'MISSING')}"
    })

    # 3. Relationships exist (both directions)
    rels = data.get("relationships", {})
    has_silver_jim = "silver:jim" in rels
    has_jim_silver = "jim:silver" in rels
    results.append({
        "text": "Bidirectional relationships defined (silver:jim AND jim:silver)",
        "passed": has_silver_jim and has_jim_silver,
        "evidence": f"silver:jim={has_silver_jim}, jim:silver={has_jim_silver}"
    })

    # 4. Scenario metadata not empty
    scenario = data.get("scenario", {})
    has_name = bool(scenario.get("name", "").strip())
    has_desc = bool(scenario.get("description", "").strip())
    results.append({
        "text": "Scenario metadata (name, description) is populated",
        "passed": has_name and has_desc,
        "evidence": f"name='{scenario.get('name', '')[:50]}', desc length={len(scenario.get('description', ''))}"
    })

    # 5. Scene has 3 focuses
    scene = data.get("scene", {})
    focuses = scene.get("focuses", [])
    results.append({
        "text": "Scene has 3 Beat focuses",
        "passed": len(focuses) == 3,
        "evidence": f"focus count: {len(focuses)}"
    })

    # 6. First focus has null trigger (Initial)
    if focuses:
        first_trigger = focuses[0].get("trigger")
        results.append({
            "text": "First Beat has null/Initial trigger",
            "passed": first_trigger is None,
            "evidence": f"trigger value: {first_trigger}"
        })

    # 7. Non-first focuses have condition triggers
    valid_triggers = True
    trigger_evidence = []
    for f in focuses[1:]:
        t = f.get("trigger")
        if not isinstance(t, list) or len(t) == 0:
            valid_triggers = False
            trigger_evidence.append(f"{f.get('id')}: invalid trigger")
        else:
            trigger_evidence.append(f"{f.get('id')}: OK ({len(t)} paths)")
    results.append({
        "text": "Non-initial focuses have valid condition triggers (OR[AND[]] structure)",
        "passed": valid_triggers and len(focuses) > 1,
        "evidence": "; ".join(trigger_evidence) if trigger_evidence else "no non-initial focuses"
    })

    # 8. Descriptions are in Korean
    korean_chars = 0
    total_chars = 0
    for npc in data.get("npcs", {}).values():
        desc = npc.get("description", "")
        total_chars += len(desc)
        korean_chars += sum(1 for c in desc if '\uac00' <= c <= '\ud7a3')

    for f in focuses:
        desc = f.get("description", "")
        total_chars += len(desc)
        korean_chars += sum(1 for c in desc if '\uac00' <= c <= '\ud7a3')

    korean_ratio = korean_chars / max(total_chars, 1)
    results.append({
        "text": "Descriptions are in Korean (>20% Korean characters)",
        "passed": korean_ratio > 0.2,
        "evidence": f"Korean ratio: {korean_ratio:.1%} ({korean_chars}/{total_chars})"
    })

    # 9. Silver is npc_id (scene perspective)
    npc_id = scene.get("npc_id", "")
    results.append({
        "text": "Scene npc_id is 'silver' (Silver's perspective)",
        "passed": npc_id == "silver",
        "evidence": f"npc_id: {npc_id}"
    })

    # 10. agent_id usage makes sense
    agent_issues = []
    for f in focuses:
        action = f.get("action", {})
        agent = action.get("agent_id", "")
        pw = action.get("praiseworthiness", 0)
        fid = f.get("id", "")
        if agent == "silver" and pw > 0:
            # Silver praising himself → Pride
            pass
        elif agent != "silver" and agent and pw > 0:
            # Others praising → Admiration
            pass
        elif agent and pw < 0:
            # Negative → Reproach/Shame
            pass
    results.append({
        "text": "agent_id is set on all focus actions",
        "passed": all(f.get("action", {}).get("agent_id") for f in focuses),
        "evidence": ", ".join(f"{f.get('id')}: agent={f.get('action',{}).get('agent_id','NONE')}" for f in focuses)
    })

    return results

def main():
    workspace = "/sessions/zealous-clever-davinci/mnt/npc-mind-rs/mcp/skills/npc-scenario-creator-workspace/iteration-1"

    scenarios = {
        "with_skill": "/sessions/zealous-clever-davinci/mnt/npc-mind-rs/data/treasure_island/ch28_silvers_gambit/실버의도박.json",
        "without_skill": "/sessions/zealous-clever-davinci/mnt/npc-mind-rs/data/treasure_island/ch28_silvers_gambit/실버의도박_baseline.json"
    }

    for variant, path in scenarios.items():
        results = grade_scenario(path)

        grading = {
            "eval_id": 1,
            "eval_name": "silvers-gambit",
            "variant": variant,
            "expectations": results,
            "pass_rate": sum(1 for r in results if r["passed"]) / len(results) if results else 0
        }

        out_dir = os.path.join(workspace, f"eval-silvers-gambit-{variant}")
        with open(os.path.join(out_dir, "grading.json"), 'w', encoding='utf-8') as f:
            json.dump(grading, f, indent=2, ensure_ascii=False)

        print(f"\n=== {variant} ===")
        print(f"Pass rate: {grading['pass_rate']:.0%}")
        for r in results:
            status = "✅" if r["passed"] else "❌"
            print(f"  {status} {r['text']}")
            print(f"     → {r['evidence']}")

if __name__ == "__main__":
    main()
