import { useEffect, useMemo } from 'react'
import { api } from '../../api/client'
import { useMemoryStore } from '../../stores/useMemoryStore'
import type { Rumor, RumorListResponse, RumorOrigin, RumorStatus } from '../../types'

/**
 * `/api/rumors` 결과를 status·topic·hops와 함께 표시한다 (Step E2).
 *
 * 읽기 전용. 시딩·확산은 Step E3(편집 GUI).
 */
export default function RumorView() {
  const rumors = useMemoryStore((s) => s.rumors)
  const setRumors = useMemoryStore((s) => s.setRumors)

  // 마운트 시 1회 로드. 이후는 SSE(rumor_*)가 갱신.
  useEffect(() => {
    api
      .get<RumorListResponse>('/api/rumors')
      .then((r) => setRumors(r.rumors || []))
      .catch(() => setRumors([]))
  }, [setRumors])

  const sorted = useMemo(
    () => [...rumors].sort((a, b) => b.created_at - a.created_at),
    [rumors],
  )

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', gap: 8 }}>
      <div style={{ fontSize: 11, color: 'var(--fg3)' }}>
        총 {rumors.length}건 — Active {rumors.filter((r) => r.status === 'active').length},
        Fading {rumors.filter((r) => r.status === 'fading').length},
        Faded {rumors.filter((r) => r.status === 'faded').length}
      </div>

      <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
        {sorted.length === 0 ? (
          <div className="empty" style={{ padding: 16, fontSize: 12 }}>
            활성 소문이 없습니다. `/api/rumors/seed` 또는 MCP 도구로 시딩하세요.
          </div>
        ) : (
          sorted.map((r) => <RumorCard key={r.id} rumor={r} />)
        )}
      </div>
    </div>
  )
}

function RumorCard({ rumor }: { rumor: Rumor }) {
  const isOrphan = rumor.topic === null
  const totalRecipients = rumor.hops.reduce((sum, h) => sum + h.recipients.length, 0)
  const originDesc = describeOrigin(rumor.origin)
  const contentSummary = rumor.seed_content || (rumor.topic ? `(topic: ${rumor.topic} — Canonical 참조)` : '(내용 미정)')

  return (
    <div
      style={{
        padding: '8px 10px',
        borderRadius: 'var(--radius)',
        background: 'var(--bg3)',
        borderLeft: `3px solid ${statusColor(rumor.status)}`,
        fontSize: 12,
      }}
    >
      {/* Badges */}
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginBottom: 4 }}>
        <Badge color={statusColor(rumor.status)}>{rumor.status}</Badge>
        {isOrphan ? (
          <Badge color="var(--warning)" title="Topic 없음 — Canonical 연결 안 됨">고아</Badge>
        ) : (
          <Badge color="var(--accent2)" title={`Topic: ${rumor.topic}`}>🏷 {rumor.topic}</Badge>
        )}
        <Badge color="var(--bg4)">{originDesc}</Badge>
        <Badge color="var(--bg4)" title="id">{rumor.id}</Badge>
      </div>

      {/* Seed content */}
      <div style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word', marginBottom: 6, color: 'var(--fg)' }}>
        {contentSummary}
      </div>

      {/* Reach summary */}
      <div style={{ fontSize: 10, color: 'var(--fg3)', marginBottom: 4 }}>
        reach: {describeReach(rumor)}
      </div>

      {/* Hops */}
      {rumor.hops.length > 0 && (
        <details style={{ marginBottom: 4 }}>
          <summary style={{ fontSize: 10, color: 'var(--fg3)', cursor: 'pointer' }}>
            {rumor.hops.length} hop{rumor.hops.length > 1 ? 's' : ''} · 수신자 총 {totalRecipients}명
          </summary>
          <div style={{ marginTop: 4, display: 'flex', flexDirection: 'column', gap: 2 }}>
            {rumor.hops.map((h) => (
              <div key={h.hop_index} style={{ fontSize: 10, color: 'var(--fg2)' }}>
                hop {h.hop_index}: {h.recipients.join(', ')}
                {h.content_version ? ` [변형 ${h.content_version}]` : ''}
              </div>
            ))}
          </div>
        </details>
      )}

      {/* Distortions */}
      {rumor.distortions.length > 0 && (
        <details>
          <summary style={{ fontSize: 10, color: 'var(--fg3)', cursor: 'pointer' }}>
            변형 {rumor.distortions.length}종
          </summary>
          <div style={{ marginTop: 4, display: 'flex', flexDirection: 'column', gap: 2 }}>
            {rumor.distortions.map((d) => (
              <div key={d.id} style={{ fontSize: 10, color: 'var(--fg2)' }}>
                <strong>{d.id}</strong>
                {d.parent ? ` ← ${d.parent}` : ' (원본 기반)'}: {d.content}
              </div>
            ))}
          </div>
        </details>
      )}
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

function statusColor(s: RumorStatus): string {
  switch (s) {
    case 'active': return 'var(--accent)'
    case 'fading': return 'var(--warning)'
    case 'faded': return 'var(--fg3)'
  }
}

/** Rust unit-enum 직렬화는 string("Seeded") 또는 {"FromWorldEvent":{...}} 형태. */
function describeOrigin(o: RumorOrigin): string {
  if (typeof o === 'string') return o
  if ('FromWorldEvent' in o && o.FromWorldEvent) return `from event#${o.FromWorldEvent.event_id}`
  if ('Authored' in o && o.Authored) return o.Authored.by ? `by ${o.Authored.by}` : 'authored'
  if ('Seeded' in o) return 'Seeded'
  return 'unknown'
}

function describeReach(r: Rumor): string {
  const parts: string[] = []
  if (r.reach_policy.regions.length) parts.push(`regions[${r.reach_policy.regions.length}]`)
  if (r.reach_policy.factions.length) parts.push(`factions[${r.reach_policy.factions.length}]`)
  if (r.reach_policy.npc_ids.length) parts.push(`npcs[${r.reach_policy.npc_ids.length}]`)
  if (r.reach_policy.min_significance > 0) parts.push(`sig≥${r.reach_policy.min_significance.toFixed(2)}`)
  return parts.length ? parts.join(' · ') : '제한 없음'
}
