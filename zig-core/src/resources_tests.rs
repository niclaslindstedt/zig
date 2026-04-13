use super::*;
use crate::workflow::model::ResourceSpec;

fn make_file(dir: &std::path::Path, name: &str, contents: &str) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, contents).unwrap();
    p
}

#[test]
fn collect_empty_when_nothing_declared() {
    let tmp = tempfile::TempDir::new().unwrap();
    let set = collect_inline_resources(&[], &[], tmp.path()).unwrap();
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
}

#[test]
fn collect_bare_path_resolves_relative_to_workflow_dir() {
    let tmp = tempfile::TempDir::new().unwrap();
    make_file(tmp.path(), "cv.md", "CV content");

    let workflow_specs = vec![ResourceSpec::Path("./cv.md".into())];
    let set = collect_inline_resources(&workflow_specs, &[], tmp.path()).unwrap();

    assert_eq!(set.len(), 1);
    let res = set.iter().next().unwrap();
    assert!(res.abs_path.ends_with("cv.md"));
    assert!(res.abs_path.is_absolute());
    assert_eq!(res.name, "cv.md");
    assert!(res.description.is_none());
    assert_eq!(res.origin, ResourceOrigin::Workflow);
}

#[test]
fn collect_detailed_form_uses_name_and_description() {
    let tmp = tempfile::TempDir::new().unwrap();
    make_file(tmp.path(), "style.md", "Style guide");

    let workflow_specs = vec![ResourceSpec::Detailed {
        path: "style.md".into(),
        name: Some("style-guide".into()),
        description: Some("House writing style".into()),
        required: false,
    }];
    let set = collect_inline_resources(&workflow_specs, &[], tmp.path()).unwrap();

    let res = set.iter().next().unwrap();
    assert_eq!(res.name, "style-guide");
    assert_eq!(res.description.as_deref(), Some("House writing style"));
}

#[test]
fn collect_missing_optional_file_is_skipped_with_warning() {
    let tmp = tempfile::TempDir::new().unwrap();

    let workflow_specs = vec![ResourceSpec::Path("./not-there.md".into())];
    let set = collect_inline_resources(&workflow_specs, &[], tmp.path()).unwrap();
    assert!(set.is_empty());
}

#[test]
fn collect_missing_required_file_errors() {
    let tmp = tempfile::TempDir::new().unwrap();

    let workflow_specs = vec![ResourceSpec::Detailed {
        path: "./not-there.md".into(),
        name: None,
        description: None,
        required: true,
    }];
    let err = collect_inline_resources(&workflow_specs, &[], tmp.path()).unwrap_err();
    assert!(err.to_string().contains("required resource"));
}

#[test]
fn collect_step_resources_append_to_workflow_resources() {
    let tmp = tempfile::TempDir::new().unwrap();
    make_file(tmp.path(), "cv.md", "CV");
    make_file(tmp.path(), "job.md", "Job");

    let workflow_specs = vec![ResourceSpec::Path("cv.md".into())];
    let step_specs = vec![ResourceSpec::Path("job.md".into())];
    let set = collect_inline_resources(&workflow_specs, &step_specs, tmp.path()).unwrap();

    assert_eq!(set.len(), 2);
    let mut iter = set.iter();
    assert_eq!(iter.next().unwrap().origin, ResourceOrigin::Workflow);
    assert_eq!(iter.next().unwrap().origin, ResourceOrigin::Step);
}

#[test]
fn collect_dedupes_same_file_referenced_twice() {
    let tmp = tempfile::TempDir::new().unwrap();
    make_file(tmp.path(), "cv.md", "CV");

    let workflow_specs = vec![ResourceSpec::Path("cv.md".into())];
    let step_specs = vec![ResourceSpec::Path("./cv.md".into())];
    let set = collect_inline_resources(&workflow_specs, &step_specs, tmp.path()).unwrap();

    assert_eq!(set.len(), 1);
    // First occurrence wins → origin is Workflow.
    assert_eq!(set.iter().next().unwrap().origin, ResourceOrigin::Workflow);
}

#[test]
fn collect_rejects_empty_path() {
    let tmp = tempfile::TempDir::new().unwrap();
    let specs = vec![ResourceSpec::Path("".into())];
    let err = collect_inline_resources(&specs, &[], tmp.path()).unwrap_err();
    assert!(err.to_string().contains("empty path"));
}

#[test]
fn render_system_block_empty_set_returns_empty_string() {
    let set = ResourceSet::new();
    assert_eq!(render_system_block(&set), "");
}

#[test]
fn render_system_block_includes_absolute_paths_and_descriptions() {
    let tmp = tempfile::TempDir::new().unwrap();
    make_file(tmp.path(), "cv.md", "CV");

    let specs = vec![ResourceSpec::Detailed {
        path: "cv.md".into(),
        name: Some("cv".into()),
        description: Some("Candidate CV".into()),
        required: false,
    }];
    let set = collect_inline_resources(&specs, &[], tmp.path()).unwrap();
    let block = render_system_block(&set);

    assert!(block.starts_with("<resources>"));
    assert!(block.trim_end().ends_with("</resources>"));
    assert!(block.contains("— Candidate CV"));
    // The rendered path should be absolute.
    let abs = std::fs::canonicalize(tmp.path().join("cv.md")).unwrap();
    assert!(block.contains(&abs.display().to_string()));
}

#[test]
fn render_system_block_falls_back_to_name_when_no_description() {
    let tmp = tempfile::TempDir::new().unwrap();
    make_file(tmp.path(), "cv.md", "CV");

    let specs = vec![ResourceSpec::Path("cv.md".into())];
    let set = collect_inline_resources(&specs, &[], tmp.path()).unwrap();
    let block = render_system_block(&set);

    assert!(block.contains("(cv.md)"));
    assert!(!block.contains(" — "));
}

// ── ResourceCollector ───────────────────────────────────────────────────────

#[test]
fn collector_disabled_returns_empty_set() {
    let tmp = tempfile::TempDir::new().unwrap();
    let collector = ResourceCollector {
        workflow_resources: &[],
        workflow_dir: tmp.path(),
        global_shared_dir: None,
        global_workflow_dir: None,
        cwd_resources_dir: None,
        disabled: true,
    };
    let set = collector.collect_for_step(&[]).unwrap();
    assert!(set.is_empty());
}

#[test]
fn collector_walks_global_shared_directory() {
    let shared = tempfile::TempDir::new().unwrap();
    std::fs::write(shared.path().join("style.md"), "Style").unwrap();

    let workflow_dir = tempfile::TempDir::new().unwrap();
    let collector = ResourceCollector {
        workflow_resources: &[],
        workflow_dir: workflow_dir.path(),
        global_shared_dir: Some(shared.path().to_path_buf()),
        global_workflow_dir: None,
        cwd_resources_dir: None,
        disabled: false,
    };

    let set = collector.collect_for_step(&[]).unwrap();
    assert_eq!(set.len(), 1);
    let res = set.iter().next().unwrap();
    assert_eq!(res.name, "style.md");
    assert_eq!(res.origin, ResourceOrigin::GlobalShared);
}

#[test]
fn collector_walks_global_workflow_directory() {
    let global = tempfile::TempDir::new().unwrap();
    std::fs::write(global.path().join("cv.md"), "CV").unwrap();

    let workflow_dir = tempfile::TempDir::new().unwrap();
    let collector = ResourceCollector {
        workflow_resources: &[],
        workflow_dir: workflow_dir.path(),
        global_shared_dir: None,
        global_workflow_dir: Some(global.path().to_path_buf()),
        cwd_resources_dir: None,
        disabled: false,
    };

    let set = collector.collect_for_step(&[]).unwrap();
    assert_eq!(set.len(), 1);
    let res = set.iter().next().unwrap();
    assert_eq!(res.name, "cv.md");
    assert_eq!(res.origin, ResourceOrigin::GlobalWorkflow);
}

#[test]
fn collector_walks_cwd_resources_directory() {
    let cwd = tempfile::TempDir::new().unwrap();
    std::fs::write(cwd.path().join("notes.md"), "Notes").unwrap();

    let workflow_dir = tempfile::TempDir::new().unwrap();
    let collector = ResourceCollector {
        workflow_resources: &[],
        workflow_dir: workflow_dir.path(),
        global_shared_dir: None,
        global_workflow_dir: None,
        cwd_resources_dir: Some(cwd.path().to_path_buf()),
        disabled: false,
    };

    let set = collector.collect_for_step(&[]).unwrap();
    assert_eq!(set.len(), 1);
    assert_eq!(set.iter().next().unwrap().origin, ResourceOrigin::Cwd);
}

#[test]
fn collector_recursively_scans_subdirectories() {
    let shared = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(shared.path().join("subdir")).unwrap();
    std::fs::write(shared.path().join("a.md"), "A").unwrap();
    std::fs::write(shared.path().join("subdir").join("b.md"), "B").unwrap();

    let workflow_dir = tempfile::TempDir::new().unwrap();
    let collector = ResourceCollector {
        workflow_resources: &[],
        workflow_dir: workflow_dir.path(),
        global_shared_dir: Some(shared.path().to_path_buf()),
        global_workflow_dir: None,
        cwd_resources_dir: None,
        disabled: false,
    };

    let set = collector.collect_for_step(&[]).unwrap();
    assert_eq!(set.len(), 2);
}

#[test]
fn collector_merges_all_tiers_in_declared_order() {
    let shared = tempfile::TempDir::new().unwrap();
    let global = tempfile::TempDir::new().unwrap();
    let cwd = tempfile::TempDir::new().unwrap();
    std::fs::write(shared.path().join("shared.md"), "S").unwrap();
    std::fs::write(global.path().join("global.md"), "G").unwrap();
    std::fs::write(cwd.path().join("cwd.md"), "C").unwrap();

    let workflow_dir = tempfile::TempDir::new().unwrap();
    std::fs::write(workflow_dir.path().join("inline.md"), "I").unwrap();
    std::fs::write(workflow_dir.path().join("step.md"), "ST").unwrap();

    let inline_specs = vec![ResourceSpec::Path("inline.md".into())];
    let collector = ResourceCollector {
        workflow_resources: &inline_specs,
        workflow_dir: workflow_dir.path(),
        global_shared_dir: Some(shared.path().to_path_buf()),
        global_workflow_dir: Some(global.path().to_path_buf()),
        cwd_resources_dir: Some(cwd.path().to_path_buf()),
        disabled: false,
    };

    let step_specs = vec![ResourceSpec::Path("step.md".into())];
    let set = collector.collect_for_step(&step_specs).unwrap();

    let origins: Vec<ResourceOrigin> = set.iter().map(|r| r.origin).collect();
    assert_eq!(
        origins,
        vec![
            ResourceOrigin::GlobalShared,
            ResourceOrigin::GlobalWorkflow,
            ResourceOrigin::Cwd,
            ResourceOrigin::Workflow,
            ResourceOrigin::Step,
        ]
    );
}

#[test]
fn collector_dedupes_across_tiers_first_wins() {
    let shared = tempfile::TempDir::new().unwrap();
    let workflow_dir = tempfile::TempDir::new().unwrap();
    std::fs::write(workflow_dir.path().join("cv.md"), "CV").unwrap();
    // Symlink the inline file under the shared dir so both tiers point at the
    // same canonical path.
    let symlink_target = workflow_dir.path().join("cv.md");
    let symlink_path = shared.path().join("cv.md");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&symlink_target, &symlink_path).unwrap();
    #[cfg(not(unix))]
    std::fs::copy(&symlink_target, &symlink_path).unwrap();

    let inline_specs = vec![ResourceSpec::Path("cv.md".into())];
    let collector = ResourceCollector {
        workflow_resources: &inline_specs,
        workflow_dir: workflow_dir.path(),
        global_shared_dir: Some(shared.path().to_path_buf()),
        global_workflow_dir: None,
        cwd_resources_dir: None,
        disabled: false,
    };
    let set = collector.collect_for_step(&[]).unwrap();

    #[cfg(unix)]
    {
        assert_eq!(set.len(), 1);
        assert_eq!(
            set.iter().next().unwrap().origin,
            ResourceOrigin::GlobalShared
        );
    }
    #[cfg(not(unix))]
    {
        assert_eq!(set.len(), 2);
    }
}
