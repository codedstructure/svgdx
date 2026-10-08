use crate::assert_contains;
use crate::cli::{cargo_bin, run_cmd};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

const SAMPLE_SVGDX: &str = r#"<svg><rect xy="0" wh="10"/></svg>"#;

#[test]
fn test_output_pandoc_filter() {
    let mut cmd = Command::new(cargo_bin());
    cmd.arg("--output-pandoc-filter");
    let (code, output) = run_cmd(&mut cmd).unwrap();
    assert_eq!(code, 0);

    assert_contains!(output, "function CodeBlock(block)");
    assert_contains!(output, "SVGDX_BIN");
}

#[test]
#[ignore = "requires pandoc"]
fn test_pandoc_filter_inline_svg() {
    let doc_dir = TestTempDir::new("doc-inline");
    let filter_path = export_filter(&doc_dir, "svgdx-pandoc.lua");
    let markdown_path = doc_dir.path().join("inline.md");

    write_file(&markdown_path, &format!("```svgdx\n{SAMPLE_SVGDX}\n```\n"));

    let output = render_markdown(&markdown_path, &filter_path, doc_dir.path());
    assert_contains!(output, "<svg");
}

#[test]
#[ignore = "requires pandoc"]
fn test_pandoc_filter_svg_output() {
    let doc_dir = TestTempDir::new("doc-svg");
    let cwd_dir = TestTempDir::new("cwd-svg");
    let filter_path = export_filter(&doc_dir, "svgdx-pandoc.lua");
    let markdown_path = doc_dir.path().join("nested").join("diagram.md");
    let output_name = PathBuf::from("images/rendered.svg");
    let expected_output = cwd_dir.path().join(&output_name);

    write_file(
        &markdown_path,
        &format!(
            "```svgdx {{filename=\"{}\"}}\n{SAMPLE_SVGDX}\n```\n",
            output_name.display(),
        ),
    );

    let output = render_markdown(&markdown_path, &filter_path, cwd_dir.path());

    assert_contains!(output, r#"src="images/rendered.svg""#);
    assert!(
        expected_output.exists(),
        "expected output file at {:?}",
        expected_output
    );

    let rendered = fs::read_to_string(&expected_output).expect("could not read rendered svg");
    assert_contains!(rendered, "<svg");
}

#[test]
#[ignore = "requires pandoc and rsvg-convert"]
fn test_pandoc_filter_png_output() {
    let doc_dir = TestTempDir::new("doc-png");
    let cwd_dir = TestTempDir::new("cwd-png");
    let filter_path = export_filter(&doc_dir, "svgdx-pandoc.lua");
    let markdown_path = doc_dir.path().join("nested").join("diagram.md");
    let output_name = PathBuf::from("images/rendered.png");
    let expected_output = cwd_dir.path().join(&output_name);

    write_file(
        &markdown_path,
        &format!(
            "```{{.svgdx filename=\"{}\"}}\n{}\n```\n",
            output_name.display(),
            SAMPLE_SVGDX
        ),
    );

    let output = render_markdown(&markdown_path, &filter_path, cwd_dir.path());

    assert_contains!(output, "src=\"images/rendered.png\"");
    assert!(expected_output.exists());

    let rendered = fs::metadata(&expected_output).expect("could not stat rendered png");
    assert!(rendered.len() > 0, "expected non-empty png output");
}

fn export_filter(dir: &TestTempDir, name: &str) -> PathBuf {
    let path = dir.path().join(name);
    let mut cmd = Command::new(cargo_bin());
    cmd.arg("--output-pandoc-filter");
    let (code, output) = run_cmd(&mut cmd).expect("failed to run pandoc filter command");
    assert_eq!(code, 0);
    fs::write(&path, output).expect("could not write pandoc filter");
    path
}

fn render_markdown(markdown_path: &Path, filter_path: &Path, working_dir: &Path) -> String {
    let mut cmd = Command::new("pandoc");
    cmd.current_dir(working_dir)
        .env("SVGDX_BIN", cargo_bin())
        .arg("--from")
        .arg("markdown")
        .arg("--to")
        .arg("html")
        .arg("--lua-filter")
        .arg(filter_path)
        .arg(markdown_path);
    let (code, output) = run_cmd(&mut cmd).unwrap();
    assert_eq!(code, 0, "pandoc command failed");
    output
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("could not create parent directories");
    }
    fs::write(path, content).expect("could not write file");
}

struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "svgdx-test-{prefix}-{}-{}",
            std::process::id(),
            TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path).expect("could not create temp dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
