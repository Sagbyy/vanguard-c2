import { connect, type NatsConnection } from 'nats.ws'
import { useEffect, useState } from 'react'
import type { InterceptorReport, PlatformView, Threat } from './types'

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
          setThreats(JSON.parse(decoder.decode(message.data)) as Threat[])
        }
      })()

      void (async () => {
        for await (const message of connection.subscribe('platform.*.report')) {
          const report = JSON.parse(decoder.decode(message.data)) as InterceptorReport
          setPlatforms((previous) => {
            const next = new Map(previous)
            next.set(report.platform_id, { report, lastSeen: Date.now() })
            return next
          })
        }
      })()
    })()

    return () => {
      cancelled = true
      void connection?.close()
    }
  }, [url])

  return { status, threats, platforms }
}
