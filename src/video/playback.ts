interface PlaybackState {
  playing: boolean
  pending: boolean
  playRequest: number
  playPosition: number
  failed: boolean
  target: number | null
  correctedAt: number
  seekPosition: number | null
  seekDelay: number
  revision: number
  requestedSeek: boolean
}

const states = new WeakMap<HTMLVideoElement, PlaybackState>()

function previewTime(video: HTMLVideoElement, desired: number) {
  return Number.isFinite(video.duration) && video.duration > 0
    ? Math.min(desired, Math.max(0, video.duration - 0.001))
    : desired
}

export function prepareVideoPosition(
  video: HTMLVideoElement,
  desired: number,
  showError: (reason: unknown) => void,
  revision: number,
  current: () => boolean,
) {
  return new Promise<void>((resolve) => {
    let frame: number | null = null
    const finish = () => {
      if (frame !== null) cancelAnimationFrame(frame)
      clearTimeout(timeout)
      resolve()
    }
    const timeout = setTimeout(finish, 5000)
    const update = () => {
      if (!current() || video.error) {
        finish()
        return
      }
      synchronizeVideo(video, desired, false, showError, revision)
      if (
        !video.seeking &&
        video.readyState >= 2 &&
        Math.abs(video.currentTime - previewTime(video, desired)) < 0.02
      ) {
        finish()
        return
      }
      frame = requestAnimationFrame(update)
    }
    update()
  })
}

export function synchronizeVideo(
  video: HTMLVideoElement,
  desired: number,
  playing: boolean,
  showError: (reason: unknown) => void,
  revision = 0,
) {
  if (Number.isFinite(video.duration) && video.duration > 0 && desired >= video.duration - 0.001)
    playing = false
  desired = previewTime(video, desired)
  let state = states.get(video)
  if (!state) {
    state = {
      playing: false,
      pending: false,
      playRequest: 0,
      playPosition: video.currentTime,
      failed: false,
      target: null,
      correctedAt: -Infinity,
      seekPosition: null,
      seekDelay: 0,
      revision,
      requestedSeek: false,
    }
    states.set(video, state)
  }
  if (state.playing !== playing) state.failed = false
  state.playing = playing
  const jumped =
    state.target !== null &&
    (desired < state.target - 0.1 || (!playing && Math.abs(desired - state.target) > 0.5))
  if (jumped || revision !== state.revision) {
    state.requestedSeek = true
  }
  const drift = Math.abs(video.currentTime - desired)
  const now = performance.now()
  if (
    state.pending &&
    !video.paused &&
    !video.seeking &&
    video.readyState >= 2 &&
    Math.abs(video.currentTime - state.playPosition) > 0.025
  )
    state.pending = false
  if (state.seekPosition !== null && !video.seeking) {
    state.seekDelay = Math.min(1.5, Math.max(0, desired - state.seekPosition))
    state.seekPosition = null
    state.correctedAt = now
  }
  if (
    !video.seeking &&
    (state.requestedSeek ||
      ((!playing || (!video.paused && !state.pending && video.readyState >= 2)) &&
        drift > (playing ? 0.25 : 0.01) &&
        (!playing || now - state.correctedAt >= 1000)))
  ) {
    video.currentTime = Math.max(
      0,
      desired + (playing && !state.requestedSeek ? state.seekDelay : 0),
    )
    state.seekPosition = desired
    state.correctedAt = now
    state.requestedSeek = false
  }
  state.target = desired
  state.revision = revision
  if (!playing) {
    if (state.pending) {
      state.pending = false
      state.playRequest++
    }
    if (!video.paused) video.pause()
    return
  }
  if (video.seeking || !video.paused || state.pending || state.failed) return
  state.pending = true
  state.playPosition = video.currentTime
  const current = state
  const request = ++state.playRequest
  void video
    .play()
    .catch((reason: unknown) => {
      if (
        request !== current.playRequest ||
        !current.playing ||
        (reason instanceof Error && reason.name === 'AbortError')
      )
        return
      current.failed = true
      showError(reason)
    })
    .finally(() => {
      if (request === current.playRequest) current.pending = false
    })
}
