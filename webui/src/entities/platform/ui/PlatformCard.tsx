import { ammoLabel, STALE_AFTER_MS, type PlatformView } from '../model/platform'

export function PlatformCard({ view, now }: { view: PlatformView; now: number }) {
  const { report, lastSeen } = view
  const age = Math.max(0, Math.round((now - lastSeen) / 1000))
  const stale = now - lastSeen > STALE_AFTER_MS

  return (
    <div
      className={`border border-emerald-400/20 bg-emerald-400/5 p-3 ${stale ? 'opacity-40' : ''}`}
    >
      <div className="flex items-baseline justify-between">
        <span className="text-sm font-bold tracking-widest text-emerald-400">
          ▲ {report.name.toUpperCase()}
        </span>
        <span className="text-[10px] text-slate-500">{report.platform_id.slice(0, 8)}</span>
      </div>
      <div className="mt-2 grid grid-cols-2 gap-1 text-[11px] text-slate-400">
        <span>
          INTERCEPTORS{' '}
          <span className="text-slate-200">{ammoLabel(report.interceptors_remaining)}</span>
        </span>
        <span>
          RANGE <span className="text-slate-200">{(report.reach / 1000).toFixed(1)} km</span>
        </span>
        <span>
          CONTACTS <span className="text-cyan-300">{report.threats.length}</span>
        </span>
        <span>
          {stale ? (
            <span className="text-red-400">LINK LOST {age}s</span>
          ) : (
            <span>SEEN {age}s ago</span>
          )}
        </span>
      </div>
    </div>
  )
}
