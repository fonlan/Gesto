# Gesto

Gesto 是一个仅面向 Windows 的鼠标手势软件：

- Rust 后台负责全局鼠标手势、透明轨迹覆盖层、系统托盘、本地 HTTP 服务与配置持久化
- React + Tailwind 配置页通过默认浏览器打开，无传统桌面配置窗口
- 配置存储在 `%AppData%/Gesto/config.json`
- Web 资源可构建后嵌入可执行文件，便于单文件分发

## 功能概览

- 右键按住划动识别 `U / D / L / R` 及任意组合手势
- 手势轨迹半透明绘制，并在结束后渐隐
- 识别手势开始显示器，并仅对该显示器上的当前前台窗口执行规则
- 支持按进程名区分手势动作，例如 `chrome.exe`、`explorer.exe`、`code.exe`
- 支持配置忽略进程列表；命中这些进程时会完全禁用手势识别并放行原生右键行为
- 支持热键动作与命令动作
- 托盘菜单可打开配置页面与退出程序
- 配置页支持中英双语、轨迹颜色/不透明度/宽度、最小触发距离、静止右键回退延迟/容差、忽略进程列表与开机自启动

## 开发

```powershell
npm install --prefix web
npm run build --prefix web
cargo run
```

## 发布

```powershell
npm install --prefix web
npm run build --prefix web
cargo build --release
```

发布产物为单个可执行文件：`target/release/gesto.exe`。
