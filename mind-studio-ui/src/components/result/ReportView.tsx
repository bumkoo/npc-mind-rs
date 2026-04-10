import { useState, useEffect, useMemo } from 'react'
import { marked } from 'marked'

// Configure marked to sanitize output
marked.setOptions({ breaks: true })

interface ReportViewProps {
  content: string
  onUpdate: (content: string) => void
  isReadOnly?: boolean
}

function sanitizeHtml(html: string): string {
  const div = document.createElement('div')
  div.innerHTML = html
  // Remove script tags and event handlers
  div.querySelectorAll('script, iframe, object, embed, link[rel="import"]').forEach((el) => el.remove())
  div.querySelectorAll('*').forEach((el) => {
    for (const attr of [...el.attributes]) {
      if (attr.name.startsWith('on') || attr.value.startsWith('javascript:')) {
        el.removeAttribute(attr.name)
      }
    }
  })
  return div.innerHTML
}

export default function ReportView({ content, onUpdate, isReadOnly }: ReportViewProps) {
  const [editMode, setEditMode] = useState(false)
  const [text, setText] = useState(content || '')

  useEffect(() => {
    setText(content || '')
  }, [content])

  const html = useMemo(
    () => sanitizeHtml(marked.parse(text || '*보고서 내용이 없습니다.*') as string),
    [text],
  )

  if (!editMode || isReadOnly) {
    return (
      <div className="report-container">
        {!isReadOnly && (
          <button
            className="btn small"
            style={{ float: 'right' }}
            onClick={() => setEditMode(true)}
          >
            편집
          </button>
        )}
        <div dangerouslySetInnerHTML={{ __html: html }} />
      </div>
    )
  }

  return (
    <div
      style={{ display: 'flex', flexDirection: 'column', gap: 8 }}
    >
      <textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        rows={15}
        style={{
          width: '100%',
          fontFamily: 'monospace',
          fontSize: 12,
          padding: 8,
          background: '#1a1a1a',
          color: '#ccc',
          border: '1px solid var(--border)',
        }}
        placeholder="마크다운 형식으로 보고서를 작성하세요..."
      />
      <div
        style={{
          display: 'flex',
          gap: 8,
          justifyContent: 'flex-end',
        }}
      >
        <button
          className="btn small"
          onClick={() => setEditMode(false)}
        >
          취소
        </button>
        <button
          className="btn primary small"
          onClick={() => {
            onUpdate(text)
            setEditMode(false)
          }}
        >
          저장
        </button>
      </div>
    </div>
  )
}
