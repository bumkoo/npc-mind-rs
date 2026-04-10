import { useEffect, useCallback } from 'react'
import { useEntityStore } from './stores/useEntityStore'
import { useUIStore } from './stores/useUIStore'
import { useResultStore } from './stores/useResultStore'
import { useChatStore } from './stores/useChatStore'
import { useSceneStore } from './stores/useSceneStore'
import { useToast } from './hooks/useToast'
import { useRefresh } from './hooks/useRefresh'
import { useChatPolling } from './hooks/useChatPolling'
import {
  handleAppraise, handleStimulus, handleGuide, handleAfterDialogue,
  handleStartChat, handleChatSend, handleEndChat,
  saveScenario, saveState,
} from './handlers/appHandlers'
import { loadScenario, loadResult, updateTestReport } from './handlers/loadHandlers'
import Sidebar from './components/sidebar/Sidebar'
import SituationPanel from './components/situation/SituationPanel'
import ChatPanel from './components/chat/ChatPanel'
import ResultPanel from './components/result/ResultPanel'
import ResultViewPanel from './components/result/ResultViewPanel'
import NpcModal from './components/modals/NpcModal'
import RelModal from './components/modals/RelModal'
import ObjModal from './components/modals/ObjModal'
import ToastContainer from './components/common/ToastContainer'
import type { Npc, Relationship, GameObject, ChatMessage, TraceEntry } from './types'

export default function App() {
  // --- Store: data selectors ---
  const npcs = useEntityStore((s) => s.npcs)
  const rels = useEntityStore((s) => s.rels)
  const objects = useEntityStore((s) => s.objects)
  const scenarios = useEntityStore((s) => s.scenarios)
  const history = useEntityStore((s) => s.history)

  const npcId = useUIStore((s) => s.npcId)
  const partnerId = useUIStore((s) => s.partnerId)
  const modal = useUIStore((s) => s.modal)
  const loading = useUIStore((s) => s.loading)
  const connected = useUIStore((s) => s.connected)
  const resultViewMode = useUIStore((s) => s.resultViewMode)
  const resultViewActive = useUIStore((s) => s.resultViewActive)
  const resultMessages = useUIStore((s) => s.resultMessages)
  const resultSelectedIdx = useUIStore((s) => s.resultSelectedIdx)

  const result = useResultStore((s) => s.result)
  const traceHistory = useResultStore((s) => s.traceHistory)
  const resultTab = useResultStore((s) => s.resultTab)
  const testReport = useResultStore((s) => s.testReport)
  const stimulusUtterance = useResultStore((s) => s.stimulusUtterance)
  const llmModelInfo = useResultStore((s) => s.llmModelInfo)

  const chatMode = useChatStore((s) => s.chatMode)
  const chatSessionId = useChatStore((s) => s.chatSessionId)
  const chatMessages = useChatStore((s) => s.chatMessages)
  const chatLoading = useChatStore((s) => s.chatLoading)
  const chatScenarioTurns = useChatStore((s) => s.chatScenarioTurns)
  const chatScenarioIdx = useChatStore((s) => s.chatScenarioIdx)
  const chatEnded = useChatStore((s) => s.chatEnded)
  const selectedMsgIdx = useChatStore((s) => s.selectedMsgIdx)

  const scenarioMeta = useSceneStore((s) => s.scenarioMeta)
  const savedSituation = useSceneStore((s) => s.savedSituation)
  const sceneInfo = useSceneStore((s) => s.sceneInfo)

  // --- Store: action selectors (stable refs from Zustand) ---
  const saveNpc = useEntityStore((s) => s.saveNpc)
  const deleteNpc = useEntityStore((s) => s.deleteNpc)
  const saveRel = useEntityStore((s) => s.saveRel)
  const deleteRel = useEntityStore((s) => s.deleteRel)
  const saveObj = useEntityStore((s) => s.saveObj)
  const deleteObj = useEntityStore((s) => s.deleteObj)

  const setNpcId = useUIStore((s) => s.setNpcId)
  const setPartnerId = useUIStore((s) => s.setPartnerId)
  const openModal = useUIStore((s) => s.openModal)
  const closeModal = useUIStore((s) => s.closeModal)
  const setLoading = useUIStore((s) => s.setLoading)
  const setResultView = useUIStore((s) => s.setResultView)
  const setResultViewActive = useUIStore((s) => s.setResultViewActive)
  const setResultSelectedIdx = useUIStore((s) => s.setResultSelectedIdx)

  const setResult = useResultStore((s) => s.setResult)
  const updateResult = useResultStore((s) => s.updateResult)
  const setTraceHistory = useResultStore((s) => s.setTraceHistory)
  const appendTrace = useResultStore((s) => s.appendTrace)
  const setResultTab = useResultStore((s) => s.setResultTab)
  const setTestReport = useResultStore((s) => s.setTestReport)
  const setStimulusUtterance = useResultStore((s) => s.setStimulusUtterance)
  const setLlmModelInfo = useResultStore((s) => s.setLlmModelInfo)

  const setChatMode = useChatStore((s) => s.setChatMode)
  const setChatSessionId = useChatStore((s) => s.setChatSessionId)
  const setChatMessages = useChatStore((s) => s.setChatMessages)
  const updateChatMessages = useChatStore((s) => s.updateChatMessages)
  const setChatLoading = useChatStore((s) => s.setChatLoading)
  const setChatScenarioTurns = useChatStore((s) => s.setChatScenarioTurns)
  const setChatScenarioIdx = useChatStore((s) => s.setChatScenarioIdx)
  const advanceScenarioIdx = useChatStore((s) => s.advanceScenarioIdx)
  const setChatEnded = useChatStore((s) => s.setChatEnded)
  const setSelectedMsgIdx = useChatStore((s) => s.setSelectedMsgIdx)

  const setSavedSituation = useSceneStore((s) => s.setSavedSituation)
  const updateSceneInfo = useSceneStore((s) => s.updateSceneInfo)

  const { toasts, toast } = useToast()
  const refresh = useRefresh()
  useChatPolling()

  // --- Initial load ---
  useEffect(() => { refresh() }, [refresh])

  // --- CRUD guards ---
  const guardChat = useCallback(() => {
    if (chatMode) { toast('대화 중에는 수정할 수 없습니다', 'error'); return true }
    if (chatEnded) { toast('결과를 저장하거나 시나리오를 다시 로드하세요', 'error'); return true }
    return false
  }, [chatMode, chatEnded, toast])

  const onSaveNpc = useCallback(async (data: Npc) => {
    if (guardChat()) return
    await saveNpc(data); closeModal(); refresh()
  }, [guardChat, saveNpc, closeModal, refresh])

  const onDeleteNpc = useCallback(async (id: string) => {
    if (guardChat()) return
    await deleteNpc(id); closeModal()
    if (npcId === id) setNpcId('')
    refresh()
  }, [guardChat, deleteNpc, closeModal, npcId, setNpcId, refresh])

  const onSaveRel = useCallback(async (data: Relationship) => {
    if (guardChat()) return
    await saveRel(data); closeModal(); refresh()
  }, [guardChat, saveRel, closeModal, refresh])

  const onDeleteRel = useCallback(async (oid: string, tid: string) => {
    if (guardChat()) return
    await deleteRel(oid, tid); closeModal(); refresh()
  }, [guardChat, deleteRel, closeModal, refresh])

  const onSaveObj = useCallback(async (data: GameObject) => {
    if (guardChat()) return
    await saveObj(data); closeModal(); refresh()
  }, [guardChat, saveObj, closeModal, refresh])

  const onDeleteObj = useCallback(async (id: string) => {
    if (guardChat()) return
    await deleteObj(id); closeModal(); refresh()
  }, [guardChat, deleteObj, closeModal, refresh])

  // --- Message selection (batched updates) ---
  const selectMessage = useCallback((idx: number, msgs: ChatMessage[], currentSelectedIdx: number | null, setIdx: (v: number | null) => void) => {
    if (currentSelectedIdx === idx) {
      setIdx(null)
      const lastSnap = [...msgs].reverse().find((m) => m.snapshot)
      if (lastSnap) setResult(lastSnap.snapshot!)
    } else {
      setIdx(idx)
      const msg = msgs[idx]
      // Batch: trace + result + utterance
      if (msg) setTraceHistory((msg as ChatMessage & { trace?: TraceEntry[] }).trace || [])
      if (msg?.snapshot) setResult(msg.snapshot)
      else if (msg?.role === 'user') {
        const next = msgs[idx + 1]
        if (next?.snapshot) setResult(next.snapshot)
      }
      if (msg) {
        if (msg.role === 'user' && msg.content) setStimulusUtterance(msg.content)
        else if (msg.role === 'assistant') {
          for (let j = idx - 1; j >= 0; j--) {
            if (msgs[j]?.role === 'user' && msgs[j]?.content) { setStimulusUtterance(msgs[j].content); break }
          }
        }
      }
    }
  }, [setResult, setTraceHistory, setStimulusUtterance])

  const handleSelectMsg = useCallback((idx: number) => {
    selectMessage(idx, chatMessages, selectedMsgIdx, setSelectedMsgIdx)
  }, [chatMessages, selectedMsgIdx, setSelectedMsgIdx, selectMessage])

  const handleResultSelectMsg = useCallback((idx: number) => {
    selectMessage(idx, resultMessages, resultSelectedIdx, setResultSelectedIdx)
  }, [resultMessages, resultSelectedIdx, setResultSelectedIdx, selectMessage])

  // --- Scenario load/save ---
  const onLoadScenario = useCallback((path: string) => {
    loadScenario(path, toast, refresh, setChatEnded, setResultView, setSavedSituation, setResult, setTraceHistory)
  }, [toast, refresh, setChatEnded, setResultView, setSavedSituation, setResult, setTraceHistory])

  const onLoadResult = useCallback((path: string) => {
    loadResult(path, toast, refresh, setChatEnded, setResultView, setSavedSituation, setResult, setTraceHistory, setLlmModelInfo, setResultTab)
  }, [toast, refresh, setChatEnded, setResultView, setSavedSituation, setResult, setTraceHistory, setLlmModelInfo, setResultTab])

  const onSaveState = useCallback(() => saveState(chatMode, chatEnded, toast), [chatMode, chatEnded, toast])

  const onUpdateReport = useCallback((content: string) => {
    updateTestReport(content, setTestReport)
  }, [setTestReport])

  // --- Render ---
  return (
    <div className="app">
      {/* Header */}
      <div className="header">
        <h1>Mind Studio</h1>
        <span className={`status ${connected ? 'ok' : ''}`}>
          {connected ? `NPC ${npcs.length} · 관계 ${rels.length} · 오브젝트 ${objects.length}` : '연결 실패'}
        </span>
        {scenarioMeta && (
          <span
            style={{ fontSize: 11, color: 'var(--accent2)', padding: '2px 8px', background: 'var(--bg3)', borderRadius: 'var(--radius)' }}
            title={[scenarioMeta.description, ...(scenarioMeta.notes || [])].filter(Boolean).join('\n')}
          >
            {scenarioMeta.name}
          </span>
        )}
        <div className="actions">
          <select
            style={{ background: 'var(--bg3)', color: 'var(--fg)', border: '1px solid var(--border)', borderRadius: 'var(--radius)', padding: '3px 8px', fontSize: 11, maxWidth: 220 }}
            onChange={(e) => {
              const val = e.target.value
              if (val) {
                if (val.startsWith('result:')) onLoadResult(val.slice(7))
                else onLoadScenario(val)
              }
              e.target.value = ''
            }}
          >
            <option value="">시나리오 로드...</option>
            {scenarios.filter((s) => !s.has_results).length > 0 && (
              <optgroup label="시나리오">
                {scenarios.filter((s) => !s.has_results).map((s) => (
                  <option key={s.path} value={s.path}>{s.label}</option>
                ))}
              </optgroup>
            )}
            {scenarios.filter((s) => s.has_results).length > 0 && (
              <optgroup label="테스트 결과">
                {scenarios.filter((s) => s.has_results).map((s) => (
                  <option key={'result:' + s.path} value={'result:' + s.path}>{s.label}</option>
                ))}
              </optgroup>
            )}
          </select>
          <button className="btn small" onClick={onSaveState}>저장</button>
          <button className="btn small" onClick={() => window.location.reload()}>새로고침</button>
        </div>
      </div>

      {/* Main 3-column */}
      <div className="main">
        <Sidebar
          npcs={npcs} rels={rels} objects={objects}
          selectedNpcId={npcId}
          onSelectNpc={setNpcId}
          disabled={chatMode || chatEnded}
          disabledMsg={chatMode ? '대화 중에는 수정할 수 없습니다' : chatEnded ? '결과 저장 또는 시나리오 로드 필요' : ''}
          onEditNpc={(n) => { if (chatMode) return toast('대화 중에는 수정할 수 없습니다', 'error'); openModal({ type: 'npc', data: n }) }}
          onEditRel={(r) => { if (chatMode) return toast('대화 중에는 수정할 수 없습니다', 'error'); openModal({ type: 'rel', data: r }) }}
          onEditObj={(o) => { if (chatMode) return toast('대화 중에는 수정할 수 없습니다', 'error'); openModal({ type: 'obj', data: o }) }}
          onNewNpc={() => { if (chatMode) return toast('대화 중에는 수정할 수 없습니다', 'error'); openModal({ type: 'npc', data: null }) }}
          onNewRel={() => { if (chatMode) return toast('대화 중에는 수정할 수 없습니다', 'error'); openModal({ type: 'rel', data: null }) }}
          onNewObj={() => { if (chatMode) return toast('대화 중에는 수정할 수 없습니다', 'error'); openModal({ type: 'obj', data: null }) }}
        />

        {chatMode ? (
          <ChatPanel
            npcId={npcId} partnerId={partnerId} npcs={npcs}
            messages={chatMessages}
            scenarioTurns={chatScenarioTurns}
            scenarioIndex={chatScenarioIdx}
            chatLoading={chatLoading}
            selectedMsgIdx={selectedMsgIdx}
            sceneInfo={sceneInfo}
            onSend={(utterance, pad) => handleChatSend(
              utterance, pad, chatSessionId!, npcId, partnerId, sceneInfo,
              result?.prompt || '', toast, setChatLoading, updateChatMessages,
              setResult, setStimulusUtterance, appendTrace, updateSceneInfo, refresh,
            )}
            onNextTurn={() => {
              if (chatScenarioIdx >= chatScenarioTurns.length) return
              const turn = chatScenarioTurns[chatScenarioIdx]
              advanceScenarioIdx()
              handleChatSend(
                turn.utterance, turn.pad || null, chatSessionId!, npcId, partnerId, sceneInfo,
                result?.prompt || '', toast, setChatLoading, updateChatMessages,
                setResult, setStimulusUtterance, appendTrace, updateSceneInfo, refresh,
              )
            }}
            onSelectMsg={handleSelectMsg}
            onEndChat={() => handleEndChat(
              chatSessionId, npcId, partnerId, toast,
              setChatMode, setChatSessionId, setChatMessages,
              setChatScenarioTurns, setChatScenarioIdx, setSelectedMsgIdx,
              setChatEnded, updateResult, setResultTab, refresh,
            )}
          />
        ) : resultViewActive ? (
          <ResultViewPanel
            npcId={npcId} partnerId={partnerId} npcs={npcs}
            messages={resultMessages}
            selectedMsgIdx={resultSelectedIdx}
            onSelectMsg={handleResultSelectMsg}
            onClose={() => { setResultViewActive(false); setResultSelectedIdx(null) }}
          />
        ) : (
          <SituationPanel
            npcs={npcs} objects={objects}
            npcId={npcId}
            setNpcId={chatEnded ? () => {} : setNpcId}
            partnerId={partnerId}
            setPartnerId={chatEnded ? () => {} : setPartnerId}
            onAppraise={chatEnded ? null : (situation: unknown) => handleAppraise(
              npcId, partnerId, situation as never, toast, setLoading, setResult, setTraceHistory, refresh,
            )}
            onStartChat={chatEnded ? null : resultViewMode ? () => { setResultViewActive(true) } : (situation: unknown) => handleStartChat(
              npcId, partnerId, situation as never, sceneInfo, toast, refresh, chatEnded,
              setChatLoading, setChatSessionId, setChatMode,
              setResultTab, setResult, setLlmModelInfo, setTraceHistory,
              setChatMessages, setChatScenarioTurns, setChatScenarioIdx,
              setSelectedMsgIdx, () => saveScenario(toast),
            )}
            startChatLabel={resultViewMode ? '대화 로드' : undefined}
            loading={loading}
            savedSituation={savedSituation as never}
            sceneInfo={sceneInfo}
            toast={toast}
            disabled={chatEnded}
          />
        )}

        <ResultPanel
          result={result} history={history} traceHistory={traceHistory}
          testReport={testReport} onUpdateReport={onUpdateReport}
          onGuide={() => handleGuide(npcId, partnerId, toast, updateResult)}
          onStimulus={(pad) => handleStimulus(
            npcId, partnerId, pad, toast, setLoading, setResult, appendTrace, refresh, !!result,
          )}
          toast={toast} sceneInfo={sceneInfo}
          chatMessages={resultViewActive ? resultMessages : chatMessages}
          selectedMsgIdx={resultViewActive ? resultSelectedIdx : selectedMsgIdx}
          stimulusUtterance={stimulusUtterance}
          onStimulusUtteranceChange={setStimulusUtterance}
          tab={resultTab} onTabChange={setResultTab}
          llmModelInfo={llmModelInfo}
        />
      </div>

      {/* Modals */}
      {modal?.type === 'npc' && <NpcModal npc={modal.data} onSave={onSaveNpc} onDelete={onDeleteNpc} onClose={closeModal} />}
      {modal?.type === 'rel' && <RelModal rel={modal.data} npcIds={npcs.map((n) => n.id)} onSave={onSaveRel} onDelete={onDeleteRel} onClose={closeModal} />}
      {modal?.type === 'obj' && <ObjModal obj={modal.data} onSave={onSaveObj} onDelete={onDeleteObj} onClose={closeModal} />}
      <ToastContainer toasts={toasts} />
    </div>
  )
}
