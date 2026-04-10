import { describe, it, expect, vi, beforeEach } from 'vitest'
import { api } from '../api/client'

beforeEach(() => {
  vi.restoreAllMocks()
})

describe('api.get', () => {
  it('성공 시 JSON 반환', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify([{ id: 'npc1' }]), { status: 200 }),
    )
    const data = await api.get<{ id: string }[]>('/api/npcs')
    expect(data).toEqual([{ id: 'npc1' }])
    expect(fetch).toHaveBeenCalledWith('/api/npcs')
  })

  it('4xx/5xx 응답 시 에러 throw', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('Not Found', { status: 404, statusText: 'Not Found' }),
    )
    await expect(api.get('/api/npcs')).rejects.toThrow('404 Not Found')
  })

  it('네트워크 에러 시 에러 전파', async () => {
    vi.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('Network error'))
    await expect(api.get('/api/npcs')).rejects.toThrow('Network error')
  })
})

describe('api.post', () => {
  it('JSON body로 POST 요청', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('', { status: 200 }))
    await api.post('/api/npcs', { id: 'test' })
    expect(fetch).toHaveBeenCalledWith('/api/npcs', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: '{"id":"test"}',
    })
  })

  it('Response 객체를 그대로 반환 (에러 체크 안 함)', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('error', { status: 500 }),
    )
    const res = await api.post('/api/npcs', {})
    expect(res.status).toBe(500)
  })
})

describe('api.postJson', () => {
  it('성공 시 JSON 파싱 반환', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify({ prompt: 'hello' }), { status: 200 }),
    )
    const data = await api.postJson<{ prompt: string }>('/api/guide', { npc_id: 'x' })
    expect(data.prompt).toBe('hello')
  })

  it('에러 응답 시 throw', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('error', { status: 400, statusText: 'Bad Request' }),
    )
    await expect(api.postJson('/api/guide', {})).rejects.toThrow('400 Bad Request')
  })
})

describe('api.put', () => {
  it('PUT 요청 전송', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('', { status: 200 }))
    await api.put('/api/test-report', { content: 'report' })
    expect(fetch).toHaveBeenCalledWith('/api/test-report', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: '{"content":"report"}',
    })
  })
})

describe('api.del', () => {
  it('DELETE 요청 전송', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('', { status: 200 }))
    await api.del('/api/npcs/test')
    expect(fetch).toHaveBeenCalledWith('/api/npcs/test', { method: 'DELETE' })
  })
})
