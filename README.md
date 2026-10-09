# CosKit 1.0.0

**对话与画笔，同一张画布。** 开源 Cosplay 照片后期工作台。

简体中文 · [English](README.en.md)

这是 CosKit 的**第一个正式版本**，由 **凉月 / Suzutsuki** 独立维护。CosKit v0.1.x 预览版及对话修图能力先独立完成；v1.0 在既有产品方向上整合 PhotoCraft 的原生编辑器，补齐基础图像编辑，并加入自主修图 Harness、MCP、CKPipe 工程和全新工作台。

[发行页](https://github.com/DongSky/CosKit/releases) · [新手指南](docs/getting-started-v1.md) · [版本说明](docs/release-1.0.0.md) · [MCP](docs/coskit-mcp.md) · [致谢](ACKNOWLEDGEMENTS.md)

## 1.0.0 带来了什么

| 能力 | 用来做什么 |
| --- | --- |
| 完整原生基础编辑 | 图层、蒙版、选区、修复、画笔、调整、滤镜、文字、矢量、智能对象和图层效果；兼容 PSD/PSB、PCraft，支持 8/16/32 位与 ICC 色彩管理。 |
| 自主对话修图 | 只描述目标。模型自行观察、设计步骤、发现并调用 MCP 工具或图像模型，复查效果；失败时回退并调整方案。 |
| 全图 + 原尺寸局部验收 | 模型可自行申请发丝、眼睛、衣纹等区域，保留累计观察；审核未通过的候选不提交。 |
| 精确笔刷与 MCP | stdio MCP、原生桌面桥接、命令发现、精确轨迹/压力/流量控制；内置 CosKit 笔刷与 David Revoy CC0 笔刷适配。 |
| CKPipe 工程 | `.ckpipe` 同时保存可编辑图层、参数记录、对话、审核过程与分支版本；参数可视化修改后在后台重放。 |
| Fluent 创作工作台 | 中性深色画布、可组合侧栏、固定对话入口、缩略图版本条；同步原尺寸比较、200% 放大和差异图。 |
| 资源与效率改进 | 跨版本共享压缩数据块、后台保存/恢复、共享资源预算、大图移动优化；低风险 AI 操作可组成事务，共享审核和观察缓存。 |

基础编辑完整保留所引入参考版本的功能，不等于实现 Photoshop 的所有功能。菜单覆盖与具体行为兼容是不同指标；格式边界见 [原生编辑器说明](native/docs/roadmap.md)。AI 全图/局部预览使用显示代理，原生工程的图层、位深和色彩信息保留；局部审核是采样，不保证检查了所有像素。

## 演示与教程

真实工作台截图：在同一画布中使用基础编辑、AI 对话修图和缩略图版本条。

![CosKit 1.0.0 工作台：花间人像、AI 修图对话与版本缩略图](docs/images/coskit-v1-workspace.jpg)

修图前后对比（左为原片，右为演示成片）：调整通透感、色彩与柔光，修图过程中画布保持 **1620 × 1080**；下图为缩放后的展示图。

![花间人像修图对比：左侧原片，右侧调色与柔光后的成片](docs/images/coskit-v1-before-after.jpg)

本节公开两张已清除元数据的展示图；完整功能介绍与新手教程视频、原始照片和工程素材单独本地交付。宣传页源码在 [website/coskit](website/coskit)，接入 `prts.si/coskit/` 的说明见 [website/README.md](website/README.md)。页面上线前可先本地预览。

[v0.1.x 历史演示（B 站）](https://www.bilibili.com/video/BV1j97U6VEwE)

## 安装

本次正式版验证与打包平台为 **Windows x64**。其他桌面平台保留源码构建路径，尚未完成本次发行验收；Android 在 TODO 中。

在 [Releases](https://github.com/DongSky/CosKit/releases) 查看可用附件；v1.0.0 的发布文件名为：

- `CosKit_1.0.0_x64-setup.exe`：当前用户安装版，无需管理员权限。
- `CosKit_1.0.0_x64_portable.zip`：解压到可写目录后运行 `coskit.exe`，同时包含 `coskit-cli.exe`。
- `SHA256SUMS-1.0.0.txt`：校验文件。附件以发行页实际上传内容为准，源码和标签不等于二进制已上传。

安装版设置默认在 `%APPDATA%/CosKit`，便携版在程序旁的 `CosKitData`。程序未进行商业代码签名。升级前建议保留已有工程备份。

## 五分钟上手

1. 打开照片（`Ctrl+O`），先“另存为” `.ckpipe`，在底部版本条记录基准。
2. 基础编辑可直接使用，无需 API。AI 修图需在对话面板的“选项”中配置文本/视觉模型与图像模型。
3. 输入目标，例如：“让花间人像更通透，保留肤质、发丝和白衣层次，不改变构图与人物身份。”无需指定工作流。
4. 查看结果与过程记录，用原尺寸对比检查细节。必要时恢复版本、修改参数或在新图层上用笔刷收尾。
5. 保存 `.ckpipe` 保留完整工程；导出 PNG/JPEG 分享成片；PSD/PSB 用于分层交换，不承载 CKPipe 的全部对话与版本历史。

完整说明见 [新手指南](docs/getting-started-v1.md)。AI 会把所需图像与目标发送到你配置的提供商；基础编辑在本地运行。API 费用、可用模型与质量由提供商决定。

### 模型配置

支持 Gemini 兼容与 OpenAI 兼容接口，文本和图像模型分别配置。自主 Harness 的文本模型需要图像理解能力。可在应用中填写 Base URL、API Key、模型名，或使用本地 `.env`：

```dotenv
OPENAI_BASE_URL=https://your-provider.example/v1
OPENAI_API_KEY=your_api_key
OPENAI_LLM_MODEL=your_vision_language_model
OPENAI_IMAGE_MODEL=your_image_edit_model
```

`.env` 只在本机保存，不提交到 Git。开发模式读取项目或父目录 `.env`，配置与恢复数据隔离到 `.dev-data`；正式版可从程序/数据目录读取。应用内已有配置优先。Gemini 配置和 Boogu 本地服务的进阶说明见 [历史配置资料](docs/legacy-v0.1.md)；其中移动端与可选 GPU 美颜部分属于旧入口。

## 从源码构建

需要 Rust **1.95+**、Node.js **20+**；源码校验与部分工具需要 Python 3。Windows 安装 Visual Studio C++ Build Tools 与 Windows SDK。Windows 安装包另需 PowerShell 7 和 NSIS。

```sh
git clone https://github.com/DongSky/CosKit.git
cd CosKit
npm run dev
# 构建原生编辑器与 CLI（默认启用 HEIF）
npm run build
```

原生入口仅使用 Node 启动构建脚本，不依赖 npm 包；使用旧 Tauri 入口时才需先运行 `npm ci`。默认产物在 `native/target/release/`，内部程序名保留 `photocraft` / `photocraft-cli`，打包后为 `coskit` / `coskit-cli`。

```powershell
pwsh -File scripts/package-native.ps1
# 使用自定义 Cargo 产物目录时：
pwsh -File scripts/package-native.ps1 -BuildDir native/target-release/release
```

输出安装包、便携 ZIP 和 SHA-256 校验文件到 `dist/`。脚本使用打包白名单，不收集 `.env`、测试照片或私人配置。NSIS 未在 PATH 中时可传 `-Makensis` 指定工具路径。

```sh
npm test
cargo test --manifest-path native/Cargo.toml -p photocraft
npm run check:native
```

`npm test` 包含上游源码完整性校验和原生库测试；桌面应用测试单独执行。旧 Tauri 代码保留供历史兼容与开发参考：`npm run dev:legacy` / `npm run build:legacy`，不属于本次原生安装包。

## 自动化与工程文档

- [MCP 接入、权限范围与精确笔刷](docs/coskit-mcp.md)
- [自主 Harness：决策、复查和回退](docs/autonomous-retouch-harness.md)
- [CKPipe：图层、参数、对话和版本](docs/ckpipe-project-format.md)
- [P0 性能记录](docs/p0-performance.md) · [P1 验证报告](docs/p1-harness-and-review.md)
- [来源追溯](native/UPSTREAM.md) · [第三方许可证](THIRD_PARTY_NOTICES.md)

## TODO

- [ ] **Android**：适配触控、小屏布局、资源预算与移动端工程流转。
- [ ] **更多新功能**：继续完善修图工作流、笔刷、局部验收、格式兼容和创作体验。

## 致谢

感谢 **PhotoCraft、ArtCraft Team 和所有贡献者**开放原生编辑器实现，为 CosKit 补齐基础图像编辑提供支持。CosKit v0.1.x 与对话修图并非由该项目起源；本次整合沿用了 CosKit 原有产品方向，并独立维护新增功能。

感谢 **David Revoy** 的 CC0 绘画笔刷，以及 **egui、wgpu、Rust 生态、Boogu-Image** 和其他开源依赖。也感谢 **Tauri** 对早期 CosKit 的支持。完整来源、版权和许可见 [ACKNOWLEDGEMENTS.md](ACKNOWLEDGEMENTS.md) 与 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

## 支持后续开发

CosKit 是免费、开源的个人项目。如果它对你有帮助，欢迎支持 **凉月 / Suzutsuki** 的后续迭代：

- **B 站充电** — [凉月](https://space.bilibili.com/886169)
- **爱发电** — [凉月](https://ifdian.net/a/dongsky)
- **GitHub Sponsors** — [github.com/sponsors/DongSky](https://github.com/sponsors/DongSky)

也欢迎 Star、Issue 和 PR，参与开发同样是支持。

## License

CosKit 自有代码采用 [MIT](LICENSE)。所整合的 PhotoCraft 代码保留 MIT OR Apache-2.0 许可，第三方素材遵从各自许可证。演示照片与视频单独交付，不随代码许可授权。
