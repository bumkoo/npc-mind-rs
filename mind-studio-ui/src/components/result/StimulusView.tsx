import { useState, useEffect, useRef } from 'react'
import type { Pad } from '../../types'
import Slider from '../common/Slider'

interface StimulusViewProps {
  utterance: string
  onUtteranceChange: (u: string) => void
  initialPad: Pad | undefined
  onApply: (data: { pleasure: number; arousal: number; dominance: number; situation_description: string }) => void
  toast: (msg: string, type?: string) => void
}

export default function StimulusView({ utterance, onUtteranceChange, initialPad, onApply, toast }: StimulusViewProps) {
  const [padP, setPadP] = useState(0)
  const [padA, setPadA] = useState(0)
  const [padD, setPadD] = useState(0)
  const [analyzing, setAnalyzing] = useState(false)
  const [analyzed, setAnalyzed] = useState(false)

  // initialPad가 바뀌면 슬라이더에 반영 (메시지 선택 시)
  const prevPadRef = useRef<Pad | null | undefined>(null)
  useEffect(() => {
    if (initialPad) {
      setPadP(Math.round(initialPad.pleasure * 100) / 100)
      setPadA(Math.round(initialPad.arousal * 100) / 100)
      setPadD(Math.round(initialPad.dominance * 100) / 100)
    } else {
      // 기록된 자극 정보가 없으면 0으로 초기화
      setPadP(0)
      setPadA(0)
      setPadD(0)
    }
    prevPadRef.current = initialPad
  }, [initialPad])

  const doAnalyze = async () => {
    if (!utterance || !utterance.trim()) return
    setAnalyzing(true)
    try {
      const res = await fetch('/api/analyze-utterance', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ utterance: utterance.trim() }),
      })
      if (res.status === 501) {
        toast('임베딩 미지원 — PAD를 수동 입력하세요', 'warn')
        return
      }
      if (!res.ok) {
        toast('분석 실패: ' + (await res.text()), 'error')
        return
      }
      const pad = await res.json()
      const p = Math.round(pad.pleasure * 100) / 100
      const a = Math.round(pad.arousal * 100) / 100
      const d = Math.round(pad.dominance * 100) / 100
      setPadP(p)
      setPadA(a)
      setPadD(d)
      setAnalyzed(true)
    } catch {
      toast('분석 오류', 'error')
    } finally {
      setAnalyzing(false)
    }
  }

  return (
    <div>
      {/* 자극 적용 (상단) */}
      <div className="section">
        <div className="section-header">
          <span className="title" style={{ color: 'var(--warn)' }}>
            자극 적용
          </span>
          {initialPad && (
            <span style={{ fontSize: 10, color: 'var(--fg3)', marginLeft: 8 }}>
              대사 PAD 반영됨
            </span>
          )}
        </div>
        <div className="section-body">
          <Slider label="P (쾌)" value={padP} onChange={setPadP} />
          <Slider label="A (각성)" value={padA} onChange={setPadA} />
          <Slider label="D (지배)" value={padD} onChange={setPadD} />
          <button
            className="btn primary btn-full"
            style={{ marginTop: 6 }}
            onClick={() => onApply({ pleasure: padP, arousal: padA, dominance: padD, situation_description: '' })}
          >
            자극 적용
          </button>
        </div>
      </div>

      {/* 대사 분석 (하단) */}
      <div className="section" style={{ marginTop: 8 }}>
        <div className="section-header">
          <span className="title" style={{ color: 'var(--accent)' }}>
            대사 분석
          </span>
        </div>
        <div className="section-body">
          <label style={{ fontSize: 11, color: 'var(--fg2)' }}>
            상대 대사
          </label>
          <div style={{ display: 'flex', gap: 4, marginBottom: 6 }}>
            <input
              type="text"
              value={utterance}
              onChange={(e) => {
                onUtteranceChange(e.target.value)
                setAnalyzed(false)
              }}
              placeholder="대사를 입력하거나 채팅에서 선택..."
              style={{
                flex: 1,
                background: 'var(--bg3)',
                color: 'var(--fg)',
                border: '1px solid var(--border)',
                borderRadius: 'var(--radius)',
                padding: '4px 8px',
                fontSize: 12,
              }}
            />
            <button
              className="btn small"
              disabled={!utterance || !utterance.trim() || analyzing}
              onClick={doAnalyze}
            >
              {analyzing ? '분석중...' : '분석'}
            </button>
          </div>
          {analyzed && (
            <div style={{
              background: 'var(--bg3)',
              borderRadius: 'var(--radius)',
              padding: '8px 10px',
              fontSize: 12,
              fontFamily: 'monospace',
            }}>
              <div style={{ color: 'var(--fg2)', fontSize: 10, marginBottom: 4 }}>
                분석 결과
              </div>
              <span style={{ color: 'var(--accent2)' }}>P</span> {padP >= 0 ? '+' : ''}{padP.toFixed(2)}
              {'  '}
              <span style={{ color: 'var(--warn)' }}>A</span> {padA >= 0 ? '+' : ''}{padA.toFixed(2)}
              {'  '}
              <span style={{ color: 'var(--purple)' }}>D</span> {padD >= 0 ? '+' : ''}{padD.toFixed(2)}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
