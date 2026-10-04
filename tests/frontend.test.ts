import { describe, expect, it } from 'vitest'
import { decodeAudioPacket } from '../src/protocol'
import { shortcutFor } from '../src/shortcuts'
import { dragViewport, thumbGeometry } from '../src/timeline/viewport'

describe('studio keyboard', () => {
  const key = (code: string, ctrlKey = false, shiftKey = false, altKey = false) =>
    shortcutFor({ code, ctrlKey, shiftKey, altKey, metaKey: false })
  it('uses physical keys, without stealing modified editing shortcuts', () => {
    expect(key('Space')).toBe('play')
    expect(key('KeyE')).toBeNull()
    expect(key('KeyR')).toBe('record')
    expect(key('KeyV')).toBe('select')
    expect(key('KeyX')).toBe('split')
    expect(key('KeyX', true)).toBeNull()
    expect(key('KeyR', false, false, true)).toBeNull()
  })
  it('distinguishes undo, redo, save and save as', () => {
    expect(key('KeyZ', true)).toBe('undo')
    expect(key('KeyZ', true, true)).toBe('redo')
    expect(key('KeyS', true)).toBe('save')
    expect(key('KeyS', true, true)).toBe('saveAs')
  })
})

describe('zoom scrollbar', () => {
  const view = { start: 20, span: 10 }
  it('keeps the centre draggable with a 60px minimum thumb', () => {
    const thumb = thumbGeometry(view, 1000, 600)
    expect(thumb.width).toBe(60)
    const next = dragViewport(view, 1000, 600, 'pan', 54)
    expect(next.span).toBe(10)
    expect(next.start).toBeCloseTo(119)
  })
  it('changes zoom from each edge, keeping the other edge fixed', () => {
    const left = dragViewport(view, 100, 600, 'left', 30)
    expect(left).toEqual({ start: 25, span: 5 })
    expect(left.start + left.span).toBe(30)
    expect(dragViewport(view, 100, 600, 'right', 30)).toEqual({ start: 20, span: 15 })
    expect(dragViewport(view, 100, 600, 'left', 600).span).toBe(2)
    expect(dragViewport(view, 100, 600, 'pan', -600).start).toBe(0)
  })
})

describe('native audio packets', () => {
  const packet = () => {
    const buffer = new ArrayBuffer(32)
    const header = new DataView(buffer)
    header.setUint32(0, 0x504f5653, true)
    header.setUint32(4, 2, true)
    header.setFloat64(8, 12.5, true)
    new Float32Array(buffer, 16).set([0.1, -0.2, 0.3, -0.4])
    return buffer
  }
  it('decodes position and interleaved stereo, independently of the source buffer', () => {
    const buffer = packet()
    const result = decodeAudioPacket(buffer)
    expect(result.position).toBe(12.5)
    expect(result.samples.length).toBe(4)
    new Float32Array(buffer, 16)[0] = 1
    expect(result.samples[0]).toBeCloseTo(0.1)
  })
  it('rejects truncated or mismatched packets before playback', () => {
    expect(() => decodeAudioPacket(new ArrayBuffer(8))).toThrow()
    const buffer = packet()
    new DataView(buffer).setUint32(4, 1000, true)
    expect(() => decodeAudioPacket(buffer)).toThrow()
  })
})
