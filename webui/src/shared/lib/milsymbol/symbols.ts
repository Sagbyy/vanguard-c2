import ms from 'milsymbol'
import type { ThreatClassification } from '@/entities/threat'

// NATO Joint Military Symbology APP-6(D) — rendered via milsymbol (numeric SIDC).
ms.setStandard('APP6')

// APP-6(D) 20-digit numeric SIDC:
//   version(10) · context 0=reality · affiliation · symbol-set(2) ·
//   status 0 · HQ/TF/dummy 0 · amplifier 00 · entity(6) · modifiers 0000
type Affiliation = '1' | '3' | '4' | '5' | '6' // Unknown · Friend · Neutral · Suspect · Hostile

function sidc(affiliation: Affiliation, symbolSet: string, entity: string): string {
  return `100${affiliation}${symbolSet}0000${entity}0000`
}

const cache = new Map<string, string>()

function render(code: string, size: number): string {
  const key = `${code}:${size}`
  let svg = cache.get(key)
  if (!svg) {
    svg = new ms.Symbol(code, { size }).asSVG()
    cache.set(key, svg)
  }
  return svg
}

// Ground-based interceptor site → friendly land Air-Defence unit (set 10, entity 130100).
export function platformSymbol(size = 20): string {
  return render(sidc('3', '10', '130100'), size)
}

// Defended asset → friendly land installation / base (set 20, entity 120802).
export function assetSymbol(size = 22): string {
  return render(sidc('3', '20', '120802'), size)
}

// Interceptor effector in flight → friendly air missile (set 02, entity 110000).
export function interceptorSymbol(size = 14): string {
  return render(sidc('3', '02', '110000'), size)
}

// Hostile / unknown air track. Affiliation and icon are driven by the fused classification.
export function threatSymbol(classification: ThreatClassification, size = 18): string {
  let affiliation: Affiliation = '6' // Hostile by default
  let symbolSet = '01' // Air
  let entity = '110000' // generic military air

  switch (classification) {
    case 'Drone':
      entity = '110300' // Unmanned Aerial Vehicle (UAV)
      break
    case 'FPVDrone':
      entity = '110400' // Vertical-Takeoff UAV (VT-UAV)
      break
    case 'Helicopter':
      entity = '110200' // Military Rotary Wing
      break
    case 'Aircraft':
      entity = '110100' // Military Fixed Wing
      break
    case 'CruiseMissile':
    case 'BallisticMissile':
      symbolSet = '02' // Air Missile
      entity = '110000'
      break
    case 'Friendly':
      affiliation = '3'
      entity = '110100'
      break
    case 'Civilian':
      affiliation = '4' // Neutral
      entity = '120100' // Civilian Fixed Wing
      break
    case 'Decoy':
      affiliation = '5' // Suspect — deceptive track
      break
    case 'Unknown':
      affiliation = '1' // Unknown
      break
  }

  return render(sidc(affiliation, symbolSet, entity), size)
}
