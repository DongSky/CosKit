use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::gemini_client;
use crate::models::ReferenceImage;
use crate::planner::WorkflowPlan;
use crate::skills;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResult {
    pub overall_score: f64,
    pub dimensions: ReviewDimensions,
    pub feedback: String,
    #[serde(default)]
    pub suggestions: Vec<String>,
    #[serde(skip_deserializing)]
    pub pass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewDimensions {
    pub aesthetic_quality: f64,
    pub requirement_match: f64,
    pub technical_quality: f64,
    pub consistency: f64,
}

#[derive(Debug, Clone)]
pub struct ReviewConfig {
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub api_key: String,
}

impl ReviewConfig {
    pub fn is_configured(&self) -> bool {
        !self.provider.is_empty() && !self.api_key.is_empty() && !self.model.is_empty()
    }
}

fn build_review_prompt(
    user_prompt: &str,
    plan: &WorkflowPlan,
    references: &[ReferenceImage],
) -> String {
    let registry = skills::skill_registry();
    let steps_desc: Vec<String> = plan
        .nodes
        .iter()
        .map(|pn| {
            let name = registry
                .get(&pn.skill_id)
                .map(|s| s.name.as_str())
                .unwrap_or("未知");
            format!("  - {}: {} — {}", pn.node_id, name, pn.skill_prompt)
        })
        .collect();

    let ref_hint = if references.is_empty() {
        String::new()
    } else {
        let descs: Vec<String> = references
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.description.trim().is_empty())
            .map(|(i, r)| format!("  - 参考图 {}：{}", i + 1, r.description.trim()))
            .collect();
        if descs.is_empty() {
            "\n用户提供了参考图像。".to_string()
        } else {
            format!("\n用户提供了参考图像：\n{}", descs.join("\n"))
        }
    };

    format!(
        r#"你是一位专业的摄影后期审核专家。请评估以下修图结果的质量。

## 输入信息
- 原始照片（第一张图片）
- 修图结果（第二张图片）
- 用户需求：{user_prompt}
- 执行的修图步骤：
{steps}
{ref_hint}

## 评估维度
1. **aesthetic_quality**（美学质量）：构图、色彩和谐、光影、整体视觉效果
2. **requirement_match**（需求匹配）：修图结果是否准确满足用户的具体需求
3. **technical_quality**（技术质量）：是否存在伪影、色彩断层、边缘不自然、变形等技术问题
4. **consistency**（一致性）：人物特征是否保持一致，风格是否统一，与参考图的匹配度

## 输出格式
严格输出以下 JSON，不要输出其他内容：
```json
{{
  "overall_score": 8.5,
  "dimensions": {{
    "aesthetic_quality": 8.0,
    "requirement_match": 9.0,
    "technical_quality": 8.5,
    "consistency": 8.0
  }},
  "feedback": "整体修图效果良好...",
  "suggestions": [
    "建议...",
    "建议..."
  ]
}}
```"#,
        user_prompt = user_prompt,
        steps = steps_desc.join("\n"),
        ref_hint = ref_hint,
    )
}

fn build_review_contents(
    prompt_text: &str,
    original_b64: &str,
    result_b64: &str,
    references: &[ReferenceImage],
) -> serde_json::Value {
    let mut parts = vec![
        json!({"text": prompt_text}),
        json!({"text": "\n原始照片："}),
        json!({"inline_data": {"mime_type": "image/jpeg", "data": original_b64}}),
        json!({"text": "\n修图结果："}),
        json!({"inline_data": {"mime_type": "image/jpeg", "data": result_b64}}),
    ];

    for (i, ref_img) in references.iter().enumerate() {
        let desc = if ref_img.description.trim().is_empty() {
            format!("\n参考图 {}：", i + 1)
        } else {
            format!("\n参考图 {}（{}）：", i + 1, ref_img.description.trim())
        };
        parts.push(json!({"text": desc}));
        parts.push(json!({"inline_data": {"mime_type": "image/jpeg", "data": ref_img.data}}));
    }

    json!([{"parts": parts}])
}

pub async fn review_image(
    config: &ReviewConfig,
    original_b64: &str,
    result_b64: &str,
    user_prompt: &str,
    plan: &WorkflowPlan,
    references: &[ReferenceImage],
    threshold: f64,
) -> Result<ReviewResult, String> {
    if !config.is_configured() {
        return Err("Review Agent 未配置（缺少 provider/model/api_key）".to_string());
    }

    let prompt_text = build_review_prompt(user_prompt, plan, references);
    let contents = build_review_contents(&prompt_text, original_b64, result_b64, references);

    let resp = gemini_client::call_text_with_provider(
        &config.provider,
        &config.base_url,
        &config.api_key,
        &config.model,
        contents,
        0.2,
        3,
    )
    .await?;

    let text = gemini_client::extract_text(&resp);
    if text.is_empty() {
        return Err("审核模型未返回内容".to_string());
    }

    let json_val = gemini_client::parse_json(&text)?;
    let mut review: ReviewResult =
        serde_json::from_value(json_val).map_err(|e| format!("解析审核结果失败: {e}"))?;

    review.pass = review.overall_score >= threshold;

    eprintln!(
        "[CosKit] review: score={:.1}, pass={}, feedback={}",
        review.overall_score,
        review.pass,
        review.feedback.chars().take(60).collect::<String>()
    );

    Ok(review)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::{PlannedNode, WorkflowPlan};

    #[test]
    fn review_result_serde_roundtrip() {
        let review = ReviewResult {
            overall_score: 8.5,
            dimensions: ReviewDimensions {
                aesthetic_quality: 8.0,
                requirement_match: 9.0,
                technical_quality: 8.5,
                consistency: 8.0,
            },
            feedback: "整体效果良好".to_string(),
            suggestions: vec!["建议加强光照一致性".to_string()],
            pass: true,
        };
        let json = serde_json::to_value(&review).unwrap();
        assert_eq!(json["overall_score"], 8.5);
        assert_eq!(json["dimensions"]["aesthetic_quality"], 8.0);
    }

    #[test]
    fn review_prompt_contains_steps() {
        let plan = WorkflowPlan {
            reasoning: "test".into(),
            nodes: vec![
                PlannedNode {
                    node_id: "step_1".into(),
                    skill_id: "bg_replace".into(),
                    skill_prompt: "换成夜景".into(),
                    depends_on: vec![],
                },
                PlannedNode {
                    node_id: "step_2".into(),
                    skill_id: "special_fx".into(),
                    skill_prompt: "添加光效".into(),
                    depends_on: vec!["step_1".into()],
                },
            ],
        };
        let prompt = build_review_prompt("请修图", &plan, &[]);
        assert!(prompt.contains("bg_replace") || prompt.contains("背景替换"));
        assert!(prompt.contains("换成夜景"));
        assert!(prompt.contains("添加光效"));
        assert!(prompt.contains("请修图"));
    }

    #[test]
    fn review_config_unconfigured() {
        let config = ReviewConfig {
            provider: String::new(),
            model: String::new(),
            base_url: String::new(),
            api_key: String::new(),
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn review_config_configured() {
        let config = ReviewConfig {
            provider: "openai".into(),
            model: "gpt-4".into(),
            base_url: "https://api.openai.com/v1".into(),
            api_key: "sk-test".into(),
        };
        assert!(config.is_configured());
    }
}
