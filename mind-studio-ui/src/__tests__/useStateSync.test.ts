import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'

// --- Mock EventSource ---
type ESListener = (e: MessageEvent) => void

class MockEventSource {
  static instances: MockEventSource[] = []
  url: string
  readyState = 0
  listeners = new Map<string, ESListener[]>()
  onerror: (() => void) | null = null
  closed = false

  constructor(url: string) {
    this.url = url
    MockEventSource.instances.push(this)
    // 비동기로 연결 완료 시뮬레이션
    queueMicrotask(() => {
      this.readyState = 1
    })
  }

  addEventListener(type: string, fn: ESListener) {
    const list = this.listeners.get(type) || []
    list.push(fn)
    this.listeners.set(type, list)
  }

  close() {
    this.closed = true
    this.readyState = 2
  }

  // 테스트 헬퍼: 이벤트 발생
  _emit(type: string, data = 'ok') {
    const fns = this.listeners.get(type) || []
    const event = new MessageEvent(type, { data })
    fns.forEach((fn) => fn(event))
  }
}

// --- Mock fetch (api.get 내부에서 사용) ---
let fetchMock: ReturnType<typeof vi.fn>

beforeEach(() => {
  MockEventSource.instances = []
  vi.stubGlobal('EventSource', MockEventSource)
  fetchMock = vi.fn().mockResolvedValue(
    new Response(JSON.stringify([]), { status: 200 }),
  )
  vi.stubGlobal('fetch', fetchMock)
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

// 동적 import로 모듈 캐시 방지
async function importHook() {
  const mod = await import('../hooks/useStateSync')
  return mod.useStateSync
}

describe('useStateSync', () => {
  it('마운트 시 EventSource 연결', async () => {
    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    renderHook(() => useStateSync(refresh))

    expect(MockEventSource.instances).toHaveLength(1)
    expect(MockEventSource.instances[0].url).toBe('/api/events')
  })

  it('npc_changed 이벤트 → /api/npcs fetch', async () => {
    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    renderHook(() => useStateSync(refresh))

    const es = MockEventSource.instances[0]
    act(() => { es._emit('npc_changed') })

    expect(fetchMock).toHaveBeenCalledWith('/api/npcs')
  })

  it('relationship_changed 이벤트 → /api/relationships fetch', async () => {
    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    renderHook(() => useStateSync(refresh))

    const es = MockEventSource.instances[0]
    act(() => { es._emit('relationship_changed') })

    expect(fetchMock).toHaveBeenCalledWith('/api/relationships')
  })

  it('object_changed 이벤트 → /api/objects fetch', async () => {
    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    renderHook(() => useStateSync(refresh))

    const es = MockEventSource.instances[0]
    act(() => { es._emit('object_changed') })

    expect(fetchMock).toHaveBeenCalledWith('/api/objects')
  })

  it('scenario_loaded 이벤트 → 전체 refresh 호출', async () => {
    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    renderHook(() => useStateSync(refresh))

    const es = MockEventSource.instances[0]
    act(() => { es._emit('scenario_loaded') })

    expect(refresh).toHaveBeenCalled()
  })

  it('resync 이벤트 → 전체 refresh 호출', async () => {
    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    renderHook(() => useStateSync(refresh))

    const es = MockEventSource.instances[0]
    act(() => { es._emit('resync') })

    expect(refresh).toHaveBeenCalled()
  })

  it('디바운스: 동일 이벤트 100ms 내 중복 fetch 방지', async () => {
    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    renderHook(() => useStateSync(refresh))

    const es = MockEventSource.instances[0]
    act(() => {
      es._emit('npc_changed')
      es._emit('npc_changed')
      es._emit('npc_changed')
    })

    // fetch는 1번만 호출
    const npcCalls = fetchMock.mock.calls.filter(
      (c: string[]) => c[0] === '/api/npcs',
    )
    expect(npcCalls).toHaveLength(1)

    // 100ms 후 디바운스 해제 → 다시 호출 가능
    act(() => { vi.advanceTimersByTime(100) })
    act(() => { es._emit('npc_changed') })

    const npcCalls2 = fetchMock.mock.calls.filter(
      (c: string[]) => c[0] === '/api/npcs',
    )
    expect(npcCalls2).toHaveLength(2)
  })

  it('에러 시 exponential backoff 재연결', async () => {
    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    renderHook(() => useStateSync(refresh))

    expect(MockEventSource.instances).toHaveLength(1)
    const es = MockEventSource.instances[0]

    // 에러 발생 → 1초 후 재연결
    act(() => { es.onerror?.() })
    expect(es.closed).toBe(true)

    act(() => { vi.advanceTimersByTime(1000) })
    expect(MockEventSource.instances).toHaveLength(2)

    // 두 번째 에러 → 2초 후 재연결
    const es2 = MockEventSource.instances[1]
    act(() => { es2.onerror?.() })
    act(() => { vi.advanceTimersByTime(1000) })
    expect(MockEventSource.instances).toHaveLength(2) // 아직 안 됨
    act(() => { vi.advanceTimersByTime(1000) })
    expect(MockEventSource.instances).toHaveLength(3) // 2초 후 재연결
  })

  it('언마운트 시 EventSource 해제', async () => {
    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    const { unmount } = renderHook(() => useStateSync(refresh))

    const es = MockEventSource.instances[0]
    expect(es.closed).toBe(false)

    unmount()
    expect(es.closed).toBe(true)
  })

  it('appraised 이벤트 → history + scene-info fetch', async () => {
    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    renderHook(() => useStateSync(refresh))

    const es = MockEventSource.instances[0]
    act(() => { es._emit('appraised') })

    const urls = fetchMock.mock.calls.map((c: string[]) => c[0])
    expect(urls).toContain('/api/history')
    expect(urls).toContain('/api/scene-info')
  })

  it('scenario_saved 이벤트 → /api/scenarios fetch', async () => {
    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    renderHook(() => useStateSync(refresh))

    const es = MockEventSource.instances[0]
    act(() => { es._emit('scenario_saved') })

    expect(fetchMock).toHaveBeenCalledWith('/api/scenarios')
  })

  // ---------------------------------------------------------------------------
  // Step E2 — Memory / Rumor SSE
  // ---------------------------------------------------------------------------

  it('rumor_seeded 이벤트 → /api/rumors fetch', async () => {
    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    renderHook(() => useStateSync(refresh))

    const es = MockEventSource.instances[0]
    act(() => { es._emit('rumor_seeded') })

    const urls = fetchMock.mock.calls.map((c: string[]) => c[0])
    expect(urls).toContain('/api/rumors')
  })

  it('rumor_spread 이벤트 → /api/rumors + selected NPC 기억 fetch', async () => {
    const { useMemoryStore } = await import('../stores/useMemoryStore')
    useMemoryStore.setState({ selectedNpcId: 'mu_baek' })

    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    renderHook(() => useStateSync(refresh))

    const es = MockEventSource.instances[0]
    act(() => { es._emit('rumor_spread') })

    const urls = fetchMock.mock.calls.map((c: string[]) => c[0])
    expect(urls).toContain('/api/rumors')
    expect(urls).toContain('/api/memory/by-npc/mu_baek')
  })

  it('memory_created 이벤트 → selected NPC의 기억 fetch', async () => {
    const { useMemoryStore } = await import('../stores/useMemoryStore')
    useMemoryStore.setState({ selectedNpcId: 'gyo_ryong' })

    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    renderHook(() => useStateSync(refresh))

    const es = MockEventSource.instances[0]
    act(() => { es._emit('memory_created') })

    const urls = fetchMock.mock.calls.map((c: string[]) => c[0])
    expect(urls).toContain('/api/memory/by-npc/gyo_ryong')
  })

  it('selected NPC가 없으면 memory_* 이벤트는 기억 fetch를 skip', async () => {
    const { useMemoryStore } = await import('../stores/useMemoryStore')
    useMemoryStore.setState({ selectedNpcId: null })

    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    renderHook(() => useStateSync(refresh))

    const es = MockEventSource.instances[0]
    act(() => { es._emit('memory_created'); es._emit('memory_superseded'); es._emit('memory_consolidated') })

    const urls = fetchMock.mock.calls.map((c: string[]) => c[0])
    expect(urls.filter((u) => u.startsWith('/api/memory/by-npc/'))).toHaveLength(0)
  })

  it('memory_superseded / memory_consolidated 이벤트 → selected NPC 기억 fetch', async () => {
    const { useMemoryStore } = await import('../stores/useMemoryStore')
    useMemoryStore.setState({ selectedNpcId: 'mu_baek' })

    const useStateSync = await importHook()
    const refresh = vi.fn().mockResolvedValue(undefined)
    renderHook(() => useStateSync(refresh))

    const es = MockEventSource.instances[0]
    // 동일 이벤트 연속 호출 시 debounce 우회를 위해 서로 다른 이벤트로.
    act(() => { es._emit('memory_superseded') })

    let urls = fetchMock.mock.calls.map((c: string[]) => c[0])
    expect(urls).toContain('/api/memory/by-npc/mu_baek')

    fetchMock.mockClear()
    // debounce 해제 대기.
    act(() => { vi.advanceTimersByTime(150) })

    act(() => { es._emit('memory_consolidated') })
    urls = fetchMock.mock.calls.map((c: string[]) => c[0])
    expect(urls).toContain('/api/memory/by-npc/mu_baek')
  })
})
