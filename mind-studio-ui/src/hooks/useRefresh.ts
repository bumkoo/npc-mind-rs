import { useCallback } from 'react'
import { api } from '../api/client'
import { useEntityStore } from '../stores/useEntityStore'
import { useSceneStore } from '../stores/useSceneStore'
import { useResultStore } from '../stores/useResultStore'
import { useUIStore } from '../stores/useUIStore'
import type { Npc, Relationship, GameObject, TurnHistory, ScenarioEntry, ScenarioMeta, SceneInfo, ScenarioSeeds } from '../types'

export function useRefresh() {
  const setNpcs = useEntityStore((s) => s.setNpcs)
  const setRels = useEntityStore((s) => s.setRels)
  const setObjects = useEntityStore((s) => s.setObjects)
  const setScenarios = useEntityStore((s) => s.setScenarios)
  const setHistory = useEntityStore((s) => s.setHistory)
  const setScenarioMeta = useSceneStore((s) => s.setScenarioMeta)
  const updateSceneInfo = useSceneStore((s) => s.updateSceneInfo)
  const setScenarioSeeds = useSceneStore((s) => s.setScenarioSeeds)
  const setTestReport = useResultStore((s) => s.setTestReport)
  const setConnected = useUIStore((s) => s.setConnected)

  const refresh = useCallback(async () => {
    try {
      const [n, r, o, s, h, sm, si, tr, ss] = await Promise.all([
        api.get<Npc[]>('/api/npcs'),
        api.get<Relationship[]>('/api/relationships'),
        api.get<GameObject[]>('/api/objects'),
        api.get<ScenarioEntry[]>('/api/scenarios'),
        api.get<TurnHistory[]>('/api/history'),
        api.get<ScenarioMeta>('/api/scenario-meta'),
        api.get<SceneInfo>('/api/scene-info'),
        api.get<{ content?: string }>('/api/test-report'),
        api.get<ScenarioSeeds>('/api/scenario-seeds').catch(() => ({})),
      ])
      setNpcs(n)
      setRels(r)
      setObjects(o)
      setScenarios(s)
      setHistory(h)
      setScenarioMeta(sm && (sm as ScenarioMeta).name ? sm : null)
      setScenarioSeeds(ss || {})

      // Optimistic update logic for sceneInfo
      updateSceneInfo((prev) => {
        const next = si && si.has_scene ? si : null
        if (!prev || !next) return next
        if (prev.active_focus_id !== next.active_focus_id) return next
        if ((prev.script_cursor || 0) > (next.script_cursor || 0)) {
          return { ...next, script_cursor: prev.script_cursor }
        }
        return next
      })

      setTestReport(tr?.content || '')
      setConnected(true)
    } catch {
      setConnected(false)
    }
  }, [setNpcs, setRels, setObjects, setScenarios, setHistory, setScenarioMeta, updateSceneInfo, setScenarioSeeds, setTestReport, setConnected])

  return refresh
}
