import { t } from './i18n'
import { AudioLines, RotateCcw, Save, SlidersHorizontal } from 'lucide-react'
import { useId, useState } from 'react'
import { type EffectSettings, type Track } from './types'
import { builtInPresets, matchingPreset, type EffectPreset } from './effectPresets'
import ParameterHelp from './components/ParameterHelp'
import type { ParameterKind } from './parameterCurves'

interface Props {
  track: Track
  onChange: (patch: Partial<Track>) => void
  presets: EffectPreset[]
  onSavePreset: (name: string, track: Track) => Promise<boolean>
  operationPending: boolean
}

export function Slider({
  label,
  value,
  min,
  max,
  step = 1,
  unit = '',
  onChange,
  help,
}: {
  label: string
  value: number
  min: number
  max: number
  step?: number
  unit?: string
  onChange: (value: number) => void
  help: ParameterKind
}) {
  const id = useId()
  return (
    <div className="parameter">
      <span>
        <label htmlFor={id}>{label}</label>
        <ParameterHelp kind={help} label={label} value={value} min={min} max={max} />
        <output>
          {Number(value.toFixed(2))}
          {unit && ` ${unit}`}
        </output>
      </span>
      <input
        id={id}
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </div>
  )
}

export default function EffectsPanel({
  track,
  onChange,
  presets,
  onSavePreset,
  operationPending,
}: Props) {
  const [preferredPreset, setPreferredPreset] = useState({ trackId: track.id, id: 'natural' })
  const [saving, setSaving] = useState(false)
  const [name, setName] = useState('')
  const allPresets = [...builtInPresets, ...presets]
  const selected = matchingPreset(
    track,
    allPresets,
    preferredPreset.trackId === track.id ? preferredPreset.id : undefined,
  )
  const applyPreset = (id = selected?.id ?? preferredPreset.id) => {
    const preset = allPresets.find((entry) => entry.id === id) ?? builtInPresets[0]
    setPreferredPreset({ trackId: track.id, id: preset.id })
    onChange({ effects: { ...preset.effects }, fxBypass: preset.fxBypass })
  }
  const setEffect = (key: Exclude<keyof EffectSettings, 'normalize'>, value: number) =>
    onChange({ effects: { ...track.effects, [key]: value } })
  return (
    <div className="effects-content">
      <div className="panel-heading">
        <span className="eyebrow">{t('effects.heading')}</span>
        <button
          className={`tiny-toggle ${!track.fxBypass ? 'enabled' : ''}`}
          aria-label={t('effects.enableTrack')}
          aria-pressed={!track.fxBypass}
          onClick={() => onChange({ fxBypass: !track.fxBypass })}
        >
          {track.fxBypass ? t('effects.disabled') : t('effects.enabled')}
        </button>
      </div>
      <div className="preset-card">
        <AudioLines size={20} />
        <div>
          <select
            aria-label={t('presets.select')}
            value={selected?.id ?? 'custom'}
            disabled={operationPending}
            onChange={(event) => applyPreset(event.target.value)}
          >
            {!selected && (
              <option value="custom" disabled>
                {t('presets.custom')}
              </option>
            )}
            {builtInPresets.map((preset) => (
              <option key={preset.id} value={preset.id}>
                {t(preset.name)}
              </option>
            ))}
            {presets.map((preset) => (
              <option key={preset.id} value={preset.id}>
                {preset.name}
              </option>
            ))}
          </select>
          <small>{t('presets.resetHint')}</small>
        </div>
        <button
          title={t('effects.reset')}
          aria-label={t('effects.reset')}
          onClick={() => applyPreset()}
          disabled={operationPending}
        >
          <RotateCcw size={15} />
        </button>
      </div>
      <button
        className="text-button preset-save-button"
        onClick={() => setSaving(!saving)}
        disabled={operationPending}
      >
        <Save size={13} />
        {t('presets.saveCurrent')}
      </button>
      {saving && (
        <form
          className="preset-save-form"
          onSubmit={(event) => {
            event.preventDefault()
            void onSavePreset(name.trim(), track).then((saved) => {
              if (saved) {
                setSaving(false)
                setName('')
              }
            })
          }}
        >
          <input
            aria-label={t('presets.name')}
            placeholder={t('presets.name')}
            maxLength={80}
            value={name}
            onChange={(event) => setName(event.target.value)}
            autoFocus
          />
          <small className="hint muted">{t('presets.overwriteHint')}</small>
          <button
            className="button secondary"
            type="submit"
            disabled={!name.trim() || operationPending}
          >
            {t('project.save')}
          </button>
        </form>
      )}
      <div className="effect-section">
        <h3>
          <span>01</span> {t('effects.equalizer')} <SlidersHorizontal size={13} />
        </h3>
        <Slider
          label={t('effects.highpass')}
          help="highpass"
          value={track.effects.highpass}
          min={20}
          max={180}
          unit={t('units.hz')}
          onChange={(v) => setEffect('highpass', v)}
        />
        <Slider
          label={t('effects.warmth')}
          help="lowMid"
          value={track.effects.lowMid}
          min={-6}
          max={6}
          step={0.1}
          unit={t('units.db')}
          onChange={(v) => setEffect('lowMid', v)}
        />
        <Slider
          label={t('effects.presence')}
          help="presence"
          value={track.effects.presence}
          min={-6}
          max={6}
          step={0.1}
          unit={t('units.db')}
          onChange={(v) => setEffect('presence', v)}
        />
        <Slider
          label={t('effects.lowpass')}
          help="lowpass"
          value={track.effects.lowpass}
          min={6000}
          max={20000}
          step={100}
          unit={t('units.hz')}
          onChange={(v) => setEffect('lowpass', v)}
        />
      </div>
      <div className="effect-section">
        <h3>
          <span>02</span> {t('effects.compressor')}{' '}
        </h3>
        <Slider
          label={t('effects.threshold')}
          help="threshold"
          value={track.effects.threshold}
          min={-48}
          max={0}
          unit={t('units.db')}
          onChange={(v) => setEffect('threshold', v)}
        />
        <Slider
          label={t('effects.ratio')}
          help="ratio"
          value={track.effects.ratio}
          min={1}
          max={8}
          step={0.1}
          unit=":1"
          onChange={(v) => setEffect('ratio', v)}
        />
        <div className="two-parameters">
          <Slider
            label={t('effects.attack')}
            help="attack"
            value={track.effects.attack}
            min={1}
            max={80}
            unit={t('units.ms')}
            onChange={(v) => setEffect('attack', v)}
          />
          <Slider
            label={t('effects.release')}
            help="release"
            value={track.effects.release}
            min={50}
            max={500}
            unit={t('units.ms')}
            onChange={(v) => setEffect('release', v)}
          />
        </div>
        <Slider
          label={t('effects.makeup')}
          help="makeup"
          value={track.effects.makeup}
          min={0}
          max={8}
          step={0.1}
          unit={t('units.db')}
          onChange={(v) => setEffect('makeup', v)}
        />
      </div>
      <div className="effect-section">
        <h3>
          <span>03</span> {t('effects.quiet')}{' '}
        </h3>
        <p className="muted hint">{t('effects.quietHint')} </p>
        <Slider
          label={t('effects.gateThreshold')}
          help="gateThreshold"
          value={track.effects.gateThreshold}
          min={-70}
          max={-25}
          unit={t('units.db')}
          onChange={(v) => setEffect('gateThreshold', v)}
        />
        <Slider
          label={t('effects.reduction')}
          help="gateReduction"
          value={track.effects.gateReduction}
          min={0}
          max={12}
          step={0.5}
          unit={t('units.db')}
          onChange={(v) => setEffect('gateReduction', v)}
        />
      </div>
      <div className="effect-section">
        <h3>
          <span>04</span>
          {t('effects.loudness')}
        </h3>
        <label className="loudness-toggle">
          <input
            type="checkbox"
            checked={track.effects.normalize ?? false}
            onChange={(event) =>
              onChange({ effects: { ...track.effects, normalize: event.target.checked } })
            }
          />
          {t('effects.normalize')}
        </label>
        <p className="muted hint">{t('effects.loudnessHint')}</p>
        <Slider
          label={t('effects.targetLufs')}
          help="targetLufs"
          value={track.effects.targetLufs ?? -16}
          min={-24}
          max={-9}
          step={0.5}
          unit="LUFS"
          onChange={(v) => setEffect('targetLufs', v)}
        />
        <Slider
          label={t('effects.truePeak')}
          help="truePeak"
          value={track.effects.truePeak ?? -1.5}
          min={-6}
          max={-0.1}
          step={0.1}
          unit="dBTP"
          onChange={(v) => setEffect('truePeak', v)}
        />
      </div>
      <button className="text-button" onClick={() => applyPreset()} disabled={operationPending}>
        {t('effects.reset')}{' '}
      </button>
      <p className="hint muted">{t('effects.footer')} </p>
    </div>
  )
}
