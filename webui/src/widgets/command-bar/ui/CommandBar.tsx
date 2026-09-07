import { useState } from "react";
import type { ConnectionStatus } from "@/shared/api";
import type { Basemap } from "@/shared/config";

const STATUS_STYLE: Record<ConnectionStatus, { label: string; dot: string; text: string }> = {
  connected: { label: "LINK ESTABLISHED", dot: "bg-emerald-400", text: "text-emerald-400" },
  connecting: { label: "ACQUIRING LINK…", dot: "animate-pulse bg-amber-400", text: "text-amber-400" },
  offline: { label: "LINK DOWN", dot: "bg-red-500", text: "text-red-500" },
};

interface CommandBarProps {
  status: ConnectionStatus;
  counts: { real: number; decoy: number; unknown: number };
  neutralized: number;
  impacts: number;
  platformCount: number;
  basemap: Basemap;
  onToggleBasemap: () => void;
  clock: string;
}

function Stat({ label, value }: { label: string; value: number | string }) {
  return (
    <span className="text-neutral-500">
      {label} <span className="font-bold text-neutral-100">{value}</span>
    </span>
  );
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
  const [logoOk, setLogoOk] = useState(true);
  return (
    <header className="flex items-center gap-6 border-b border-white/10 bg-black px-4 py-2">
      <div className="flex items-center gap-3">
        {logoOk ? (
          <img
            src={`${import.meta.env.BASE_URL}vangward-logo-name-white-horizontal.png`}
            alt=""
            onError={() => setLogoOk(false)}
            className="h-6 text-neutral-100"
          />
        ) : null}
      </div>

      <div className={`flex items-center gap-2 text-[11px] ${STATUS_STYLE[status].text}`}>
        <span className={`h-2 w-2 rounded-full ${STATUS_STYLE[status].dot}`} />
        {STATUS_STYLE[status].label}
      </div>

      <div className="ml-auto flex items-center gap-5 text-[11px]">
        <Stat label="REAL" value={counts.real} />
        <Stat label="DECOY" value={counts.decoy} />
        <Stat label="???" value={counts.unknown} />
        <Stat label="NEUTRALIZED" value={neutralized} />
        <Stat label="IMPACTS" value={impacts} />
        <Stat label="PLATFORMS" value={platformCount} />
        <button
          type="button"
          onClick={onToggleBasemap}
          className="border border-white/20 px-2 py-0.5 font-bold tracking-widest text-neutral-300 hover:bg-white/10"
        >
          {basemap === "dark" ? "DARK" : "SAT"}
        </button>
        <span className="text-neutral-300">{clock}Z</span>
      </div>
    </header>
  );
}
