import { useState } from 'react'
import type { Npc, Relationship, GameObject } from '../../types'

interface SidebarProps {
  npcs: Npc[]
  rels: Relationship[]
  objects: GameObject[]
  selectedNpcId: string
  onSelectNpc: (id: string) => void
  onEditNpc: (npc: Npc) => void
  onEditRel: (rel: Relationship) => void
  onEditObj: (obj: GameObject) => void
  onNewNpc: () => void
  onNewRel: () => void
  onNewObj: () => void
  disabled: boolean
  disabledMsg?: string
}

export default function Sidebar({
  npcs, rels, objects, selectedNpcId,
  onSelectNpc, onEditNpc, onEditRel, onEditObj,
  onNewNpc, onNewRel, onNewObj,
  disabled, disabledMsg,
}: SidebarProps) {
  const [openSections, setOpenSections] = useState({ npc: true, rel: true, obj: true })
  const toggle = (k: 'npc' | 'rel' | 'obj') =>
    setOpenSections((p) => ({ ...p, [k]: !p[k] }))

  return (
    <div className="sidebar" style={disabled ? { opacity: 0.5, pointerEvents: 'none' } : {}}>
      {disabled && (
        <div style={{
          padding: '8px 12px', background: '#fff3cd', color: '#856404',
          fontSize: 12, borderRadius: 4, margin: '0 8px 8px',
          textAlign: 'center', pointerEvents: 'auto',
        }}>
          {disabledMsg || '수정할 수 없습니다'}
        </div>
      )}
      {/* NPC */}
      <div className="sidebar-section">
        <div className="sidebar-header" onClick={() => toggle('npc')}>
          <span>{openSections.npc ? '▾' : '▸'} NPC</span>
          <span className="count">{npcs.length}</span>
        </div>
        {openSections.npc && (
          <div className="sidebar-body">
            {npcs.map((n) => (
              <div key={n.id} className={`item-card ${selectedNpcId === n.id ? 'selected' : ''}`} onClick={() => onSelectNpc(n.id)}>
                <div>
                  <div className="name">{n.name || n.id}</div>
                  <div className="sub">{n.id}</div>
                </div>
                <span className="edit-btn" onClick={(e) => { e.stopPropagation(); onEditNpc(n) }}>편집</span>
              </div>
            ))}
            <button className="add-btn" onClick={onNewNpc}>+ NPC 추가</button>
          </div>
        )}
      </div>
      {/* Relationships */}
      <div className="sidebar-section">
        <div className="sidebar-header" onClick={() => toggle('rel')}>
          <span>{openSections.rel ? '▾' : '▸'} 관계</span>
          <span className="count">{rels.length}</span>
        </div>
        {openSections.rel && (
          <div className="sidebar-body">
            {rels.map((r) => (
              <div key={r.owner_id + ':' + r.target_id} className="item-card" onClick={() => onEditRel(r)}>
                <div>
                  <div className="name">{r.owner_id} ↔ {r.target_id}</div>
                  <div className="sub">친:{r.closeness.toFixed(1)} 신:{r.trust.toFixed(1)} 상:{r.power.toFixed(1)}</div>
                </div>
              </div>
            ))}
            <button className="add-btn" onClick={onNewRel}>+ 관계 추가</button>
          </div>
        )}
      </div>
      {/* Objects */}
      <div className="sidebar-section">
        <div className="sidebar-header" onClick={() => toggle('obj')}>
          <span>{openSections.obj ? '▾' : '▸'} 오브젝트</span>
          <span className="count">{objects.length}</span>
        </div>
        {openSections.obj && (
          <div className="sidebar-body">
            {objects.map((o) => (
              <div key={o.id} className="item-card" onClick={() => onEditObj(o)}>
                <div>
                  <div className="name">{o.id}</div>
                  <div className="sub">{o.description?.substring(0, 30)}</div>
                </div>
              </div>
            ))}
            <button className="add-btn" onClick={onNewObj}>+ 오브젝트 추가</button>
          </div>
        )}
      </div>
    </div>
  )
}
