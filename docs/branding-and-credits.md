# CosKit 1.0.0-beta.6：品牌与致谢

产品欢迎页、帮助菜单、关于、经典布局标题栏、系统信息、打印/格式提示、原生命令行与 Web 加载页统一为 CosKit。帮助、发布与反馈链接指向 CosKit；上游 Discord、ArtCraft 推广与个人主页联系方式入口移除。

关于窗口新增独立致谢页，明确基础编辑器派生自 PhotoCraft，保留 ArtCraft Team、PhotoCraft contributors 的版权、MIT OR Apache-2.0、上游源码链接和 David Revoy 笔刷致谢。上游 66 位贡献者及模型贡献记录保留，并标明来源。完整 ACKNOWLEDGEMENTS.md 随安装版和便携版发布。

原许可证、NOTICE 和贡献者数据与参考项目逐字核对通过；原素材归属全文保留。内部 crate/命令/偏好标识、.pcraft 格式及源码历史保持兼容。参考项目未修改。源码门禁：1,047 个原始指纹保持冻结，968 个文件原样保留，79 个覆盖文件附用途说明；原编辑命令注册未删除。

验证（2026-10-09）：

- 修改涉及的桌面、CLI、UI、引擎、codec、IO 回归：3,216 passed，27 ignored，0 failed；恶意参数测试另 1 passed。
- Clippy all-targets -D warnings、29 crate 分层、627/627 菜单覆盖、WASM L0–L6（含 HEIF）通过。
- 五张离屏真实 UI 渲染人工核对：欢迎页、关于、1024×720 中文致谢、上游贡献者、1024×720 经典布局。未加载或改动照片。
- 最终 release CLI 版本/帮助已验证；安装包与便携包清单、二进制与 ZIP 字节一致性、私密凭据排除、SHA256 校验通过。

本次没有改动修图算法、画布尺寸或工程内容。开发验证日志和截图保存在本地 test_output/，不随发行包分发。
