import { afterEach, describe, expect, it, vi } from 'vitest'
import { prepareVideoPosition, synchronizeVideo } from '../src/video/playback'

function preview(time = 0) {
  let currentTime = time
  const seek = vi.fn((value: number) => {
    currentTime = value
  })
  const video = {
    seeking: false,
    paused: true,
    readyState: 2,
    playbackRate: 1,
    play: vi.fn().mockResolvedValue(undefined),
    pause: vi.fn(),
    get currentTime() {
      return currentTime
    },
    set currentTime(value: number) {
      seek(value)
    },
  } as unknown as HTMLVideoElement
  return {
    video,
    seek,
    advance: (value: number) => {
      currentTime = value
    },
    showError: vi.fn(),
  }
}

afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
  vi.useRealTimers()
})

describe('video preview synchronization', () => {
  it('prepares a decoded seek before letting transport resume', async () => {
    vi.useFakeTimers()
    vi.stubGlobal('requestAnimationFrame', (callback: () => void) => setTimeout(callback, 16))
    vi.stubGlobal('cancelAnimationFrame', (frame: number) => clearTimeout(frame))
    const { video, showError } = preview()
    Object.assign(video, { seeking: true })
    let ready = false
    const prepared = prepareVideoPosition(video, 4, showError, 1, () => true).then(() => {
      ready = true
    })
    await vi.advanceTimersByTimeAsync(80)
    expect(ready).toBe(false)
    Object.assign(video, { seeking: false })
    await vi.advanceTimersByTimeAsync(16)
    await prepared
    expect(video.currentTime).toBe(4)
    expect(video.play).not.toHaveBeenCalled()
    expect(vi.getTimerCount()).toBe(0)
  })
  it('cancels preparation when a newer request supersedes it', async () => {
    vi.useFakeTimers()
    vi.stubGlobal('requestAnimationFrame', (callback: () => void) => setTimeout(callback, 16))
    vi.stubGlobal('cancelAnimationFrame', (frame: number) => clearTimeout(frame))
    const { video, seek, showError } = preview()
    Object.assign(video, { seeking: true })
    let current = true
    const prepared = prepareVideoPosition(video, 8, showError, 1, () => current)
    current = false
    await vi.advanceTimersByTimeAsync(16)
    await prepared
    expect(seek).not.toHaveBeenCalled()
    expect(vi.getTimerCount()).toBe(0)
  })
  it('bounds preparation even if a hidden window stops animation frames', async () => {
    vi.useFakeTimers()
    vi.stubGlobal('requestAnimationFrame', () => 1)
    vi.stubGlobal('cancelAnimationFrame', vi.fn())
    const { video, showError } = preview()
    Object.assign(video, { seeking: true })
    const prepared = prepareVideoPosition(video, 4, showError, 1, () => true)
    await vi.advanceTimersByTimeAsync(5000)
    await prepared
    expect(cancelAnimationFrame).toHaveBeenCalledWith(1)
    expect(vi.getTimerCount()).toBe(0)
  })
  it('holds the final video frame when recording continues past its duration', () => {
    const { video, seek, showError } = preview(1.9)
    Object.assign(video, { duration: 2, paused: false })
    synchronizeVideo(video, 3, true, showError)
    expect(seek).toHaveBeenLastCalledWith(1.999)
    expect(video.pause).toHaveBeenCalledOnce()
    expect(video.play).not.toHaveBeenCalled()
  })
  it('preserves frame delivery through a small stable playback offset', () => {
    const { video, advance, seek, showError } = preview()
    Object.assign(video, { paused: false })
    for (let tick = 1; tick <= 50; tick++) {
      advance(tick / 10)
      synchronizeVideo(video, tick / 10 + 0.2, true, showError)
    }
    expect(seek).not.toHaveBeenCalled()
  })
  it('recovers from a slow decoder without repeatedly restarting its corrective seek', () => {
    let currentTime = 0
    let completion = 0
    let now = 0
    const positions: number[] = []
    vi.spyOn(performance, 'now').mockImplementation(() => now)
    const video = {
      paused: false,
      readyState: 2,
      seeking: false,
      playbackRate: 1,
      get currentTime() {
        return currentTime
      },
      set currentTime(value: number) {
        currentTime = value
        this.seeking = true
        completion = now + 600
        positions.push(value)
      },
      play: vi.fn().mockResolvedValue(undefined),
      pause: vi.fn(),
    } as unknown as HTMLVideoElement
    const showError = vi.fn()
    for (let tick = 0; tick <= 60; tick++) {
      now = tick * 100
      if (video.seeking && now >= completion) Object.assign(video, { seeking: false })
      else if (!video.seeking && tick) currentTime += 0.1 * video.playbackRate
      synchronizeVideo(video, 1 + now / 1000, true, showError)
    }
    expect(positions.length).toBeLessThanOrEqual(2)
    expect(Math.abs(currentTime - 7)).toBeLessThan(0.12)
    expect(showError).not.toHaveBeenCalled()
  })
  it('keeps synchronization active when decoded playback advances before play resolves', () => {
    const { video, advance, seek, showError } = preview()
    vi.mocked(video.play).mockReturnValue(new Promise(() => {}))
    synchronizeVideo(video, 0, true, showError)
    Object.assign(video, { paused: false })
    advance(0.2)
    synchronizeVideo(video, 0.2, true, showError)
    advance(0.25)
    synchronizeVideo(video, 1, true, showError)
    expect(seek).toHaveBeenCalled()
  })
  it('restarts after an interrupted play promise and waits for seek completion', () => {
    const { video, showError } = preview()
    vi.mocked(video.play).mockReturnValue(new Promise(() => {}))
    synchronizeVideo(video, 0, true, showError)
    synchronizeVideo(video, 2, false, showError, 1)
    Object.assign(video, { seeking: true })
    synchronizeVideo(video, 2, true, showError, 1)
    expect(video.play).toHaveBeenCalledOnce()
    Object.assign(video, { seeking: false })
    synchronizeVideo(video, 2, true, showError, 1)
    expect(video.play).toHaveBeenCalledTimes(2)
  })
  it('plays through short decoder delays without repeated corrective seeks', () => {
    const { video, seek, showError } = preview(1)
    synchronizeVideo(video, 1.2, true, showError)
    synchronizeVideo(video, 1.3, true, showError)
    synchronizeVideo(video, 1.4, true, showError)
    expect(seek).not.toHaveBeenCalled()
    expect(video.play).toHaveBeenCalledOnce()
  })
  it('waits for an active seek and limits automatic corrections while playing', () => {
    const { video, seek, showError } = preview()
    Object.assign(video, { paused: false })
    let now = 0
    vi.spyOn(performance, 'now').mockImplementation(() => now)
    synchronizeVideo(video, 1, true, showError)
    Object.assign(video, { seeking: true, paused: false })
    synchronizeVideo(video, 1.1, true, showError)
    synchronizeVideo(video, 1.4, true, showError)
    expect(seek).toHaveBeenCalledOnce()
    Object.assign(video, { seeking: false })
    now = 200
    synchronizeVideo(video, 1.7, true, showError)
    expect(seek).toHaveBeenCalledOnce()
    now = 1300
    synchronizeVideo(video, 3, true, showError)
    expect(seek).toHaveBeenCalledTimes(2)
  })
  it('waits for playback startup before correcting drift', async () => {
    const { video, seek, showError } = preview(1)
    let resolve!: () => void
    vi.mocked(video.play).mockImplementation(
      () =>
        new Promise<void>((complete) => {
          resolve = complete
        }),
    )
    synchronizeVideo(video, 1.2, true, showError)
    Object.assign(video, { paused: false })
    synchronizeVideo(video, 1.4, true, showError)
    expect(seek).not.toHaveBeenCalled()
    resolve()
    await new Promise((complete) => setTimeout(complete, 0))
    synchronizeVideo(video, 1.45, true, showError)
    expect(seek).toHaveBeenCalled()
  })
  it('waits for decoded data before correcting a playing preview', () => {
    const { video, seek, showError } = preview(1)
    Object.assign(video, { paused: false, readyState: 1 })
    synchronizeVideo(video, 1.2, true, showError)
    synchronizeVideo(video, 1.4, true, showError)
    expect(seek).not.toHaveBeenCalled()
    Object.assign(video, { readyState: 2 })
    synchronizeVideo(video, 1.4, true, showError)
    expect(seek).toHaveBeenCalled()
  })
  it('coalesces timeline jumps until the active decoder seek finishes', () => {
    const { video, seek, showError } = preview()
    synchronizeVideo(video, 0, true, showError)
    Object.assign(video, { seeking: true })
    synchronizeVideo(video, 30, true, showError)
    synchronizeVideo(video, 30.03, true, showError)
    synchronizeVideo(video, 10, true, showError)
    expect(seek).not.toHaveBeenCalled()
    Object.assign(video, { seeking: false })
    synchronizeVideo(video, 10.03, true, showError)
    expect(seek).toHaveBeenCalledOnce()
    expect(seek).toHaveBeenCalledWith(10.03)
  })
  it('bounds compensation after a stalled decoder and positions user seeks exactly', () => {
    const { video, seek, advance, showError } = preview()
    Object.assign(video, { paused: false })
    let now = 0
    vi.spyOn(performance, 'now').mockImplementation(() => now)
    synchronizeVideo(video, 1, true, showError)
    Object.assign(video, { seeking: true })
    now = 2000
    synchronizeVideo(video, 10, true, showError)
    Object.assign(video, { seeking: false })
    now = 2010
    synchronizeVideo(video, 10.02, true, showError)
    now = 3100
    advance(10.5)
    synchronizeVideo(video, 12, true, showError)
    expect(seek.mock.calls.at(-1)?.[0]).toBeCloseTo(13.5)
    synchronizeVideo(video, 12.1, true, showError, 1)
    expect(seek).toHaveBeenLastCalledWith(12.1)
  })
  it('finishes positioning a paused preview when the decoder reports seek completion', () => {
    const { video, seek, showError } = preview()
    synchronizeVideo(video, 1, false, showError)
    Object.assign(video, { seeking: true })
    synchronizeVideo(video, 1.1, false, showError)
    expect(seek).toHaveBeenCalledOnce()
    Object.assign(video, { seeking: false })
    synchronizeVideo(video, 1.1, false, showError)
    expect(seek).toHaveBeenLastCalledWith(1.1)
    expect(video.play).not.toHaveBeenCalled()
  })
  it('reports playback failures once and permits retry after stopping', async () => {
    const { video, showError } = preview()
    const error = new Error('Unsupported video')
    vi.mocked(video.play).mockRejectedValue(error)
    synchronizeVideo(video, 0, true, showError)
    await vi.waitFor(() => expect(showError).toHaveBeenCalledOnce())
    synchronizeVideo(video, 0.03, true, showError)
    expect(video.play).toHaveBeenCalledOnce()
    synchronizeVideo(video, 0.03, false, showError)
    vi.mocked(video.play).mockResolvedValue(undefined)
    synchronizeVideo(video, 0.03, true, showError)
    expect(video.play).toHaveBeenCalledTimes(2)
  })
  it('does not report expected cancellation when pausing pending playback', async () => {
    const { video, showError } = preview()
    let reject!: (reason: Error) => void
    vi.mocked(video.play).mockImplementation(
      () =>
        new Promise<void>((_, fail) => {
          reject = fail
        }),
    )
    synchronizeVideo(video, 0, true, showError)
    Object.assign(video, { paused: false })
    synchronizeVideo(video, 0, false, showError)
    expect(video.pause).toHaveBeenCalledOnce()
    reject(new Error('Interrupted'))
    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(showError).not.toHaveBeenCalled()
  })
})
