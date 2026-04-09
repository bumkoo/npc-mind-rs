import { useState, useEffect } from 'react'
import type { GameObject } from '../../types'

interface ObjModalProps {
  obj: GameObject | null
  onSave: (data: GameObject) => void
  onDelete: (id: string) => void
  onClose: () => void
}

const emptyObj: GameObject = { id: '', description: '', category: null }

export default function ObjModal({ obj, onSave, onDelete, onClose }: ObjModalProps) {
  const [data, setData] = useState<GameObject>(obj || emptyObj)

  useEffect(() => {
    setData(obj || emptyObj)
  }, [obj])

  const set = (k: keyof GameObject, v: string | null) => setData((p) => ({ ...p, [k]: v }))

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" style={{ width: 400 }} onClick={(e) => e.stopPropagation()}>
        <h2>
          {obj ? '오브젝트 편집' : '새 오브젝트'}
          <button className="close-btn" onClick={onClose}>✕</button>
        </h2>
        <label>ID</label>
        <input type="text" value={data.id} onChange={(e) => set('id', e.target.value)} />
        <label>설명</label>
        <textarea value={data.description} onChange={(e) => set('description', e.target.value)} rows={2} />
        <label>카테고리</label>
        <input type="text" value={data.category || ''} onChange={(e) => set('category', e.target.value || null)} placeholder="사물/장소/NPC특성..." />
        <div className="btn-row" style={{ marginTop: 12 }}>
          <button className="btn primary" onClick={() => { if (!data.id) return alert('ID 필수'); onSave(data) }}>저장</button>
          {obj && <button className="btn danger" onClick={() => onDelete(data.id)}>삭제</button>}
          <button className="btn" onClick={onClose}>취소</button>
        </div>
      </div>
    </div>
  )
}
