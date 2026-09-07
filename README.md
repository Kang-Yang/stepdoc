# StepDoc

操作步骤图文教程录制工具 —— 轻量级 Tauri 桌面应用，自动记录屏幕点击并生成带截图、带文字标注的分步教程文档。

## 功能特性

- **点击自动录制**：监听全局鼠标点击（左键 / 右键），每次点击自动截取所在显示器的全屏，并在截图上用品牌青色圆环标注点击位置
- **OCR 自动命名**：每次点击后自动识别光标附近的界面文字，把步骤描述从「点击此处」自动更新为「点击「文件」」这类可读文案。优先使用 RapidOCR，在 Windows 上回退到系统自带 OCR
- **悬浮录制栏**：录制时弹出置顶悬浮栏，实时显示耗时与步数，可随时暂停 / 继续 / 结束，无需切回主窗口
- **实时预览与编辑**：步骤列表实时更新，可查看每步截图与时间戳，并支持手动修正步骤描述、删除单步
- **全局快捷键**：默认 `Ctrl + Alt + R` 随时开始 / 结束录制，可在设置中自定义或清除
- **分享教程**：支持导出为 GIF 动图（适合微信 / 钉钉发送）或 Word（`.docx`）图文步骤

## 环境要求

| 依赖 | 说明 |
|------|------|
| [Node.js](https://nodejs.org/) | 18+，用于前端构建 |
| [Rust](https://www.rust-lang.org/) | 1.70+，用于 Tauri 后端 |
| 操作系统 | 当前主要支持 **Windows**（`rdev` 全局输入监听、`screenshots` 截屏、`Windows.Media.Ocr` 作为 OCR 回退） |

> 首次使用 OCR 标注时，RapidOCR 需要联网下载一次识别模型（PPOCRV5_CH），之后离线可用；Windows 上的系统 OCR 回退无需额外依赖。
>
> 首次运行 `npm run tauri dev` 时，Rust 会下载并编译依赖，耗时较长属正常现象。

## 快速开始

```bash
# 安装前端依赖
npm install

# 启动开发模式（同时启动 Vite 前端与 Tauri 桌面窗口）
npm run tauri dev
```

## 构建发布

```bash
# 构建前端并打包桌面应用
npm run tauri build
```

构建产物位于 `src-tauri/target/release/`，安装包（NSIS）位于 `src-tauri/target/release/bundle/nsis/`。

## 使用说明

1. 点击 **开始录制**（或直接按全局快捷键 `Ctrl + Alt + R`，无需切回本应用），应用开始监听桌面鼠标操作并弹出悬浮录制栏
2. 在目标软件中正常操作 —— 每次点击自动生成一步，截图上以青色圆环标注点击位置
3. 录制过程中可从悬浮栏 **暂停 / 继续**，暂停期间的操作不会被记录
4. 再次按 `Ctrl + Alt + R` 或点击 **结束录制** 停止；停止后 OCR 会在后台为每步点击识别界面文字，主窗口会显示进度
5. 识别结果不准确时，可在步骤列表中直接修改描述，或删除某一步
6. 点击 **GIF 动图** 或 **Word 文档** 导出，在弹窗中选择保存位置后发给对方
7. 点击 **清空** 可清除当前所有步骤（录制中不可用）
8. 在 **设置 → 快捷键** 中可以修改或清除全局快捷键（支持 `Ctrl / Alt / Shift / Win` 组合键与 F1~F12 功能键）；**设置 → OCR 调试** 可开启「保存 OCR 调试明细」并打开调试目录

## 项目结构

```
stepdoc/
├── src/                          # React 前端
│   ├── main.tsx                  # 入口（按 ?view= 区分主窗口 / 录制悬浮栏）
│   ├── App.tsx                   # 主窗口入口
│   ├── components/               # UI 组件
│   │   ├── common/               # 通用组件
│   │   ├── icons/                # 图标
│   │   ├── layout/               # 布局（顶栏、操作区、设置、侧栏）
│   │   └── steps/                # 步骤列表与缩略图
│   ├── hooks/                    # React Hooks
│   ├── lib/                      # 工具与 Tauri 封装
│   │   ├── format/               # 格式化函数
│   │   ├── settings/             # 本地设置（录制选项、快捷键）
│   │   └── tauri/                # IPC 命令与事件
│   ├── styles/                   # 样式
│   ├── types/                    # TypeScript 类型
│   └── windows/                  # 窗口级页面（主窗口 / 录制悬浮栏）
├── src-tauri/                    # Tauri / Rust 后端
│   ├── src/
│   │   ├── lib.rs                # 应用入口
│   │   ├── commands/             # Tauri 命令（录制、导出、快捷键、OCR 调试）
│   │   ├── capture/              # 输入监听与步骤采集
│   │   ├── export/               # Word(.docx) / GIF 导出
│   │   ├── models/               # 数据模型与状态
│   │   ├── ocr/                  # OCR（RapidOCR / Windows 回退、候选筛选）
│   │   ├── platform/             # 平台相关代码
│   │   ├── screenshot/           # 全屏截图、裁剪与标注
│   │   ├── utils/                # 工具函数
│   │   └── window/               # 窗口管理
│   ├── gen/schemas/              # Tauri 生成的 schema
│   ├── icons/                    # 应用图标
│   ├── windows/                  # NSIS 安装器钩子
│   ├── examples/                 # Rust 调试示例
│   ├── capabilities/             # Tauri 能力声明
│   ├── Cargo.toml
│   └── tauri.conf.json
└── package.json
```

## 技术栈

- **前端**：React 19、TypeScript、Vite 7、`@tauri-apps/api`（dialog / opener 插件）
- **桌面框架**：[Tauri 2](https://v2.tauri.app/)
- **后端**：Rust
  - `rdev` — 全局鼠标事件监听
  - `screenshots` — 屏幕截图
  - `image` — 截图标注与图片编解码
  - `gif` + `color_quant` — GIF 编码（共享全局调色板 + Floyd–Steinberg 抖动）
  - `docx-rs` — 生成 Word `.docx` 文档
  - `rapidocr-core` — OCR 文字识别；Windows 上回退到 `windows` crate 的系统 OCR
  - `tauri-plugin-global-shortcut` — 全局快捷键
  - `uuid`、`chrono`、`base64`、`serde` — 基础能力

## 开发命令

| 命令 | 说明 |
|------|------|
| `npm run dev` | 仅启动 Vite 前端（http://localhost:1420） |
| `npm run tauri dev` | 启动完整桌面应用（推荐） |
| `npm run build` | 构建前端到 `dist/` |
| `npm run tauri build` | 构建并打包桌面应用 |
| `npm run icon` | 从 `app-icon.png` 生成应用图标 |

## 已知限制

- 录制期间监听的是**全局**输入事件，在其他窗口的点击同样会被记录
- 当前版本采集的是**鼠标点击**（左键 / 右键），键盘输入暂不单独生成步骤
- 每步截图为**操作所在显示器的全屏**；多显示器环境下，点击在哪块屏就截哪块屏
- OCR 标注偶尔会识别不准，可在步骤列表中手动修正描述；RapidOCR 首次使用需联网下载模型
- GIF 导出为无限循环，帧间隔固定 1.8 秒；宽度超过 1920px 的截图会等比缩小后合入动图
- Word 导出为真实 `.docx` 文档（宋体、编号标题 + 居中截图），适合逐步图文说明的场景
- 全局快捷键若与其他软件冲突会注册失败，可在设置中换一个组合键
- 全局输入监听可能被部分安全软件拦截

## License

本项目采用 [MIT License](LICENSE) 开源。