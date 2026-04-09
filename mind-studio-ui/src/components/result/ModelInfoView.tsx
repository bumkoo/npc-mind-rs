import type { LlmModelInfo } from '../../types'

interface ModelInfoViewProps {
  info: LlmModelInfo | null
}

export default function ModelInfoView({ info }: ModelInfoViewProps) {
  if (!info) return <div className="text-gray-500 italic p-4">LLM 모델 정보가 없습니다.</div>

  const renderItem = (label: string, value: unknown) => {
    if (value === null || value === undefined) return null
    const displayValue = Array.isArray(value) ? JSON.stringify(value) : String(value)
    return (
      <div className="flex justify-between border-b border-gray-100 py-2 text-sm">
        <span className="text-gray-600 font-medium whitespace-nowrap mr-4">{label} :</span>
        <span className="text-gray-800 break-all text-right">{displayValue}</span>
      </div>
    )
  }

  return (
    <div className="bg-white rounded shadow p-4">
      <h3 className="text-md font-bold text-gray-800 mb-3 border-b pb-2">LLM Environment</h3>
      <div className="space-y-1">
        {renderItem('Provider URL', info.provider_url)}
        {renderItem('Model Name', info.model_name)}
        {renderItem('Temperature', info.temperature)}
        {renderItem('Max Tokens', info.max_tokens)}
        {renderItem('Top P', info.top_p)}
        {renderItem('Frequency Penalty', info.frequency_penalty)}
        {renderItem('Presence Penalty', info.presence_penalty)}
        {renderItem('Stop Sequences', info.stop_sequences)}
        {renderItem('Seed', info.seed)}
      </div>
    </div>
  )
}
