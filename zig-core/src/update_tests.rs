use super::*;

use std::fs;

const SIMPLE_WORKFLOW: &str = r#"[workflow]
name = "test-update"
description = "Test workflow for update module"

[[step]]
name = "hello"
prompt = "Say hello"
"#;

#[test]
fn workflow_kind_from_path_detects_zipped() {
    let zipped = Path::new("/tmp/thing.zwfz");
    assert_eq!(WorkflowKind::from_path(zipped), WorkflowKind::Zipped);
}

#[test]
fn workflow_kind_from_path_defaults_to_plain() {
    let plain = Path::new("/tmp/thing.zwf");
    assert_eq!(WorkflowKind::from_path(plain), WorkflowKind::Plain);

    // Unknown extension also treated as plain.
    let other = Path::new("/tmp/thing.toml");
    assert_eq!(WorkflowKind::from_path(other), WorkflowKind::Plain);
}

#[test]
fn prepare_update_plain_copies_file_to_staging() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("simple.zwf");
    fs::write(&original, SIMPLE_WORKFLOW).unwrap();

    let params = prepare_update(original.to_str().unwrap()).unwrap();

    assert_eq!(params.kind, WorkflowKind::Plain);
    assert_eq!(params.original_path, original);
    assert!(params.staging_path.exists(), "staging file should exist");
    assert_ne!(
        params.staging_path, original,
        "staging path should differ from original"
    );

    // Staging file contents should round-trip the original.
    let staged = fs::read_to_string(&params.staging_path).unwrap();
    assert_eq!(staged, SIMPLE_WORKFLOW);
}

#[test]
fn prepare_update_builds_prompts_pointing_at_staging_path() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("prompts-check.zwf");
    fs::write(&original, SIMPLE_WORKFLOW).unwrap();

    let params = prepare_update(original.to_str().unwrap()).unwrap();

    // Initial prompt must reference the staging path so the agent edits in place.
    assert!(
        params
            .initial_prompt
            .contains(params.staging_path.to_str().unwrap()),
        "initial prompt should reference staging path"
    );
    assert!(
        params.initial_prompt.contains("do not rename"),
        "initial prompt should forbid renaming"
    );
    // Initial prompt must include a validation report and the wait-for-instructions rule.
    assert!(
        params.initial_prompt.contains("Validation:"),
        "initial prompt should include a validation report"
    );
    assert!(
        params.initial_prompt.contains("no issues found"),
        "clean workflow should report no validation issues"
    );
    assert!(
        params
            .initial_prompt
            .contains("do not start fixing anything yet"),
        "initial prompt should tell the agent to wait for explicit instructions"
    );

    // System prompt should be fully rendered — no leftover placeholders.
    assert!(!params.system_prompt.contains("{{zwf_format_spec}}"));
    assert!(!params.system_prompt.contains("{{examples_reference}}"));
    assert!(params.system_prompt.contains("revision specialist"));
}

#[test]
fn prepare_update_surfaces_validation_errors_in_prompt() {
    // Workflow parses as valid TOML but fails semantic validation: the step
    // depends on a nonexistent step.
    const BROKEN: &str = r#"[workflow]
name = "broken"
description = "Depends on a missing step"

[[step]]
name = "only-step"
prompt = "Do something"
depends_on = ["nonexistent"]
"#;

    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("broken-validate.zwf");
    fs::write(&original, BROKEN).unwrap();

    let params = prepare_update(original.to_str().unwrap())
        .expect("parser accepts the file; only validation should fail");

    assert!(
        params.initial_prompt.contains("Validation:"),
        "initial prompt should include a validation report"
    );
    assert!(
        params.initial_prompt.contains("nonexistent"),
        "validation report should surface the unknown-step error: {}",
        params.initial_prompt
    );
    assert!(
        params
            .initial_prompt
            .contains("do not start fixing anything yet"),
        "initial prompt should still tell the agent to wait for instructions"
    );
}

#[test]
fn prepare_update_fails_on_invalid_workflow() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("broken.zwf");
    fs::write(&original, "this is not valid toml [[[").unwrap();

    let result = prepare_update(original.to_str().unwrap());
    assert!(result.is_err(), "should reject unparseable workflow");
}

#[test]
fn prepare_update_zipped_unzips_into_staging_dir() {
    use std::io::Write;

    let dir = tempfile::tempdir().unwrap();
    let zipped = dir.path().join("zipped.zwfz");

    // Build a minimal `.zwfz` archive containing a single `.zwf` file.
    let file = fs::File::create(&zipped).unwrap();
    let mut zip_writer = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip_writer.start_file("workflow.zwf", options).unwrap();
    zip_writer.write_all(SIMPLE_WORKFLOW.as_bytes()).unwrap();
    zip_writer.finish().unwrap();

    let params = prepare_update(zipped.to_str().unwrap()).unwrap();

    assert_eq!(params.kind, WorkflowKind::Zipped);
    assert_eq!(params.original_path, zipped);
    assert!(params.staging_path.exists(), "staging file should exist");
    assert_eq!(
        params.staging_path.extension().and_then(|s| s.to_str()),
        Some("zwf")
    );

    let staged = fs::read_to_string(&params.staging_path).unwrap();
    assert_eq!(staged, SIMPLE_WORKFLOW);
}

#[test]
fn commit_update_plain_writes_through_sibling_temp() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("commit-plain.zwf");
    fs::write(&original, SIMPLE_WORKFLOW).unwrap();

    let params = prepare_update(original.to_str().unwrap()).unwrap();

    // Simulate an agent edit in staging.
    let edited = SIMPLE_WORKFLOW.replace("Say hello", "Say hello, world");
    fs::write(&params.staging_path, &edited).unwrap();

    commit_update(&params).unwrap();

    let final_contents = fs::read_to_string(&original).unwrap();
    assert_eq!(final_contents, edited);

    // No sibling temp file left behind.
    let siblings: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        !siblings.iter().any(|n| n.contains(".update.")),
        "no leftover sibling temp file, found: {siblings:?}"
    );
}

#[test]
fn commit_update_zipped_repacks_into_place() {
    use std::io::Write;

    let dir = tempfile::tempdir().unwrap();
    let zipped = dir.path().join("commit-zipped.zwfz");

    let file = fs::File::create(&zipped).unwrap();
    let mut zip_writer = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip_writer.start_file("workflow.zwf", options).unwrap();
    zip_writer.write_all(SIMPLE_WORKFLOW.as_bytes()).unwrap();
    zip_writer.finish().unwrap();

    let params = prepare_update(zipped.to_str().unwrap()).unwrap();

    let edited = SIMPLE_WORKFLOW.replace("Say hello", "Say hello, world");
    fs::write(&params.staging_path, &edited).unwrap();

    commit_update(&params).unwrap();

    // Re-parse the archive and verify the edit round-tripped.
    let (wf, _source) = crate::workflow::parser::parse_workflow(&zipped).unwrap();
    assert_eq!(wf.workflow.name, "test-update");
    assert_eq!(wf.steps.len(), 1);
    assert_eq!(wf.steps[0].prompt, "Say hello, world");
}
