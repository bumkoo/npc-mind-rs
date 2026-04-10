import { create } from 'zustand'
import type { ChatMessage, ScenarioTurn } from '../types'

interface ChatStore {
  chatMode: boolean
  chatSessionId: string | null
  chatMessages: ChatMessage[]
  chatLoading: boolean
  chatScenarioTurns: ScenarioTurn[]
  chatScenarioIdx: number
  chatEnded: boolean
  selectedMsgIdx: number | null

  setChatMode: (mode: boolean) => void
  setChatSessionId: (id: string | null) => void
  setChatMessages: (messages: ChatMessage[]) => void
  updateChatMessages: (updater: (prev: ChatMessage[]) => ChatMessage[]) => void
  setChatLoading: (loading: boolean) => void
  setChatScenarioTurns: (turns: ScenarioTurn[]) => void
  setChatScenarioIdx: (idx: number) => void
  advanceScenarioIdx: () => void
  setChatEnded: (ended: boolean) => void
  setSelectedMsgIdx: (idx: number | null) => void

  reset: () => void
}

export const useChatStore = create<ChatStore>((set) => ({
  chatMode: false,
  chatSessionId: null,
  chatMessages: [],
  chatLoading: false,
  chatScenarioTurns: [],
  chatScenarioIdx: 0,
  chatEnded: false,
  selectedMsgIdx: null,

  setChatMode: (chatMode) => set({ chatMode }),
  setChatSessionId: (chatSessionId) => set({ chatSessionId }),
  setChatMessages: (chatMessages) => set({ chatMessages }),
  updateChatMessages: (updater) =>
    set((state) => ({ chatMessages: updater(state.chatMessages) })),
  setChatLoading: (chatLoading) => set({ chatLoading }),
  setChatScenarioTurns: (chatScenarioTurns) => set({ chatScenarioTurns }),
  setChatScenarioIdx: (chatScenarioIdx) => set({ chatScenarioIdx }),
  advanceScenarioIdx: () =>
    set((state) => ({ chatScenarioIdx: state.chatScenarioIdx + 1 })),
  setChatEnded: (chatEnded) => set({ chatEnded }),
  setSelectedMsgIdx: (selectedMsgIdx) => set({ selectedMsgIdx }),

  reset: () =>
    set({
      chatMode: false,
      chatSessionId: null,
      chatMessages: [],
      chatLoading: false,
      chatScenarioTurns: [],
      chatScenarioIdx: 0,
      chatEnded: false,
      selectedMsgIdx: null,
    }),
}))
