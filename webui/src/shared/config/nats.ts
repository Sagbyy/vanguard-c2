declare global {
  interface Window {
    __NATS_URL__?: string
  }
}

export const NATS_WS_URL = window.__NATS_URL__ ?? 'ws://127.0.0.1:8080'

export const MAP_THREATS_SUBJECT = 'map.threats'
export const PLATFORM_REPORT_SUBJECT = 'platform.*.report'
export const ENGAGEMENTS_SUBJECT = 'control.engagements'
export const INTERCEPTORS_SUBJECT = 'control.interceptors'
export const THREAT_DESTROYED_SUBJECT = 'control.threat.destroyed'
export const LEAKER_SUBJECT = 'control.leaker'

export const MAP_CONFIG_SUBJECT = 'control.map.config'
export const PLATFORM_ADD_SUBJECT = 'control.platform.add'
export const PLATFORM_REMOVE_SUBJECT = 'control.platform.remove'
export const CONTROL_RESET_SUBJECT = 'control.reset'
export const INTERCEPTOR_RETARGET_SUBJECT = 'control.interceptor.retarget'
export const INTERCEPTOR_ABORT_SUBJECT = 'control.interceptor.abort'
