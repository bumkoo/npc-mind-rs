import { useEffect, useRef } from 'react'
import { api } from '../api/client'
import { useEntityStore } from '../stores/useEntityStore'
import { useSceneStore } from '../stores/useSceneStore'
import { useResultStore } from '../stores/useResultStore'
import { useMemoryStore } from '../stores/useMemoryStore'
import type { Npc, Relationship, GameObject, TurnHistory, ScenarioEntry, ScenarioMeta, SceneInfo, Situation, MemoryListResponse, RumorListResponse } from '../types'

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

    // 디바운스: 동일 fetch가 100ms 내 중복 요청되지 않도록 (leading edge only).
    // 엔티티 CRUD처럼 단발성 이벤트에 적합. 100ms 윈도우 안의 후속 이벤트는 drop.
    const pending = new Map<string, ReturnType<typeof setTimeout>>()
    function debounced(key: string, fn: () => void) {
      if (pending.has(key)) return
      fn()
      pending.set(key, setTimeout(() => pending.delete(key), 100))
    }

    // Leading + trailing debounce (Step E2 M4 — Memory/Rumor 이벤트 전용).
    // - 첫 이벤트는 즉시 fetch (leading).
    // - 100ms 윈도우 안에 추가 이벤트가 오면 윈도우 끝에 1회 더 fetch (trailing).
    // - 윈도우 안 이벤트가 없으면 trailing 생략.
    // 목적: 빠르게 연속되는 memory_* 이벤트가 drop되더라도 마지막 상태를 반드시
    // 반영. 단발 이벤트와 버스트 모두 정확.
    const trailing = new Map<string, { timer: ReturnType<typeof setTimeout>; retriggered: boolean; fn: () => void }>()
    function debouncedLeadingTrailing(key: string, fn: () => void) {
      const existing = trailing.get(key)
      if (!existing) {
        fn()
        const timer = setTimeout(() => {
          const entry = trailing.get(key)
          trailing.delete(key)
          if (entry?.retriggered) entry.fn()
        }, 100)
        trailing.set(key, { timer, retriggered: false, fn })
      } else {
        existing.retriggered = true
        // 최신 fn으로 교체 — selectedNpcId가 변경된 상태에서 트리거된 이벤트가 있다면
        // 윈도우 끝 fetch는 그 최신 상태를 봐야 한다.
        existing.fn = fn
      }
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

    // Step E2 — Memory/Rumor 탭 refetch (선택된 NPC에만 해당).
    // SSE는 페이로드 없이 이름만 오므로 현재 selected NPC 기준으로 갱신.
    // selected NPC가 없으면 조용히 skip (기억 탭이 열려 있지 않은 상태).
    // M4: leading+trailing 디바운스로 버스트 이벤트의 trailing 상태도 반영.
    function fetchMemoriesForSelected() {
      const npc = useMemoryStore.getState().selectedNpcId
      if (!npc) return
      debouncedLeadingTrailing(`mem:${npc}`, () => {
        const latest = useMemoryStore.getState().selectedNpcId
        if (!latest) return
        api
          .get<MemoryListResponse>(`/api/memory/by-npc/${encodeURIComponent(latest)}`)
          .then((r) => {
            // 요청 중 사용자가 다른 NPC로 전환했을 가능성 — 응답 도착 시점의 selectedNpcId와
            // 요청 대상 npc가 다르면 stale이라 무시. MemoryView의 AbortController와 중복
            // 방어이지만 SSE 경로는 MemoryView 바깥에서 호출되므로 여기서도 체크.
            if (useMemoryStore.getState().selectedNpcId === latest) {
              useMemoryStore.getState().setEntries(r.entries || [])
            }
          })
          .catch(() => {})
      })
    }
    function fetchRumors() {
      debouncedLeadingTrailing('rumors', () => {
        api
          .get<RumorListResponse>('/api/rumors')
          .then((r) => useMemoryStore.getState().setRumors(r.rumors || []))
          .catch(() => {})
      })
    }

    function connect() {
      if (closed) return
      // 이전 연결의 debounce 타이머 정리
      pending.forEach((t) => clearTimeout(t))
      pending.clear()
      trailing.forEach((e) => clearTimeout(e.timer))
      trailing.clear()
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

      // Step E2 — Memory / Rumor SSE
      // memory_created는 tell / apply_world / spread_rumor / 수동 주입에서 방출.
      // memory_superseded는 실제 기존 Canonical이 교체될 때만 (M1 수정 반영).
      // memory_consolidated는 현재 방출 지점 없음 (Step F에서 Memory 이벤트 팬아웃 시 연결).
      es.addEventListener('memory_created', () => fetchMemoriesForSelected())
      es.addEventListener('memory_superseded', () => fetchMemoriesForSelected())
      es.addEventListener('memory_consolidated', () => fetchMemoriesForSelected())
      es.addEventListener('rumor_seeded', () => fetchRumors())
      es.addEventListener('rumor_spread', () => {
        fetchRumors()
        fetchMemoriesForSelected()
      })

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
      trailing.forEach((e) => clearTimeout(e.timer))
      trailing.clear()
    }
  }, [])
}
