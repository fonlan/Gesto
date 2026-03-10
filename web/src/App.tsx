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
const HOTKEY_MODIFIER_ORDER = ['Ctrl', 'Alt', 'Shift', 'Meta'] as const
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

const normalizeHotkey = (hotkey: HotkeySpec): HotkeySpec => ({
  modifiers: HOTKEY_MODIFIER_ORDER.filter((modifier) => hotkey.modifiers.includes(modifier)),
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
  const ignored = new Set(['Control', 'Shift', 'Alt', 'Meta'])
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
    event.metaKey ? 'Meta' : null
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
    <div className='min-h-screen bg-slate-100 px-4 py-8 text-slate-900 sm:px-8'>
      <div className='mx-auto flex max-w-7xl flex-col gap-6'>
        <header className='flex flex-col gap-4 rounded-[2rem] bg-gradient-to-r from-slate-900 via-blue-900 to-cyan-700 px-8 py-8 text-white shadow-panel'>
          <div className='flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between'>
            <div>
              <div className='mb-2 inline-flex rounded-full bg-white/15 px-3 py-1 text-xs font-semibold uppercase tracking-[0.2em] text-cyan-100'>
                Gesto
              </div>
              <h1 className='text-3xl font-bold sm:text-4xl'>{t.title}</h1>
              <p className='mt-2 max-w-3xl text-sm leading-6 text-cyan-50/90'>{t.subtitle}</p>
            </div>
            <button className='btn-primary' onClick={saveConfig} disabled={saving}>
              {saving ? t.saving : t.save}
            </button>
          </div>
          {(message || error) && (
            <div
              className={
                'rounded-2xl px-4 py-3 text-sm font-medium ' +
                (error ? 'bg-red-500/15 text-red-100' : 'bg-emerald-500/15 text-emerald-100')
              }
            >
              {error || message}
            </div>
          )}
        </header>

        <div className='grid gap-6 xl:grid-cols-[1.2fr_0.8fr]'>
          <section className='panel'>
            <div className='mb-6 flex items-center justify-between'>
              <div>
                <h2 className='text-xl font-semibold'>{t.globalSettings}</h2>
                <p className='mt-1 text-sm text-slate-500'>{t.directionHint}</p>
              </div>
            </div>
            <div className='grid gap-5 md:grid-cols-2'>
              <div>
                <label className='field-label'>{t.language}</label>
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
              </div>
              <div>
                <label className='field-label'>{t.trailColor}</label>
                <input
                  className='h-12 w-full rounded-2xl border border-slate-200 bg-white px-3 py-2'
                  type='color'
                  value={config.general.trailColor}
                  onChange={(event) =>
                    patchConfig((current) => ({
                      ...current,
                      general: { ...current.general, trailColor: event.target.value }
                    }))
                  }
                />
              </div>
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
              <NumberField
                label={t.rightClickIdleFallback}
                min={0}
                max={1000}
                step={10}
                value={config.general.rightClickIdleFallbackMs}
                onChange={(value) =>
                  patchConfig((current) => ({
                    ...current,
                    general: { ...current.general, rightClickIdleFallbackMs: value }
                  }))
                }
              />
              <NumberField
                label={t.rightClickIdleMovementTolerance}
                min={0}
                max={24}
                step={0.5}
                value={config.general.rightClickIdleMovementTolerance}
                onChange={(value) =>
                  patchConfig((current) => ({
                    ...current,
                    general: { ...current.general, rightClickIdleMovementTolerance: value }
                  }))
                }
              />
              <div className='md:col-span-2'>
                <label className='field-label'>{t.ignoredProcessNames}</label>
                <textarea
                  className='text-input min-h-24 resize-y'
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
                <p className='mt-2 text-sm text-slate-500'>{t.ignoredProcessHint}</p>
              </div>
              <div className='flex items-center justify-between rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3'>
                <div>
                  <div className='text-sm font-medium text-slate-700'>{t.autostart}</div>
                </div>
                <label className='inline-flex cursor-pointer items-center gap-3'>
                  <span className='text-sm text-slate-500'>{config.general.autostart ? 'On' : 'Off'}</span>
                  <input
                    type='checkbox'
                    className='h-5 w-5 rounded border-slate-300 text-blue-600 focus:ring-blue-500'
                    checked={config.general.autostart}
                    onChange={(event) =>
                      patchConfig((current) => ({
                        ...current,
                        general: { ...current.general, autostart: event.target.checked }
                      }))
                    }
                  />
                </label>
              </div>
            </div>
          </section>

          <section className='panel'>
            <h2 className='text-xl font-semibold'>{t.status}</h2>
            <div className='mt-5 space-y-4 text-sm'>
              <InfoRow label={t.serverUrl} value={status?.serverUrl ?? '-'} />
              <InfoRow label={t.configPath} value={status?.configPath ?? '-'} />
            </div>
          </section>
        </div>

        <section className='panel'>
          <div className='mb-5 flex items-center justify-between'>
            <div>
              <h2 className='text-xl font-semibold'>{t.appRules}</h2>
              <p className='mt-1 text-sm text-slate-500'>{t.processRulesHint}</p>
            </div>
            <button
              className='btn-secondary'
              onClick={() => {
                const nextRule = createEmptyRule()
                setSelectedRuleId(nextRule.id)
                patchConfig((current) => ({ ...current, appRules: [...current.appRules, nextRule] }))
              }}
            >
              {t.addRule}
            </button>
          </div>

          <div className='grid gap-5 xl:grid-cols-[19rem_minmax(0,1fr)]'>
            <div className='rounded-[1.75rem] border border-slate-200 bg-slate-50 p-3 shadow-sm xl:sticky xl:top-6 xl:max-h-[70vh]'>
              <div className='max-h-[calc(70vh-1.5rem)] space-y-3 overflow-y-auto pr-1'>
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

            <div className='rounded-[1.75rem] border border-slate-200 bg-slate-50 p-5'>
              {selectedRuleId === GLOBAL_RULE_ID || !selectedRule ? (
                <>
                  <div className='flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between'>
                    <div>
                      <h3 className='text-lg font-semibold text-slate-900'>{t.defaultRules}</h3>
                      <p className='mt-1 text-sm text-slate-500'>{t.globalProcessHint}</p>
                    </div>
                    <button
                      className='btn-secondary'
                      onClick={() =>
                        patchConfig((current) => ({
                          ...current,
                          defaultActions: [...current.defaultActions, createEmptyBinding()]
                        }))
                      }
                    >
                      {t.addBinding}
                    </button>
                  </div>

                  <div className='mt-5 space-y-4'>
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
                  <div className='flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between'>
                    <div>
                      <h3 className='text-lg font-semibold text-slate-900'>{getRuleEditorTitle(selectedRule, t)}</h3>
                      <p className='mt-1 text-sm text-slate-500'>{t.processHint}</p>
                    </div>
                    <button
                      className='btn-danger xl:min-w-28'
                      onClick={() => {
                        setSelectedRuleId(GLOBAL_RULE_ID)
                        patchConfig((current) => ({
                          ...current,
                          appRules: current.appRules.filter((item) => item.id !== selectedRule.id)
                        }))
                      }}
                    >
                      {t.delete}
                    </button>
                  </div>

                  <div className='mt-5 grid gap-4 lg:grid-cols-[1fr_1.4fr]'>
                    <div>
                      <label className='field-label'>{t.ruleName}</label>
                      <input
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
                    </div>
                    <div>
                      <label className='field-label'>{t.processNames}</label>
                      <input
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
                    </div>
                  </div>

                  <div className='mt-5 space-y-4'>
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

function NumberField(props: {
  label: string
  value: number
  min: number
  max: number
  step: number
  onChange: (value: number) => void
}) {
  return (
    <div>
      <label className='field-label'>{props.label}</label>
      <input
        className='text-input'
        type='number'
        min={props.min}
        max={props.max}
        step={props.step}
        value={props.value}
        onChange={(event) => props.onChange(Number(event.target.value))}
      />
    </div>
  )
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className='rounded-2xl border border-slate-200 bg-slate-50 p-4'>
      <div className='text-xs font-semibold uppercase tracking-[0.18em] text-slate-400'>{label}</div>
      <div className='mt-2 break-all text-sm text-slate-700'>{value}</div>
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
          ? 'relative w-full overflow-hidden rounded-[1.5rem] border border-blue-300 bg-white p-4 text-left shadow-sm ring-2 ring-blue-100 transition'
          : 'relative w-full overflow-hidden rounded-[1.5rem] border border-slate-200 bg-white p-4 text-left transition hover:border-slate-300 hover:bg-slate-50'
      }
      onClick={props.onClick}
      type='button'
    >
      <div className={props.selected ? 'absolute inset-y-0 left-0 w-1 bg-blue-500' : 'absolute inset-y-0 left-0 w-1 bg-transparent'} />
      <div className='pl-2'>
        <div className='text-sm font-semibold text-slate-900'>{props.title}</div>
        <div className='mt-1 text-xs text-slate-500'>{props.subtitle}</div>
        {props.processNames.length > 0 && (
          <div className='mt-3 flex flex-wrap gap-2'>
            {props.processNames.map((processName) => (
              <span
                key={processName}
                className={
                  props.selected
                    ? 'rounded-full bg-blue-50 px-2.5 py-1 text-xs font-medium text-blue-700'
                    : 'rounded-full bg-slate-100 px-2.5 py-1 text-xs font-medium text-slate-600'
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
    <div className='rounded-[1.5rem] border border-slate-200 bg-white p-4'>
      <div className='grid gap-4 lg:grid-cols-[1fr_1fr_1.2fr_auto]'>
        <div>
          <label className='field-label'>{props.text.gesture}</label>
          <GestureComposer
            value={props.binding.gesture}
            text={props.text}
            onChange={(gesture) => props.onChange({ ...props.binding, gesture })}
          />
        </div>

        <div>
          <label className='field-label'>{props.text.actionType}</label>
          <select
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
            <div className='rounded-2xl border border-dashed border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-500'>
              {props.text.none}
            </div>
          )}
        </div>

        <div className='flex items-end'>
          <button className='btn-danger w-full' onClick={props.onDelete}>
            {props.text.delete}
          </button>
        </div>
      </div>
    </div>
  )
}

function GestureComposer(props: {
  value: string
  text: I18nText
  onChange: (value: string) => void
}) {
  return (
    <div className='space-y-3'>
      <input
        className='text-input'
        value={props.value}
        onChange={(event) => props.onChange(normalizeGesture(event.target.value))}
        placeholder='UDLR'
      />
      <div className='flex flex-wrap gap-2'>
        {DIRECTION_BUTTONS.map((direction) => (
          <button
            key={direction}
            className='btn-secondary min-w-12'
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
    <div className='space-y-3'>
      <input
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

      <div className='rounded-2xl border border-dashed border-slate-200 bg-slate-50 px-4 py-3 text-xs text-slate-500'>
        {props.text.hotkeyManualHint}
      </div>

      <div className='flex flex-wrap gap-2'>
        {HOTKEY_MODIFIER_ORDER.map((modifier) => {
          const active = props.hotkey.modifiers.includes(modifier)

          return (
            <button
              key={modifier}
              className={
                active
                  ? 'inline-flex items-center justify-center rounded-2xl border border-blue-200 bg-blue-50 px-4 py-2 text-sm font-medium text-blue-700 transition hover:bg-blue-100'
                  : 'btn-secondary'
              }
              onClick={() => props.onChange(toggleHotkeyModifier(props.hotkey, modifier))}
              type='button'
            >
              {modifier}
            </button>
          )
        })}
      </div>

      <div className='flex flex-col gap-2 sm:flex-row'>
        <select
          className='text-input'
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
