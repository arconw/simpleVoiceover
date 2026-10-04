import { memo, useEffect, useRef } from 'react'
import type { MediaAsset } from '../types'
export default memo(function Waveform({
  asset,
  offset,
  duration,
  width,
  color,
}: {
  asset?: MediaAsset
  offset: number
  duration: number
  width: number
  color: string
}) {
  const ref = useRef<HTMLCanvasElement>(null)
  useEffect(() => {
    const canvas = ref.current
    if (!canvas || !asset || width < 1 || !asset.waveform.length) return
    const height = 43
    const ratio = window.devicePixelRatio || 1
    const pixels = Math.ceil(width)
    canvas.width = Math.ceil(pixels * ratio)
    canvas.height = Math.ceil(height * ratio)
    const context = canvas.getContext('2d')
    if (!context) return
    context.scale(ratio, ratio)
    context.fillStyle = color
    context.globalAlpha = 0.84
    for (let x = 0; x < pixels; x++) {
      const from = Math.max(
        0,
        Math.floor(((offset + (x * duration) / pixels) * asset.sampleRate) / asset.peakFrames),
      )
      const to = Math.min(
        asset.waveform.length,
        Math.max(
          from + 1,
          Math.ceil(
            ((offset + ((x + 1) * duration) / pixels) * asset.sampleRate) / asset.peakFrames,
          ),
        ),
      )
      let low = 0
      let high = 0
      for (let i = from; i < to; i++) {
        low = Math.min(low, asset.waveform[i][0])
        high = Math.max(high, asset.waveform[i][1])
      }
      context.fillRect(x, height / 2 - high * 19.5, 1, Math.max(0.7, (high - low) * 19.5))
    }
  }, [asset, offset, duration, width, color])
  return <canvas ref={ref} className="tl-waveform" aria-label="Форма звуковой волны" />
})
