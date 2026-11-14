//! On-device text recognition (OCR) for pages without a text layer:
//! scanned documents, canvas-drawn pages, text inside images.
//!
//! Uses the `ocrs` engine (pure Rust, CPU). Its two models (about 12 MB)
//! are downloaded on first use from the ocrs project and checked against
//! pinned SHA-256 hashes; set `OYNIX_OCR_MODELS` to a folder holding
//! `text-detection.onnx` and `text-recognition.onnx` to use local copies
//! (e.g. a distribution package). Latin script only.

use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const MODELS: [(&str, &str, &str); 2] = [
    ("text-detection.onnx", "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.onnx", "a917b23dbd9524b465df7e922641b3ff2981623df4ded5a0234004ef2fee7cfe"),
    ("text-recognition.onnx", "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.onnx", "86c145c2edb96c8caed5b1ebb8f44d706408922211451c309c625157dd6061c5"),
];

/// Shared by all profiles: ~/.local/share/oynix/ocr-models.
pub fn models_dir() -> PathBuf {
    if let Ok(d) = std::env::var("OYNIX_OCR_MODELS") {
        return PathBuf::from(d);
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("oynix").join("ocr-models")
}

pub fn models_present() -> bool {
    MODELS.iter().all(|(f, _, _)| models_dir().join(f).is_file())
}

fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
}

/// Download any missing model, verifying its hash.
pub fn ensure_models() -> Result<(), String> {
    let dir = models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn’t create {}: {e}", dir.display()))?;
    for (file, url, hash) in MODELS {
        let dest = dir.join(file);
        if dest.is_file() {
            continue;
        }
        if std::env::var_os("OYNIX_OCR_MODELS").is_some() {
            return Err(format!("{} is missing", dest.display()));
        }
        let mut resp = super::updater::agent().get(url).call().map_err(|e| format!("Couldn’t download the text recognition model: {e}"))?;
        let mut data = Vec::new();
        resp.body_mut().with_config().limit(64 << 20).reader().read_to_end(&mut data).map_err(|e| e.to_string())?;
        if sha256_hex(&data) != hash {
            return Err("The downloaded text recognition model didn’t match its expected checksum".into());
        }
        let part = dest.with_extension("part");
        std::fs::write(&part, &data).map_err(|e| e.to_string())?;
        std::fs::rename(&part, &dest).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn engine() -> Result<&'static Mutex<OcrEngine>, String> {
    static ENGINE: OnceLock<Mutex<OcrEngine>> = OnceLock::new();
    if let Some(e) = ENGINE.get() {
        return Ok(e);
    }
    ensure_models()?;
    let dir = models_dir();
    let load = |f: &str| rten::Model::load_file(dir.join(f)).map_err(|e| format!("Couldn’t load {f}: {e}"));
    let e = OcrEngine::new(OcrEngineParams {
        detection_model: Some(load(MODELS[0].0)?),
        recognition_model: Some(load(MODELS[1].0)?),
        ..Default::default()
    })
    .map_err(|e| e.to_string())?;
    Ok(ENGINE.get_or_init(|| Mutex::new(e)))
}

/// Recognise text in an image file, optionally only in a region (pixels of
/// that image). Returns the text with one line per detected line.
pub fn recognize(path: &std::path::Path, region: Option<(u32, u32, u32, u32)>) -> Result<String, String> {
    let img = image::open(path).map_err(|e| format!("Couldn’t read the captured image: {e}"))?.into_rgb8();
    let img = match region {
        Some((x, y, w, h)) => {
            let (iw, ih) = img.dimensions();
            let (x, y) = (x.min(iw.saturating_sub(1)), y.min(ih.saturating_sub(1)));
            let (w, h) = (w.min(iw - x).max(1), h.min(ih - y).max(1));
            image::imageops::crop_imm(&img, x, y, w, h).to_image()
        }
        None => img,
    };
    let engine = engine()?;
    let engine = engine.lock().map_err(|_| "The text recognition engine failed earlier".to_string())?;
    let src = ImageSource::from_bytes(img.as_raw(), img.dimensions()).map_err(|e| e.to_string())?;
    let input = engine.prepare_input(src).map_err(|e| e.to_string())?;
    engine.get_text(&input).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    /// Needs the models (downloads ~12 MB); run with `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn reads_rendered_text() {
        let dir = std::env::temp_dir().join("oynix-ocr-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.png");
        let ok = std::process::Command::new("convert")
            .args(["-size", "700x120", "xc:white", "-pointsize", "40", "-fill", "black", "-annotate", "+20+75", "Scrape this text", p.to_str().unwrap()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            return;
        }
        let text = super::recognize(&p, None).unwrap();
        assert!(text.contains("Scrape this text"), "{text}");
        let part = super::recognize(&p, Some((0, 0, 260, 120))).unwrap();
        assert!(part.contains("Scrape") && !part.contains("text"), "{part}");
    }
}
