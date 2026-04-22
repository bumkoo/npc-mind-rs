import { useEffect, useMemo } from 'react'
import { api } from '../../api/client'
import { useMemoryStore } from '../../stores/useMemoryStore'
import type { MemoryEntry, MemoryLayer, MemoryListResponse, MemoryScope, MemorySource, MemoryType, Npc, Provenance } from '../../types'

interface MemoryViewProps {
  npcs: Npc[]
}

/**
 * `/api/memory/by-npc/:id` 결과를 표시한다 (Step E2).
 *
 * - NPC 선택 드롭다운 (기본: npc_id URL 파라미터 없음 상태에서는 미선택).
 * - Layer A/B 필터 (탭 느낌의 버튼 그룹).
 * - Scope/Source/Provenance/Type 뱃지, retention bar, recall count.
 *
 * 실시간 갱신은 `useStateSync`의 `memory_created/superseded/consolidated` 이벤트가
 * 처리. NPC 변경 시 수동 fetch 추가.
 */
export default function MemoryView({ npcs }: MemoryViewProps) {
  const entries = useMemoryStore((s) => s.entriesByNpc)
  const selectedNpcId = useMemoryStore((s) => s.selectedNpcId)
  const layerFilter = useMemoryStore((s) => s.layerFilter)
  const loading = useMemoryStore((s) => s.loading)
  const setEntries = useMemoryStore((s) => s.setEntries)
  const setSelectedNpcId = useMemoryStore((s) => s.setSelectedNpcId)
  const setLayerFilter = useMemoryStore((s) => s.setLayerFilter)
  const setLoading = useMemoryStore((s) => s.setLoading)

  // NPC 선택 시 targeted fetch.
  useEffect(() => {
    if (!selectedNpcId) {
      setEntries([])
      return
    }
    setLoading(true)
    api
      .get<MemoryListResponse>(`/api/memory/by-npc/${encodeURIComponent(selectedNpcId)}`)
      .then((r) => setEntries(r.entries || []))
      .catch(() => setEntries([]))
      .finally(() => setLoading(false))
  }, [selectedNpcId, setEntries, setLoading])

  const filtered = useMemo(() => {
    if (layerFilter === 'all') return entries
    return entries.filter((e) => e.layer === layerFilter)
  }, [entries, layerFilter])

  const counts = useMemo(() => {
    const a = entries.filter((e) => e.layer === 'A').length
    const b = entries.filter((e) => e.layer === 'B').length
    return { a, b, total: entries.length }
  }, [entries])

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', gap: 8 }}>
      {/* Controls */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
        <label style={{ fontSize: 11, color: 'var(--fg3)' }}>NPC</label>
        <select
          value={selectedNpcId || ''}
          onChange={(e) => setSelectedNpcId(e.target.value || null)}
          style={{
            background: 'var(--bg3)', color: 'var(--fg)',
            border: '1px solid var(--border)', borderRadius: 'var(--radius)',
            padding: '3px 8px', fontSize: 11,
          }}
        >
          <option value="">— 선택 —</option>
          {npcs.map((n) => (
            <option key={n.id} value={n.id}>{n.name} ({n.id})</option>
          ))}
        </select>

        <div style={{ display: 'flex', gap: 2, marginLeft: 'auto' }}>
          {(['all', 'A', 'B'] as const).map((l) => {
            const label = l === 'all' ? `전체 ${counts.total}` : l === 'A' ? `Layer A ${counts.a}` : `Layer B ${counts.b}`
            return (
              <button
                key={l}
                className={`btn small ${layerFilter === l ? '' : 'ghost'}`}
                style={{ fontSize: 10, padding: '2px 8px' }}
                onClick={() => setLayerFilter(l)}
              >
                {label}
              </button>
            )
          })}
        </div>
      </div>

      {/* List */}
      <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
        {!selectedNpcId ? (
          <div className="empty" style={{ padding: 16, fontSize: 12 }}>
            NPC를 선택하면 그가 접근 가능한 기억 (Personal + 참여 Relationship + World)을 표시합니다.
          </div>
        ) : loading ? (
          <div className="empty" style={{ padding: 16, fontSize: 12 }}>로딩 중…</div>
        ) : filtered.length === 0 ? (
          <div className="empty" style={{ padding: 16, fontSize: 12 }}>
            {entries.length === 0 ? '기억이 아직 없습니다.' : '필터에 해당하는 기억이 없습니다.'}
          </div>
        ) : (
          filtered.map((e) => <MemoryRow key={e.id} entry={e} />)
        )}
      </div>
    </div>
  )
}

// ---------------------------------------------------------------------------
// Row
// ---------------------------------------------------------------------------

function MemoryRow({ entry }: { entry: MemoryEntry }) {
  const retention = useMemo(() => computeRetention(entry), [entry])
  const age = useMemo(() => formatAge(entry.timestamp_ms), [entry.timestamp_ms])

  return (
    <div
      style={{
        padding: '6px 8px',
        borderRadius: 'var(--radius)',
        background: 'var(--bg3)',
        borderLeft: `3px solid ${sourceColor(entry.source)}`,
        fontSize: 12,
      }}
    >
      {/* Badges */}
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginBottom: 4 }}>
        <Badge color={scopeColor(entry.scope)} title={scopeTooltip(entry.scope)}>
          {scopeLabel(entry.scope)}
        </Badge>
        <Badge color={sourceColor(entry.source)}>{sourceLabel(entry.source)}</Badge>
        <Badge color="var(--bg4)">{typeLabel(entry.memory_type)}</Badge>
        <Badge color="var(--bg4)">Layer {entry.layer}</Badge>
        {entry.provenance === 'seeded' && (
          <Badge color="var(--warning)" title="작가 시드 — 런타임에서 생성되지 않음">Seeded</Badge>
        )}
        {entry.superseded_by && (
          <Badge color="var(--fg3)" title={`${entry.superseded_by}에 의해 대체됨`}>superseded</Badge>
        )}
        {entry.consolidated_into && (
          <Badge color="var(--fg3)" title={`Layer B ${entry.consolidated_into}로 흡수됨`}>consolidated</Badge>
        )}
        {entry.topic && (
          <Badge color="var(--accent2)" title={`Topic: ${entry.topic}`}>🏷 {entry.topic}</Badge>
        )}
      </div>

      {/* Content */}
      <div style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word', marginBottom: 4 }}>
        {entry.content}
      </div>

      {/* Meta row — retention bar + recall + age + confidence */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 10, color: 'var(--fg3)' }}>
        <RetentionBar value={retention} />
        <span title="회상 누적 횟수">🔁 {entry.recall_count}</span>
        <span title="생성 시점">⏱ {age}</span>
        <span title="생성 시 계산된 신뢰도 (불변)">conf {entry.confidence.toFixed(2)}</span>
        {entry.origin_chain.length > 0 && (
          <span title={`출처 체인: ${entry.origin_chain.join(' → ')}`}>
            via {entry.origin_chain.length === 1 ? entry.origin_chain[0] : `${entry.origin_chain[0]} +${entry.origin_chain.length - 1}`}
          </span>
        )}
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

function RetentionBar({ value }: { value: number }) {
  const pct = Math.max(0, Math.min(1, value)) * 100
  // cutoff 0.10 이하는 회색으로 (MEMORY_RETENTION_CUTOFF).
  const color = value < 0.10 ? 'var(--fg3)' : value < 0.4 ? 'var(--warning)' : 'var(--accent)'
  return (
    <div
      title={`retention ≈ ${value.toFixed(2)} (UI 추정, τ=30d 기준)`}
      style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}
    >
      <div style={{
        width: 40, height: 6, background: 'var(--bg4)', borderRadius: 3, overflow: 'hidden',
      }}>
        <div style={{ width: `${pct}%`, height: '100%', background: color, transition: 'width 0.2s' }} />
      </div>
      <span style={{ fontSize: 9, color: 'var(--fg3)', minWidth: 22 }}>{value.toFixed(2)}</span>
    </div>
  )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * 프런트엔드용 retention 대략치. 실제 `MemoryRanker`는 (MemoryType × MemorySource × Provenance)
 * 3축 τ 룩업 테이블을 쓰지만 UI는 서버 기본값(30d)을 가정해 시각화만 목적.
 * Canonical(Provenance=Seeded ∧ Scope=World)은 τ=∞ → 항상 1.0.
 */
function computeRetention(e: { timestamp_ms: number; last_recalled_at: number | null; recall_count: number; provenance: string; scope: { kind: string } }): number {
  if (e.provenance === 'seeded' && e.scope.kind === 'world') return 1.0
  const now = Date.now()
  const ref = e.last_recalled_at ?? e.timestamp_ms
  const ageDays = Math.max(0, (now - ref) / 86_400_000)
  const tau = 30 // DECAY_TAU_DEFAULT_DAYS
  const base = Math.exp(-ageDays / tau)
  const boost = 1 + Math.log1p(e.recall_count) * 0.15 // RECALL_BOOST_FACTOR
  return Math.max(0, Math.min(1, base * boost))
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

function scopeLabel(s: MemoryScope): string {
  switch (s.kind) {
    case 'personal': return `🧍 ${s.npc_id}`
    case 'relationship': return `🤝 ${s.a}↔${s.b}`
    case 'faction': return `⚔ ${s.faction_id}`
    case 'family': return `🏠 ${s.family_id}`
    case 'world': return `🌍 ${s.world_id}`
  }
}

function scopeTooltip(s: MemoryScope): string {
  switch (s.kind) {
    case 'personal': return `Personal — ${s.npc_id} 개인 기억`
    case 'relationship': return `Relationship — ${s.a}과(와) ${s.b}의 관계 기억 (대칭)`
    case 'faction': return `Faction — ${s.faction_id} 문파 공용`
    case 'family': return `Family — ${s.family_id} 가문 공용`
    case 'world': return `World — ${s.world_id} 세계관`
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

function typeLabel(t: MemoryType): string {
  return t
}

// Provenance는 badge에 이미 표시되므로 추가 헬퍼 불필요.
export type { Provenance, MemoryLayer }
