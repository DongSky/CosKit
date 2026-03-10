# CosKit

AI 驱动的人像修图桌面应用，基于 Gemini API 实现智能修图、背景替换和 Cosplay 特效。

## 功能

- **智能修图**：上传照片，输入自然语言指令，AI 自动完成美颜、磨皮、光线优化
- **背景替换**：自动分析场景并推荐匹配背景，保持透视一致性
- **Cosplay 特效**：识别 Cosplay 摄影，自动添加轻度氛围光效和粒子效果
- **分支编辑**：基于任意历史节点创建新分支，支持树状编辑历史
- **会话管理**：多会话支持，自动保存，随时切换
- **图片导出**：原始分辨率导出，原生系统保存对话框

## 下载

前往 [Releases](https://github.com/DongSky/CosKit/releases) 下载最新版本：

| 平台 | 文件 |
|------|------|
| macOS (Apple Silicon) | `CosKit_x.x.x_aarch64.dmg` |
| macOS (Intel) | `CosKit_x.x.x_x64.dmg` |
| Windows (安装版) | `CosKit_x.x.x_x64-setup.exe` 或 `.msi` |
| Windows (便携版) | `CosKit_x.x.x_x64_portable.exe` |

### macOS 用户注意

由于应用未经 Apple 签名，首次打开可能被 macOS Gatekeeper 拦截。请执行以下命令解除限制：

```bash
sudo xattr -r -d com.apple.quarantine /Applications/CosKit.app
```

如果是从 DMG 拖入其他目录，请将路径替换为实际安装位置。

## 配置

CosKit 需要 Gemini API Key 才能使用。有两种配置方式：

### 方式一：应用内设置

点击右上角 ⚙ 设置按钮 → API 配置 → 填写 API Key 和 Base URL → 保存。

### 方式二：.env 文件

在应用同级目录下创建 `.env` 文件：

```env
GEMINI_API_KEY=your_api_key_here
GEMINI_BASE_URL=https://your-proxy-url.com/v1beta/models/gemini-3.1-pro-preview:generateContent
GEMINI_IMAGE_BASE_URL=https://your-proxy-url.com/v1beta/models/gemini-3.1-pro-image-preview:generateContent
```

> 应用内设置优先级高于 .env 文件。

## 从源码构建

### 环境要求

- [Node.js](https://nodejs.org/) >= 20
- [Rust](https://rustup.rs/) >= 1.77

### 构建步骤

```bash
# 安装依赖
npm install

# 开发模式
npx tauri dev

# 构建发行版
npx tauri build
```

产物位于 `src-tauri/target/release/bundle/`。

也可以使用打包脚本：

```bash
# macOS
./build_mac.sh

# Windows
build_win.bat
```

## 技术栈

- **后端**：Rust + Tauri v2
- **前端**：原生 HTML/CSS/JS（无框架）
- **AI**：Gemini API（文本模型 + 图像模型）
- **图像处理**：image-rs
- **HTTP 客户端**：reqwest + tokio

## License

[MIT](LICENSE)
