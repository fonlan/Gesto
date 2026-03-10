import { KeyboardEvent, useEffect, useMemo, useState } from 'react'

import { messages, type I18nText } from './i18n'
import type {
  AppConfig,
  ApplicationRule,
  GestureAction,
  GestureBinding,
  HotkeySpec,
  Locale,
  StatusPayload
} from './types'

const DIRECTION_BUTTONS = ['U', 'D', 'L', 'R'] as const
const HOTKEY_MODIFIER_ORDER = ['Ctrl', 'Alt', 'Shift', 'Win'] as const
const HOTKEY_KEY_OPTIONS = [
  ...Array.from({ length: 26 }, (_, index) => 'Key' + String.fromCharCode(65 + index)),
  ...Array.from({ length: 10 }, (_, index) => 'Digit' + index),
  ...Array.from({ length: 24 }, (_, index) => 'F' + (index + 1)),
  'ArrowLeft',
  'ArrowRight',
  'ArrowUp',
  'ArrowDown',
  'Enter',
  'Tab',
  'Space',
  'Backspace',
  'Delete',
  'Escape',
  'Home',
  'End',
  'PageUp',
  'PageDown'
]
const GLOBAL_RULE_ID = '__global__'

const createEmptyBinding = (): GestureBinding => ({
  gesture: '',
  description: '',
  action: {
    type: 'hotkey',
    hotkey: { modifiers: [], key: '' }
  }
})

const createEmptyRule = (): ApplicationRule => ({
  id: crypto.randomUUID(),
  name: '',
  processNames: [],
  gestures: [createEmptyBinding()]
})

const normalizeGesture = (value: string) =>
  value
    .toUpperCase()
    .split('')
    .filter((item) => DIRECTION_BUTTONS.includes(item as (typeof DIRECTION_BUTTONS)[number]))
    .join('')

const formatProcessNames = (processNames: string[], fallback: string) =>
  processNames.length > 0 ? processNames.join(', ') : fallback

const parseProcessNames = (value: string) =>
  value
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean)

const getRuleEditorTitle = (rule: ApplicationRule, text: I18nText) => {
  const trimmedName = rule.name.trim()
  if (trimmedName) {
    return trimmedName
  }

  return formatProcessNames(rule.processNames, text.emptyProcessNames)
}

const formatHotkey = (hotkey?: HotkeySpec) => {
  if (!hotkey) {
    return ''
  }

  const normalizedHotkey = normalizeHotkey(hotkey)
  const parts = [...normalizedHotkey.modifiers]
  if (normalizedHotkey.key) {
    parts.push(formatKeyName(normalizedHotkey.key))
  }
  return parts.join(' + ')
}

const formatKeyName = (key: string) => {
  if (key.startsWith('Key')) {
    return key.slice(3)
  }
  if (key.startsWith('Digit')) {
    return key.slice(5)
  }
  return key.replace('Arrow', '')
}

const normalizeHotkeyModifier = (
  modifier: string
): ((typeof HOTKEY_MODIFIER_ORDER)[number]) | null => {
  switch (modifier) {
    case 'Ctrl':
    case 'Alt':
    case 'Shift':
    case 'Win':
      return modifier
    default:
      return null
  }
}

const normalizeHotkey = (hotkey: HotkeySpec): HotkeySpec => ({
  modifiers: HOTKEY_MODIFIER_ORDER.filter((modifier) =>
    hotkey.modifiers.map(normalizeHotkeyModifier).includes(modifier)
  ),
  key: hotkey.key
})

const toggleHotkeyModifier = (
  hotkey: HotkeySpec,
  modifier: (typeof HOTKEY_MODIFIER_ORDER)[number]
): HotkeySpec => {
  const modifiers = hotkey.modifiers.includes(modifier)
    ? hotkey.modifiers.filter((item) => item !== modifier)
    : [...hotkey.modifiers, modifier]

  return normalizeHotkey({ ...hotkey, modifiers })
}

const normalizeHotkeyFromEvent = (event: KeyboardEvent<HTMLInputElement>): HotkeySpec | null => {
  const ignored = new Set(['Control', 'Shift', 'Alt', 'Meta', 'OS'])
  if (ignored.has(event.key)) {
    return null
  }

  const specialMap: Record<string, string> = {
    ArrowLeft: 'ArrowLeft',
    ArrowRight: 'ArrowRight',
    ArrowUp: 'ArrowUp',
    ArrowDown: 'ArrowDown',
    Enter: 'Enter',
    Tab: 'Tab',
    ' ': 'Space',
    Backspace: 'Backspace',
    Delete: 'Delete',
    Escape: 'Escape',
    Home: 'Home',
    End: 'End',
    PageUp: 'PageUp',
    PageDown: 'PageDown'
  }

  let key = specialMap[event.key]
  if (!key && /^F\d{1,2}$/.test(event.key)) {
    key = event.key.toUpperCase()
  }
  if (!key && (/^Key[A-Z]$/.test(event.code) || /^Digit\d$/.test(event.code))) {
    key = event.code
  }
  if (!key && event.key.length === 1 && /[a-z0-9]/i.test(event.key)) {
    key = /\d/.test(event.key) ? 'Digit' + event.key : 'Key' + event.key.toUpperCase()
  }

  if (!key) {
    return null
  }

  const modifiers = [
    event.ctrlKey ? 'Ctrl' : null,
    event.altKey ? 'Alt' : null,
    event.shiftKey ? 'Shift' : null,
    event.metaKey ? 'Win' : null
  ].filter(Boolean) as string[]

  return normalizeHotkey({ modifiers, key })
}

const normalizeActionType = (type: GestureAction['type']): GestureAction => {
  if (type === 'none') {
    return { type: 'none' }
  }
  if (type === 'shell') {
    return { type: 'shell', command: '' }
  }
  return {
    type: 'hotkey',
    hotkey: {
      modifiers: [],
      key: ''
    }
  }
}

export default function App() {
  const [config, setConfig] = useState<AppConfig | null>(null)
  const [status, setStatus] = useState<StatusPayload | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [message, setMessage] = useState<string>('')
  const [error, setError] = useState<string>('')
  const [selectedRuleId, setSelectedRuleId] = useState<string>(GLOBAL_RULE_ID)
  const fallbackText = messages['zh-CN']

  useEffect(() => {
    const load = async () => {
      try {
        const [configResponse, statusResponse] = await Promise.all([
          fetch('/api/config'),
          fetch('/api/status')
        ])
        if (!configResponse.ok || !statusResponse.ok) {
          const configMessage = configResponse.ok ? '' : await configResponse.text()
          const statusMessage = statusResponse.ok ? '' : await statusResponse.text()
          throw new Error(configMessage || statusMessage || fallbackText.fetchConfigFailed)
        }

        const configPayload = (await configResponse.json()) as AppConfig
        const statusPayload = (await statusResponse.json()) as StatusPayload
        setConfig(configPayload)
        setStatus(statusPayload)
      } catch (loadError) {
        setError(loadError instanceof Error ? loadError.message : fallbackText.unknownError)
      } finally {
        setLoading(false)
      }
    }

    void load()
  }, [])

  const locale: Locale = config?.locale ?? 'zh-CN'
  const t = useMemo(() => messages[locale], [locale])

  useEffect(() => {
    if (!config || selectedRuleId === GLOBAL_RULE_ID) {
      return
    }

    if (!config.appRules.some((rule) => rule.id === selectedRuleId)) {
      setSelectedRuleId(GLOBAL_RULE_ID)
    }
  }, [config, selectedRuleId])

  const patchConfig = (updater: (current: AppConfig) => AppConfig) => {
    setConfig((current) => (current ? updater(current) : current))
  }

  const selectedRuleIndex = config?.appRules.findIndex((rule) => rule.id === selectedRuleId) ?? -1
  const selectedRule = selectedRuleIndex >= 0 && config ? config.appRules[selectedRuleIndex] : null
  const totalBindings =
    (config?.defaultActions.length ?? 0) +
    (config?.appRules.reduce((count, rule) => count + rule.gestures.length, 0) ?? 0)

  const saveConfig = async () => {
    if (!config) {
      return
    }

    setSaving(true)
    setError('')
    setMessage('')
    try {
      const response = await fetch('/api/config', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(config)
      })
      if (!response.ok) {
        throw new Error(await response.text())
      }

      const updated = (await response.json()) as AppConfig
      setConfig(updated)
      setMessage(t.saved)
    } catch (saveError) {
      setError(t.saveFailed + ': ' + (saveError instanceof Error ? saveError.message : t.unknownError))
    } finally {
      setSaving(false)
    }
  }

  if (loading) {
    return <div className='p-10 text-center text-slate-500'>{t.loading}</div>
  }

  if (!config) {
    return <div className='p-10 text-center text-red-500'>{error || t.noConfigLoaded}</div>
  }

  return (
    <div className='min-h-screen bg-slate-100 px-4 py-4 text-slate-900 sm:px-6 sm:py-6'>
      <div className='mx-auto flex max-w-[88rem] flex-col gap-4'>
        <header className='overflow-hidden rounded-[1.75rem] bg-gradient-to-r from-slate-950 via-slate-900 to-cyan-900 text-white shadow-panel'>
          <div className='flex flex-col gap-4 px-5 py-5 lg:flex-row lg:items-end lg:justify-between'>
            <div className='space-y-3'>
              <div className='inline-flex rounded-full border border-white/10 bg-white/10 px-3 py-1 text-xs font-semibold uppercase tracking-[0.2em] text-cyan-50'>
                Gesto
              </div>
              <div>
                <h1 className='text-2xl font-semibold sm:text-3xl'>{t.title}</h1>
                <p className='mt-1 max-w-3xl text-sm leading-6 text-cyan-50/80'>{t.subtitle}</p>
              </div>
              <div className='flex flex-wrap gap-2'>
                <OverviewStat label={t.defaultRules} value={config.defaultActions.length} />
                <OverviewStat label={t.appRules} value={config.appRules.length} />
                <OverviewStat label={t.gesture} value={totalBindings} />
              </div>
            </div>
            <div className='flex w-full flex-col gap-3 lg:w-auto lg:min-w-[18rem] lg:max-w-sm'>
              <HeaderPillSwitch
                checked={config.general.gesturesEnabled}
                hint={t.gesturesEnabledHint}
                label={t.gesturesEnabled}
                onChange={(checked) =>
                  patchConfig((current) => ({
                    ...current,
                    general: { ...current.general, gesturesEnabled: checked }
                  }))
                }
              />
              <button className='btn-primary w-full' onClick={saveConfig} disabled={saving} type='button'>
                {saving ? t.saving : t.save}
              </button>
              {(message || error) && (
                <div
                  aria-live='polite'
                  className={
                    'rounded-2xl px-4 py-3 text-sm font-medium ' +
                    (error ? 'bg-red-500/15 text-red-100' : 'bg-emerald-500/15 text-emerald-100')
                  }
                  role={error ? 'alert' : 'status'}
                >
                  {error || message}
                </div>
              )}
            </div>
          </div>
        </header>

        <div className='grid gap-4 xl:grid-cols-[minmax(0,1fr)_19rem]'>
          <section className='panel'>
            <div className='border-b border-slate-100 pb-4'>
              <h2 className='text-lg font-semibold text-slate-900'>{t.globalSettings}</h2>
              <p className='mt-1 text-sm text-slate-500'>{t.directionHint}</p>
            </div>

            <div className='mt-4 grid gap-3 lg:grid-cols-3'>
              <label className='setting-card'>
                <span className='field-label'>{t.language}</span>
                <select
                  className='text-input'
                  value={config.locale}
                  onChange={(event) =>
                    patchConfig((current) => ({ ...current, locale: event.target.value as Locale }))
                  }
                >
                  <option value='zh-CN'>简体中文</option>
                  <option value='en-US'>English</option>
                </select>
              </label>

              <label className='setting-card'>
                <span className='field-label'>{t.trailColor}</span>
                <input
                  className='h-11 w-full rounded-xl border border-slate-200 bg-white px-2 py-2'
                  type='color'
                  value={config.general.trailColor}
                  onChange={(event) =>
                    patchConfig((current) => ({
                      ...current,
                      general: { ...current.general, trailColor: event.target.value }
                    }))
                  }
                />
              </label>

              <label className='setting-card flex cursor-pointer items-center justify-between gap-4'>
                <div>
                  <span className='field-label'>{t.autostart}</span>
                  <span className='text-sm font-medium text-slate-700'>
                    {config.general.autostart ? '✓' : '—'}
                  </span>
                </div>
                <input
                  aria-label={t.autostart}
                  type='checkbox'
                  className='h-4 w-4 rounded border-slate-300 text-blue-600 focus:ring-blue-500'
                  checked={config.general.autostart}
                  onChange={(event) =>
                    patchConfig((current) => ({
                      ...current,
                      general: { ...current.general, autostart: event.target.checked }
                    }))
                  }
                />
              </label>

              <SliderNumberField
                className='lg:col-span-2'
                label={t.trailOpacity}
                min={0}
                max={100}
                step={1}
                value={config.general.trailOpacity}
                onChange={(value) =>
                  patchConfig((current) => ({
                    ...current,
                    general: { ...current.general, trailOpacity: value }
                  }))
                }
              />

              <NumberField
                label={t.trailWidth}
                min={1}
                max={24}
                step={0.5}
                value={config.general.trailWidth}
                onChange={(value) =>
                  patchConfig((current) => ({
                    ...current,
                    general: { ...current.general, trailWidth: value }
                  }))
                }
              />

              <NumberField
                label={t.minimumDistance}
                min={8}
                max={120}
                step={1}
                value={config.general.minimumDistance}
                onChange={(value) =>
                  patchConfig((current) => ({
                    ...current,
                    general: { ...current.general, minimumDistance: value }
                  }))
                }
              />

              <NumberField
                label={t.fadeDuration}
                min={60}
                max={2000}
                step={10}
                value={config.general.fadeDurationMs}
                onChange={(value) =>
                  patchConfig((current) => ({
                    ...current,
                    general: { ...current.general, fadeDurationMs: value }
                  }))
                }
              />


              <label className='setting-card lg:col-span-3'>
                <span className='field-label'>{t.ignoredProcessNames}</span>
                <textarea
                  className='text-input min-h-20 resize-y'
                  value={config.general.ignoredProcessNames.join(', ')}
                  onChange={(event) =>
                    patchConfig((current) => ({
                      ...current,
                      general: {
                        ...current.general,
                        ignoredProcessNames: parseProcessNames(event.target.value)
                      }
                    }))
                  }
                />
                <span className='mt-2 block text-xs leading-5 text-slate-500'>{t.ignoredProcessHint}</span>
              </label>
            </div>
          </section>

          <section className='panel xl:sticky xl:top-4 xl:h-fit'>
            <h2 className='text-lg font-semibold text-slate-900'>{t.status}</h2>
            <div className='mt-4 space-y-3 text-sm'>
              <InfoRow label={t.serverUrl} value={status?.serverUrl ?? '-'} />
              <InfoRow label={t.configPath} value={status?.configPath ?? '-'} />
            </div>
          </section>
        </div>

        <section className='panel'>
          <div className='flex flex-col gap-3 border-b border-slate-100 pb-4 sm:flex-row sm:items-start sm:justify-between'>
            <div>
              <h2 className='text-lg font-semibold text-slate-900'>{t.appRules}</h2>
              <p className='mt-1 text-sm text-slate-500'>{t.processRulesHint}</p>
            </div>
            <button
              className='btn-secondary shrink-0'
              onClick={() => {
                const nextRule = createEmptyRule()
                setSelectedRuleId(nextRule.id)
                patchConfig((current) => ({ ...current, appRules: [...current.appRules, nextRule] }))
              }}
              type='button'
            >
              {t.addRule}
            </button>
          </div>

          <div className='mt-4 grid gap-4 xl:grid-cols-[16rem_minmax(0,1fr)]'>
            <div className='rounded-[1.5rem] border border-slate-200 bg-slate-50/80 p-2.5 shadow-sm xl:sticky xl:top-4 xl:max-h-[calc(100vh-2rem)]'>
              <div className='max-h-[calc(100vh-15rem)] space-y-2 overflow-y-auto pr-1'>
                <ProcessRuleListItem
                  title={t.globalProcessName}
                  subtitle={t.globalProcessHint}
                  processNames={[]}
                  selected={selectedRuleId === GLOBAL_RULE_ID}
                  onClick={() => setSelectedRuleId(GLOBAL_RULE_ID)}
                />

                {config.appRules.map((rule) => (
                  <ProcessRuleListItem
                    key={rule.id}
                    title={formatProcessNames(rule.processNames, t.emptyProcessNames)}
                    subtitle={rule.name.trim() || t.unnamedRule}
                    processNames={rule.processNames}
                    selected={selectedRuleId === rule.id}
                    onClick={() => setSelectedRuleId(rule.id)}
                  />
                ))}
              </div>
            </div>

            <div className='rounded-[1.5rem] border border-slate-200 bg-slate-50/80 p-4'>
              {selectedRuleId === GLOBAL_RULE_ID || !selectedRule ? (
                <>
                  <div className='flex flex-col gap-3 border-b border-slate-200 pb-4 sm:flex-row sm:items-start sm:justify-between'>
                    <div>
                      <h3 className='text-lg font-semibold text-slate-900'>{t.defaultRules}</h3>
                      <p className='mt-1 text-sm text-slate-500'>{t.globalProcessHint}</p>
                    </div>
                    <button
                      className='btn-secondary shrink-0'
                      onClick={() =>
                        patchConfig((current) => ({
                          ...current,
                          defaultActions: [...current.defaultActions, createEmptyBinding()]
                        }))
                      }
                      type='button'
                    >
                      {t.addBinding}
                    </button>
                  </div>

                  <div className='mt-4 space-y-3'>
                    {config.defaultActions.map((binding, index) => (
                      <BindingEditor
                        key={'default-' + index}
                        binding={binding}
                        text={t}
                        onChange={(nextBinding) =>
                          patchConfig((current) => ({
                            ...current,
                            defaultActions: current.defaultActions.map((item, itemIndex) =>
                              itemIndex === index ? nextBinding : item
                            )
                          }))
                        }
                        onDelete={() =>
                          patchConfig((current) => ({
                            ...current,
                            defaultActions: current.defaultActions.filter((_, itemIndex) => itemIndex !== index)
                          }))
                        }
                      />
                    ))}
                  </div>
                </>
              ) : (
                <>
                  <div className='flex flex-col gap-3 border-b border-slate-200 pb-4 xl:flex-row xl:items-start xl:justify-between'>
                    <div>
                      <h3 className='text-lg font-semibold text-slate-900'>{getRuleEditorTitle(selectedRule, t)}</h3>
                      <p className='mt-1 text-sm text-slate-500'>{t.processHint}</p>
                    </div>
                    <button
                      className='btn-danger xl:min-w-24'
                      onClick={() => {
                        setSelectedRuleId(GLOBAL_RULE_ID)
                        patchConfig((current) => ({
                          ...current,
                          appRules: current.appRules.filter((item) => item.id !== selectedRule.id)
                        }))
                      }}
                      type='button'
                    >
                      {t.delete}
                    </button>
                  </div>

                  <div className='mt-4 grid gap-3 lg:grid-cols-[minmax(0,0.85fr)_minmax(0,1.15fr)]'>
                    <label className='setting-card'>
                      <span className='field-label'>{t.ruleName}</span>
                      <input
                        aria-label={t.ruleName}
                        autoComplete='off'
                        className='text-input'
                        value={selectedRule.name}
                        onChange={(event) =>
                          patchConfig((current) => ({
                            ...current,
                            appRules: current.appRules.map((item, index) =>
                              index === selectedRuleIndex ? { ...item, name: event.target.value } : item
                            )
                          }))
                        }
                      />
                    </label>

                    <label className='setting-card'>
                      <span className='field-label'>{t.processNames}</span>
                      <input
                        aria-label={t.processNames}
                        autoComplete='off'
                        className='text-input'
                        value={selectedRule.processNames.join(', ')}
                        onChange={(event) => {
                          const processNames = parseProcessNames(event.target.value)

                          patchConfig((current) => ({
                            ...current,
                            appRules: current.appRules.map((item, index) =>
                              index === selectedRuleIndex ? { ...item, processNames } : item
                            )
                          }))
                        }}
                      />
                    </label>
                  </div>

                  <div className='mt-4 space-y-3'>
                    {selectedRule.gestures.map((binding, bindingIndex) => (
                      <BindingEditor
                        key={selectedRule.id + '-' + bindingIndex}
                        binding={binding}
                        text={t}
                        onChange={(nextBinding) =>
                          patchConfig((current) => ({
                            ...current,
                            appRules: current.appRules.map((item, index) =>
                              index === selectedRuleIndex
                                ? {
                                    ...item,
                                    gestures: item.gestures.map((gestureItem, gestureIndex) =>
                                      gestureIndex === bindingIndex ? nextBinding : gestureItem
                                    )
                                  }
                                : item
                            )
                          }))
                        }
                        onDelete={() =>
                          patchConfig((current) => ({
                            ...current,
                            appRules: current.appRules.map((item, index) =>
                              index === selectedRuleIndex
                                ? {
                                    ...item,
                                    gestures: item.gestures.filter((_, gestureIndex) => gestureIndex !== bindingIndex)
                                  }
                                : item
                            )
                          }))
                        }
                      />
                    ))}
                  </div>

                  <button
                    className='btn-secondary mt-4'
                    onClick={() =>
                      patchConfig((current) => ({
                        ...current,
                        appRules: current.appRules.map((item, index) =>
                          index === selectedRuleIndex
                            ? { ...item, gestures: [...item.gestures, createEmptyBinding()] }
                            : item
                        )
                      }))
                    }
                    type='button'
                  >
                    {t.addBinding}
                  </button>
                </>
              )}
            </div>
          </div>
        </section>
      </div>
    </div>
  )
}

function SliderNumberField(props: {
  className?: string
  label: string
  value: number
  min: number
  max: number
  step: number
  onChange: (value: number) => void
}) {
  return (
    <label className={'setting-card ' + (props.className ?? '')}>
      <span className='field-label'>{props.label}</span>
      <div className='flex flex-col gap-2 sm:flex-row sm:items-center'>
        <input
          className='h-1.5 min-w-0 w-full flex-1 cursor-pointer accent-blue-600'
          type='range'
          min={props.min}
          max={props.max}
          step={props.step}
          value={props.value}
          onChange={(event) => props.onChange(Number(event.target.value))}
        />
        <input
          aria-label={props.label}
          autoComplete='off'
          className='text-input w-full shrink-0 sm:w-20'
          type='number'
          min={props.min}
          max={props.max}
          step={props.step}
          value={props.value}
          onChange={(event) => props.onChange(Number(event.target.value))}
        />
      </div>
    </label>
  )
}

function NumberField(props: {
  className?: string
  label: string
  value: number
  min: number
  max: number
  step: number
  onChange: (value: number) => void
}) {
  return (
    <label className={'setting-card ' + (props.className ?? '')}>
      <span className='field-label'>{props.label}</span>
      <input
        aria-label={props.label}
        autoComplete='off'
        className='text-input'
        type='number'
        min={props.min}
        max={props.max}
        step={props.step}
        value={props.value}
        onChange={(event) => props.onChange(Number(event.target.value))}
      />
    </label>
  )
}

function OverviewStat({ label, value }: { label: string; value: number }) {
  return (
    <div className='min-w-[7rem] rounded-2xl border border-white/10 bg-white/10 px-3 py-2 backdrop-blur-sm'>
      <div className='text-[0.65rem] font-semibold uppercase tracking-[0.16em] text-cyan-50/70'>{label}</div>
      <div className='mt-1 text-lg font-semibold text-white'>{value}</div>
    </div>
  )
}

function HeaderPillSwitch(props: {
  label: string
  hint: string
  checked: boolean
  onChange: (value: boolean) => void
}) {
  return (
    <button
      aria-checked={props.checked}
      aria-label={props.label}
      className='flex w-full items-center justify-between gap-4 rounded-2xl border border-white/10 bg-white/10 px-4 py-3 text-left backdrop-blur-sm transition hover:bg-white/15 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-200'
      onClick={() => props.onChange(!props.checked)}
      role='switch'
      type='button'
    >
      <div className='min-w-0'>
        <div className='text-[0.68rem] font-semibold uppercase tracking-[0.16em] text-cyan-50/70'>{props.label}</div>
        <div className='mt-1 text-sm leading-5 text-cyan-50/80'>{props.hint}</div>
      </div>
      <span
        aria-hidden='true'
        className={
          'relative inline-flex h-7 w-12 shrink-0 items-center rounded-full border p-0.5 transition ' +
          (props.checked
            ? 'border-cyan-300/80 bg-cyan-300/90'
            : 'border-white/15 bg-slate-700/80')
        }
      >
        <span
          className={
            'h-5 w-5 rounded-full bg-white shadow-sm transition-transform ' +
            (props.checked ? 'translate-x-5' : 'translate-x-0')
          }
        />
      </span>
    </button>
  )
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className='rounded-2xl border border-slate-200 bg-slate-50/80 p-3.5'>
      <div className='text-xs font-semibold uppercase tracking-[0.18em] text-slate-400'>{label}</div>
      <div className='mt-1.5 break-all text-sm text-slate-700'>{value}</div>
    </div>
  )
}

function ProcessRuleListItem(props: {
  title: string
  subtitle: string
  processNames: string[]
  selected: boolean
  onClick: () => void
}) {
  return (
    <button
      className={
        props.selected
          ? 'relative w-full overflow-hidden rounded-[1.25rem] border border-blue-200 bg-white px-3.5 py-3 text-left shadow-sm ring-1 ring-blue-100 transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300'
          : 'relative w-full overflow-hidden rounded-[1.25rem] border border-slate-200 bg-white px-3.5 py-3 text-left transition hover:border-slate-300 hover:bg-slate-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-200'
      }
      onClick={props.onClick}
      type='button'
    >
      <div className={props.selected ? 'absolute inset-y-0 left-0 w-1 bg-blue-500' : 'absolute inset-y-0 left-0 w-1 bg-transparent'} />
      <div className='pl-2'>
        <div className='text-sm font-semibold leading-5 text-slate-900'>{props.title}</div>
        <div className='mt-0.5 text-xs text-slate-500'>{props.subtitle}</div>
        {props.processNames.length > 0 && (
          <div className='mt-2 flex flex-wrap gap-1.5'>
            {props.processNames.map((processName) => (
              <span
                key={processName}
                className={
                  props.selected
                    ? 'rounded-full bg-blue-50 px-2 py-1 text-[0.7rem] font-medium text-blue-700'
                    : 'rounded-full bg-slate-100 px-2 py-1 text-[0.7rem] font-medium text-slate-600'
                }
              >
                {processName}
              </span>
            ))}
          </div>
        )}
      </div>
    </button>
  )
}

function BindingEditor(props: {
  binding: GestureBinding
  text: I18nText
  onChange: (binding: GestureBinding) => void
  onDelete: () => void
}) {
  const actionType = props.binding.action.type

  return (
    <div className='rounded-[1.25rem] border border-slate-200 bg-white p-3.5'>
      <div className='grid gap-3 xl:grid-cols-[11rem_10rem_minmax(0,1fr)_5.5rem]'>
        <div>
          <label className='field-label'>{props.text.gesture}</label>
          <GestureComposer
            label={props.text.gesture}
            value={props.binding.gesture}
            text={props.text}
            onChange={(gesture) => props.onChange({ ...props.binding, gesture })}
          />
        </div>

        <div className='space-y-3'>
          <div>
            <label className='field-label'>{props.text.actionType}</label>
            <select
              aria-label={props.text.actionType}
              className='text-input'
              value={actionType}
              onChange={(event) =>
                props.onChange({
                  ...props.binding,
                  action: normalizeActionType(event.target.value as GestureAction['type'])
                })
              }
            >
              <option value='hotkey'>{props.text.hotkey}</option>
              <option value='shell'>{props.text.shell}</option>
              <option value='none'>{props.text.none}</option>
            </select>
          </div>

          <div>
            <label className='field-label'>{props.text.description}</label>
            <input
              aria-label={props.text.description}
              autoComplete='off'
              className='text-input'
              value={props.binding.description}
              onChange={(event) =>
                props.onChange({
                  ...props.binding,
                  description: event.target.value
                })
              }
              placeholder={props.text.descriptionPlaceholder}
            />
          </div>
        </div>

        <div className='space-y-2.5'>
          {actionType === 'hotkey' && 'hotkey' in props.binding.action && (
            <>
              <label className='field-label'>{props.text.hotkey}</label>
              <HotkeyRecorder
                hotkey={props.binding.action.hotkey}
                text={props.text}
                onChange={(hotkey) =>
                  props.onChange({
                    ...props.binding,
                    action: { type: 'hotkey', hotkey }
                  })
                }
              />
            </>
          )}

          {actionType === 'shell' && 'command' in props.binding.action && (
            <>
              <label className='field-label'>{props.text.command}</label>
              <input
                aria-label={props.text.command}
                autoComplete='off'
                className='text-input'
                value={props.binding.action.command}
                onChange={(event) =>
                  props.onChange({
                    ...props.binding,
                    action: { type: 'shell', command: event.target.value }
                  })
                }
              />
            </>
          )}

          {actionType === 'none' && (
            <div className='rounded-xl border border-dashed border-slate-200 bg-slate-50 px-3 py-2.5 text-sm text-slate-500'>
              {props.text.none}
            </div>
          )}
        </div>

        <div className='flex items-end xl:items-start'>
          <button className='btn-danger w-full xl:mt-6' onClick={props.onDelete} type='button'>
            {props.text.delete}
          </button>
        </div>
      </div>
    </div>
  )
}

function GestureComposer(props: {
  label: string
  value: string
  text: I18nText
  onChange: (value: string) => void
}) {
  return (
    <div className='space-y-2'>
      <input
        aria-label={props.label}
        autoComplete='off'
        className='text-input'
        value={props.value}
        onChange={(event) => props.onChange(normalizeGesture(event.target.value))}
        placeholder='UDLR'
      />
      <div className='flex flex-wrap gap-1.5'>
        {DIRECTION_BUTTONS.map((direction) => (
          <button
            key={direction}
            className='btn-secondary min-w-10'
            onClick={() => props.onChange(normalizeGesture(props.value + direction))}
            type='button'
          >
            {direction}
          </button>
        ))}
        <button className='btn-secondary' onClick={() => props.onChange(props.value.slice(0, -1))} type='button'>
          {props.text.backspace}
        </button>
        <button className='btn-secondary' onClick={() => props.onChange('')} type='button'>
          {props.text.clear}
        </button>
      </div>
    </div>
  )
}

function HotkeyRecorder(props: {
  hotkey: HotkeySpec
  text: I18nText
  onChange: (value: HotkeySpec) => void
}) {
  const keyOptions =
    props.hotkey.key && !HOTKEY_KEY_OPTIONS.includes(props.hotkey.key)
      ? [props.hotkey.key, ...HOTKEY_KEY_OPTIONS]
      : HOTKEY_KEY_OPTIONS

  return (
    <div className='space-y-2'>
      <input
        aria-label={props.text.hotkey}
        className='text-input'
        readOnly
        value={formatHotkey(props.hotkey)}
        placeholder={props.text.hotkeyHint}
        onKeyDown={(event) => {
          event.preventDefault()
          const nextHotkey = normalizeHotkeyFromEvent(event)
          if (nextHotkey) {
            props.onChange(nextHotkey)
          }
        }}
      />


      <div className='flex flex-wrap gap-1.5'>
        {HOTKEY_MODIFIER_ORDER.map((modifier) => {
          const active = props.hotkey.modifiers.includes(modifier)

          return (
            <button
              key={modifier}
              className={
                active
                  ? 'inline-flex items-center justify-center rounded-xl border border-blue-200 bg-blue-50 px-3.5 py-2 text-sm font-medium text-blue-700 transition hover:bg-blue-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-200'
                  : 'btn-secondary'
              }
              onClick={() => props.onChange(toggleHotkeyModifier(props.hotkey, modifier))}
              type='button'
            >
              {modifier}
            </button>
          )
        })}

        <select
          aria-label={props.text.selectKey}
          className='text-input min-w-[9rem] flex-1 sm:flex-none sm:w-40'
          value={props.hotkey.key}
          onChange={(event) => props.onChange(normalizeHotkey({ ...props.hotkey, key: event.target.value }))}
        >
          <option value=''>{props.text.selectKey}</option>
          {keyOptions.map((key) => (
            <option key={key} value={key}>
              {formatKeyName(key)}
            </option>
          ))}
        </select>

        <button className='btn-secondary shrink-0' onClick={() => props.onChange({ modifiers: [], key: '' })} type='button'>
          {props.text.clear}
        </button>
      </div>
    </div>
  )
}
