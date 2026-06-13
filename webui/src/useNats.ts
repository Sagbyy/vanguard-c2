import { connect, type NatsConnection } from 'nats.ws'
import { useCallback, useEffect, useRef, useState } from 'react'
import {
  CONTROL_RESET_SUBJECT,
  ENGAGEMENTS_SUBJECT,
  INTERCEPTOR_ABORT_SUBJECT,
  INTERCEPTOR_RETARGET_SUBJECT,
  INTERCEPTORS_SUBJECT,
  LEAKER_SUBJECT,
  PLATFORM_REMOVE_SUBJECT,
  THREAT_DESTROYED_SUBJECT,
  type Burst,
  type EngagementReport,
  type FeedEvent,
  type FlyingInterceptor,
  type InterceptorReport,
  type PlatformView,
  type Threat,
  type ThreatClassification,
  type ThreatDestroyed,
} from './types'

const REMOVE_GRACE_MS = 3_000
// Coalesce the high-frequency feeds into React state at this cadence instead of
// on every NATS message (4 Hz × ~9 platforms ≈ 50 msg/s would re-render that often).
const FLUSH_MS = 150

export type ConnectionStatus = 'connecting' | 'connected' | 'offline'

const NATS_WS_URL = 'ws://127.0.0.1:8080'

/**
 * Connects to the NATS WebSocket listener and keeps the live picture. Incoming
 * high-rate messages are buffered in refs and flushed to React state at a
 * bounded cadence; rare events (kills, impacts) update state immediately.
 */
export function useNats(url: string = NATS_WS_URL) {
  const [status, setStatus] = useState<ConnectionStatus>('connecting')
  const [threats, setThreats] = useState<Threat[]>([])
  const [platforms, setPlatforms] = useState<Map<string, PlatformView>>(new Map())
  const [classifications, setClassifications] = useState<Map<string, ThreatClassification>>(new Map())
  const [engagements, setEngagements] = useState<EngagementReport>({ lines: [], neutralized: 0, safe_zones: [] })
  const [interceptors, setInterceptors] = useState<FlyingInterceptor[]>([])
  const [feed, setFeed] = useState<FeedEvent[]>([])
  const [bursts, setBursts] = useState<Burst[]>([])
  const [impacts, setImpacts] = useState(0)

  const connectionRef = useRef<NatsConnection | undefined>(undefined)
  const eventKey = useRef(0)
  const recentlyRemoved = useRef<Map<string, number>>(new Map())

  // Buffers for the high-rate feeds + a dirty flag driving the flush.
  const threatsRef = useRef<Threat[]>([])
  const platformsRef = useRef<Map<string, PlatformView>>(new Map())
  const classRef = useRef<Map<string, ThreatClassification>>(new Map())
  const engagementsRef = useRef<EngagementReport>({ lines: [], neutralized: 0, safe_zones: [] })
  const interceptorsRef = useRef<FlyingInterceptor[]>([])
  const dirty = useRef(false)

  const publish = useCallback((subject: string, payload: unknown) => {
    const data = new TextEncoder().encode(
      typeof payload === 'string' ? payload : JSON.stringify(payload),
    )
    connectionRef.current?.publish(subject, data)
  }, [])

  const removePlatform = useCallback(
    (platformId: string) => {
      recentlyRemoved.current.set(platformId, Date.now())
      publish(PLATFORM_REMOVE_SUBJECT, platformId)
      platformsRef.current.delete(platformId)
      dirty.current = true
    },
    [publish],
  )

  const retargetInterceptor = useCallback(
    (interceptorId: string, targetId: string) =>
      publish(INTERCEPTOR_RETARGET_SUBJECT, { interceptor_id: interceptorId, target_id: targetId }),
    [publish],
  )
  const abortInterceptor = useCallback(
    (interceptorId: string) => publish(INTERCEPTOR_ABORT_SUBJECT, interceptorId),
    [publish],
  )

  // Flush buffered state to React at a bounded rate.
  useEffect(() => {
    const id = setInterval(() => {
      if (!dirty.current) return
      dirty.current = false
      setThreats(threatsRef.current)
      setPlatforms(new Map(platformsRef.current))
      setClassifications(new Map(classRef.current))
      setEngagements(engagementsRef.current)
      setInterceptors(interceptorsRef.current)
    }, FLUSH_MS)
    return () => clearInterval(id)
  }, [])

  useEffect(() => {
    let connection: NatsConnection | undefined
    let cancelled = false
    const decoder = new TextDecoder()

    ;(async () => {
      try {
        connection = await connect({ servers: url, maxReconnectAttempts: -1, waitOnFirstConnect: true })
      } catch {
        if (!cancelled) setStatus('offline')
        return
      }
      if (cancelled) {
        void connection.close()
        return
      }
      connectionRef.current = connection
      setStatus('connected')

      void (async () => {
        for await (const event of connection.status()) {
          if (cancelled) return
          if (event.type === 'disconnect') setStatus('connecting')
          if (event.type === 'reconnect') setStatus('connected')
        }
      })()

      void (async () => {
        for await (const message of connection.subscribe('map.threats')) {
          const live = JSON.parse(decoder.decode(message.data)) as Threat[]
          threatsRef.current = live
          const liveIds = new Set(live.map((t) => t.id))
          for (const id of classRef.current.keys()) if (!liveIds.has(id)) classRef.current.delete(id)
          dirty.current = true
        }
      })()

      void (async () => {
        for await (const message of connection.subscribe(ENGAGEMENTS_SUBJECT)) {
          engagementsRef.current = JSON.parse(decoder.decode(message.data)) as EngagementReport
          dirty.current = true
        }
      })()

      void (async () => {
        for await (const message of connection.subscribe(INTERCEPTORS_SUBJECT)) {
          interceptorsRef.current = JSON.parse(decoder.decode(message.data)) as FlyingInterceptor[]
          dirty.current = true
        }
      })()

      void (async () => {
        for await (const message of connection.subscribe('platform.*.report')) {
          const report = JSON.parse(decoder.decode(message.data)) as InterceptorReport
          const removedAt = recentlyRemoved.current.get(report.platform_id)
          if (removedAt !== undefined) {
            if (Date.now() - removedAt < REMOVE_GRACE_MS) continue
            recentlyRemoved.current.delete(report.platform_id)
          }
          platformsRef.current.set(report.platform_id, { report, lastSeen: Date.now() })
          for (const contact of report.threats) {
            if (contact.classification !== 'Unknown') {
              classRef.current.set(contact.id, contact.classification)
            }
          }
          dirty.current = true
        }
      })()

      // --- Rare events: update state immediately for snappy feedback.
      const clock = () => new Date().toISOString().slice(11, 19)
      const pushFeed = (event: FeedEvent) => setFeed((prev) => [event, ...prev].slice(0, 14))
      const pushBurst = (burst: Burst) => setBursts((prev) => [...prev, burst].slice(-30))

      void (async () => {
        for await (const message of connection.subscribe(THREAT_DESTROYED_SUBJECT)) {
          const d = JSON.parse(decoder.decode(message.data)) as ThreatDestroyed
          const key = eventKey.current++
          pushFeed({ key, time: clock(), kind: 'kill', text: `NEUTRALIZED ${d.id.slice(0, 8)}` })
          pushBurst({ key, position: d.position, kind: 'kill' })
        }
      })()

      void (async () => {
        for await (const message of connection.subscribe(LEAKER_SUBJECT)) {
          const t = JSON.parse(decoder.decode(message.data)) as Threat
          const key = eventKey.current++
          if (t.is_decoy) {
            pushFeed({ key, time: clock(), kind: 'decoy', text: `decoy spent ${t.id.slice(0, 8)}` })
          } else {
            setImpacts((n) => n + 1)
            pushFeed({ key, time: clock(), kind: 'impact', text: `⚠ IMPACT ${t.id.slice(0, 8)} @ KYIV` })
            pushBurst({ key, position: t.position, kind: 'impact' })
          }
        }
      })()
    })()

    return () => {
      cancelled = true
      connectionRef.current = undefined
      void connection?.close()
    }
  }, [url])

  const reset = useCallback(() => {
    publish(CONTROL_RESET_SUBJECT, '')
    threatsRef.current = []
    platformsRef.current = new Map()
    classRef.current = new Map()
    engagementsRef.current = { lines: [], neutralized: 0, safe_zones: [] }
    interceptorsRef.current = []
    setThreats([])
    setPlatforms(new Map())
    setClassifications(new Map())
    setEngagements({ lines: [], neutralized: 0, safe_zones: [] })
    setInterceptors([])
    setFeed([])
    setBursts([])
    setImpacts(0)
  }, [publish])

  return {
    status,
    threats,
    platforms,
    classifications,
    engagements,
    interceptors,
    feed,
    bursts,
    impacts,
    publish,
    removePlatform,
    retargetInterceptor,
    abortInterceptor,
    reset,
  }
}
