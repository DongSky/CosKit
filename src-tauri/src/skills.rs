use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub prompt_template: String,
    pub requires_image_model: bool,
    pub default_temperature: f64,
}

pub fn builtin_skills() -> Vec<SkillDef> {
    vec![
        SkillDef {
            id: "skin_smooth".into(),
            name: "磨皮美肤".into(),
            description: "对人像进行磨皮处理，去除瑕疵、痘印、毛孔，使皮肤光滑细腻，同时保留皮肤纹理的自然感".into(),
            category: "skin".into(),
            prompt_template: "你是一位专业的人像修图师。请对这张照片进行磨皮美肤处理。\n\n具体要求：{{SKILL_PROMPT}}\n\n技术要点：\n- 去除明显瑕疵（痘印、斑点、毛孔粗大区域）\n- 保留皮肤自然纹理，避免过度磨皮导致的塑料感\n- 保持五官轮廓清晰\n- 不改变人物的其他特征和背景\n\n必须返回处理后的图片。".into(),
            requires_image_model: true,
            default_temperature: 0.3,
        },
        SkillDef {
            id: "skin_whiten".into(),
            name: "美白提亮".into(),
            description: "提亮肤色，使皮肤看起来更白皙通透，同时保持自然的肤色过渡".into(),
            category: "skin".into(),
            prompt_template: "你是一位专业的人像修图师。请对这张照片进行美白提亮处理。\n\n具体要求：{{SKILL_PROMPT}}\n\n技术要点：\n- 均匀提亮肤色，使皮肤白皙通透\n- 保持肤色过渡自然，避免死白\n- 保留面部立体感和光影层次\n- 不改变人物的其他特征和背景\n\n必须返回处理后的图片。".into(),
            requires_image_model: true,
            default_temperature: 0.3,
        },
        SkillDef {
            id: "body_reshape".into(),
            name: "身材调整".into(),
            description: "调整人物身材比例，包括瘦脸、瘦身、长腿等，使身材更加匀称好看".into(),
            category: "body".into(),
            prompt_template: "你是一位专业的人像修图师。请对这张照片中的人物进行身材调整。\n\n具体要求：{{SKILL_PROMPT}}\n\n技术要点：\n- 调整要自然，不能出现变形、扭曲\n- 保持人物整体比例协调\n- 背景不能出现液化痕迹\n- 保持衣物纹理和褶皱的自然感\n\n必须返回处理后的图片。".into(),
            requires_image_model: true,
            default_temperature: 0.3,
        },
        SkillDef {
            id: "bg_replace".into(),
            name: "背景替换".into(),
            description: "替换照片背景，可以换成任意场景，保持人物与新背景的自然融合".into(),
            category: "background".into(),
            prompt_template: "你是一位专业的人像修图师。请替换这张照片的背景。\n\n具体要求：{{SKILL_PROMPT}}\n\n技术要点：\n- 新背景的透视灭点、地平线高度、镜头焦距感必须与原图人物一致\n- 避免人物'悬浮'或比例失调，保持原图拍摄角度不变\n- 新背景与人物的光照方向、色温、景深需自然融合\n- 边缘过渡柔和无硬切割\n- 保持人物完整，不裁切\n\n必须返回处理后的图片。".into(),
            requires_image_model: true,
            default_temperature: 0.4,
        },
        SkillDef {
            id: "lighting_adjust".into(),
            name: "光线调整".into(),
            description: "调整照片光线，包括补光、调整光线方向、增强光影对比等".into(),
            category: "lighting".into(),
            prompt_template: "你是一位专业的人像修图师。请调整这张照片的光线。\n\n具体要求：{{SKILL_PROMPT}}\n\n技术要点：\n- 光线调整要自然，符合物理规律\n- 保持人物面部光影的立体感\n- 注意光线方向的一致性\n- 不改变人物特征\n\n必须返回处理后的图片。".into(),
            requires_image_model: true,
            default_temperature: 0.3,
        },
        SkillDef {
            id: "special_fx".into(),
            name: "特效添加".into(),
            description: "添加视觉特效，如光效、粒子、氛围效果、魔法效果等".into(),
            category: "effects".into(),
            prompt_template: "你是一位专业的视觉特效师。请为这张照片添加特效。\n\n具体要求：{{SKILL_PROMPT}}\n\n技术要点：\n- 特效要与画面风格协调\n- 不遮挡人物主体\n- 特效的光照方向要与画面一致\n- 保持画面整体美感\n\n必须返回处理后的图片。".into(),
            requires_image_model: true,
            default_temperature: 0.5,
        },
        SkillDef {
            id: "color_grade".into(),
            name: "调色风格化".into(),
            description: "对照片进行调色处理，可以模拟电影色调、日系清新、赛博朋克等各种风格".into(),
            category: "color".into(),
            prompt_template: "你是一位专业的调色师。请对这张照片进行调色风格化处理。\n\n具体要求：{{SKILL_PROMPT}}\n\n技术要点：\n- 色调统一，风格明确\n- 保持人物肤色在风格范围内自然\n- 注意高光和阴影的色彩倾向\n- 整体画面氛围感强\n\n必须返回处理后的图片。".into(),
            requires_image_model: true,
            default_temperature: 0.4,
        },
    ]
}

pub fn skill_registry() -> HashMap<String, SkillDef> {
    builtin_skills()
        .into_iter()
        .map(|s| (s.id.clone(), s))
        .collect()
}

pub fn skills_catalog_for_planner() -> String {
    let mut catalog = String::from("可用技能列表：\n");
    for skill in builtin_skills() {
        catalog.push_str(&format!(
            "\n- id: \"{}\"\n  名称: {}\n  说明: {}\n  类别: {}\n",
            skill.id, skill.name, skill.description, skill.category
        ));
    }
    catalog
}
