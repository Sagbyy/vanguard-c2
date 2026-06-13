// TypeScript mirrors of the Rust structs in vanguard-core (serde_json wire format).

export interface Position {
  x: number
  y: number
}

export interface Speed {
  x: number
  y: number
}

/** Ground truth published by vanguard-map on `map.threats`. */
export interface Threat {
  id: string
  position: Position
  speed: number
  threat_level: number
}

/** One radar contact inside an InterceptorReport. */
export interface DetectedThreat {
  id: string
  position: Position
  speed: Speed
  threat_level: number
}

/** Radar report published by each platform on `platform.<id>.report`. */
export interface InterceptorReport {
  platform_id: string
  name: string
  position: Position
  range: number
  threats: DetectedThreat[]
  interceptors_remaining: number
  timestamp: number
}

/** Client-side view of a platform, with reception freshness. */
export interface PlatformView {
  report: InterceptorReport
  lastSeen: number
}

export const STALE_AFTER_MS = 5_000
/** A platform silent for this long is removed from the picture entirely. */
export const REMOVE_AFTER_MS = 30_000
