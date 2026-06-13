import { useEffect, useState } from 'react'
import {
  DEFAULT_MAP_CONFIG,
  MAP_CONFIG_SUBJECT,
  PLATFORM_ADD_SUBJECT,
  type MapConfig,
  type PlatformView,
  type Position,
} from './types'

interface ControlPanelProps {
  publish: (subject: string, payload: unknown) => void
  removePlatform: (platformId: string) => void
  reset: () => void
  platforms: PlatformView[]
  placing: boolean
  setPlacing: (v: boolean) => void
  pending: Position | null
  clearPending: () => void
  /** Reported live so the map can preview the reach ring (metres). */
  onReachChange: (reachM: number) => void
  /** Reported live so the map can draw the defended zone (metres). */
  onZoneRadiusChange: (zoneM: number) => void
}

function Slider({
  label,
  value,
  min,
  max,
  step,
  fmt,
  onChange,
}: {
  label: string
  value: number
  min: number
  max: number
  step: number
  fmt: (v: number) => string
  onChange: (v: number) => void
}) {
  return (
    <label className="block text-[11px] text-slate-400">
      <div className="mb-0.5 flex justify-between">
        <span>{label}</span>
        <span className="text-slate-200">{fmt(value)}</span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full accent-cyan-400"
      />
    </label>
  )
}

export function ControlPanel({
  publish,
  removePlatform,
  reset,
  platforms,
  placing,
  setPlacing,
  pending,
  clearPending,
  onReachChange,
  onZoneRadiusChange,
}: ControlPanelProps) {
  const [cfg, setCfg] = useState<MapConfig>(DEFAULT_MAP_CONFIG)
  const [name, setName] = useState('site')
  const [reachKm, setReachKm] = useState(15)
  const [ammo, setAmmo] = useState(6)

  // Keep the map's reach preview in sync with the slider.
  useEffect(() => {
    onReachChange(reachKm * 1000)
  }, [reachKm, onReachChange])

  // Keep the map's defended-zone circle in sync with the slider.
  useEffect(() => {
    onZoneRadiusChange(cfg.zone_radius)
  }, [cfg.zone_radius, onZoneRadiusChange])

  const pushConfig = (next: MapConfig) => {
    setCfg(next)
    publish(MAP_CONFIG_SUBJECT, next)
  }

  const cancel = () => {
    clearPending()
    setPlacing(false)
  }

  const doReset = () => {
    cancel()
    setCfg(DEFAULT_MAP_CONFIG)
    reset()
  }

  const addPlatform = () => {
    if (!pending) return
    publish(PLATFORM_ADD_SUBJECT, {
      id: crypto.randomUUID(),
      name,
      position: pending,
      reach: reachKm * 1000,
      ammo: Math.max(0, Math.floor(ammo) || 0),
    })
    cancel()
  }

  return (
    <section className="border border-cyan-400/15 bg-cyan-400/[0.03] p-3">
      <div className="mb-2 flex items-center justify-between">
        <h2 className="text-[10px] font-bold tracking-[0.3em] text-cyan-400/80">
          SIMULATION CONTROL
        </h2>
        <button type="button" onClick={doReset}
          className="border border-amber-400/40 px-2 py-0.5 text-[10px] font-bold tracking-widest text-amber-300 hover:bg-amber-400/10">
          ↺ RESET
        </button>
      </div>

      <div className="flex flex-col gap-2">
        <Slider label="TIME" value={cfg.time_scale} min={1} max={10} step={1}
          fmt={(v) => `${v}×`}
          onChange={(v) => pushConfig({ ...cfg, time_scale: v })} />
        <Slider label="DECOY RATIO" value={cfg.decoy_ratio} min={0} max={1} step={0.05}
          fmt={(v) => `${Math.round(v * 100)}%`}
          onChange={(v) => pushConfig({ ...cfg, decoy_ratio: v })} />
        <Slider label="SWARM MIN" value={cfg.swarm_min} min={1} max={cfg.swarm_max} step={1}
          fmt={(v) => `${v}`}
          onChange={(v) => pushConfig({ ...cfg, swarm_min: v })} />
        <Slider label="SWARM MAX" value={cfg.swarm_max} min={cfg.swarm_min} max={30} step={1}
          fmt={(v) => `${v}`}
          onChange={(v) => pushConfig({ ...cfg, swarm_max: v })} />
        <Slider label="WAVE INTERVAL" value={cfg.spawn_interval_s} min={5} max={120} step={5}
          fmt={(v) => `${v}s`}
          onChange={(v) => pushConfig({ ...cfg, spawn_interval_s: v })} />
        <Slider label="ZONE RADIUS" value={cfg.zone_radius} min={1000} max={15000} step={500}
          fmt={(v) => `${(v / 1000).toFixed(1)}km`}
          onChange={(v) => pushConfig({ ...cfg, zone_radius: v })} />
        <Slider label="MAX ACTIVE" value={cfg.max_active} min={5} max={120} step={5}
          fmt={(v) => `${v}`}
          onChange={(v) => pushConfig({ ...cfg, max_active: v })} />
      </div>

      <h2 className="mt-4 mb-2 text-[10px] font-bold tracking-[0.3em] text-emerald-400/80">
        ADD PLATFORM
      </h2>
      <button
        type="button"
        onClick={() => setPlacing(!placing)}
        className={`w-full border px-2 py-1 text-[11px] font-bold tracking-widest ${
          placing
            ? 'border-amber-400 text-amber-300'
            : 'border-emerald-400/40 text-emerald-300 hover:bg-emerald-400/10'
        }`}
      >
        {placing ? 'CLICK MAP TO PLACE…' : 'PLACE ON MAP'}
      </button>

      {pending && (
        <div className="mt-2 flex flex-col gap-1.5 text-[11px] text-slate-400">
          <span className="text-slate-300">
            @ ({(pending.x / 1000).toFixed(1)}, {(pending.y / 1000).toFixed(1)}) km
          </span>
          <label className="flex items-center justify-between gap-2">
            NAME
            <input value={name} onChange={(e) => setName(e.target.value)}
              className="w-28 bg-black/40 px-1 text-slate-200 outline-none" />
          </label>
          <Slider label="REACH" value={reachKm} min={3} max={30} step={1}
            fmt={(v) => `${v}km`} onChange={setReachKm} />

          <label className="flex items-center justify-between gap-2">
            AMMO
            <input type="number" min={0} step={1} value={ammo}
              onChange={(e) => setAmmo(Math.max(0, Math.floor(Number(e.target.value)) || 0))}
              className="w-20 bg-black/40 px-1 text-right text-slate-200 outline-none" />
          </label>

          <div className="mt-1 flex gap-2">
            <button type="button" onClick={addPlatform}
              className="flex-1 border border-emerald-400/40 px-2 py-1 font-bold tracking-widest text-emerald-300 hover:bg-emerald-400/10">
              ADD
            </button>
            <button type="button" onClick={cancel}
              className="flex-1 border border-slate-600 px-2 py-1 font-bold tracking-widest text-slate-400 hover:bg-slate-700/30">
              CANCEL
            </button>
          </div>
        </div>
      )}

      {platforms.length > 0 && (
        <div className="mt-3 flex flex-col gap-1">
          {platforms.map((view) => (
            <div key={view.report.platform_id}
              className="flex items-center justify-between text-[11px] text-slate-400">
              <span className="text-emerald-300">{view.report.name.toUpperCase()}</span>
              <span className="text-slate-500">{(view.report.reach / 1000).toFixed(0)}km</span>
              <button type="button"
                onClick={() => removePlatform(view.report.platform_id)}
                className="text-red-400 hover:text-red-300">
                ✕
              </button>
            </div>
          ))}
        </div>
      )}
    </section>
  )
}
