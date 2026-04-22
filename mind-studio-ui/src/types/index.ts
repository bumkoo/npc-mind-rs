// --- NPC ---
export interface Npc {
  id: string
  name: string
  description: string
  // HEXACO facets
  sincerity: number
  fairness: number
  greed_avoidance: number
  modesty: number
  fearfulness: number
  anxiety: number
  dependence: number
  sentimentality: number
  social_self_esteem: number
  social_boldness: number
  sociability: number
  liveliness: number
  forgiveness: number
  gentleness: number
  flexibility: number
  patience: number
  organization: number
  diligence: number
  perfectionism: number
  prudence: number
  aesthetic_appreciation: number
  inquisitiveness: number
  creativity: number
  unconventionality: number
}

// --- Relationship ---
export interface Relationship {
  owner_id: string
  target_id: string
  closeness: number
  trust: number
  power: number
}

// --- GameObject ---
export interface GameObject {
  id: string
  description: string
  category: string | null
}

// --- Emotion ---
export interface Emotion {
  emotion_type: string
  intensity: number
  context?: string
}

// --- PAD ---
export interface Pad {
  pleasure: number
  arousal: number
  dominance: number
}

// --- Situation ---
export interface Situation {
  npc_id?: string
  partner_id?: string
  description?: string
  significance?: number
  event?: SituationEvent
  action?: SituationAction
  object?: SituationObject
  focuses?: FocusInput[]
}

export interface SituationEvent {
  description: string
  desirability_for_self: number
  has_other: boolean
  other_target_id: string
  desirability_for_other: number
  prospect: string
}

export interface SituationAction {
  description: string
  agent_id: string
  praiseworthiness: number
}

export interface SituationObject {
  target_id: string
  appealingness: number
}

export interface FocusInput {
  id: string
  description: string
  trigger: string
  event?: SituationEvent
  action?: SituationAction
  object?: SituationObject
}

// --- Appraise / Stimulus Result ---
export interface AppraiseResult {
  npc_id?: string
  partner_id?: string
  emotions?: Emotion[]
  dominant?: Emotion
  mood?: number
  prompt?: string
  relationship?: Relationship
  trace?: string[]
  beat_changed?: boolean
  active_focus_id?: string
  input_pad?: Pad
  afterDialogue?: boolean
  llm_model?: LlmModelInfo
  [key: string]: unknown
}

// --- Scene ---
export interface SceneFocus {
  id: string
  description: string
  trigger: string
  trigger_display?: string
  is_active: boolean
  test_script?: string[]
  event?: SituationEvent
  action?: SituationAction
  object?: SituationObject
}

export interface SceneInfo {
  has_scene: boolean
  turns?: ScenarioTurn[]
  script_cursor?: number
  focuses?: SceneFocus[]
  significance?: number
  active_focus_id?: string
}

// --- Scenario ---
export interface ScenarioEntry {
  path: string
  label: string
  has_results: boolean
}

export interface ScenarioMeta {
  name: string
  description?: string
  notes?: string[]
}

export interface ScenarioTurn {
  utterance: string
  pad?: Pad
}

// --- Chat ---
export interface ChatMessage {
  role: 'system' | 'user' | 'assistant'
  content: string
  emotions?: Record<string, number> | null
  mood?: number | null
  snapshot?: AppraiseResult | null
  activePrompt?: string
  beat_changed?: boolean
  new_focus?: string | null
  pad?: Pad | null
  trace?: TraceEntry[]
  streaming?: boolean
  llm_model?: LlmModelInfo
}

// --- Turn History ---
export interface TurnHistory {
  action: string
  label: string
  request?: Record<string, unknown>
  response?: Record<string, unknown>
  llm_model?: LlmModelInfo
}

// --- Trace ---
export type TraceEntry = string | { label: string; trace: string[] }

// --- LLM Model Info ---
export interface LlmModelInfo {
  id?: string
  object?: string
  owned_by?: string
  [key: string]: unknown
}

// --- Save Dir Info ---
export interface SaveDirInfo {
  dir: string
  loaded_path: string
  scenario_name: string
  scenario_modified: boolean
  has_turn_history: boolean
  has_existing_results: boolean
}

// --- Modal ---
export type ModalState =
  | { type: 'npc'; data: Npc | null }
  | { type: 'rel'; data: Relationship | null }
  | { type: 'obj'; data: GameObject | null }

// --- Focus Settings (UI helper) ---
export interface FocusSettings {
  hasEvent: boolean
  evDesc: string
  evSelf: number
  hasOther: boolean
  otherTarget: string
  otherD: number
  prospect: string
  hasAction: boolean
  acDesc: string
  agentId: string
  pw: number
  hasObject: boolean
  objTarget: string
  objAp: number
}

// --- Toast ---
export type ToastType = 'info' | 'success' | 'error'

export interface Toast {
  id: number
  msg: string
  type: ToastType
}

export type ToastFn = (msg: string, type?: ToastType) => void

// ---------------------------------------------------------------------------
// Memory / Rumor (Step E2 — Mind Studio 표시 UI)
// ---------------------------------------------------------------------------

/**
 * 소유·접근 범위. Rust `#[serde(tag = "kind", rename_all = "snake_case")]`.
 * Relationship은 `a ≤ b`로 정규화돼 있음.
 */
export type MemoryScope =
  | { kind: 'personal'; npc_id: string }
  | { kind: 'relationship'; a: string; b: string }
  | { kind: 'faction'; faction_id: string }
  | { kind: 'family'; family_id: string }
  | { kind: 'world'; world_id: string }

/** Rust `rename_all = "snake_case"`. */
export type MemorySource = 'experienced' | 'witnessed' | 'heard' | 'rumor'

/** Rust `rename_all = "snake_case"`. `seeded` = 작가 시드, `runtime` = 엔진 파생. */
export type Provenance = 'seeded' | 'runtime'

/** Rust `rename_all = "UPPERCASE"`. */
export type MemoryLayer = 'A' | 'B'

/**
 * Rust `MemoryType` — 기본 derive(Serialize)라 PascalCase 그대로.
 * 구 JSON(`Dialogue`/`Relationship`/`SceneEnd`)은 serde alias로 역호환되지만
 * 서버는 항상 신규 이름으로 내보낸다.
 */
export type MemoryType =
  | 'DialogueTurn'
  | 'RelationshipChange'
  | 'BeatTransition'
  | 'SceneSummary'
  | 'GameEvent'
  | 'WorldEvent'
  | 'FactionKnowledge'
  | 'FamilyFact'

export interface MemoryEntry {
  id: string
  created_seq: number
  event_id: number
  scope: MemoryScope
  source: MemorySource
  provenance: Provenance
  memory_type: MemoryType
  layer: MemoryLayer
  content: string
  topic: string | null
  emotional_context: [number, number, number] | null
  timestamp_ms: number
  last_recalled_at: number | null
  recall_count: number
  origin_chain: string[]
  confidence: number
  acquired_by: string | null
  superseded_by: string | null
  consolidated_into: string | null
  /** grand-fathered Personal-scope 투영값. 신규 UI는 `scope`를 우선 사용. */
  npc_id: string
}

/** Rust `#[serde(rename_all = "snake_case")]`. */
export type RumorStatus = 'active' | 'fading' | 'faded'

/** Rust `RumorOrigin` 기본 derive — PascalCase variant. */
export type RumorOrigin =
  | { Seeded: null } // unit variant는 Rust serde 기본 "Seeded" string이므로 별도 파싱
  | { FromWorldEvent: { event_id: number } }
  | { Authored: { by: string | null } }
  | string // unit variant 직렬화 대응 — 서버 반환 원형 유지

export interface ReachPolicy {
  regions: string[]
  factions: string[]
  npc_ids: string[]
  min_significance: number
}

export interface RumorHop {
  hop_index: number
  content_version: string | null
  recipients: string[]
  spread_at: number
}

export interface RumorDistortion {
  id: string
  parent: string | null
  content: string
  created_at: number
}

export interface Rumor {
  id: string
  topic: string | null
  seed_content: string | null
  origin: RumorOrigin
  reach_policy: ReachPolicy
  hops: RumorHop[]
  distortions: RumorDistortion[]
  created_at: number
  status: RumorStatus
}

/** `GET /api/memory/search|by-npc|by-topic` 공통 응답. */
export interface MemoryListResponse {
  entries: MemoryEntry[]
}

/** `GET /api/memory/canonical/{topic}` 응답. */
export interface CanonicalResponse {
  entry: MemoryEntry | null
}

/** `GET /api/rumors` 응답. */
export interface RumorListResponse {
  rumors: Rumor[]
}
