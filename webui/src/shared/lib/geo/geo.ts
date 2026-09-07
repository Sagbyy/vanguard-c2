export interface Position {
  x: number
  y: number
}

export interface Speed {
  x: number
  y: number
}

export const KYIV: [number, number] = [30.5234, 50.4501]

const M_PER_DEG_LAT = 111_320
const M_PER_DEG_LON = M_PER_DEG_LAT * Math.cos((KYIV[1] * Math.PI) / 180)

export function toLngLat(position: Position): [number, number] {
  return [KYIV[0] + position.x / M_PER_DEG_LON, KYIV[1] + position.y / M_PER_DEG_LAT]
}

export function fromLngLat(lng: number, lat: number): Position {
  return { x: (lng - KYIV[0]) * M_PER_DEG_LON, y: (lat - KYIV[1]) * M_PER_DEG_LAT }
}

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
