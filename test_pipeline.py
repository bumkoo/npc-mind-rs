"""Treasure Island - Full Pipeline Test"""
import json, urllib.request

BASE = "http://127.0.0.1:3000"

def api(method, path, body=None):
    data = json.dumps(body).encode("utf-8") if body else None
    req = urllib.request.Request(f"{BASE}{path}", data=data,
        headers={"Content-Type": "application/json"} if data else {}, method=method)
    resp = urllib.request.urlopen(req)
    text = resp.read().decode("utf-8")
    return json.loads(text) if text.strip() else None

def show_emotions(result):
    emotions = result.get("emotions", [])
    for e in emotions:
        etype = e["emotion_type"]
        intensity = e["intensity"]
        ctx = e.get("context", "")[:50]
        print(f"    {etype:15s} {intensity:.3f}  {ctx}")
    dominant = result.get("dominant")
    if dominant:
        print(f"    >>> dominant: {dominant['emotion_type']} ({dominant['intensity']:.3f})")

print("=" * 70)
print("TREASURE ISLAND - Parlour Confrontation")
print("Billy Bones vs Dr. Livesey - Full Pipeline")
print("=" * 70)

# 0. Check initial state (from create_full_scenario)
print("\n[Initial State] after create_full_scenario")
guide = api("POST", "/api/guide", {"npc_id": "billy", "partner_id": "livesey"})
print(f"  Prompt: {len(guide['prompt'])} chars")
show_emotions(guide)

# 1. Appraise (explicit - like WebUI first click)
print("\n[Beat 1] Billy's dominance - explicit appraise")
result = api("POST", "/api/appraise", {
    "npc_id": "billy", "partner_id": "livesey",
    "situation": {
        "description": "Billy demands silence in the parlour",
        "event": {
            "description": "Billy is drunk, singing sea-songs, forcing everyone to listen",
            "desirability_for_self": 0.3
        },
        "action": {
            "description": "Billy slaps the table demanding silence, threatens violence",
            "agent_id": "billy",
            "praiseworthiness": -0.6
        }
    }
})
print(f"  Prompt: {len(result['prompt'])} chars")
show_emotions(result)

# 2. Stimulus: Livesey ignores Billy's command
print("\n[Turn 1] Livesey ignores Billy, keeps talking calmly")
stim1 = api("POST", "/api/stimulus", {
    "npc_id": "billy", "partner_id": "livesey",
    "pleasure": -0.4, "arousal": 0.5, "dominance": -0.3,
    "situation_description": "Livesey completely ignores Billy's silence command"
})
print(f"  Beat changed: {stim1['beat_changed']}")
show_emotions(stim1)

# 3. Stimulus: Billy shouts louder, Livesey unmoved
print("\n[Turn 2] Billy: 'Silence, between decks!' - Livesey unmoved")
stim2 = api("POST", "/api/stimulus", {
    "npc_id": "billy", "partner_id": "livesey",
    "pleasure": -0.6, "arousal": 0.7, "dominance": -0.5,
    "situation_description": "Billy shouts an oath but Livesey does not even flinch"
})
print(f"  Beat changed: {stim2['beat_changed']}")
show_emotions(stim2)

# 4. Stimulus: Livesey calls Billy a dirty scoundrel
print("\n[Turn 3] Livesey: 'the world will be quit of a dirty scoundrel'")
stim3 = api("POST", "/api/stimulus", {
    "npc_id": "billy", "partner_id": "livesey",
    "pleasure": -0.8, "arousal": 0.8, "dominance": -0.7,
    "situation_description": "Livesey publicly calls Billy a dirty scoundrel who will die of rum"
})
print(f"  Beat changed: {stim3['beat_changed']}")
show_emotions(stim3)
if stim3['beat_changed']:
    print(f"  >>> BEAT TRANSITION to: {stim3.get('active_focus_id','?')}")

# 5. If beat not yet changed, push harder
if not stim3.get('beat_changed'):
    print("\n[Turn 4] Livesey threatens hanging - Billy draws knife but backs down")
    stim4 = api("POST", "/api/stimulus", {
        "npc_id": "billy", "partner_id": "livesey",
        "pleasure": -0.9, "arousal": 0.6, "dominance": -0.8,
        "situation_description": "Livesey promises hanging at the assizes, Billy puts up his knife"
    })
    print(f"  Beat changed: {stim4['beat_changed']}")
    show_emotions(stim4)
    if stim4['beat_changed']:
        print(f"  >>> BEAT TRANSITION to: {stim4.get('active_focus_id','?')}")

# 6. After dialogue
print("\n[Scene End] after_dialogue (significance=0.8)")
after = api("POST", "/api/after-dialogue", {
    "npc_id": "billy", "partner_id": "livesey",
    "significance": 0.8
})
b, a = after['before'], after['after']
print(f"  closeness: {b['closeness']:.3f} -> {a['closeness']:.3f} ({a['closeness']-b['closeness']:+.3f})")
print(f"  trust:     {b['trust']:.3f} -> {a['trust']:.3f} ({a['trust']-b['trust']:+.3f})")
print(f"  power:     {b['power']:.3f} -> {a['power']:.3f} ({a['power']-b['power']:+.3f})")

# 7. Turn history
print("\n[History]")
history = api("GET", "/api/history")
print(f"  {len(history)} turns:")
for h in history:
    print(f"    [{h['action']:15s}] {h['label']}")

print("\n" + "=" * 70)
print("Full cycle complete!")
print("=" * 70)
