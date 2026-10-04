import { AudioLines, RotateCcw, SlidersHorizontal } from 'lucide-react'
import { neutralEffects, voiceEffects, type EffectSettings, type Track } from './types'

interface Props {
  track: Track
  onChange: (patch: Partial<Track>) => void
}

export function Slider({
  label,
  value,
  min,
  max,
  step = 1,
  unit = '',
  onChange,
}: {
  label: string
  value: number
  min: number
  max: number
  step?: number
  unit?: string
  onChange: (value: number) => void
}) {
  return (
    <label className="parameter">
      <span>
        {label}
        <output>
          {Number(value.toFixed(2))}
          {unit && ` ${unit}`}
        </output>
      </span>
      <input
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </label>
  )
}

export default function EffectsPanel({ track, onChange }: Props) {
  const setEffect = (key: keyof EffectSettings, value: number) =>
    onChange({ effects: { ...track.effects, [key]: value } })
  return (
    <div className="effects-content">
      <div className="panel-heading">
        <span className="eyebrow">ЦЕПОЧКА ЭФФЕКТОВ</span>
        <button
          className={`tiny-toggle ${!track.fxBypass ? 'enabled' : ''}`}
          aria-label="Включить эффекты дорожки"
          aria-pressed={!track.fxBypass}
          onClick={() => onChange({ fxBypass: !track.fxBypass })}
        >
          {track.fxBypass ? 'Выключена' : 'Включена'}
        </button>
      </div>
      <div className="preset-card">
        <AudioLines size={20} />
        <div>
          <strong>Естественный войс</strong>
          <small>Мягко. Чисто. Твой голос.</small>
        </div>
        <button
          title="Применить пресет Естественный войс"
          aria-label="Применить пресет Естественный войс"
          onClick={() => onChange({ effects: { ...voiceEffects }, fxBypass: false })}
        >
          <RotateCcw size={15} />
        </button>
      </div>
      <div className="effect-section">
        <h3>
          <span>01</span> Эквалайзер <SlidersHorizontal size={13} />
        </h3>
        <Slider
          label="Срез низких"
          value={track.effects.highpass}
          min={20}
          max={180}
          unit="Гц"
          onChange={(v) => setEffect('highpass', v)}
        />
        <Slider
          label="Теплота · 250 Гц"
          value={track.effects.lowMid}
          min={-6}
          max={6}
          step={0.1}
          unit="дБ"
          onChange={(v) => setEffect('lowMid', v)}
        />
        <Slider
          label="Разборчивость · 3,2 кГц"
          value={track.effects.presence}
          min={-6}
          max={6}
          step={0.1}
          unit="дБ"
          onChange={(v) => setEffect('presence', v)}
        />
        <Slider
          label="Срез высоких"
          value={track.effects.lowpass}
          min={6000}
          max={20000}
          step={100}
          unit="Гц"
          onChange={(v) => setEffect('lowpass', v)}
        />
      </div>
      <div className="effect-section">
        <h3>
          <span>02</span> Компрессор
        </h3>
        <Slider
          label="Порог компрессора"
          value={track.effects.threshold}
          min={-48}
          max={0}
          unit="дБ"
          onChange={(v) => setEffect('threshold', v)}
        />
        <Slider
          label="Степень сжатия"
          value={track.effects.ratio}
          min={1}
          max={8}
          step={0.1}
          unit=":1"
          onChange={(v) => setEffect('ratio', v)}
        />
        <div className="two-parameters">
          <Slider
            label="Атака"
            value={track.effects.attack}
            min={1}
            max={80}
            unit="мс"
            onChange={(v) => setEffect('attack', v)}
          />
          <Slider
            label="Восстановление"
            value={track.effects.release}
            min={50}
            max={500}
            unit="мс"
            onChange={(v) => setEffect('release', v)}
          />
        </div>
        <Slider
          label="Усиление после сжатия"
          value={track.effects.makeup}
          min={0}
          max={8}
          step={0.1}
          unit="дБ"
          onChange={(v) => setEffect('makeup', v)}
        />
      </div>
      <div className="effect-section">
        <h3>
          <span>03</span> Тихие участки
        </h3>
        <p className="muted hint">
          Мягко приглушает фон ниже порога. Начни с 3–6 дБ и проверь тихие окончания слов.
        </p>
        <Slider
          label="Порог тихих участков"
          value={track.effects.gateThreshold}
          min={-70}
          max={-25}
          unit="дБ"
          onChange={(v) => setEffect('gateThreshold', v)}
        />
        <Slider
          label="Приглушение"
          value={track.effects.gateReduction}
          min={0}
          max={12}
          step={0.5}
          unit="дБ"
          onChange={(v) => setEffect('gateReduction', v)}
        />
      </div>
      <button
        className="text-button"
        onClick={() => onChange({ effects: { ...neutralEffects }, fxBypass: true })}
      >
        Сбросить обработку
      </button>
      <p className="hint muted">
        EQ и динамика по рецепту VideoLab. Без распознавания речи и нормализации LUFS.
      </p>
    </div>
  )
}
