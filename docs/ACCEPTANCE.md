# XTools V1 验收记录

规格依据：[XTools_V1_Development_Spec.md](../XTools_V1_Development_Spec.md)。本记录将代码实现与实际验收分开：勾选表示已有实际验证证据，未勾选表示仍需完成该项验证。

支持 Windows 10 1703（build 15063）及以上和 Windows 11 原生 AMD64。最低版本来自 Per-Monitor V2 使用的 [SetProcessDpiAwarenessContext API](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setprocessdpiawarenesscontext)；安装器使用本地 NSIS `WinVer.nsh` 提供的 `AtLeastBuild` 宏检查 build 15063。

## 已完成的验证

- [x] Rust 核心测试 22/22 通过：中文/拼音搜索、匹配优先级、最近使用与去重、扫描边界、Registry 过滤、剪贴板存储与限额、重命名规则/冲突/两阶段执行/撤销等。
- [x] 前端 TypeScript/Vue 类型检查及 Vite 生产构建通过。
- [x] Windows x64 发布构建成功，生成 NSIS 安装包，当前约 2.47 MiB；安装模板包含目录选择、桌面快捷方式和开机启动选项。
- [x] 本机调用原生 Shell/COM 扫描成功：95 个应用，61 个来自开始菜单、3 个来自 Registry、31 个来自 UWP；共提取 95 个真实 PNG 图标。
- [x] Microsoft Store 与计算器缓存图标已进行视觉检查；UWP 图标使用 `shell:AppsFolder\AUMID` 正确解析。
- [x] 图标缓存后的本机扫描约 224 ms；2000 应用模拟索引执行 100 次匹配搜索约 125 ms，即约 1.25 ms/次（Debug，单次本机测量）。
- [x] 安装模板不使用网络引导安装；缺少 WebView2 Runtime 时提示并停止，不下载运行时。

2026-10-05 在 Windows 10 Pro 22H2（build 19045）x64、已安装 WebView2 的本机补充验证：

- [x] 发布版初次启动保持隐藏；再次启动只保留一个实例并显示 Launcher。隔离数据目录用于测试，未使用日常历史进行删改。
- [x] 原生 `Alt + Space` 能从其他应用呼出 Launcher；原生搜索 `wx` 将真实微信快捷方式列为最佳匹配。
- [x] 搜索 `计算器` 并按 Enter 激活真实 Microsoft Store/UWP 计算器，Launcher 随后隐藏。
- [x] 生产剪贴板 parser/native/storage 的原生 smoke：文本、2×2 RGBA 图片、文件及文件夹 CF_HDROP 路径逐项回写、捕获和真实监听入库通过；文本去重更新时间与图片 PNG 持久化通过。原主要剪贴板内容在测试后恢复并校验。
- [x] 浏览器交互检查覆盖四个前端视图、键盘操作、过期 IPC 响应、重命名预览和设置错误恢复。该检查使用仅存在于测试过程的 IPC mock，系统功能另以原生验证为依据。
- [x] `cargo clippy --all-targets -- -D clippy::correctness` 通过；图标模块仅有代码风格建议。
- [x] 最新安装器静默 smoke 31/31 通过：D 盘含空格目录；默认勾选和同时取消桌面/启动选项；解析快捷方式、Run 项、DisplayIcon 指向实际 exe；既有配置的安装选择仅应用一次；静止及运行中卸载；两个注册表视图清理；保留数据目录与安装目录其他文件。原注册表与快捷方式状态在测试后恢复。

原生窗口的 Windows.Graphics.Capture 截图在本机超时，Launcher 实际交互通过可访问性状态与原生进程结果确认。尚未完成的取色交互、混合 DPI、整进程树内存及完全断网验收仍见下面清单。

复现自动检查：

```powershell
npm ci
npm run check
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
npm run tauri build
```

首次获取构建依赖需要开发环境网络；安装后的运行时离线性应按下面的断网步骤单独验收。标准工具链或可选的 `. .\scripts\enter-build-env.ps1` 均可用于执行检查。

## 原生操作验收

### Launcher、托盘与设置

- [ ] 启动后隐藏并显示托盘图标；重复运行只保留一个实例，并唤起原实例。
- [ ] 在其他应用中按 `Alt + Space` 能呼出，再按一次或 Esc 隐藏；每次呼出清空查询并聚焦搜索框。
- [ ] 空查询显示内置工具及最近 8 个通过 XTools 成功启动的应用；关闭最近使用设置后只显示工具。
- [ ] `↑`、`↓`、`Enter` 在应用和工具结果间工作正常；中文、拼音、首字母查询命中真实应用。
- [ ] Enter 实际启动 Win32 `.exe`、含参数和工作目录的 `.lnk`、Microsoft Store/UWP 应用；启动成功更新最近使用，失败显示明确错误。
- [ ] 重新扫描不会阻塞 UI，失败保留旧索引；重新启动可立即使用数据库缓存。
- [ ] 快捷键设置立即生效；被占用的新快捷键不保存，原快捷键仍可用。
- [ ] 设置页和托盘切换开机启动立即更新当前用户 Run 项；重新登录后行为一致。
- [ ] 普通窗口关闭只隐藏，托盘退出才结束进程；`--quit` 可结束维护中的实例。

### 剪贴板

- [ ] 从不同应用复制文本、浏览器/Word/聊天软件图片、文件和文件夹路径，历史内容正确。
- [ ] 连续复制相同内容只保留一项并更新到最新位置；同图不同来源可按内容去重。
- [ ] 普通历史达到设置限额后淘汰最旧普通项；收藏项不自动淘汰；更改限额即时生效。
- [ ] 搜索、收藏、取消收藏、删除和清空在真实历史上正确工作。
- [ ] 选择文本、图片、路径历史按 Enter 后只写入 Windows Clipboard 并隐藏；无自动粘贴或窗口切换。
- [ ] 空闲时采用原生剪贴板通知，无高频轮询；退出后监听线程结束。

### 屏幕取色

- [ ] 进入后 Launcher 隐藏，鼠标附近出现像素放大镜并显示 HEX、RGB、HSL。
- [ ] 对照已知纯色测试采样结果；移动指针时浮窗自身不污染取色。
- [ ] 左键复制 `#RRGGBB` 并结束；右键或 Esc 取消且不修改剪贴板。
- [ ] 跨显示器、负坐标和显示器边缘取色位置正确；退出 XTools 后取色窗口与钩子清理。

### 批量重命名

- [ ] 拖入/选择文件、文件夹及混合对象；只改所选对象自身，子目录内容不变。
- [ ] 每种规则及组合规则实时预览正确；文件扩展名保留；编号的起始、步长和位数符合设置。
- [ ] 重复名称、非法字符、空名、目标已存在、来源已消失均标明原因并禁止执行。
- [ ] 真实测试文件交换名称成功；中途失败尽可能回滚，不覆盖未知对象。
- [ ] 最近一次成功操作可撤销；原名称被其他对象占用时停止撤销并显示冲突。

测试重命名时使用单独准备的样本目录，覆盖含中文名称、文件/文件夹混合、交换名称和占用冲突的情形。

## 安装与卸载

- [ ] 在 Windows 10 1703+/Windows 11 原生 AMD64 环境正常安装；早期 Windows 10 和不支持的平台被阻止。
- [x] 安装到含空格的 D 盘目录，确认程序、开始菜单入口和卸载入口正常。
- [ ] 桌面快捷方式和开机启动默认勾选；分别取消后安装结果与选择一致。
- [ ] 已有配置时重装仍应用本次安装选择；启动后该选择与设置页一致。
- [ ] 无 WebView2 的环境得到明确提示，安装器不发起下载。
- [x] 运行中卸载可通过 `--quit` 退出并移除程序文件、快捷方式、卸载项及 XTools Run 项。
- [x] 卸载保留 `%APPDATA%\XTools`，不删除自选安装目录内的其他文件。

## 平台、DPI 与性能矩阵

以下均为待验证项，单机测试结果不能覆盖整张矩阵。

| 环境/指标 | 目标 | 验证状态 |
| --- | --- | --- |
| Windows 10 1703+ x64 | 所有 V1 操作正常；最低 build 15063 | 待验证 |
| Windows 11 x64 | 所有 V1 操作正常 | 待验证 |
| 100% / 125% / 150% / 175% 缩放 | Launcher 布局、显示器定位和取色坐标正确 | 待验证 |
| 多显示器混合缩放、负坐标 | 优先前台窗口显示器，鼠标位置回退有效 | 待验证 |
| 快捷键到窗口可见 | 小于 150 ms | 待测量 |
| 实际应用索引搜索 | 小于 30 ms | 模拟 2000 项测量约 1.25 ms；真实端到端待测量 |
| 空查询最近使用 | 小于 20 ms | 待测量 |
| Idle CPU | 接近 0% | 待测量 |
| Idle RAM | 尽量小于 80 MB | 待测量 |
| 完全断网运行 | 启动、搜索、剪贴板、取色、重命名、设置全部正常 | 待验证 |

检查离线行为时关闭网络后重新启动已安装版本，完成全部主要操作并观察进程连接；安装器也应在 WebView2 已就绪的断网环境中完成安装。开发用 Vite 本地服务与生产版运行行为分别记录。

## 范围约束

实现只扫描应用来源，不索引普通文件或遍历全盘；重命名不递归；剪贴板 Enter 只重新复制；取色不保存截图；运行时没有账号、云同步、遥测或主动网络请求。数据位于 `%APPDATA%\XTools`，与安装目录分离。

本记录不将代码具备的行为自动记为通过；后续补充手工结果时记录系统版本、显示器缩放、测试操作与实际结果。


## V2 / 0.2.0 验收记录

本次按本地 XTools_V2_Development_Spec.md 实现 V2-01 至 V2-11。以下状态区分实现、自动验证和真实系统集成；未勾选的项目仍需对应环境验收。

验证环境：Windows 10 Pro 22H2，build 19045，x64；微信 4.1.15.13。版本保持原应用标识 com.antique.xtools 和 AppData 数据目录；数据库结构不变。

### 已有证据

- [x] 28 项 Rust 核心测试，包括内容解码、微信临时路径边界、旧记录合并与收藏保留、原图删除后管理副本仍有效、自然排序、清除旧选择和迟到请求拒绝；V1 的重命名冲突、两阶段执行、回滚和撤销测试继续通过。
- [x] 前端类型检查、生产构建、格式检查、Clippy correctness 检查和版本一致性检查通过。Clippy 剩余建议来自 V1 图标模块的代码风格。
- [x] 前端导航与调用上下文队列测试通过；浏览器交互检查覆盖实际卡片布局、文本光标编辑、原生悬浮调用失败恢复、首开四文件上下文、执行中保护、新批次替换及规则重置。该浏览器检查使用 IPC mock，不作为 Windows 集成通过的证据。
- [x] 真实微信复制：来源 weixin.exe，CF_HDROP 单文件、未提供 PNG 注册格式；捕获为 image，285 × 113，独立 PNG 保存并重新打开数据库读取成功。日志未记录正文、私人路径或图像数据。PowerPoint PNG 格式捕获另验证为 128 × 94。
- [x] 规则标签 #52657D / #EAF3FF 对比度 5.34:1，输入框占位 #64748B / #FFFFFF 对比度 4.76:1；重命名页面视觉检查通过。
- [x] NSIS 安装器在 D 盘含空格隔离目录安装/卸载；桌面快捷方式可选、开机启动选择和一次性标记、运行中卸载、32/64 位注册表清理、保留 AppData 和非应用文件均通过。
- [x] 最终本地安装器 32/32 检查通过。另用真实 0.1.0 安装包在同一隔离目录运行中升级到 0.2.0；共 40/40 检查通过。自定义快捷键、历史限额、最近使用设置、文本/图片收藏、PNG 字节、撤销批次与路径记录保持一致，二次启动转交给已有进程。升级数据为隔离夹具，未改动日常 AppData。
- [x] 未创建桌面快捷方式时，开始菜单快捷方式的 System.AppUserModel.ID 实测为 com.antique.xtools。
- [x] 安装身份存在时，生产 WinRT 通知发送器的原生 short 通知被 Windows API 接受；标题为“颜色已复制”，测试正文 #FFFFFF。实际取色后显示/取消/复制失败的完整链路见待验收项。
- [x] 安装版 WebView 已渲染新版悬浮按钮、文案和四向提示；真实搜索 color 返回屏幕取色入口。
- [x] 原生 Explorer COM 原型：空选择返回 empty，不误带入父目录。四文件正向读取仍等待人工选择输入。

### 取色测量与实现

同设备的 read-only GDI 基准，预热 100 次，随后每路径 1000 次：V1 BitBlt + 81 次 GetPixel 平均 10.26 ms，P95 14.43 ms；V2 BitBlt + GdiFlush + DIB 内存访问平均 9.83 ms，P95 14.71 ms。平均改善约 4%，P95 未改善；这个微基准不包含窗口绘制、鼠标移动、CPU 和端到端帧率，不能作为 55–60 FPS 达标证明。

实现包括持久化 DIB、StretchBlt 放大、双缓冲、库存画刷、显示器缓存、活动时 16 ms/静止时 32 ms 调度、相同像素跳过绘制，以及避免每帧重置 timer。点击确认仍重新采样。可设置 XTOOLS_PICKER_METRICS=1 在隔离测试进程中写入 logs/picker-metrics.json；只含帧间隔与耗时，不含坐标或像素内容。

通知使用 Tauri 插件底层的 tauri-winrt-notification 直接发送，以明确指定 Short 并记录发送错误：当前 tauri-plugin-notification 的桌面包装会丢弃后台 show 错误。发送仍在关闭浮窗、释放钩子和绘图资源之后。

### 尚需真实环境验收

- [ ] A01/A02：安装版中观察真实微信缩略图，重新复制/粘贴，重启和原临时文件消失后的完整操作。
- [ ] A03：资源管理器复制单个 PNG 和多文件的真实回写语义（分类约束和原生文件回写代码已实现）。
- [ ] A04/A05：真正取色后 Toast 外观与实际 HEX、取消/复制失败无成功通知、无桌面快捷方式的完整操作。
- [ ] A08–A13：所有原生窗口的外部点击/悬浮、文件对话框、主屏切换、多屏与混合 DPI 操作。
- [ ] A14–A18：真实前台 Explorer 四文件端到端导入、替换、执行中保护；Windows 11 活动标签页。代码对隐藏/歧义视图拒绝导入，未将 Windows 10 原型当作 Windows 11 已通过。
- [ ] A21：至少 60 秒实际快速移动，平均 FPS/P95/跟随延迟/CPU，以及 100 次开启取消的 GDI/句柄测量。没有根据 timer 数值宣称性能达标。
- [ ] A22 界面复核：安装器运行中升级和各类数据记录保留已通过；升级后的图片显示/复制及撤销入口仍需真实界面操作确认。

Windows.Graphics.Capture 在本机返回 FrameArrived timeout，点击几何不可用。按 Computer Use 的恢复指引保留真实界面验收的限制；没有使用自制输入注入绕过该限制。可访问性检查和浏览器检查分别记录为对应层面的证据。

### 复现命令

~~~~powershell
npm ci
npm test
npm run build
node scripts/validate-version.mjs v0.2.0
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D clippy::correctness
npm run tauri build
.\scripts\installer-smoke.ps1
# 在微信复制测试图片后（输出目录须为隔离目录）
cargo run --manifest-path src-tauri/Cargo.toml --example clipboard_probe -- .tools/clipboard-probe
# 在资源管理器中选中隔离测试文件，直接运行无参数原型；可另指定测试窗口标题
cargo run --manifest-path src-tauri/Cargo.toml --example explorer_probe -- v2-four-files
cargo run --manifest-path src-tauri/Cargo.toml --example picker_gdi_benchmark
~~~~

安装脚本会备份并恢复 XTools 的注册表项和快捷方式；测试前通过 --quit 关闭运行中的 XTools，避免单实例将测试转交给日常实例。测试完成后可重新启动原程序。

### 正式发布与下载验证（2026-10-06）

- [x] A23：[v0.2.0 Release](https://github.com/0Antique/XTools/releases/tag/v0.2.0) 已发布，手动上传资产只有 [XTools_Setup.exe](https://github.com/0Antique/XTools/releases/download/v0.2.0/XTools_Setup.exe)。文件大小 2,786,871 字节，约 2.66 MiB。
- 发布标签实际指向已验证的功能提交 d3827614ef1e38a250f6a781330655364cca87d6。随后文档提交补充下载证据，不改变发布代码。
- [Release 工作流](https://github.com/0Antique/XTools/actions/runs/37339151033) 成功；该功能提交的 [PR Windows CI](https://github.com/0Antique/XTools/actions/runs/37338990655) 和 [分支 Windows CI](https://github.com/0Antique/XTools/actions/runs/37338840205) 均成功。
- 正式构建产物完成 33/33 安装/卸载检查，包括安装后的 EXE 实际版本 0.2.0、未创建桌面快捷方式时的通知身份、含空格目录、开机启动选项、运行中卸载、注册表清理和用户数据保留。随后从 Release 直接下载的 EXE 与这份已测产物逐字节哈希一致；无需解压。
- Release 安装器 SHA256：`B818FE0CEB9F73E8C8E2FC891AED706E056EAB3CBC008F883F06A6C6737C2EE4`。GitHub 资产 digest、Release 正文和下载文件计算结果一致。
- 本地下载副本为 artifacts/v0.2.0/XTools_Setup.exe。可用 `scripts/installer-smoke.ps1 -Installer <下载的 EXE 绝对路径>` 复测；此脚本需要先退出日常 XTools。

发布成功不等于上文所有原生交互与平台矩阵已验收。PR 保留草稿状态，等待对应环境补充证据。

## V2.1 界面小更新（2026-10-06）

发布名称 V2.1，标签 v2.1；Cargo、npm、Tauri 和安装器要求三段语义版本，工程版本为 2.1.0。关于页显示 V2.1。

- [x] 移除三个工具卡片的字母和回车图标；移除首页底部方向键、Enter、Esc 及设置入口整行。
- [x] 设置入口位于右上角悬浮按钮左侧；点击沿用 open_tool/settings，原生错误通过现有提示呈现。
- [x] 浏览器视觉与交互检查通过：按钮位置与顺序、设置调用及失败提示、关于页版本、窄窗口卡片无残留列。使用 IPC mock，截图保存在本地 .tools/v21-launcher-ui.png。
- [x] 前端类型检查、生产构建、15 项原有前端行为断言、Rust 格式及版本一致性检查通过；v2.1 与工程 2.1.0 对应。

本次只调整界面和版本标识；V2 部分尚未验证的平台与原生交互矩阵继续保留。
