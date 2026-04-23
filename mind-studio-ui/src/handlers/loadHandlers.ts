import { api } from '../api/client'
import type { AppraiseResult, ChatMessage, TurnHistory, Situation, SceneInfo, LlmModelInfo, LoadResponse, ToastFn, TraceEntry } from '../types'

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
    // Step E3.3: 응답 본문에서 seed 적용 결과(M1) 추출. 구버전 백엔드(혹은 non-embed
    // 빌드)는 빈 객체이거나 파싱 실패 — 그래도 로드 자체는 성공.
    let report: LoadResponse | null = null
    try {
      report = await res.json()
    } catch {
      // 응답 body가 JSON이 아닐 수 있음 (기존 형태) — 기본 success 토스트만.
    }
    const applied =
      (report?.applied_rumors ?? 0) + (report?.applied_memories ?? 0)
    if (applied > 0) {
      toast(
        `시나리오 로드 완료 — Rumor ${report?.applied_rumors ?? 0}건, Memory ${report?.applied_memories ?? 0}건 시딩`,
        'success',
      )
    } else {
      toast('시나리오 로드 완료', 'success')
    }
    if (report?.warnings && report.warnings.length > 0) {
      // E3.3 follow-up M2/L1: 시나리오가 크게 망가지면 warnings가 수십 개 쏟아져
      // 토스트로 화면을 덮어버릴 수 있다. 3건 이하면 그대로 띄우고, 그 이상이면
      // 첫 건 + 총 건수만 토스트 + 나머지는 console.warn으로 내린다.
      // `String(w)` 방어로 서버가 비문자열을 보내도 크래시하지 않음.
      const toMsg = (w: unknown) => {
        const s = String(w)
        return s.length > 200 ? s.slice(0, 200) + '…' : s
      }
      if (report.warnings.length <= 3) {
        for (const w of report.warnings) {
          toast(`시드 경고: ${toMsg(w)}`, 'error')
        }
      } else {
        toast(
          `시드 경고 ${report.warnings.length}건 — ${toMsg(report.warnings[0])} 외 (콘솔 참조)`,
          'error',
        )
        for (const w of report.warnings) {
          console.warn('[scenario seed]', String(w))
        }
      }
    }
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
