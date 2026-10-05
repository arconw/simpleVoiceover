export interface ViewWindow {
  start: number
  span: number
}
export type ScrollMode = 'pan' | 'left' | 'right'
export const minimumSpan = 2
export const clamp = (value: number, min: number, max: number) =>
  Math.max(min, Math.min(max, value))

export function zoomViewport(
  view: ViewWindow,
  factor: number,
  maximumSpan: number,
  position: number,
  pointerFraction?: number,
): ViewWindow {
  const span = clamp(view.span * factor, minimumSpan, maximumSpan)
  const fraction =
    pointerFraction !== undefined
      ? clamp(pointerFraction, 0, 1)
      : position >= view.start && position <= view.start + view.span
        ? (position - view.start) / view.span
        : 0.5
  const anchor = view.start + view.span * fraction
  return { start: Math.max(0, anchor - fraction * span), span }
}

export function thumbGeometry(view: ViewWindow, extent: number, width: number) {
  const thumbWidth = Math.min(width, Math.max(60, (width * view.span) / extent))
  const left = extent > view.span ? (view.start / (extent - view.span)) * (width - thumbWidth) : 0
  return { width: thumbWidth, left }
}

export function dragViewport(
  view: ViewWindow,
  extent: number,
  width: number,
  mode: ScrollMode,
  pixels: number,
): ViewWindow {
  if (mode === 'pan') {
    const thumbWidth = thumbGeometry(view, extent, width).width
    const maximum = extent - view.span
    const delta = width > thumbWidth ? (pixels / (width - thumbWidth)) * maximum : 0
    return { ...view, start: clamp(view.start + delta, 0, maximum) }
  }
  const delta = (pixels / width) * extent
  const end = view.start + view.span
  if (mode === 'left') {
    const start = clamp(view.start + delta, 0, end - minimumSpan)
    return { start, span: end - start }
  }
  return { start: view.start, span: clamp(view.span + delta, minimumSpan, extent - view.start) }
}
