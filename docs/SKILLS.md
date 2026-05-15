# 内置技能（Skills）

内置 10 个修图技能，定义于 `src-tauri/src/skills.rs`。每个技能由 `SkillDef` 描述，规划器从 `skills_catalog_for_planner()` 读取列表后由 LLM 选择组合。

## 数据结构

```rust
pub struct SkillDef {
    pub id: String,                  // 唯一 ID（planner 引用）
    pub name: String,                // 中文显示名
    pub description: String,         // 给 LLM 的简介
    pub category: String,            // 分组类别
    pub prompt_template: String,     // 内含 {{SKILL_PROMPT}} 占位
    pub requires_image_model: bool,  // 是否需要图像模型（当前全部 true）
    pub default_temperature: f64,    // 调用图像模型时的温度
}
```

## 类别 → 技能 速查

| 类别 | id | 名称 | 默认温度 |
|------|----|------|----------|
| skin | skin_smooth | 磨皮美肤 | 0.3 |
| skin | skin_whiten | 美白提亮 | 0.3 |
| body | body_reshape | 身材调整 | 0.3 |
| face | face_adjust | 人脸调整 | 0.3 |
| background | bg_replace | 背景替换 | 0.4 |
| lighting | lighting_adjust | 光线调整 | 0.3 |
| effects | special_fx | 特效添加 | 0.5 |
| color | tone_adjust | 影调调整 | 0.3 |
| color | color_style | 色彩风格化 | 0.4 |
| color | detail_enhance | 细节增强 | 0.3 |

## 专业修图流程参考

planner 的 system prompt 中嵌入了如下流程指引：

```
完整流程（按阶段依序，各阶段内有顺序依赖，跨阶段可并行）：
1. 调色阶段：tone_adjust → color_style → detail_enhance
2. 人像阶段：skin_smooth → skin_whiten → face_adjust → body_reshape
3. 背景阶段：bg_replace → lighting_adjust
4. 特效阶段：special_fx

简易流程（需求简单时优先考虑，2-3 步即可）：
tone_adjust → color_style → skin_smooth/skin_whiten → face_adjust/body_reshape → bg_replace/special_fx
```

## 提示词模板规范

每个 `prompt_template` 包含：

1. 角色：「你是一位专业的人像修图师 / 调色师 / 视觉特效师」
2. 任务陈述
3. `具体要求：{{SKILL_PROMPT}}`
4. 技术要点（每个技能 4-6 条）
5. 「必须返回处理后的图片」

`workflow.rs` 在执行节点时填充：

```rust
let prompt = skill.prompt_template.replace("{{SKILL_PROMPT}}", &pn.skill_prompt);
```

## 技能详细描述

### skin_smooth — 磨皮美肤

去除瑕疵、痘印、毛孔，使皮肤光滑细腻，同时保留皮肤纹理的自然感。

技术要点：
- 去除明显瑕疵（痘印、斑点、毛孔粗大区域）
- 保留皮肤自然纹理，避免过度磨皮导致的塑料感
- 保持五官轮廓清晰
- 不改变人物的其他特征和背景

### skin_whiten — 美白提亮

提亮肤色，使皮肤看起来更白皙通透，同时保持自然的肤色过渡。

### body_reshape — 身材调整

调整人物身材比例（瘦脸、瘦身、长腿等）。

技术要点：调整自然、不变形、保持整体比例协调、背景无液化痕迹、保持衣物纹理自然。

### face_adjust — 人脸调整

脸型微调、五官比例优化、大眼、瘦脸等。保持自然辨识度。

### bg_replace — 背景替换

技术要点（重点）：
- 新背景的透视灭点、地平线高度、镜头焦距感必须与原图人物一致
- 避免人物「悬浮」或比例失调，保持原图拍摄角度不变
- 光照方向、色温、景深需自然融合
- 边缘羽化过渡，头发丝细节保留飘逸感
- 描边效果与新背景色调匹配

### lighting_adjust — 光线调整

补光、调整光线方向、增强光影对比等。

### special_fx — 特效添加

光效、粒子、氛围效果、魔法效果。

技术要点：
- 特效与画面风格协调
- 不遮挡人物主体
- 光照方向与画面一致

### tone_adjust — 影调调整

曝光、对比度、高光、阴影、色阶等明暗层次控制。

### color_style — 色彩风格化

白平衡、色调曲线、混色、颜色分级。

### detail_enhance — 细节增强

锐化、降噪、颗粒、暗角、纹理、清晰度。

## 给规划器的目录文本

`skills_catalog_for_planner()` 输出形如：

```
可用技能列表：

- id: "skin_smooth"
  名称: 磨皮美肤
  说明: 对人像进行磨皮处理，去除瑕疵、痘印、毛孔...
  类别: skin

- id: "skin_whiten"
  ...
```

LLM 必须使用列表中的 `id` 作为 `skill_id`，否则 `validate_plan` 拒绝。

## 扩展新技能

1. 在 `builtin_skills()` 数组追加 `SkillDef`
2. `id` 全局唯一
3. `prompt_template` 必须含 `{{SKILL_PROMPT}}` 占位
4. 选择合适的 `default_temperature`（生成性强的内容如特效用 0.5，精修用 0.3）
5. 视需要更新 planner.rs 中的「专业修图流程参考」段落
6. 添加测试：`workflow::tests` 中的合并 prompt 测试可放入新 skill 来验证
