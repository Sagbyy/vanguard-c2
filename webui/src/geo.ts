import type { Position } from './types'

/**
 * The Rust domain works in a local cartesian frame (metres around the
 * defended asset). The UI anchors that frame on Kyiv city centre and
 * converts to lon/lat with an equirectangular approximation — accurate to
 * well under a metre at this scale.
 */
export const KYIV: [number, number] = [30.5234, 50.4501] // lon, lat

const M_PER_DEG_LAT = 111_320
const M_PER_DEG_LON = M_PER_DEG_LAT * Math.cos((KYIV[1] * Math.PI) / 180)

export function toLngLat(position: Position): [number, number] {
  return [KYIV[0] + position.x / M_PER_DEG_LON, KYIV[1] + position.y / M_PER_DEG_LAT]
}

/** Polygon ring approximating a metric circle, as GeoJSON coordinates. */
export function rangeRing(center: Position, radiusM: number, steps = 64): [number, number][] {
  const ring: [number, number][] = []
  for (let i = 0; i <= steps; i++) {
    const angle = (i / steps) * 2 * Math.PI
    ring.push(
      toLngLat({
        x: center.x + radiusM * Math.cos(angle),
        y: center.y + radiusM * Math.sin(angle),
      }),
    )
  }
  return ring
}
