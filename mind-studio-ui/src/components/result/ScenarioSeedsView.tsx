import { useMemo } from 'react'
import { useSceneStore } from '../../stores/useSceneStore'
import type { MemoryEntrySeedInput, RumorOrigin, RumorSeedInput, WorldKnowledgeSeed } from '../../types'

/**
 * 시나리오 JSON에 선언된 memory/rumor 시드 조회 전용 패널 (Step E3.3).
 *
 * 데이터 출처는 `useSceneStore.scenarioSeeds` (← `GET /api/scenario-seeds` →
 * `StateInner.scenario_seeds`). 작가가 "내가 시나리오에 선언한 게 무엇인가"를
 * 빠르게 확인하기 위함. 편집은 JSON 파일 직접 또는 런타임 시드 폼(소문 탭)으로.
 */
export default function ScenarioSeedsView() {
  const seeds = useSceneStore((s) => s.scenarioSeeds)

  const counts = useMemo(() => ({
    rumors: seeds.initial_rumors?.length ?? 0,
    world: seeds.world_knowledge?.length ?? 0,
    factions: Object.values(seeds.faction_knowledge ?? {}).reduce((n, arr) => n + arr.length, 0),
    families: Object.values(seeds.family_facts ?? {}).reduce((n, arr) => n + arr.length, 0),
  }), [seeds])

  const total = counts.rumors + counts.world + counts.factions + counts.families

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', gap: 8 }}>
      <div style={{ fontSize: 11, color: 'var(--fg3)' }}>
        총 {total}건 — Rumor {counts.rumors}, World {counts.world},
        Faction {counts.factions}, Family {counts.families}
        <span style={{ marginLeft: 8, color: 'var(--fg3)' }}>
          (편집은 시나리오 JSON 직접 또는 소문 탭의 시드 폼)
        </span>
      </div>

      <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
        {total === 0 ? (
          <div className="empty" style={{ padding: 16, fontSize: 12 }}>
            현재 시나리오 JSON에 선언된 시드가 없습니다.
            <br />
            <code style={{ fontSize: 10 }}>initial_rumors</code> /{' '}
            <code style={{ fontSize: 10 }}>world_knowledge</code> /{' '}
            <code style={{ fontSize: 10 }}>faction_knowledge</code> /{' '}
            <code style={{ fontSize: 10 }}>family_facts</code>{' '}
            섹션을 시나리오 JSON 최상위에 추가해 시딩하세요.
          </div>
        ) : (
          <>
            {counts.rumors > 0 && (
              <Section title={`📰 initial_rumors (${counts.rumors})`}>
                {seeds.initial_rumors!.map((r, i) => <RumorSeedRow key={i} idx={i} seed={r} />)}
              </Section>
            )}
            {counts.world > 0 && (
              <Section title={`🌍 world_knowledge (${counts.world})`}>
                {seeds.world_knowledge!.map((w, i) => <WorldSeedRow key={i} idx={i} seed={w} />)}
              </Section>
            )}
            {counts.factions > 0 && (
              <Section title={`⚔ faction_knowledge (${counts.factions})`}>
                {Object.entries(seeds.faction_knowledge!).map(([fid, list]) => (
                  <GroupCard key={fid} groupLabel={`문파: ${fid}`} entries={list} />
                ))}
              </Section>
            )}
            {counts.families > 0 && (
              <Section title={`🏠 family_facts (${counts.families})`}>
                {Object.entries(seeds.family_facts!).map(([fid, list]) => (
                  <GroupCard key={fid} groupLabel={`가문: ${fid}`} entries={list} />
                ))}
              </Section>
            )}
          </>
        )}
      </div>
    </div>
  )
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <div style={{ fontSize: 11, fontWeight: 600, color: 'var(--accent2)', marginBottom: 4 }}>{title}</div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>{children}</div>
    </div>
  )
}

function RumorSeedRow({ idx, seed }: { idx: number; seed: RumorSeedInput }) {
  const isOrphan = !seed.topic
  return (
    <div style={baseRowStyle}>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginBottom: 4 }}>
        <Badge color="var(--bg4)">#{idx}</Badge>
        {seed.id && <Badge color="var(--bg4)" title="id">{seed.id}</Badge>}
        {isOrphan ? (
          <Badge color="var(--warning)" title="topic 없음 — 고아">고아</Badge>
        ) : (
          <Badge color="var(--accent2)">🏷 {seed.topic}</Badge>
        )}
        <Badge color="var(--bg4)">{describeOrigin(seed.origin)}</Badge>
      </div>
      <div style={contentStyle}>
        {seed.seed_content || (seed.topic ? `(topic ${seed.topic} — Canonical 참조)` : '(내용 미정)')}
      </div>
    </div>
  )
}

function WorldSeedRow({ idx, seed }: { idx: number; seed: WorldKnowledgeSeed }) {
  return (
    <div style={baseRowStyle}>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginBottom: 4 }}>
        <Badge color="var(--bg4)">#{idx}</Badge>
        {seed.id && <Badge color="var(--bg4)">{seed.id}</Badge>}
        <Badge color="var(--accent)">🌍 {seed.world_id}</Badge>
        {seed.topic && <Badge color="var(--accent2)">🏷 {seed.topic}</Badge>}
        <MemoryMetaBadges seed={seed} />
      </div>
      <div style={contentStyle}>{seed.content}</div>
    </div>
  )
}

function GroupCard({ groupLabel, entries }: { groupLabel: string; entries: MemoryEntrySeedInput[] }) {
  return (
    <div style={{ ...baseRowStyle, paddingLeft: 12 }}>
      <div style={{ fontSize: 10, fontWeight: 600, color: 'var(--accent2)', marginBottom: 4 }}>
        {groupLabel} · {entries.length}건
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        {entries.map((e, i) => (
          <div key={i} style={{ borderLeft: '2px solid var(--border)', paddingLeft: 6 }}>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginBottom: 2 }}>
              <Badge color="var(--bg4)">#{i}</Badge>
              {e.id && <Badge color="var(--bg4)">{e.id}</Badge>}
              {e.topic && <Badge color="var(--accent2)">🏷 {e.topic}</Badge>}
              <MemoryMetaBadges seed={e} />
            </div>
            <div style={contentStyle}>{e.content}</div>
          </div>
        ))}
      </div>
    </div>
  )
}

function MemoryMetaBadges({ seed }: { seed: MemoryEntrySeedInput }) {
  return (
    <>
      {seed.memory_type && <Badge color="var(--bg4)">{seed.memory_type}</Badge>}
      {seed.source && <Badge color="var(--bg4)">{seed.source}</Badge>}
      {seed.layer && <Badge color="var(--bg4)">L{seed.layer}</Badge>}
      {seed.confidence != null && seed.confidence !== 1.0 && (
        <Badge color="var(--bg4)">conf {seed.confidence.toFixed(2)}</Badge>
      )}
      {seed.acquired_by && <Badge color="var(--bg4)" title="acquired_by">by {seed.acquired_by}</Badge>}
    </>
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

function describeOrigin(o?: RumorOrigin): string {
  if (!o) return 'seeded (default)'
  switch (o.kind) {
    case 'seeded': return 'seeded'
    case 'from_world_event': return `from event#${o.event_id}`
    case 'authored': return o.by ? `by ${o.by}` : 'authored'
  }
}

const baseRowStyle: React.CSSProperties = {
  padding: '6px 8px',
  borderRadius: 'var(--radius)',
  background: 'var(--bg3)',
  fontSize: 12,
}

const contentStyle: React.CSSProperties = {
  whiteSpace: 'pre-wrap',
  wordBreak: 'break-word',
  color: 'var(--fg)',
}
