import requests, json, sys
sys.stdout.reconfigure(encoding='utf-8')

# check error details
r = requests.post(
    'http://127.0.0.1:3000/api/chat/message',
    json={
        'session_id': 'test-001',
        'npc_id': 'jim',
        'partner_id': 'huck',
        'utterance': 'Jim! Is that you?',
        'pad': {'pleasure': 0.3, 'arousal': 0.5, 'dominance': 0.2}
    },
    timeout=120
)
print('status:', r.status_code)
print('body:', r.text[:1000])
