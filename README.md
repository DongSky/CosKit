# CosKit

AI 驱动的人像修图桌面应用，支持 **Gemini 兼容 API**、**OpenAI 兼容 API**，以及 **Boogu-Image-Edit 本地模型** 等模型后端，实现智能修图、背景替换和 Cosplay 特效。

通过 [boogu_image_edit_model_parallel_server](https://github.com/DongSky/boogu_image_edit_model_parallel_server) 项目，CosKit 可以调用运行在你自己机器（或局域网服务器）上的 **Boogu-Image-0.1-Edit** 本地模型修图，无需依赖云端 API。详见 [使用 Boogu 本地模型](#使用-boogu-本地模型)。

## News

- **2026-07**：新增 **Boogu-Image-Edit 本地模型** 支持。借助 [boogu_image_edit_model_parallel_server](https://github.com/DongSky/boogu_image_edit_model_parallel_server) 将 Boogu-Image-0.1-Edit 部署为 OpenAI 兼容服务，即可在本地 GPU 上完成全部修图推理，图片数据不出本地。配置方式见 [使用 Boogu 本地模型](#使用-boogu-本地模型)。

## 演示

[![CosKit 演示视频](http://i0.hdslb.com/bfs/archive/b3a3e0014551168ec4eb678b6d16358695a8bc7e.jpg)](https://www.bilibili.com/video/BV1j97U6VEwE)

## 功能

- **智能修图**：上传照片，输入自然语言指令，AI 自动完成美颜、磨皮、光线优化
- **背景替换**：自动分析场景并推荐匹配背景，保持透视一致性
- **Cosplay 特效**：识别 Cosplay 摄影，自动添加轻度氛围光效和粒子效果
- **多模型支持**：文本模型和图像模型可独立切换 Gemini / OpenAI 兼容 API 提供商
  - Gemini 兼容 API：`gemini-3.1-pro-preview`（文本）、`gemini-3-pro-image-preview`（图像）
  - OpenAI 兼容 API：`gpt-5.5`（文本）、`gpt-image-2`（图像）
- **本地模型支持**：借助 [boogu_image_edit_model_parallel_server](https://github.com/DongSky/boogu_image_edit_model_parallel_server) 部署 Boogu-Image-Edit 本地模型，以 OpenAI 兼容协议接入图像修图，数据不出本地
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

## 使用 Boogu 本地模型

CosKit 可以调用运行在本地（或局域网内）的 **Boogu-Image-0.1-Edit** 模型进行修图，全部推理在你自己的 GPU 上完成，图片数据不上传任何云端服务。这套能力由独立项目 [boogu_image_edit_model_parallel_server](https://github.com/DongSky/boogu_image_edit_model_parallel_server) 提供：它把 Boogu 的 FP8 推理管线封装成一个 **OpenAI 兼容的图像编辑服务**，因此 CosKit 只需把「OpenAI 兼容 API」的图像提供商指向这个本地服务即可。

### 工作原理

Boogu 服务暴露标准的 `POST /v1/images/edits` 接口，与 OpenAI 图像编辑协议一致。CosKit 在有输入图片时正是调用该接口，所以配置上把 Base URL 指向 Boogu 服务、模型名填 Boogu 的模型 id，就能无缝切换到本地模型。

> **关于选区（mask）编辑**：Boogu 是指令条件模型，不接受 mask 参数（服务会忽略 `mask` 字段）。但 CosKit 的选区编辑在后端有合成兜底（`composite_with_mask`）——无论模型返回什么，选区外的像素都会用原图逐位还原，因此选区编辑功能在 Boogu 下依然可用，只是模型本身会「看到」整张图。

### 硬件要求

Boogu FP8 管线约需 21 GB 显存。该服务默认采用 **双卡模型并行**：

| GPU 角色 | 组件 | 显存（1024×1024 稳态） |
|---|---|---|
| `--device`（如 `cuda:0`） | transformer + VAE + latents | ~16 GB |
| `--mllm_device`（如 `cuda:1`） | MLLM 指令编码器 | ~10 GB |

一张 24 GB + 一张 16 GB 的组合（例如 RTX 4090 + RTX 5060 Ti）即可在 1024 级分辨率下运行，无需 offload。单卡若有 24 GB 显存，也可将两个 device 参数都设为同一张卡。

### 第一步：部署 Boogu 服务

以下命令均在 **运行 GPU 的机器**（Linux/Windows）上执行，而非 CosKit 桌面端。完整说明见 [项目仓库](https://github.com/DongSky/boogu_image_edit_model_parallel_server)。

1. 创建环境并安装依赖：

```bash
conda create -n boogu python=3.10 -y
conda activate boogu

# PyTorch（按你的 CUDA 版本选择 index）
pip install --index-url https://download.pytorch.org/whl/cu128 torch==2.11.0 torchvision==0.26.0 torchaudio==2.11.0
pip install "torchao>=0.15,<0.18"

# 项目依赖
pip install "diffusers[torch]>=0.35.2,<0.39" "transformers[torch]>=4.57.3,<6" \
            "accelerate>=1.0" "kernels>=0.14,<0.15" "cache-dit>=1.3,<2" \
            "einops>=0.7" "scipy>=1.11" "webdataset>=1.0,<2" \
            "python-dotenv>=1.0,<2" "omegaconf>=2.3,<3"

# Triton（FP8 fallback kernel）
pip install triton              # Linux
pip install triton-windows      # Windows

# API 服务依赖
pip install "fastapi>=0.110" "uvicorn[standard]>=0.27" "python-multipart>=0.0.9"

# 以可编辑方式安装 boogu 包（需先 git clone 本项目并进入目录）
pip install -e .
```

2. 下载 FP8 模型权重（约 21 GB）：

```bash
mkdir -p models
git lfs install
git clone https://huggingface.co/Boogu/Boogu-Image-0.1-Edit-fp8 models/Boogu-Image-0.1-Edit-fp8
```

3. 启动 OpenAI 兼容服务：

```bash
bash run_api_server.sh
```

默认监听 `0.0.0.0:8000`，模型 id 为 `boogu-image-edit-fp8`，`cuda:0` 跑 transformer/VAE、`cuda:1` 跑 MLLM。可用环境变量覆盖：

```bash
HOST=0.0.0.0 PORT=8000 \
DEVICE=cuda:0 MLLM_DEVICE=cuda:1 \
PRETRAINED_PATH=models/Boogu-Image-0.1-Edit-fp8 \
bash run_api_server.sh
```

4. 验证服务就绪：

```bash
# 权重加载完成后返回 {"status":"ok","ready":true,...}
curl http://127.0.0.1:8000/health

# 快速修图测试（api_key 可为任意非空字符串，服务不校验）
curl -X POST http://127.0.0.1:8000/v1/images/edits \
    -F "model=boogu-image-edit-fp8" \
    -F "image=@input_image_examples/03.jpg" \
    -F "prompt=Replace the background with a sandy beach." \
    -F "response_format=url"
```

> 服务在启动时一次性加载模型；推理通过 `asyncio.Lock` 串行执行，`/health`、`/v1/models` 在长任务运行时仍可响应。并发的修图请求会排队，同一时刻只处理一个。

### 第二步：在 CosKit 中接入

打开 CosKit → 右上角 ⚙ 设置 → API 配置，将 **图像模型提供商** 设为「OpenAI 兼容 API」，并填写：

| 字段 | 值 |
|------|------|
| Base URL | `http://<Boogu 主机 IP>:8000/v1`（本机为 `http://127.0.0.1:8000/v1`） |
| 图像模型名 | `boogu-image-edit-fp8`（须与服务的模型 id 一致） |
| API Key | 任意非空字符串，如 `sk-local`（服务不校验，但字段不能为空） |

保存后即生效。文本模型提供商可继续用 Gemini / OpenAI 云端 API（负责理解指令、规划流程），图像修图则走本地 Boogu 模型——CosKit 支持文本与图像提供商独立配置。

若用 `.env` 文件配置，对应字段为：

```env
OPENAI_API_KEY=sk-local
OPENAI_IMAGE_BASE_URL=http://127.0.0.1:8000/v1
OPENAI_IMAGE_MODEL=boogu-image-edit-fp8
```

> 若 Boogu 部署在另一台机器上，把 `127.0.0.1` 换成该机器的局域网 IP，并确保启动服务时 `HOST=0.0.0.0`、防火墙放行 `8000` 端口。

### 调优提示

Boogu 服务的 `/v1/images/edits` 除标准字段外，还支持通过表单额外参数微调（对应 `inference.py` 的 CLI 参数，默认值来自 argparse）：

- `num_inference_steps`（默认 50）——步数越多越精细，也越慢
- `text_guidance_scale`（默认 4.0）——指令遵循强度
- `image_guidance_scale`（默认 1.0）——需要保持原图身份/五官一致时调到 ~1.5
- `seed`（默认 0）
- `negative_instruction`——默认使用 Boogu 标准负面提示模板

这些属于 Boogu 扩展参数；CosKit 当前通过标准 OpenAI 字段调用，如需定制上述参数可直接调整服务端默认值。

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
- **AI**：Gemini 兼容 API + OpenAI 兼容 API（文本模型 + 图像模型）+ Boogu-Image-Edit 本地模型
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
