import type { FocusSettings } from '../types'

export function makeFocusSettings(focusInfo?: {
  event?: {
    description?: string
    desirability_for_self?: number
    has_other?: boolean
    other_target_id?: string
    desirability_for_other?: number
    prospect?: string
  }
  action?: {
    description?: string
    agent_id?: string
    praiseworthiness?: number
  }
  object?: {
    target_id?: string
    appealingness?: number
  }
}): FocusSettings {
  const ev = focusInfo?.event
  const ac = focusInfo?.action
  const ob = focusInfo?.object
  return {
    hasEvent: !!ev,
    evDesc: ev?.description || '',
    evSelf: ev?.desirability_for_self || 0,
    hasOther: ev?.has_other || false,
    otherTarget: ev?.other_target_id || '',
    otherD: ev?.desirability_for_other || 0,
    prospect: ev?.prospect || '',
    hasAction: !!ac,
    acDesc: ac?.description || '',
    agentId: ac?.agent_id || '',
    pw: ac?.praiseworthiness || 0,
    hasObject: !!ob,
    objTarget: ob?.target_id || '',
    objAp: ob?.appealingness || 0,
  }
}
