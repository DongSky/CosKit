# CosKit

AI 驱动的人像修图桌面应用，支持 **Gemini 兼容 API** 和 **OpenAI 兼容 API** 等模型后端，实现智能修图、背景替换和 Cosplay 特效。

## 演示

[![CosKit 演示视频](http://i0.hdslb.com/bfs/archive/b3a3e0014551168ec4eb678b6d16358695a8bc7e.jpg)](https://www.bilibili.com/video/BV1j97U6VEwE)

## 功能

- **智能修图**：上传照片，输入自然语言指令，AI 自动完成美颜、磨皮、光线优化
- **背景替换**：自动分析场景并推荐匹配背景，保持透视一致性
- **Cosplay 特效**：识别 Cosplay 摄影，自动添加轻度氛围光效和粒子效果
- **多模型支持**：文本模型和图像模型可独立切换 Gemini / OpenAI 兼容 API 提供商
  - Gemini 兼容 API：`gemini-3.1-pro-preview`（文本）、`gemini-3-pro-image-preview`（图像）
  - OpenAI 兼容 API：`gpt-5.5`（文本）、`gpt-image-2`（图像）
- **Provider 配置记忆**：切换提供商时自动保存/恢复各自的 API 参数
- **分支编辑**：基于任意历史节点创建新分支，支持树状编辑历史
- **会话管理**：多会话支持，自动保存，随时切换
- **图片导出**：保持原始分辨率导出，原生系统保存对话框

## 下载

前往 [Releases](https://github.com/DongSky/CosKit/releases) 下载最新版本：

| 平台 | 文件 |
|------|------|
| macOS (Apple Silicon) | `CosKit_x.x.x_aarch64.dmg` |
| macOS (Intel) | `CosKit_x.x.x_x64.dmg` |
| Windows (安装版) | `CosKit_x.x.x_x64-setup.exe` 或 `.msi` |
| Windows (便携版) | `CosKit_x.x.x_x64_portable.exe` |
| Android | `CosKit_x.x.x_universal.apk` |

> iOS 版本需自行从源码构建并安装到设备。

### macOS 用户注意

由于应用未经 Apple 签名，首次打开可能被 macOS Gatekeeper 拦截。请执行以下命令解除限制：

```bash
sudo xattr -r -d com.apple.quarantine /Applications/CosKit.app
```

如果是从 DMG 拖入其他目录，请将路径替换为实际安装位置。

## 配置

CosKit 支持 **Gemini 兼容 API** 和 **OpenAI 兼容 API** 双提供商，需配置对应 API Key。

### 方式一：应用内设置（推荐）

点击右上角 ⚙ 设置按钮 → API 配置：
- 选择文本模型提供商（Gemini 兼容 API / OpenAI 兼容 API）
- 选择图像模型提供商（Gemini 兼容 API / OpenAI 兼容 API）
- 填写对应的 API Key、Base URL、模型名称
- 保存后自动生效

**提示**：切换提供商时，应用会自动保存当前配置并恢复目标提供商的历史配置。

### 方式二：.env 文件

在应用同级目录下创建 `.env` 文件：

```env
# Gemini 兼容 API 配置
GEMINI_API_KEY=your_gemini_key
GEMINI_BASE_URL=https://your-proxy/v1beta/models/gemini-3.1-pro-preview:generateContent
GEMINI_IMAGE_BASE_URL=https://your-proxy/v1beta/models/gemini-3-pro-image-preview:generateContent

# OpenAI 兼容 API 配置
OPENAI_API_KEY=your_openai_key
OPENAI_BASE_URL=https://yunwu.ai/v1
OPENAI_MODEL=gpt-5.5
OPENAI_IMAGE_MODEL=gpt-image-2
```

> 应用内设置优先级高于 .env 文件。

## 从源码构建

CosKit 支持 **macOS、Windows、Android、iOS** 全平台。

### 环境要求

- [Node.js](https://nodejs.org/) >= 20
- [Rust](https://rustup.rs/) >= 1.77

### 桌面端（macOS / Windows）

```bash
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

### Android（本地构建 Release）

**额外要求**：Android Studio、Android SDK（含 NDK）、JDK（Android Studio 自带）

1. 设置签名环境变量（建议写入 `~/.zshrc`）：

```bash
export COSKIT_ANDROID_STORE_PASSWORD=your_store_password
export COSKIT_ANDROID_KEY_ALIAS=your_key_alias
export COSKIT_ANDROID_KEY_PASSWORD=your_key_password   # 若与 store password 相同可省略
```

2. 运行构建脚本：

```bash
./build_android.sh
```

脚本会自动初始化 Android 项目（首次运行）、编译并签名，产物为：
- APK：`src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk`
- AAB：`src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab`

> 需要自备 keystore 文件，通过 `COSKIT_ANDROID_KEYSTORE=/path/to/your.jks` 指定路径。生成方法：`keytool -genkey -v -keystore your.jks -alias your_alias -keyalg RSA -keysize 2048 -validity 10000`

## 技术栈

- **后端**：Rust + Tauri v2
- **前端**：原生 HTML/CSS/JS（无框架）
- **AI**：Gemini 兼容 API + OpenAI 兼容 API（文本模型 + 图像模型）
- **图像处理**：image-rs
- **HTTP 客户端**：reqwest + tokio

## License

[MIT](LICENSE)

## 支持后续开发

CosKit 是一个免费、开源的个人项目。如果它对你有帮助，欢迎通过下面任意方式打赏一杯咖啡 ☕，支持后续迭代：

- **B站充电** — [哇凉月](https://space.bilibili.com/886169)
- **爱发电** — [凉月](https://ifdian.net/a/dongsky)
- **GitHub Sponsors** — [github.com/sponsors/DongSky](https://github.com/sponsors/DongSky)

也欢迎给项目点 Star、提 Issue 或 PR，参与开发同样是最好的支持。
