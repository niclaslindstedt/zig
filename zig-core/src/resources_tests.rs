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
    let set = collect_resources(&[], &[], tmp.path()).unwrap();
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
}

#[test]
fn collect_bare_path_resolves_relative_to_workflow_dir() {
    let tmp = tempfile::TempDir::new().unwrap();
    make_file(tmp.path(), "cv.md", "CV content");

    let workflow_specs = vec![ResourceSpec::Path("./cv.md".into())];
    let set = collect_resources(&workflow_specs, &[], tmp.path()).unwrap();

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
    let set = collect_resources(&workflow_specs, &[], tmp.path()).unwrap();

    let res = set.iter().next().unwrap();
    assert_eq!(res.name, "style-guide");
    assert_eq!(res.description.as_deref(), Some("House writing style"));
}

#[test]
fn collect_missing_optional_file_is_skipped_with_warning() {
    let tmp = tempfile::TempDir::new().unwrap();

    let workflow_specs = vec![ResourceSpec::Path("./not-there.md".into())];
    let set = collect_resources(&workflow_specs, &[], tmp.path()).unwrap();
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
    let err = collect_resources(&workflow_specs, &[], tmp.path()).unwrap_err();
    assert!(err.to_string().contains("required resource"));
}

#[test]
fn collect_step_resources_append_to_workflow_resources() {
    let tmp = tempfile::TempDir::new().unwrap();
    make_file(tmp.path(), "cv.md", "CV");
    make_file(tmp.path(), "job.md", "Job");

    let workflow_specs = vec![ResourceSpec::Path("cv.md".into())];
    let step_specs = vec![ResourceSpec::Path("job.md".into())];
    let set = collect_resources(&workflow_specs, &step_specs, tmp.path()).unwrap();

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
    let set = collect_resources(&workflow_specs, &step_specs, tmp.path()).unwrap();

    assert_eq!(set.len(), 1);
    // First occurrence wins → origin is Workflow.
    assert_eq!(set.iter().next().unwrap().origin, ResourceOrigin::Workflow);
}

#[test]
fn collect_rejects_empty_path() {
    let tmp = tempfile::TempDir::new().unwrap();
    let specs = vec![ResourceSpec::Path("".into())];
    let err = collect_resources(&specs, &[], tmp.path()).unwrap_err();
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
    let set = collect_resources(&specs, &[], tmp.path()).unwrap();
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
    let set = collect_resources(&specs, &[], tmp.path()).unwrap();
    let block = render_system_block(&set);

    assert!(block.contains("(cv.md)"));
    assert!(!block.contains(" — "));
}
