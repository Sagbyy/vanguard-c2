import type { FeatureCollection } from 'geojson'
import maplibregl, { type GeoJSONSource, type StyleSpecification } from 'maplibre-gl'
import 'maplibre-gl/dist/maplibre-gl.css'
import { useEffect, useRef, useState } from 'react'
import { KYIV, rangeRing, toLngLat } from './geo'
import { STALE_AFTER_MS, type PlatformView, type Position, type Threat } from './types'

export type Basemap = 'dark' | 'sat'

interface TacticalMapProps {
  threats: Threat[]
  platforms: PlatformView[]
  basemap: Basemap
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

function threatColor(level: number): string {
  if (level >= 5) return '#ff3b4d'
  if (level >= 4) return '#ff6b35'
  if (level >= 3) return '#ffa02e'
  return '#ffd23e'
}


export function TacticalMap({ threats, platforms, basemap }: TacticalMapProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const mapRef = useRef<maplibregl.Map | null>(null)
  const markersRef = useRef(new Map<string, maplibregl.Marker>())
  // Smooth interpolation between the 1 Hz ground-truth samples.
  const animRef = useRef(new Map<string, { from: Position; to: Position }>())
  const curRef = useRef(new Map<string, Position>())
  const segRef = useRef({ start: 0, dur: 1000 })
  const lastDataMs = useRef(0)
  const loopDataRef = useRef<{
    threats: Threat[]
    platforms: PlatformView[]
  }>({ threats: [], platforms: [] })
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

    map.on('load', () => {
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
        coordinates: [rangeRing(report.position, report.range)],
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
    segRef.current = { start: performance.now(), dur }
    loopDataRef.current = { threats, platforms }

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
        `range ${(report.range / 1000).toFixed(1)} km, ${report.threats.length} contact(s)`
    }

    for (const threat of threats) {
      const key = `t:${threat.id}`
      liveIds.add(key)

      let marker = markers.get(key)
      if (!marker) {
        const el = document.createElement('div')
        el.className = 'threat-marker'
        el.innerHTML = '<div class="threat-dot"></div><span class="threat-label"></span><div class="marker-tip"></div>'
        // Initial position only — the rAF loop owns motion from here on.
        marker = new maplibregl.Marker({ element: el }).setLngLat(toLngLat(threat.position)).addTo(map)
        markers.set(key, marker)
      }
      const el = marker.getElement()
      el.classList.toggle('tracked', trackedIds.has(threat.id))
      const dot = el.querySelector('.threat-dot') as HTMLElement
      const size = 7 + threat.threat_level * 1.6
      dot.style.width = `${size}px`
      dot.style.height = `${size}px`
      dot.style.background = threatColor(threat.threat_level)
      dot.style.boxShadow = `0 0 10px 2px ${threatColor(threat.threat_level)}88`
      el.querySelector('.threat-label')!.textContent = threat.id.slice(0, 8)
      el.querySelector('.marker-tip')!.textContent =
        `HOSTILE ${threat.id.slice(0, 8)} — LVL ${threat.threat_level}, ` +
        `${threat.speed.toFixed(0)} m/s, ${(Math.hypot(threat.position.x, threat.position.y) / 1000).toFixed(2)} km from asset` +
        `${trackedIds.has(threat.id) ? ' — TRACKED' : ''}`
    }

    for (const [key, marker] of markers) {
      if (!liveIds.has(key)) {
        marker.remove()
        markers.delete(key)
        if (key.startsWith('t:')) {
          const id = key.slice(2)
          animRef.current.delete(id)
          curRef.current.delete(id)
        }
      }
    }
  }, [threats, platforms, ready])

  // Animation loop: tween threat markers + their vectors/links between samples.
  useEffect(() => {
    const map = mapRef.current
    if (!map || !ready) return

    let raf = 0
    const frame = () => {
      const { start, dur } = segRef.current
      const k = dur > 0 ? Math.min(1, (performance.now() - start) / dur) : 1
      const { threats, platforms } = loopDataRef.current
      const markers = markersRef.current
      const now = Date.now()

      for (const threat of threats) {
        const seg = animRef.current.get(threat.id)
        if (!seg) continue
        const cur: Position = {
          x: seg.from.x + (seg.to.x - seg.from.x) * k,
          y: seg.from.y + (seg.to.y - seg.from.y) * k,
        }
        curRef.current.set(threat.id, cur)
        markers.get(`t:${threat.id}`)?.setLngLat(toLngLat(cur))
      }

      const links = platforms.flatMap(({ report, lastSeen }) =>
        now - lastSeen > STALE_AFTER_MS
          ? []
          : report.threats.map((contact) => ({
              type: 'Feature' as const,
              geometry: {
                type: 'LineString' as const,
                coordinates: [
                  toLngLat(report.position),
                  toLngLat(curRef.current.get(contact.id) ?? contact.position),
                ],
              },
              properties: {},
            })),
      )

      ;(map.getSource('links') as GeoJSONSource | undefined)?.setData({
        type: 'FeatureCollection',
        features: links,
      })

      raf = requestAnimationFrame(frame)
    }
    raf = requestAnimationFrame(frame)
    return () => cancelAnimationFrame(raf)
  }, [ready])

  return <div ref={containerRef} className="h-full w-full" />
}

function empty(): FeatureCollection {
  return { type: 'FeatureCollection', features: [] }
}
