import json, urllib.request, threading, time
BASE = "http://127.0.0.1:3000"
sse = []
def reader():
    resp = urllib.request.urlopen(urllib.request.Request(BASE+"/mcp/sse"), timeout=10)
    for l in resp:
        sse.append(l.decode("utf-8").strip())
        if len(sse) > 50: break
t = threading.Thread(target=reader, daemon=True); t.start(); time.sleep(1)
sid = next((l.split("session_id=")[1].strip() for l in sse if "session_id=" in l), None)
if not sid: print("NO SESSION"); exit()
payload = json.dumps({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}).encode()
urllib.request.urlopen(urllib.request.Request(
    BASE+"/mcp/message?session_id="+sid, data=payload,
    headers={"Content-Type":"application/json"}, method="POST")).read()
time.sleep(0.5)
for l in reversed(sse):
    if l.startswith("data:") and "tools" in l:
        msg = json.loads(l[5:])
        tools = msg.get("result",{}).get("tools",[])
        names = [t["name"] for t in tools]
        print(f"Server reports: {len(names)} tools")
        for n in names:
            chat_mark = " <<<" if "chat" in n else ""
            print(f"  {n}{chat_mark}")
        break
