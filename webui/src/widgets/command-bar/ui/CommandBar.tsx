import type { ConnectionStatus } from '@/shared/api'
import type { Basemap } from '@/shared/config'

const STATUS_STYLE: Record<ConnectionStatus, { label: string; dot: string; text: string }> = {
  connected: { label: 'LINK ESTABLISHED', dot: 'bg-emerald-400', text: 'text-emerald-400' },
  connecting: { label: 'ACQUIRING LINK…', dot: 'bg-amber-400 animate-pulse', text: 'text-amber-400' },
  offline: { label: 'LINK DOWN', dot: 'bg-red-500', text: 'text-red-500' },
}

interface CommandBarProps {
  status: ConnectionStatus
  counts: { real: number; decoy: number; unknown: number }
  neutralized: number
  impacts: number
  platformCount: number
  basemap: Basemap
  onToggleBasemap: () => void
  clock: string
}

export function CommandBar({
  status,
  counts,
  neutralized,
  impacts,
  platformCount,
  basemap,
  onToggleBasemap,
  clock,
}: CommandBarProps) {
  const statusStyle = STATUS_STYLE[status]
  return (
    <header className="flex items-center gap-6 border-b border-cyan-400/15 bg-[#070d13] px-4 py-2">
      <h1 className="text-sm font-black tracking-[0.35em] text-cyan-300">
        VANGUARD <span className="text-slate-600">//</span> TACTICAL C2
      </h1>
      <div className={`flex items-center gap-2 text-[11px] ${statusStyle.text}`}>
        <span className={`h-2 w-2 rounded-full ${statusStyle.dot}`} />
        {statusStyle.label}
      </div>
      <div className="ml-auto flex items-center gap-5 text-[11px] text-slate-400">
        <span>
          REAL <span className="font-bold text-red-400">{counts.real}</span>
        </span>
        <span>
          DECOY <span className="font-bold text-slate-300">{counts.decoy}</span>
        </span>
        <span>
          ??? <span className="font-bold text-amber-300">{counts.unknown}</span>
        </span>
        <span>
          NEUTRALIZED <span className="font-bold text-cyan-300">{neutralized}</span>
        </span>
        <span>
          IMPACTS{' '}
          <span className={`font-bold ${impacts > 0 ? 'text-red-500' : 'text-slate-500'}`}>
            {impacts}
          </span>
        </span>
        <span>
          PLATFORMS <span className="font-bold text-emerald-400">{platformCount}</span>
        </span>
        <button
          type="button"
          onClick={onToggleBasemap}
          className="border border-cyan-400/30 px-2 py-0.5 font-bold tracking-widest text-cyan-300 hover:bg-cyan-400/10"
        >
          {basemap === 'dark' ? 'DARK' : 'SAT'}
        </button>
        <span className="text-slate-300">{clock}Z</span>
      </div>
    </header>
  )
}
