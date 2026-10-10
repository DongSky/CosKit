# CosKit / prts.si 子页面

独立静态页面，设计参考 https://prts.si/ 在 2026-10-10 的公开页面：近黑绿背景、荧光绿重点、等宽字体、终端信息块、细线边框。页面自己维护 CSS，无构建步骤、第三方脚本、统计服务、外部字体或后端依赖。

## 本地预览

```sh
python -m http.server 8080 --bind 127.0.0.1 --directory website
```

打开 http://127.0.0.1:8080/coskit/ 。源码中的 `media/` 被 Git 忽略：按作者要求，照片、截图、视频和工程素材单独在本地交付，不推送到仓库。拿到素材包后把其中的 `media/` 放进 `website/coskit/`。没有素材时文字和导航仍可查看，正式部署应先检查文件完整性。

## 接入 prts.si

1. 将 `coskit/` 复制到站点发布根目录，使地址为 `/coskit/`。保留相对资源路径。
2. 将单独交付、已经移除 EXIF 的 `media/` 复制到 `/coskit/media/`，不复制 `originals/`、原始录屏、CKPipe 或私有验收记录。
3. 在首页站点结构加入一个 `/coskit/` 项目入口，导航中按需加入 `coskit`。参考片段：

```html
<a href="/coskit/">
  <span>/coskit</span>
  <h3>CosKit 创作工作台</h3>
  <p>对话与画笔，同一张画布。开源 Cosplay 照片后期。</p>
  <span>● v1.0.0</span>
</a>
```

4. 检查下载入口。当前指向已发布的 v1.0.0，包含 Windows 安装版、便携版和 macOS Apple Silicon / Intel ZIP。
5. 推荐响应头 `X-Content-Type-Options: nosniff`，HTML 使用 `text/html; charset=utf-8`，VTT 使用 `text/vtt; charset=utf-8`；视频支持 Range 请求。MP4 已准备 faststart。
6. 检查桌面/手机布局、键盘操作比较滑杆、视频播放、字幕和赞助链接。

## 素材清单

`workspace.webp`, `before.webp`, `after.webp`, `ai-review.webp`, `brush.webp`, `parameters.webp`, `detail.webp`, `tutorial-poster.jpg`, `social-card.jpg`, `coskit-tutorial.mp4`, `tutorial.zh.vtt`, `tutorial.en.vtt`。

页面注明真实窗口由 MCP 驱动、等待片段加速、旁白为合成语音。原图与修图结果属于用户提供的独立演示素材，不随代码许可证重新授权。

## 重建教程

在仓库根目录准备 Python 环境，安装 Pillow；另建语音 venv 安装 `edge-tts`，并安装 FFmpeg。在线语音服务只接收公开解说文本，不上传照片或工程。默认声线为 `zh-CN-XiaoxiaoNeural`，语速降低 4%；不再默认使用 Windows 离线语音。

```sh
python scripts/render-release-media.py --media /path/to/recorded-demo --advanced --voice-python /path/to/voice-venv/python --voice zh-CN-XiaoxiaoNeural
```

素材目录须包含录屏脚本产出的截图、导出图、`recordings/timeline.json` 和真实窗口录屏。渲染器生成中文神经旁白、中英文字幕、1080p 教程及网页素材，旁白指纹缓存允许中断后继续。服务失败会明确报错，不静默换回旧声线。`video/voice-manifest.json` 记录声线及合成来源。公开解说不含私人路径与模型配置。

角色创作依据见 [神里绫华演示参考](../docs/demo-ayaka-brief.md)。
