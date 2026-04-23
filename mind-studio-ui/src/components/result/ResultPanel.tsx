import type { AppraiseResult, TurnHistory, TraceEntry, ChatMessage, SceneInfo, LlmModelInfo, Npc, Pad, ToastFn } from '../../types'
import ScenePanel from './ScenePanel'
import EmotionView from './EmotionView'
import ContextView from './ContextView'
import StimulusView from './StimulusView'
import TraceView from './TraceView'
import ReportView from './ReportView'
import HistoryView from './HistoryView'
import ModelInfoView from './ModelInfoView'
import MemoryView from './MemoryView'
import RumorView from './RumorView'
import ScenarioSeedsView from './ScenarioSeedsView'

interface ResultPanelProps {
  result: AppraiseResult | null
  history: TurnHistory[]
  traceHistory: TraceEntry[]
  testReport: string
  onUpdateReport: (content: string) => void
  onGuide: () => void
  onStimulus: (data: { pleasure: number; arousal: number; dominance: number; situation_description: string | null }) => void
  toast: ToastFn
  sceneInfo: SceneInfo | null
  chatMessages: ChatMessage[]
  selectedMsgIdx: number | null
  stimulusUtterance: string
  onStimulusUtteranceChange: (u: string) => void
  tab: string
  onTabChange: (tab: string) => void
  llmModelInfo: LlmModelInfo | null
  /** Step E2: Memory 탭에서 NPC 선택 드롭다운에 쓰임. */
  npcs: Npc[]
}

export default function ResultPanel({
  result, history, traceHistory, testReport, onUpdateReport,
  onGuide, onStimulus, toast, sceneInfo,
  chatMessages, selectedMsgIdx,
  stimulusUtterance, onStimulusUtteranceChange,
  tab, onTabChange, llmModelInfo, npcs,
}: ResultPanelProps) {
  const setTab = onTabChange

  // Compute initialPad for StimulusView
  const getInitialPad = (): Pad | undefined => {
    if (selectedMsgIdx == null || !chatMessages || !chatMessages[selectedMsgIdx]) return undefined
    const msg = chatMessages[selectedMsgIdx]
    if (msg.role === 'user' && msg.pad) return msg.pad
    if (msg.role === 'assistant') {
      for (let j = selectedMsgIdx - 1; j >= 0; j--) {
        if (chatMessages[j]?.role === 'user' && chatMessages[j]?.pad) {
          return chatMessages[j].pad!
        }
      }
    }
    return undefined
  }

  return (
    <div className="right">
      <div className="result-tabs">
        <div className={`result-tab ${tab === 'emotions' ? 'active' : ''}`} onClick={() => setTab('emotions')}>감정 상태</div>
        <div className={`result-tab ${tab === 'prompt' ? 'active' : ''}`} onClick={() => setTab('prompt')}>컨텍스트</div>
        <div className={`result-tab ${tab === 'stimulus' ? 'active' : ''}`} onClick={() => setTab('stimulus')}>자극</div>
        <div className={`result-tab ${tab === 'trace' ? 'active' : ''}`} onClick={() => setTab('trace')}>Trace</div>
        <div className={`result-tab ${tab === 'report' ? 'active' : ''}`} onClick={() => setTab('report')}>보고서</div>
        <div className={`result-tab ${tab === 'history' ? 'active' : ''}`} onClick={() => setTab('history')}>히스토리 ({history.length})</div>
        <div className={`result-tab ${tab === 'memory' ? 'active' : ''}`} onClick={() => setTab('memory')}>기억</div>
        <div className={`result-tab ${tab === 'rumor' ? 'active' : ''}`} onClick={() => setTab('rumor')}>소문</div>
        <div className={`result-tab ${tab === 'seeds' ? 'active' : ''}`} onClick={() => setTab('seeds')}>시드</div>
        <div className={`result-tab ${tab === 'model' ? 'active' : ''}`} onClick={() => setTab('model')}>LLM Model</div>
      </div>
      <div className="result-content">
        {tab === 'emotions' && <ScenePanel sceneInfo={sceneInfo} />}
        {!result && tab !== 'history' && tab !== 'stimulus' && tab !== 'memory' && tab !== 'rumor' && tab !== 'seeds' ? (
          <div className="empty">
            {!sceneInfo && <div style={{ fontSize: 24, marginBottom: 8 }}>🎭</div>}
            {!sceneInfo ? (
              <>NPC와 대화 상대를 선택하고<br />상황을 설정한 뒤 감정 평가를 실행하세요</>
            ) : (
              <div style={{ fontSize: 11, color: 'var(--fg3)', marginTop: 8 }}>
                Scene이 로드됨 — stimulus를 적용하세요
              </div>
            )}
          </div>
        ) : (
          <>
            {tab === 'emotions' && result && <EmotionView result={result} />}
            {tab === 'prompt' && result && !result.afterDialogue && (
              <ContextView
                prompt={result.prompt || ''}
                chatMessages={chatMessages}
                selectedMsgIdx={selectedMsgIdx}
                onRegenerate={onGuide}
                onCopy={() => toast && toast('클립보드에 복사됨', 'success')}
              />
            )}
            {tab === 'stimulus' && (
              <StimulusView
                utterance={stimulusUtterance}
                onUtteranceChange={onStimulusUtteranceChange}
                initialPad={getInitialPad()}
                onApply={(pad) => {
                  onStimulus({ ...pad, situation_description: null })
                  setTab('emotions')
                }}
                toast={toast}
              />
            )}
            {tab === 'trace' && result && !result.afterDialogue && (
              <TraceView traceHistory={traceHistory} />
            )}
            {tab === 'report' && <ReportView content={testReport} onUpdate={onUpdateReport} />}
            {tab === 'history' && <HistoryView history={history} />}
            {tab === 'memory' && <MemoryView npcs={npcs} />}
            {tab === 'rumor' && <RumorView />}
            {tab === 'seeds' && <ScenarioSeedsView />}
            {tab === 'model' && <ModelInfoView info={llmModelInfo} />}
          </>
        )}
      </div>
    </div>
  )
}
