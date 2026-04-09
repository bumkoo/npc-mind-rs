import { useState, useEffect } from 'react'
import type { Relationship } from '../../types'
import Slider from '../common/Slider'

interface RelModalProps {
  rel: Relationship | null
  npcIds: string[]
  onSave: (data: Relationship) => void
  onDelete: (ownerId: string, targetId: string) => void
  onClose: () => void
}

const emptyRel: Relationship = { owner_id: '', target_id: '', closeness: 0, trust: 0, power: 0 }

export default function RelModal({ rel, npcIds, onSave, onDelete, onClose }: RelModalProps) {
  const [data, setData] = useState<Relationship>(rel || emptyRel)

  useEffect(() => {
    setData(rel || emptyRel)
  }, [rel])

  const set = (k: keyof Relationship, v: string | number) => setData((p) => ({ ...p, [k]: v }))
  const allIds = ['player', ...npcIds]

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" style={{ width: 400 }} onClick={(e) => e.stopPropagation()}>
        <h2>
          {rel ? '관계 편집' : '새 관계'}
          <button className="close-btn" onClick={onClose}>✕</button>
        </h2>
        <div style={{ display: 'flex', gap: 8 }}>
          <div style={{ flex: 1 }}>
            <label>소유자</label>
            <select value={data.owner_id} onChange={(e) => set('owner_id', e.target.value)}>
              <option value="">선택...</option>
              {allIds.map((id) => <option key={id} value={id}>{id}</option>)}
            </select>
          </div>
          <div style={{ flex: 1 }}>
            <label>대상</label>
            <select value={data.target_id} onChange={(e) => set('target_id', e.target.value)}>
              <option value="">선택...</option>
              {allIds.map((id) => <option key={id} value={id}>{id}</option>)}
            </select>
          </div>
        </div>
        <Slider label="친밀도" value={data.closeness} onChange={(v) => set('closeness', v)} />
        <Slider label="신뢰도" value={data.trust} onChange={(v) => set('trust', v)} />
        <Slider label="상하" value={data.power} onChange={(v) => set('power', v)} />
        <div className="btn-row" style={{ marginTop: 12 }}>
          <button className="btn primary" onClick={() => { if (!data.owner_id || !data.target_id) return alert('양쪽 ID 필수'); onSave(data) }}>저장</button>
          {rel && <button className="btn danger" onClick={() => onDelete(rel.owner_id, rel.target_id)}>삭제</button>}
          <button className="btn" onClick={onClose}>취소</button>
        </div>
      </div>
    </div>
  )
}
