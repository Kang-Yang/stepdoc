# StepDoc

操作步骤图文教程录制工具 —— 轻量级 Tauri 桌面应用，自动记录屏幕操作并生成带截图的分步教程文档。

## 功能特性

- **自动录制**：监听桌面鼠标点击，每步自动截取当前显示器全屏，并在截图上标注点击位置
- **键盘输入记录**：录制期间合并连续键盘输入，在回车、Tab、Esc 或停顿后生成步骤
- **全局快捷键**：默认 `Ctrl + Alt + R` 随时开始 / 结束录制，无需切回应用窗口，可在设置中自定义
- **实时预览**：步骤列表实时更新，可查看每步截图与时间戳
- **分享教程**：支持导出为 GIF 动图（适合微信/钉钉发送）或 Word（`.doc`）图文步骤

## 环境要求

| 依赖 | 说明 |
|------|------|
| [Node.js](https://nodejs.org/) | 18+，用于前端构建 |
| [Rust](https://www.rust-lang.org/) | 1.70+，用于 Tauri 后端 |
| 操作系统 | 当前主要支持 **Windows**（使用 `rdev` 全局输入监听与 `screenshots` 截屏） |

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

构建产物位于 `src-tauri/target/release/`，安装包位于 `src-tauri/target/release/bundle/`。

## 使用说明

1. 点击 **开始录制**，应用开始监听桌面上的鼠标与键盘操作（或直接按全局快捷键 `Ctrl + Alt + R`，无需切回本应用）
2. 在目标软件中正常操作 —— 每次点击会自动生成一步，截图上会显示红色圆圈标记点击位置
3. 输入文字后按回车 / Tab，或停顿约 1.2 秒，会合并为一条「键盘输入」步骤
4. 再次按 `Ctrl + Alt + R` 或点击 **停止录制** 结束录制
5. 点击 **GIF 动图** 或 **Word** 导出，在弹窗中选择保存位置后发给对方
6. 点击 **清空** 可清除当前所有步骤（录制中不可用）
7. 在 **设置 → 快捷键** 中可以修改或清除全局快捷键（支持 `Ctrl / Alt / Shift / Win` 组合键与 F1~F12 功能键）

## 项目结构

```
stepdoc/
├── src/                          # React 前端
│   ├── main.tsx                  # 入口（主窗口 / 录制悬浮栏）
│   ├── App.tsx                   # 主窗口入口
│   ├── components/               # UI 组件
│   │   ├── common/               # 通用组件
│   │   ├── icons/                # 图标
│   │   ├── layout/               # 布局（顶栏、操作区、侧栏）
│   │   └── steps/                # 步骤列表
│   ├── hooks/                    # React Hooks
│   ├── lib/                      # 工具与 Tauri 封装
│   │   ├── format/               # 格式化函数
│   │   ├── settings/             # 本地设置（录制选项、快捷键）
│   │   └── tauri/                # IPC 命令与事件
│   ├── styles/                   # 样式
│   ├── types/                    # TypeScript 类型
│   └── windows/                  # 窗口级页面
├── src-tauri/                    # Tauri / Rust 后端
│   ├── src/
│   │   ├── lib.rs                # 应用入口
│   │   ├── commands/             # Tauri 命令
│   │   ├── capture/              # 输入监听与步骤采集
│   │   ├── export/               # Word / GIF 导出
│   │   ├── models/               # 数据模型与状态
│   │   ├── platform/             # 平台相关代码
│   │   ├── screenshot/           # 全屏截图与标注
│   │   ├── utils/                # 工具函数
│   │   └── window/               # 窗口管理
│   └── tauri.conf.json
└── package.json
```

## 技术栈

- **前端**：React 19、TypeScript、Vite 7
- **桌面框架**：[Tauri 2](https://v2.tauri.app/)
- **后端**：Rust
  - `rdev` — 全局鼠标/键盘事件监听
  - `screenshots` — 屏幕截图
  - `image` — 截图标注与 PNG 编码

## 开发命令

| 命令 | 说明 |
|------|------|
| `npm run dev` | 仅启动 Vite 前端（http://localhost:1420） |
| `npm run tauri dev` | 启动完整桌面应用（推荐） |
| `npm run build` | 构建前端到 `dist/` |
| `npm run tauri build` | 构建并打包桌面应用 |
| `npm run icon` | 从 `app-icon.png` 生成应用图标 |

## 已知限制

- 录制期间监听的是**全局**输入事件，在其他窗口的操作也会被记录
- 每步截图为**操作所在显示器的全屏**；多显示器环境下，点击在哪块屏就截哪块屏
- Word 导出为 HTML 格式的 `.doc` 文件，适合需要逐步图文说明的场景
- 全局输入监听在部分系统上可能需要管理员权限
- 全局快捷键若与其他软件的快捷键冲突会注册失败，可在设置中换一个组合键

## License

本项目采用 [MIT License](LICENSE) 开源。
