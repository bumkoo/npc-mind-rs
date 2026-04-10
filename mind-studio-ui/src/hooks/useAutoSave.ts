import { useRef, useCallback } from 'react'

export function useAutoSave(saveFn: (data: unknown) => Promise<void>, delay = 500) {
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const scheduleSave = useCallback(
    (data: unknown) => {
      if (timerRef.current) clearTimeout(timerRef.current)
      timerRef.current = setTimeout(() => {
        saveFn(data)
      }, delay)
    },
    [saveFn, delay],
  )

  const cancelSave = useCallback(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current)
      timerRef.current = null
    }
  }, [])

  return { scheduleSave, cancelSave }
}
