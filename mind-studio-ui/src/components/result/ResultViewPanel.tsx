import { Fragment } from 'react'
import type { Npc, ChatMessage } from '../../types'

interface ResultViewPanelProps {
  npcId: string
  partnerId: string
  npcs: Npc[]
  messages: ChatMessage[]
  selectedMsgIdx: number | null
  onSelectMsg: (idx: number) => void
  onClose: () => void
}

export default function ResultViewPanel({
  npcId, partnerId, npcs, messages,
  selectedMsgIdx, onSelectMsg, onClose,
}: ResultViewPanelProps) {
  const npc = npcs.find((n) => n.id === npcId)
  const partner = npcs.find((n) => n.id === partnerId)

  return (
    <div className="center" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      <div className="chat-header">
        <div>
          <span className="chat-title">📋 {npc?.name || npcId} ↔ {partner?.name || partnerId}</span>
          <span className="chat-mode">결과 보기</span>
        </div>
        <button className="btn small" onClick={onClose}>닫기</button>
      </div>
      <div className="chat-messages">
        {messages.map((msg, i) => {
          if (msg.beat_changed) {
            return (
              <Fragment key={i}>
                <div className="chat-beat-divider">
                  🔄 Beat 전환{msg.new_focus ? `: ${msg.new_focus}` : ''}
                </div>
                <div
                  className={`chat-msg ${msg.role} ${selectedMsgIdx === i ? 'selected' : ''}`}
                  onClick={() => onSelectMsg(i)}
                >
                  <div className="msg-role">
                    {msg.role === 'assistant' ? npc?.name || npcId : msg.role === 'user' ? partner?.name || partnerId : 'System'}
                  </div>
                  {msg.content}
                  {msg.emotions && (
                    <div className="msg-emotions">
                      📊 {Object.entries(msg.emotions).map(([k, v]) => `${k} ${(v as number).toFixed(3)}`).join(' · ')}
                    </div>
                  )}
                </div>
              </Fragment>
            )
          }
          return (
            <div
              key={i}
              className={`chat-msg ${msg.role} ${selectedMsgIdx === i ? 'selected' : ''}`}
              onClick={() => onSelectMsg(i)}
            >
              <div className="msg-role">
                {msg.role === 'assistant' ? npc?.name || npcId : msg.role === 'user' ? partner?.name || partnerId : 'System'}
              </div>
              {msg.content}
              {msg.emotions && (
                <div className="msg-emotions">
                  📊 {Object.entries(msg.emotions).map(([k, v]) => `${k} ${(v as number).toFixed(3)}`).join(' · ')}
                </div>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}
