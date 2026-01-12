use anyhow::{Result, Context};
use image::DynamicImage;

pub async fn download_image(url: &str) -> Result<DynamicImage> {
    let bytes = reqwest::get(url).await?.bytes().await?;
    let img = image::load_from_memory(&bytes)
        .context("Failed to decode image")?;
    Ok(img)
}
