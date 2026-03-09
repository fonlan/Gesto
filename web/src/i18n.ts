import type { Locale } from './types'

export interface I18nText {
  title: string
  subtitle: string
  save: string
  saving: string
  saved: string
  saveFailed: string
  globalSettings: string
  language: string
  trailColor: string
  trailWidth: string
  minimumDistance: string
  fadeDuration: string
  autostart: string
  status: string
  serverUrl: string
  configPath: string
  defaultRules: string
  appRules: string
  addRule: string
  addBinding: string
  ruleName: string
  processNames: string
  processHint: string
  gesture: string
  actionType: string
  hotkey: string
  command: string
  delete: string
  clear: string
  backspace: string
  none: string
  shell: string
  hotkeyHint: string
  directionHint: string
}

export const messages: Record<Locale, I18nText> = {
  'zh-CN': {
    title: 'Gesto 配置中心',
    subtitle: 'Rust 后台正在本机运行，当前页面通过浏览器配置鼠标手势。',
    save: '保存配置',
    saving: '保存中...',
    saved: '配置已保存',
    saveFailed: '保存失败',
    globalSettings: '全局设置',
    language: '语言',
    trailColor: '轨迹颜色',
    trailWidth: '轨迹宽度',
    minimumDistance: '最小触发距离',
    fadeDuration: '渐隐时长（毫秒）',
    autostart: '开机自启动',
    status: '运行状态',
    serverUrl: '服务地址',
    configPath: '配置文件',
    defaultRules: '全局默认手势',
    appRules: '按程序定制',
    addRule: '新增程序规则',
    addBinding: '新增手势',
    ruleName: '规则名称',
    processNames: '进程名',
    processHint: '多个进程名请用逗号分隔，例如 chrome.exe, msedge.exe',
    gesture: '手势',
    actionType: '动作类型',
    hotkey: '快捷键',
    command: '命令',
    delete: '删除',
    clear: '清空',
    backspace: '退格',
    none: '无动作',
    shell: '命令执行',
    hotkeyHint: '点击输入框后直接按下快捷键组合',
    directionHint: '支持 U / D / L / R 任意组合，例如 DR、ULR'
  },
  'en-US': {
    title: 'Gesto Control Center',
    subtitle: 'The Rust background service is running locally; this page configures gestures in your browser.',
    save: 'Save Config',
    saving: 'Saving...',
    saved: 'Configuration saved',
    saveFailed: 'Save failed',
    globalSettings: 'Global Settings',
    language: 'Language',
    trailColor: 'Trail Color',
    trailWidth: 'Trail Width',
    minimumDistance: 'Minimum Trigger Distance',
    fadeDuration: 'Fade Duration (ms)',
    autostart: 'Launch on Startup',
    status: 'Runtime Status',
    serverUrl: 'Server URL',
    configPath: 'Config File',
    defaultRules: 'Default Gestures',
    appRules: 'Per-App Rules',
    addRule: 'Add App Rule',
    addBinding: 'Add Gesture',
    ruleName: 'Rule Name',
    processNames: 'Process Names',
    processHint: 'Separate multiple process names with commas, e.g. chrome.exe, msedge.exe',
    gesture: 'Gesture',
    actionType: 'Action Type',
    hotkey: 'Shortcut',
    command: 'Command',
    delete: 'Delete',
    clear: 'Clear',
    backspace: 'Backspace',
    none: 'No Action',
    shell: 'Shell Command',
    hotkeyHint: 'Focus the field and press the shortcut combination directly',
    directionHint: 'Supports any U / D / L / R combination, such as DR or ULR'
  }
}
