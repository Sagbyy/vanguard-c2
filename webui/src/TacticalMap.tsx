import type { FeatureCollection } from 'geojson'
import maplibregl, { type GeoJSONSource, type StyleSpecification } from 'maplibre-gl'
import 'maplibre-gl/dist/maplibre-gl.css'
import { useEffect, useRef, useState } from 'react'
import { KYIV, fromLngLat, rangeRing, toLngLat } from './geo'
import {
  STALE_AFTER_MS,
  trackCategory,
  type PlatformView,
  type Position,
  type Threat,
  type ThreatClassification,
} from './types'

export type Basemap = 'dark' | 'sat'

interface TacticalMapProps {
  threats: Threat[]
  platforms: PlatformView[]
  basemap: Basemap
  classifications: Map<string, ThreatClassification>
  placing: boolean
  onMapClick: (pos: Position) => void
  /** Live preview of the platform being placed (position + reach in metres). */
  preview: { position: Position; reach: number } | null
  /** Active engagements (platform id → threat id) to draw firing lines. */
  engagements: { platform_id: string; threat_id: string }[]
  /** Interceptors currently in flight (id + position + abort state) to animate. */
  interceptors: { id: string; position: Position; diverting: boolean }[]
  /** Impact bursts to play once (kill = cyan, real impact = red). */
  bursts: { key: number; position: Position; kind: 'kill' | 'impact' }[]
  /** Defended-zone radius in metres (follows the config slider). */
  zoneRadius: number
  /** Safe drop zones where aborted interceptors self-destruct. */
  safeZones: Position[]
  /** Currently selected interceptor id (for re-task / abort), or null. */
  selectedInterceptor: string | null
  onSelectInterceptor: (id: string | null) => void
  onRetarget: (interceptorId: string, threatId: string) => void
}

const BASE_STYLE: StyleSpecification = {
  version: 8,
  sources: {
    dark: {
      type: 'raster',
      tiles: ['a', 'b', 'c', 'd'].map(
        (s) => `https://${s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}.png`,
      ),
      tileSize: 256,
      attribution: '© OpenStreetMap contributors © CARTO',
    },
    satellite: {
      type: 'raster',
      tiles: [
        'https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}',
      ],
      tileSize: 256,
      attribution: 'Imagery © Esri',
    },
  },
  layers: [
    { id: 'dark', type: 'raster', source: 'dark' },
    { id: 'satellite', type: 'raster', source: 'satellite', layout: { visibility: 'none' } },
    // Dimming layer over satellite imagery to keep tactical overlays readable.
    {
      id: 'sat-dim',
      type: 'background',
      layout: { visibility: 'none' },
      paint: { 'background-color': '#04070b', 'background-opacity': 0.45 },
    },
  ],
}

// Operator-picture colours: unknown until classified, then real vs decoy.
const CATEGORY_COLOR = {
  unknown: '#ffd23e', // amber
  real: '#ff3b4d', // red
  decoy: '#8aa3b5', // grey
} as const


export function TacticalMap({
  threats,
  platforms,
  basemap,
  classifications,
  placing,
  onMapClick,
  preview,
  engagements,
  interceptors,
  bursts,
  zoneRadius,
  safeZones,
  selectedInterceptor,
  onSelectInterceptor,
  onRetarget,
}: TacticalMapProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const mapRef = useRef<maplibregl.Map | null>(null)
  const markersRef = useRef(new Map<string, maplibregl.Marker>())
  // Latest props, read by the (once-registered) marker/map click handlers.
  const placingRef = useRef(placing)
  placingRef.current = placing
  const onMapClickRef = useRef(onMapClick)
  onMapClickRef.current = onMapClick
  const selectedRef = useRef(selectedInterceptor)
  selectedRef.current = selectedInterceptor
  const onSelectRef = useRef(onSelectInterceptor)
  onSelectRef.current = onSelectInterceptor
  const onRetargetRef = useRef(onRetarget)
  onRetargetRef.current = onRetarget
  // Smooth interpolation between the 1 Hz ground-truth samples.
  const animRef = useRef(new Map<string, { from: Position; to: Position }>())
  const curRef = useRef(new Map<string, Position>())
  const segRef = useRef({ start: 0, dur: 1000 })
  const lastDataMs = useRef(0)
  const loopDataRef = useRef<{
    threats: Threat[]
    platforms: PlatformView[]
    engagements: { platform_id: string; threat_id: string }[]
    interceptors: { id: string; position: Position; diverting: boolean }[]
  }>({ threats: [], platforms: [], engagements: [], interceptors: [] })
  const intTrailsRef = useRef(new Map<string, [number, number][]>())
  const safeMarkersRef = useRef(new Map<number, maplibregl.Marker>())
  // Active impact bursts: key → {position, kind, start ms}.
  const burstsRef = useRef(new Map<number, { position: Position; kind: string; start: number }>())
  // Highest burst key already played — bursts are monotonic, so each plays once.
  const lastBurstKey = useRef(-1)
  const [ready, setReady] = useState(false)

  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    const map = new maplibregl.Map({
      container,
      style: BASE_STYLE,
      center: KYIV,
      zoom: 8.2,
    })
    mapRef.current = map
    map.addControl(new maplibregl.NavigationControl({ showCompass: false }), 'top-left')
    map.addControl(new maplibregl.ScaleControl({ unit: 'metric' }), 'bottom-left')

    map.on('click', (e) => {
      if (!e?.lngLat) return
      if (placingRef.current) {
        onMapClickRef.current(fromLngLat(e.lngLat.lng, e.lngLat.lat))
      } else if (selectedRef.current) {
        onSelectRef.current(null) // click empty map → deselect interceptor
      }
    })

    map.on('load', () => {

      // Defended zone — where threats aim. Radius set by an effect (config slider).
      map.addSource('zone', { type: 'geojson', data: empty() })
      map.addLayer({
        id: 'zone-fill',
        type: 'fill',
        source: 'zone',
        paint: { 'fill-color': '#39d5ff', 'fill-opacity': 0.05 },
      })
      map.addLayer({
        id: 'zone-line',
        type: 'line',
        source: 'zone',
        paint: {
          'line-color': '#39d5ff',
          'line-width': 1,
          'line-dasharray': [3, 3],
          'line-opacity': 0.4,
        },
      })

      // Live preview of a platform being placed (amber, follows the reach slider).
      map.addSource('preview', { type: 'geojson', data: empty() })
      map.addLayer({
        id: 'preview-fill',
        type: 'fill',
        source: 'preview',
        paint: { 'fill-color': '#ffd23e', 'fill-opacity': 0.08 },
      })
      map.addLayer({
        id: 'preview-line',
        type: 'line',
        source: 'preview',
        paint: { 'line-color': '#ffd23e', 'line-width': 1.5, 'line-dasharray': [2, 2] },
      })

      map.addSource('ranges', { type: 'geojson', data: empty() })
      map.addLayer({
        id: 'ranges-fill',
        type: 'fill',
        source: 'ranges',
        paint: {
          'fill-color': '#35f0a8',
          'fill-opacity': ['case', ['get', 'stale'], 0.02, 0.07],
        },
      })
      map.addLayer({
        id: 'ranges-line',
        type: 'line',
        source: 'ranges',
        paint: {
          'line-color': '#35f0a8',
          'line-width': 1.5,
          'line-opacity': ['case', ['get', 'stale'], 0.15, 0.5],
        },
      })

      map.addSource('links', { type: 'geojson', data: empty() })
      map.addLayer({
        id: 'links',
        type: 'line',
        source: 'links',
        paint: {
          'line-color': '#39d5ff',
          'line-width': 1.2,
          'line-opacity': 0.55,
          'line-dasharray': [2, 3],
        },
      })

      // Firing lines: platform engaging a threat (dashed red, the assignment).
      map.addSource('engagements', { type: 'geojson', data: empty() })
      map.addLayer({
        id: 'engagements',
        type: 'line',
        source: 'engagements',
        paint: {
          'line-color': '#ff3b4d',
          'line-width': 1,
          'line-opacity': 0.35,
          'line-dasharray': [1, 3],
        },
      })

      // Impact bursts: expanding fading rings (cyan = kill, red = real impact).
      map.addSource('bursts', { type: 'geojson', data: empty() })
      map.addLayer({
        id: 'bursts',
        type: 'circle',
        source: 'bursts',
        paint: {
          'circle-radius': ['get', 'r'],
          'circle-color': 'transparent',
          'circle-stroke-color': ['get', 'color'],
          'circle-stroke-width': 2.5,
          'circle-stroke-opacity': ['get', 'op'],
        },
      })

      // Interceptor trails (cyan comet tail behind each munition in flight).
      map.addSource('int-trails', { type: 'geojson', data: empty() })
      map.addLayer({
        id: 'int-trails',
        type: 'line',
        source: 'int-trails',
        paint: { 'line-color': '#7df9ff', 'line-width': 2, 'line-opacity': 0.7 },
      })

      // Defended asset: a single permanent marker.
      const asset = document.createElement('div')
      asset.className = 'asset-marker'
      asset.innerHTML =
        '<div class="asset-pulse"></div><div class="asset-core"></div><span class="asset-label">DEFENDED ASSET — KYIV</span>'
      new maplibregl.Marker({ element: asset }).setLngLat(KYIV).addTo(map)

      setReady(true)
    })

    return () => {
      markersRef.current.clear()
      map.remove()
      mapRef.current = null
    }
  }, [])

  // Toggle the satellite imagery + dimming overlay on/off.
  useEffect(() => {
    const map = mapRef.current
    if (!map || !ready) return
    const sat = basemap === 'sat'
    map.setLayoutProperty('satellite', 'visibility', sat ? 'visible' : 'none')
    map.setLayoutProperty('sat-dim', 'visibility', sat ? 'visible' : 'none')
  }, [basemap, ready])

  // Safe drop-zone markers (one per zone; aborted interceptors crash here).
  useEffect(() => {
    const map = mapRef.current
    if (!map || !ready) return
    const markers = safeMarkersRef.current
    safeZones.forEach((zone, i) => {
      let marker = markers.get(i)
      if (!marker) {
        const el = document.createElement('div')
        el.className = 'safe-marker'
        el.innerHTML = '<div class="safe-core"></div><span class="safe-label">SAFE DROP ZONE</span>'
        // Position MUST be set before addTo, or MapLibre reads an undefined lngLat.
        marker = new maplibregl.Marker({ element: el }).setLngLat(toLngLat(zone)).addTo(map)
        markers.set(i, marker)
      }
      marker.setLngLat(toLngLat(zone))
    })
    for (const [i, marker] of markers) {
      if (i >= safeZones.length) {
        marker.remove()
        markers.delete(i)
      }
    }
  }, [safeZones, ready])

  // Defended-zone circle follows the config slider.
  useEffect(() => {
    const map = mapRef.current
    if (!map || !ready) return
    setSource(map, 'zone', [
      {
        type: 'Feature',
        geometry: { type: 'Polygon', coordinates: [rangeRing({ x: 0, y: 0 }, zoneRadius)] },
        properties: {},
      },
    ])
  }, [zoneRadius, ready])

  // Register only bursts not yet played (key strictly above the last seen),
  // so re-renders of the capped `bursts` array don't replay old ones.
  useEffect(() => {
    let maxKey = lastBurstKey.current
    for (const b of bursts) {
      if (b.key > lastBurstKey.current) {
        burstsRef.current.set(b.key, { position: b.position, kind: b.kind, start: performance.now() })
      }
      if (b.key > maxKey) maxKey = b.key
    }
    lastBurstKey.current = maxKey
  }, [bursts])

  // Live reach preview while placing a platform.
  useEffect(() => {
    const map = mapRef.current
    if (!map || !ready) return
    const source = map.getSource('preview') as GeoJSONSource | undefined
    if (!source) return
    source.setData({
      type: 'FeatureCollection',
      features: preview
        ? [
            {
              type: 'Feature',
              geometry: {
                type: 'Polygon',
                coordinates: [rangeRing(preview.position, preview.reach)],
              },
              properties: {},
            },
          ]
        : [],
    })
  }, [preview, ready])

  useEffect(() => {
    const map = mapRef.current
    if (!map || !ready) return

    const now = Date.now()
    const trackedIds = new Set<string>()
    for (const { report, lastSeen } of platforms) {
      if (now - lastSeen <= STALE_AFTER_MS) {
        for (const contact of report.threats) trackedIds.add(contact.id)
      }
    }

    // --- Range bubbles (platforms are static — set once per data tick).
    const ranges = platforms.map(({ report, lastSeen }) => ({
      type: 'Feature' as const,
      geometry: {
        type: 'Polygon' as const,
        coordinates: [rangeRing(report.position, report.reach)],
      },
      properties: { stale: now - lastSeen > STALE_AFTER_MS },
    }))
    ;(map.getSource('ranges') as GeoJSONSource).setData({ type: 'FeatureCollection', features: ranges })

    // --- Animation: retarget each threat from its current drawn position to
    // the new sample. The rAF loop tweens between them at constant velocity.
    const dataNow = Date.now()
    const dur = lastDataMs.current ? Math.min(2000, Math.max(500, dataNow - lastDataMs.current)) : 1000
    lastDataMs.current = dataNow
    for (const threat of threats) {
      const from = curRef.current.get(threat.id) ?? threat.position
      animRef.current.set(threat.id, { from, to: threat.position })
    }
    // Same smooth-interpolation treatment for in-flight interceptors.
    for (const it of interceptors) {
      const from = curRef.current.get(it.id) ?? it.position
      animRef.current.set(it.id, { from, to: it.position })
    }
    segRef.current = { start: performance.now(), dur }
    loopDataRef.current = { threats, platforms, engagements, interceptors }

    // --- DOM markers: platforms and threats, diffed by id.
    const markers = markersRef.current
    const liveIds = new Set<string>()

    for (const { report, lastSeen } of platforms) {
      const key = `p:${report.platform_id}`
      liveIds.add(key)
      const stale = now - lastSeen > STALE_AFTER_MS

      let marker = markers.get(key)
      if (!marker) {
        const el = document.createElement('div')
        el.className = 'platform-marker'
        el.innerHTML =
          '<div class="platform-icon"></div><span class="platform-label"></span><div class="marker-tip"></div>'
        marker = new maplibregl.Marker({ element: el }).setLngLat(toLngLat(report.position)).addTo(map)
        markers.set(key, marker)
      }
      marker.setLngLat(toLngLat(report.position))
      const el = marker.getElement()
      el.classList.toggle('stale', stale)
      el.querySelector('.platform-label')!.textContent = report.name.toUpperCase()
      el.querySelector('.marker-tip')!.textContent =
        `${report.name.toUpperCase()} — ${report.interceptors_remaining} interceptor(s), ` +
        `range ${(report.reach / 1000).toFixed(1)} km, ${report.threats.length} contact(s)`
    }

    for (const threat of threats) {
      const key = `t:${threat.id}`
      liveIds.add(key)

      let marker = markers.get(key)
      if (!marker) {
        const el = document.createElement('div')
        el.className = 'threat-marker'
        el.innerHTML = '<div class="threat-dot"></div><span class="threat-label"></span><div class="marker-tip"></div>'
        // Click a threat while an interceptor is selected → re-task onto it.
        const tid = threat.id
        el.addEventListener('click', (ev) => {
          if (selectedRef.current) {
            ev.stopPropagation()
            onRetargetRef.current(selectedRef.current, tid)
            onSelectRef.current(null)
          }
        })
        // Initial position only — the rAF loop owns motion from here on.
        marker = new maplibregl.Marker({ element: el }).setLngLat(toLngLat(threat.position)).addTo(map)
        markers.set(key, marker)
      }
      const el = marker.getElement()
      el.classList.toggle('tracked', trackedIds.has(threat.id))

      // Colour/shape by the operator's classification, not ground truth.
      const category = trackCategory(classifications.get(threat.id) ?? 'Unknown')
      const color = CATEGORY_COLOR[category]
      const dot = el.querySelector('.threat-dot') as HTMLElement
      const size = 7 + threat.threat_level * 1.6
      dot.style.width = `${size}px`
      dot.style.height = `${size}px`
      if (category === 'decoy') {
        // Hollow grey ring — a harmless decoy.
        dot.style.background = 'transparent'
        dot.style.border = `2px solid ${color}`
        dot.style.boxShadow = 'none'
      } else {
        dot.style.background = color
        dot.style.border = 'none'
        dot.style.boxShadow = `0 0 10px 2px ${color}88`
      }
      const label =
        category === 'real' ? 'REAL' : category === 'decoy' ? 'DECOY' : 'UNKNOWN'
      el.querySelector('.threat-label')!.textContent = threat.id.slice(0, 8)
      el.querySelector('.marker-tip')!.textContent =
        `${label} ${threat.id.slice(0, 8)} — LVL ${threat.threat_level}, ` +
        `${threat.speed.toFixed(0)} m/s, ${(Math.hypot(threat.position.x, threat.position.y) / 1000).toFixed(2)} km from asset` +
        `${trackedIds.has(threat.id) ? ' — TRACKED' : ''}`
    }

    // Interceptor markers (cyan darts in flight; clickable to select).
    for (const it of interceptors) {
      const key = `i:${it.id}`
      liveIds.add(key)
      let marker = markers.get(key)
      if (!marker) {
        const el = document.createElement('div')
        el.className = 'interceptor-marker'
        const iid = it.id
        el.addEventListener('click', (ev) => {
          ev.stopPropagation()
          onSelectRef.current(selectedRef.current === iid ? null : iid)
        })
        marker = new maplibregl.Marker({ element: el }).setLngLat(toLngLat(it.position)).addTo(map)
        markers.set(key, marker)
      }
      const el = marker.getElement()
      el.classList.toggle('selected', it.id === selectedInterceptor)
      el.classList.toggle('diverting', it.diverting)
    }

    for (const [key, marker] of markers) {
      if (!liveIds.has(key)) {
        marker.remove()
        markers.delete(key)
        if (key.startsWith('t:') || key.startsWith('i:')) {
          const id = key.slice(2)
          animRef.current.delete(id)
          curRef.current.delete(id)
          intTrailsRef.current.delete(id)
        }
      }
    }
  }, [threats, platforms, ready, classifications, engagements, interceptors, selectedInterceptor])

  // Animation loop: tween threat markers + their vectors/links between samples.
  useEffect(() => {
    const map = mapRef.current
    if (!map || !ready) return

    let raf = 0
    let lastFrame = 0
    let lastGeo = 0

    const frame = () => {
      raf = requestAnimationFrame(frame)
      const tp = performance.now()
      if (tp - lastFrame < 33) return // cap marker movement at ~30 fps
      lastFrame = tp

      const { start, dur } = segRef.current
      const k = dur > 0 ? Math.min(1, (tp - start) / dur) : 1
      const { threats, platforms, engagements, interceptors } = loopDataRef.current
      const markers = markersRef.current

      // Move markers smoothly (cheap DOM transforms).
      for (const threat of threats) {
        const seg = animRef.current.get(threat.id)
        if (!seg) continue
        const cur = lerp(seg, k)
        curRef.current.set(threat.id, cur)
        markers.get(`t:${threat.id}`)?.setLngLat(toLngLat(cur))
      }
      for (const it of interceptors) {
        const seg = animRef.current.get(it.id)
        if (!seg) continue
        const cur = lerp(seg, k)
        curRef.current.set(it.id, cur)
        markers.get(`i:${it.id}`)?.setLngLat(toLngLat(cur))
        const trail = intTrailsRef.current.get(it.id) ?? []
        trail.push(toLngLat(cur))
        if (trail.length > 24) trail.shift()
        intTrailsRef.current.set(it.id, trail)
      }

      // Rebuild the GeoJSON overlays at ~10 fps (heavy; not needed every frame).
      if (tp - lastGeo < 100) return
      lastGeo = tp
      const now = Date.now()

      const trailFeatures = [...intTrailsRef.current.values()]
        .filter((t) => t.length > 1)
        .map((t) => line(t))
      setSource(map, 'int-trails', trailFeatures)

      const links = platforms.flatMap(({ report, lastSeen }) =>
        now - lastSeen > STALE_AFTER_MS
          ? []
          : report.threats.map((c) =>
              line([toLngLat(report.position), toLngLat(curRef.current.get(c.id) ?? c.position)]),
            ),
      )
      setSource(map, 'links', links)

      const platformPos = new Map(platforms.map((p) => [p.report.platform_id, p.report.position]))
      const engLines = engagements.flatMap((e) => {
        const from = platformPos.get(e.platform_id)
        const to = curRef.current.get(e.threat_id)
        return from && to ? [line([toLngLat(from), toLngLat(to)])] : []
      })
      setSource(map, 'engagements', engLines)

      const burstFeatures = []
      for (const [key, b] of burstsRef.current) {
        const age = tp - b.start
        if (age >= 700) {
          burstsRef.current.delete(key)
          continue
        }
        const p = age / 700
        burstFeatures.push({
          type: 'Feature' as const,
          geometry: { type: 'Point' as const, coordinates: toLngLat(b.position) },
          properties: { r: 8 + p * 46, op: 1 - p, color: b.kind === 'impact' ? '#ff3b4d' : '#7df9ff' },
        })
      }
      setSource(map, 'bursts', burstFeatures)
    }
    raf = requestAnimationFrame(frame)
    return () => cancelAnimationFrame(raf)
  }, [ready])

  return (
    <div ref={containerRef} className={`h-full w-full ${placing ? '[&_canvas]:cursor-crosshair' : ''}`} />
  )
}

function empty(): FeatureCollection {
  return { type: 'FeatureCollection', features: [] }
}

function lerp(seg: { from: Position; to: Position }, k: number): Position {
  return {
    x: seg.from.x + (seg.to.x - seg.from.x) * k,
    y: seg.from.y + (seg.to.y - seg.from.y) * k,
  }
}

function line(coordinates: [number, number][]) {
  return {
    type: 'Feature' as const,
    geometry: { type: 'LineString' as const, coordinates },
    properties: {},
  }
}

function setSource(map: maplibregl.Map, id: string, features: object[]) {
  ;(map.getSource(id) as GeoJSONSource | undefined)?.setData({
    type: 'FeatureCollection',
    features: features as FeatureCollection['features'],
  })
}
