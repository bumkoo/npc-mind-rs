import { create } from 'zustand'
import type { AppraiseResult, TraceEntry, LlmModelInfo, AfterDialogueResponse } from '../types'

type ResultTab = 'emotions' | 'stimulus' | 'context' | 'trace' | 'history' | 'model' | 'report' | 'reflection'

interface ResultStore {
  result: AppraiseResult | null
  traceHistory: TraceEntry[]
  resultTab: string
  testReport: string
  stimulusUtterance: string
  llmModelInfo: LlmModelInfo | null
  /**
   * Phase 1.5 — 가장 최근 after_dialogue 결과. chitchat 시에도 박제 (reflection.is_some).
   * SSE `dialogue_reflected` 시점 또는 `/api/after-dialogue` 응답 시점에 갱신.
   * `null`이면 ReflectionView가 빈 상태 안내.
   */
  lastAfterDialogue: AfterDialogueResponse | null

  setResult: (result: AppraiseResult | null) => void
  updateResult: (updater: (prev: AppraiseResult | null) => AppraiseResult | null) => void
  setTraceHistory: (trace: TraceEntry[]) => void
  appendTrace: (entry: TraceEntry) => void
  setResultTab: (tab: string) => void
  setTestReport: (report: string) => void
  setStimulusUtterance: (utterance: string) => void
  setLlmModelInfo: (info: LlmModelInfo | null) => void
  setLastAfterDialogue: (response: AfterDialogueResponse | null) => void
}

export const useResultStore = create<ResultStore>((set) => ({
  result: null,
  traceHistory: [],
  resultTab: 'emotions',
  testReport: '',
  stimulusUtterance: '',
  llmModelInfo: null,
  lastAfterDialogue: null,

  setResult: (result) => set({ result }),
  updateResult: (updater) => set((state) => ({ result: updater(state.result) })),
  setTraceHistory: (traceHistory) => set({ traceHistory }),
  appendTrace: (entry) => set((state) => ({ traceHistory: [...state.traceHistory, entry] })),
  setResultTab: (resultTab) => set({ resultTab }),
  setTestReport: (testReport) => set({ testReport }),
  setStimulusUtterance: (stimulusUtterance) => set({ stimulusUtterance }),
  setLlmModelInfo: (llmModelInfo) => set({ llmModelInfo }),
  setLastAfterDialogue: (lastAfterDialogue) => set({ lastAfterDialogue }),
}))
