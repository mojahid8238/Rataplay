use anyhow::Result;
use image::DynamicImage;
use std::sync::OnceLock;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn get_client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/115.0")
            .build()
            .expect("Failed to build reqwest client")
    })
}

pub async fn download_image(url: &str, video_id: &str) -> Result<DynamicImage> {
    let client = get_client();

    // List of URLs to try in order
    let mut urls_to_try = Vec::new();

    // 1. The provided URL (could be from yt-dlp or our initial fallback)
    if !url.is_empty() {
        urls_to_try.push(url.to_string());
    }

    // 2. Fallbacks based on ID
    if !video_id.is_empty() {
        urls_to_try.push(format!(
            "https://i.ytimg.com/vi/{}/maxresdefault.jpg",
            video_id
        ));
        urls_to_try.push(format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", video_id));
        urls_to_try.push(format!("https://i.ytimg.com/vi/{}/mqdefault.jpg", video_id));
    }

    // Try each URL
    for target_url in urls_to_try {
        if let Ok(resp) = client.get(&target_url).send().await {
            if resp.status().is_success() {
                if let Ok(bytes) = resp.bytes().await {
                    if let Ok(img) = image::load_from_memory(&bytes) {
                        return Ok(img);
                    }
                }
            }
        }
    }

    anyhow::bail!("Failed to download image for {}", video_id)
}
