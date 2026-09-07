import type { Position } from '@/shared/lib/geo'

export interface FlyingInterceptor {
  id: string
  position: Position
  target_id: string
  diverting: boolean
}
