import type { AppraiseResult } from '../../types'
import { emotionColor } from '../../utils/emotionColor'

interface RelMetrics {
  closeness: number
  trust: number
  power: number
}

interface EmotionViewResult extends AppraiseResult {
  before?: RelMetrics
  after?: RelMetrics
}

interface EmotionViewProps {
  result: EmotionViewResult
}

export default function EmotionView({ result }: EmotionViewProps) {
  const hasEmotions = result.emotions && result.emotions.length > 0
  const relMetrics = [
    { key: 'closeness' as const, label: '친밀도', color: 'var(--accent2)' },
    { key: 'trust' as const, label: '신뢰도', color: 'var(--accent)' },
    { key: 'power' as const, label: '상하관계', color: 'var(--purple)' },
  ]
  const toPercent = (v: number) => ((v + 1) / 2) * 100
  return (
    <div>
      {result.beat_changed && (
        <div
          style={{
            background: 'var(--accent2)',
            color: 'var(--bg)',
            padding: '6px 12px',
            borderRadius: 'var(--radius)',
            marginBottom: 8,
            fontSize: 12,
            fontWeight: 600,
          }}
        >
          ★ Beat 전환! → {result.active_focus_id || '?'}
        </div>
      )}
      {result.dominant && (
        <div className="dominant-card">
          <div className="d-type">
            {result.dominant.emotion_type}
          </div>
          <div className="d-intensity">
            강도: {result.dominant.intensity.toFixed(3)}
          </div>
          {result.dominant.context && (
            <div className="d-ctx">
              — {result.dominant.context}
            </div>
          )}
        </div>
      )}
      {hasEmotions && (
        <>
          <div className="mood-bar">
            전반적 분위기:{' '}
            <span
              style={{
                color:
                  result.mood != null && result.mood >= 0
                    ? 'var(--green)'
                    : 'var(--err)',
                fontWeight: 600,
              }}
            >
              {result.mood != null ? result.mood.toFixed(3) : '—'}
            </span>
          </div>
          {(result.emotions || []).map((e, i) => (
            <div key={i} className="emotion-row">
              <span className="etype">{e.emotion_type}</span>
              <div className="bar-bg">
                <div
                  className="bar-fill"
                  style={{
                    width: `${Math.min(e.intensity * 100, 100)}%`,
                    background: emotionColor(e.emotion_type),
                  }}
                />
              </div>
              <span className="intensity">
                {e.intensity.toFixed(3)}
              </span>
              {e.context && (
                <span className="ctx" title={e.context}>
                  — {e.context}
                </span>
              )}
            </div>
          ))}
        </>
      )}
      {/* 대화 종료 — 관계 변화 */}
      {result.afterDialogue && result.before && result.after && (
        <div style={{ marginTop: hasEmotions ? 16 : 0 }}>
          <div
            className="dominant-card"
            style={{ borderColor: 'var(--accent2)' }}
          >
            <div
              className="d-type"
              style={{ color: 'var(--accent2)' }}
            >
              대화 종료 — 관계 변화
            </div>
            <div className="d-ctx">
              {result.npc_id} → {result.partner_id}
            </div>
          </div>
          {relMetrics.map((m) => {
            const before = result.before![m.key]
            const after = result.after![m.key]
            const delta = after - before
            return (
              <div key={m.key} className="rel-change">
                <div className="rc-label">
                  <span>{m.label}</span>
                  <span style={{ fontFamily: 'monospace' }}>
                    {before.toFixed(3)} → {after.toFixed(3)}
                  </span>
                </div>
                <div className="rc-bar">
                  <div
                    className="rc-fill rc-before"
                    style={{ width: `${toPercent(before)}%` }}
                  />
                  <div
                    className="rc-fill rc-after"
                    style={{
                      width: `${toPercent(after)}%`,
                      background: m.color,
                    }}
                  />
                </div>
                <div
                  className="rc-delta"
                  style={{
                    color:
                      delta > 0
                        ? 'var(--green)'
                        : delta < 0
                          ? 'var(--err)'
                          : 'var(--fg3)',
                  }}
                >
                  {delta > 0 ? '+' : ''}{delta.toFixed(3)}
                </div>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}
