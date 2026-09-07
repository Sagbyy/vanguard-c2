export interface MapConfig {
  decoy_ratio: number
  swarm_min: number
  swarm_max: number
  spawn_interval_s: number
  zone_radius: number
  max_active: number
  time_scale: number
}

export const DEFAULT_MAP_CONFIG: MapConfig = {
  decoy_ratio: 0.4,
  swarm_min: 6,
  swarm_max: 12,
  spawn_interval_s: 45,
  zone_radius: 6_000,
  max_active: 40,
  time_scale: 1,
}
