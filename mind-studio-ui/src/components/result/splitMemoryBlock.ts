/**
 * `DialogueAgent.inject_memory_push` → `LocaleMemoryFramer.frame_block` 이 프롬프트에
 * prepend한 "떠오르는 기억" 블록을 분리한다 (Step E2 — 시각화 전용).
 *
 * Framer 출력 포맷 (`locales/{ko,en}.toml [memory.framing.block]`):
 * ```
 * \n# 떠오르는 기억\n[겪음] c1\n[목격] c2\n...\n
 * ```
 * 그 뒤에 원본 system prompt (`[역할: ...]` 등 사각괄호 섹션)가 이어진다.
 *
 * 전략:
 * 1. 프롬프트 선두에서 `# 떠오르는 기억` 또는 `# Recollections` 헤더를 찾는다.
 * 2. 헤더 뒤부터 알려진 Source 라벨 (`[겪음]/[목격]/[전해 들음]/[강호에 떠도는 소문]`·
 *    영문 대응)로 시작하는 라인을 연속으로 수집한다.
 * 3. 첫 번째 비-엔트리 라인이 나오면 거기서 블록 종료.
 *
 * 한계:
 * - Source 라벨이 entry content에 다시 등장할 수 있다(본문이 "[겪음] ..."을 인용하는 경우).
 *   이 경우 연속으로 라벨 라인이 이어지므로 보수적으로 포함. 드물고 허용 가능.
 * - Custom locale이 Source 라벨을 재정의하면 여기서 매칭 못 함 → 헤더만 인식되고
 *   첫 라인이 비-엔트리로 판정되어 memory 블록이 헤더 1줄만 담김. ko/en 빌트인만 지원.
 */
export function splitMemoryBlock(prompt: string): { memory: string | null; rest: string } {
  const headers = [/#\s*떠오르는 기억\s*\n/, /#\s*Recollections\s*\n/]
  // 헤더는 선두 근처(선행 공백·newline 허용)에서만 허용 — 중간에 동일 문자열이 우연히
  // 있더라도 오탐 안 함.
  const leadingSlice = prompt.slice(0, 32)
  let headerEnd = -1
  let headerStart = -1
  for (const re of headers) {
    const m = leadingSlice.match(re)
    if (m && m.index !== undefined) {
      headerStart = m.index
      headerEnd = m.index + m[0].length
      break
    }
  }
  if (headerEnd < 0) return { memory: null, rest: prompt }

  const labelRe = /^\[(?:겪음|목격|전해 들음|강호에 떠도는 소문|Experienced|Witnessed|Heard|Rumor)\]/

  const afterHeader = prompt.slice(headerEnd)
  const lines = afterHeader.split('\n')

  // 연속된 라벨 라인들을 수집. 첫 비-라벨·비-빈줄에서 중단.
  let lastEntryIdx = -1
  for (let i = 0; i < lines.length; i++) {
    const l = lines[i]
    if (labelRe.test(l)) {
      lastEntryIdx = i
      continue
    }
    // 엔트리 다음의 빈 줄(footer)은 블록에 포함 허용 — 단 1줄만.
    if (l === '' && lastEntryIdx === i - 1) {
      continue
    }
    // 첫 번째 비-엔트리·비-footer 라인 → 블록 종료.
    break
  }

  if (lastEntryIdx < 0) {
    // 헤더만 있고 엔트리 없음 — 비정상 상태. memory는 헤더만, rest는 헤더 이후 전체.
    return { memory: prompt.slice(headerStart, headerEnd).trim(), rest: afterHeader }
  }

  const memoryLines = lines.slice(0, lastEntryIdx + 1)
  const memory = (prompt.slice(headerStart, headerEnd) + memoryLines.join('\n')).trim()
  // 블록 종료 후 첫 라인부터 rest. 엔트리 직후의 빈 줄 하나는 footer로 흡수.
  let restStart = lastEntryIdx + 1
  if (lines[restStart] === '') restStart++
  const rest = lines.slice(restStart).join('\n')
  return { memory, rest }
}
