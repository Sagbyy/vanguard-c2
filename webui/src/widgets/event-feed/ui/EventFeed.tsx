import type { FeedEvent } from '@/entities/engagement'

export function EventFeed({ feed }: { feed: FeedEvent[] }) {
  return (
    <section>
      <h2 className="mb-1 text-[10px] font-bold tracking-[0.3em] text-neutral-500">EVENT FEED</h2>
      <div className="flex max-h-40 flex-col gap-0.5 overflow-y-auto">
        {feed.length === 0 && <p className="text-[11px] text-neutral-600">No events.</p>}
        {feed.map((e) => (
          <div key={e.key} className="flex gap-2 text-[11px]">
            <span className="text-neutral-600">{e.time}</span>
            <span
              className={
                e.kind === 'kill'
                  ? 'text-neutral-100'
                  : e.kind === 'impact'
                    ? 'font-bold text-red-400'
                    : 'text-neutral-500'
              }
            >
              {e.text}
            </span>
          </div>
        ))}
      </div>
    </section>
  )
}
