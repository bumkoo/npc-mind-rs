import { create } from 'zustand'
import type { ScenarioMeta, Situation, SceneInfo } from '../types'

interface SceneStore {
  scenarioMeta: ScenarioMeta | null
  savedSituation: Situation | null
  sceneInfo: SceneInfo | null

  setScenarioMeta: (meta: ScenarioMeta | null) => void
  setSavedSituation: (situation: Situation | null) => void
  setSceneInfo: (info: SceneInfo | null) => void
  updateSceneInfo: (updater: (prev: SceneInfo | null) => SceneInfo | null) => void
}

export const useSceneStore = create<SceneStore>((set) => ({
  scenarioMeta: null,
  savedSituation: null,
  sceneInfo: null,

  setScenarioMeta: (scenarioMeta) => set({ scenarioMeta }),
  setSavedSituation: (savedSituation) => set({ savedSituation }),
  setSceneInfo: (sceneInfo) => set({ sceneInfo }),
  updateSceneInfo: (updater) =>
    set((state) => ({ sceneInfo: updater(state.sceneInfo) })),
}))
