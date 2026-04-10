import { useEffect } from 'react'
import { api } from '../api/client'
import { useChatStore } from '../stores/useChatStore'
import { useEntityStore } from '../stores/useEntityStore'
import type { TurnHistory } from '../types'

export function useChatPolling() {
  const chatMode = useChatStore((s) => s.chatMode)
  const setHistory = useEntityStore((s) => s.setHistory)

  useEffect(() => {
    if (!chatMode) return
    const interval = setInterval(async () => {
      try {
        const h = await api.get<TurnHistory[]>('/api/history')
        setHistory(h)
      } catch {
        // ignore polling errors
      }
    }, 2000)
    return () => clearInterval(interval)
  }, [chatMode, setHistory])
}
