import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useToast } from '../hooks/useToast'

describe('useToast', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('toast 추가 시 목록에 나타남', () => {
    const { result } = renderHook(() => useToast())
    act(() => { result.current.toast('테스트 메시지', 'success') })
    expect(result.current.toasts).toHaveLength(1)
    expect(result.current.toasts[0].msg).toBe('테스트 메시지')
    expect(result.current.toasts[0].type).toBe('success')
  })

  it('기본 타입은 info', () => {
    const { result } = renderHook(() => useToast())
    act(() => { result.current.toast('정보') })
    expect(result.current.toasts[0].type).toBe('info')
  })

  it('3초 후 자동 제거', () => {
    const { result } = renderHook(() => useToast())
    act(() => { result.current.toast('사라질 메시지') })
    expect(result.current.toasts).toHaveLength(1)
    act(() => { vi.advanceTimersByTime(3000) })
    expect(result.current.toasts).toHaveLength(0)
  })

  it('여러 toast 동시 관리', () => {
    const { result } = renderHook(() => useToast())
    act(() => {
      result.current.toast('첫번째', 'success')
      result.current.toast('두번째', 'error')
      result.current.toast('세번째', 'info')
    })
    expect(result.current.toasts).toHaveLength(3)
    // 3초 후 모두 제거
    act(() => { vi.advanceTimersByTime(3000) })
    expect(result.current.toasts).toHaveLength(0)
  })

  it('toast 함수가 안정적 참조 (useCallback)', () => {
    const { result, rerender } = renderHook(() => useToast())
    const firstRef = result.current.toast
    rerender()
    expect(result.current.toast).toBe(firstRef)
  })
})
