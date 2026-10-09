"""Exercise the autonomous harness via desktop MCP; no workflow is provided by this driver."""
import argparse
import json
import sys
from coskit_mcp_client import MCP, ROOT

sys.stdout.reconfigure(encoding="utf-8")
parser = argparse.ArgumentParser()
parser.add_argument("stage", choices=["start", "status", "finish"])
parser.add_argument("--native-only", action="store_true")
args = parser.parse_args()
out = ROOT / "test_output/harness"
out.mkdir(exist_ok=True)

with MCP(bridge=50507) as m:
    if args.stage == "start":
        m.tool("doc_open", {"path": "harness/source.pcraft"})
        before = m.tool("doc_inspect")
        assert (before["width"], before["height"]) == (5472, 3648)
        m.tool("doc_save", {"path": "harness/before.png"})
        m.tool("ai_configure", {"harness_enabled": True, "harness_max_steps": 16,
                                "harness_max_images": 0 if args.native_only else 2, "harness_max_rollbacks": 3})
        prompt = "请让这张 Cosplay 照片的肤色自然一点，保留冷色科幻氛围和皮肤细节。保留人物身份、妆容、服装、姿势、背景布局及画布尺寸，不要增加特效。你自行判断需要处理的问题并完成修图。"
        if args.native_only:
            prompt = "请把整张照片的曝光稍微压低一点，保持暗部层次，保留人物、服装、背景及所有细节。由你自行判断具体处理步骤。"
        m.tool("ai_edit", {"prompt": prompt})
        (out / "live-before.json").write_text(json.dumps({"prompt": prompt, "document": before}, ensure_ascii=False, indent=2), encoding="utf-8")
        print("Submitted goal only; source is 5472 x 3648. No tool choices or steps supplied.")
    elif args.stage == "status":
        status = m.tool("ai_status")
        print(json.dumps({k: status.get(k) for k in ["busy", "outcome", "status", "error", "result"]}, ensure_ascii=False))
        print(json.dumps(status.get("harness", [])[-2:], ensure_ascii=False))
    else:
        status = m.tool("ai_status")
        assert not status["busy"], status["status"]
        (out / "live-status.json").write_text(json.dumps(status, ensure_ascii=False, indent=2), encoding="utf-8")
        assert status["outcome"] in ["applied", "unchanged"], status.get("error")
        after = m.tool("doc_inspect")
        assert (after["width"], after["height"]) == (5472, 3648)
        for ext in ["ckpipe", "pcraft", "psd", "png", "jpg"]:
            m.tool("doc_save", {"path": "harness/autonomous-retouch." + ext})
        m.tool("ui_set", {"fields": {"fit": True}})
        m.tool("ui_screenshot", {"max_side": 1600}, image_path=out / "workspace.png")
        if status["outcome"] == "applied":
            from PIL import Image, ImageChops
            m.command("edit.undo")
            m.tool("doc_save", {"path": "harness/undone.png"})
            a = Image.open(out / "before.png").convert("RGB")
            b = Image.open(out / "undone.png").convert("RGB")
            assert ImageChops.difference(a, b).getbbox() is None
            m.command("edit.redo")
            m.tool("doc_save", {"path": "harness/redone.png"})
            a = Image.open(out / "autonomous-retouch.png").convert("RGB")
            b = Image.open(out / "redone.png").convert("RGB")
            assert ImageChops.difference(a, b).getbbox() is None
        (out / "live-after.json").write_text(json.dumps(after, ensure_ascii=False, indent=2), encoding="utf-8")
        print("Verified native result, source dimensions, complete undo and redo.")
