# 桌面持续集成 / Desktop CI

两个工作流均覆盖 Windows x64、macOS Apple Silicon (`macos-15`) 和 Intel (`macos-15-intel`)，任意平台失败都会使对应工作流失败，不使用 `continue-on-error`。三平台独立执行；较新的同分支提交会取消旧任务，避免重复编译。

## 构建与产物

[Build CosKit Native Studio](../.github/workflows/build.yml) 在 main 提交、PR、`v*` 标签和手动触发时执行：

1. 校验引入的基础编辑器源码完整性。
2. 使用 Cargo.lock 构建 release 桌面程序与 CLI，启用 HEIF。
3. 运行 CLI 版本与 JSON 命令清单冒烟检查。
4. Windows 打包 NSIS 安装器和便携 ZIP；macOS 打包 `CosKit.app` 与 CLI，验证架构、Info.plist、签名，以及从 ZIP 解压后的签名和 CLI 启动。
5. 上传各架构的包与 SHA-256，缺少产物即失败；附件保留 14 天。

macOS 应用包含 CKPipe、PCraft、PSD 等文件关联、CosKit 图标和第三方许可证。签名是无需私钥的 ad-hoc 签名，尚无 Developer ID 签名或 Apple 公证。CI 不自动上传 GitHub Release，也不部署网站。

## 测试

[Verify CosKit Studio](../.github/workflows/check.yml) 在 push、PR 和手动触发时执行：前端/宣传页脚本语法、发布脱敏检查、源码完整性、全部原生库测试、引擎/UI/桌面应用测试（含 HEIF），以及显式启用的异常参数 panic 检查。macOS 应用集成测试包含真实 AppKit 菜单测试。

构建和测试分开缓存，固定最多 3 个 Cargo 编译任务，测试不生成调试符号，以控制 runner 的磁盘与内存占用。工作流只申请读取仓库权限，不需要模型 API 或签名密钥。

CI 成功证明该提交在指定 runner 上完成上述检查，不代表所有 macOS 版本、GPU、数位板或在线模型均已验收。AI 供应商联调、实际画布交互、笔压与系统安装体验仍需实机验证。GitHub 分支保护需另行设置才能强制阻止合并，工作流本身不等于已开启分支保护。

## English

Both workflows cover Windows x64, macOS Apple Silicon and Intel. Builds run for main pushes, pull requests, version tags and manual dispatch; tests run for pushes, pull requests and manual dispatch. Every platform is required for its workflow to succeed. New runs supersede older runs on the same ref.

Release builds include the desktop editor, CLI and HEIF support, followed by CLI smoke checks and packaging. Windows produces an installer and portable ZIP. macOS produces an ad-hoc-signed app/CLI ZIP, verifies architecture and metadata, then verifies signatures and CLI execution after extraction. Artifacts include SHA-256 manifests and expire after 14 days.

Tests cover syntax, publication auditing, upstream integrity, native libraries, engine/UI/app tests and adversarial command parameters. macOS includes the AppKit menu integration test. No model credentials are needed. CI does not automatically publish Releases, notarize macOS builds or enable branch protection. Successful CI does not replace hands-on testing of graphics, tablets, installation or online AI providers.
