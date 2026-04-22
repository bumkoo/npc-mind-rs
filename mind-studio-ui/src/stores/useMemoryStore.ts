import { create } from 'zustand'
import type { MemoryEntry, MemoryLayer, Rumor } from '../types'

/**
 * Memory / Rumor 뷰 상태 (Step E2 → E3.1 확장).
 *
 * - `mode`: `'npc' | 'topic'` — 기억 탭의 조회 모드.
 * - `entriesByNpc`: NPC 모드에서 사용 (Layer 필터 적용).
 * - `topicEntries`: Topic 모드 결과 — supersede 이력 포함해 created_seq DESC.
 * - `selectedTopic`: Topic 모드에서 사용자가 입력한 topic 키.
 * - `rumors`: 전체 소문 목록.
 * - `selectedNpcId`: NPC 모드에서 조회 중인 NPC.
 * - `layerFilter`: NPC 모드 클라이언트 필터.
 */
export type MemoryViewMode = 'npc' | 'topic'

interface MemoryStore {
  mode: MemoryViewMode
  entriesByNpc: MemoryEntry[]
  topicEntries: MemoryEntry[]
  rumors: Rumor[]
  selectedNpcId: string | null
  selectedTopic: string | null
  layerFilter: 'all' | MemoryLayer
  loading: boolean

  setMode: (mode: MemoryViewMode) => void
  setEntries: (entries: MemoryEntry[]) => void
  setTopicEntries: (entries: MemoryEntry[]) => void
  setRumors: (rumors: Rumor[]) => void
  setSelectedNpcId: (id: string | null) => void
  setSelectedTopic: (topic: string | null) => void
  setLayerFilter: (layer: 'all' | MemoryLayer) => void
  setLoading: (loading: boolean) => void
  clear: () => void
}

export const useMemoryStore = create<MemoryStore>((set) => ({
  mode: 'npc',
  entriesByNpc: [],
  topicEntries: [],
  rumors: [],
  selectedNpcId: null,
  selectedTopic: null,
  layerFilter: 'all',
  loading: false,

  setMode: (mode) => set({ mode }),
  setEntries: (entriesByNpc) => set({ entriesByNpc }),
  setTopicEntries: (topicEntries) => set({ topicEntries }),
  setRumors: (rumors) => set({ rumors }),
  setSelectedNpcId: (selectedNpcId) => set({ selectedNpcId }),
  setSelectedTopic: (selectedTopic) => set({ selectedTopic }),
  setLayerFilter: (layerFilter) => set({ layerFilter }),
  setLoading: (loading) => set({ loading }),
  clear: () => set({
    mode: 'npc',
    entriesByNpc: [],
    topicEntries: [],
    rumors: [],
    selectedNpcId: null,
    selectedTopic: null,
    layerFilter: 'all',
  }),
}))
