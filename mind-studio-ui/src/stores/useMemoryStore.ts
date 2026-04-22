import { create } from 'zustand'
import type { MemoryEntry, MemoryLayer, Rumor } from '../types'

/**
 * Memory / Rumor 뷰 상태 (Step E2).
 *
 * - `entriesByNpc`: 현재 화면에 표시 중인 NPC 기준 기억 목록. Layer 필터는 UI 쪽.
 *   SSE(memory_*) 수신 시 selected NPC에 한해 targeted refetch.
 * - `rumors`: 전체 소문 목록. SSE(rumor_*) 수신 시 갱신.
 * - `selectedNpcId`: Memory 탭에서 조회 중인 NPC. `null`이면 시나리오 전체 NPC 드롭다운에서 선택 대기.
 * - `layerFilter`: `'all' | 'A' | 'B'` — 클라이언트 사이드 필터.
 */
interface MemoryStore {
  entriesByNpc: MemoryEntry[]
  rumors: Rumor[]
  selectedNpcId: string | null
  layerFilter: 'all' | MemoryLayer
  loading: boolean

  setEntries: (entries: MemoryEntry[]) => void
  setRumors: (rumors: Rumor[]) => void
  setSelectedNpcId: (id: string | null) => void
  setLayerFilter: (layer: 'all' | MemoryLayer) => void
  setLoading: (loading: boolean) => void
  clear: () => void
}

export const useMemoryStore = create<MemoryStore>((set) => ({
  entriesByNpc: [],
  rumors: [],
  selectedNpcId: null,
  layerFilter: 'all',
  loading: false,

  setEntries: (entriesByNpc) => set({ entriesByNpc }),
  setRumors: (rumors) => set({ rumors }),
  setSelectedNpcId: (selectedNpcId) => set({ selectedNpcId }),
  setLayerFilter: (layerFilter) => set({ layerFilter }),
  setLoading: (loading) => set({ loading }),
  clear: () => set({ entriesByNpc: [], rumors: [], selectedNpcId: null, layerFilter: 'all' }),
}))
