import type { FeatureCollection } from 'geojson'
import maplibregl, { type GeoJSONSource, type StyleSpecification } from 'maplibre-gl'
import 'maplibre-gl/dist/maplibre-gl.css'
import { useEffect, useRef, useState } from 'react'
import { KYIV, rangeRing, toLngLat } from './geo'
import { STALE_AFTER_MS, type PlatformView, type Position, type Threat } from './types'

interface TacticalMapProps {
  threats: Threat[]
  platforms: PlatformView[]
}

const DARK_BASEMAP: StyleSpecification = {
  version: 8,
  sources: {
    basemap: {
      type: 'raster',
      tiles: ['a', 'b', 'c', 'd'].map(
        (s) => `https://${s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}.png`,
      ),
      tileSize: 256,
      attribution: '© OpenStreetMap contributors © CARTO',
    },
  },
  layers: [{ id: 'basemap', type: 'raster', source: 'basemap' }],
}

function threatColor(level: number): string {
  if (level >= 5) return '#ff3b4d'
  if (level >= 4) return '#ff6b35'
  if (level >= 3) return '#ffa02e'
  return '#ffd23e'
}

/** 20 s projection of the threat's course (it flies straight at the asset). */
function vectorEnd(threat: Threat): Position {
  const distance = Math.hypot(threat.position.x, threat.position.y)
  if (distance < 1) return threat.position
  const projection = 20 * threat.speed
  return {
    x: threat.position.x - (threat.position.x / distance) * projection,
    y: threat.position.y - (threat.position.y / distance) * projection,
  }
}

export function TacticalMap({ threats, platforms }: TacticalMapProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const mapRef = useRef<maplibregl.Map | null>(null)
  const markersRef = useRef(new Map<string, maplibregl.Marker>())
  const [ready, setReady] = useState(false)

  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    const map = new maplibregl.Map({
      container,
      style: DARK_BASEMAP,
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

      map.addSource('vectors', { type: 'geojson', data: empty() })
      map.addLayer({
        id: 'vectors',
        type: 'line',
        source: 'vectors',
        paint: {
          'line-color': ['get', 'color'],
          'line-width': 1.6,
          'line-opacity': 0.8,
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

    // --- GeoJSON layers: range bubbles, detection links, velocity vectors.
    const ranges = platforms.map(({ report, lastSeen }) => ({
      type: 'Feature' as const,
      geometry: {
        type: 'Polygon' as const,
        coordinates: [rangeRing(report.position, report.range)],
      },
      properties: { stale: now - lastSeen > STALE_AFTER_MS },
    }))

    const links = platforms.flatMap(({ report, lastSeen }) =>
      now - lastSeen > STALE_AFTER_MS
        ? []
        : report.threats.map((contact) => ({
            type: 'Feature' as const,
            geometry: {
              type: 'LineString' as const,
              coordinates: [toLngLat(report.position), toLngLat(contact.position)],
            },
            properties: {},
          })),
    )

    const vectors = threats.map((threat) => ({
      type: 'Feature' as const,
      geometry: {
        type: 'LineString' as const,
        coordinates: [toLngLat(threat.position), toLngLat(vectorEnd(threat))],
      },
      properties: { color: threatColor(threat.threat_level) },
    }))

    ;(map.getSource('ranges') as GeoJSONSource).setData({ type: 'FeatureCollection', features: ranges })
    ;(map.getSource('links') as GeoJSONSource).setData({ type: 'FeatureCollection', features: links })
    ;(map.getSource('vectors') as GeoJSONSource).setData({ type: 'FeatureCollection', features: vectors })

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
        marker = new maplibregl.Marker({ element: el }).setLngLat(toLngLat(threat.position)).addTo(map)
        markers.set(key, marker)
      }
      marker.setLngLat(toLngLat(threat.position))
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
      }
    }
  }, [threats, platforms, ready])

  return <div ref={containerRef} className="h-full w-full" />
}

function empty(): FeatureCollection {
  return { type: 'FeatureCollection', features: [] }
}
