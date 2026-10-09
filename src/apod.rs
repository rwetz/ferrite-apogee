//! NASA APOD metadata. Keys stay in the environment and out of diagnostics.
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Entry {
    pub title: String,
    pub date: String,
    pub explanation: String,
    pub credit: String,
    pub url: String,
    pub image_url: String,
    pub image: bool,
}

pub fn parse(json: &str) -> Result<Entry, String> {
    let data: Value = serde_json::from_str(json).map_err(|_| "Invalid APOD response")?;
    let v = if data.is_array() { data.get(0).ok_or("Empty APOD feed")? } else { &data };
    let string = |key: &str| v[key].as_str().map(str::to_string).ok_or_else(|| format!("Missing APOD {key}"));
    let media = string("media_type")?;
    if media != "image" && media != "video" {
        return Err("Unsupported APOD media type".into());
    }
    let url = string("url")?;
    if !url.starts_with("https://") {
        return Err("APOD media requires an HTTPS URL".into());
    }
    let image_url = v["hdurl"].as_str().unwrap_or(&url).to_string();
    if !image_url.starts_with("https://") {
        return Err("APOD image requires HTTPS".into());
    }
    Ok(Entry {
        title: plain_text(&string("title")?),
        date: string("date")?,
        explanation: plain_text(&string("explanation")?),
        credit: plain_text(v["copyright"].as_str().or_else(|| v["credit"].as_str()).unwrap_or("NASA / APOD")),
        url,
        image_url,
        image: media == "image",
    })
}

fn plain_text(html: &str) -> String {
    let mut tag = false;
    let mut text = String::new();
    for ch in html.chars() {
        match ch {
            '<' => tag = true,
            '>' => tag = false,
            _ if !tag => text.push(ch),
            _ => {}
        }
    }
    text.replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

pub fn fetch() -> Result<Entry, String> {
    let key = std::env::var("NASA_API_KEY").ok().filter(|key| !key.trim().is_empty()).unwrap_or_else(|| "DEMO_KEY".into());
    let primary = ureq::get("https://api.nasa.gov/planetary/apod")
        .query("api_key", key.trim())
        .config()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build()
        .call();
    let mut response = match primary {
        Ok(response) => response,
        Err(ureq::Error::StatusCode(429)) => return Err("NASA rate limit reached. Set NASA_API_KEY for your own quota.".into()),
        Err(ureq::Error::StatusCode(401 | 403)) => return Err("NASA rejected the API key.".into()),
        Err(ureq::Error::StatusCode(code)) if code < 500 => return Err(format!("NASA APOD returned HTTP {code}")),
        Err(_) => ureq::get("https://science.nasa.gov/wp-json/wp/v2/apod-basic")
            .query("per_page", "1")
            .config()
            .timeout_global(Some(std::time::Duration::from_secs(30)))
            .build()
            .call()
            .map_err(|_| "NASA APOD and its public fallback are unavailable")?,
    };
    let json = response.body_mut().with_config().limit(256 * 1024).read_to_string().map_err(|_| "Couldn't read APOD metadata")?;
    parse(&json)
}

pub struct Preview {
    pub width: u32,
    pub height: u32,
    pub levels: Vec<u8>,
}

pub fn picture(entry: &Entry) -> Result<Preview, String> {
    if !entry.image {
        return Err("Today's APOD is a video. Open the source to watch it.".into());
    }
    let mut response = ureq::get(&entry.image_url)
        .config()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build()
        .call()
        .map_err(|_| "Couldn't download the APOD image")?;
    let bytes =
        response.body_mut().with_config().limit(16 * 1024 * 1024).read_to_vec().map_err(|_| "Image exceeds the 16 MB preview limit")?;
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().map_err(|_| "Unknown image format")?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().map_err(|_| "Unsupported image or image exceeds preview limits")?.thumbnail(720, 440).to_luma8();
    Ok(Preview { width: image.width(), height: image.height(), levels: image.into_raw() })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn videos_are_metadata_not_images() {
        let entry =
            parse(r#"{"title":"Sky","date":"2026-10-09","explanation":"Test","media_type":"video","url":"https://example.com/video"}"#)
                .unwrap();
        assert!(!entry.image);
        assert!(picture(&entry).is_err());
    }
    #[test]
    fn rejects_bad_metadata() {
        assert!(parse("{}").is_err());
        assert!(parse("not json").is_err());
    }
    #[test]
    fn public_feed_uses_image_url_and_strips_markup() {
        let entry = parse(r#"[{"title":"Sky &amp; Stars","date":"2026-10-09","explanation":"A <a href=\"https://example.com\">star</a>.","media_type":"image","url":"https://science.nasa.gov/article","hdurl":"https://assets.science.nasa.gov/image.jpg","copyright":"<b>NASA</b>"}]"#).unwrap();
        assert_eq!(entry.image_url, "https://assets.science.nasa.gov/image.jpg");
        assert_eq!(entry.explanation, "A star.");
        assert_eq!(entry.credit, "NASA");
        assert_eq!(entry.title, "Sky & Stars");
        assert!(parse("[]").is_err());
    }
}
