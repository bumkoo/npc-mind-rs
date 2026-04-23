import { describe, it, expect, beforeEach } from 'vitest'
import { useEntityStore } from '../stores/useEntityStore'
import { useUIStore } from '../stores/useUIStore'
import { useResultStore } from '../stores/useResultStore'
import { useChatStore } from '../stores/useChatStore'
import { useSceneStore } from '../stores/useSceneStore'
import { useMemoryStore } from '../stores/useMemoryStore'
import type { Npc, Relationship, GameObject, AppraiseResult, ChatMessage, SceneInfo, MemoryEntry, Rumor } from '../types'

// Helper to reset stores between tests
function resetAllStores() {
  useEntityStore.setState({
    npcs: [], rels: [], objects: [], scenarios: [], history: [],
  })
  useUIStore.setState({
    npcId: '', partnerId: '', modal: null, loading: false, connected: false,
    resultViewMode: false, resultViewActive: false, resultTurnHistory: [],
    resultMessages: [], resultSelectedIdx: null,
  })
  useResultStore.setState({
    result: null, traceHistory: [], resultTab: 'emotions', testReport: '',
    stimulusUtterance: '', llmModelInfo: null,
  })
  useChatStore.getState().reset()
  useSceneStore.setState({
    scenarioMeta: null, savedSituation: null, sceneInfo: null, scenarioSeeds: {},
  })
  useMemoryStore.getState().clear()
}

const mockNpc: Npc = {
  id: 'mu_baek', name: '무백', description: '정의로운 검객',
  sincerity: 0.8, fairness: 0.7, greed_avoidance: 0.6, modesty: 0.5,
  fearfulness: -0.3, anxiety: -0.2, dependence: -0.1, sentimentality: 0.4,
  social_self_esteem: 0.6, social_boldness: 0.5, sociability: 0.3, liveliness: 0.4,
  forgiveness: 0.7, gentleness: 0.6, flexibility: 0.5, patience: 0.8,
  organization: 0.5, diligence: 0.7, perfectionism: 0.4, prudence: 0.6,
  aesthetic_appreciation: 0.3, inquisitiveness: 0.5, creativity: 0.4, unconventionality: 0.2,
}

const mockRel: Relationship = {
  owner_id: 'mu_baek', target_id: 'player', closeness: 0.3, trust: 0.5, power: 0,
}

const mockObj: GameObject = { id: 'sword', description: '낡은 검', category: '무기' }

// ===== EntityStore =====
describe('useEntityStore', () => {
  beforeEach(resetAllStores)

  it('초기 상태가 빈 배열', () => {
    const { npcs, rels, objects } = useEntityStore.getState()
    expect(npcs).toEqual([])
    expect(rels).toEqual([])
    expect(objects).toEqual([])
  })

  it('setNpcs로 NPC 목록 설정', () => {
    useEntityStore.getState().setNpcs([mockNpc])
    expect(useEntityStore.getState().npcs).toHaveLength(1)
    expect(useEntityStore.getState().npcs[0].id).toBe('mu_baek')
  })

  it('setRels로 관계 설정', () => {
    useEntityStore.getState().setRels([mockRel])
    expect(useEntityStore.getState().rels[0].owner_id).toBe('mu_baek')
  })

  it('setObjects로 오브젝트 설정', () => {
    useEntityStore.getState().setObjects([mockObj])
    expect(useEntityStore.getState().objects[0].category).toBe('무기')
  })

  it('setHistory로 히스토리 설정', () => {
    useEntityStore.getState().setHistory([
      { action: 'appraise', label: '감정 평가' },
    ])
    expect(useEntityStore.getState().history).toHaveLength(1)
  })

  it('setScenarios로 시나리오 목록 설정', () => {
    useEntityStore.getState().setScenarios([
      { path: 'test/scenario.json', label: '테스트', has_results: false },
    ])
    expect(useEntityStore.getState().scenarios[0].label).toBe('테스트')
  })
})

// ===== UIStore =====
describe('useUIStore', () => {
  beforeEach(resetAllStores)

  it('초기 상태', () => {
    const s = useUIStore.getState()
    expect(s.npcId).toBe('')
    expect(s.modal).toBeNull()
    expect(s.connected).toBe(false)
  })

  it('NPC/파트너 ID 설정', () => {
    useUIStore.getState().setNpcId('mu_baek')
    useUIStore.getState().setPartnerId('player')
    expect(useUIStore.getState().npcId).toBe('mu_baek')
    expect(useUIStore.getState().partnerId).toBe('player')
  })

  it('모달 열기/닫기', () => {
    useUIStore.getState().openModal({ type: 'npc', data: mockNpc })
    expect(useUIStore.getState().modal?.type).toBe('npc')
    useUIStore.getState().closeModal()
    expect(useUIStore.getState().modal).toBeNull()
  })

  it('setResultView로 결과 뷰 상태 일괄 설정', () => {
    const msgs: ChatMessage[] = [{ role: 'system', content: 'test' }]
    useUIStore.getState().setResultView({
      mode: true, active: true, turnHistory: [], messages: msgs, selectedIdx: 0,
    })
    const s = useUIStore.getState()
    expect(s.resultViewMode).toBe(true)
    expect(s.resultViewActive).toBe(true)
    expect(s.resultMessages).toHaveLength(1)
    expect(s.resultSelectedIdx).toBe(0)
  })

  it('closeResultView로 결과 뷰 상태 초기화', () => {
    useUIStore.getState().setResultView({
      mode: true, active: true, turnHistory: [], messages: [{ role: 'system', content: 'x' }], selectedIdx: 1,
    })
    useUIStore.getState().closeResultView()
    const s = useUIStore.getState()
    expect(s.resultViewMode).toBe(false)
    expect(s.resultViewActive).toBe(false)
    expect(s.resultMessages).toEqual([])
    expect(s.resultSelectedIdx).toBeNull()
  })

  it('loading/connected 토글', () => {
    useUIStore.getState().setLoading(true)
    expect(useUIStore.getState().loading).toBe(true)
    useUIStore.getState().setConnected(true)
    expect(useUIStore.getState().connected).toBe(true)
  })
})

// ===== ResultStore =====
describe('useResultStore', () => {
  beforeEach(resetAllStores)

  it('초기 상태', () => {
    const s = useResultStore.getState()
    expect(s.result).toBeNull()
    expect(s.traceHistory).toEqual([])
    expect(s.resultTab).toBe('emotions')
  })

  it('setResult로 결과 설정/해제', () => {
    const r: AppraiseResult = { emotions: [{ emotion_type: 'Joy', intensity: 0.8 }], mood: 0.6 }
    useResultStore.getState().setResult(r)
    expect(useResultStore.getState().result?.emotions?.[0].emotion_type).toBe('Joy')
    useResultStore.getState().setResult(null)
    expect(useResultStore.getState().result).toBeNull()
  })

  it('updateResult로 기존 결과 변환', () => {
    useResultStore.getState().setResult({ emotions: [], mood: 0.5 })
    useResultStore.getState().updateResult((prev) =>
      prev ? { ...prev, mood: 0.9 } : null,
    )
    expect(useResultStore.getState().result?.mood).toBe(0.9)
  })

  it('updateResult — null 상태에서 호출 시 null 유지', () => {
    useResultStore.getState().updateResult((prev) =>
      prev ? { ...prev, mood: 1.0 } : null,
    )
    expect(useResultStore.getState().result).toBeNull()
  })

  it('appendTrace로 트레이스 누적', () => {
    useResultStore.getState().appendTrace('step1')
    useResultStore.getState().appendTrace('step2')
    expect(useResultStore.getState().traceHistory).toEqual(['step1', 'step2'])
  })

  it('setTraceHistory로 트레이스 덮어쓰기', () => {
    useResultStore.getState().appendTrace('old')
    useResultStore.getState().setTraceHistory(['new1', 'new2'])
    expect(useResultStore.getState().traceHistory).toEqual(['new1', 'new2'])
  })

  it('탭/보고서/대사/모델 설정', () => {
    useResultStore.getState().setResultTab('trace')
    expect(useResultStore.getState().resultTab).toBe('trace')
    useResultStore.getState().setTestReport('# 보고서')
    expect(useResultStore.getState().testReport).toBe('# 보고서')
    useResultStore.getState().setStimulusUtterance('안녕하세요')
    expect(useResultStore.getState().stimulusUtterance).toBe('안녕하세요')
    useResultStore.getState().setLlmModelInfo({ id: 'test-model' })
    expect(useResultStore.getState().llmModelInfo?.id).toBe('test-model')
  })
})

// ===== ChatStore =====
describe('useChatStore', () => {
  beforeEach(resetAllStores)

  it('초기 상태', () => {
    const s = useChatStore.getState()
    expect(s.chatMode).toBe(false)
    expect(s.chatSessionId).toBeNull()
    expect(s.chatMessages).toEqual([])
    expect(s.chatEnded).toBe(false)
  })

  it('대화 모드 설정', () => {
    useChatStore.getState().setChatMode(true)
    useChatStore.getState().setChatSessionId('sess-123')
    expect(useChatStore.getState().chatMode).toBe(true)
    expect(useChatStore.getState().chatSessionId).toBe('sess-123')
  })

  it('메시지 추가 및 업데이트', () => {
    useChatStore.getState().setChatMessages([
      { role: 'system', content: '대화 시작' },
    ])
    useChatStore.getState().updateChatMessages((prev) => [
      ...prev,
      { role: 'user', content: '안녕' },
    ])
    expect(useChatStore.getState().chatMessages).toHaveLength(2)
    expect(useChatStore.getState().chatMessages[1].content).toBe('안녕')
  })

  it('advanceScenarioIdx 증가', () => {
    expect(useChatStore.getState().chatScenarioIdx).toBe(0)
    useChatStore.getState().advanceScenarioIdx()
    expect(useChatStore.getState().chatScenarioIdx).toBe(1)
    useChatStore.getState().advanceScenarioIdx()
    expect(useChatStore.getState().chatScenarioIdx).toBe(2)
  })

  it('reset()으로 전체 초기화 (chatEnded 포함)', () => {
    useChatStore.getState().setChatMode(true)
    useChatStore.getState().setChatSessionId('sess')
    useChatStore.getState().setChatEnded(true)
    useChatStore.getState().setChatMessages([{ role: 'user', content: 'x' }])
    useChatStore.getState().setChatScenarioIdx(5)

    useChatStore.getState().reset()
    const s = useChatStore.getState()
    expect(s.chatMode).toBe(false)
    expect(s.chatSessionId).toBeNull()
    expect(s.chatMessages).toEqual([])
    expect(s.chatEnded).toBe(false)
    expect(s.chatScenarioIdx).toBe(0)
    expect(s.selectedMsgIdx).toBeNull()
  })

  it('selectedMsgIdx 설정', () => {
    useChatStore.getState().setSelectedMsgIdx(3)
    expect(useChatStore.getState().selectedMsgIdx).toBe(3)
    useChatStore.getState().setSelectedMsgIdx(null)
    expect(useChatStore.getState().selectedMsgIdx).toBeNull()
  })
})

// ===== SceneStore =====
describe('useSceneStore', () => {
  beforeEach(resetAllStores)

  it('초기 상태', () => {
    const s = useSceneStore.getState()
    expect(s.scenarioMeta).toBeNull()
    expect(s.savedSituation).toBeNull()
    expect(s.sceneInfo).toBeNull()
    expect(s.scenarioSeeds).toEqual({})
  })

  // Step E3.3 — 시나리오 시드 조회 상태
  it('setScenarioSeeds로 4 섹션 저장', () => {
    useSceneStore.getState().setScenarioSeeds({
      initial_rumors: [{ id: 'r1', topic: 'x' }],
      world_knowledge: [{ world_id: 'jianghu', content: '본문' }],
    })
    const s = useSceneStore.getState()
    expect(s.scenarioSeeds.initial_rumors).toHaveLength(1)
    expect(s.scenarioSeeds.world_knowledge).toHaveLength(1)
    expect(s.scenarioSeeds.faction_knowledge).toBeUndefined()
  })

  it('시나리오 메타 설정', () => {
    useSceneStore.getState().setScenarioMeta({ name: '테스트 시나리오', description: '설명' })
    expect(useSceneStore.getState().scenarioMeta?.name).toBe('테스트 시나리오')
  })

  it('savedSituation 설정', () => {
    useSceneStore.getState().setSavedSituation({ description: '상황 설명' })
    expect(useSceneStore.getState().savedSituation?.description).toBe('상황 설명')
  })

  it('setSceneInfo 직접 설정', () => {
    const info: SceneInfo = { has_scene: true, script_cursor: 0, significance: 0.5 }
    useSceneStore.getState().setSceneInfo(info)
    expect(useSceneStore.getState().sceneInfo?.has_scene).toBe(true)
  })

  it('updateSceneInfo로 기존 값 변환', () => {
    useSceneStore.getState().setSceneInfo({ has_scene: true, script_cursor: 0 })
    useSceneStore.getState().updateSceneInfo((prev) =>
      prev ? { ...prev, script_cursor: 3 } : null,
    )
    expect(useSceneStore.getState().sceneInfo?.script_cursor).toBe(3)
  })

  it('updateSceneInfo — null 상태에서 호출', () => {
    useSceneStore.getState().updateSceneInfo((prev) =>
      prev ? { ...prev, script_cursor: 1 } : null,
    )
    expect(useSceneStore.getState().sceneInfo).toBeNull()
  })
})

// ===== MemoryStore (Step E2) =====
const mockMemoryEntry: MemoryEntry = {
  id: 'mem-000001',
  created_seq: 1,
  event_id: 1,
  scope: { kind: 'personal', npc_id: 'mu_baek' },
  source: 'experienced',
  provenance: 'runtime',
  memory_type: 'DialogueTurn',
  layer: 'A',
  content: '첫 만남의 기억',
  topic: null,
  emotional_context: null,
  timestamp_ms: 1_700_000_000_000,
  last_recalled_at: null,
  recall_count: 0,
  origin_chain: [],
  confidence: 1.0,
  acquired_by: null,
  superseded_by: null,
  consolidated_into: null,
  npc_id: 'mu_baek',
}

const mockRumor: Rumor = {
  id: 'rumor-000001',
  topic: 'sect:leader',
  seed_content: null,
  origin: { kind: 'seeded' },
  reach_policy: { regions: [], factions: [], npc_ids: [], min_significance: 0 },
  hops: [],
  distortions: [],
  created_at: 1_700_000_000_000,
  status: 'active',
}

describe('useMemoryStore', () => {
  beforeEach(resetAllStores)

  it('초기 상태는 빈 목록 + 선택 없음', () => {
    const s = useMemoryStore.getState()
    expect(s.entriesByNpc).toEqual([])
    expect(s.rumors).toEqual([])
    expect(s.selectedNpcId).toBeNull()
    expect(s.layerFilter).toBe('all')
    expect(s.loading).toBe(false)
  })

  it('setEntries로 기억 목록 설정', () => {
    useMemoryStore.getState().setEntries([mockMemoryEntry])
    expect(useMemoryStore.getState().entriesByNpc).toHaveLength(1)
    expect(useMemoryStore.getState().entriesByNpc[0].id).toBe('mem-000001')
  })

  it('setRumors로 소문 목록 설정', () => {
    useMemoryStore.getState().setRumors([mockRumor])
    expect(useMemoryStore.getState().rumors).toHaveLength(1)
    expect(useMemoryStore.getState().rumors[0].status).toBe('active')
  })

  it('setSelectedNpcId 후 null로 리셋 가능', () => {
    useMemoryStore.getState().setSelectedNpcId('mu_baek')
    expect(useMemoryStore.getState().selectedNpcId).toBe('mu_baek')
    useMemoryStore.getState().setSelectedNpcId(null)
    expect(useMemoryStore.getState().selectedNpcId).toBeNull()
  })

  it('setLayerFilter로 A/B/all 전환', () => {
    useMemoryStore.getState().setLayerFilter('A')
    expect(useMemoryStore.getState().layerFilter).toBe('A')
    useMemoryStore.getState().setLayerFilter('B')
    expect(useMemoryStore.getState().layerFilter).toBe('B')
    useMemoryStore.getState().setLayerFilter('all')
    expect(useMemoryStore.getState().layerFilter).toBe('all')
  })

  it('clear() 전체 리셋', () => {
    const s = useMemoryStore.getState()
    s.setEntries([mockMemoryEntry])
    s.setRumors([mockRumor])
    s.setSelectedNpcId('mu_baek')
    s.setLayerFilter('A')
    s.clear()
    const after = useMemoryStore.getState()
    expect(after.entriesByNpc).toEqual([])
    expect(after.rumors).toEqual([])
    expect(after.selectedNpcId).toBeNull()
    expect(after.layerFilter).toBe('all')
  })

  // Step E3.1 — Topic 모드 상태
  it('setMode / setTopicEntries / setSelectedTopic', () => {
    const s = useMemoryStore.getState()
    expect(s.mode).toBe('npc')
    expect(s.selectedTopic).toBeNull()
    expect(s.topicEntries).toEqual([])

    s.setMode('topic')
    s.setSelectedTopic('sect:leader')
    s.setTopicEntries([mockMemoryEntry])

    const after = useMemoryStore.getState()
    expect(after.mode).toBe('topic')
    expect(after.selectedTopic).toBe('sect:leader')
    expect(after.topicEntries).toHaveLength(1)
  })

  it('clear()가 topic 상태도 리셋', () => {
    const s = useMemoryStore.getState()
    s.setMode('topic')
    s.setSelectedTopic('sect:leader')
    s.setTopicEntries([mockMemoryEntry])
    s.clear()

    const after = useMemoryStore.getState()
    expect(after.mode).toBe('npc')
    expect(after.selectedTopic).toBeNull()
    expect(after.topicEntries).toEqual([])
  })
})
