> beta.5 默认使用新的 [CosKit 创作工作台](fluent-workspace.md)：单列可组合侧栏、固定 AI 输入框和缩略图版本条。以下专业工具与快捷键继续有效；原完整面板布局可从顶部工作空间菜单切换。

# CosKit Studio 1.0.0-beta.4

新增 [MCP 与精确笔刷控制](coskit-mcp.md)；本轮模型、笔刷和照片工作流的实测范围见 [验证报告](cosplay-validation.md)。

CosKit Studio 将完整原生图像编辑器与 AI 对话修图工作流整合在同一窗口。图层、工具组、参数对话框、文档格式及快捷键均保留。基础编辑器来源与贡献者见 `ACKNOWLEDGEMENTS.md`。

## 启动和编辑

安装版打开「CosKit Studio」；便携版解压后运行 `coskit.exe`。使用「文件 → 打开」导入图片、PSD/PSB、`.pcraft` 分层文档或 `.ckpipe` 完整工程。新建文档可选择尺寸、色彩模式和位深。左侧工具长按/右键可访问同组其他工具，顶部选项栏配置当前工具；右侧原有属性、图层、通道、路径面板及窗口菜单均保留。

菜单和工具的完整对照见源码仓库的 `docs/photocraft-feature-matrix.md`；详细操作与当前上游限制见源码仓库中的 `native/docs/`。包括文字、钢笔与形状、智能对象、调整图层、图层效果、选区、通道、液化、Camera Raw、滤镜库、曲线、ICC、软打样、动作和批处理。

## 对话编辑

默认开启自主修图：描述目标后发送即可，模型自行设计步骤、发现并调用原生工具或图像模型、检查效果并回退重试。无需手动选择工作流模块。通过审核后应用完整分层结果，可一次整体撤销。详见 [自主修图架构](autonomous-retouch-harness.md)。关闭自主模式后可使用下述传统工作流。

右侧「CosKit · AI 对话编辑」可收起；窄窗口显示在底部。输入指令，按「发送编辑」即可使用当前文档作为输入。可以选择智能规划、人物精修、背景、特效、合并执行和结果审核，也可以附加参考图。发送请求前不会上传当前图像。

已有选区会限制 AI 结果作用区域。输出以新图层置于整个图层栈上方，原有像素、文字、矢量和智能对象均保留。Ctrl+Z / Ctrl+Shift+Z 使用同一套原生撤销/重做。生成期间可继续编辑；如果源文档或版本变化，自动应用会被拒绝，结果仍可作为新文档打开。

AI 使用 sRGB 8 位代理图，返回图层会转换至原文档色彩空间和位深；它不会使原文档降为 8 位，但也不会凭空生成原生高动态范围信息。原生工具的位深与格式能力不受 AI 代理图限制。AI 单次输入上限 4000 万像素；Indexed/Bitmap/Duotone/Multichannel 文档先在副本上转为 RGB 后再请求 AI。

对话按文档隔离；保存 `.ckpipe` 后，重新打开会恢复项目对话、图层、参数和持久版本。AI 中间候选仍保留本地审计记录；完整工程请保存为 `.ckpipe`，与 Photoshop 交换时另存 PSD/PSB。

## 保存

Ctrl+S /「文件 → 另存为」保存分层文档；「导出」保存合成图像。原生自动恢复遵循编辑器的偏好设置。退出时对未保存文档仍会提示。若需要兼容其他编辑器，注意查看 PSD/格式转换时的上游警告；627 个菜单项接通不意味着完全等同 Photoshop。

## 模型和数据

对话面板「设置」配置文本、图像和审核模型。支持 `openai`、`gemini`、`qwen`，以及兼容 OpenAI 的本地后端。配置留空时读取 `.env`。支持 `OPENAI_BASE_URL`、`OPENAI_API_KEY`、`OPENAI_LLM_MODEL` / `OPENAI_MODEL`、`OPENAI_IMAGE_MODEL`；文件不随发布包分发。

安装版默认数据目录是 `%APPDATA%/CosKit`，沿用已有 AI 配置与会话。便携版包含 `portable.txt`，使用程序旁的 `CosKitData`。`COSKIT_DATA_DIR` 可覆盖 AI 数据目录，`COSKIT_NATIVE_CONFIG_DIR` 可覆盖原生偏好/自动恢复目录。备份这些目录可以保留记录，其中可能含密钥，请勿公开上传。

## 开发与旧版

`npm run dev` 启动完整原生版，开发数据隔离到 `.dev-data`；需要 Rust 1.95+。`npm run build` 构建原生应用与 CLI；Windows 再执行 `pwsh -File scripts/package-native.ps1` 打包。`npm test` 核对源码指纹并运行原生工作区测试。

原 Tauri 版本保留在 `src/` 与 `src-tauri/`，使用 `npm run dev:legacy` / `npm run build:legacy`，可继续访问旧分支会话。它不是当前完整编辑器入口。默认构建不启用旧版可选 GPUPixel / PixelFree SDK。

便携包中的 `coskit-cli.exe` 支持原 PhotoCraft CLI 的 `--help`、命令、转换、批处理和 MCP。程序内核/命令/原生文件仍保留 `photocraft` 名称以兼容既有协议。


完整工程保存为 `.ckpipe`：保存图层、参数化记录、对话与持久分支版本。参见 [CKPipe 格式与操作](ckpipe-project-format.md)。
