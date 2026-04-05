import json, urllib.request
BASE = "http://127.0.0.1:3000"
def api(method, path, body=None):
    data = json.dumps(body).encode("utf-8") if body else None
    req = urllib.request.Request(f"{BASE}{path}", data=data,
        headers={"Content-Type": "application/json"} if data else {}, method=method)
    try:
        resp = urllib.request.urlopen(req); text = resp.read().decode("utf-8")
        return resp.status, json.loads(text) if text.strip() else None
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read().decode("utf-8"))

print("=== Check: emotions after create_full_scenario ===")
code, guide = api("POST", "/api/guide", {"npc_id": "billy", "partner_id": "livesey"})
if code == 200 and guide:
    print(f"Status: {code}")
    print(f"Prompt length: {len(guide.get('prompt',''))}")
    emotions = guide.get('emotions', [])
    print(f"Emotions: {len(emotions)}")
    for e in emotions:
        print(f"  {e['emotion_type']:15s} intensity={e['intensity']:.3f}  ctx={e.get('context','')[:50]}")
    print(f"\nPrompt preview:\n{guide['prompt'][:400]}")
else:
    print(f"Status: {code}")
    print(f"Response: {guide}")
