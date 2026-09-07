import type { Position } from '@/shared/lib/geo'
import type { DetectedThreat } from '@/entities/threat/@x/platform'

export interface InterceptorReport {
  platform_id: string
  name: string
  position: Position
  reach: number
  threats: DetectedThreat[]
  interceptors_remaining: number
  timestamp: number
}

export interface PlatformView {
  report: InterceptorReport
  lastSeen: number
}

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

export const STALE_AFTER_MS = 5_000
export const REMOVE_AFTER_MS = 30_000
