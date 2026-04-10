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
import type { Npc, Relationship, GameObject, ChatMessage } from './types'

export default function App() {
  // --- Stores ---
  const { npcs, rels, objects, scenarios, history } = useEntityStore()
  const entityStore = useEntityStore()
  const {
    npcId, partnerId, modal, loading, connected,
    resultViewMode, resultViewActive, resultMessages, resultSelectedIdx,
  } = useUIStore()
  const ui = useUIStore()
  const {
    result, traceHistory, resultTab, testReport, stimulusUtterance, llmModelInfo,
  } = useResultStore()
  const rs = useResultStore()
  const {
    chatMode, chatSessionId, chatMessages, chatLoading,
    chatScenarioTurns, chatScenarioIdx, chatEnded, selectedMsgIdx,
  } = useChatStore()
  const chat = useChatStore()
  const { scenarioMeta, savedSituation, sceneInfo } = useSceneStore()
  const scene = useSceneStore()

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
    await entityStore.saveNpc(data); ui.closeModal(); refresh()
  }, [guardChat, entityStore, ui, refresh])

  const onDeleteNpc = useCallback(async (id: string) => {
    if (guardChat()) return
    await entityStore.deleteNpc(id); ui.closeModal()
    if (npcId === id) ui.setNpcId('')
    refresh()
  }, [guardChat, entityStore, ui, npcId, refresh])

  const onSaveRel = useCallback(async (data: Relationship) => {
    if (guardChat()) return
    await entityStore.saveRel(data); ui.closeModal(); refresh()
  }, [guardChat, entityStore, ui, refresh])

  const onDeleteRel = useCallback(async (oid: string, tid: string) => {
    if (guardChat()) return
    await entityStore.deleteRel(oid, tid); ui.closeModal(); refresh()
  }, [guardChat, entityStore, ui, refresh])

  const onSaveObj = useCallback(async (data: GameObject) => {
    if (guardChat()) return
    await entityStore.saveObj(data); ui.closeModal(); refresh()
  }, [guardChat, entityStore, ui, refresh])

  const onDeleteObj = useCallback(async (id: string) => {
    if (guardChat()) return
    await entityStore.deleteObj(id); ui.closeModal(); refresh()
  }, [guardChat, entityStore, ui, refresh])

  // --- Message selection ---
  const handleSelectMsg = useCallback((idx: number) => {
    const msgs = chatMessages
    if (selectedMsgIdx === idx) {
      chat.setSelectedMsgIdx(null)
      const lastSnap = [...msgs].reverse().find((m) => m.snapshot)
      if (lastSnap) rs.setResult(lastSnap.snapshot!)
    } else {
      chat.setSelectedMsgIdx(idx)
      const msg = msgs[idx]
      if (msg) rs.setTraceHistory((msg as ChatMessage & { trace?: unknown[] }).trace || [])
      if (msg?.snapshot) rs.setResult(msg.snapshot)
      else if (msg?.role === 'user') {
        const next = msgs[idx + 1]
        if (next?.snapshot) rs.setResult(next.snapshot)
      }
      if (msg) {
        if (msg.role === 'user' && msg.content) rs.setStimulusUtterance(msg.content)
        else if (msg.role === 'assistant') {
          for (let j = idx - 1; j >= 0; j--) {
            if (msgs[j]?.role === 'user' && msgs[j]?.content) { rs.setStimulusUtterance(msgs[j].content); break }
          }
        }
      }
    }
  }, [chatMessages, selectedMsgIdx, chat, rs])

  const handleResultSelectMsg = useCallback((idx: number) => {
    const msgs = resultMessages
    if (resultSelectedIdx === idx) {
      ui.setResultSelectedIdx(null)
      const lastSnap = [...msgs].reverse().find((m) => m.snapshot)
      if (lastSnap) rs.setResult(lastSnap.snapshot!)
    } else {
      ui.setResultSelectedIdx(idx)
      const msg = msgs[idx]
      if (msg) rs.setTraceHistory((msg as ChatMessage & { trace?: unknown[] }).trace || [])
      if (msg?.snapshot) rs.setResult(msg.snapshot)
      else if (msg?.role === 'user') {
        const next = msgs[idx + 1]
        if (next?.snapshot) rs.setResult(next.snapshot)
      }
      if (msg) {
        if (msg.role === 'user' && msg.content) rs.setStimulusUtterance(msg.content)
        else if (msg.role === 'assistant') {
          for (let j = idx - 1; j >= 0; j--) {
            if (msgs[j]?.role === 'user' && msgs[j]?.content) { rs.setStimulusUtterance(msgs[j].content); break }
          }
        }
      }
    }
  }, [resultMessages, resultSelectedIdx, ui, rs])

  // --- Scenario load/save ---
  const onLoadScenario = useCallback((path: string) => {
    loadScenario(path, toast, refresh, chat.setChatEnded, ui.setResultView, scene.setSavedSituation, rs.setResult, rs.setTraceHistory)
  }, [toast, refresh, chat, ui, scene, rs])

  const onLoadResult = useCallback((path: string) => {
    loadResult(path, toast, refresh, chat.setChatEnded, ui.setResultView, scene.setSavedSituation, rs.setResult, rs.setTraceHistory, rs.setLlmModelInfo, rs.setResultTab)
  }, [toast, refresh, chat, ui, scene, rs])

  const onSaveState = useCallback(() => saveState(chatMode, chatEnded, toast), [chatMode, chatEnded, toast])

  const onUpdateReport = useCallback((content: string) => {
    updateTestReport(content, rs.setTestReport)
  }, [rs])

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
          onSelectNpc={(id) => ui.setNpcId(id)}
          disabled={chatMode || chatEnded}
          disabledMsg={chatMode ? '대화 중에는 수정할 수 없습니다' : chatEnded ? '결과 저장 또는 시나리오 로드 필요' : ''}
          onEditNpc={(n) => { if (chatMode) return toast('대화 중에는 수정할 수 없습니다', 'error'); ui.openModal({ type: 'npc', data: n }) }}
          onEditRel={(r) => { if (chatMode) return toast('대화 중에는 수정할 수 없습니다', 'error'); ui.openModal({ type: 'rel', data: r }) }}
          onEditObj={(o) => { if (chatMode) return toast('대화 중에는 수정할 수 없습니다', 'error'); ui.openModal({ type: 'obj', data: o }) }}
          onNewNpc={() => { if (chatMode) return toast('대화 중에는 수정할 수 없습니다', 'error'); ui.openModal({ type: 'npc', data: null }) }}
          onNewRel={() => { if (chatMode) return toast('대화 중에는 수정할 수 없습니다', 'error'); ui.openModal({ type: 'rel', data: null }) }}
          onNewObj={() => { if (chatMode) return toast('대화 중에는 수정할 수 없습니다', 'error'); ui.openModal({ type: 'obj', data: null }) }}
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
              result?.prompt || '', toast, chat.setChatLoading, chat.updateChatMessages,
              rs.setResult, rs.setStimulusUtterance, rs.appendTrace, scene.updateSceneInfo, refresh,
            )}
            onNextTurn={() => {
              if (chatScenarioIdx >= chatScenarioTurns.length) return
              const turn = chatScenarioTurns[chatScenarioIdx]
              chat.advanceScenarioIdx()
              handleChatSend(
                turn.utterance, turn.pad || null, chatSessionId!, npcId, partnerId, sceneInfo,
                result?.prompt || '', toast, chat.setChatLoading, chat.updateChatMessages,
                rs.setResult, rs.setStimulusUtterance, rs.appendTrace, scene.updateSceneInfo, refresh,
              )
            }}
            onSelectMsg={handleSelectMsg}
            onEndChat={() => handleEndChat(
              chatSessionId, npcId, partnerId, toast,
              chat.setChatMode, chat.setChatSessionId, chat.setChatMessages,
              chat.setChatScenarioTurns, chat.setChatScenarioIdx, chat.setSelectedMsgIdx,
              chat.setChatEnded, rs.updateResult, rs.setResultTab, refresh,
            )}
          />
        ) : resultViewActive ? (
          <ResultViewPanel
            npcId={npcId} partnerId={partnerId} npcs={npcs}
            messages={resultMessages}
            selectedMsgIdx={resultSelectedIdx}
            onSelectMsg={handleResultSelectMsg}
            onClose={() => { ui.setResultViewActive(false); ui.setResultSelectedIdx(null) }}
          />
        ) : (
          <SituationPanel
            npcs={npcs} objects={objects}
            npcId={npcId}
            setNpcId={chatEnded ? () => {} : ui.setNpcId}
            partnerId={partnerId}
            setPartnerId={chatEnded ? () => {} : ui.setPartnerId}
            onAppraise={chatEnded ? null : (situation: unknown) => handleAppraise(
              npcId, partnerId, situation as never, toast, ui.setLoading, rs.setResult, rs.setTraceHistory, refresh,
            )}
            onStartChat={chatEnded ? null : resultViewMode ? () => { ui.setResultViewActive(true) } : (situation: unknown) => handleStartChat(
              npcId, partnerId, situation as never, sceneInfo, toast, refresh, chatEnded,
              chat.setChatLoading, chat.setChatSessionId, chat.setChatMode,
              rs.setResultTab, rs.setResult, rs.setLlmModelInfo, rs.setTraceHistory,
              chat.setChatMessages, chat.setChatScenarioTurns, chat.setChatScenarioIdx,
              chat.setSelectedMsgIdx, () => saveScenario(toast),
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
          onGuide={() => handleGuide(npcId, partnerId, toast, rs.updateResult)}
          onStimulus={(pad) => handleStimulus(
            npcId, partnerId, pad, toast, ui.setLoading, rs.setResult, rs.appendTrace, refresh, !!result,
          )}
          toast={toast} sceneInfo={sceneInfo}
          chatMessages={resultViewActive ? resultMessages : chatMessages}
          selectedMsgIdx={resultViewActive ? resultSelectedIdx : selectedMsgIdx}
          stimulusUtterance={stimulusUtterance}
          onStimulusUtteranceChange={rs.setStimulusUtterance}
          tab={resultTab} onTabChange={rs.setResultTab}
          llmModelInfo={llmModelInfo}
        />
      </div>

      {/* Modals */}
      {modal?.type === 'npc' && <NpcModal npc={modal.data} onSave={onSaveNpc} onDelete={onDeleteNpc} onClose={ui.closeModal} />}
      {modal?.type === 'rel' && <RelModal rel={modal.data} npcIds={npcs.map((n) => n.id)} onSave={onSaveRel} onDelete={onDeleteRel} onClose={ui.closeModal} />}
      {modal?.type === 'obj' && <ObjModal obj={modal.data} onSave={onSaveObj} onDelete={onDeleteObj} onClose={ui.closeModal} />}
      <ToastContainer toasts={toasts} />
    </div>
  )
}
