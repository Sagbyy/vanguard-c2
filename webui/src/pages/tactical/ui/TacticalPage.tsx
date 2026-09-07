import { useEffect, useMemo, useState } from "react";
import type { Position } from "@/shared/lib/geo";
import type { Basemap } from "@/shared/config";
import type { ConnectionStatus } from "@/shared/api";
import {
  trackCategory,
  type Threat,
  type ThreatClassification,
  type TrackCategory,
} from "@/entities/threat";
import { REMOVE_AFTER_MS, type PlatformView } from "@/entities/platform";
import type { FlyingInterceptor } from "@/entities/interceptor";
import type { Burst, EngagementReport, FeedEvent } from "@/entities/engagement";
import { CommandBar } from "@/widgets/command-bar";
import { TacticalMap } from "@/widgets/tactical-map";
import { SeekerFeed } from "@/widgets/seeker-feed";
import { ControlPanel } from "@/widgets/control-panel";
import { EventFeed } from "@/widgets/event-feed";
import { PlatformRoster } from "@/widgets/platform-roster";
import { ThreatBoard } from "@/widgets/threat-board";

export interface TacticalPageProps {
  status: ConnectionStatus;
  threats: Threat[];
  platforms: Map<string, PlatformView>;
  classifications: Map<string, ThreatClassification>;
  engagements: EngagementReport;
  interceptors: FlyingInterceptor[];
  feed: FeedEvent[];
  bursts: Burst[];
  impacts: number;
  publish: (subject: string, payload: unknown) => void;
  removePlatform: (platformId: string) => void;
  retargetInterceptor: (interceptorId: string, targetId: string) => void;
  abortInterceptor: (interceptorId: string) => void;
  reset: () => void;
}

export function TacticalPage({
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
}: TacticalPageProps) {
  const [now, setNow] = useState(() => Date.now());
  const [basemap, setBasemap] = useState<Basemap>("sat");
  const [placing, setPlacing] = useState(false);
  const [selectedInterceptor, setSelectedInterceptor] = useState<string | null>(
    null,
  );
  const [pending, setPending] = useState<Position | null>(null);
  const [previewReach, setPreviewReach] = useState(15_000);
  const [zoneRadius, setZoneRadius] = useState(6_000);

  const categoryOf = (threat: Threat): TrackCategory =>
    trackCategory(classifications.get(threat.id) ?? "Unknown");

  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);

  const platformList = useMemo(
    () =>
      [...platforms.values()]
        .filter((view) => now - view.lastSeen < REMOVE_AFTER_MS)
        .sort((a, b) => a.report.name.localeCompare(b.report.name)),
    [platforms, now],
  );
  const sortedThreats = useMemo(
    () =>
      [...threats].sort(
        (a, b) =>
          Math.hypot(a.position.x, a.position.y) -
          Math.hypot(b.position.x, b.position.y),
      ),
    [threats],
  );
  const counts = useMemo(() => {
    const c = { real: 0, decoy: 0, unknown: 0 };
    for (const t of threats)
      c[trackCategory(classifications.get(t.id) ?? "Unknown")] += 1;
    return c;
  }, [threats, classifications]);
  const clock = new Date(now).toISOString().slice(11, 19);

  const selectedIcptr = selectedInterceptor
    ? (interceptors.find((i) => i.id === selectedInterceptor) ?? null)
    : null;
  const selectedTarget = selectedIcptr
    ? (threats.find((t) => t.id === selectedIcptr.target_id) ?? null)
    : null;

  return (
    <div className="flex h-full flex-col bg-black text-neutral-200">
      <CommandBar
        status={status}
        counts={counts}
        neutralized={engagements.neutralized}
        impacts={impacts}
        platformCount={platformList.length}
        basemap={basemap}
        onToggleBasemap={() =>
          setBasemap((b) => (b === "dark" ? "sat" : "dark"))
        }
        clock={clock}
      />

      <div className="flex min-h-0 flex-1">
        <main className="relative min-w-0 flex-1">
          <TacticalMap
            threats={threats}
            platforms={platformList}
            basemap={basemap}
            classifications={classifications}
            placing={placing}
            onMapClick={(pos) => setPending(pos)}
            preview={
              pending ? { position: pending, reach: previewReach } : null
            }
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
            <div className="absolute left-1/2 top-3 flex -translate-x-1/2 items-center gap-3 border border-white/25 bg-black/95 px-3 py-1.5 text-[11px] text-neutral-300">
              <span>
                INTERCEPTOR{" "}
                <span className="text-neutral-100">
                  {selectedInterceptor.slice(0, 8)}
                </span>{" "}
                — click a hostile to re-task
              </span>
              <button
                type="button"
                onClick={() => {
                  abortInterceptor(selectedInterceptor);
                  setSelectedInterceptor(null);
                }}
                className="border border-white/40 px-2 py-0.5 font-bold tracking-widest text-neutral-100 hover:bg-white/10"
              >
                ABORT
              </button>
              <button
                type="button"
                onClick={() => setSelectedInterceptor(null)}
                className="text-neutral-500 hover:text-neutral-300"
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

        <aside className="flex w-72 flex-col gap-4 overflow-y-auto border-l border-white/10 bg-black p-3">
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

          <EventFeed feed={feed} />

          <PlatformRoster platforms={platformList} now={now} />

          <ThreatBoard threats={sortedThreats} categoryOf={categoryOf} />
        </aside>
      </div>
    </div>
  );
}
