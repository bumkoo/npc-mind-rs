import type { SceneInfo } from '../../types'

interface ScenePanelProps {
  sceneInfo: SceneInfo | null
}

export default function ScenePanel({ sceneInfo }: ScenePanelProps) {
  if (!sceneInfo) return null
  return (
    <div className="section" style={{ marginBottom: 8 }}>
      <div className="section-header">
        <span
          className="title"
          style={{ color: 'var(--accent2)' }}
        >
          Scene Focus
        </span>
      </div>
      <div
        className="section-body"
        style={{ padding: '6px 10px' }}
      >
        {sceneInfo.focuses?.map((f, i) => {
          const isActive = f.is_active
          const hasScript = f.test_script && f.test_script.length > 0
          const cursor = isActive ? (sceneInfo.script_cursor || 0) : 0
          return (
            <div key={i} style={{ marginBottom: 8 }}>
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 6,
                  fontSize: 11,
                }}
              >
                <span
                  style={{
                    color: isActive
                      ? 'var(--green)'
                      : 'var(--fg3)',
                    fontWeight: isActive ? 700 : 400,
                  }}
                >
                  {isActive ? '\u25CF' : '\u25CB'}
                </span>
                <span
                  style={{
                    color: isActive
                      ? 'var(--fg)'
                      : 'var(--fg3)',
                    fontWeight: isActive ? 600 : 400,
                  }}
                >
                  {f.id}
                </span>
                <span
                  style={{ color: 'var(--fg3)', flex: 1 }}
                >
                  — {f.description}
                </span>
                <span
                  style={{
                    color: 'var(--fg3)',
                    fontSize: 10,
                    fontFamily: 'monospace',
                  }}
                >
                  {f.trigger_display}
                </span>
              </div>
              {hasScript && f.test_script && (
                <div style={{
                  marginTop: 4,
                  marginLeft: 18,
                  padding: '4px 8px',
                  background: isActive ? 'var(--bg2)' : 'transparent',
                  borderRadius: 'var(--radius)',
                  border: isActive ? '1px solid var(--border)' : '1px solid transparent',
                  opacity: isActive ? 1 : 0.6,
                }}>
                  <div style={{ fontSize: 10, color: isActive ? 'var(--accent2)' : 'var(--fg3)', fontWeight: 600, marginBottom: 2 }}>
                    {isActive ? '\uD83D\uDCCB' : '\uD83D\uDCDD'} 테스트 스크립트 ({f.id}) — {isActive ? `${cursor}/${f.test_script.length}` : `${f.test_script.length}턴`}
                  </div>
                  {f.test_script.map((line, idx) => (
                    <div key={idx} style={{
                      fontSize: 10,
                      padding: '2px 4px',
                      color: isActive
                        ? (idx < cursor ? 'var(--fg3)' : idx === cursor ? 'var(--fg)' : 'var(--fg2)')
                        : 'var(--fg3)',
                      fontWeight: isActive && idx === cursor ? 600 : 400,
                      textDecoration: isActive && idx < cursor ? 'line-through' : 'none',
                      opacity: isActive && idx < cursor ? 0.5 : 1,
                    }}>
                      {idx + 1}. {line}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}
