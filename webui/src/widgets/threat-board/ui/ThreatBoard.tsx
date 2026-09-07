import { ThreatRow, type Threat, type TrackCategory } from '@/entities/threat'

interface ThreatBoardProps {
  threats: Threat[]
  categoryOf: (threat: Threat) => TrackCategory
}

export function ThreatBoard({ threats, categoryOf }: ThreatBoardProps) {
  return (
    <section className="min-h-0">
      <h2 className="mb-1 text-[10px] font-bold tracking-[0.3em] text-red-400/80">
        HOSTILE TRACKS — CLOSEST FIRST
      </h2>
      {threats.length === 0 && <p className="text-[11px] text-slate-600">Airspace clear.</p>}
      {threats.map((threat) => (
        <ThreatRow key={threat.id} threat={threat} category={categoryOf(threat)} />
      ))}
    </section>
  )
}
