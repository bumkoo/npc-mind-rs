import { describe, it, expect, vi, beforeEach } from 'vitest'
import {
  handleAppraise, handleStimulus, handleGuide, handleAfterDialogue,
  doSave, saveState,
} from '../handlers/appHandlers'
import { updateTestReport } from '../handlers/loadHandlers'

beforeEach(() => {
  vi.restoreAllMocks()
})

// --- handleAppraise ---
describe('handleAppraise', () => {
  it('npcId/partnerId 없으면 toast 에러', async () => {
    const toast = vi.fn()
    const setLoading = vi.fn()
    const setResult = vi.fn()
    const setTrace = vi.fn()
    const refresh = vi.fn()

    await handleAppraise('', 'player', {}, toast, setLoading, setResult, setTrace, refresh)
    expect(toast).toHaveBeenCalledWith('NPC와 대화 상대를 선택하세요', 'error')
    expect(setLoading).not.toHaveBeenCalled()
  })

  it('성공 시 result/trace 설정 + refresh 호출', async () => {
    const mockResult = { emotions: [{ emotion_type: 'Joy', intensity: 0.8 }], trace: ['step1'] }
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify(mockResult), { status: 200 }),
    )
    const toast = vi.fn()
    const setLoading = vi.fn()
    const setResult = vi.fn()
    const setTrace = vi.fn()
    const refresh = vi.fn()

    await handleAppraise('npc1', 'player', { description: '상황' }, toast, setLoading, setResult, setTrace, refresh)

    expect(setLoading).toHaveBeenCalledWith(true)
    expect(setResult).toHaveBeenCalledWith(mockResult)
    expect(setTrace).toHaveBeenCalledWith([{ label: 'appraise', trace: ['step1'] }])
    expect(refresh).toHaveBeenCalled()
    expect(setLoading).toHaveBeenCalledWith(false) // finally
  })

  it('API 에러 시 toast 표시 + loading 해제', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('Internal Error', { status: 500, statusText: 'Internal Error' }),
    )
    const toast = vi.fn()
    const setLoading = vi.fn()
    const setResult = vi.fn()
    const setTrace = vi.fn()
    const refresh = vi.fn()

    await handleAppraise('npc1', 'player', {}, toast, setLoading, setResult, setTrace, refresh)

    expect(toast).toHaveBeenCalledWith(expect.stringContaining('오류'), 'error')
    expect(setResult).not.toHaveBeenCalled()
    expect(setLoading).toHaveBeenCalledWith(false) // finally
  })

  it('네트워크 에러 시 toast + loading 해제', async () => {
    vi.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('fetch failed'))
    const toast = vi.fn()
    const setLoading = vi.fn()

    await handleAppraise('npc1', 'player', {}, toast, setLoading, vi.fn(), vi.fn(), vi.fn())

    expect(toast).toHaveBeenCalledWith(expect.stringContaining('요청 실패'), 'error')
    expect(setLoading).toHaveBeenCalledWith(false)
  })
})

// --- handleStimulus ---
describe('handleStimulus', () => {
  it('result 없으면 에러 toast', async () => {
    const toast = vi.fn()
    await handleStimulus(
      'npc1', 'player', { pleasure: 0.5, arousal: 0.3, dominance: 0.1, situation_description: null },
      toast, vi.fn(), vi.fn(), vi.fn(), vi.fn(), false,
    )
    expect(toast).toHaveBeenCalledWith('감정 평가를 먼저 실행하세요', 'error')
  })

  it('성공 시 result 설정 + trace 추가', async () => {
    const mockData = { emotions: [], trace: ['s1'], mood: 0.3 }
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify(mockData), { status: 200 }),
    )
    const setResult = vi.fn()
    const appendTrace = vi.fn()
    const refresh = vi.fn()

    await handleStimulus(
      'npc1', 'player', { pleasure: 0.5, arousal: 0.3, dominance: 0.1, situation_description: null },
      vi.fn(), vi.fn(), setResult, appendTrace, refresh, true,
    )

    expect(setResult).toHaveBeenCalledWith(mockData)
    expect(appendTrace).toHaveBeenCalledWith({ label: 'stimulus', trace: ['s1'] })
    expect(refresh).toHaveBeenCalled()
  })
})

// --- handleGuide ---
describe('handleGuide', () => {
  it('npcId 없으면 아무 동작 안 함', async () => {
    const toast = vi.fn()
    const updateResult = vi.fn()
    await handleGuide('', 'player', toast, updateResult)
    expect(updateResult).not.toHaveBeenCalled()
  })

  it('성공 시 prompt 업데이트', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify({ prompt: '새 가이드' }), { status: 200 }),
    )
    const updateResult = vi.fn()

    await handleGuide('npc1', 'player', vi.fn(), updateResult)

    expect(updateResult).toHaveBeenCalled()
    // updater 함수 검증
    const updater = updateResult.mock.calls[0][0]
    const result = updater({ emotions: [], prompt: '이전', afterDialogue: false })
    expect(result.prompt).toBe('새 가이드')
  })

  it('afterDialogue 상태에서는 prompt 안 바꿈', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify({ prompt: '새 가이드' }), { status: 200 }),
    )
    const updateResult = vi.fn()
    await handleGuide('npc1', 'player', vi.fn(), updateResult)

    const updater = updateResult.mock.calls[0][0]
    const prev = { emotions: [], prompt: '이전', afterDialogue: true }
    expect(updater(prev)).toBe(prev) // 변경 없이 원본 반환
  })
})

// --- handleAfterDialogue ---
describe('handleAfterDialogue', () => {
  it('성공 시 afterDialogue 플래그 포함한 결과 설정', async () => {
    const respData = { relationship: { closeness: 0.5, trust: 0.6 } }
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify(respData), { status: 200 }),
    )
    const updateResult = vi.fn()

    await handleAfterDialogue('npc1', 'player', 0.5, vi.fn(), updateResult, vi.fn())

    const updater = updateResult.mock.calls[0][0]
    const result = updater(null)
    expect(result.afterDialogue).toBe(true)
    expect(result.npc_id).toBe('npc1')
    expect(result.relationship.closeness).toBe(0.5)
  })
})

// --- doSave ---
describe('doSave', () => {
  it('성공 시 저장 경로 반환', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify({ path: 'data/test.json' }), { status: 200 }),
    )
    const path = await doSave('data/test.json', 'scenario')
    expect(path).toBe('data/test.json')
  })

  it('실패 시 에러 throw', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('파일을 찾을 수 없음', { status: 404 }),
    )
    await expect(doSave('bad/path.json')).rejects.toThrow('파일을 찾을 수 없음')
  })
})

// --- saveState ---
describe('saveState', () => {
  it('대화 중이면 에러 toast', async () => {
    const toast = vi.fn()
    await saveState(true, false, toast)
    expect(toast).toHaveBeenCalledWith('대화 종료 후 저장 가능합니다', 'error')
  })

  it('시나리오 미로드 시 에러 toast', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('', { status: 404 }),
    )
    const toast = vi.fn()
    await saveState(false, false, toast)
    expect(toast).toHaveBeenCalledWith('시나리오를 먼저 로드하세요', 'error')
  })
})

// --- updateTestReport ---
describe('updateTestReport', () => {
  it('PUT 요청 후 setTestReport 호출', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('', { status: 200 }))
    const setTestReport = vi.fn()

    await updateTestReport('# 새 보고서', setTestReport)

    expect(fetch).toHaveBeenCalledWith('/api/test-report', expect.objectContaining({
      method: 'PUT',
      body: JSON.stringify({ content: '# 새 보고서' }),
    }))
    expect(setTestReport).toHaveBeenCalledWith('# 새 보고서')
  })
})
