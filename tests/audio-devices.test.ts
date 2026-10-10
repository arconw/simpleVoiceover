import { describe, expect, it } from 'vitest'
import {
  browserAudioDevices,
  defaultInputSignature,
  effectiveAudioDevice,
  nativeBrowserInput,
} from '../src/audio/devices'

const device = (kind: MediaDeviceKind, deviceId: string, groupId: string, label = deviceId) =>
  ({ kind, deviceId, groupId, label }) as MediaDeviceInfo

describe('audio device discovery', () => {
  it('maps native selections and changing defaults to WebKit capture identifiers', () => {
    const inputs = [
      { id: 'pulse-a', label: 'Microphone A', isDefault: false },
      { id: 'pulse-b', label: 'Microphone B', isDefault: true },
    ]
    const browser = [
      device('audioinput', 'default', 'a', 'Microphone A'),
      device('audioinput', 'webkit-a', 'a', 'Microphone A'),
      device('audioinput', 'webkit-b', 'b', 'Microphone B'),
    ]
    expect(nativeBrowserInput('', inputs, browser)).toBe('webkit-b')
    expect(nativeBrowserInput('pulse-a', inputs, browser)).toBe('webkit-a')
    expect(nativeBrowserInput('unplugged', inputs, browser)).toBe('webkit-b')
    expect(nativeBrowserInput('pulse-b', inputs, [device('audioinput', 'anonymous', '', '')])).toBe(
      '',
    )
    inputs[0].isDefault = true
    inputs[1].isDefault = false
    expect(nativeBrowserInput('', inputs, browser)).toBe('webkit-a')
  })
  it('resolves system aliases to physical devices and excludes cameras', () => {
    const catalog = browserAudioDevices([
      device('audioinput', 'default', 'mic-b'),
      device('audioinput', 'communications', 'mic-a'),
      device('audioinput', 'mic-a', 'mic-a'),
      device('audioinput', 'mic-b', 'mic-b'),
      device('audiooutput', 'default', 'speaker'),
      device('audiooutput', 'headphones', 'headphones'),
      device('audiooutput', 'speaker', 'speaker'),
      device('videoinput', 'camera', 'camera'),
    ])
    expect(catalog.inputs.map((item) => [item.id, item.isDefault])).toEqual([
      ['mic-a', false],
      ['mic-b', true],
    ])
    expect(catalog.outputs.find((item) => item.isDefault)?.id).toBe('speaker')
    expect(effectiveAudioDevice('mic-a', catalog.inputs)).toBe('mic-a')
    expect(effectiveAudioDevice('disconnected', catalog.inputs)).toBe('')
  })
  it('detects a changed default microphone even when the default alias remains unchanged', () => {
    const previous = [device('audioinput', 'default', 'mic-a', 'Default microphone')]
    const next = [device('audioinput', 'default', 'mic-b', 'Default microphone')]
    expect(defaultInputSignature(previous)).not.toBe(defaultInputSignature(next))
    expect(defaultInputSignature([])).toBe('')
  })
})
