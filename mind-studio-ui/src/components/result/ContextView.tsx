import type { ChatMessage } from '../../types'

interface ContextViewProps {
  prompt: string
  chatMessages: ChatMessage[] | null
  selectedMsgIdx: number | null
  onRegenerate: () => void
  onCopy: () => void
}

/**
 * 프롬프트 문자열에서 `LocaleMemoryFramer`가 prepend한 "떠오르는 기억" 블록을 분리한다
 * (Step E2 시각화).
 *
 * 블록 포맷 (`locales/{ko,en}.toml` [memory.framing.block]):
 * - ko: `\n# 떠오르는 기억\n` header + entries + `\n` footer
 * - en: `\n# Recollections\n` header + entries + `\n` footer
 *
 * 헤더 매칭 실패 시 전체 프롬프트를 원본 그대로 반환.
 */
function splitMemoryBlock(prompt: string): { memory: string | null; rest: string } {
  const patterns = [/^\s*#\s*떠오르는 기억\s*\n/, /^\s*#\s*Recollections\s*\n/]
  for (const re of patterns) {
    const m = prompt.match(re)
    if (m) {
      const afterHeader = prompt.slice(m.index! + m[0].length)
      // 다음 h1 heading(`\n# `) 또는 두 줄 이상의 빈 줄까지를 블록으로 본다.
      const endMatch = afterHeader.match(/\n(?=#\s)|\n{2,}/)
      if (endMatch) {
        const blockEnd = endMatch.index! + endMatch[0].length
        const memory = m[0] + afterHeader.slice(0, blockEnd)
        const rest = afterHeader.slice(blockEnd)
        return { memory: memory.trim(), rest }
      }
      // 블록 끝을 못 찾으면 전체가 메모리 블록.
      return { memory: prompt.trim(), rest: '' }
    }
  }
  return { memory: null, rest: prompt }
}

export default function ContextView({ prompt, chatMessages, selectedMsgIdx, onRegenerate, onCopy }: ContextViewProps) {
  // selectedMsgIdx가 있으면: 해당 시점의 activePrompt + 그 이전 대화만 표시
  const isHistorical = selectedMsgIdx !== null && selectedMsgIdx !== undefined
  const selectedMsg = isHistorical && chatMessages ? chatMessages[selectedMsgIdx] : null
  const displayPrompt = (isHistorical && selectedMsg?.activePrompt) ? selectedMsg.activePrompt : prompt
  const { memory, rest } = splitMemoryBlock(displayPrompt)

  const llmHistory = chatMessages
    ? (isHistorical
      // 선택된 메시지 이전까지만 (선택된 메시지 자체 포함하지 않음)
      ? chatMessages.slice(0, selectedMsgIdx!).filter((m) => m.role === 'user' || m.role === 'assistant')
      : chatMessages.filter((m) => m.role === 'user' || m.role === 'assistant'))
    : []
  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', gap: 8 }}>
      {/* 시스템 프롬프트 */}
      {isHistorical && (
        <div style={{ fontSize: 10, color: 'var(--warning)', marginBottom: 2, fontWeight: 600 }}>
          📌 선택된 응답 시점의 컨텍스트
        </div>
      )}
      <div style={{ flex: llmHistory.length > 0 ? '0 0 auto' : 1, maxHeight: llmHistory.length > 0 ? '45%' : 'none', overflowY: 'auto' }}>
        <div style={{ fontSize: 11, fontWeight: 600, color: 'var(--accent)', marginBottom: 4 }}>
          System Prompt {isHistorical && '(해당 시점)'}
        </div>
        {memory && (
          <div
            data-testid="memory-block"
            title="DialogueAgent가 주입한 떠오르는 기억 블록 (Step B Framer)"
            style={{
              borderLeft: '3px solid var(--accent2)',
              background: 'var(--bg4)',
              padding: '6px 10px',
              marginBottom: 6,
              borderRadius: 'var(--radius)',
              fontSize: 12,
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-word',
            }}
          >
            <div style={{ fontSize: 10, fontWeight: 600, color: 'var(--accent2)', marginBottom: 4 }}>
              💭 주입된 기억 블록
            </div>
            {memory}
          </div>
        )}
        <div className="prompt-box">{memory ? rest : displayPrompt}</div>
        <div className="btn-row" style={{ marginTop: 6 }}>
          <button
            className="btn small"
            onClick={() => {
              navigator.clipboard.writeText(displayPrompt)
              if (onCopy) onCopy()
            }}
          >
            클립보드 복사
          </button>
          {onRegenerate && !isHistorical && (
            <button
              className="btn small"
              onClick={onRegenerate}
            >
              가이드 재생성
            </button>
          )}
        </div>
      </div>
      {/* LLM 대화 이력 */}
      {llmHistory.length > 0 && (
        <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', borderTop: '1px solid var(--border)', paddingTop: 8 }}>
          <div style={{ fontSize: 11, fontWeight: 600, color: 'var(--accent2)', marginBottom: 4 }}>
            LLM 대화 이력 ({Math.ceil(llmHistory.length / 2)}턴){isHistorical && ' — 이 시점까지'}
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {llmHistory.map((msg, i) => (
              <div
                key={i}
                style={{
                  padding: '6px 8px',
                  borderRadius: 'var(--radius)',
                  background: msg.role === 'user' ? 'var(--bg4)' : 'var(--bg3)',
                  borderLeft: msg.role === 'user'
                    ? '3px solid var(--fg3)'
                    : '3px solid var(--accent)',
                  fontSize: 12,
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-word',
                }}
              >
                <div style={{
                  fontSize: 10,
                  fontWeight: 600,
                  color: msg.role === 'user' ? 'var(--fg3)' : 'var(--accent)',
                  marginBottom: 2,
                }}>
                  {msg.role === 'user' ? '👤 User' : '🤖 NPC'}
                </div>
                {(() => {
                  const trimmed = msg.content.split('\n').filter(l => l.trim() !== '').join('\n')
                  return trimmed.length > 200 ? trimmed.substring(0, 200) + '...' : trimmed
                })()}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
