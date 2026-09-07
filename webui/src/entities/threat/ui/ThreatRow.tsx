import { threatDistance, type Threat, type TrackCategory } from '../model/threat'

const CATEGORY_TAG: Record<TrackCategory, { label: string; cls: string }> = {
  real: { label: 'REAL', cls: 'bg-red-500/20 text-red-400 border-red-500/40' },
  decoy: { label: 'DECOY', cls: 'bg-slate-500/20 text-slate-300 border-slate-500/40' },
  unknown: { label: '???', cls: 'bg-amber-500/20 text-amber-300 border-amber-500/40' },
}

function threatBadge(level: number): string {
  if (level >= 5) return 'bg-red-500/20 text-red-400 border-red-500/40'
  if (level >= 4) return 'bg-orange-500/20 text-orange-400 border-orange-500/40'
  if (level >= 3) return 'bg-amber-500/20 text-amber-400 border-amber-500/40'
  return 'bg-yellow-500/20 text-yellow-300 border-yellow-500/40'
}

export function ThreatRow({ threat, category }: { threat: Threat; category: TrackCategory }) {
  const distance = threatDistance(threat)
  const tag = CATEGORY_TAG[category]
  return (
    <div
      className={`flex items-center gap-2 border-b border-slate-800/60 py-1.5 text-[11px] ${
        category === 'decoy' ? 'opacity-50' : ''
      }`}
    >
      <span className={`border px-1.5 py-0.5 font-bold ${tag.cls}`}>{tag.label}</span>
      <span className={`border px-1.5 py-0.5 font-bold ${threatBadge(threat.threat_level)}`}>
        L{threat.threat_level}
      </span>
      <span className="text-slate-300">{threat.id.slice(0, 8)}</span>
      <span className="ml-auto text-slate-400">{(distance / 1000).toFixed(2)} km</span>
    </div>
  )
}
