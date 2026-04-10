import { useState, useCallback } from 'react'
import type { Toast } from '../types'

export function useToast() {
  const [toasts, setToasts] = useState<Toast[]>([])

  const toast = useCallback((msg: string, type: Toast['type'] = 'info') => {
    const id = Date.now() + Math.random()
    setToasts((prev) => [...prev, { id, msg, type }])
    setTimeout(
      () => setToasts((prev) => prev.filter((t) => t.id !== id)),
      3000,
    )
  }, [])

  return { toasts, toast }
}
