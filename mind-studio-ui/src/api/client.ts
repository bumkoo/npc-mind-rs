export const api = {
  get: <T = unknown>(url: string): Promise<T> =>
    fetch(url).then((r) => r.json()),

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
    }).then((r) => r.json()),
}
