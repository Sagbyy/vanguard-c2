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

/** Mirror of vanguard_core::ThreatClassification. */
export type ThreatClassification =
  | 'Unknown'
  | 'Decoy'
  | 'Drone'
  | 'FPVDrone'
  | 'Helicopter'
  | 'Aircraft'
  | 'CruiseMissile'
  | 'BallisticMissile'
  | 'Friendly'
  | 'Civilian'

/** One radar contact inside an InterceptorReport. */
export interface DetectedThreat {
  id: string
  position: Position
  speed: Speed
  threat_level: number
  classification: ThreatClassification
  confidence: number
  detected_at: number
}

/** Operator-side category derived from fused platform classifications. */
export type TrackCategory = 'unknown' | 'real' | 'decoy'

export function trackCategory(classification: ThreatClassification): TrackCategory {
  if (classification === 'Decoy') return 'decoy'
  if (classification === 'Unknown') return 'unknown'
  return 'real'
}

/** Radar report published by each platform on `platform.<id>.report`. */
export interface InterceptorReport {
  platform_id: string
  name: string
  position: Position
  reach: number
  threats: DetectedThreat[]
  interceptors_remaining: number
  timestamp: number
}

/** Client-side view of a platform, with reception freshness. */
export interface PlatformView {
  report: InterceptorReport
  lastSeen: number
}

/** Mirror of vanguard_core::MapConfig (live-tunable swarm parameters). */
export interface MapConfig {
  decoy_ratio: number
  swarm_min: number
  swarm_max: number
  spawn_interval_s: number
  zone_radius: number
  max_active: number
}

export const DEFAULT_MAP_CONFIG: MapConfig = {
  decoy_ratio: 0.4,
  swarm_min: 6,
  swarm_max: 12,
  spawn_interval_s: 45,
  zone_radius: 6_000,
  max_active: 40,
}

/** Mirror of vanguard_core::PlatformSpec. */
export interface PlatformSpec {
  id: string
  name: string
  position: Position
  reach: number
  ammo: number
}

export function ammoLabel(n: number): string {
  return n === 0 ? '∅' : String(n)
}

// Control subjects (UI → Rust).
export const MAP_CONFIG_SUBJECT = 'control.map.config'
export const PLATFORM_ADD_SUBJECT = 'control.platform.add'
export const PLATFORM_REMOVE_SUBJECT = 'control.platform.remove'
export const CONTROL_RESET_SUBJECT = 'control.reset'
export const ENGAGEMENTS_SUBJECT = 'control.engagements'
export const INTERCEPTORS_SUBJECT = 'control.interceptors'

/** Mirror of vanguard_core::FlyingInterceptor. */
export interface FlyingInterceptor {
  id: string
  position: Position
  target_id: string
}

/** Mirror of vanguard_core::Engagement. */
export interface Engagement {
  platform_id: string
  threat_id: string
}

/** Mirror of vanguard_core::EngagementReport. */
export interface EngagementReport {
  lines: Engagement[]
  neutralized: number
}

export const STALE_AFTER_MS = 5_000
/** A platform silent for this long is removed from the picture entirely. */
export const REMOVE_AFTER_MS = 30_000
