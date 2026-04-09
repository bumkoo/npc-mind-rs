import { create } from 'zustand'
import type { ModalState, ChatMessage, TurnHistory } from '../types'

interface UIStore {
  npcId: string
  partnerId: string
  modal: ModalState | null
  loading: boolean
  connected: boolean
  resultViewMode: boolean
  resultViewActive: boolean
  resultTurnHistory: TurnHistory[]
  resultMessages: ChatMessage[]
  resultSelectedIdx: number | null

  setNpcId: (id: string) => void
  setPartnerId: (id: string) => void
  openModal: (modal: ModalState) => void
  closeModal: () => void
  setLoading: (loading: boolean) => void
  setConnected: (connected: boolean) => void
  setResultView: (opts: {
    mode: boolean
    active: boolean
    turnHistory: TurnHistory[]
    messages: ChatMessage[]
    selectedIdx: number | null
  }) => void
  setResultViewActive: (active: boolean) => void
  setResultSelectedIdx: (idx: number | null) => void
  setResultMessages: (messages: ChatMessage[]) => void
  closeResultView: () => void
}

export const useUIStore = create<UIStore>((set) => ({
  npcId: '',
  partnerId: '',
  modal: null,
  loading: false,
  connected: false,
  resultViewMode: false,
  resultViewActive: false,
  resultTurnHistory: [],
  resultMessages: [],
  resultSelectedIdx: null,

  setNpcId: (npcId) => set({ npcId }),
  setPartnerId: (partnerId) => set({ partnerId }),
  openModal: (modal) => set({ modal }),
  closeModal: () => set({ modal: null }),
  setLoading: (loading) => set({ loading }),
  setConnected: (connected) => set({ connected }),
  setResultView: (opts) =>
    set({
      resultViewMode: opts.mode,
      resultViewActive: opts.active,
      resultTurnHistory: opts.turnHistory,
      resultMessages: opts.messages,
      resultSelectedIdx: opts.selectedIdx,
    }),
  setResultViewActive: (resultViewActive) => set({ resultViewActive }),
  setResultSelectedIdx: (resultSelectedIdx) => set({ resultSelectedIdx }),
  setResultMessages: (resultMessages) => set({ resultMessages }),
  closeResultView: () =>
    set({
      resultViewMode: false,
      resultViewActive: false,
      resultTurnHistory: [],
      resultMessages: [],
      resultSelectedIdx: null,
    }),
}))
