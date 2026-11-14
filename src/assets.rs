//! This position's selected art; source custody and recipes stay game-specific.
use serde::Deserialize;
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};
use unfolded_tabletop::assets::{AssetError, read_bounded, read_verified};

pub const RECIPE: &str = include_str!("../resources/bran-assets.json");
pub const TURNS_RECIPE: &str = include_str!("../resources/turns-assets.json");

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
struct TokenCrop {
    source: String,
    key: String,
    bytes: u64,
    sha256: String,
    source_width: u32,
    source_height: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
struct Selection {
    collection: String,
    pdf_sha256: String,
    pdf_bytes: u64,
    token_sha256: String,
    token_bytes: u64,
    corp_back_sha256: String,
    corp_back_bytes: u64,
    runner_back_sha256: Option<String>,
    runner_back_bytes: Option<u64>,
    font_sha256: String,
    font_bytes: u64,
    cards: Vec<Crop>,
    #[serde(default)]
    extra_tokens: Vec<TokenCrop>,
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

fn selection(recipe: &str) -> Selection {
    serde_json::from_str(recipe).expect("checked-in art recipe")
}

pub fn default_root() -> PathBuf {
    data_root().join("netrunner-unfolded/bran-assets")
}

pub fn default_turns_root() -> PathBuf {
    data_root().join("netrunner-unfolded/turns-assets")
}

fn data_root() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// No state creation or persistence occurs here. The renderer must also load these bytes.
pub fn inspect(root: &Path) -> Result<PreparedCollection, AssetError> {
    inspect_recipe(root, RECIPE)
}

/// The bounded ordinary-turns collection cannot admit the complete starter decks.
pub fn inspect_turns(root: &Path) -> Result<PreparedCollection, AssetError> {
    inspect_recipe(root, TURNS_RECIPE)
}

fn inspect_recipe(root: &Path, recipe: &str) -> Result<PreparedCollection, AssetError> {
    let selected = selection(recipe);
    let manifest = read_bounded(&root.join("manifest.json"), 65536).map_err(|error| {
        AssetError(format!(
            "{}: {error}. Prepare the {} artwork collection with netrunner-assets.",
            root.display(),
            selected.collection
        ))
    })?;
    let actual: serde_json::Value = serde_json::from_slice(&manifest)?;
    let expected: serde_json::Value = serde_json::from_str(recipe)?;
    if actual != expected {
        return Err(AssetError(format!(
            "Wrong artwork collection or recipe. Prepare the selected {} collection in a fresh folder.",
            selected.collection
        )));
    }
    let mut images = Vec::new();
    for asset in selected.assets {
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
    prepare_recipe(pdf, tokens, corp_back, None, output, RECIPE)
}

pub fn prepare_turns(
    pdf: &Path,
    tokens: &Path,
    corp_back: &Path,
    runner_back: &Path,
    output: &Path,
) -> Result<(), AssetError> {
    prepare_recipe(
        pdf,
        tokens,
        corp_back,
        Some(runner_back),
        output,
        TURNS_RECIPE,
    )
}

fn prepare_recipe(
    pdf: &Path,
    tokens: &Path,
    corp_back: &Path,
    runner_back: Option<&Path>,
    output: &Path,
    recipe: &str,
) -> Result<(), AssetError> {
    use unfolded_tabletop::assets::{PngCrop, crop_png, publish_directory, verify_bytes};
    if fs::symlink_metadata(output).is_ok() {
        inspect_recipe(output, recipe)?;
        return Ok(());
    }
    let selected = selection(recipe);
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
    // These exact TTS sources provide opaque, consistent backs for each side.
    let back = image::load_from_memory_with_format(&back_bytes, image::ImageFormat::Jpeg)
        .map_err(|error| AssetError(format!("Corp card back cannot decode: {error}")))?;
    let runner = if let Some(path) = runner_back {
        let length = selected.runner_back_bytes.expect("turns Runner back size");
        let hash = selected
            .runner_back_sha256
            .as_ref()
            .expect("turns Runner back hash");
        let bytes = read_bounded(path, length)?;
        verify_bytes(&bytes, length, hash)
            .map_err(|error| AssetError(format!("Runner card back: {error}")))?;
        Some(
            image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg)
                .map_err(|error| AssetError(format!("Runner card back cannot decode: {error}")))?,
        )
    } else {
        None
    };
    let mut extra_tokens = Vec::new();
    for crop in selected.extra_tokens {
        let bytes = read_verified(tokens, &crop.source, crop.bytes, &crop.sha256)
            .map_err(|error| AssetError(format!("{}: {error}", crop.source)))?;
        let prepared = crop_png(
            &bytes,
            PngCrop {
                source_width: crop.source_width,
                source_height: crop.source_height,
                x: crop.x,
                y: crop.y,
                width: crop.width,
                height: crop.height,
            },
        )?;
        extra_tokens.push((crop.key, prepared));
    }
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
        || runner_back.map(Path::canonicalize).transpose()?.as_ref() == Some(&output)
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
    if let Some(runner) = runner {
        runner
            .save(stage.path().join("backs/runner.png"))
            .map_err(|error| AssetError(error.to_string()))?;
    }
    for (key, bytes) in extra_tokens {
        fs::write(stage.path().join(key), bytes)?;
    }
    fs::write(stage.path().join("manifest.json"), recipe)?;
    if let Err(error) = inspect_recipe(stage.path(), recipe) {
        let retained = stage.keep();
        return Err(AssetError(format!(
            "Prepared output refused: {error}. Diagnostic output retained at {}. Nothing was installed.",
            retained.display()
        )));
    }
    publish_directory(stage.path(), &output)
}
