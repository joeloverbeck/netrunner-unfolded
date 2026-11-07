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
            root.path()
        )
        .is_err()
    );
    assert_eq!(fs::read(sentinel).unwrap(), b"unchanged");
}

use std::path::Path;

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
