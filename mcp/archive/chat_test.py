import requests, json, sys
sys.stdout.reconfigure(encoding='utf-8')

r = requests.post(
    'http://127.0.0.1:3000/api/chat/message',
    json={
        'session_id': 'test-001',
        'npc_id': 'jim',
        'partner_id': 'huck',
        'utterance': 'Jim! Is that you? What are you doing here?',
        'pad': {'pleasure': 0.3, 'arousal': 0.5, 'dominance': 0.2}
    },
    timeout=120
)
print('status:', r.status_code)
d = r.json()
print('npc_response:', d.get('npc_response', '')[:800])
print('beat_changed:', d.get('beat_changed'))
if d.get('stimulus'):
    print('dominant:', d['stimulus'].get('dominant'))
    print('mood:', d['stimulus'].get('mood'))
