use std::collections::BTreeMap;

use super::*;

fn temp_dir_with_file(name: &str, content: &str) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::TempDir::new().unwrap();
    let file = tmp.path().join(name);
    std::fs::write(&file, content).unwrap();
    (tmp, file)
}

// =====================================================================
// Manifest I/O
// =====================================================================

#[test]
fn load_manifest_returns_empty_when_missing() {
    let tmp = tempfile::TempDir::new().unwrap();
    let m = load_manifest(tmp.path()).unwrap();
    assert_eq!(m.next_id, 1);
    assert!(m.entries.is_empty());
}

#[test]
fn manifest_round_trip() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut m = Manifest {
        next_id: 42,
        entries: BTreeMap::new(),
    };
    m.entries.insert(
        "1".into(),
        MemoryEntry {
            name: "test.md".into(),
            file: "test.md".into(),
            description: Some("A test file".into()),
            tags: vec!["tag1".into()],
            step: None,
            source: "/tmp/test.md".into(),
            added: Utc::now(),
        },
    );
    save_manifest(tmp.path(), &m).unwrap();

    let loaded = load_manifest(tmp.path()).unwrap();
    assert_eq!(loaded.next_id, 42);
    assert_eq!(loaded.entries.len(), 1);
    assert_eq!(loaded.entries["1"].name, "test.md");
    assert_eq!(
        loaded.entries["1"].description.as_deref(),
        Some("A test file")
    );
    assert_eq!(loaded.entries["1"].tags, vec!["tag1".to_string()]);
}

// =====================================================================
// Add
// =====================================================================

#[test]
fn add_copies_file_and_assigns_id() {
    let (_src_dir, src_file) = temp_dir_with_file("notes.md", "# Notes\nSome content.");
    let target_dir = tempfile::TempDir::new().unwrap();

    // Manually ensure the target directory so we can test add_to_target_dir.
    let mut manifest = load_manifest(target_dir.path()).unwrap();
    let id = manifest.next_id;
    manifest.next_id += 1;

    let dest = target_dir.path().join("notes.md");
    std::fs::copy(&src_file, &dest).unwrap();

    manifest.entries.insert(
        id.to_string(),
        MemoryEntry {
            name: "notes.md".into(),
            file: "notes.md".into(),
            description: None,
            tags: vec![],
            step: None,
            source: src_file.display().to_string(),
            added: Utc::now(),
        },
    );
    save_manifest(target_dir.path(), &manifest).unwrap();

    let loaded = load_manifest(target_dir.path()).unwrap();
    assert_eq!(loaded.next_id, 2);
    assert_eq!(loaded.entries.len(), 1);
    assert_eq!(loaded.entries["1"].name, "notes.md");
    assert!(dest.exists());
    assert_eq!(
        std::fs::read_to_string(&dest).unwrap(),
        "# Notes\nSome content."
    );
}

// =====================================================================
// Update
// =====================================================================

#[test]
fn update_modifies_metadata() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("file.md"), "content").unwrap();

    let mut manifest = Manifest {
        next_id: 2,
        entries: BTreeMap::new(),
    };
    manifest.entries.insert(
        "1".into(),
        MemoryEntry {
            name: "file.md".into(),
            file: "file.md".into(),
            description: None,
            tags: vec![],
            step: None,
            source: "/tmp/file.md".into(),
            added: Utc::now(),
        },
    );
    save_manifest(tmp.path(), &manifest).unwrap();

    // Simulate update.
    let mut m = load_manifest(tmp.path()).unwrap();
    let entry = m.entries.get_mut("1").unwrap();
    entry.description = Some("Updated description".into());
    entry.tags = vec!["new-tag".into()];
    save_manifest(tmp.path(), &m).unwrap();

    let loaded = load_manifest(tmp.path()).unwrap();
    assert_eq!(
        loaded.entries["1"].description.as_deref(),
        Some("Updated description")
    );
    assert_eq!(loaded.entries["1"].tags, vec!["new-tag".to_string()]);
}

// =====================================================================
// Delete
// =====================================================================

#[test]
fn delete_removes_entry_and_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let file_path = tmp.path().join("data.txt");
    std::fs::write(&file_path, "hello").unwrap();

    let mut manifest = Manifest {
        next_id: 2,
        entries: BTreeMap::new(),
    };
    manifest.entries.insert(
        "1".into(),
        MemoryEntry {
            name: "data.txt".into(),
            file: "data.txt".into(),
            description: Some("test".into()),
            tags: vec![],
            step: None,
            source: "/tmp/data.txt".into(),
            added: Utc::now(),
        },
    );
    save_manifest(tmp.path(), &manifest).unwrap();

    // Simulate delete.
    let mut m = load_manifest(tmp.path()).unwrap();
    let entry = m.entries.remove("1").unwrap();
    std::fs::remove_file(tmp.path().join(&entry.file)).unwrap();
    save_manifest(tmp.path(), &m).unwrap();

    let loaded = load_manifest(tmp.path()).unwrap();
    assert!(loaded.entries.is_empty());
    assert!(!file_path.exists());
}

// =====================================================================
// Search
// =====================================================================

#[test]
fn search_sentence_scope() {
    let content = "The quick brown fox jumps. The lazy dog sleeps. Another sentence here.";
    let results = extract_sentences(content, "lazy dog");
    assert_eq!(results.len(), 1);
    assert!(results[0].text.contains("lazy dog sleeps"));
}

#[test]
fn search_paragraph_scope() {
    let content = "First paragraph about apples.\nStill first paragraph.\n\nSecond paragraph about bananas.\n\nThird paragraph.";
    let results = extract_paragraphs(content, "bananas");
    assert_eq!(results.len(), 1);
    assert!(results[0].text.contains("bananas"));
}

#[test]
fn search_section_scope() {
    let content = "## Introduction\nSome intro text.\n\n## Methods\nWe used bananas.\n\n## Results\nGood results.";
    let results = extract_sections(content, "bananas");
    assert_eq!(results.len(), 1);
    assert!(results[0].text.starts_with("## Methods"));
}

#[test]
fn search_file_scope() {
    let content = "This file contains the word banana somewhere.";
    let results = extract_file(content, "banana");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].line_number, Some(1));
}

#[test]
fn search_no_match() {
    let content = "Nothing relevant here.";
    assert!(extract_sentences(content, "xyz123").is_empty());
    assert!(extract_paragraphs(content, "xyz123").is_empty());
    assert!(extract_sections(content, "xyz123").is_empty());
    assert!(extract_file(content, "xyz123").is_empty());
}

// =====================================================================
// Render memory block
// =====================================================================

#[test]
fn render_empty_block() {
    let block = render_memory_block(&[], "my-wf", Some("step1"));
    assert!(block.is_empty());
}

#[test]
fn render_block_with_entries() {
    let entries = vec![
        (
            PathBuf::from("/tmp/mem/notes.md"),
            "1".to_string(),
            MemoryEntry {
                name: "notes.md".into(),
                file: "notes.md".into(),
                description: Some("Architecture notes".into()),
                tags: vec!["arch".into()],
                step: None,
                source: "/src/notes.md".into(),
                added: Utc::now(),
            },
        ),
        (
            PathBuf::from("/tmp/mem/todo.md"),
            "2".to_string(),
            MemoryEntry {
                name: "todo.md".into(),
                file: "todo.md".into(),
                description: None,
                tags: vec![],
                step: None,
                source: "/src/todo.md".into(),
                added: Utc::now(),
            },
        ),
    ];
    let block = render_memory_block(&entries, "my-wf", Some("analysis"));
    assert!(block.contains("<memory>"));
    assert!(block.contains("</memory>"));
    assert!(block.contains("--workflow my-wf --step analysis"));
    assert!(block.contains("(id: 1) — Architecture notes [arch]"));
    assert!(block.contains("(id: 2, no description — run: zig memory update 2"));
}

#[test]
fn render_block_without_step() {
    let entries = vec![(
        PathBuf::from("/tmp/mem/notes.md"),
        "1".to_string(),
        MemoryEntry {
            name: "notes.md".into(),
            file: "notes.md".into(),
            description: Some("test".into()),
            tags: vec![],
            step: None,
            source: "/src/notes.md".into(),
            added: Utc::now(),
        },
    )];
    let block = render_memory_block(&entries, "my-wf", None);
    assert!(block.contains("--workflow my-wf`"));
    assert!(!block.contains("--step"));
}

// =====================================================================
// MemoryMode
// =====================================================================

#[test]
fn memory_mode_from_str_opt() {
    assert_eq!(MemoryMode::from_str_opt(None), MemoryMode::All);
    assert_eq!(MemoryMode::from_str_opt(Some("all")), MemoryMode::All);
    assert_eq!(MemoryMode::from_str_opt(Some("global")), MemoryMode::Global);
    assert_eq!(MemoryMode::from_str_opt(Some("none")), MemoryMode::None);
    assert_eq!(MemoryMode::from_str_opt(Some("unknown")), MemoryMode::All);
}

// =====================================================================
// MemoryCollector
// =====================================================================

#[test]
fn collector_disabled_returns_empty() {
    let config = ZigConfig::default();
    let collector = MemoryCollector {
        global_shared_dir: None,
        global_workflow_dir: None,
        cwd_memory_dir: None,
        workflow_mode: MemoryMode::All,
        local_enabled: config.memory.local,
        disabled: true,
    };
    let entries = collector.collect_for_step(None).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn collector_none_mode_returns_empty() {
    let config = ZigConfig::default();
    let collector = MemoryCollector {
        global_shared_dir: None,
        global_workflow_dir: None,
        cwd_memory_dir: None,
        workflow_mode: MemoryMode::None,
        local_enabled: config.memory.local,
        disabled: false,
    };
    let entries = collector.collect_for_step(None).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn collector_step_override_to_none() {
    let config = ZigConfig::default();
    let collector = MemoryCollector {
        global_shared_dir: None,
        global_workflow_dir: None,
        cwd_memory_dir: None,
        workflow_mode: MemoryMode::All,
        local_enabled: config.memory.local,
        disabled: false,
    };
    let entries = collector.collect_for_step(Some("none")).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn collector_global_mode_skips_local() {
    let tmp_local = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp_local.path().join("local.md"), "local content").unwrap();
    let mut local_manifest = Manifest {
        next_id: 2,
        entries: BTreeMap::new(),
    };
    local_manifest.entries.insert(
        "1".into(),
        MemoryEntry {
            name: "local.md".into(),
            file: "local.md".into(),
            description: Some("local".into()),
            tags: vec![],
            step: None,
            source: "/tmp/local.md".into(),
            added: Utc::now(),
        },
    );
    save_manifest(tmp_local.path(), &local_manifest).unwrap();

    let collector = MemoryCollector {
        global_shared_dir: None,
        global_workflow_dir: None,
        cwd_memory_dir: Some(tmp_local.path().to_path_buf()),
        workflow_mode: MemoryMode::Global,
        local_enabled: true,
        disabled: false,
    };
    let entries = collector.collect_for_step(None).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn collector_collects_from_tiers() {
    let tmp_shared = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp_shared.path().join("shared.md"), "shared content").unwrap();
    let mut shared_manifest = Manifest {
        next_id: 2,
        entries: BTreeMap::new(),
    };
    shared_manifest.entries.insert(
        "1".into(),
        MemoryEntry {
            name: "shared.md".into(),
            file: "shared.md".into(),
            description: Some("shared".into()),
            tags: vec![],
            step: None,
            source: "/tmp/shared.md".into(),
            added: Utc::now(),
        },
    );
    save_manifest(tmp_shared.path(), &shared_manifest).unwrap();

    let tmp_local = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp_local.path().join("local.md"), "local content").unwrap();
    let mut local_manifest = Manifest {
        next_id: 2,
        entries: BTreeMap::new(),
    };
    local_manifest.entries.insert(
        "1".into(),
        MemoryEntry {
            name: "local.md".into(),
            file: "local.md".into(),
            description: Some("local".into()),
            tags: vec![],
            step: None,
            source: "/tmp/local.md".into(),
            added: Utc::now(),
        },
    );
    save_manifest(tmp_local.path(), &local_manifest).unwrap();

    let collector = MemoryCollector {
        global_shared_dir: Some(tmp_shared.path().to_path_buf()),
        global_workflow_dir: None,
        cwd_memory_dir: Some(tmp_local.path().to_path_buf()),
        workflow_mode: MemoryMode::All,
        local_enabled: true,
        disabled: false,
    };
    let entries = collector.collect_for_step(None).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn collector_local_disabled_skips_cwd() {
    let tmp_local = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp_local.path().join("local.md"), "content").unwrap();
    let mut manifest = Manifest {
        next_id: 2,
        entries: BTreeMap::new(),
    };
    manifest.entries.insert(
        "1".into(),
        MemoryEntry {
            name: "local.md".into(),
            file: "local.md".into(),
            description: None,
            tags: vec![],
            step: None,
            source: "/tmp/local.md".into(),
            added: Utc::now(),
        },
    );
    save_manifest(tmp_local.path(), &manifest).unwrap();

    let collector = MemoryCollector {
        global_shared_dir: None,
        global_workflow_dir: None,
        cwd_memory_dir: Some(tmp_local.path().to_path_buf()),
        workflow_mode: MemoryMode::All,
        local_enabled: false, // global config disables local
        disabled: false,
    };
    let entries = collector.collect_for_step(None).unwrap();
    assert!(entries.is_empty());
}

// =====================================================================
// MemoryTarget
// =====================================================================

#[test]
fn target_from_flags_defaults_to_cwd() {
    let target = MemoryTarget::from_flags(None, false, false).unwrap();
    assert!(matches!(target, MemoryTarget::Cwd));
}

#[test]
fn target_from_flags_workflow() {
    let target = MemoryTarget::from_flags(Some("my-wf"), false, false).unwrap();
    assert!(matches!(target, MemoryTarget::GlobalWorkflow(ref n) if n == "my-wf"));
}

#[test]
fn target_from_flags_global() {
    let target = MemoryTarget::from_flags(None, true, false).unwrap();
    assert!(matches!(target, MemoryTarget::GlobalShared));
}

#[test]
fn target_from_flags_cwd() {
    let target = MemoryTarget::from_flags(None, false, true).unwrap();
    assert!(matches!(target, MemoryTarget::Cwd));
}

#[test]
fn target_from_flags_workflow_and_cwd_conflicts() {
    let result = MemoryTarget::from_flags(Some("wf"), false, true);
    assert!(result.is_err());
}
