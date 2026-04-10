import type { TraceEntry } from '../../types'

interface TraceViewProps {
  traceHistory: TraceEntry[]
}

export default function TraceView({ traceHistory }: TraceViewProps) {
  if (!traceHistory || !Array.isArray(traceHistory) || traceHistory.length === 0)
    return <div className="trace-box">(trace 없음)</div>

  // 문자열 배열인 경우 (단일 턴 표시)
  if (typeof traceHistory[0] === 'string') {
    return <div className="trace-box">{(traceHistory as string[]).join('\n')}</div>
  }

  // 객체 배열인 경우 (히스토리 전체 표시 - 하위 호환용)
  const entries = traceHistory as { label: string; trace: string[] }[]
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 6,
      }}
    >
      {[...entries].reverse().map((entry, i) => (
        <div key={i}>
          <div
            style={{
              fontSize: 10,
              color: 'var(--fg2)',
              marginBottom: 2,
            }}
          >
            #{entries.length - i} {entry.label || 'Appraisal'}
          </div>
          <div className="trace-box">
            {(entry.trace || []).join('\n') || '(내용 없음)'}
          </div>
        </div>
      ))}
    </div>
  )
}
