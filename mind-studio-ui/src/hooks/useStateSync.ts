import { useEffect, useRef } from 'react'
import { api } from '../api/client'
import { useEntityStore } from '../stores/useEntityStore'
import { useSceneStore } from '../stores/useSceneStore'
import { useResultStore } from '../stores/useResultStore'
import type { Npc, Relationship, GameObject, TurnHistory, ScenarioEntry, ScenarioMeta, SceneInfo, Situation } from '../types'

/**
 * SSE 기반 실시간 상태 동기화 훅.
 * 백엔드 /api/events 엔드포인트를 구독하여 MCP/REST 상태 변경을
 * 이벤트 종류별 targeted re-fetch로 Zustand 스토어에 반영한다.
 */
export function useStateSync(refresh: () => Promise<void>) {
  const refreshRef = useRef(refresh)
  refreshRef.current = refresh

  useEffect(() => {
    let es: EventSource | null = null
    let retryDelay = 1000
    let closed = false

    // 디바운스: 동일 fetch가 100ms 내 중복 요청되지 않도록
    const pending = new Map<string, ReturnType<typeof setTimeout>>()
    function debounced(key: string, fn: () => void) {
      if (pending.has(key)) return
      fn()
      pending.set(key, setTimeout(() => pending.delete(key), 100))
    }

    // Targeted re-fetch 함수들
    function fetchNpcs() {
      debounced('npcs', () => {
        api.get<Npc[]>('/api/npcs').then((n) => useEntityStore.getState().setNpcs(n)).catch(() => {})
      })
    }
    function fetchRels() {
      debounced('rels', () => {
        api.get<Relationship[]>('/api/relationships').then((r) => useEntityStore.getState().setRels(r)).catch(() => {})
      })
    }
    function fetchObjs() {
      debounced('objs', () => {
        api.get<GameObject[]>('/api/objects').then((o) => useEntityStore.getState().setObjects(o)).catch(() => {})
      })
    }
    function fetchHistory() {
      debounced('history', () => {
        api.get<TurnHistory[]>('/api/history').then((h) => useEntityStore.getState().setHistory(h)).catch(() => {})
      })
    }
    function fetchScenarios() {
      debounced('scenarios', () => {
        api.get<ScenarioEntry[]>('/api/scenarios').then((s) => useEntityStore.getState().setScenarios(s)).catch(() => {})
      })
    }
    function fetchSceneInfo() {
      debounced('scene', () => {
        api.get<SceneInfo>('/api/scene-info').then((si) => {
          useSceneStore.getState().updateSceneInfo((prev) => {
            const next = si && si.has_scene ? si : null
            if (!prev || !next) return next
            if (prev.active_focus_id !== next.active_focus_id) return next
            if ((prev.script_cursor || 0) > (next.script_cursor || 0)) {
              return { ...next, script_cursor: prev.script_cursor }
            }
            return next
          })
        }).catch(() => {})
      })
    }
    function fetchScenarioMeta() {
      debounced('meta', () => {
        api.get<ScenarioMeta>('/api/scenario-meta').then((sm) => {
          useSceneStore.getState().setScenarioMeta(sm && (sm as ScenarioMeta).name ? sm : null)
        }).catch(() => {})
      })
    }
    function fetchSituation() {
      debounced('situation', () => {
        api.get<Situation | null>('/api/situation').then((s) => {
          useSceneStore.getState().setSavedSituation(s)
        }).catch(() => {})
      })
    }
    function fetchReport() {
      debounced('report', () => {
        api.get<{ content?: string }>('/api/test-report').then((tr) => {
          useResultStore.getState().setTestReport(tr?.content || '')
        }).catch(() => {})
      })
    }
    function fetchHistoryAndScene() {
      fetchHistory()
      fetchSceneInfo()
    }
    function fetchRelsAndHistory() {
      fetchRels()
      fetchHistory()
    }

    function connect() {
      if (closed) return
      // 이전 연결의 debounce 타이머 정리
      pending.forEach((t) => clearTimeout(t))
      pending.clear()
      es = new EventSource('/api/events')

      es.addEventListener('connected', () => { retryDelay = 1000 })

      // 엔티티
      es.addEventListener('npc_changed', () => fetchNpcs())
      es.addEventListener('relationship_changed', () => fetchRels())
      es.addEventListener('object_changed', () => fetchObjs())

      // 액션
      es.addEventListener('appraised', () => fetchHistoryAndScene())
      es.addEventListener('stimulus_applied', () => fetchHistoryAndScene())
      es.addEventListener('after_dialogue', () => fetchRelsAndHistory())
      es.addEventListener('guide_generated', () => {}) // 가이드는 요청자에게만 관련
      es.addEventListener('scene_started', () => { fetchHistoryAndScene(); fetchScenarioMeta() })
      es.addEventListener('scene_info_changed', () => fetchSceneInfo())

      // 시나리오 라이프사이클 — 전체 상태 교체
      es.addEventListener('scenario_loaded', () => refreshRef.current())
      es.addEventListener('result_loaded', () => refreshRef.current())
      es.addEventListener('scenario_saved', () => fetchScenarios())

      // 개별 필드
      es.addEventListener('situation_changed', () => fetchSituation())
      es.addEventListener('test_report_changed', () => fetchReport())

      // 채팅
      es.addEventListener('chat_started', () => fetchHistory())
      es.addEventListener('chat_turn_completed', () => fetchHistory())
      es.addEventListener('chat_ended', () => fetchRelsAndHistory())

      es.addEventListener('history_changed', () => fetchHistory())

      // 이벤트 누락 시 전체 동기화
      es.addEventListener('resync', () => refreshRef.current())

      es.onerror = () => {
        es?.close()
        if (!closed) {
          setTimeout(connect, retryDelay)
          retryDelay = Math.min(retryDelay * 2, 30000)
        }
      }
    }

    connect()
    return () => {
      closed = true
      es?.close()
      pending.forEach((t) => clearTimeout(t))
      pending.clear()
    }
  }, [])
}
