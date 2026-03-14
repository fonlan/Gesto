import type { Locale } from './types'

export interface I18nText {
  title: string
  subtitle: string
  save: string
  saving: string
  saved: string
  noChangesToSave: string
  saveFailed: string
  fetchConfigFailed: string
  unknownError: string
  unsavedChangesHint: string
  unsavedChangesGuard: string
  loading: string
  noConfigLoaded: string
  globalSettings: string
  language: string
  gesturesEnabled: string
  gesturesEnabledHint: string
  trailColor: string
  trailOpacity: string
  trailWidth: string
  minimumDistance: string
  fadeDuration: string
  ignoredProcessNames: string
  ignoredProcessHint: string
  autostart: string
  status: string
  serverUrl: string
  configPath: string
  logPath: string
  defaultRules: string
  appRules: string
  processRulesHint: string
  searchRules: string
  searchRulesPlaceholder: string
  noMatchingRules: string
  addRule: string
  addBinding: string
  ruleName: string
  ruleEnabled: string
  processNames: string
  processHint: string
  globalProcessName: string
  globalProcessHint: string
  emptyProcessNames: string
  unnamedRule: string
  enabled: string
  disabled: string
  gesture: string
  description: string
  descriptionPlaceholder: string
  actionType: string
  hotkey: string
  command: string
  delete: string
  clear: string
  backspace: string
  none: string
  shell: string
  hotkeyHint: string
  hotkeyManualHint: string
  selectKey: string
  directionHint: string
  duplicateGestureConflict: string
  saveBlockedByGestureConflicts: string
}

export const messages: Record<Locale, I18nText> = {
  'zh-CN': {
    title: 'Gesto 配置中心',
    subtitle: 'Rust 后台正在本机运行，当前页面通过浏览器配置鼠标手势。',
    save: '保存配置',
    saving: '保存中...',
    saved: '配置已保存',
    noChangesToSave: '当前没有需要保存的更改',
    saveFailed: '保存失败',
    fetchConfigFailed: '获取配置失败',
    unknownError: '未知错误',
    unsavedChangesHint: '存在未保存修改，刷新或关闭页面前请先保存。',
    unsavedChangesGuard: '存在未保存修改，确认要离开当前页面吗？',
    loading: '加载中...',
    noConfigLoaded: '未加载到配置',
    globalSettings: '全局设置',
    language: '语言',
    gesturesEnabled: '启用鼠标手势',
    gesturesEnabledHint: '关闭后将完全放行原生右键，不绘制轨迹也不触发手势动作。',
    trailColor: '轨迹颜色',
    trailOpacity: '轨迹不透明度（%）',
    trailWidth: '轨迹宽度',
    minimumDistance: '最小触发距离',
    fadeDuration: '渐隐时长（毫秒）',
    ignoredProcessNames: '忽略进程列表',
    ignoredProcessHint: '多个进程名请用逗号分隔；命中这些进程时将完全禁用鼠标手势并放行原生右键。',
    autostart: '开机自启动',
    status: '运行状态',
    serverUrl: '服务地址',
    configPath: '配置文件',
    logPath: '日志文件',
    defaultRules: '全局默认手势',
    appRules: '按程序定制',
    processRulesHint: '左侧选择进程，右侧编辑该进程对应的手势规则；未命中特定程序时会回退到全局规则。',
    searchRules: '搜索程序规则',
    searchRulesPlaceholder: '搜索规则名或进程名',
    noMatchingRules: '没有匹配当前搜索条件的程序规则。',
    addRule: '新增程序规则',
    addBinding: '新增手势',
    ruleName: '规则名称',
    ruleEnabled: '启用此规则',
    processNames: '进程名',
    processHint: '多个进程名请用逗号分隔，例如 chrome.exe, msedge.exe',
    globalProcessName: '全局规则',
    globalProcessHint: '当没有命中专属程序规则时，将应用这里的默认手势。',
    emptyProcessNames: '暂未设置进程名',
    unnamedRule: '未命名规则',
    enabled: '已启用',
    disabled: '已禁用',
    gesture: '手势',
    description: '作用描述',
    descriptionPlaceholder: '例如：返回上一页',
    actionType: '动作类型',
    hotkey: '快捷键',
    command: '命令',
    delete: '删除',
    clear: '清空',
    backspace: '退格',
    none: '无动作',
    shell: '命令执行',
    hotkeyHint: '点击输入框后直接按下快捷键组合',
    hotkeyManualHint: '部分浏览器快捷键（如 Ctrl+W、Ctrl+L）会被浏览器优先处理；这类组合请用下方按钮和按键列表手动设置。',
    selectKey: '选择按键',
    directionHint: '支持 U / D / L / R 任意组合，例如 DR、ULR',
    duplicateGestureConflict: '重复手势会导致同一作用域内较早的条目在运行时优先命中。',
    saveBlockedByGestureConflicts: '保存前请先解决重复手势：'
  },
  'en-US': {
    title: 'Gesto Control Center',
    subtitle: 'The Rust background service is running locally; this page configures gestures in your browser.',
    save: 'Save Config',
    saving: 'Saving...',
    saved: 'Configuration saved',
    noChangesToSave: 'There are no changes to save',
    saveFailed: 'Save failed',
    fetchConfigFailed: 'Failed to fetch config payload',
    unknownError: 'Unknown error',
    unsavedChangesHint: 'You have unsaved changes. Save before refreshing or closing this page.',
    unsavedChangesGuard: 'You have unsaved changes. Are you sure you want to leave this page?',
    loading: 'Loading...',
    noConfigLoaded: 'No config loaded',
    globalSettings: 'Global Settings',
    language: 'Language',
    gesturesEnabled: 'Enable Mouse Gestures',
    gesturesEnabledHint: 'When turned off, Gesto passes native right-click events through without drawing trails or triggering gesture actions.',
    trailColor: 'Trail Color',
    trailOpacity: 'Trail Opacity (%)',
    trailWidth: 'Trail Width',
    minimumDistance: 'Minimum Trigger Distance',
    fadeDuration: 'Fade Duration (ms)',
    ignoredProcessNames: 'Ignored Processes',
    ignoredProcessHint: 'Separate multiple process names with commas; matching processes bypass gesture handling and keep native right-click behavior.',
    autostart: 'Launch on Startup',
    status: 'Runtime Status',
    serverUrl: 'Server URL',
    configPath: 'Config File',
    logPath: 'Log File',
    defaultRules: 'Default Gestures',
    appRules: 'Per-App Rules',
    processRulesHint: 'Select a process on the left and edit its gesture rules on the right; unmatched apps fall back to the global rule.',
    searchRules: 'Search app rules',
    searchRulesPlaceholder: 'Search by rule name or process name',
    noMatchingRules: 'No app rules match the current search.',
    addRule: 'Add App Rule',
    addBinding: 'Add Gesture',
    ruleName: 'Rule Name',
    ruleEnabled: 'Enable this rule',
    processNames: 'Process Names',
    processHint: 'Separate multiple process names with commas, e.g. chrome.exe, msedge.exe',
    globalProcessName: 'Global Rule',
    globalProcessHint: 'These default gestures apply when no app-specific rule matches.',
    emptyProcessNames: 'No process names yet',
    unnamedRule: 'Unnamed Rule',
    enabled: 'Enabled',
    disabled: 'Disabled',
    gesture: 'Gesture',
    description: 'Description',
    descriptionPlaceholder: 'e.g. Go back to the previous page',
    actionType: 'Action Type',
    hotkey: 'Shortcut',
    command: 'Command',
    delete: 'Delete',
    clear: 'Clear',
    backspace: 'Backspace',
    none: 'No Action',
    shell: 'Shell Command',
    hotkeyHint: 'Focus the field and press the shortcut combination directly',
    hotkeyManualHint: 'Some browser shortcuts, such as Ctrl+W and Ctrl+L, are handled by the browser first; use the modifier buttons and key list below for those combinations.',
    selectKey: 'Select a key',
    directionHint: 'Supports any U / D / L / R combination, such as DR or ULR',
    duplicateGestureConflict: 'Duplicate gestures make earlier bindings in the same scope win at runtime.',
    saveBlockedByGestureConflicts: 'Resolve duplicate gestures before saving:'
  }
}
