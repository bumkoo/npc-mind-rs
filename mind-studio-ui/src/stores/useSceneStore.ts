import { create } from 'zustand'
import type { ScenarioMeta, Situation, SceneInfo, ScenarioSeeds } from '../types'

interface SceneStore {
  scenarioMeta: ScenarioMeta | null
  savedSituation: Situation | null
  sceneInfo: SceneInfo | null
  /** 시나리오 JSON에 선언된 memory/rumor 시드 (Step E3.3 — 조회 전용). */
  scenarioSeeds: ScenarioSeeds

  setScenarioMeta: (meta: ScenarioMeta | null) => void
  setSavedSituation: (situation: Situation | null) => void
  setSceneInfo: (info: SceneInfo | null) => void
  updateSceneInfo: (updater: (prev: SceneInfo | null) => SceneInfo | null) => void
  setScenarioSeeds: (seeds: ScenarioSeeds) => void
}

export const useSceneStore = create<SceneStore>((set) => ({
  scenarioMeta: null,
  savedSituation: null,
  sceneInfo: null,
  scenarioSeeds: {},

  setScenarioMeta: (scenarioMeta) => set({ scenarioMeta }),
  setSavedSituation: (savedSituation) => set({ savedSituation }),
  setSceneInfo: (sceneInfo) => set({ sceneInfo }),
  updateSceneInfo: (updater) =>
    set((state) => ({ sceneInfo: updater(state.sceneInfo) })),
  setScenarioSeeds: (scenarioSeeds) => set({ scenarioSeeds }),
}))
