# XTools

XTools 0.2.0 是面向 **Windows 10 1703（build 15063）及以上 / Windows 11 原生 AMD64（x64）** 的本地效率工具，通过 `Alt + Space` 搜索应用、管理剪贴板、屏幕取色和批量重命名。使用 Tauri 2、Rust、Vue 3、TypeScript 和 SQLite。

## 功能

以下功能已实现；已完成的验证与待验证项目见 [验收记录](docs/ACCEPTANCE.md)。

| 模块 | 已实现内容 |
| --- | --- |
| Launcher | 全局快捷键、键盘导航、开始菜单与桌面应用扫描、Registry 补充、UWP 发现与激活、真实图标缓存、中文/拼音/首字母搜索、去重、最近 8 个应用、后台刷新和旧索引保留 |
| 剪贴板 | 原生事件监听；文本、图片、文件/文件夹路径历史；搜索、收藏、删除、清空、去重、重新复制；默认最多 100 条普通历史，收藏不计入限额 |
| 取色 | 原生像素放大镜、HEX/RGB/HSL；左键复制 HEX，右键或 Esc 取消；Per-Monitor DPI Awareness V2 |
| 批量重命名 | 文件与文件夹混合选择/拖入；前后缀、替换、删除字符、编号、大小写；实时预览、冲突检测、两阶段执行、失败回滚、撤销最近一次成功操作 |
| 设置与托盘 | 快捷键录制和失败恢复、开机启动、历史数量、最近使用开关、重新扫描、关于、单实例、托盘退出 |
| 安装包 | NSIS、当前用户安装、自选安装目录、桌面快捷方式和开机启动选项；开机启动默认勾选 |

XTools 不做全盘文件搜索，不递归重命名文件夹内容。运行时没有账号、云服务、遥测、网络请求或 Node.js 后台进程。取色只实时采样，不保存截图。

## V2 更新

微信图片临时文件回退、独立图片保存和大缩略图；四向导航；悬浮与外部隐藏；主屏定位；资源管理器选中项带入；淡蓝规则区与简化文案；DIB 取色缓冲和原生通知。实际验证边界见 [V2 验收记录](docs/ACCEPTANCE.md)。

## 安装与使用

[下载 XTools 0.2.0 安装器](https://github.com/0Antique/XTools/releases/download/v0.2.0/XTools_Setup.exe)，或查看 [Release 页面](https://github.com/0Antique/XTools/releases/tag/v0.2.0)。附件只有 `XTools_Setup.exe`，约 2.66 MiB，下载后直接运行。正式附件已核对 SHA256 并完成隔离安装/卸载检查。

最低 Windows 10 版本为 1703，因为 Per-Monitor DPI Awareness V2 使用的 [SetProcessDpiAwarenessContext API](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setprocessdpiawarenesscontext) 从该版本起可用。安装器检查系统版本和 build，阻止早期 Windows 10 安装。

系统需要已安装 **Microsoft Edge WebView2 Runtime**。安装程序会检查此条件；缺少运行时会给出提示并停止，**不会联网下载**。需预先准备 WebView2 时，可使用 Microsoft 提供的离线安装程序。

运行 `XTools_Setup.exe`，选择安装目录，例如 `D:\Applications\XTools`，然后选择桌面快捷方式和开机启动选项。默认安装在当前用户目录，无需管理员权限；自选目录必须可写。卸载保留 `%APPDATA%\XTools` 中的个人数据。

启动后 XTools 隐藏并驻留托盘：

- `Alt + Space` 显示/隐藏 Launcher，`Esc` 隐藏；`↑`、`↓`、`←`、`→` 选择结果，`Enter` 执行。
- 输入 `微信`、`weixin` 或 `wx` 搜索已发现的微信应用；输入 `cb`、`picker`、`rn`、`settings` 打开内置工具。
- 在剪贴板历史按 `Enter` 只重新复制所选内容并隐藏 XTools，由用户自行粘贴。
- 右上角图钉开启悬浮后窗口置顶，点击外部继续显示；未悬浮时点击外部隐藏。
- 在资源管理器选中文件后按 `Alt + Space`，打开批量重命名即可带入选中项，新批次会替换原列表并重置规则；手动添加和拖入仍追加。
- 取色成功后使用 Windows 原生短时通知显示 HEX；系统勿扰或关闭通知时遵守系统设置。
- 每次呼出都放在当前主显示器工作区。
- 关闭窗口只隐藏；使用托盘的“退出 XTools”退出进程。`xtools.exe --quit` 用于安装/卸载等维护操作。

## 开发与构建

准备 Windows x64 环境：Node.js 22.12 或以上、Rust MSVC 工具链（项目最低版本 1.90）、Cargo/rustfmt、Visual Studio C++ 桌面开发工具及 Windows SDK，以及 WebView2 Runtime。第一次获取 npm/Cargo 依赖和构建工具可能需要网络；这与安装后离线运行分开。

```powershell
npm ci
npm run tauri dev
```

检查前端、运行 Rust 核心测试、检查格式并生成安装包：

```powershell
npm run check
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
npm run tauri build
```

NSIS 默认构建输出为 `src-tauri/target/release/bundle/nsis/XTools_0.2.0_x64-setup.exe`，Release 附件统一命名为 `XTools_Setup.exe`，不包含 WebView2 Runtime。

此工作区可选使用便携构建工具。存在本机 `.tools` 时，先在 PowerShell 执行：

```powershell
. .\scripts\enter-build-env.ps1
```

`.tools` 和构建输出不进入 Git；该脚本只加载已有工具的环境变量，不安装依赖。使用标准安装的工具链时无需执行。`npm run dev` 是浏览器前端预览；系统功能需通过 `npm run tauri dev` 验证。

调试版启动不会修改 Windows 开机启动项；在设置页保存或切换开机启动时仍会即时修改当前用户的 XTools 启动项。发布版按保存的设置初始化开机启动。

## 本地数据

安装目录和数据目录分离，数据均保存在 `%APPDATA%\XTools`：

```text
XTools/
├── config.json                 快捷键、开机启动、历史限额、最近使用开关
├── data/xtools.db              应用索引、使用记录、剪贴板元数据、重命名历史
├── clipboard/images/           剪贴板图片 PNG
├── cache/icons/                应用原始图标 PNG 缓存
└── logs/                       本地警告和错误日志
```

文本和路径历史存入 SQLite，图片保存在独立文件中；文件/文件夹历史只保存路径，不复制文件内容。启动先读取应用缓存，再后台刷新；扫描失败保留旧缓存。

## 验证与版本控制

Rust 核心测试、前端类型检查和生产构建已通过，并已生成 NSIS 安装包。本机原生扫描发现 **95 个应用并提取 95 个图标**，其中 31 个 UWP 应用；2000 应用模拟索引的 Debug 搜索平均约 **1.25 ms/次**。这些数据不代替 Windows 10/11、混合 DPI、多显示器、完整原生交互与资源占用验收。

完整操作步骤和未完成的验证项见 [docs/ACCEPTANCE.md](docs/ACCEPTANCE.md)。仓库为 [0Antique/XTools](https://github.com/0Antique/XTools)，本次开发分支为 `codex/xtools-v2`。
