import { connect, type NatsConnection } from 'nats.ws'
import { useCallback, useEffect, useRef, useState } from 'react'
import {
  CONTROL_RESET_SUBJECT,
  ENGAGEMENTS_SUBJECT,
  INTERCEPTORS_SUBJECT,
  PLATFORM_REMOVE_SUBJECT,
  type EngagementReport,
  type FlyingInterceptor,
  type InterceptorReport,
  type PlatformView,
  type Threat,
  type ThreatClassification,
} from './types'

const REMOVE_GRACE_MS = 3_000

export type ConnectionStatus = 'connecting' | 'connected' | 'offline'

const NATS_WS_URL = 'ws://127.0.0.1:8080'

/**
 * Connects to the NATS WebSocket listener and keeps the live picture:
 * ground-truth threats from `map.threats`, latest radar report per platform
 * from `platform.*.report`.
 */
export function useNats(url: string = NATS_WS_URL) {
  const [status, setStatus] = useState<ConnectionStatus>('connecting')
  const [threats, setThreats] = useState<Threat[]>([])
  const [platforms, setPlatforms] = useState<Map<string, PlatformView>>(new Map())
  // Operator picture: best classification known per track id, fused across
  // platform reports. A track stays out of this map until a platform resolves
  // it within its classification range.
  const [classifications, setClassifications] = useState<Map<string, ThreatClassification>>(
    new Map(),
  )
  const [engagements, setEngagements] = useState<EngagementReport>({ lines: [], neutralized: 0 })
  const [interceptors, setInterceptors] = useState<FlyingInterceptor[]>([])
  const connectionRef = useRef<NatsConnection | undefined>(undefined)
  // Platforms just removed — ignore any in-flight report for them briefly so
  // they don't flicker back before the host stops publishing.
  const recentlyRemoved = useRef<Map<string, number>>(new Map())

  // Publish a JSON command (config / platform add / remove) to the Rust side.
  const publish = useCallback((subject: string, payload: unknown) => {
    const data = new TextEncoder().encode(
      typeof payload === 'string' ? payload : JSON.stringify(payload),
    )
    connectionRef.current?.publish(subject, data)
  }, [])

  // Remove a platform: tell the host and drop it from the UI immediately.
  const removePlatform = useCallback(
    (platformId: string) => {
      recentlyRemoved.current.set(platformId, Date.now())
      publish(PLATFORM_REMOVE_SUBJECT, platformId)
      setPlatforms((previous) => {
        const next = new Map(previous)
        next.delete(platformId)
        return next
      })
    },
    [publish],
  )

  useEffect(() => {
    let connection: NatsConnection | undefined
    let cancelled = false
    const decoder = new TextDecoder()

    ;(async () => {
      try {
        connection = await connect({
          servers: url,
          maxReconnectAttempts: -1,
          waitOnFirstConnect: true,
        })
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
          setThreats(live)
          // Drop classifications for tracks that no longer exist.
          const liveIds = new Set(live.map((t) => t.id))
          setClassifications((previous) => {
            const next = new Map(previous)
            for (const id of next.keys()) if (!liveIds.has(id)) next.delete(id)
            return next
          })
        }
      })()

      void (async () => {
        for await (const message of connection.subscribe(ENGAGEMENTS_SUBJECT)) {
          setEngagements(JSON.parse(decoder.decode(message.data)) as EngagementReport)
        }
      })()

      void (async () => {
        for await (const message of connection.subscribe(INTERCEPTORS_SUBJECT)) {
          setInterceptors(JSON.parse(decoder.decode(message.data)) as FlyingInterceptor[])
        }
      })()

      void (async () => {
        for await (const message of connection.subscribe('platform.*.report')) {
          const report = JSON.parse(decoder.decode(message.data)) as InterceptorReport
          // Skip reports for a just-removed platform during the grace window.
          const removedAt = recentlyRemoved.current.get(report.platform_id)
          if (removedAt !== undefined) {
            if (Date.now() - removedAt < REMOVE_GRACE_MS) continue
            recentlyRemoved.current.delete(report.platform_id)
          }
          setPlatforms((previous) => {
            const next = new Map(previous)
            next.set(report.platform_id, { report, lastSeen: Date.now() })
            return next
          })
          // Record any definitive classification (a closer platform resolves it).
          setClassifications((previous) => {
            const next = new Map(previous)
            for (const contact of report.threats) {
              if (contact.classification !== 'Unknown') {
                next.set(contact.id, contact.classification)
              }
            }
            return next
          })
        }
      })()
    })()

    return () => {
      cancelled = true
      connectionRef.current = undefined
      void connection?.close()
    }
  }, [url])

  // Reset to baseline: tell the Rust side and clear the local picture so it
  // repopulates from the preset feeds.
  const reset = useCallback(() => {
    publish(CONTROL_RESET_SUBJECT, '')
    setThreats([])
    setPlatforms(new Map())
    setClassifications(new Map())
    setEngagements({ lines: [], neutralized: 0 })
    setInterceptors([])
  }, [publish])

  return {
    status,
    threats,
    platforms,
    classifications,
    engagements,
    interceptors,
    publish,
    removePlatform,
    reset,
  }
}
