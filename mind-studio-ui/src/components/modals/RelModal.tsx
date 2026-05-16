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

const emptyRel: Relationship = {
  owner_id: '', target_id: '',
  trust: 0, affinity: 0, respect: 0, wariness: 0,
}

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
        {/* Stage 3 — 4축 ±100 raw. Slider default(±1/0.05)는 PAD/Focus용이라 props 명시.
            wariness 0~100 (음수 의미 없음 — TS 보호). */}
        <Slider label="신뢰" value={data.trust} onChange={(v) => set('trust', v)} min={-100} max={100} step={1} />
        <Slider label="호감" value={data.affinity} onChange={(v) => set('affinity', v)} min={-100} max={100} step={1} />
        <Slider label="존중" value={data.respect} onChange={(v) => set('respect', v)} min={-100} max={100} step={1} />
        <Slider label="경계" value={data.wariness} onChange={(v) => set('wariness', v)} min={0} max={100} step={1} />
        <div className="btn-row" style={{ marginTop: 12 }}>
          <button className="btn primary" onClick={() => { if (!data.owner_id || !data.target_id) return alert('양쪽 ID 필수'); onSave(data) }}>저장</button>
          {rel && <button className="btn danger" onClick={() => onDelete(rel.owner_id, rel.target_id)}>삭제</button>}
          <button className="btn" onClick={onClose}>취소</button>
        </div>
      </div>
    </div>
  )
}
