import type { FocusSettings, GameObject } from '../../types'
import Slider from '../common/Slider'

interface FocusEditorProps {
  fs: FocusSettings
  onChange: (fs: FocusSettings) => void
  npcIds: string[]
  objects: GameObject[]
}

export default function FocusEditor({ fs, onChange, npcIds, objects }: FocusEditorProps) {
  const set = <K extends keyof FocusSettings>(key: K, val: FocusSettings[K]) =>
    onChange({ ...fs, [key]: val })

  return (
    <div style={{ marginTop: 4 }}>
      {/* Event */}
      <div className="section" style={{ margin: '4px 0' }}>
        <div className="section-header" style={{ padding: '2px 8px' }}>
          <input
            type="checkbox"
            checked={fs.hasEvent}
            onChange={(e) => set('hasEvent', e.target.checked)}
          />
          <span className="title" style={{ fontSize: 11 }}>Event</span>
        </div>
        {fs.hasEvent && (
          <div className="section-body" style={{ padding: '4px 8px' }}>
            <input
              type="text"
              value={fs.evDesc}
              onChange={(e) => set('evDesc', e.target.value)}
              placeholder="사건 설명..."
            />
            <Slider label="자기 영향" value={fs.evSelf} onChange={(v) => set('evSelf', v)} />
            <label style={{ display: 'inline', fontSize: 11 }}>
              <input
                type="checkbox"
                checked={fs.hasOther}
                onChange={(e) => set('hasOther', e.target.checked)}
              />{' '}
              타인 영향
            </label>
            {fs.hasOther && (
              <div style={{ marginLeft: 16, marginTop: 4 }}>
                <select
                  value={fs.otherTarget}
                  onChange={(e) => set('otherTarget', e.target.value)}
                  style={{ marginBottom: 4 }}
                >
                  <option value="">대상 선택...</option>
                  {npcIds.map((id) => (
                    <option key={id} value={id}>
                      {id}
                    </option>
                  ))}
                </select>
                <Slider label="타인 영향" value={fs.otherD} onChange={(v) => set('otherD', v)} />
              </div>
            )}
            <label style={{ fontSize: 11 }}>전망</label>
            <select value={fs.prospect} onChange={(e) => set('prospect', e.target.value)}>
              <option value="">없음</option>
              <option value="anticipation">Anticipation (전망)</option>
              <option value="hope_fulfilled">HopeFulfilled</option>
              <option value="hope_unfulfilled">HopeUnfulfilled</option>
              <option value="fear_unrealized">FearUnrealized</option>
              <option value="fear_confirmed">FearConfirmed</option>
            </select>
          </div>
        )}
      </div>
      {/* Action */}
      <div className="section" style={{ margin: '4px 0' }}>
        <div className="section-header" style={{ padding: '2px 8px' }}>
          <input
            type="checkbox"
            checked={fs.hasAction}
            onChange={(e) => set('hasAction', e.target.checked)}
          />
          <span className="title" style={{ fontSize: 11 }}>Action</span>
        </div>
        {fs.hasAction && (
          <div className="section-body" style={{ padding: '4px 8px' }}>
            <input
              type="text"
              value={fs.acDesc}
              onChange={(e) => set('acDesc', e.target.value)}
              placeholder="행동 설명..."
            />
            <label style={{ fontSize: 11 }}>행위자</label>
            <select value={fs.agentId} onChange={(e) => set('agentId', e.target.value)}>
              <option value="">자기 (NPC 본인)</option>
              {npcIds.map((id) => (
                <option key={id} value={id}>
                  {id}
                </option>
              ))}
            </select>
            <Slider label="도덕성" value={fs.pw} onChange={(v) => set('pw', v)} />
          </div>
        )}
      </div>
      {/* Object */}
      <div className="section" style={{ margin: '4px 0' }}>
        <div className="section-header" style={{ padding: '2px 8px' }}>
          <input
            type="checkbox"
            checked={fs.hasObject}
            onChange={(e) => set('hasObject', e.target.checked)}
          />
          <span className="title" style={{ fontSize: 11 }}>Object</span>
        </div>
        {fs.hasObject && (
          <div className="section-body" style={{ padding: '4px 8px' }}>
            <select value={fs.objTarget} onChange={(e) => set('objTarget', e.target.value)}>
              <option value="">대상 선택...</option>
              {objects.map((o) => (
                <option key={o.id} value={o.id}>
                  {o.id} — {o.description?.substring(0, 20)}
                </option>
              ))}
              {npcIds.map((id) => (
                <option key={'npc_' + id} value={id}>
                  {id} (NPC)
                </option>
              ))}
            </select>
            <Slider label="매력도" value={fs.objAp} onChange={(v) => set('objAp', v)} />
          </div>
        )}
      </div>
    </div>
  )
}
