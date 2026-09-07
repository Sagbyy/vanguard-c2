import type { Position } from '@/shared/lib/geo'

export interface Engagement {
  platform_id: string
  threat_id: string
}

export interface EngagementReport {
  lines: Engagement[]
  neutralized: number
  safe_zones: Position[]
}

export interface ThreatDestroyed {
  id: string
  position: Position
}

export interface FeedEvent {
  key: number
  time: string
  kind: 'kill' | 'impact' | 'decoy'
  text: string
}

export interface Burst {
  key: number
  position: Position
  kind: 'kill' | 'impact'
}
