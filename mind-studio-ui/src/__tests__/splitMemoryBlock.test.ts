import { describe, it, expect } from 'vitest'
import { splitMemoryBlock } from '../components/result/splitMemoryBlock'

describe('splitMemoryBlock', () => {
  it('헤더 없으면 전체 프롬프트를 rest로 반환', () => {
    const prompt = '[역할: 무백]\n당신은 검객입니다.'
    const { memory, rest } = splitMemoryBlock(prompt)
    expect(memory).toBeNull()
    expect(rest).toBe(prompt)
  })

  it('빈 문자열은 그대로 통과', () => {
    expect(splitMemoryBlock('')).toEqual({ memory: null, rest: '' })
  })

  it('ko 헤더 + 엔트리 + 사각괄호 섹션이 이어지는 실제 프롬프트 분리', () => {
    // Framer: `\n# 떠오르는 기억\n[겪음] ...\n[목격] ...\n` + 시스템 프롬프트 본문
    const prompt =
      '\n# 떠오르는 기억\n[겪음] 처음 만난 날\n[목격] 장문인의 무예\n[역할: 무백]\n당신은 검객입니다.'
    const { memory, rest } = splitMemoryBlock(prompt)
    expect(memory).toContain('# 떠오르는 기억')
    expect(memory).toContain('[겪음] 처음 만난 날')
    expect(memory).toContain('[목격] 장문인의 무예')
    expect(memory).not.toContain('[역할')
    expect(rest).toBe('[역할: 무백]\n당신은 검객입니다.')
  })

  it('en 헤더 + Source 라벨도 정확히 분리', () => {
    const prompt =
      '\n# Recollections\n[Experienced] First meeting\n[Rumor] Distant gossip\n[Role: Mu Baek]\nYou are a swordsman.'
    const { memory, rest } = splitMemoryBlock(prompt)
    expect(memory).toContain('# Recollections')
    expect(memory).toContain('[Experienced] First meeting')
    expect(memory).toContain('[Rumor] Distant gossip')
    expect(rest).toBe('[Role: Mu Baek]\nYou are a swordsman.')
  })

  it('엔트리 직후의 단일 빈 줄(footer)은 블록에 흡수', () => {
    const prompt = '\n# 떠오르는 기억\n[겪음] 어제의 대화\n\n[역할: 무백]'
    const { memory, rest } = splitMemoryBlock(prompt)
    expect(memory).toContain('[겪음] 어제의 대화')
    expect(rest).toBe('[역할: 무백]')
  })

  it('전해 들음 / 강호에 떠도는 소문 라벨 모두 인식', () => {
    const prompt =
      '# 떠오르는 기억\n[전해 들음] 풍문\n[강호에 떠도는 소문] 누군가의 이야기\n[상황]\n본문'
    const { memory, rest } = splitMemoryBlock(prompt)
    expect(memory).toContain('[전해 들음] 풍문')
    expect(memory).toContain('[강호에 떠도는 소문] 누군가의 이야기')
    expect(rest).toBe('[상황]\n본문')
  })

  it('헤더 뒤 바로 비-엔트리 라인이 오면 memory는 헤더만, rest는 그 이후', () => {
    const prompt = '# 떠오르는 기억\n[역할: 무백]\n본문'
    const { memory, rest } = splitMemoryBlock(prompt)
    expect(memory).toBe('# 떠오르는 기억')
    expect(rest).toBe('[역할: 무백]\n본문')
  })

  it('헤더가 프롬프트 깊숙이 등장하면(선두 아님) 오탐 안 함', () => {
    // 선두 32자 윈도우 바깥.
    const prompt = '이것은 매우 긴 소개 문구입니다 정말로 길어서 32자를 훌쩍 넘어갑니다\n# 떠오르는 기억\n[겪음] x'
    const { memory, rest } = splitMemoryBlock(prompt)
    expect(memory).toBeNull()
    expect(rest).toBe(prompt)
  })

  it('memory 블록의 trim 처리 — 선두 newline 제거', () => {
    const prompt = '\n# 떠오르는 기억\n[겪음] a\n[역할]'
    const { memory } = splitMemoryBlock(prompt)
    expect(memory?.startsWith('#')).toBe(true)
  })

  it('엔트리만 있고 이후 섹션이 없는 경우 rest는 빈 문자열', () => {
    const prompt = '# 떠오르는 기억\n[겪음] 마지막 기억\n'
    const { memory, rest } = splitMemoryBlock(prompt)
    expect(memory).toContain('[겪음] 마지막 기억')
    expect(rest).toBe('')
  })
})
