//! This position's selected art; source custody and recipes stay game-specific.
use serde::Deserialize;
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};
use unfolded_tabletop::assets::{AssetError, read_bounded, read_verified};

pub const RECIPE: &str = include_str!("../resources/bran-assets.json");

#[derive(Deserialize)]
pub struct AssetSpec {
    pub key: String,
    pub bytes: u64,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Deserialize)]
struct Crop {
    printing: String,
    page: u32,
    x: u32,
    y: u32,
}

#[derive(Deserialize)]
struct Selection {
    pdf_sha256: String,
    pdf_bytes: u64,
    token_sha256: String,
    token_bytes: u64,
    corp_back_sha256: String,
    corp_back_bytes: u64,
    font_sha256: String,
    font_bytes: u64,
    cards: Vec<Crop>,
    assets: Vec<AssetSpec>,
}

#[derive(Debug)]
pub struct VerifiedImage {
    pub key: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug)]
pub struct PreparedCollection {
    pub root: PathBuf,
    pub images: Vec<VerifiedImage>,
    pub font: Vec<u8>,
}

fn selection() -> Selection {
    serde_json::from_str(RECIPE).expect("checked-in art recipe")
}

pub fn default_root() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("netrunner-unfolded/bran-assets")
}

/// No state creation or persistence occurs here. The renderer must also load these bytes.
pub fn inspect(root: &Path) -> Result<PreparedCollection, AssetError> {
    let manifest = read_bounded(&root.join("manifest.json"), 65536).map_err(|error| {
        AssetError(format!(
            "{}: {error}. Prepare the Brân artwork collection with netrunner-assets.",
            root.display()
        ))
    })?;
    let actual: serde_json::Value = serde_json::from_slice(&manifest)?;
    let expected: serde_json::Value = serde_json::from_str(RECIPE)?;
    if actual != expected {
        return Err(AssetError("Wrong artwork collection or recipe. Prepare the selected bran-run-v2 collection in a fresh folder.".into()));
    }
    let mut images = Vec::new();
    for asset in selection().assets {
        let bytes = read_verified(root, &asset.key, asset.bytes, &asset.sha256)
            .map_err(|error| AssetError(format!("{}: {error}", asset.key)))?;
        let mut reader =
            image::ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::Png);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(asset.width);
        limits.max_image_height = Some(asset.height);
        limits.max_alloc = Some(16 * 1024 * 1024);
        reader.limits(limits);
        let decoded = reader
            .decode()
            .map_err(|error| AssetError(format!("{} cannot decode: {error}", asset.key)))?;
        if decoded.width() != asset.width || decoded.height() != asset.height {
            return Err(AssetError(format!(
                "{} has incorrect dimensions",
                asset.key
            )));
        }
        images.push(VerifiedImage {
            key: asset.key,
            width: asset.width,
            height: asset.height,
            rgba: decoded.into_rgba8().into_raw(),
        });
    }
    let selected = selection();
    let font = read_verified(root, "ui.ttf", selected.font_bytes, &selected.font_sha256)?;
    Ok(PreparedCollection {
        root: root.canonicalize()?,
        images,
        font,
    })
}

/// The supplied PDF and token archive are read-only. Existing outputs are preserved.
pub fn prepare(
    pdf: &Path,
    tokens: &Path,
    corp_back: &Path,
    output: &Path,
) -> Result<(), AssetError> {
    use unfolded_tabletop::assets::{PngCrop, crop_png, publish_directory, verify_bytes};
    if fs::symlink_metadata(output).is_ok() {
        inspect(output)?;
        return Ok(());
    }
    let selected = selection();
    let font = read_verified(Path::new("/usr/share/fonts/truetype/dejavu"), "DejaVuSans.ttf", selected.font_bytes, &selected.font_sha256)
        .map_err(|error| AssetError(format!("Required Ubuntu DejaVu Sans font: {error}. Install fonts-dejavu-core or use the recorded compatible font input.")))?;
    let pdf_bytes = read_bounded(pdf, selected.pdf_bytes)?;
    verify_bytes(&pdf_bytes, selected.pdf_bytes, &selected.pdf_sha256)?;
    let token = read_verified(
        tokens,
        "images/tokens/credit-advancement-light.png",
        selected.token_bytes,
        &selected.token_sha256,
    )?;
    let back_bytes = read_bounded(corp_back, selected.corp_back_bytes)?;
    verify_bytes(
        &back_bytes,
        selected.corp_back_bytes,
        &selected.corp_back_sha256,
    )?;
    // This exact, verified TTS Corp printing is the only back in this position.
    let back = image::load_from_memory_with_format(&back_bytes, image::ImageFormat::Jpeg)
        .map_err(|error| AssetError(format!("Corp card back cannot decode: {error}")))?;
    let credit = crop_png(
        &token,
        PngCrop {
            source_width: 512,
            source_height: 256,
            x: 256,
            y: 0,
            width: 256,
            height: 256,
        },
    )?;
    let output = unfolded_tabletop::assets::resolve_output(output)?;
    if output.starts_with(tokens.canonicalize()?)
        || output == pdf.canonicalize()?
        || output == corp_back.canonicalize()?
    {
        return Err(AssetError(
            "Choose an output outside the source collection.".into(),
        ));
    }
    let parent = output
        .parent()
        .ok_or_else(|| AssetError("Output needs a parent directory.".into()))?;
    fs::create_dir_all(parent)?;
    let stage = tempfile::Builder::new()
        .prefix(".netrunner-art-")
        .tempdir_in(parent)?;
    fs::create_dir(stage.path().join("cards"))?;
    fs::create_dir(stage.path().join("tokens"))?;
    fs::create_dir(stage.path().join("backs"))?;
    // Render our verified copy, so a source-file change cannot race the renderer.
    let source = stage.path().join("source.pdf");
    fs::write(&source, pdf_bytes)?;
    for crop in selected.cards {
        let key = format!("cards/{}.png", crop.printing);
        let spec = selected
            .assets
            .iter()
            .find(|asset| asset.key == key)
            .expect("each selected card has an output specification");
        let target = stage.path().join("cards").join(&crop.printing);
        let status = std::process::Command::new("pdftoppm")
            .args([
                "-f",
                &crop.page.to_string(),
                "-l",
                &crop.page.to_string(),
                "-r",
                "300",
                "-x",
                &crop.x.to_string(),
                "-y",
                &crop.y.to_string(),
                "-W",
                &spec.width.to_string(),
                "-H",
                &spec.height.to_string(),
                "-png",
                "-singlefile",
            ])
            .arg(&source)
            .arg(&target)
            .status()
            .map_err(|error| {
                AssetError(format!(
                    "Cannot run pdftoppm (install poppler-utils): {error}"
                ))
            })?;
        if !status.success() {
            return Err(AssetError(format!(
                "pdftoppm failed for {}: {status}",
                crop.printing
            )));
        }
    }
    fs::remove_file(source)?;
    fs::write(stage.path().join("tokens/credit.png"), credit)?;
    fs::write(stage.path().join("ui.ttf"), font)?;
    back.save(stage.path().join("backs/corp.png"))
        .map_err(|error| AssetError(error.to_string()))?;
    fs::write(stage.path().join("manifest.json"), RECIPE)?;
    if let Err(error) = inspect(stage.path()) {
        let retained = stage.keep();
        return Err(AssetError(format!(
            "Prepared output refused: {error}. Diagnostic output retained at {}. Nothing was installed.",
            retained.display()
        )));
    }
    publish_directory(stage.path(), &output)
}
