import json, urllib.request
BASE = "http://127.0.0.1:3000"
# Check if chat endpoint exists
try:
    req = urllib.request.Request(f"{BASE}/api/chat/start",
        data=json.dumps({"session_id":"probe","appraise":{"npc_id":"billy","partner_id":"livesey","situation":{"description":"test"}}}).encode("utf-8"),
        headers={"Content-Type": "application/json"}, method="POST")
    resp = urllib.request.urlopen(req)
    print(f"Chat endpoint: {resp.status} OK")
except urllib.error.HTTPError as e:
    body = e.read().decode("utf-8")
    print(f"Chat endpoint: {e.code} -> {body[:200]}")
except Exception as e:
    print(f"Chat endpoint: ERROR -> {e}")
