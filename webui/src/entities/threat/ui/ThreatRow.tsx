import { threatDistance, type Threat, type TrackCategory } from '../model/threat'

const CATEGORY_TAG: Record<TrackCategory, { label: string; cls: string }> = {
  real: { label: 'REAL', cls: 'bg-red-500/10 text-red-400 border-red-500/40' },
  decoy: { label: 'DECOY', cls: 'bg-transparent text-neutral-500 border-white/15' },
  unknown: { label: '???', cls: 'bg-amber-500/10 text-amber-300 border-amber-500/40' },
}

function threatBadge(level: number): string {
  if (level >= 5) return 'bg-white/10 text-neutral-100 border-white/60'
  if (level >= 4) return 'bg-white/5 text-neutral-200 border-white/40'
  if (level >= 3) return 'bg-transparent text-neutral-300 border-white/25'
  return 'bg-transparent text-neutral-400 border-white/15'
}

export function ThreatRow({ threat, category }: { threat: Threat; category: TrackCategory }) {
  const distance = threatDistance(threat)
  const tag = CATEGORY_TAG[category]
  return (
    <div
      className={`flex items-center gap-2 border-b border-white/8 py-1.5 text-[11px] ${
        category === 'decoy' ? 'opacity-50' : ''
      }`}
    >
      <span className={`border px-1.5 py-0.5 font-bold ${tag.cls}`}>{tag.label}</span>
      <span className={`border px-1.5 py-0.5 font-bold ${threatBadge(threat.threat_level)}`}>
        L{threat.threat_level}
      </span>
      <span className="text-neutral-300">{threat.id.slice(0, 8)}</span>
      <span className="ml-auto text-neutral-400">{(distance / 1000).toFixed(2)} km</span>
    </div>
  )
}
