//! Explicit network smoke test. Uses only a generated geometric fixture, never user photos.
use coskit::{dotenv, gemini_client, image_utils, openai_client as api};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenv::load_dotenv_files();
    let base = api::resolve_base_url("");
    let key = api::resolve_api_key("");
    if key.is_empty() {
        return Err("OPENAI_API_KEY is missing".into());
    }
    let text_model = api::resolve_text_model("");
    let image_model = api::resolve_image_model("");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(240))
        .build()
        .map_err(|e| e.to_string())?;
    let response = api::call_text(
        &client,
        &base,
        &key,
        &text_model,
        json!([{"role":"user","parts":[{"text":"Reply with the word READY only."}]}]),
        0.,
        1,
    )
    .await?;
    if gemini_client::extract_text(&response).trim().is_empty() {
        return Err("Text model returned no content".into());
    }
    println!("Text model: PASS ({text_model})");
    if std::env::args().any(|v| v == "--text-only") {
        return Ok(());
    }
    let img = image::RgbaImage::from_fn(256, 256, |x, y| {
        if (64..192).contains(&x) && (64..192).contains(&y) {
            image::Rgba([220, 50, 50, 255])
        } else {
            image::Rgba([220, 230, 235, 255])
        }
    });
    let bytes = image_utils::image_to_png_bytes(&image::DynamicImage::ImageRgba8(img))?;
    let response=api::call_image(&client,&base,&key,&image_model,json!([{"role":"user","parts":[{"text":"Change the red square to a blue square. Preserve the square shape and light background. Return the edited image."},{"inline_data":{"mime_type":"image/png","data":image_utils::bytes_to_base64(&bytes)}}]}]),1,Some((256,256)),None).await?;
    let bytes =
        gemini_client::extract_image_bytes(&response).ok_or("Image model returned no image")?;
    let image = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../test_output");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    image
        .save(dir.join("model-smoke.png"))
        .map_err(|e| e.to_string())?;
    println!(
        "Image model: PASS ({image_model}), {}x{} decodable image",
        image.width(),
        image.height()
    );
    Ok(())
}
