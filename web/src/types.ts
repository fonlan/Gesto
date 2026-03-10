export type Locale = 'zh-CN' | 'en-US'

export interface AppConfig {
  version: number
  locale: Locale
  general: GeneralSettings
  defaultActions: GestureBinding[]
  appRules: ApplicationRule[]
}

export interface GeneralSettings {
  trailColor: string
  trailWidth: number
  minimumDistance: number
  fadeDurationMs: number
  rightClickIdleFallbackMs: number
  rightClickIdleMovementTolerance: number
  ignoredProcessNames: string[]
  autostart: boolean
}

export interface ApplicationRule {
  id: string
  name: string
  processNames: string[]
  gestures: GestureBinding[]
}

export interface GestureBinding {
  gesture: string
  action: GestureAction
}

export type GestureAction =
  | { type: 'none' }
  | { type: 'hotkey'; hotkey: HotkeySpec }
  | { type: 'shell'; command: string }

export interface HotkeySpec {
  modifiers: string[]
  key: string
}

export interface StatusPayload {
  serverUrl: string
  configPath: string
  port: number
  appName: string
}
