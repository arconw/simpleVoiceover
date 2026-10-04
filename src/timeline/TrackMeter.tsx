import { memo, useEffect, useRef } from 'react'
import { clamp } from './viewport'
export default memo(function TrackMeter({
  id,
  getLevel,
}: {
  id: string
  getLevel: (id: string) => number
}) {
  const ref = useRef<HTMLSpanElement>(null)
  useEffect(() => {
    let frame = 0
    let previous = 0
    const draw = (time: number) => {
      if (time - previous > 32 && ref.current) {
        const level = clamp(getLevel(id), 0, 1)
        ref.current.style.transform = `scaleX(${level})`
        ref.current.style.background = level > 0.94 ? '#f0887f' : 'var(--tl-track-color)'
        previous = time
      }
      frame = requestAnimationFrame(draw)
    }
    frame = requestAnimationFrame(draw)
    return () => cancelAnimationFrame(frame)
  }, [id, getLevel])
  return (
    <span className="tl-meter" aria-hidden="true">
      <span ref={ref} />
    </span>
  )
})
