import { useEffect, useMemo, useState } from 'react'
import { api } from '../../api/client'
import { useMemoryStore } from '../../stores/useMemoryStore'
import type { MemoryEntry, MemoryListResponse, MemoryScope, MemorySource } from '../../types'

/**
 * Topic별 MemoryEntry 이력 뷰어 (Step E3.1).
 *
 * - Topic 입력 → `GET /api/memory/by-topic/{topic}` (supersede 이력 전체 포함).
 * - Canonical(provenance=seeded ∧ scope=world) 엔트리를 강조.
 * - superseded 체인을 시각적으로 표시.
 */
export default function TopicHistoryView() {
  const topicEntries = useMemoryStore((s) => s.topicEntries)
  const selectedTopic = useMemoryStore((s) => s.selectedTopic)
  const loading = useMemoryStore((s) => s.loading)
  const setTopicEntries = useMemoryStore((s) => s.setTopicEntries)
  const setSelectedTopic = useMemoryStore((s) => s.setSelectedTopic)
  const setLoading = useMemoryStore((s) => s.setLoading)

  const [draft, setDraft] = useState(selectedTopic || '')

  // selectedTopic이 외부(시나리오 로드 등에 의한 clear)에서 변경되면 draft도 sync.
  // 사용자 타이핑 중 직접 입력과 간섭하지 않도록 selectedTopic이 진짜 바뀔 때만 반영.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => { setDraft(selectedTopic || '') }, [selectedTopic])

  // selectedTopic 변경 시 fetch.
  useEffect(() => {
    if (!selectedTopic) {
      setTopicEntries([])
      setLoading(false)
      return
    }
    setTopicEntries([])
    setLoading(true)
    const ctrl = new AbortController()
    api
      .get<MemoryListResponse>(`/api/memory/by-topic/${encodeURIComponent(selectedTopic)}`, { signal: ctrl.signal })
      .then((r) => setTopicEntries(r.entries || []))
      .catch((e) => {
        if (e?.name !== 'AbortError') setTopicEntries([])
      })
      .finally(() => {
        if (!ctrl.signal.aborted) setLoading(false)
      })
    return () => ctrl.abort()
  }, [selectedTopic, setTopicEntries, setLoading])

  // Canonical 하나 식별 + 나머지는 시간순 (이미 created_seq DESC).
  const canonical = useMemo(
    () => topicEntries.find((e) => isCanonical(e)) || null,
    [topicEntries],
  )
  const others = useMemo(
    () => topicEntries.filter((e) => e !== canonical),
    [topicEntries, canonical],
  )

  function submit() {
    const t = draft.trim()
    setSelectedTopic(t ? t : null)
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', gap: 8 }}>
      {/* Search input */}
      <div style={{ display: 'flex', gap: 4, alignItems: 'center' }}>
        <label style={{ fontSize: 11, color: 'var(--fg3)' }}>Topic</label>
        <input
          type="text"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Enter') submit() }}
          placeholder="예: sect:leader"
          style={{
            flex: 1,
            background: 'var(--bg3)', color: 'var(--fg)',
            border: '1px solid var(--border)', borderRadius: 'var(--radius)',
            padding: '3px 8px', fontSize: 11,
          }}
        />
        <button className="btn small" onClick={submit}>조회</button>
        {selectedTopic && (
          <button
            className="btn small ghost"
            onClick={() => { setDraft(''); setSelectedTopic(null) }}
          >
            지우기
          </button>
        )}
      </div>

      {/* Canonical banner */}
      {canonical && (
        <div
          data-testid="canonical-banner"
          style={{
            borderLeft: '3px solid var(--warning)',
            background: 'var(--bg4)',
            padding: '6px 10px',
            borderRadius: 'var(--radius)',
            fontSize: 12,
          }}
        >
          <div style={{ fontSize: 10, fontWeight: 600, color: 'var(--warning)', marginBottom: 4 }}>
            👑 Canonical (τ=∞, Seeded + World)
          </div>
          <div style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
            {canonical.content}
          </div>
          <div style={{ marginTop: 4, fontSize: 10, color: 'var(--fg3)' }}>
            id: {canonical.id} · seq: {canonical.created_seq}
          </div>
        </div>
      )}

      {/* Timeline */}
      <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
        {!selectedTopic ? (
          <div className="empty" style={{ padding: 16, fontSize: 12 }}>
            Topic을 입력하면 해당 주제의 기억 이력(superseded 포함)을 시간 역순으로 표시합니다.
            Canonical(Seeded + World)은 τ=∞로 상단에 고정됩니다.
          </div>
        ) : loading ? (
          <div className="empty" style={{ padding: 16, fontSize: 12 }}>로딩 중…</div>
        ) : topicEntries.length === 0 ? (
          <div className="empty" style={{ padding: 16, fontSize: 12 }}>
            '{selectedTopic}' 토픽의 기억이 없습니다.
          </div>
        ) : others.length === 0 && canonical ? (
          <div style={{ fontSize: 11, color: 'var(--fg3)', padding: 8 }}>
            이 Topic의 엔트리는 Canonical 1건뿐입니다.
          </div>
        ) : (
          others.map((e) => <TimelineRow key={e.id} entry={e} />)
        )}
      </div>
    </div>
  )
}

function TimelineRow({ entry }: { entry: MemoryEntry }) {
  const isSuperseded = entry.superseded_by !== null
  const age = useMemo(() => formatAge(entry.timestamp_ms), [entry.timestamp_ms])
  return (
    <div
      style={{
        padding: '6px 8px',
        borderRadius: 'var(--radius)',
        background: 'var(--bg3)',
        borderLeft: `3px solid ${isSuperseded ? 'var(--fg3)' : 'var(--accent)'}`,
        opacity: isSuperseded ? 0.75 : 1,
        fontSize: 12,
      }}
    >
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginBottom: 4 }}>
        <Badge color={scopeColor(entry.scope)}>{scopeLabel(entry.scope)}</Badge>
        <Badge color={sourceColor(entry.source)}>{sourceLabel(entry.source)}</Badge>
        <Badge color="var(--bg4)">Layer {entry.layer}</Badge>
        {entry.provenance === 'seeded' && (
          <Badge color="var(--warning)">Seeded</Badge>
        )}
        {isSuperseded && (
          <Badge color="var(--fg3)" title={`→ ${entry.superseded_by}에 의해 대체`}>
            superseded → {entry.superseded_by}
          </Badge>
        )}
      </div>
      <div style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>{entry.content}</div>
      <div style={{ marginTop: 4, fontSize: 10, color: 'var(--fg3)' }}>
        id: {entry.id} · seq: {entry.created_seq} · ⏱ {age}
      </div>
    </div>
  )
}

function Badge({ color, title, children }: { color: string; title?: string; children: React.ReactNode }) {
  return (
    <span
      title={title}
      style={{
        background: color,
        color: 'var(--bg)',
        padding: '1px 6px',
        borderRadius: 3,
        fontSize: 9,
        fontWeight: 600,
        whiteSpace: 'nowrap',
      }}
    >
      {children}
    </span>
  )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function isCanonical(e: MemoryEntry): boolean {
  return e.provenance === 'seeded' && e.scope.kind === 'world'
}

function scopeLabel(s: MemoryScope): string {
  switch (s.kind) {
    case 'personal': return `🧍 ${s.npc_id}`
    case 'relationship': return `🤝 ${s.a}↔${s.b}`
    case 'faction': return `⚔ ${s.faction_id}`
    case 'family': return `🏠 ${s.family_id}`
    case 'world': return `🌍 ${s.world_id}`
  }
}

function scopeColor(s: MemoryScope): string {
  switch (s.kind) {
    case 'personal': return 'var(--accent)'
    case 'relationship': return 'var(--accent2)'
    case 'faction': return 'var(--warning)'
    case 'family': return 'var(--warning)'
    case 'world': return 'var(--fg)'
  }
}

function sourceLabel(src: MemorySource): string {
  switch (src) {
    case 'experienced': return '[겪음]'
    case 'witnessed': return '[목격]'
    case 'heard': return '[전해 들음]'
    case 'rumor': return '[소문]'
  }
}

function sourceColor(src: MemorySource): string {
  switch (src) {
    case 'experienced': return 'var(--accent)'
    case 'witnessed': return 'var(--accent2)'
    case 'heard': return 'var(--warning)'
    case 'rumor': return 'var(--fg3)'
  }
}

function formatAge(timestampMs: number): string {
  const now = Date.now()
  const diff = Math.max(0, now - timestampMs)
  const sec = Math.floor(diff / 1000)
  if (sec < 60) return `${sec}s`
  const min = Math.floor(sec / 60)
  if (min < 60) return `${min}m`
  const hr = Math.floor(min / 60)
  if (hr < 24) return `${hr}h`
  const day = Math.floor(hr / 24)
  if (day < 30) return `${day}d`
  return `${Math.floor(day / 30)}mo`
}
