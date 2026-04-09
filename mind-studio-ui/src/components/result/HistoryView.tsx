import { useState } from 'react'
import type { TurnHistory } from '../../types'

interface HistoryViewProps {
  history: TurnHistory[]
}

export default function HistoryView({ history }: HistoryViewProps) {
  const [expanded, setExpanded] = useState<number | null>(null)
  if (!history.length)
    return <div className="empty">아직 턴 기록이 없습니다</div>
  return (
    <div>
      {history
        .slice()
        .reverse()
        .map((h, i) => (
          <div
            key={i}
            className="history-item"
            onClick={() =>
              setExpanded(expanded === i ? null : i)
            }
          >
            <div className="hlabel">{h.label}</div>
            <div className="haction">{h.action}</div>
            {expanded === i && (
              <div
                className="trace-box"
                style={{
                  marginTop: 6,
                  fontSize: 10,
                  maxHeight: 200,
                  overflow: 'auto',
                }}
              >
                {JSON.stringify(
                  h.response,
                  null,
                  2,
                )}
              </div>
            )}
          </div>
        ))}
    </div>
  )
}
