import type { Position, Speed } from '@/shared/lib/geo'

export interface Threat {
  id: string
  position: Position
  speed: number
  threat_level: number
  is_decoy: boolean
}

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

export interface DetectedThreat {
  id: string
  position: Position
  speed: Speed
  threat_level: number
  classification: ThreatClassification
  confidence: number
  detected_at: number
}

export type TrackCategory = 'unknown' | 'real' | 'decoy'

export function trackCategory(classification: ThreatClassification): TrackCategory {
  if (classification === 'Decoy') return 'decoy'
  if (classification === 'Unknown') return 'unknown'
  return 'real'
}

export function threatDistance(threat: Threat): number {
  return Math.hypot(threat.position.x, threat.position.y)
}

export const CATEGORY_COLOR: Record<TrackCategory, string> = {
  unknown: '#ffd23e',
  real: '#ff3b4d',
  decoy: '#8aa3b5',
}
