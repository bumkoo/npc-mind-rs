interface SliderProps {
  label: string
  value: number
  onChange: (value: number) => void
  min?: number
  max?: number
  step?: number
}

export default function Slider({
  label,
  value,
  onChange,
  min = -1,
  max = 1,
  step = 0.05,
}: SliderProps) {
  return (
    <div className="slider-row">
      <label>{label}</label>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
      />
      <span className="val">{value.toFixed(2)}</span>
    </div>
  )
}
