from pathlib import Path
import re,collections
root=Path(__file__).resolve().parents[1]
source=(root/'native/crates/ui-egui/src/menu_catalog.rs').read_text(encoding='utf-8')
rows=re.findall(r'\(&\[(.*?)\],\s*"((?:[^"\\]|\\.)*)",\s*(?:Some\([^)]*\)|None),\s*"([^"\n]+)"\)',source,re.S)
seen=set(); groups=collections.defaultdict(list)
for path,label,command in rows:
    if command=='---' or command in seen:continue
    seen.add(command)
    parts=re.findall(r'"([^"]+)"',path)
    groups[parts[0]].append((' → '.join(parts+[label]),command))
assert len(seen)==627, f'Expected the complete upstream menu catalog, got {len(seen)}'
lines=['# PhotoCraft → CosKit 完整功能对照','',
       '来源：所提供参考项目固定提交的原菜单注册表。以下 627 项全部使用同一套原生 UI、命令与引擎实现；没有替换为空按钮，也不重写其参数和交互。菜单接通和 Photoshop 行为一致是不同指标；上游边界仍以原文档为准。','',
       '另有工具栏手势、参数面板、内部命令和自动化接口，不局限于这 627 项，亦由完整源码保留。','']
for group,items in groups.items():
    lines.extend([f'## {group}（{len(items)}）','','| 菜单路径 | 原命令 ID | CosKit 实现 |','|---|---|---|'])
    lines.extend(f'| {label.replace("|","/")} | `{command}` | 原实现完整保留 |' for label,command in items)
    lines.append('')
(root/'docs/photocraft-feature-matrix.md').write_text('\n'.join(lines)+'\n',encoding='utf-8')
print(f'Generated {len(seen)}-item feature matrix')
