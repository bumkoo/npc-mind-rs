import { useEffect, useMemo, useState } from 'react'
import { api } from '../../api/client'
import { useMemoryStore } from '../../stores/useMemoryStore'
import type { Rumor, RumorListResponse, RumorOrigin, RumorStatus } from '../../types'

/**
 * `/api/rumors` 결과를 status·topic·hops와 함께 표시하고 시드·확산 액션을 제공한다
 * (Step E2 표시 + Step E3.1 편집 GUI).
 */
export default function RumorView() {
  const rumors = useMemoryStore((s) => s.rumors)
  const setRumors = useMemoryStore((s) => s.setRumors)
  const [seedOpen, setSeedOpen] = useState(false)
  const [err, setErr] = useState<string | null>(null)

  // 마운트 시 1회 로드. 이후는 SSE(rumor_*)가 갱신.
  useEffect(() => {
    api
      .get<RumorListResponse>('/api/rumors')
      .then((r) => setRumors(r.rumors || []))
      .catch(() => setRumors([]))
  }, [setRumors])

  async function reloadAll() {
    try {
      const r = await api.get<RumorListResponse>('/api/rumors')
      setRumors(r.rumors || [])
    } catch {
      // swallow — SSE가 후속 갱신을 처리.
    }
  }

  const sorted = useMemo(
    () => [...rumors].sort((a, b) => b.created_at - a.created_at),
    [rumors],
  )

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', gap: 8 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <div style={{ fontSize: 11, color: 'var(--fg3)', flex: 1 }}>
          총 {rumors.length}건 — Active {rumors.filter((r) => r.status === 'active').length},
          Fading {rumors.filter((r) => r.status === 'fading').length},
          Faded {rumors.filter((r) => r.status === 'faded').length}
        </div>
        <button
          className={`btn small ${seedOpen ? '' : 'ghost'}`}
          style={{ fontSize: 10, padding: '2px 10px' }}
          onClick={() => { setSeedOpen(!seedOpen); setErr(null) }}
        >
          {seedOpen ? '× 닫기' : '＋ 시드'}
        </button>
      </div>

      {seedOpen && (
        <SeedForm
          onSuccess={(id) => { setErr(null); setSeedOpen(false); reloadAll(); console.log('seeded rumor:', id) }}
          onError={(msg) => setErr(msg)}
        />
      )}
      {err && (
        <div style={{ fontSize: 11, color: 'var(--warning)', padding: '4px 8px', background: 'var(--bg3)', borderRadius: 'var(--radius)' }}>
          {err}
        </div>
      )}

      <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
        {sorted.length === 0 ? (
          <div className="empty" style={{ padding: 16, fontSize: 12 }}>
            활성 소문이 없습니다. `＋ 시드` 버튼으로 시딩하세요.
          </div>
        ) : (
          sorted.map((r) => <RumorCard key={r.id} rumor={r} reload={reloadAll} onError={setErr} />)
        )}
      </div>
    </div>
  )
}

function RumorCard({ rumor, reload, onError }: { rumor: Rumor; reload: () => void; onError: (msg: string) => void }) {
  const isOrphan = rumor.topic === null
  const totalRecipients = rumor.hops.reduce((sum, h) => sum + h.recipients.length, 0)
  const originDesc = describeOrigin(rumor.origin)
  const contentSummary = rumor.seed_content || (rumor.topic ? `(topic: ${rumor.topic} — Canonical 참조)` : '(내용 미정)')

  const [spreadOpen, setSpreadOpen] = useState(false)

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

      {/* Spread action (Step E3.1). Faded는 확산 불가 (backend가 reject). */}
      <div style={{ marginTop: 6, display: 'flex', gap: 4 }}>
        {rumor.status !== 'faded' && (
          <button
            className={`btn small ${spreadOpen ? '' : 'ghost'}`}
            style={{ fontSize: 10, padding: '2px 8px' }}
            onClick={() => setSpreadOpen(!spreadOpen)}
          >
            {spreadOpen ? '× 취소' : '확산'}
          </button>
        )}
      </div>
      {spreadOpen && (
        <SpreadForm
          rumorId={rumor.id}
          distortions={rumor.distortions.map((d) => d.id)}
          onSuccess={() => { setSpreadOpen(false); reload() }}
          onError={onError}
        />
      )}
    </div>
  )
}

// ---------------------------------------------------------------------------
// Forms (Step E3.1)
// ---------------------------------------------------------------------------

interface SeedFormProps {
  onSuccess: (rumorId: string) => void
  onError: (msg: string) => void
}

function SeedForm({ onSuccess, onError }: SeedFormProps) {
  const [topic, setTopic] = useState('')
  const [seedContent, setSeedContent] = useState('')
  const [originKind, setOriginKind] = useState<'seeded' | 'authored'>('authored')
  const [authoredBy, setAuthoredBy] = useState('')
  // Reach — 단순화를 위해 CSV 입력.
  const [regions, setRegions] = useState('')
  const [factions, setFactions] = useState('')
  const [npcIds, setNpcIds] = useState('')
  const [minSig, setMinSig] = useState('0')
  const [submitting, setSubmitting] = useState(false)

  function parseCsv(s: string): string[] {
    return s.split(',').map((x) => x.trim()).filter(Boolean)
  }

  async function submit() {
    const trimmedTopic = topic.trim()
    const trimmedSeed = seedContent.trim()
    // Orphan rumor는 topic=None + seed_content 필수 (I-RU-4).
    if (!trimmedTopic && !trimmedSeed) {
      onError('topic 또는 seed_content 중 최소 하나는 입력해야 합니다.')
      return
    }
    const sig = Number.parseFloat(minSig)
    if (Number.isNaN(sig) || sig < 0 || sig > 1) {
      onError('min_significance는 0.0~1.0 범위여야 합니다.')
      return
    }
    const origin =
      originKind === 'seeded'
        ? { kind: 'seeded' }
        : { kind: 'authored', by: authoredBy.trim() || null }
    const body = {
      topic: trimmedTopic || null,
      seed_content: trimmedSeed || null,
      reach: {
        regions: parseCsv(regions),
        factions: parseCsv(factions),
        npc_ids: parseCsv(npcIds),
        min_significance: sig,
      },
      origin,
    }
    setSubmitting(true)
    try {
      const resp = await api.postJson<{ rumor_id: string }>('/api/rumors/seed', body)
      onSuccess(resp.rumor_id)
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e)
      onError(`시드 실패: ${msg}`)
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <form
      data-testid="rumor-seed-form"
      onSubmit={(e) => { e.preventDefault(); submit() }}
      style={{
        padding: '8px 10px',
        borderRadius: 'var(--radius)',
        background: 'var(--bg4)',
        display: 'flex', flexDirection: 'column', gap: 4, fontSize: 11,
      }}
    >
      <Row label="Topic">
        <input
          type="text" value={topic} onChange={(e) => setTopic(e.target.value)}
          placeholder="예: sect:leader (선택 — 없으면 고아)" style={inputStyle}
        />
      </Row>
      <Row label="Seed content">
        <input
          type="text" value={seedContent} onChange={(e) => setSeedContent(e.target.value)}
          placeholder="소문 본문 (topic 없으면 필수)" style={inputStyle}
        />
      </Row>
      <Row label="Origin">
        <select
          value={originKind} onChange={(e) => setOriginKind(e.target.value as 'seeded' | 'authored')}
          style={inputStyle}
        >
          <option value="seeded">seeded — 작가 시드</option>
          <option value="authored">authored — NPC 작성</option>
        </select>
        {originKind === 'authored' && (
          <input
            type="text" value={authoredBy} onChange={(e) => setAuthoredBy(e.target.value)}
            placeholder="authored by (NPC id, 선택)" style={{ ...inputStyle, marginLeft: 4 }}
          />
        )}
      </Row>
      <Row label="Reach regions">
        <input type="text" value={regions} onChange={(e) => setRegions(e.target.value)} placeholder="쉼표 구분 (예: jianghu, sect_yun)" style={inputStyle} />
      </Row>
      <Row label="Reach factions">
        <input type="text" value={factions} onChange={(e) => setFactions(e.target.value)} placeholder="쉼표 구분" style={inputStyle} />
      </Row>
      <Row label="Reach npcs">
        <input type="text" value={npcIds} onChange={(e) => setNpcIds(e.target.value)} placeholder="쉼표 구분" style={inputStyle} />
      </Row>
      <Row label="Min significance">
        <input type="number" min={0} max={1} step={0.05} value={minSig} onChange={(e) => setMinSig(e.target.value)} style={{ ...inputStyle, maxWidth: 80 }} />
      </Row>
      <div>
        <button type="submit" className="btn small" disabled={submitting} style={{ fontSize: 10 }}>
          {submitting ? '시딩 중…' : '시드 생성'}
        </button>
      </div>
    </form>
  )
}

interface SpreadFormProps {
  rumorId: string
  distortions: string[]
  onSuccess: () => void
  onError: (msg: string) => void
}

function SpreadForm({ rumorId, distortions, onSuccess, onError }: SpreadFormProps) {
  const [recipients, setRecipients] = useState('')
  const [contentVersion, setContentVersion] = useState<string>('')
  const [submitting, setSubmitting] = useState(false)

  async function submit() {
    const list = recipients.split(',').map((x) => x.trim()).filter(Boolean)
    if (list.length === 0) {
      onError('recipients를 1명 이상 입력하세요.')
      return
    }
    const body: { recipients: string[]; content_version: string | null } = {
      recipients: list,
      content_version: contentVersion.trim() || null,
    }
    setSubmitting(true)
    try {
      await api.postJson(`/api/rumors/${encodeURIComponent(rumorId)}/spread`, body)
      onSuccess()
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e)
      onError(`확산 실패: ${msg}`)
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <form
      data-testid="rumor-spread-form"
      onSubmit={(e) => { e.preventDefault(); submit() }}
      style={{
        marginTop: 6,
        padding: '6px 8px',
        borderRadius: 'var(--radius)',
        background: 'var(--bg4)',
        display: 'flex', flexDirection: 'column', gap: 4, fontSize: 11,
      }}
    >
      <Row label="Recipients">
        <input
          type="text" value={recipients} onChange={(e) => setRecipients(e.target.value)}
          placeholder="쉼표 구분 (예: mu_baek, gyo_ryong)" style={inputStyle}
          autoFocus
        />
      </Row>
      <Row label="Content version">
        <select
          value={contentVersion} onChange={(e) => setContentVersion(e.target.value)}
          style={inputStyle}
          disabled={distortions.length === 0}
        >
          <option value="">(원본 — distortion 없음)</option>
          {distortions.map((id) => <option key={id} value={id}>{id}</option>)}
        </select>
      </Row>
      <div>
        <button type="submit" className="btn small" disabled={submitting} style={{ fontSize: 10 }}>
          {submitting ? '확산 중…' : '확산'}
        </button>
      </div>
    </form>
  )
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
      <span style={{ color: 'var(--fg3)', minWidth: 95, fontSize: 10 }}>{label}</span>
      <div style={{ flex: 1, display: 'flex', alignItems: 'center', gap: 2 }}>{children}</div>
    </label>
  )
}

const inputStyle: React.CSSProperties = {
  flex: 1,
  background: 'var(--bg3)', color: 'var(--fg)',
  border: '1px solid var(--border)', borderRadius: 'var(--radius)',
  padding: '2px 6px', fontSize: 11,
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

/** Rust `RumorOrigin`은 internally tagged — `kind` 필드로 분기. */
function describeOrigin(o: RumorOrigin): string {
  switch (o.kind) {
    case 'seeded': return 'Seeded'
    case 'from_world_event': return `from event#${o.event_id}`
    case 'authored': return o.by ? `by ${o.by}` : 'authored'
  }
}

function describeReach(r: Rumor): string {
  const parts: string[] = []
  if (r.reach_policy.regions.length) parts.push(`regions[${r.reach_policy.regions.length}]`)
  if (r.reach_policy.factions.length) parts.push(`factions[${r.reach_policy.factions.length}]`)
  if (r.reach_policy.npc_ids.length) parts.push(`npcs[${r.reach_policy.npc_ids.length}]`)
  if (r.reach_policy.min_significance > 0) parts.push(`sig≥${r.reach_policy.min_significance.toFixed(2)}`)
  return parts.length ? parts.join(' · ') : '제한 없음'
}
