import { platformSymbol } from '@/shared/lib/milsymbol'
import { ammoLabel, STALE_AFTER_MS, type PlatformView } from '../model/platform'

export function PlatformCard({ view, now }: { view: PlatformView; now: number }) {
  const { report, lastSeen } = view
  const age = Math.max(0, Math.round((now - lastSeen) / 1000))
  const stale = now - lastSeen > STALE_AFTER_MS

  return (
    <div
      className={`border border-white/12 bg-white/[0.03] p-3 ${stale ? 'opacity-40' : ''}`}
    >
      <div className="flex items-baseline justify-between">
        <span className="flex items-center gap-1.5 text-sm font-bold tracking-widest text-[#80e0ff]">
          <span
            className="mil-inline"
            aria-hidden
            dangerouslySetInnerHTML={{ __html: platformSymbol(16) }}
          />
          {report.name.toUpperCase()}
        </span>
        <span className="text-[10px] text-neutral-500">{report.platform_id.slice(0, 8)}</span>
      </div>
      <div className="mt-2 grid grid-cols-2 gap-1 text-[11px] text-neutral-400">
        <span>
          INTERCEPTORS{' '}
          <span className="text-neutral-100">{ammoLabel(report.interceptors_remaining)}</span>
        </span>
        <span>
          RANGE <span className="text-neutral-100">{(report.reach / 1000).toFixed(1)} km</span>
        </span>
        <span>
          CONTACTS <span className="text-neutral-100">{report.threats.length}</span>
        </span>
        <span>
          {stale ? (
            <span className="text-neutral-100">LINK LOST {age}s</span>
          ) : (
            <span>SEEN {age}s ago</span>
          )}
        </span>
      </div>
    </div>
  )
}
