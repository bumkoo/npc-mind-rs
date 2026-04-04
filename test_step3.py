"""Step 3: New MCP tools verification"""
import json, urllib.request, threading, time

BASE = "http://127.0.0.1:3000"

sse_lines = []
def sse_reader():
    req = urllib.request.Request(f"{BASE}/mcp/sse")
    resp = urllib.request.urlopen(req, timeout=30)
    for line in resp:
        sse_lines.append(line.decode("utf-8").strip())
        if len(sse_lines) > 200: break
t = threading.Thread(target=sse_reader, daemon=True); t.start(); time.sleep(1)

session_id = None
for line in sse_lines:
    if "session_id=" in line:
        session_id = line.split("session_id=")[1].strip(); break
if not session_id:
    print("FATAL: no MCP session"); exit(1)

msg_id = [0]
def mcp_call(tool_name, arguments=None):
    msg_id[0] += 1
    payload = {"jsonrpc": "2.0", "id": msg_id[0], "method": "tools/call",
        "params": {"name": tool_name, "arguments": arguments or {}}}
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(f"{BASE}/mcp/message?session_id={session_id}",
        data=data, headers={"Content-Type": "application/json"}, method="POST")
    try: urllib.request.urlopen(req).read()
    except: pass
    time.sleep(0.5)
    for line in reversed(sse_lines):
        if not line.startswith("data:"): continue
        try:
            msg = json.loads(line[5:])
            if msg.get("id") == msg_id[0]:
                result = msg.get("result", {})
                content = result.get("content", [{}])[0]
                return {"ok": not result.get("isError", False), "text": content.get("text", "")}
        except: continue
    return None

# Check tool count
msg_id[0] += 1
p = {"jsonrpc":"2.0","id":msg_id[0],"method":"tools/list","params":{}}
d = json.dumps(p).encode("utf-8")
r = urllib.request.Request(f"{BASE}/mcp/message?session_id={session_id}",data=d,headers={"Content-Type":"application/json"},method="POST")
urllib.request.urlopen(r).read(); time.sleep(0.5)
for line in reversed(sse_lines):
    if "tools" in line:
        try:
            msg = json.loads(line.replace("data:","").strip())
            tools = msg.get("result",{}).get("tools",[])
            names = [t["name"] for t in tools]
            print(f"Total tools: {len(tools)}")
            for nt in ["list_source_texts","read_source_text","create_full_scenario"]:
                print(f"  {nt:25s} {'OK' if nt in names else 'MISSING'}")
        except: pass
        break

print("\n[1] list_source_texts")
resp = mcp_call("list_source_texts")
if resp and resp["ok"]:
    info = json.loads(resp["text"])
    files = info.get("files", [])
    print(f"  OK: {len(files)} txt files")
    for f in files:
        print(f"    {f['name']:45s} {f['size_kb']}KB")
else:
    print(f"  FAIL: {resp['text'][:100] if resp else 'no resp'}")

print("\n[2] read_source_text (chapter list)")
resp = mcp_call("read_source_text", {"path": "TREASURE ISLAND.txt"})
if resp and resp["ok"]:
    info = json.loads(resp["text"])
    print(f"  OK: {info.get('chapter_count')} chapters, {info.get('total_lines')} lines")
    for ch in info.get("chapters", [])[:5]:
        print(f"    Ch.{ch['number']:2d}: {ch['title'][:60]:60s} ({ch['line_count']} lines)")
    if info.get("chapter_count", 0) > 5:
        print(f"    ... ({info['chapter_count'] - 5} more)")
else:
    print(f"  FAIL: {resp['text'][:120] if resp else 'no resp'}")

print("\n[3] read_source_text (specific chapter)")
resp = mcp_call("read_source_text", {"path": "TREASURE ISLAND.txt", "chapter": 1})
if resp and resp["ok"]:
    info = json.loads(resp["text"])
    text = info.get("text", "")
    print(f"  OK: Ch.{info.get('chapter')}: {info.get('title')}")
    print(f"      {info.get('line_count')} lines, {len(text)} chars")
    print(f"      First 100 chars: {text[:100]}...")
else:
    print(f"  FAIL: {resp['text'][:120] if resp else 'no resp'}")

print("\n[4] create_full_scenario")
resp = mcp_call("create_full_scenario", {
    "save_path": "treasure_island/__test__/scenario.json",
    "scenario": {
        "scenario": {
            "name": "Ch.1 The Old Sea-dog",
            "description": "Test scenario from Treasure Island",
            "notes": ["API test"]
        },
        "npcs": {
            "jim": {
                "id": "jim", "name": "Jim Hawkins", "description": "The narrator, a brave young boy",
                "sincerity": 0.5, "fairness": 0.6, "greed_avoidance": 0.4, "modesty": 0.3,
                "fearfulness": 0.3, "anxiety": 0.4, "dependence": 0.2, "sentimentality": 0.5,
                "social_self_esteem": 0.3, "social_boldness": 0.4, "sociability": 0.3, "liveliness": 0.5
            },
            "billy": {
                "id": "billy", "name": "Billy Bones", "description": "A rough old sea captain",
                "sincerity": -0.5, "fairness": -0.3, "greed_avoidance": -0.6, "modesty": -0.7,
                "fearfulness": 0.6, "anxiety": 0.7, "dependence": -0.3, "sentimentality": -0.4,
                "social_self_esteem": 0.5, "social_boldness": 0.6, "sociability": -0.5, "liveliness": -0.3
            }
        },
        "relationships": {
            "jim:billy": {"owner_id": "jim", "target_id": "billy", "closeness": -0.2, "trust": -0.3, "power": -0.5},
            "billy:jim": {"owner_id": "billy", "target_id": "jim", "closeness": 0.1, "trust": 0.0, "power": 0.5}
        },
        "objects": {},
        "scene": {
            "npc_id": "jim", "partner_id": "billy",
            "description": "Billy Bones arrives at the Admiral Benbow inn",
            "focuses": [
                {"id": "arrival", "description": "Billy Bones arrives demanding rum",
                 "event": {"description": "A scary stranger arrives at the inn", "desirability_for_self": -0.4},
                 "action": {"description": "Billy threatens and intimidates", "agent_id": "billy", "praiseworthiness": -0.5}}
            ]
        }
    }
})
if resp and resp["ok"]:
    info = json.loads(resp["text"])
    print(f"  OK: path={info.get('path')}, npcs={info.get('npcs')}, rels={info.get('relationships')}")
else:
    print(f"  FAIL: {resp['text'][:150] if resp else 'no resp'}")

# Cleanup test file
import os
test_dir = os.path.join("data", "treasure_island", "__test__")
test_file = os.path.join(test_dir, "scenario.json")
if os.path.exists(test_file):
    os.remove(test_file)
    os.rmdir(test_dir)
    print("  Cleaned up test file")

print("\n" + "=" * 60)
print("Step 3 verification complete!")
print("=" * 60)
