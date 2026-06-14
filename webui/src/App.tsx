import { useEffect, useMemo, useState } from 'react'
import { ControlPanel } from './ControlPanel'
import { SeekerFeed } from './SeekerFeed'
import { type Basemap, TacticalMap } from './TacticalMap'
import {
  ammoLabel,
  REMOVE_AFTER_MS,
  STALE_AFTER_MS,
  trackCategory,
  type PlatformView,
  type Position,
  type Threat,
  type TrackCategory,
} from './types'
import { useNats, type ConnectionStatus } from './useNats'

const CATEGORY_TAG: Record<TrackCategory, { label: string; cls: string }> = {
  real: { label: 'REAL', cls: 'bg-red-500/20 text-red-400 border-red-500/40' },
  decoy: { label: 'DECOY', cls: 'bg-slate-500/20 text-slate-300 border-slate-500/40' },
  unknown: { label: '???', cls: 'bg-amber-500/20 text-amber-300 border-amber-500/40' },
}

const STATUS_STYLE: Record<ConnectionStatus, { label: string; dot: string; text: string }> = {
  connected: { label: 'LINK ESTABLISHED', dot: 'bg-emerald-400', text: 'text-emerald-400' },
  connecting: { label: 'ACQUIRING LINK…', dot: 'bg-amber-400 animate-pulse', text: 'text-amber-400' },
  offline: { label: 'LINK DOWN', dot: 'bg-red-500', text: 'text-red-500' },
}

function threatBadge(level: number): string {
  if (level >= 5) return 'bg-red-500/20 text-red-400 border-red-500/40'
  if (level >= 4) return 'bg-orange-500/20 text-orange-400 border-orange-500/40'
  if (level >= 3) return 'bg-amber-500/20 text-amber-400 border-amber-500/40'
  return 'bg-yellow-500/20 text-yellow-300 border-yellow-500/40'
}

function PlatformCard({ view, now }: { view: PlatformView; now: number }) {
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

function ThreatRow({ threat, category }: { threat: Threat; category: TrackCategory }) {
  const distance = Math.hypot(threat.position.x, threat.position.y)
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

export default function App() {
  const {
    status,
    threats,
    platforms,
    classifications,
    engagements,
    interceptors,
    feed,
    bursts,
    impacts,
    publish,
    removePlatform,
    retargetInterceptor,
    abortInterceptor,
    reset,
  } = useNats()
  const [now, setNow] = useState(() => Date.now())
  const [basemap, setBasemap] = useState<Basemap>('dark')
  const [placing, setPlacing] = useState(false)
  const [selectedInterceptor, setSelectedInterceptor] = useState<string | null>(null)
  const [pending, setPending] = useState<Position | null>(null)
  const [previewReach, setPreviewReach] = useState(15_000)
  const [zoneRadius, setZoneRadius] = useState(6_000)

  const categoryOf = (threat: Threat): TrackCategory =>
    trackCategory(classifications.get(threat.id) ?? 'Unknown')

  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000)
    return () => clearInterval(id)
  }, [])

  const platformList = useMemo(
    () =>
      [...platforms.values()]
        .filter((view) => now - view.lastSeen < REMOVE_AFTER_MS)
        .sort((a, b) => a.report.name.localeCompare(b.report.name)),
    [platforms, now],
  )
  const sortedThreats = useMemo(
    () =>
      [...threats].sort(
        (a, b) =>
          Math.hypot(a.position.x, a.position.y) - Math.hypot(b.position.x, b.position.y),
      ),
    [threats],
  )
  const counts = useMemo(() => {
    const c = { real: 0, decoy: 0, unknown: 0 }
    for (const t of threats) c[categoryOf(t)] += 1
    return c
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [threats, classifications])
  const statusStyle = STATUS_STYLE[status]
  const clock = new Date(now).toISOString().slice(11, 19)

  // Onboard seeker camera: the selected interceptor + the target it is chasing.
  const selectedIcptr = selectedInterceptor
    ? (interceptors.find((i) => i.id === selectedInterceptor) ?? null)
    : null
  const selectedTarget = selectedIcptr
    ? (threats.find((t) => t.id === selectedIcptr.target_id) ?? null)
    : null

  return (
    <div className="flex h-full flex-col bg-[#04070b] text-slate-200">
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
            NEUTRALIZED <span className="font-bold text-cyan-300">{engagements.neutralized}</span>
          </span>
          <span>
            IMPACTS <span className={`font-bold ${impacts > 0 ? 'text-red-500' : 'text-slate-500'}`}>{impacts}</span>
          </span>
          <span>
            PLATFORMS <span className="font-bold text-emerald-400">{platformList.length}</span>
          </span>
          <button
            type="button"
            onClick={() => setBasemap((b) => (b === 'dark' ? 'sat' : 'dark'))}
            className="border border-cyan-400/30 px-2 py-0.5 font-bold tracking-widest text-cyan-300 hover:bg-cyan-400/10"
          >
            {basemap === 'dark' ? 'DARK' : 'SAT'}
          </button>
          <span className="text-slate-300">{clock}Z</span>
        </div>
      </header>

      <div className="flex min-h-0 flex-1">
        <main className="relative min-w-0 flex-1">
          <TacticalMap
            threats={threats}
            platforms={platformList}
            basemap={basemap}
            classifications={classifications}
            placing={placing}
            onMapClick={(pos) => setPending(pos)}
            preview={pending ? { position: pending, reach: previewReach } : null}
            engagements={engagements.lines}
            interceptors={interceptors}
            bursts={bursts}
            zoneRadius={zoneRadius}
            safeZones={engagements.safe_zones}
            selectedInterceptor={selectedInterceptor}
            onSelectInterceptor={setSelectedInterceptor}
            onRetarget={retargetInterceptor}
          />
          {selectedInterceptor && (
            <div className="absolute left-1/2 top-3 flex -translate-x-1/2 items-center gap-3 border border-cyan-400/40 bg-[#070d13]/95 px-3 py-1.5 text-[11px] text-slate-300">
              <span>
                INTERCEPTOR <span className="text-cyan-300">{selectedInterceptor.slice(0, 8)}</span> —
                click a hostile to re-task
              </span>
              <button
                type="button"
                onClick={() => {
                  abortInterceptor(selectedInterceptor)
                  setSelectedInterceptor(null)
                }}
                className="border border-orange-400/50 px-2 py-0.5 font-bold tracking-widest text-orange-300 hover:bg-orange-400/10"
              >
                ABORT
              </button>
              <button
                type="button"
                onClick={() => setSelectedInterceptor(null)}
                className="text-slate-500 hover:text-slate-300"
              >
                ✕
              </button>
            </div>
          )}
          {selectedInterceptor && (
            <SeekerFeed
              interceptor={selectedIcptr}
              target={selectedTarget}
              onClose={() => setSelectedInterceptor(null)}
            />
          )}
        </main>

        <aside className="flex w-72 flex-col gap-4 overflow-y-auto border-l border-cyan-400/15 bg-[#070d13] p-3">
          <ControlPanel
            publish={publish}
            removePlatform={removePlatform}
            reset={reset}
            platforms={platformList}
            placing={placing}
            setPlacing={setPlacing}
            pending={pending}
            clearPending={() => setPending(null)}
            onReachChange={setPreviewReach}
            onZoneRadiusChange={setZoneRadius}
          />

          <section>
            <h2 className="mb-1 text-[10px] font-bold tracking-[0.3em] text-cyan-400/80">
              EVENT FEED
            </h2>
            <div className="flex max-h-40 flex-col gap-0.5 overflow-y-auto">
              {feed.length === 0 && <p className="text-[11px] text-slate-600">No events.</p>}
              {feed.map((e) => (
                <div key={e.key} className="flex gap-2 text-[11px]">
                  <span className="text-slate-600">{e.time}</span>
                  <span
                    className={
                      e.kind === 'kill'
                        ? 'text-cyan-300'
                        : e.kind === 'impact'
                          ? 'font-bold text-red-400'
                          : 'text-slate-500'
                    }
                  >
                    {e.text}
                  </span>
                </div>
              ))}
            </div>
          </section>

          <section>
            <h2 className="mb-2 text-[10px] font-bold tracking-[0.3em] text-emerald-400/80">
              INTERCEPTOR PLATFORMS
            </h2>
            <div className="flex flex-col gap-2">
              {platformList.length === 0 && (
                <p className="text-[11px] text-slate-600">No platform reporting…</p>
              )}
              {platformList.map((view) => (
                <PlatformCard key={view.report.platform_id} view={view} now={now} />
              ))}
            </div>
          </section>

          <section className="min-h-0">
            <h2 className="mb-1 text-[10px] font-bold tracking-[0.3em] text-red-400/80">
              HOSTILE TRACKS — CLOSEST FIRST
            </h2>
            {sortedThreats.length === 0 && (
              <p className="text-[11px] text-slate-600">Airspace clear.</p>
            )}
            {sortedThreats.map((threat) => (
              <ThreatRow key={threat.id} threat={threat} category={categoryOf(threat)} />
            ))}
          </section>
        </aside>
      </div>
    </div>
  )
}
