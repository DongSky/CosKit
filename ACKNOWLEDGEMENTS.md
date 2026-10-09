# 致谢 / Acknowledgements

CosKit 是独立开发的图像后期项目，**v0.1.x 预览版及其基于大模型的对话修图能力先于本次参考项目整合完成**。v1.0 的升级延续了 CosKit 原有的产品方向与对话修图能力，并引入开源项目的实现，补齐基础图像编辑操作。

其中，CosKit 整合了 [PhotoCraft](https://github.com/storytold/photocraft) 的原生编辑器实现，用于补齐图层与文档管理、选区、画笔、滤镜、文件格式及相关基础编辑和自动化能力；所采用的上游版本为 `ec350d64aedd019afc5eda290cc32909bc9bab7d`。感谢 **ArtCraft Team 和所有 PhotoCraft 贡献者**在这些功能和基础设施上的开源贡献。

Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.

PhotoCraft is licensed under MIT OR Apache-2.0. Its original copyright, license and asset notices are preserved. The contributor and model credit tables in About identify upstream contributions; they are not presented as CosKit development statistics.

CosKit v1.0 将原有的对话修图能力与引入的基础编辑能力整合，并进一步开发自主分析与反思回退 Harness、面向 CosKit 工作流的 MCP 工具、CKPipe 参数化编辑与版本管理，以及 Fluent 风格工作台。CosKit 独立维护与发布；所引入和修改的 PhotoCraft 代码继续保留其来源、版权和许可证。CosKit 未表示由 PhotoCraft 或 ArtCraft 官方发布或背书。

同时感谢 **David Revoy** 提供的 2023 绘画笔刷（CC0-1.0），以及 egui、wgpu、Lucide 等开源依赖和素材的贡献者。

发布包的 `licenses/` 包含 `PhotoCraft-MIT.txt`、`PhotoCraft-Apache.txt`、`PhotoCraft-NOTICE.txt`、`ATTRIBUTION.md`、`THIRD_PARTY_NOTICES.md` 及其他素材许可证。源码中的对应文件位于 `native/`，完整来源追溯见 `native/UPSTREAM.md`。上游名称和链接保留在致谢、版权及技术来源记录中；产品帮助与反馈入口指向 CosKit。
