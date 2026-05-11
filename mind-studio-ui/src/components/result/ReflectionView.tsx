import { useResultStore } from '../../stores/useResultStore'

/**
 * Phase 1.5 — 가장 최근 `AfterDialogueResponse.reflection`을 표시하는 read-only 패널.
 *
 * 데이터 출처: `useResultStore.lastAfterDialogue` (← `/api/chat/end` 또는
 * `/api/after-dialogue` 응답 또는 SSE `dialogue_reflected` 트리거 시 fetch).
 *
 * 표시 항목:
 * - `is_chitchat` 라벨 + significance band (낮음 < 0.3 / 중간 0.3~0.7 / 높음 ≥ 0.7)
 * - LLM `summary` (1~2문장)
 * - LLM `llm_reasoning` (선택 — 합리적 설명)
 * - axes before/after (chitchat 시 동일 → 보존 확인)
 * - turn_count
 *
 * relationships.md v0.7 §6 Scene Boundary Reflection 박제. Phase 2에서
 * declarative_events / partnership_event 활성 시 본 패널에 섹션 추가 예정.
 */
export default function ReflectionView() {
  const lastAfter = useResultStore((s) => s.lastAfterDialogue)

  if (!lastAfter) {
    return (
      <div className="empty" style={{ padding: 16, fontSize: 12 }}>
        아직 reflection이 없습니다.
        <br />
        대화를 종료하면 (chat/end 또는 after_dialogue) Scene Boundary Reflection이
        박제됩니다.
      </div>
    )
  }

  const refl = lastAfter.reflection
  const before = lastAfter.before
  const after = lastAfter.after

  if (!refl) {
    return (
      <div className="empty" style={{ padding: 16, fontSize: 12 }}>
        Reflection이 부착되지 않은 dispatch입니다 (legacy 경로).
        <br />
        Mind Studio가 chat feature + ReflectionService와 함께 빌드되어야 박제됩니다.
        <br />
        Axes — closeness {before.closeness.toFixed(2)} → {after.closeness.toFixed(2)},
        trust {before.trust.toFixed(2)} → {after.trust.toFixed(2)},
        power {before.power.toFixed(2)} → {after.power.toFixed(2)}
      </div>
    )
  }

  const sig = refl.significance_score
  const band = sig < 0.3 ? '낮음 (잡담)' : sig < 0.7 ? '중간 (일상)' : '높음 (결단)'
  const bandColor = sig < 0.3 ? '#888' : sig < 0.7 ? '#a07f3f' : '#a04040'

  const axesChanged =
    Math.abs(before.closeness - after.closeness) > 0.001 ||
    Math.abs(before.trust - after.trust) > 0.001 ||
    Math.abs(before.power - after.power) > 0.001

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', gap: 12, padding: 8 }}>
      <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
        <span
          style={{
            padding: '2px 8px',
            borderRadius: 4,
            fontSize: 11,
            color: refl.is_chitchat ? '#fff' : '#fff',
            background: refl.is_chitchat ? '#888' : '#5a8',
          }}
        >
          {refl.is_chitchat ? '잡담 (chitchat)' : '의미 있음 (significant)'}
        </span>
        <span style={{ fontSize: 11, color: bandColor }}>
          significance {sig.toFixed(3)} — {band}
        </span>
        {refl.turn_count != null && (
          <span style={{ fontSize: 11, color: 'var(--fg3)' }}>
            {refl.turn_count} turn
          </span>
        )}
      </div>

      <div>
        <div style={{ fontSize: 11, color: 'var(--fg3)', marginBottom: 4 }}>요약 (summary)</div>
        <div style={{ fontSize: 13, lineHeight: 1.5, padding: 8, background: 'var(--bg2)', borderRadius: 4 }}>
          {refl.summary}
        </div>
      </div>

      {refl.llm_reasoning && (
        <div>
          <div style={{ fontSize: 11, color: 'var(--fg3)', marginBottom: 4 }}>판정 근거 (reasoning)</div>
          <div style={{ fontSize: 12, lineHeight: 1.5, padding: 8, background: 'var(--bg2)', borderRadius: 4, color: 'var(--fg2)' }}>
            {refl.llm_reasoning}
          </div>
        </div>
      )}

      <div>
        <div style={{ fontSize: 11, color: 'var(--fg3)', marginBottom: 4 }}>
          관계 변화 (axes)
          {!axesChanged && (
            <span style={{ marginLeft: 8, color: '#888' }}>
              — 변화 없음 (chitchat skip 또는 legacy 미적용)
            </span>
          )}
        </div>
        <table style={{ fontSize: 12, borderCollapse: 'collapse' }}>
          <thead>
            <tr style={{ color: 'var(--fg3)' }}>
              <th style={{ textAlign: 'left', padding: '2px 8px' }}>축</th>
              <th style={{ textAlign: 'right', padding: '2px 8px' }}>before</th>
              <th style={{ textAlign: 'right', padding: '2px 8px' }}>after</th>
              <th style={{ textAlign: 'right', padding: '2px 8px' }}>Δ</th>
            </tr>
          </thead>
          <tbody>
            <AxisRow label="closeness" before={before.closeness} after={after.closeness} />
            <AxisRow label="trust" before={before.trust} after={after.trust} />
            <AxisRow label="power" before={before.power} after={after.power} />
          </tbody>
        </table>
      </div>

      <div style={{ fontSize: 10, color: 'var(--fg3)', marginTop: 'auto' }}>
        Phase 1.5 — relationships.md v0.7 §6. declarative_events / partnership_event는
        Phase 2 Channel 1 활성 시 추가 표시.
      </div>
    </div>
  )
}

function AxisRow({ label, before, after }: { label: string; before: number; after: number }) {
  const delta = after - before
  const deltaStr = (delta >= 0 ? '+' : '') + delta.toFixed(3)
  const color =
    Math.abs(delta) < 0.001 ? 'var(--fg3)' : delta > 0 ? '#5a8' : '#c66'
  return (
    <tr>
      <td style={{ padding: '2px 8px', color: 'var(--fg2)' }}>{label}</td>
      <td style={{ padding: '2px 8px', textAlign: 'right' }}>{before.toFixed(3)}</td>
      <td style={{ padding: '2px 8px', textAlign: 'right' }}>{after.toFixed(3)}</td>
      <td style={{ padding: '2px 8px', textAlign: 'right', color }}>{deltaStr}</td>
    </tr>
  )
}
