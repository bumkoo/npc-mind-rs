import requests, json, sys
sys.stdout.reconfigure(encoding='utf-8')

# Direct test against llama.cpp server
r = requests.post(
    'http://127.0.0.1:8081/v1/chat/completions',
    json={
        "model": "local-model",
        "messages": [
            {"role": "system", "content": "You are Jim."},
            {"role": "user", "content": "Hello"}
        ],
        "max_tokens": 50
    },
    timeout=120
)
print('direct llama.cpp status:', r.status_code)
print('body:', r.text[:500])
