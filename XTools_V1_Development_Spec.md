# XTools V1 开发规格说明书

> **产品名称：** XTools  
> **目标平台：** Windows 10 / Windows 11 x64  
> **技术路线：** Tauri 2 + Rust + Vue 3 + TypeScript  
> **文档状态：** V1 需求冻结版  
> **用途：** 可直接交给 Codex 作为初版开发依据

---

## 1. 产品定位

XTools 是一款 **轻量、完全本地、面向 Windows 的效率工具**。

它参考 uTools 的核心交互方式，但不复制 uTools 的完整产品形态。XTools 不做插件生态、不做云服务、不做全盘文件搜索，而是围绕少量高频功能提供快速入口。

核心体验：

```text
Alt + Space
    ↓
快速呼出 XTools Launcher
    ↓
搜索应用 / 调用内置工具
```

### 1.1 核心原则

- 完全单机运行
- 不需要账号
- 不需要网络
- 不上传任何用户数据
- 不做遥测
- 不做广告
- 不做插件市场
- 不做复杂扩展系统
- 启动快
- 常驻资源占用低
- 所有主要操作路径尽量控制在 2～3 步以内
- 用户可以自定义软件安装位置

---

# 2. V1 功能范围

XTools V1 只包含以下功能：

```text
XTools
│
├── Launcher
│   ├── Alt + Space 呼出
│   ├── 应用搜索
│   ├── 中文搜索
│   ├── 拼音搜索
│   ├── 拼音首字母搜索
│   ├── 最近使用软件推荐
│   ├── 原始应用图标显示
│   └── Enter 启动应用
│
├── Clipboard
│   ├── 文本历史
│   ├── 图片历史
│   ├── 文件 / 文件夹路径历史
│   ├── 搜索
│   ├── 收藏
│   ├── 删除
│   ├── 清空
│   └── 默认保留 100 条普通历史
│
├── Color Picker
│   ├── 屏幕取色
│   ├── 像素放大镜
│   ├── HEX
│   ├── RGB
│   ├── HSL
│   └── 左键复制 HEX
│
├── Batch Rename
│   ├── 文件
│   ├── 文件夹
│   ├── 不递归
│   ├── 前缀 / 后缀
│   ├── 查找替换
│   ├── 自动编号
│   ├── 删除字符
│   ├── 大小写转换
│   ├── 实时预览
│   ├── 冲突检测
│   └── 撤销上一次操作
│
└── Settings
    ├── 全局快捷键
    ├── 开机自启动
    ├── 剪贴板最大历史数量
    ├── 最近使用开关
    ├── 重新扫描应用
    └── 关于
```

---

# 3. 明确不做的功能

V1 明确排除：

- 文件全盘搜索
- Everything 类文件索引
- 截图
- 截图悬浮
- OCR
- LaTeX 公式识别
- AI 功能
- 云同步
- 账号系统
- 插件市场
- 第三方插件系统
- 在线服务
- 遥测
- 广告
- macOS 支持
- Linux 支持
- Windows ARM 支持

---

# 4. 平台要求

目前只支持：

```text
Windows 10 x64
Windows 11 x64
```

不考虑：

```text
Windows ARM
macOS
Linux
```

---

# 5. 技术栈

## 5.1 推荐技术方案

| 模块 | 技术 |
|---|---|
| Desktop Framework | Tauri 2 |
| 后端 | Rust |
| 前端 | Vue 3 |
| 前端语言 | TypeScript |
| 构建工具 | Vite |
| 状态管理 | Pinia |
| 本地数据库 | SQLite |
| Rust SQLite | rusqlite |
| Windows API | windows crate |
| 安装器 | NSIS |
| 样式 | CSS / SCSS |
| 图标库 | Lucide 或自定义 SVG |

## 5.2 不使用

- Electron
- Python
- .NET 作为主程序
- Node.js 后台常驻进程
- OCR Runtime
- Web Server
- 云端 API

---

# 6. 总体架构

```text
┌─────────────────────────────────────────┐
│                 Vue 3                   │
│                                         │
│ Launcher / Clipboard / Rename / Setting │
└────────────────────┬────────────────────┘
                     │
                 Tauri IPC
                     │
┌────────────────────▼────────────────────┐
│                  Rust                   │
│                                         │
│ Hotkey        Application Discovery     │
│ Clipboard     Color Picker              │
│ Rename        SQLite                    │
│ Tray          Autostart                 │
│ Windows API   Icon Extraction           │
└─────────────────────────────────────────┘
```

原则：

- Vue 只负责 UI、交互状态和展示
- Rust 负责系统能力和核心逻辑
- 应用搜索算法统一放在 Rust
- 前端不重复实现另一套搜索算法
- SQLite 负责持久化
- 所有数据只保存在本机

---

# 7. 推荐项目结构

```text
XTools/
│
├── src/
│   ├── main.ts
│   ├── App.vue
│   │
│   ├── components/
│   │   ├── SearchBar.vue
│   │   ├── AppCard.vue
│   │   ├── ToolCard.vue
│   │   ├── ClipboardItem.vue
│   │   └── RenamePreview.vue
│   │
│   ├── views/
│   │   ├── LauncherView.vue
│   │   ├── ClipboardView.vue
│   │   ├── RenameView.vue
│   │   └── SettingsView.vue
│   │
│   ├── stores/
│   │   ├── launcher.ts
│   │   ├── clipboard.ts
│   │   └── settings.ts
│   │
│   ├── types/
│   │   ├── application.ts
│   │   ├── clipboard.ts
│   │   └── rename.ts
│   │
│   └── styles/
│       ├── global.scss
│       └── variables.scss
│
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   │
│   │   ├── launcher/
│   │   │   ├── mod.rs
│   │   │   ├── scanner.rs
│   │   │   ├── search.rs
│   │   │   ├── launch.rs
│   │   │   ├── pinyin.rs
│   │   │   └── icon.rs
│   │   │
│   │   ├── clipboard/
│   │   │   ├── mod.rs
│   │   │   ├── listener.rs
│   │   │   ├── parser.rs
│   │   │   └── storage.rs
│   │   │
│   │   ├── color_picker/
│   │   │   ├── mod.rs
│   │   │   └── picker.rs
│   │   │
│   │   ├── rename/
│   │   │   ├── mod.rs
│   │   │   ├── rules.rs
│   │   │   ├── executor.rs
│   │   │   └── history.rs
│   │   │
│   │   ├── database/
│   │   │   ├── mod.rs
│   │   │   └── schema.rs
│   │   │
│   │   ├── settings/
│   │   │   └── mod.rs
│   │   │
│   │   ├── tray/
│   │   │   └── mod.rs
│   │   │
│   │   └── windows/
│   │       ├── shortcuts.rs
│   │       ├── registry.rs
│   │       └── shell.rs
│   │
│   ├── icons/
│   └── tauri.conf.json
│
├── installer/
│   └── xtools.nsi
│
└── README.md
```

要求：

- 禁止把所有 Rust 逻辑堆进 `main.rs`
- 各模块相互独立
- 功能之间尽量低耦合
- 为未来扩展留接口，但不要提前实现未确认功能

---

# 8. Launcher

Launcher 是 XTools 最重要的功能。

---

## 8.1 默认快捷键

```text
Alt + Space
```

行为：

```text
XTools 隐藏
↓
Alt + Space
↓
显示 Launcher
```

再次：

```text
Launcher 已显示
↓
Alt + Space
↓
隐藏 Launcher
```

另外：

```text
Esc      隐藏 Launcher
↑ / ↓    切换搜索结果
Enter    启动应用 / 打开内置工具
```

Launcher 出现时：

- 搜索框自动获得焦点
- 默认清空上一次搜索内容

---

# 9. Launcher UI

交互逻辑参考 uTools，但视觉上做成 XTools 自己的简洁风格。

## 9.1 风格

- 无系统标题栏
- 无传统边框
- 圆角
- 轻阴影
- 白色 / 浅灰色背景
- 图标清晰
- 不使用复杂动画
- 不使用大面积高饱和度色块
- 不使用复杂侧边栏
- 搜索区域是视觉核心

## 9.2 建议尺寸

```text
宽：约 780 px
高：约 520 px
```

## 9.3 显示位置

- 当前显示器顶部偏中央
- 距离屏幕顶部约 100～140 px
- 优先使用当前前台窗口所在显示器
- 若无法判断，则使用鼠标所在显示器

---

# 10. Launcher 空输入状态

用户按：

```text
Alt + Space
```

但没有输入任何内容时：

```text
┌──────────────────────────────────────────────────┐
│ 🔍 搜索应用或 XTools 功能                       │
├──────────────────────────────────────────────────┤
│ 最近使用                                         │
│                                                  │
│  🟢 微信    🟣 VS Code    🔵 Edge    📚 Zotero   │
│                                                  │
├──────────────────────────────────────────────────┤
│ XTools                                           │
│                                                  │
│  📋 剪贴板       🎨 取色       📁 批量重命名    │
└──────────────────────────────────────────────────┘
```

要求：

- 不显示文件搜索
- 不扫描普通文件
- 默认推荐最近使用的软件
- 默认显示最近 8 个应用
- 同时显示 XTools 内置工具

---

# 11. Launcher 输入状态

输入：

```text
微信
```

示意：

```text
┌──────────────────────────────────────────────────┐
│ 微信                                             │
├──────────────────────────────────────────────────┤
│ 最佳搜索结果                                     │
│                                                  │
│   🟢 微信                                        │
│                                                  │
├──────────────────────────────────────────────────┤
│ 其他结果                                         │
│                                                  │
│   微信开发者工具                                 │
└──────────────────────────────────────────────────┘
```

输入：

```text
wx
```

也必须找到：

```text
微信
```

---

# 12. 应用扫描范围

V1 需要覆盖以下来源。

---

## 12.1 开始菜单

扫描：

```text
%APPDATA%\Microsoft\Windows\Start Menu\Programs

%PROGRAMDATA%\Microsoft\Windows\Start Menu\Programs
```

处理：

```text
*.lnk
```

解析：

- 应用名称
- 目标路径
- 参数
- 工作目录
- 图标

---

## 12.2 桌面

扫描：

```text
%USERPROFILE%\Desktop

C:\Users\Public\Desktop
```

处理：

- `.lnk`
- 可直接启动的 `.exe`

---

## 12.3 传统 Win32 已安装程序

补充扫描 Registry：

```text
HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall

HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall

HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall
```

需要过滤明显不是用户应用的项目，例如：

- Runtime
- Driver
- SDK
- Update
- Microsoft Visual C++ Redistributable
- 系统组件

Registry 仅作为补充来源。

---

## 12.4 Microsoft Store / UWP

需要支持：

- Microsoft Store
- Windows Terminal
- Calculator
- Photos
- 其他 UWP / Store App

建议通过：

```text
shell:AppsFolder
```

获取：

- AUMID
- DisplayName
- Icon

并使用 Windows Shell / Application Activation API 启动。

---

# 13. 应用数据结构

建议：

```rust
struct Application {
    id: String,
    name: String,
    normalized_name: String,
    pinyin: String,
    pinyin_initials: String,
    source: AppSource,
    launch_type: LaunchType,
    launch_target: String,
    arguments: Option<String>,
    icon_path: Option<String>,
    launch_count: u64,
    last_launched_at: Option<i64>,
}
```

示例：

```text
name:
微信

normalized_name:
微信

pinyin:
weixin

pinyin_initials:
wx
```

以下搜索均应命中：

```text
微信
weixin
wx
wei
weix
```

---

# 14. 拼音搜索

中文名称需要预先生成：

- 完整拼音
- 拼音首字母

示例：

```text
微信
↓
weixin
↓
wx
```

输入：

```text
wx
```

必须能快速找到微信。

建议：

- 拼音预计算
- 存入 SQLite
- 加载到内存
- 搜索时不要重新生成拼音

---

# 15. 搜索评分规则

建议：

| 匹配类型 | 分数 |
|---|---:|
| 名称完全匹配 | 1000 |
| 名称前缀匹配 | 900 |
| 拼音首字母完全匹配 | 880 |
| 拼音首字母前缀匹配 | 850 |
| 完整拼音前缀匹配 | 820 |
| 名称包含 | 700 |
| 拼音包含 | 650 |
| 模糊匹配 | 500 |

最终：

```text
total_score
=
match_score
+
usage_bonus
+
recency_bonus
```

要求：

- 精确匹配始终优先
- 使用频率不能压过明显更准确的结果
- 最近使用仅做合理加权

---

# 16. 最近使用

每次通过 XTools 成功启动应用：

```text
launch_count += 1
last_launched_at = now
```

空搜索状态：

```text
ORDER BY last_launched_at DESC
```

默认显示：

```text
8 个
```

最近使用只统计：

```text
通过 XTools 启动过的应用
```

V1 不要求读取 Windows 系统级最近使用历史。

---

# 17. 内置工具搜索

XTools 内置工具也参与 Launcher 搜索。

建议别名：

| 功能 | Alias |
|---|---|
| 剪贴板 | 剪贴板、clipboard、cb |
| 取色 | 取色、颜色、color、picker |
| 重命名 | 重命名、rename、batch、rn |
| 设置 | 设置、setting、settings |

结构：

```rust
struct BuiltinTool {
    id: String,
    name: String,
    aliases: Vec<String>,
}
```

示例：

```text
cb
```

应找到：

```text
剪贴板管理
```

---

# 18. 应用图标

必须显示应用原始图标。

来源：

- `.exe`
- `.lnk`
- UWP Package

缓存目录：

```text
%APPDATA%\XTools\cache\icons\
```

要求：

- 提取一次后缓存
- 后续优先读取缓存
- 只有无法提取时使用 XTools 默认图标

---

# 19. 应用去重

同一个应用可能存在于：

- Start Menu
- Desktop
- Registry

不能显示三个相同应用。

唯一键优先：

```text
AUMID
```

否则：

```text
normalized executable path
```

来源优先级建议：

```text
Start Menu
>
Desktop
>
Registry
```

---

# 20. 应用索引加载策略

不要每次启动都等完整扫描。

正确流程：

```text
XTools 启动
↓
读取 SQLite 缓存
↓
Launcher 立即可用
↓
后台重新扫描应用
↓
更新索引
```

要求：

- 后台刷新不得阻塞 UI
- 扫描失败时继续使用旧缓存

---

# 21. Clipboard

支持三类内容：

1. 文本
2. 图片
3. 文件 / 文件夹路径

---

# 22. Clipboard 监听

禁止高频轮询。

使用 Windows 原生机制：

```text
AddClipboardFormatListener
WM_CLIPBOARDUPDATE
```

目标：

```text
Idle CPU 接近 0%
```

只有剪贴板发生变化时才处理。

---

# 23. 文本剪贴板

保存：

- 原始文本
- 预览文本
- hash
- 创建时间
- 收藏状态

示例：

```text
Hello World
```

```text
一段论文内容
```

```text
https://github.com/...
```

---

# 24. 图片剪贴板

支持来自：

- 浏览器
- Word
- 微信
- 其他 Windows 软件

的常见位图数据。

建议统一存成：

```text
PNG
```

保存路径：

```text
%APPDATA%\XTools\clipboard\images\
```

SQLite 只存：

- 文件路径
- hash
- 时间
- 收藏状态
- 必要预览信息

不要把大图片 BLOB 直接塞进 SQLite。

---

# 25. 文件 / 文件夹剪贴板

仅保存路径列表。

例如：

```text
D:\Papers\FedAvg.pdf
```

```text
D:\Projects\
```

不要复制文件本身到 XTools 数据目录。

---

# 26. Clipboard 最大历史数量

默认：

```text
100 条普通历史
```

当普通历史达到 100 条后又新增一条：

```text
删除最旧的一条普通记录
```

收藏项：

```text
不参与自动清理
```

所以实际逻辑：

```text
100 条普通历史 + 任意数量收藏
```

设置页允许修改数量。

---

# 27. Clipboard 去重

连续复制相同内容：

```text
Hello
Hello
Hello
```

不能产生三条记录。

建议使用稳定 Hash。

行为：

```text
发现相同内容
↓
更新 created_at
↓
移动到最新位置
↓
不重复插入
```

---

# 28. Clipboard UI

示意：

```text
┌──────────────────────────────────────────┐
│ 🔍 搜索剪贴板                           │
├──────────────────────────────────────────┤
│ 今天                                     │
│                                          │
│ 📄 Federated learning is...       ⭐     │
│                                          │
│ 🖼 [图片预览]                            │
│                                          │
│ 📁 D:\Papers\FedAvg.pdf                  │
│                                          │
│ 📄 https://github.com/...                │
└──────────────────────────────────────────┘
```

功能：

- 搜索
- 收藏
- 删除
- 清空
- 重新复制

---

# 29. Clipboard Enter 行为【已冻结】

采用方案：

## A：只重新复制到 Windows Clipboard，然后关闭 XTools

流程：

```text
用户选择一条历史记录
↓
Enter
↓
重新写入 Windows Clipboard
↓
关闭 / 隐藏 XTools
```

不做：

- 自动切回上一窗口
- 自动粘贴
- 模拟 Ctrl + V

示例：

```text
选择“你好”
↓
Enter
↓
Windows Clipboard = “你好”
↓
XTools 关闭
↓
用户自行 Ctrl + V
```

这是 V1 的固定行为。

---

# 30. Color Picker

入口关键词：

```text
取色
颜色
color
picker
```

进入后：

1. Launcher 隐藏
2. 进入取色模式
3. 鼠标附近显示像素放大镜
4. 实时显示当前颜色

示意：

```text
┌──────────────┐
│ ▓▓▓▓▓▓▓▓▓▓  │
│ ▓▓▓╋▓▓▓▓▓▓  │
│ ▓▓▓▓▓▓▓▓▓▓  │
├──────────────┤
│ #F3C0C1      │
│ 243 192 193  │
│ HSL(...)     │
└──────────────┘
```

---

# 31. Color Picker 操作

```text
左键
```

复制：

```text
#RRGGBB
```

例如：

```text
#F3C0C1
```

然后退出取色模式。

其他操作：

```text
Esc
```

取消退出。

```text
右键
```

取消退出。

浮窗显示：

- HEX
- RGB
- HSL

默认写入 Clipboard：

```text
HEX
```

---

# 32. Color Picker 技术实现

不实现截图功能。

建议使用：

```text
GetCursorPos
GetDC
GetPixel
```

像素放大镜可使用：

```text
BitBlt
```

要求：

- 仅实时采样
- 不保存截图
- 不生成截图文件
- 不提供截图编辑

---

# 33. 多显示器与 DPI

从 V1 第一版就处理：

```text
Per-Monitor DPI Awareness V2
```

必须正确应对：

- 100%
- 125%
- 150%
- 175%
- 多显示器不同缩放比例

要求：

- Launcher 出现在正确显示器
- 鼠标坐标正确
- 取色位置不偏移
- DPI 相关坐标转换封装成独立模块

---

# 34. Batch Rename

入口：

```text
Alt + Space
↓
rename
↓
Enter
```

打开独立窗口。

---

# 35. Batch Rename 支持对象

支持：

- 文件
- 文件夹
- 文件和文件夹混合拖入

不支持：

```text
递归处理
```

示例：

拖入：

```text
D:\Papers
```

只允许重命名：

```text
Papers
```

绝不能自动修改其内部文件。

---

# 36. Batch Rename 规则

V1 支持：

1. 添加前缀
2. 添加后缀
3. 查找替换
4. 自动编号
5. 删除指定字符
6. 大小写转换

---

# 37. 自动编号

支持：

```text
起始值
步长
位数
```

例如：

```text
起始：1
步长：1
位数：3
```

结果：

```text
001
002
003
```

---

# 38. 实时预览

示意：

```text
┌───────────────────┬────────────────────────────┐
│ 原名称            │ 新名称                     │
│ IMG_001.jpg       │ Paper_01.jpg               │
│ IMG_002.jpg       │ Paper_02.jpg               │
│ IMG_003.jpg       │ Paper_03.jpg               │
└───────────────────┴────────────────────────────┘
```

规则变化时：

```text
实时更新预览
```

执行前必须检查：

- 新名称是否重复
- 是否包含非法字符
- 目标路径是否已存在
- 名称是否为空
- 原文件是否仍存在
- 原文件夹是否仍存在

出错时：

- 对应行标红
- 禁止执行
- 提示明确原因

---

# 39. 安全重命名

不要简单逐个直接改最终名称。

使用两阶段 rename：

```text
阶段 1：

A → .xtools_tmp_UUID1
B → .xtools_tmp_UUID2

阶段 2：

tmp1 → B
tmp2 → A
```

要求：

- 临时名称必须唯一
- 不覆盖未知文件
- 中途异常时尽可能回滚
- 返回明确错误原因

---

# 40. 撤销上一次重命名

每次成功操作保存：

```text
batch_id
old_path
new_path
timestamp
```

支持：

```text
撤销上一次操作
```

撤销时执行反向 rename。

如果原文件名已经被其他对象占用：

- 不强制覆盖
- 停止撤销
- 提示冲突

---

# 41. Settings

V1 设置界面：

```text
XTools 设置

通用
────────────────

☑ 开机自动启动

快捷键
Alt + Space

剪贴板
最大历史数量
100

启动器
☑ 显示最近使用

[重新扫描应用]

关于
XTools
Version 0.1.0
```

不做：

- 账号
- 云同步
- 插件
- 在线市场
- 主题商城
- 遥测

---

# 42. 快捷键设置

默认：

```text
Alt + Space
```

允许修改。

流程：

```text
点击快捷键设置
↓
进入快捷键录制状态
↓
用户按组合键
↓
尝试注册
```

如果被占用：

```text
无法注册该快捷键
```

要求：

- 不保存失败的新快捷键
- 自动恢复原快捷键
- 不出现“设置显示成功但实际不可用”的状态

---

# 43. 开机自启动

默认：

```text
开启
```

要求：

- 安装完成后默认启用
- 设置中可以关闭
- 设置修改立即生效
- 关闭后下次开机不自动启动

---

# 44. 系统托盘

XTools 启动后常驻系统托盘。

右键菜单：

```text
打开 XTools

剪贴板

批量重命名

设置

────────

开机启动 ✓

────────

退出 XTools
```

要求：

- 普通关闭窗口不退出 XTools
- `Esc` 仅隐藏 Launcher
- 只有“退出 XTools”真正退出进程

---

# 45. 单实例

必须保证：

```text
系统中只有一个 XTools 实例
```

用户重复启动：

```text
XTools.exe
XTools.exe
XTools.exe
```

行为：

```text
新实例发现已有实例
↓
通知已有实例显示 Launcher
↓
新实例退出
```

---

# 46. SQLite 数据库

默认路径：

```text
%APPDATA%\XTools\data\xtools.db
```

---

## 46.1 applications

```sql
CREATE TABLE applications (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    normalized_name TEXT,
    pinyin TEXT,
    pinyin_initials TEXT,
    source TEXT,
    launch_type TEXT,
    launch_target TEXT,
    arguments TEXT,
    icon_path TEXT,
    unique_key TEXT UNIQUE,
    last_seen_at INTEGER
);
```

---

## 46.2 app_usage

```sql
CREATE TABLE app_usage (
    app_id TEXT PRIMARY KEY,
    launch_count INTEGER DEFAULT 0,
    last_launched_at INTEGER
);
```

---

## 46.3 clipboard_items

```sql
CREATE TABLE clipboard_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_type TEXT NOT NULL,
    text_content TEXT,
    data_path TEXT,
    preview TEXT,
    content_hash TEXT,
    created_at INTEGER NOT NULL,
    is_favorite INTEGER DEFAULT 0
);
```

---

## 46.4 rename_batches

```sql
CREATE TABLE rename_batches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER NOT NULL
);
```

---

## 46.5 rename_items

```sql
CREATE TABLE rename_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    batch_id INTEGER NOT NULL,
    old_path TEXT NOT NULL,
    new_path TEXT NOT NULL
);
```

---

# 47. 数据目录

建议：

```text
%APPDATA%\XTools\
```

结构：

```text
XTools
│
├── config.json
│
├── data
│   └── xtools.db
│
├── clipboard
│   └── images
│
├── cache
│   └── icons
│
└── logs
```

安装目录和数据目录分离。

例如：

```text
程序：
D:\Applications\XTools\
```

```text
数据：
C:\Users\<User>\AppData\Roaming\XTools\
```

---

# 48. 安装器

最终产物：

```text
XTools_Setup.exe
```

要求：

- NSIS
- 可自定义安装路径
- 不强制安装到 C 盘固定位置
- 提供桌面快捷方式选项
- 提供开机自启动选项
- 开机自启动默认勾选

安装流程示意：

```text
欢迎使用 XTools
↓
选择安装位置

D:\Applications\XTools

[浏览...]

↓
附加选项

☑ 创建桌面快捷方式
☑ 开机自动启动

↓
安装
```

“自定义安装位置”属于强制需求。

---

# 49. 网络行为

XTools V1：

```text
0 网络依赖
```

禁止主动：

- 请求服务器
- 上传剪贴板
- 上传应用列表
- 上传日志
- 下载配置
- 登录账号
- 云同步
- 遥测

软件在完全断网情况下仍应正常运行所有 V1 功能。

---

# 50. 程序启动流程

建议：

```text
XTools.exe
↓
检查单实例
↓
初始化日志
↓
读取 config.json
↓
初始化 SQLite
↓
读取应用缓存
↓
注册 Alt + Space
↓
创建系统托盘
↓
启动 Clipboard Listener
↓
后台刷新应用索引
↓
保持隐藏
```

---

# 51. Launcher 搜索流程

示例：

```text
Alt + Space
↓
输入 wx
↓
Rust Search Engine
↓
Application Index
↓
匹配：

微信
pinyin_initials = wx

↓
返回 Vue
↓
显示微信真实图标
↓
Enter
↓
Rust Launcher
↓
启动 WeChat
↓
更新 app_usage
↓
隐藏 XTools
```

---

# 52. 性能目标

| 指标 | V1 目标 |
|---|---:|
| Alt + Space → Launcher 显示 | < 150 ms |
| 搜索响应 | < 30 ms |
| 空查询最近应用 | < 20 ms |
| Idle CPU | 接近 0% |
| Idle RAM | 尽量 < 80 MB |
| 应用索引数量 | 至少 2000 |
| Clipboard 普通历史 | 默认 100 |
| 安装包大小 | 目标 15～30 MB |
| 网络请求 | 0 |

性能目标优先级：

1. 稳定
2. 功能正确
3. 快捷键响应及时
4. 搜索流畅
5. 再做极限资源优化

---

# 53. UI 设计原则

XTools 不直接复制 uTools 视觉。

参考的是：

- 快速呼出
- 顶部搜索
- 最佳结果
- 最近使用
- 低操作成本

视觉原则：

- 白色 / 浅灰背景
- 大圆角
- 轻阴影
- 应用图标 40～48 px
- 留白充分
- 文字精简
- 不使用复杂动画
- 不使用夸张渐变
- 不使用重型侧栏
- 选中项使用淡灰色背景即可

---

# 54. 开发阶段规划

## Phase 1：项目骨架

内容：

- Tauri 2
- Vue 3
- TypeScript
- Vite
- Rust 基础模块
- 基础窗口

验收：

- `npm run tauri dev` 可以运行
- 项目结构符合本文档
- 无无关依赖

---

## Phase 2：托盘 + 单实例

内容：

- 系统托盘
- 托盘菜单
- 正确退出
- 单实例

验收：

- 重复启动只存在一个实例
- 托盘菜单可正常工作
- 普通关闭窗口不会退出进程

---

## Phase 3：Alt + Space

内容：

- 注册全局快捷键
- 显示 / 隐藏 Launcher
- Esc 隐藏
- 自动聚焦

验收：

- 全局快捷键稳定
- Launcher 呼出速度快
- 不与窗口状态冲突

---

## Phase 4：Launcher UI

内容：

- 搜索框
- 最近使用区域
- XTools 工具区域
- 搜索结果
- 键盘导航

验收：

- 无文件搜索
- 空输入显示最近使用
- `↑ / ↓ / Enter / Esc` 正常

---

## Phase 5：Start Menu / Desktop 应用扫描

内容：

- Start Menu
- Desktop
- `.lnk`
- `.exe`
- 图标提取

验收：

- 常见程序能被发现
- 显示真实图标
- Enter 可正常启动

---

## Phase 6：拼音搜索

内容：

- 中文
- 完整拼音
- 拼音首字母

验收：

```text
wx
```

必须找到：

```text
微信
```

---

## Phase 7：Registry / UWP

内容：

- Registry
- Win32
- UWP
- Store App
- AUMID
- AppsFolder

验收：

- Microsoft Store 应用可以搜索和打开
- Runtime / Driver 等垃圾项目被合理过滤

---

## Phase 8：最近使用

内容：

- launch_count
- last_launched_at
- 最近 8 个

验收：

- 通过 XTools 打开的软件会进入最近使用
- 最近打开的优先显示

---

## Phase 9：Clipboard

内容：

- 文本
- 图片
- 文件 / 文件夹
- SQLite
- 搜索
- 收藏
- 删除
- 清空
- 去重
- 100 条普通历史

验收：

- 三类数据均可正确记录
- 达到 100 条后自动删除最旧普通记录
- 收藏项不会被自动删除
- Enter 只重新复制到 Clipboard 并关闭 XTools

---

## Phase 10：Color Picker

内容：

- 屏幕像素读取
- 放大镜
- HEX
- RGB
- HSL

验收：

- 左键复制 HEX
- Esc / 右键取消
- 多显示器和 DPI 正确

---

## Phase 11：Batch Rename

内容：

- 文件 / 文件夹
- 拖入
- 不递归
- 前缀
- 后缀
- 查找替换
- 编号
- 删除字符
- 大小写
- 预览
- 两阶段 Rename
- 撤销

验收：

- 不误处理子目录
- 冲突时禁止执行
- 可撤销最近一次成功操作

---

## Phase 12：Settings

内容：

- 快捷键
- 开机启动
- Clipboard 最大数量
- 最近使用开关
- 重新扫描应用

验收：

- 设置立即生效
- 失败的快捷键不会被保存
- 开机启动可以实时开关

---

## Phase 13：Installer

内容：

- `XTools_Setup.exe`
- NSIS
- 自定义安装路径
- 桌面快捷方式
- 开机启动选项

验收：

- 可安装至 D 盘等任意正常目录
- 卸载正常
- 安装后可正常启动
- 默认开启开机启动

---

## Phase 14：测试与优化

测试范围：

- Windows 10 x64
- Windows 11 x64
- 100 / 125 / 150 / 175% DPI
- 单显示器
- 多显示器
- 中英文应用
- UWP 应用
- Win32 应用
- 大量 Clipboard 历史
- 文件 / 文件夹混合 Rename

---

# 55. V1 验收清单

## Launcher

- [ ] `Alt + Space` 可以呼出
- [ ] 再按一次可以隐藏
- [ ] `Esc` 可以隐藏
- [ ] 搜索框自动聚焦
- [ ] 空输入显示最近使用
- [ ] 空输入显示 XTools 内置工具
- [ ] 不存在文件搜索
- [ ] Start Menu 程序可搜索
- [ ] Desktop 程序可搜索
- [ ] Win32 应用可搜索
- [ ] UWP 应用可搜索
- [ ] `wx` 可以搜索到“微信”
- [ ] 显示应用真实图标
- [ ] Enter 启动应用
- [ ] 启动后更新最近使用

## Clipboard

- [ ] 支持文本
- [ ] 支持图片
- [ ] 支持文件
- [ ] 支持文件夹
- [ ] 默认 100 条普通历史
- [ ] 收藏不自动清理
- [ ] 支持搜索
- [ ] 支持删除
- [ ] 支持清空
- [ ] 支持去重
- [ ] Enter 仅复制，不自动粘贴

## Color Picker

- [ ] 显示 HEX
- [ ] 显示 RGB
- [ ] 显示 HSL
- [ ] 左键复制 HEX
- [ ] Esc 退出
- [ ] 右键退出
- [ ] 支持多显示器
- [ ] DPI 正确

## Batch Rename

- [ ] 支持文件
- [ ] 支持文件夹
- [ ] 不递归
- [ ] 支持前缀
- [ ] 支持后缀
- [ ] 支持查找替换
- [ ] 支持自动编号
- [ ] 支持删除字符
- [ ] 支持大小写转换
- [ ] 实时预览
- [ ] 冲突检查
- [ ] 安全两阶段 Rename
- [ ] 可撤销上一次

## System

- [ ] 系统托盘
- [ ] 单实例
- [ ] 默认开机启动
- [ ] 设置中可关闭开机启动
- [ ] 快捷键可修改
- [ ] 自定义安装位置
- [ ] 完全离线可用
- [ ] 无账号
- [ ] 无遥测
- [ ] 无云服务

---

# 56. 建议的初版开发边界

第一次交给 Codex 时，建议优先只开发：

```text
Phase 1 ～ Phase 6
```

也就是先完成：

```text
XTools
↓
托盘驻留
↓
Alt + Space
↓
Launcher UI
↓
扫描 Start Menu / Desktop
↓
显示真实应用 Icon
↓
中文 / 拼音 / 首字母搜索
↓
wx → 微信
↓
Enter → 启动微信
```

这是 XTools 最核心的产品闭环。

Clipboard、Color Picker、Batch Rename 再逐阶段加入。

---

# 57. 可直接给 Codex 的第一阶段 Prompt

```text
开发一个名为 XTools 的 Windows 10 / Windows 11 x64 桌面效率工具。

技术栈必须使用：
- Tauri 2
- Rust
- Vue 3
- TypeScript
- Vite

XTools 是完全本地软件，不使用服务器、账号、云服务、遥测、插件系统、OCR、截图和文件搜索。

第一阶段只实现 Launcher 核心能力。

要求：

1. XTools 启动后常驻 Windows 系统托盘，不主动显示主窗口。
2. 系统中只能存在一个 XTools 实例。
3. 默认启用开机自动启动。
4. 注册全局快捷键 Alt + Space。
5. 按 Alt + Space 后，在当前显示器顶部偏中央显示一个无标题栏、圆角、有轻微阴影的 Launcher 窗口。
6. 再次按 Alt + Space 或按 Esc 隐藏 Launcher。
7. Launcher 出现时搜索框自动获得焦点。
8. UI 参考 uTools 的交互方式，但不要复制其视觉设计。
9. 顶部为大型简洁搜索输入区域。
10. 没有输入内容时显示“最近使用”应用，以及 XTools 内置工具入口。
11. 不实现任何文件搜索功能。
12. 扫描 Windows Start Menu、用户 Desktop、Public Desktop 中的应用程序和快捷方式。
13. 处理 .lnk 和常见 .exe。
14. 对每个应用保存名称、启动目标、Icon、拼音、拼音首字母等信息。
15. 中文应用必须支持：
    - 中文名称搜索
    - 完整拼音搜索
    - 拼音首字母搜索
16. 例如“微信”需要索引：
    - 微信
    - weixin
    - wx
17. 用户输入 wx 必须能搜索到微信。
18. 搜索结果使用应用真实 Icon。
19. 使用上下方向键切换结果。
20. 按 Enter 启动应用。
21. 启动成功后隐藏 Launcher。
22. 为后续记录 launch_count 和 last_launched_at 预留结构。
23. 使用 SQLite 持久化应用索引。
24. XTools 启动时优先读取 SQLite 缓存，使 Launcher 可以立即使用，然后后台刷新应用索引。
25. Rust 代码必须模块化，禁止把所有逻辑写入 main.rs。
26. Launcher、scanner、search、icon、database、tray、settings 必须拆分为独立模块。
27. 从项目第一天开始处理 Windows 多显示器和 Per-Monitor DPI Awareness V2。
28. 本阶段不要实现 Clipboard、Color Picker、Batch Rename，只预留模块接口。
29. 完成后提供：
    - 项目结构
    - 运行方式
    - 已完成功能
    - 未完成功能
    - 已知问题
30. 必须保证 npm run tauri dev 可以正常运行。
```

---

# 58. 后续 Codex 开发原则

每次只给 Codex 一个 Phase。

不要一次要求 Codex 同时完成全部功能。

推荐：

```text
Phase 1-6
↓
本机验收
↓
修 Bug
↓
Phase 7-8
↓
本机验收
↓
Phase 9
↓
本机验收
↓
Phase 10
↓
本机验收
↓
Phase 11
↓
本机验收
↓
Settings + Installer
```

原因：

- 更容易定位问题
- 避免 Codex 一次生成过多不可维护代码
- 每一步都可以得到可运行版本
- 更容易控制产品边界
- 防止 Codex 擅自增加不需要的功能

---

# 59. V1 最终产品定义

XTools V1 最终应当是一款：

> **Windows 10 / 11 x64 下，完全本地、轻量、无账号、无云服务，以 Alt + Space 为统一入口，用于快速启动应用、管理剪贴板、屏幕取色和批量重命名的效率工具。**

核心体验必须满足：

```text
启动 Windows
↓
XTools 后台自动启动
↓
几乎无存在感
↓
需要时按 Alt + Space
↓
立即得到想要的应用或工具
```

其中 Launcher 的体验优先级最高。

如果 V1 的 Launcher 做得足够快、稳定、自然，那么 XTools 的产品基础就已经成立。
