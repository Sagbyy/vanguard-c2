import { PlatformCard, type PlatformView } from '@/entities/platform'

export function PlatformRoster({ platforms, now }: { platforms: PlatformView[]; now: number }) {
  return (
    <section>
      <h2 className="mb-2 text-[10px] font-bold tracking-[0.3em] text-emerald-400/70">
        INTERCEPTOR PLATFORMS
      </h2>
      <div className="flex flex-col gap-2">
        {platforms.length === 0 && (
          <p className="text-[11px] text-neutral-600">No platform reporting…</p>
        )}
        {platforms.map((view) => (
          <PlatformCard key={view.report.platform_id} view={view} now={now} />
        ))}
      </div>
    </section>
  )
}
