import { api } from '../api/client'
import type { AppraiseResult, ChatMessage, SceneInfo, SaveDirInfo, ToastFn, TraceEntry, ScenarioTurn, LlmModelInfo } from '../types'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Situation = any

// --- Pipeline handlers ---
export async function handleAppraise(
  npcId: string, partnerId: string, situation: Situation,
  toast: ToastFn,
  setLoading: (v: boolean) => void,
  setResult: (r: AppraiseResult | null) => void,
  setTraceHistory: (t: TraceEntry[]) => void,
  refresh: () => Promise<void>,
) {
  if (!npcId || !partnerId) return toast('NPC와 대화 상대를 선택하세요', 'error')
  setLoading(true)
  try {
    const res = await fetch('/api/appraise', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ npc_id: npcId, partner_id: partnerId, situation }),
    })
    if (!res.ok) { toast('오류: ' + (await res.text()), 'error'); return }
    const data = await res.json()
    setResult(data)
    setTraceHistory([{ label: 'appraise', trace: data.trace || [] }])
    refresh()
  } catch (e) { toast('요청 실패: ' + e, 'error') }
  finally { setLoading(false) }
}

export async function handleStimulus(
  npcId: string, partnerId: string,
  pad: { pleasure: number; arousal: number; dominance: number; situation_description: string | null },
  toast: ToastFn,
  setLoading: (v: boolean) => void,
  setResult: (r: AppraiseResult | null) => void,
  appendTrace: (entry: TraceEntry) => void,
  refresh: () => Promise<void>,
  hasResult: boolean,
) {
  if (!npcId || !partnerId) return toast('NPC와 대화 상대를 선택하세요', 'error')
  if (!hasResult) return toast('감정 평가를 먼저 실행하세요', 'error')
  setLoading(true)
  try {
    const res = await fetch('/api/stimulus', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ npc_id: npcId, partner_id: partnerId, ...pad }),
    })
    if (!res.ok) { toast('오류: ' + (await res.text()), 'error'); return }
    const data = await res.json()
    setResult(data)
    appendTrace({ label: 'stimulus', trace: data.trace || [] })
    refresh()
  } catch (e) { toast('요청 실패: ' + e, 'error') }
  finally { setLoading(false) }
}

export async function handleGuide(
  npcId: string, partnerId: string,
  toast: ToastFn,
  updateResult: (updater: (prev: AppraiseResult | null) => AppraiseResult | null) => void,
) {
  if (!npcId || !partnerId) return
  try {
    const data = await api.postJson<{ prompt: string }>('/api/guide', {
      npc_id: npcId, partner_id: partnerId, situation_description: null,
    })
    updateResult((prev) => prev && !prev.afterDialogue ? { ...prev, prompt: data.prompt } : prev)
  } catch (e) { toast('가이드 재생성 실패: ' + e, 'error') }
}

export async function handleAfterDialogue(
  npcId: string, partnerId: string, sig: number | null,
  toast: ToastFn,
  updateResult: (updater: (prev: AppraiseResult | null) => AppraiseResult | null) => void,
  refresh: () => Promise<void>,
) {
  if (!npcId || !partnerId) return toast('NPC와 대화 상대를 선택하세요', 'error')
  try {
    const res = await fetch('/api/after-dialogue', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ npc_id: npcId, partner_id: partnerId, significance: sig }),
    })
    if (!res.ok) { toast('오류: ' + (await res.text()), 'error'); return }
    const data = await res.json()
    updateResult(() => ({ afterDialogue: true, npc_id: npcId, partner_id: partnerId, ...data }))
    refresh()
  } catch (e) { toast('요청 실패: ' + e, 'error') }
}

// --- Chat handlers ---
export async function handleStartChat(
  npcId: string, partnerId: string, situation: Situation,
  sceneInfo: SceneInfo | null,
  toast: ToastFn,
  refresh: () => Promise<void>,
  chatEnded: boolean,
  setChatLoading: (v: boolean) => void,
  setChatSessionId: (id: string | null) => void,
  setChatMode: (v: boolean) => void,
  setResultTab: (tab: string) => void,
  setResult: (r: AppraiseResult | null) => void,
  setLlmModelInfo: (info: LlmModelInfo | null) => void,
  setTraceHistory: (t: TraceEntry[]) => void,
  setChatMessages: (msgs: ChatMessage[]) => void,
  setChatScenarioTurns: (turns: ScenarioTurn[]) => void,
  setChatScenarioIdx: (idx: number) => void,
  setSelectedMsgIdx: (idx: number | null) => void,
  saveScenarioFn: () => Promise<boolean>,
) {
  if (!npcId || !partnerId) return toast('NPC와 대화 상대를 선택하세요', 'error')
  if (chatEnded) { toast('테스트 결과를 저장하거나 시나리오를 다시 로드하세요', 'error'); return }
  try {
    const dirRes = await fetch('/api/save-dir')
    if (dirRes.ok) {
      const info: SaveDirInfo = await dirRes.json()
      if (info.scenario_modified) {
        const ok = await saveScenarioFn()
        if (!ok) return
        await refresh()
      }
    }
  } catch (_) { /* ignore */ }
  const sessionId = `chat-${Date.now()}`
  setChatLoading(true)
  try {
    const res = await fetch('/api/chat/start', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ session_id: sessionId, appraise: { npc_id: npcId, partner_id: partnerId, situation } }),
    })
    if (!res.ok) { toast('대화 시작 실패: ' + (await res.text()), 'error'); return }
    const data = await res.json()
    setChatSessionId(sessionId)
    setChatMode(true)
    setResultTab('emotions')
    setResult(data.appraise)
    setLlmModelInfo(data.llm_model_info || null)
    setTraceHistory([{ label: 'chat/start', trace: data.appraise.trace || [] }])
    const initEmotions: Record<string, number> = {}
    ;(data.appraise.emotions || []).forEach((e: { emotion_type: string; intensity: number }) => {
      initEmotions[e.emotion_type] = e.intensity
    })
    setChatMessages([{
      role: 'system',
      content: `대화 시작 — ${data.appraise.dominant?.emotion_type || 'neutral'} (${(data.appraise.mood || 0).toFixed(3)})`,
      emotions: initEmotions,
      mood: data.appraise.mood,
      snapshot: { ...data.appraise, llm_model: data.llm_model_info },
      activePrompt: data.appraise.prompt || '',
    }])
    setSelectedMsgIdx(null)
    if (sceneInfo?.turns) {
      setChatScenarioTurns(sceneInfo.turns)
      setChatScenarioIdx(0)
    } else {
      setChatScenarioTurns([])
      setChatScenarioIdx(0)
    }
    refresh()
    toast('대화 시작', 'success')
  } catch (e) { toast('요청 실패: ' + e, 'error') }
  finally { setChatLoading(false) }
}

export async function handleChatSend(
  utterance: string, pad: { pleasure: number; arousal: number; dominance: number } | null | undefined,
  chatSessionId: string, npcId: string, partnerId: string,
  sceneInfo: SceneInfo | null, currentPrompt: string,
  toast: ToastFn,
  setChatLoading: (v: boolean) => void,
  updateChatMessages: (fn: (prev: ChatMessage[]) => ChatMessage[]) => void,
  setResult: (r: AppraiseResult | null) => void,
  setStimulusUtterance: (u: string) => void,
  appendTrace: (entry: TraceEntry) => void,
  updateSceneInfo: (fn: (prev: SceneInfo | null) => SceneInfo | null) => void,
  refresh: () => Promise<void>,
) {
  setChatLoading(true)
  // Optimistic script cursor advance
  if (sceneInfo) {
    const af = sceneInfo.focuses?.find((f) => f.is_active)
    const cur = sceneInfo.script_cursor || 0
    if (af?.test_script && cur < af.test_script.length && utterance === af.test_script[cur]) {
      updateSceneInfo((prev) => prev ? { ...prev, script_cursor: cur + 1 } : prev)
    }
  }
  // Add user message
  updateChatMessages((prev) => [...prev, { role: 'user', content: utterance, emotions: null, mood: null }])
  // Add assistant placeholder — capture index via closure (not mutable ref)
  let capturedIdx = 0
  updateChatMessages((prev) => {
    capturedIdx = prev.length
    return [...prev, {
      role: 'assistant', content: '', emotions: null, mood: null,
      beat_changed: false, new_focus: null, snapshot: null,
      activePrompt: currentPrompt, streaming: true,
    }]
  })

  const controller = new AbortController()
  try {
    const body: Record<string, unknown> = { session_id: chatSessionId, npc_id: npcId, partner_id: partnerId, utterance }
    if (pad) body.pad = pad
    const res = await fetch('/api/chat/message/stream', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
      signal: controller.signal,
    })
    if (!res.ok) {
      toast('대사 전송 실패: ' + (await res.text()), 'error')
      updateChatMessages((prev) => prev.filter((_, i) => i !== capturedIdx))
      return
    }
    const reader = res.body!.getReader()
    const decoder = new TextDecoder()
    let buffer = ''
    while (true) {
      const { done, value } = await reader.read()
      if (done) break
      buffer += decoder.decode(value, { stream: true })
      const lines = buffer.split('\n')
      buffer = lines.pop()!
      let eventType = ''
      for (const line of lines) {
        if (line.startsWith('event: ')) {
          eventType = line.slice(7).trim()
        } else if (line.startsWith('data: ')) {
          const data = line.slice(6)
          if (eventType === 'token') {
            updateChatMessages((prev) => {
              const updated = [...prev]
              if (updated[capturedIdx]) {
                updated[capturedIdx] = { ...updated[capturedIdx], content: updated[capturedIdx].content + data }
              }
              return updated
            })
          } else if (eventType === 'done') {
            const result = JSON.parse(data)
            const emotions: Record<string, number> = {}
            if (result.stimulus?.emotions) {
              result.stimulus.emotions.forEach((e: { emotion_type: string; intensity: number }) => {
                emotions[e.emotion_type] = e.intensity
              })
            }
            updateChatMessages((prev) => {
              const updated = [...prev]
              if (updated[capturedIdx]) {
                updated[capturedIdx] = {
                  ...updated[capturedIdx],
                  content: result.npc_response,
                  emotions: result.stimulus ? emotions : null,
                  mood: result.stimulus?.mood || null,
                  beat_changed: result.beat_changed,
                  new_focus: result.stimulus?.active_focus_id || null,
                  snapshot: result.stimulus || null,
                  activePrompt: result.stimulus?.prompt || currentPrompt,
                  streaming: false,
                }
              }
              if (result.stimulus?.input_pad) {
                for (let j = updated.length - 1; j >= 0; j--) {
                  if (updated[j]?.role === 'user' && !updated[j]?.pad) {
                    updated[j] = { ...updated[j], pad: result.stimulus.input_pad }
                    break
                  }
                }
              }
              return updated
            })
            if (result.stimulus) {
              setResult(result.stimulus)
              if (result.stimulus.input_pad) setStimulusUtterance(utterance)
              appendTrace({ label: 'chat/message', trace: result.stimulus.trace || [] })
            }
          } else if (eventType === 'error') {
            toast('LLM 오류: ' + data, 'error')
          }
          eventType = ''
        }
      }
    }
    // History sync
    try {
      const histRes = await fetch('/api/history')
      if (histRes.ok) {
        const history = await histRes.json()
        if (history.length > 0) {
          const lastTurn = history[history.length - 1]
          updateChatMessages((prev) => {
            const updated = [...prev]
            if (updated[capturedIdx]) {
              updated[capturedIdx] = {
                ...updated[capturedIdx],
                snapshot: { ...updated[capturedIdx].snapshot, llm_model: lastTurn.llm_model },
              }
            }
            return updated
          })
        }
      }
    } catch (_) { /* ignore */ }
    refresh()
  } catch (e) {
    if ((e as Error).name !== 'AbortError') {
      toast('요청 실패: ' + e, 'error')
    }
  } finally { setChatLoading(false) }
}

export async function handleEndChat(
  chatSessionId: string | null, npcId: string, partnerId: string,
  toast: ToastFn,
  setChatMode: (v: boolean) => void,
  setChatSessionId: (id: string | null) => void,
  setChatMessages: (msgs: ChatMessage[]) => void,
  setChatScenarioTurns: (turns: ScenarioTurn[]) => void,
  setChatScenarioIdx: (idx: number) => void,
  setSelectedMsgIdx: (idx: number | null) => void,
  setChatEnded: (v: boolean) => void,
  updateResult: (fn: (prev: AppraiseResult | null) => AppraiseResult | null) => void,
  setResultTab: (tab: string) => void,
  refresh: () => Promise<void>,
) {
  if (!chatSessionId) { setChatMode(false); return }
  try {
    let significance: number | null = null
    try {
      const siRes = await fetch('/api/scene-info')
      if (siRes.ok) {
        const si = await siRes.json()
        if (si.has_scene && si.significance != null) significance = si.significance
      }
    } catch (_) { /* ignore */ }
    const body: Record<string, unknown> = {
      session_id: chatSessionId,
      after_dialogue: npcId && partnerId ? { npc_id: npcId, partner_id: partnerId, significance } : undefined,
    }
    const res = await fetch('/api/chat/end', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })
    if (res.ok) {
      const data = await res.json()
      if (data.after_dialogue) {
        updateResult(() => ({ afterDialogue: true, npc_id: npcId, partner_id: partnerId, ...data.after_dialogue }))
        setResultTab('emotions')
        toast('대화 종료 — 관계 갱신 완료', 'success')
      } else {
        setResultTab('emotions')
        toast('대화 종료', 'success')
      }
    }
  } catch (e) { toast('대화 종료 실패: ' + e, 'error') }
  setChatMode(false)
  setChatSessionId(null)
  setChatMessages([])
  setChatScenarioTurns([])
  setChatScenarioIdx(0)
  setSelectedMsgIdx(null)
  setChatEnded(true)
  refresh()
}

// --- Save / Load helpers ---
export async function doSave(path: string, saveType?: string): Promise<string> {
  const res = await fetch('/api/save', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path, save_type: saveType || null }),
  })
  if (res.ok) return (await res.json()).path
  throw new Error(await res.text())
}

export async function saveScenario(toast: ToastFn): Promise<boolean> {
  let info: SaveDirInfo | null = null
  try {
    const dirRes = await fetch('/api/save-dir')
    if (dirRes.ok) info = await dirRes.json()
  } catch (_) { /* ignore */ }
  if (!info) { toast('시나리오를 먼저 로드하세요', 'error'); return false }
  const { loaded_path, scenario_name, scenario_modified, has_existing_results } = info
  if (scenario_modified && has_existing_results) {
    const displayDir = (loaded_path || '').replace(/^data\//, '').replace(/\/[^/]+$/, '/')
    const name = prompt(`[${scenario_name}] 시나리오 저장\n기존 테스트 결과가 있어 원본을 덮어쓸 수 없습니다.\n위치: ${displayDir}\n새 파일 이름:`, '')
    if (!name) return false
    const filename = name.endsWith('.json') ? name : name + '.json'
    const parentDir = (loaded_path || 'data').replace(/\/[^/]+$/, '')
    try { await doSave(`${parentDir}/${filename}`, 'scenario') }
    catch (e) { toast('시나리오 저장 실패: ' + (e as Error).message, 'error'); return false }
  } else {
    try { await doSave(loaded_path, 'scenario') }
    catch (e) { toast('시나리오 저장 실패: ' + (e as Error).message, 'error'); return false }
  }
  toast('시나리오 저장 완료', 'success')
  return true
}

export async function saveState(
  chatMode: boolean, chatEnded: boolean,
  toast: ToastFn,
) {
  if (chatMode) { toast('대화 종료 후 저장 가능합니다', 'error'); return }
  let info: SaveDirInfo | null = null
  try {
    const dirRes = await fetch('/api/save-dir')
    if (dirRes.ok) info = await dirRes.json()
  } catch (_) { /* ignore */ }
  if (!info) { toast('시나리오를 먼저 로드하세요', 'error'); return }
  const { dir, scenario_name, scenario_modified, has_turn_history, has_existing_results } = info
  if (chatEnded) {
    if (!has_turn_history) { toast('저장할 대화 기록이 없습니다', 'info'); return }
    const displayDir = dir.replace(/^data\//, '') + '/'
    const name = prompt(`[${scenario_name}] 테스트 결과 저장\n저장 위치: ${displayDir}\n파일 이름:`, '')
    if (!name) return
    const filename = name.endsWith('.json') ? name : name + '.json'
    try { await doSave(`${dir}/${filename}`, 'result'); toast('테스트 결과 저장 완료', 'success') }
    catch (e) { toast('결과 저장 실패: ' + (e as Error).message, 'error') }
    return
  }
  if (scenario_modified) {
    const ok = await saveScenario(toast)
    if (!ok) return
    toast('시나리오 저장 완료', 'success')
    return
  }
  toast('저장할 변경사항이 없습니다', 'info')
}
