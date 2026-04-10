import { useState, useEffect } from 'react'
import type { Npc } from '../../types'
import Slider from '../common/Slider'

interface NpcModalProps {
  npc: Npc | null
  onSave: (data: Npc) => void
  onDelete: (id: string) => void
  onClose: () => void
}

const emptyNpc: Npc = {
  id: '', name: '', description: '',
  sincerity: 0, fairness: 0, greed_avoidance: 0, modesty: 0,
  fearfulness: 0, anxiety: 0, dependence: 0, sentimentality: 0,
  social_self_esteem: 0, social_boldness: 0, sociability: 0, liveliness: 0,
  forgiveness: 0, gentleness: 0, flexibility: 0, patience: 0,
  organization: 0, diligence: 0, perfectionism: 0, prudence: 0,
  aesthetic_appreciation: 0, inquisitiveness: 0, creativity: 0, unconventionality: 0,
}

const dims = [
  { key: 'H', name: '정직-겸손성', color: 'var(--accent2)', facets: [['sincerity', '진실성'], ['fairness', '공정성'], ['greed_avoidance', '탐욕회피'], ['modesty', '겸손']] },
  { key: 'E', name: '정서성', color: 'var(--warn)', facets: [['fearfulness', '공포성'], ['anxiety', '불안'], ['dependence', '의존성'], ['sentimentality', '감상성']] },
  { key: 'X', name: '외향성', color: 'var(--accent)', facets: [['social_self_esteem', '자존감'], ['social_boldness', '대담성'], ['sociability', '사교성'], ['liveliness', '활발성']] },
  { key: 'A', name: '원만성', color: 'var(--green)', facets: [['forgiveness', '용서'], ['gentleness', '온화함'], ['flexibility', '유연성'], ['patience', '인내심']] },
  { key: 'C', name: '성실성', color: 'var(--purple)', facets: [['organization', '조직성'], ['diligence', '근면성'], ['perfectionism', '완벽주의'], ['prudence', '신중함']] },
  { key: 'O', name: '경험개방성', color: '#f06292', facets: [['aesthetic_appreciation', '미적감상'], ['inquisitiveness', '탐구심'], ['creativity', '창의성'], ['unconventionality', '비관습성']] },
] as const

export default function NpcModal({ npc, onSave, onDelete, onClose }: NpcModalProps) {
  const [data, setData] = useState<Npc>(npc || emptyNpc)

  useEffect(() => {
    setData(npc || emptyNpc)
  }, [npc])

  const set = (k: string, v: unknown) => setData((p) => ({ ...p, [k]: v }))

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>
          {npc ? `NPC 편집: ${npc.name || npc.id}` : '새 NPC 생성'}
          <button className="close-btn" onClick={onClose}>✕</button>
        </h2>
        <div style={{ display: 'flex', gap: 8 }}>
          <div style={{ flex: 1 }}>
            <label>ID</label>
            <input type="text" value={data.id} onChange={(e) => set('id', e.target.value)} placeholder="mu_baek" />
          </div>
          <div style={{ flex: 1 }}>
            <label>이름</label>
            <input type="text" value={data.name} onChange={(e) => set('name', e.target.value)} placeholder="무백" />
          </div>
        </div>
        <label>설명</label>
        <textarea value={data.description} onChange={(e) => set('description', e.target.value)} rows={2} placeholder="캐릭터 설명..." />
        {dims.map((dim) => {
          const avg = dim.facets.reduce((s, f) => s + (data[f[0] as keyof Npc] as number), 0) / 4
          return (
            <div key={dim.key} className="dim-group">
              <div className="dim-title">
                <span style={{ color: dim.color }}>{dim.key}: {dim.name}</span>
                <span style={{ color: dim.color }}>{avg.toFixed(2)}</span>
              </div>
              {dim.facets.map(([k, label]) => (
                <Slider key={k} label={label} value={data[k as keyof Npc] as number} onChange={(v) => set(k, v)} />
              ))}
            </div>
          )
        })}
        <div className="btn-row" style={{ marginTop: 12 }}>
          <button className="btn primary" onClick={() => { if (!data.id) return alert('ID 필수'); onSave(data) }}>저장</button>
          {npc && <button className="btn danger" onClick={() => onDelete(data.id)}>삭제</button>}
          <button className="btn" onClick={onClose}>취소</button>
        </div>
      </div>
    </div>
  )
}
