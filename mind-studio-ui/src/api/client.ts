export const api = {
  get: <T = unknown>(url: string, opts?: { signal?: AbortSignal }): Promise<T> => {
    const p = opts?.signal ? fetch(url, { signal: opts.signal }) : fetch(url)
    return p.then((r) => {
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`)
      return r.json()
    })
  },

  post: (url: string, data?: unknown): Promise<Response> =>
    fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

  put: (url: string, data?: unknown): Promise<Response> =>
    fetch(url, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

  del: (url: string): Promise<Response> =>
    fetch(url, { method: 'DELETE' }),

  postJson: <T = unknown>(url: string, data?: unknown): Promise<T> =>
    fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }).then((r) => {
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`)
      return r.json()
    }),
}
