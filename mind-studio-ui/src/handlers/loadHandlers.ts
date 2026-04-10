import { api } from '../api/client'
import type { AppraiseResult, ChatMessage, TurnHistory, Situation, SceneInfo, LlmModelInfo, ToastFn, TraceEntry } from '../types'

export async function loadScenario(
  scenPath: string,
  toast: ToastFn,
  refresh: () => Promise<void>,
  setChatEnded: (v: boolean) => void,
  setResultView: (opts: { mode: boolean; active: boolean; turnHistory: TurnHistory[]; messages: ChatMessage[]; selectedIdx: number | null }) => void,
  setSavedSituation: (s: Situation | null) => void,
  setResult: (r: AppraiseResult | null) => void,
  setTraceHistory: (t: TraceEntry[]) => void,
) {
  if (!scenPath) return
  const path = `data/${scenPath}`
  const res = await fetch('/api/load', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path }),
  })
  if (res.ok) {
    toast('시나리오 로드 완료', 'success')
    setChatEnded(false)
    setResultView({ mode: false, active: false, turnHistory: [], messages: [], selectedIdx: null })
    await refresh()
    try {
      const sit = await api.get<Situation>('/api/situation')
      try {
        const si = await api.get<SceneInfo>('/api/scene-info')
        if (si?.has_scene && si.significance != null) {
          sit.significance = si.significance
        }
      } catch (_) { /* ignore */ }
      if (sit && typeof sit === 'object' && Object.keys(sit).length > 0) {
        setSavedSituation(sit)
      }
    } catch (_) { /* ignore */ }
    setResult(null)
    setTraceHistory([])
  } else {
    toast('로드 실패: ' + (await res.text()), 'error')
  }
}

export async function loadResult(
  scenPath: string,
  toast: ToastFn,
  refresh: () => Promise<void>,
  setChatEnded: (v: boolean) => void,
  setResultView: (opts: { mode: boolean; active: boolean; turnHistory: TurnHistory[]; messages: ChatMessage[]; selectedIdx: number | null }) => void,
  setSavedSituation: (s: Situation | null) => void,
  setResult: (r: AppraiseResult | null) => void,
  setTraceHistory: (t: TraceEntry[]) => void,
  setLlmModelInfo: (info: LlmModelInfo | null) => void,
  setResultTab: (tab: string) => void,
) {
  if (!scenPath) return
  const path = `data/${scenPath}`
  const res = await fetch('/api/load-result', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path }),
  })
  if (res.ok) {
    const data = await res.json()
    setChatEnded(false)
    toast('테스트 결과 로드 완료', 'success')
    await refresh()
    // Restore situation
    try {
      const sit = await api.get<Situation>('/api/situation')
      try {
        const si = await api.get<SceneInfo>('/api/scene-info')
        if (si?.has_scene && si.significance != null) sit.significance = si.significance
      } catch (_) { /* ignore */ }
      if (sit && typeof sit === 'object' && Object.keys(sit).length > 0) setSavedSituation(sit)
    } catch (_) { /* ignore */ }
    // Convert turn_history → messages
    const msgs: ChatMessage[] = []
    const history: TurnHistory[] = data.turn_history || []
    if (history.length > 0 && history[0].llm_model) {
      setLlmModelInfo(history[0].llm_model)
    } else {
      setLlmModelInfo(null)
    }
    history.forEach((turn) => {
      const resp = (turn.response || {}) as Record<string, unknown>
      const req = (turn.request || {}) as Record<string, unknown>
      if (turn.action === 'stimulus') {
        const utterance = (req.situation_description || req.utterance || '') as string
        const emotions: Record<string, number> = {}
        ;((resp.emotions || []) as { emotion_type: string; intensity: number }[]).forEach((e) => { emotions[e.emotion_type] = e.intensity })
        if (utterance) {
          msgs.push({
            role: 'user', content: utterance,
            pad: req.pleasure != null ? { pleasure: req.pleasure as number, arousal: req.arousal as number, dominance: req.dominance as number } : null,
          })
        }
        msgs.push({
          role: 'assistant', content: turn.label, emotions, mood: resp.mood as number,
          snapshot: { ...resp, llm_model: turn.llm_model } as unknown as AppraiseResult,
          beat_changed: resp.beat_changed as boolean,
          new_focus: resp.active_focus_id as string || null,
          trace: (resp.trace || []) as string[],
        })
      } else if (turn.action === 'scene' || turn.action === 'appraise' || turn.action === 'chat_start') {
        const emotions: Record<string, number> = {}
        ;((resp.emotions || []) as { emotion_type: string; intensity: number }[]).forEach((e) => { emotions[e.emotion_type] = e.intensity })
        msgs.push({
          role: 'system', content: turn.label, emotions, mood: resp.mood as number,
          snapshot: { ...resp, llm_model: turn.llm_model } as unknown as AppraiseResult,
          trace: (resp.trace || []) as string[],
        })
      } else if (turn.action === 'chat_message') {
        const emotions: Record<string, number> = {}
        ;((resp.emotions || []) as { emotion_type: string; intensity: number }[]).forEach((e) => { emotions[e.emotion_type] = e.intensity })
        msgs.push({
          role: 'user', content: (req.utterance || '') as string,
          pad: (req.pad || resp.input_pad || null) as ChatMessage['pad'],
          trace: [],
        })
        msgs.push({
          role: 'assistant', content: (resp.npc_response || turn.label) as string,
          emotions, mood: resp.mood as number,
          snapshot: { ...resp, llm_model: turn.llm_model } as unknown as AppraiseResult,
          beat_changed: resp.beat_changed as boolean,
          new_focus: resp.active_focus_id as string || null,
          trace: (resp.trace || []) as string[],
        })
      } else if (turn.action === 'after_dialogue') {
        msgs.push({
          role: 'system', content: `대화 종료 — ${turn.label}`,
          snapshot: resp as unknown as AppraiseResult, trace: [],
        })
      }
    })
    setResultView({ mode: true, active: false, turnHistory: data.turn_history || [], messages: msgs, selectedIdx: null })
    setResult(msgs.length > 0 && msgs[0].snapshot ? msgs[0].snapshot : null)
    setResultTab('emotions')
    setTraceHistory([])
  } else {
    toast('결과 로드 실패: ' + (await res.text()), 'error')
  }
}

export async function updateTestReport(content: string, setTestReport: (c: string) => void) {
  await fetch('/api/test-report', {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ content }),
  })
  setTestReport(content)
}
