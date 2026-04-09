import { create } from 'zustand'
import type { AppraiseResult, TraceEntry, LlmModelInfo } from '../types'

type ResultTab = 'emotions' | 'stimulus' | 'context' | 'trace' | 'history' | 'model' | 'report'

interface ResultStore {
  result: AppraiseResult | null
  traceHistory: TraceEntry[]
  resultTab: ResultTab
  testReport: string
  stimulusUtterance: string
  llmModelInfo: LlmModelInfo | null

  setResult: (result: AppraiseResult | null) => void
  updateResult: (updater: (prev: AppraiseResult | null) => AppraiseResult | null) => void
  setTraceHistory: (trace: TraceEntry[]) => void
  appendTrace: (entry: TraceEntry) => void
  setResultTab: (tab: ResultTab) => void
  setTestReport: (report: string) => void
  setStimulusUtterance: (utterance: string) => void
  setLlmModelInfo: (info: LlmModelInfo | null) => void
}

export const useResultStore = create<ResultStore>((set) => ({
  result: null,
  traceHistory: [],
  resultTab: 'emotions',
  testReport: '',
  stimulusUtterance: '',
  llmModelInfo: null,

  setResult: (result) => set({ result }),
  updateResult: (updater) => set((state) => ({ result: updater(state.result) })),
  setTraceHistory: (traceHistory) => set({ traceHistory }),
  appendTrace: (entry) => set((state) => ({ traceHistory: [...state.traceHistory, entry] })),
  setResultTab: (resultTab) => set({ resultTab }),
  setTestReport: (testReport) => set({ testReport }),
  setStimulusUtterance: (stimulusUtterance) => set({ stimulusUtterance }),
  setLlmModelInfo: (llmModelInfo) => set({ llmModelInfo }),
}))
