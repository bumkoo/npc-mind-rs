import { useState, useEffect, useCallback, useRef } from 'react'
import type { Npc, GameObject, FocusSettings, SceneInfo, SceneFocus } from '../../types'
import Slider from '../common/Slider'
import { makeFocusSettings } from '../../utils/makeFocusSettings'
import { api } from '../../api/client'
import FocusEditor from './FocusEditor'

interface SituationData {
  description: string
  event: {
    description: string
    desirability_for_self: number
    other: { target_id: string; desirability: number } | null
    prospect: string | null
  } | null
  action: {
    description: string
    agent_id: string | null
    praiseworthiness: number
  } | null
  object: {
    target_id: string
    appealingness: number
  } | null
}

interface SavedSituation {
  desc?: string
  npcId?: string
  partnerId?: string
  significance?: number
  hasEvent?: boolean
  evDesc?: string
  evSelf?: number
  hasOther?: boolean
  otherTarget?: string
  otherD?: number
  prospect?: string
  hasAction?: boolean
  acDesc?: string
  agentId?: string
  pw?: number
  hasObject?: boolean
  objTarget?: string
  objAp?: number
  [key: string]: unknown
}

interface SituationPanelProps {
  npcs: Npc[]
  objects: GameObject[]
  npcId: string
  setNpcId: (id: string) => void
  partnerId: string
  setPartnerId: (id: string) => void
  onAppraise: ((situation: SituationData) => void) | null
  onStartChat: ((situation: SituationData) => void) | null
  startChatLabel?: string | null
  loading: boolean
  savedSituation: SavedSituation | null
  sceneInfo: SceneInfo | null
  toast: (msg: string, type?: 'info' | 'success' | 'error') => void
  disabled: boolean
}

export default function SituationPanel({
  npcs,
  objects,
  npcId,
  setNpcId,
  partnerId,
  setPartnerId,
  onAppraise,
  onStartChat,
  startChatLabel,
  loading,
  savedSituation,
  sceneInfo,
  toast,
  disabled,
}: SituationPanelProps) {
  const npcIds = ['player', ...npcs.map((n) => n.id)]
  const [desc, setDesc] = useState('')
  const [significance, setSignificance] = useState(0)

  // Focus별 설정 (Scene 모드)
  const [focusSettings, setFocusSettings] = useState<Record<string, FocusSettings>>({})
  // 펼침 상태: { focusId: true/false }
  const [expanded, setExpanded] = useState<Record<string, boolean>>({})

  // 단일 모드 (Scene 없을 때) — 기존 호환
  const [singleFs, setSingleFs] = useState<FocusSettings>(makeFocusSettings())

  // Scene focuses가 바뀌면 focusSettings 초기화
  const sceneKeyRef = useRef<string | null>(null)
  useEffect(() => {
    if (!sceneInfo?.focuses) return
    const key = sceneInfo.focuses.map((f) => f.id).join(',')
    if (key === sceneKeyRef.current) return
    sceneKeyRef.current = key
    const settings: Record<string, FocusSettings> = {}
    const exp: Record<string, boolean> = {}
    sceneInfo.focuses.forEach((f) => {
      settings[f.id] = makeFocusSettings(f)
      exp[f.id] = f.is_active // 활성 Focus만 펼침
    })
    setFocusSettings(settings)
    setExpanded(exp)
  }, [sceneInfo])

  // --- 시나리오 로드 시 상황설정 복원 ---
  const restoreKeyRef = useRef<SavedSituation | null>(null)
  const skipSituationSaveRef = useRef(false)
  useEffect(() => {
    if (!savedSituation || savedSituation === restoreKeyRef.current) return
    restoreKeyRef.current = savedSituation
    const s = savedSituation
    if (s.desc != null) setDesc(s.desc)
    if (s.npcId != null) setNpcId(s.npcId)
    if (s.partnerId != null) setPartnerId(s.partnerId)
    if (s.significance != null) setSignificance(s.significance)
    // 단일 모드 복원
    if (!sceneInfo) {
      setSingleFs({
        hasEvent: s.hasEvent ?? true,
        evDesc: s.evDesc || '',
        evSelf: s.evSelf || 0,
        hasOther: s.hasOther || false,
        otherTarget: s.otherTarget || '',
        otherD: s.otherD || 0,
        prospect: s.prospect || '',
        hasAction: s.hasAction || false,
        acDesc: s.acDesc || '',
        agentId: s.agentId || '',
        pw: s.pw || 0,
        hasObject: s.hasObject || false,
        objTarget: s.objTarget || '',
        objAp: s.objAp || 0,
      })
    }
    // 복원으로 인한 state 변경이 auto-save를 트리거하지 않도록 스킵 플래그 설정
    skipSituationSaveRef.current = true
  }, [savedSituation])

  // --- 상황설정 변경 시 자동 저장 (debounced) ---
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const allDeps = JSON.stringify({ desc, significance, focusSettings, singleFs, npcId, partnerId })
  useEffect(() => {
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current)
    // 로드 직후 첫 자동저장은 건너뜀 (scenario_modified 오염 방지)
    if (skipSituationSaveRef.current) {
      skipSituationSaveRef.current = false
      return
    }
    saveTimerRef.current = setTimeout(() => {
      const activeFs = sceneInfo ? focusSettings : { _single: singleFs }
      const data = {
        desc,
        npcId,
        partnerId,
        significance,
        focusSettings: activeFs,
        // 하위 호환: 단일 모드일 때 기존 필드도 저장
        ...(!sceneInfo ? singleFs : {}),
      }
      api.put('/api/situation', data).catch(() => {})
    }, 500)
    return () => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current)
    }
  }, [allDeps])

  const buildSituationFromFs = useCallback(
    (fs: FocusSettings): SituationData => ({
      description: desc,
      event: fs.hasEvent
        ? {
            description: fs.evDesc,
            desirability_for_self: fs.evSelf,
            other: fs.hasOther ? { target_id: fs.otherTarget, desirability: fs.otherD } : null,
            prospect: fs.prospect || null,
          }
        : null,
      action: fs.hasAction
        ? {
            description: fs.acDesc,
            agent_id: fs.agentId || null,
            praiseworthiness: fs.pw,
          }
        : null,
      object: fs.hasObject ? { target_id: fs.objTarget, appealingness: fs.objAp } : null,
    }),
    [desc],
  )

  const buildSituation = useCallback((): SituationData => {
    if (sceneInfo?.focuses) {
      const active = sceneInfo.focuses.find((f) => f.is_active)
      if (active && focusSettings[active.id]) {
        return buildSituationFromFs(focusSettings[active.id])
      }
    }
    return buildSituationFromFs(singleFs)
  }, [sceneInfo, focusSettings, singleFs, buildSituationFromFs])

  const toggleExpand = (id: string) => setExpanded((prev) => ({ ...prev, [id]: !prev[id] }))
  const updateFocusSetting = (id: string, newFs: FocusSettings) =>
    setFocusSettings((prev) => ({ ...prev, [id]: newFs }))

  const focuses: SceneFocus[] = sceneInfo?.focuses || []

  return (
    <div className="center" style={disabled ? { opacity: 0.6, pointerEvents: 'none' } : {}}>
      <h2>상황 설정</h2>
      {disabled && (
        <div
          style={{
            padding: '8px 12px',
            background: '#fff3cd',
            color: '#856404',
            fontSize: 12,
            borderRadius: 4,
            marginBottom: 8,
            textAlign: 'center',
            pointerEvents: 'auto',
          }}
        >
          대화 종료됨 — 결과를 저장하거나 시나리오를 다시 로드하세요
        </div>
      )}
      {/* NPC pair */}
      <div className="pair-select">
        <div>
          <label>NPC (주체)</label>
          <select value={npcId} onChange={(e) => setNpcId(e.target.value)}>
            <option value="">선택...</option>
            {npcs.map((n) => (
              <option key={n.id} value={n.id}>
                {n.name || n.id}
              </option>
            ))}
          </select>
        </div>
        <div
          style={{
            display: 'flex',
            alignItems: 'flex-end',
            padding: '0 0 2px',
            color: 'var(--fg3)',
          }}
        >
          ↔
        </div>
        <div>
          <label>대화 상대</label>
          <select value={partnerId} onChange={(e) => setPartnerId(e.target.value)}>
            <option value="">선택...</option>
            {npcIds.map((id) => (
              <option key={id} value={id}>
                {id}
              </option>
            ))}
          </select>
        </div>
      </div>
      <div className="section">
        <div className="section-header">
          <span className="title">상황 설명</span>
        </div>
        <div className="section-body">
          <textarea
            value={desc}
            onChange={(e) => setDesc(e.target.value)}
            rows={2}
            placeholder="전체 상황 맥락..."
          />
          <Slider
            label="중요도"
            value={significance}
            onChange={setSignificance}
            min={0}
            max={1}
            step={0.1}
          />
        </div>
      </div>

      {/* Scene 모드: Focus별 접기/펼치기 */}
      {focuses.length > 0 ? (
        focuses.map((f, fi) => (
          <div key={f.id}>
            <div className="section" style={{ marginBottom: 2 }}>
              <div
                className="section-header"
                style={{ cursor: 'pointer', userSelect: 'none' }}
                onClick={() => toggleExpand(f.id)}
              >
                <span
                  style={{
                    fontSize: 12,
                    marginRight: 6,
                    color: f.is_active ? 'var(--accent)' : 'var(--fg3)',
                  }}
                >
                  {f.is_active ? '\u25CF' : '\u25CB'}
                </span>
                <span className="title" style={{ flex: 1 }}>
                  {f.id}
                </span>
                <span style={{ fontSize: 10, color: 'var(--fg3)' }}>
                  {expanded[f.id] ? '\u25BC' : '\u25B6'}
                </span>
              </div>
              <div style={{ padding: '0 10px 4px', fontSize: 11, color: 'var(--fg2)' }}>
                {f.description}
              </div>
              {expanded[f.id] && focusSettings[f.id] && (
                <FocusEditor
                  fs={focusSettings[f.id]}
                  onChange={(newFs) => updateFocusSetting(f.id, newFs)}
                  npcIds={npcIds}
                  objects={objects}
                />
              )}
            </div>
            {/* 전환 조건 (다음 Focus가 있고, 현재가 마지막이 아닐 때) */}
            {fi < focuses.length - 1 &&
              focuses[fi + 1].trigger_display &&
              focuses[fi + 1].trigger_display !== 'initial' && (
                <div
                  style={{
                    textAlign: 'center',
                    fontSize: 10,
                    color: 'var(--warn)',
                    padding: '3px 0',
                    margin: '0 8px',
                    borderLeft: '1px dashed var(--border)',
                    borderRight: '1px dashed var(--border)',
                  }}
                >
                  전환 조건: {focuses[fi + 1].trigger_display}
                </div>
              )}
          </div>
        ))
      ) : (
        /* 단일 모드: Scene 없을 때 기존 UI */
        <FocusEditor fs={singleFs} onChange={setSingleFs} npcIds={npcIds} objects={objects} />
      )}

      {/* Action buttons */}
      {onStartChat && (
        <button
          className="btn btn-full"
          style={{ marginTop: 4, borderColor: 'var(--purple)', color: 'var(--purple)' }}
          disabled={loading}
          onClick={() => onStartChat(buildSituation())}
        >
          {startChatLabel || '\uD83D\uDCAC 대화 시작'}
        </button>
      )}
    </div>
  )
}
