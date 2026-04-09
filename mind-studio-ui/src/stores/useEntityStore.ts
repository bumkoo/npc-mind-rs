import { create } from 'zustand'
import { api } from '../api/client'
import type { Npc, Relationship, GameObject, TurnHistory, ScenarioEntry } from '../types'

interface EntityStore {
  npcs: Npc[]
  rels: Relationship[]
  objects: GameObject[]
  scenarios: ScenarioEntry[]
  history: TurnHistory[]

  setNpcs: (npcs: Npc[]) => void
  setRels: (rels: Relationship[]) => void
  setObjects: (objects: GameObject[]) => void
  setScenarios: (scenarios: ScenarioEntry[]) => void
  setHistory: (history: TurnHistory[]) => void

  saveNpc: (data: Npc) => Promise<void>
  deleteNpc: (id: string) => Promise<void>
  saveRel: (data: Relationship) => Promise<void>
  deleteRel: (ownerId: string, targetId: string) => Promise<void>
  saveObj: (data: GameObject) => Promise<void>
  deleteObj: (id: string) => Promise<void>
}

export const useEntityStore = create<EntityStore>((set) => ({
  npcs: [],
  rels: [],
  objects: [],
  scenarios: [],
  history: [],

  setNpcs: (npcs) => set({ npcs }),
  setRels: (rels) => set({ rels }),
  setObjects: (objects) => set({ objects }),
  setScenarios: (scenarios) => set({ scenarios }),
  setHistory: (history) => set({ history }),

  saveNpc: async (data) => {
    await api.post('/api/npcs', data)
  },
  deleteNpc: async (id) => {
    await api.del(`/api/npcs/${id}`)
  },
  saveRel: async (data) => {
    await api.post('/api/relationships', data)
  },
  deleteRel: async (ownerId, targetId) => {
    await api.del(`/api/relationships/${ownerId}/${targetId}`)
  },
  saveObj: async (data) => {
    await api.post('/api/objects', data)
  },
  deleteObj: async (id) => {
    await api.del(`/api/objects/${id}`)
  },
}))
