import { useCallback } from 'react'
import { api } from '../api/client'
import { useEntityStore } from '../stores/useEntityStore'
import { useSceneStore } from '../stores/useSceneStore'
import { useResultStore } from '../stores/useResultStore'
import { useUIStore } from '../stores/useUIStore'
import type { Npc, Relationship, GameObject, TurnHistory, ScenarioEntry, ScenarioMeta, SceneInfo } from '../types'

export function useRefresh() {
  const setNpcs = useEntityStore((s) => s.setNpcs)
  const setRels = useEntityStore((s) => s.setRels)
  const setObjects = useEntityStore((s) => s.setObjects)
  const setScenarios = useEntityStore((s) => s.setScenarios)
  const setHistory = useEntityStore((s) => s.setHistory)
  const setScenarioMeta = useSceneStore((s) => s.setScenarioMeta)
  const updateSceneInfo = useSceneStore((s) => s.updateSceneInfo)
  const setTestReport = useResultStore((s) => s.setTestReport)
  const setConnected = useUIStore((s) => s.setConnected)

  // NOTE (E3.3 follow-up M1): `/api/scenario-seeds`는 시나리오 라이프사이클(load/
  // result_load)에서만 변하는 데이터라 빈번한 NPC·관계 CRUD refresh마다 fetch할
  // 필요가 없다. useStateSync의 scenario_loaded/result_loaded 이벤트 + 최초
  // 마운트(App.tsx)에서만 `fetchScenarioSeeds`를 호출한다.
  const refresh = useCallback(async () => {
    try {
      const [n, r, o, s, h, sm, si, tr] = await Promise.all([
        api.get<Npc[]>('/api/npcs'),
        api.get<Relationship[]>('/api/relationships'),
        api.get<GameObject[]>('/api/objects'),
        api.get<ScenarioEntry[]>('/api/scenarios'),
        api.get<TurnHistory[]>('/api/history'),
        api.get<ScenarioMeta>('/api/scenario-meta'),
        api.get<SceneInfo>('/api/scene-info'),
        api.get<{ content?: string }>('/api/test-report'),
      ])
      setNpcs(n)
      setRels(r)
      setObjects(o)
      setScenarios(s)
      setHistory(h)
      setScenarioMeta(sm && (sm as ScenarioMeta).name ? sm : null)

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
  }, [setNpcs, setRels, setObjects, setScenarios, setHistory, setScenarioMeta, updateSceneInfo, setTestReport, setConnected])

  return refresh
}
