# CosKit 1.0.0 — 首个正式版本

**对话与画笔，同一张画布。** 作者：凉月 / Suzutsuki。

CosKit v0.1.x 预览版与对话修图先独立完成。1.0 在既有产品上整合 PhotoCraft 原生编辑器，补齐基础编辑，并带来：

- 图层、蒙版、选区、修复、笔刷、调整、滤镜、文字、矢量与图层效果，支持 PSD/PSB、PCraft 和 HEIF。
- 自主对话修图 Harness：观察照片、规划步骤、调用 MCP 或图像模型、复查结果，必要时回退重试。
- 精确笔刷 MCP，以及保存图层、参数、对话与可回退版本的 `.ckpipe` 工程。
- Fluent 风格工作台、可组合侧栏、缩略图版本条与原尺寸细节对比。
- 共享版本数据、后台保存与恢复、资源预算和大图性能改进。

## 下载与安装

| 平台 | 附件 | 使用方法 |
| --- | --- | --- |
| Windows x64 | `CosKit_1.0.0_x64-setup.exe` | 当前用户安装器 |
| Windows x64 便携版 | `CosKit_1.0.0_x64_portable.zip` | 解压后运行 `coskit.exe` |
| macOS Apple Silicon | `CosKit_1.0.0_macOS_arm64.zip` | 解压，将 `CosKit.app` 拖入“应用程序” |
| macOS Intel | `CosKit_1.0.0_macOS_x64.zip` | 同上 |

ZIP 包包含 CLI；各平台提供 SHA-256 文件。Windows 未商业签名；macOS 使用 ad-hoc 签名，尚未经过 Apple 公证。系统拦截时，仅对确认来源可信的包使用 macOS“隐私与安全性 → 仍要打开”。

打开照片后，先另存为 `.ckpipe`。基础编辑无需 API；AI 修图需在对话面板“选项”中配置支持视觉的文本模型及图像模型。模型服务费用由提供商决定。升级前备份工程。

[中文使用与构建指南](https://github.com/DongSky/CosKit/blob/main/README.md) · [新手指南](https://github.com/DongSky/CosKit/blob/main/docs/getting-started-v1.md) · [MCP 文档](https://github.com/DongSky/CosKit/blob/main/docs/coskit-mcp.md)

## 构建来源与验证范围

三个平台的 release 编译、CLI 冒烟检查、打包和产物上传均已通过，[构建记录](https://github.com/DongSky/CosKit/actions/runs/37967846647)。macOS 包还经过架构检查、签名检查及 ZIP 解压后的 CLI 启动验证。Windows 完成交互验收；macOS CI 不替代所有 GPU、数位板与桌面交互的实机验收。

保留原 `v1.0.0` 标签（`78e246b`）。附件来自构建提交 `30602a3`；相较标签只增加了 CI、macOS 打包脚本、文档/展示图及菜单测试名称修正，应用运行时代码和 Cargo.lock 相同。发行流程校验差异白名单与全部附件哈希。

基础编辑继承参考项目已有格式与行为边界，不声称与 Photoshop 完全兼容。Android 和更多创作功能列入后续计划。完整演示视频与原始工程不随发行包发布。

## 致谢与支持

感谢 PhotoCraft、ArtCraft Team、所有上游贡献者和 David Revoy 的开放实现与笔刷。CosKit v0.1.x 和对话修图并非由这些项目起源。原始版权、许可证和素材归属保留在发行包中。[完整致谢](https://github.com/DongSky/CosKit/blob/main/ACKNOWLEDGEMENTS.md)；赞助方式延续此前版本，见 [README](https://github.com/DongSky/CosKit#readme)。

---

# English

**Conversation and brushwork, on one canvas.** CosKit 1.0.0 is the first stable release, maintained by **Suzutsuki / 凉月**.

Building on the independently developed v0.1.x preview and conversational editing, this release integrates PhotoCraft's native editor for basic editing and adds autonomous retouching, precise MCP brush control, CKPipe projects with editable layers and version history, a Fluent-inspired workspace, and resource/performance improvements.

Download the Windows installer or portable ZIP, or the macOS ZIP matching your processor (arm64 for Apple Silicon, x64 for Intel). Drag `CosKit.app` into Applications on macOS. ZIP packages include the CLI; SHA-256 manifests are provided. Windows binaries are not commercially signed. macOS bundles are ad-hoc signed and are not Apple-notarized; use Privacy & Security → Open Anyway only for a trusted download.

Basic editing works locally without an API. Configure your vision-language and image providers in the conversation panel for AI editing. Save `.ckpipe` to retain layers, conversations and rollback versions; export PNG/JPEG to share results. Back up existing projects before upgrading.

All three release builds, CLI smoke checks and packaging passed [CI](https://github.com/DongSky/CosKit/actions/runs/37967846647). macOS archives also passed signature checks and CLI execution after extraction. CI is not a substitute for hands-on GPU/tablet and desktop validation. The original `v1.0.0` tag stays at `78e246b`; artifacts were built at `30602a3`, which adds only CI, macOS packaging, documentation and menu-test branding fixes. Runtime source and Cargo.lock are unchanged.

Thanks to PhotoCraft, ArtCraft Team, upstream contributors and David Revoy. Original notices and licenses remain bundled. Android and further creative features are planned. See the [English README](https://github.com/DongSky/CosKit/blob/main/README.en.md) for usage, build instructions and the existing sponsorship links.
