use std::collections::HashMap;

use super::*;
use crate::workflow::model::{StorageFileHint, StorageKind, StorageSpec};

fn folder_spec(path: &str) -> StorageSpec {
    StorageSpec {
        kind: StorageKind::Folder,
        path: path.to_string(),
        description: None,
        hint: None,
        files: Vec::new(),
    }
}

fn file_spec(path: &str) -> StorageSpec {
    StorageSpec {
        kind: StorageKind::File,
        path: path.to_string(),
        description: None,
        hint: None,
        files: Vec::new(),
    }
}

#[test]
fn ensure_creates_missing_folder() {
    let tmp = tempfile::TempDir::new().unwrap();
    let backend = FilesystemBackend::new(tmp.path().to_path_buf());
    let spec = folder_spec("./characters");

    backend.ensure(&spec).unwrap();

    let expected = tmp.path().join("characters");
    assert!(expected.is_dir(), "folder should exist");
}

#[test]
fn ensure_is_idempotent_on_folder() {
    let tmp = tempfile::TempDir::new().unwrap();
    let backend = FilesystemBackend::new(tmp.path().to_path_buf());
    let spec = folder_spec("./world");

    backend.ensure(&spec).unwrap();
    // Write a file to verify the second ensure doesn't wipe contents.
    std::fs::write(tmp.path().join("world/notes.md"), "pre-existing").unwrap();
    backend.ensure(&spec).unwrap();

    assert_eq!(
        std::fs::read_to_string(tmp.path().join("world/notes.md")).unwrap(),
        "pre-existing"
    );
}

#[test]
fn ensure_creates_missing_file_and_parent() {
    let tmp = tempfile::TempDir::new().unwrap();
    let backend = FilesystemBackend::new(tmp.path().to_path_buf());
    let spec = file_spec("./nested/bible.md");

    backend.ensure(&spec).unwrap();

    let expected = tmp.path().join("nested/bible.md");
    assert!(expected.is_file());
}

#[test]
fn ensure_absolute_path_bypasses_root() {
    let tmp = tempfile::TempDir::new().unwrap();
    let other = tempfile::TempDir::new().unwrap();
    let target = other.path().join("abs-store");
    let backend = FilesystemBackend::new(tmp.path().to_path_buf());
    let spec = folder_spec(target.to_str().unwrap());

    backend.ensure(&spec).unwrap();

    assert!(target.is_dir());
    // The zig-root was not touched.
    assert!(!tmp.path().join("abs-store").exists());
}

#[test]
fn listing_reflects_current_folder_contents() {
    let tmp = tempfile::TempDir::new().unwrap();
    let backend = FilesystemBackend::new(tmp.path().to_path_buf());
    let spec = folder_spec("./characters");
    backend.ensure(&spec).unwrap();

    let before = backend.listing(&spec).unwrap();
    assert!(before.entries.is_empty());

    std::fs::write(tmp.path().join("characters/alice.md"), "alice").unwrap();
    std::fs::write(tmp.path().join("characters/bob.md"), "bob").unwrap();

    let after = backend.listing(&spec).unwrap();
    let names: Vec<&str> = after.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["alice.md", "bob.md"]);
}

#[test]
fn listing_empty_folder_gracefully_returns_nothing() {
    let tmp = tempfile::TempDir::new().unwrap();
    let backend = FilesystemBackend::new(tmp.path().to_path_buf());
    // Not ensured — folder absent.
    let listing = backend.listing(&folder_spec("./missing")).unwrap();
    assert!(listing.entries.is_empty());
}

#[test]
fn listing_file_spec_reports_single_entry_after_write() {
    let tmp = tempfile::TempDir::new().unwrap();
    let backend = FilesystemBackend::new(tmp.path().to_path_buf());
    let spec = file_spec("./bible.md");
    backend.ensure(&spec).unwrap();

    let before = backend.listing(&spec).unwrap();
    assert_eq!(before.entries.len(), 1);
    assert_eq!(before.entries[0].name, "bible.md");
}

#[test]
fn manager_items_for_step_applies_scoping() {
    let tmp = tempfile::TempDir::new().unwrap();
    let backend = FilesystemBackend::new(tmp.path().to_path_buf());

    let mut storage = HashMap::new();
    storage.insert("characters".to_string(), folder_spec("./characters"));
    storage.insert("world".to_string(), folder_spec("./world"));
    storage.insert("bible".to_string(), file_spec("./bible.md"));

    let manager = StorageManager::build(&storage, backend).unwrap();

    // None = all
    let all = manager.items_for_step(None);
    assert_eq!(all.len(), 3);

    // Empty = none
    let none = manager.items_for_step(Some(&[]));
    assert!(none.is_empty());

    // Named subset
    let scoped_names = vec!["characters".to_string(), "bible".to_string()];
    let subset = manager.items_for_step(Some(&scoped_names));
    let names: Vec<&str> = subset.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"characters"));
    assert!(names.contains(&"bible"));
    assert!(!names.contains(&"world"));
}

#[test]
fn render_block_contains_item_paths_hints_and_contents() {
    let tmp = tempfile::TempDir::new().unwrap();
    let backend = FilesystemBackend::new(tmp.path().to_path_buf());

    let mut storage = HashMap::new();
    storage.insert(
        "characters".to_string(),
        StorageSpec {
            kind: StorageKind::Folder,
            path: "./characters".to_string(),
            description: Some("Character profiles".to_string()),
            hint: Some("One file per character".to_string()),
            files: vec![StorageFileHint {
                name: "README.md".into(),
                description: Some("Index of characters".into()),
            }],
        },
    );

    let manager = StorageManager::build(&storage, backend).unwrap();
    // Simulate a previous step writing a file.
    std::fs::write(tmp.path().join("characters/alice.md"), "alice").unwrap();

    let block = manager.render_block(None).unwrap().expect("block");
    assert!(block.contains("<storage>"));
    assert!(block.contains("name=\"characters\""));
    assert!(block.contains("type=\"folder\""));
    assert!(block.contains("Character profiles"));
    assert!(block.contains("One file per character"));
    assert!(block.contains("README.md: Index of characters"));
    assert!(block.contains("alice.md"));
}

#[test]
fn render_block_returns_none_for_empty_scope() {
    let tmp = tempfile::TempDir::new().unwrap();
    let backend = FilesystemBackend::new(tmp.path().to_path_buf());

    let mut storage = HashMap::new();
    storage.insert("characters".to_string(), folder_spec("./characters"));
    let manager = StorageManager::build(&storage, backend).unwrap();

    let block = manager.render_block(Some(&[])).unwrap();
    assert!(block.is_none());
}

#[test]
fn render_block_none_when_no_storage_declared() {
    let manager = StorageManager::empty();
    assert!(manager.render_block(None).unwrap().is_none());
}

#[test]
fn add_dirs_for_step_uses_parent_for_file_storage() {
    let tmp = tempfile::TempDir::new().unwrap();
    let backend = FilesystemBackend::new(tmp.path().to_path_buf());

    let mut storage = HashMap::new();
    storage.insert("characters".to_string(), folder_spec("./characters"));
    storage.insert("bible".to_string(), file_spec("./nested/bible.md"));

    let manager = StorageManager::build(&storage, backend).unwrap();
    let dirs = manager.add_dirs_for_step(None);

    // characters folder and the parent of the bible file.
    assert!(dirs.iter().any(|d| d.ends_with("characters")));
    assert!(dirs.iter().any(|d| d.ends_with("nested")));
}

#[test]
fn render_block_escapes_xml_metacharacters() {
    let tmp = tempfile::TempDir::new().unwrap();
    let backend = FilesystemBackend::new(tmp.path().to_path_buf());

    let mut storage = HashMap::new();
    storage.insert(
        "notes".to_string(),
        StorageSpec {
            kind: StorageKind::Folder,
            path: "./notes".to_string(),
            description: Some("tags <x> & <y>".to_string()),
            hint: None,
            files: Vec::new(),
        },
    );
    let manager = StorageManager::build(&storage, backend).unwrap();

    let block = manager.render_block(None).unwrap().expect("block");
    assert!(block.contains("tags &lt;x&gt; &amp; &lt;y&gt;"));
    assert!(!block.contains("tags <x> & <y>"));
}
