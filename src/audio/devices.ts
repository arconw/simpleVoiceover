export interface AudioDevice {
  id: string
  label: string
  isDefault: boolean
}

export interface AudioDeviceCatalog {
  inputs: AudioDevice[]
  outputs: AudioDevice[]
  nativeRouting: boolean
  available: boolean
}

export const emptyAudioDevices: AudioDeviceCatalog = {
  inputs: [],
  outputs: [],
  nativeRouting: false,
  available: true,
}

export function browserAudioDevices(devices: MediaDeviceInfo[]): AudioDeviceCatalog {
  const list = (kind: MediaDeviceKind) => {
    const candidates = devices.filter((device) => device.kind === kind)
    const defaultDevice = candidates.find((device) => device.deviceId === 'default')
    const physical = candidates.filter(
      (device) => device.deviceId && !['default', 'communications'].includes(device.deviceId),
    )
    const defaultId =
      physical.find((device) => defaultDevice?.groupId && device.groupId === defaultDevice.groupId)
        ?.deviceId ?? physical[0]?.deviceId
    return physical.map((device) => ({
      id: device.deviceId,
      label: device.label,
      isDefault: device.deviceId === defaultId,
    }))
  }
  return {
    inputs: list('audioinput'),
    outputs: list('audiooutput'),
    nativeRouting: false,
    available: true,
  }
}

export function effectiveAudioDevice(selected: string, devices: AudioDevice[]) {
  return devices.some((device) => device.id === selected) ? selected : ''
}

export function nativeBrowserInput(
  selected: string,
  inputs: AudioDevice[],
  browserDevices: MediaDeviceInfo[],
) {
  const target =
    inputs.find((device) => device.id === selected) ?? inputs.find((device) => device.isDefault)
  return (
    browserDevices.find(
      (device) =>
        device.kind === 'audioinput' &&
        device.deviceId &&
        device.deviceId !== 'default' &&
        target?.label &&
        device.label === target.label,
    )?.deviceId ?? ''
  )
}

export function defaultInputSignature(devices: MediaDeviceInfo[]) {
  const inputs = devices.filter((device) => device.kind === 'audioinput')
  const device = inputs.find((device) => device.deviceId === 'default') ?? inputs[0]
  return device ? JSON.stringify([device.deviceId, device.groupId, device.label]) : ''
}
