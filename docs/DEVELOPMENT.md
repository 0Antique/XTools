# 开发与构建

本页面向开发者。产品介绍与使用方法见 [README](../README.md)。

技术栈：Tauri 2、Rust、Vue 3、TypeScript 和 SQLite。

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

NSIS 默认构建输出为 `src-tauri/target/release/bundle/nsis/XTools_<工程版本>_x64-setup.exe`，Release 附件统一命名为 `XTools_Setup.exe`，不包含 WebView2 Runtime。

此工作区可选使用便携构建工具。存在本机 `.tools` 时，先在 PowerShell 执行：

```powershell
. .\scripts\enter-build-env.ps1
```

`.tools` 和构建输出不进入 Git；该脚本只加载已有工具的环境变量，不安装依赖。使用标准安装的工具链时无需执行。`npm run dev` 是浏览器前端预览；系统功能需通过 `npm run tauri dev` 验证。

调试版启动不会修改 Windows 开机启动项；在设置页保存或切换开机启动时仍会即时修改当前用户的 XTools 启动项。发布版按保存的设置初始化开机启动。
