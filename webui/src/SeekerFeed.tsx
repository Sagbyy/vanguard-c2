import { useEffect, useRef, useState } from 'react'
import type { FlyingInterceptor, Threat } from './types'

// Onboard-camera clips live in `public/seeker/`. Each in-flight interceptor is
// mapped to one deterministically (by id) so feeds look distinct. Drop your own
// .mp4 files here and extend this list; a missing file falls back to noise.
const CLIPS = ['seeker-1.mp4', 'seeker-2.mp4', 'seeker-3.mp4', 'seeker-4.mp4']
// Interceptor cruise speed (m/s), mirrors INTERCEPTOR_SPEED in engagement.rs.
const INT_SPEED = 800

function hash(s: string): number {
  let h = 0
  for (let i = 0; i < s.length; i++) h = (Math.imul(h, 31) + s.charCodeAt(i)) | 0
  return Math.abs(h)
}

/** Procedural "NO SIGNAL" static — used when a clip is missing or hasn't loaded. */
function Noise() {
  const ref = useRef<HTMLCanvasElement>(null)
  useEffect(() => {
    const cv = ref.current
    const ctx = cv?.getContext('2d')
    if (!cv || !ctx) return
    let raf = 0
    let last = 0
    const draw = (t: number) => {
      raf = requestAnimationFrame(draw)
      if (t - last < 66) return // ~15 fps is plenty for static
      last = t
      const img = ctx.createImageData(cv.width, cv.height)
      for (let i = 0; i < img.data.length; i += 4) {
        const v = (Math.random() * 255) | 0
        img.data[i] = img.data[i + 1] = img.data[i + 2] = v
        img.data[i + 3] = 255
      }
      ctx.putImageData(img, 0, 0)
    }
    raf = requestAnimationFrame(draw)
    return () => cancelAnimationFrame(raf)
  }, [])
  return (
    <canvas ref={ref} width={96} height={60} className="h-full w-full" style={{ imageRendering: 'pixelated' }} />
  )
}

interface Props {
  /** The selected interceptor, or null if it just left the picture (impact/RTB). */
  interceptor: FlyingInterceptor | null
  /** Its current target threat, if still alive. */
  target: Threat | null
  onClose: () => void
}

export function SeekerFeed({ interceptor, target, onClose }: Props) {
  const [failed, setFailed] = useState(false)
  const [clock, setClock] = useState('')
  const [flash, setFlash] = useState(false)
  const wasPresent = useRef(false)

  // Mission timer / REC timestamp.
  useEffect(() => {
    const id = setInterval(() => setClock(new Date().toISOString().slice(11, 23)), 80)
    return () => clearInterval(id)
  }, [])

  // The interceptor leaving the picture = detonation or signal loss: flash, then
  // hold a SIGNAL LOST screen until the operator closes it.
  useEffect(() => {
    if (interceptor) {
      wasPresent.current = true
    } else if (wasPresent.current) {
      wasPresent.current = false
      setFlash(true)
      const id = setTimeout(() => setFlash(false), 320)
      return () => clearTimeout(id)
    }
  }, [interceptor])

  const lost = !interceptor
  const diverting = interceptor?.diverting ?? false
  const range = interceptor && target ? Math.hypot(
    interceptor.position.x - target.position.x,
    interceptor.position.y - target.position.y,
  ) : null
  const tti = range != null ? range / INT_SPEED : null
  // Lock tightens as the interceptor closes (drives box size + shake).
  const lock = tti != null ? Math.max(0, Math.min(1, 1 - tti / 12)) : 0
  const accent = lost ? '#ff3b4d' : diverting ? '#ffb020' : '#7df9ff'
  const status = lost ? 'SIGNAL LOST' : diverting ? 'ABORT · RTB' : range != null ? 'LOCK' : 'ACQUIRING'

  const clip = interceptor ? CLIPS[hash(interceptor.id) % CLIPS.length] : null
  const boxSize = 120 - lock * 64 // px, shrinks onto the target as it closes
  const shake = lock > 0.75 ? (lock - 0.75) * 8 : 0

  return (
    <div
      className="pointer-events-auto absolute right-3 top-3 w-[340px] overflow-hidden border bg-black/90 shadow-2xl"
      style={{ borderColor: `${accent}66` }}
    >
      <div className="relative aspect-video w-full overflow-hidden bg-black">
        {/* Video (or procedural static fallback). */}
        {clip && !failed && !lost ? (
          <video
            key={clip}
            src={`${import.meta.env.BASE_URL}seeker/${clip}`}
            autoPlay
            loop
            muted
            playsInline
            onError={() => setFailed(true)}
            className="h-full w-full object-cover"
            style={{ transform: shake ? `translate(${(Math.random() - 0.5) * shake}px, ${(Math.random() - 0.5) * shake}px)` : undefined }}
          />
        ) : (
          <Noise />
        )}

        {/* CRT scanlines + vignette. */}
        <div
          className="pointer-events-none absolute inset-0"
          style={{
            background:
              'repeating-linear-gradient(0deg, rgba(0,0,0,0.25) 0px, rgba(0,0,0,0.25) 1px, transparent 2px, transparent 3px)',
          }}
        />
        <div
          className="pointer-events-none absolute inset-0"
          style={{ boxShadow: 'inset 0 0 60px 10px rgba(0,0,0,0.85)' }}
        />

        {/* HUD overlay. */}
        {!lost && (
          <>
            {/* Corner brackets. */}
            {[
              'left-2 top-2 border-l-2 border-t-2',
              'right-2 top-2 border-r-2 border-t-2',
              'left-2 bottom-2 border-l-2 border-b-2',
              'right-2 bottom-2 border-r-2 border-b-2',
            ].map((c) => (
              <div key={c} className={`absolute h-4 w-4 ${c}`} style={{ borderColor: accent }} />
            ))}

            {/* Center crosshair. */}
            <div className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2" style={{ color: accent }}>
              <div className="absolute h-px w-6 -translate-x-1/2 -translate-y-3" style={{ background: accent }} />
              <div className="absolute h-px w-6 -translate-x-1/2 translate-y-3" style={{ background: accent }} />
              <div className="absolute w-px h-6 -translate-y-1/2 -translate-x-3" style={{ background: accent }} />
              <div className="absolute w-px h-6 -translate-y-1/2 translate-x-3" style={{ background: accent }} />
            </div>

            {/* Target lock box (only when tracking a live target). */}
            {!diverting && range != null && (
              <div
                className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 border-2"
                style={{
                  width: `${boxSize}px`,
                  height: `${boxSize}px`,
                  borderColor: accent,
                  borderStyle: 'dashed',
                  opacity: 0.9,
                }}
              />
            )}
          </>
        )}

        {/* Top bar: REC + interceptor id + timestamp. */}
        <div className="absolute left-2 right-2 top-2 flex items-center justify-between font-mono text-[10px]" style={{ color: accent }}>
          <span className="flex items-center gap-1">
            <span className="h-2 w-2 animate-pulse rounded-full" style={{ background: '#ff3b4d' }} />
            REC {interceptor ? interceptor.id.slice(0, 6).toUpperCase() : '------'}
          </span>
          <span>{clock}Z</span>
        </div>

        {/* Bottom telemetry bar. */}
        <div className="absolute bottom-2 left-2 right-2 flex items-end justify-between font-mono text-[10px]" style={{ color: accent }}>
          <div className="leading-tight">
            <div>SEEKER · CAM</div>
            <div className="opacity-80">
              TGT {target ? target.id.slice(0, 6).toUpperCase() : '------'}
            </div>
          </div>
          <div className="text-right leading-tight">
            <div>RNG {range != null ? `${(range / 1000).toFixed(2)} km` : '--'}</div>
            <div>TTI {tti != null ? `${tti.toFixed(1)} s` : '--'}</div>
          </div>
        </div>

        {/* Status banner (center) when locked / aborting / lost. */}
        <div
          className="absolute left-1/2 top-[18%] -translate-x-1/2 border px-2 py-0.5 font-mono text-[10px] font-bold tracking-widest"
          style={{ color: accent, borderColor: `${accent}88`, background: '#00000088' }}
        >
          {status}
        </div>

        {/* Detonation / loss flash. */}
        {flash && <div className="absolute inset-0 bg-white" style={{ animation: 'none' }} />}
      </div>

      {/* Footer: title + close. */}
      <div className="flex items-center justify-between border-t px-2 py-1 font-mono text-[10px] text-slate-400" style={{ borderColor: `${accent}33` }}>
        <span style={{ color: accent }}>◉ ONBOARD FEED</span>
        <button type="button" onClick={onClose} className="text-slate-500 hover:text-slate-200">
          ✕ CLOSE
        </button>
      </div>
    </div>
  )
}
