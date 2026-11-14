use netrunner_unfolded::assets;
use std::fs;

#[test]
fn missing_art_is_refused_without_touching_existing_files() {
    let root = tempfile::tempdir().unwrap();
    let save = root.path().join("existing-save.json");
    fs::write(&save, b"owner's existing save").unwrap();
    assert!(assets::inspect(root.path()).is_err());
    assert_eq!(fs::read(&save).unwrap(), b"owner's existing save");
}

#[test]
fn a_manifest_cannot_remove_required_art_or_make_wrong_bytes_acceptable() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("manifest.json"),
        r#"{"collection":"bran-run-v1","assets":[]}"#,
    )
    .unwrap();
    assert!(
        assets::inspect(root.path())
            .unwrap_err()
            .to_string()
            .contains("Wrong artwork")
    );
}

#[test]
fn failed_preparation_preserves_an_existing_directory() {
    let root = tempfile::tempdir().unwrap();
    let sentinel = root.path().join("keep");
    fs::write(&sentinel, b"unchanged").unwrap();
    assert!(
        assets::prepare(
            Path::new("/missing.pdf"),
            Path::new("/missing-tokens"),
            Path::new("/missing-corp-back.jpg"),
            root.path()
        )
        .is_err()
    );
    assert_eq!(fs::read(sentinel).unwrap(), b"unchanged");
}

use std::path::Path;

#[test]
fn ordinary_turns_refuse_the_bran_manifest_before_reading_or_changing_state() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("manifest.json"), assets::RECIPE).unwrap();
    let save = root.path().join("existing-save.json");
    fs::write(&save, b"owner's existing save").unwrap();
    let error = assets::inspect_turns(root.path()).unwrap_err().to_string();
    assert!(error.contains("Wrong artwork collection"), "{error}");
    assert_eq!(fs::read(&save).unwrap(), b"owner's existing save");
}

#[test]
fn turns_preparation_preserves_an_existing_bran_collection() {
    let root = tempfile::tempdir().unwrap();
    let manifest = root.path().join("manifest.json");
    fs::write(&manifest, assets::RECIPE).unwrap();
    let error = assets::prepare_turns(
        Path::new("/missing.pdf"),
        Path::new("/missing-tokens"),
        Path::new("/missing-corp-back.jpg"),
        Path::new("/missing-runner-back.jpg"),
        root.path(),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("Wrong artwork collection"), "{error}");
    assert_eq!(fs::read_to_string(&manifest).unwrap(), assets::RECIPE);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
#[ignore = "requires NETRUNNER_ART_ROOT pointing to the prepared private collection"]
fn real_collection_is_relocatable_and_corruption_is_refused() {
    let source = std::env::var_os("NETRUNNER_ART_ROOT").expect("NETRUNNER_ART_ROOT");
    let admitted = assets::inspect(Path::new(&source)).unwrap();
    assert_eq!(admitted.images.len(), 6);
    let copy = tempfile::tempdir().unwrap();
    fs::copy(
        admitted.root.join("manifest.json"),
        copy.path().join("manifest.json"),
    )
    .unwrap();
    fs::copy(admitted.root.join("ui.ttf"), copy.path().join("ui.ttf")).unwrap();
    for image in &admitted.images {
        let target = copy.path().join(&image.key);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(admitted.root.join(&image.key), target).unwrap();
    }
    assert_eq!(assets::inspect(copy.path()).unwrap().images.len(), 6);
    fs::write(copy.path().join("tokens/credit.png"), b"not a PNG").unwrap();
    assert!(assets::inspect(copy.path()).is_err());
    assert!(assets::inspect(&admitted.root).is_ok());
}

#[test]
#[ignore = "requires NETRUNNER_TURNS_ART_ROOT pointing to the prepared private collection"]
fn real_turns_collection_relocates_and_refuses_missing_or_changed_new_art() {
    let source = std::env::var_os("NETRUNNER_TURNS_ART_ROOT").expect("NETRUNNER_TURNS_ART_ROOT");
    let admitted = assets::inspect_turns(Path::new(&source)).unwrap();
    assert_eq!(admitted.images.len(), 24);
    assert!(assets::inspect(&admitted.root).is_err());
    let copy = tempfile::tempdir().unwrap();
    for key in ["manifest.json", "ui.ttf"]
        .into_iter()
        .chain(admitted.images.iter().map(|image| image.key.as_str()))
    {
        let target = copy.path().join(key);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(admitted.root.join(key), target).unwrap();
    }
    assert_eq!(assets::inspect_turns(copy.path()).unwrap().images.len(), 24);
    let save = copy.path().join("existing-save.json");
    fs::write(&save, b"owner's existing save").unwrap();
    for key in [
        "cards/30075.png",
        "cards/30029.png",
        "cards/30074.png",
        "cards/30042.png",
        "cards/30028.png",
        "cards/30069.png",
        "cards/30070.png",
        "tokens/click-blue.png",
        "backs/runner.png",
    ] {
        let target = copy.path().join(key);
        let original = fs::read(&target).unwrap();
        fs::remove_file(&target).unwrap();
        assert!(
            assets::inspect_turns(copy.path())
                .unwrap_err()
                .to_string()
                .contains(key)
        );
        let mut damaged = original.clone();
        damaged[0] ^= 1;
        fs::write(&target, damaged).unwrap();
        assert!(
            assets::inspect_turns(copy.path())
                .unwrap_err()
                .to_string()
                .contains(key)
        );
        fs::write(target, original).unwrap();
    }
    assert_eq!(fs::read(save).unwrap(), b"owner's existing save");
    assert!(assets::inspect_turns(copy.path()).is_ok());
    assert!(assets::inspect_turns(&admitted.root).is_ok());
}

#[test]
#[ignore = "requires NETRUNNER_TURNS_ART_ROOT and NETRUNNER_REVIEWED_ART_ROOT for PhotoCraft exports"]
fn real_turns_outputs_match_reviewed_photocraft_exports() {
    let source = std::env::var_os("NETRUNNER_TURNS_ART_ROOT").expect("NETRUNNER_TURNS_ART_ROOT");
    let reviewed =
        std::env::var_os("NETRUNNER_REVIEWED_ART_ROOT").expect("NETRUNNER_REVIEWED_ART_ROOT");
    let admitted = assets::inspect_turns(Path::new(&source)).unwrap();
    let recipe: serde_json::Value = serde_json::from_str(assets::TURNS_RECIPE).unwrap();
    let exports: Vec<assets::AssetSpec> =
        serde_json::from_value(recipe["reviewed_exports"].clone()).unwrap();
    for export in exports {
        let bytes = unfolded_tabletop::assets::read_verified(
            Path::new(&reviewed),
            &export.key,
            export.bytes,
            &export.sha256,
        )
        .unwrap();
        let expected = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .unwrap()
            .into_rgba8();
        let actual = admitted
            .images
            .iter()
            .find(|image| image.key == export.key)
            .unwrap();
        assert_eq!(
            (actual.width, actual.height),
            expected.dimensions(),
            "{}",
            export.key
        );
        assert!(
            actual.rgba == expected.as_raw().as_slice(),
            "{} differs from the reviewed PhotoCraft pixels",
            export.key
        );
        println!("PhotoCraft pixel equivalence: {}", export.key);
    }
}
