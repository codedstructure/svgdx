use assert_cmd::{Command, cargo, pkg_name};
use assertables::assert_contains;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use svgdx::Result;
use svgdx::cli::{CliAction, parse_args};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TestTempFile {
    path: PathBuf,
}

impl TestTempFile {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "svgdx-test-{}-{}.tmp",
            std::process::id(),
            TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        File::create(&path).expect("could not create tmpfile");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestTempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Create a `Config` object set up given a command line string.
///
/// The string is parsed using `shlex::split()`, so values containing
/// spaces or quotes should be quoted or escaped appropriately.
pub fn from_cmdline(args: &str) -> Result<CliAction> {
    let args = shlex::split(args).unwrap_or_default();
    parse_args(args)
}

#[test]
fn test_cmdline_bad_args() {
    let mut cmd = Command::new(cargo::cargo_bin!());
    cmd.arg("-zyx").assert().failure().code(2);
}

#[test]
fn test_cmdline_help() {
    let mut cmd = Command::new(cargo::cargo_bin!());
    let output = String::from_utf8(cmd.arg("-h").assert().success().get_output().stdout.clone())
        .expect("non-UTF8");
    assert_contains!(output, "Usage");
}

#[test]
fn test_cmdline_config() {
    let config = from_cmdline(&format!("{} --help", pkg_name!()));
    assert!(matches!(config, Ok(CliAction::Help)));

    let mut tmpfile = TestTempFile::new();
    write!(tmpfile, r#"<svg><rect xy="0" wh="1"/></svg>"#).expect("tmpfile write failed");
    let config = from_cmdline(&format!(
        "{} -i {}",
        pkg_name!(),
        tmpfile.path().to_str().unwrap(),
    ))
    .expect("cmdline should be valid");
    svgdx::cli::run(config, "test").expect("run failed");

    let mut tmpfile = TestTempFile::new();
    write!(tmpfile, r#"<svg><rect xy="0" wh="1"/></svg>"#).expect("tmpfile write failed");
    let outfile = TestTempFile::new();
    let config = from_cmdline(&format!(
        "{} -i {} -o {}",
        pkg_name!(),
        tmpfile.path().to_str().unwrap(),
        outfile.path().to_str().unwrap(),
    ))
    .expect("cmdline should be valid");
    svgdx::cli::run(config, "test").expect("run failed");
}

#[test]
fn test_cmdline_same_file() {
    let mut tmpfile = TestTempFile::new();
    write!(tmpfile, r#"<svg><rect xy="0" wh="1"/></svg>"#).expect("tmpfile write failed");

    let cwd = tmpfile.path().parent().unwrap();
    let filename = tmpfile.path().file_name().unwrap();

    // Different files: should succeed
    let mut cmd = Command::new(cargo::cargo_bin!());
    cmd.current_dir(cwd)
        .args([
            "-i",
            filename.to_str().unwrap(),
            "-o",
            &format!("./{}x", filename.to_str().unwrap()),
        ])
        .assert()
        .success();

    // Same file (even with different 'spelling') - should fail.
    let mut cmd = Command::new(cargo::cargo_bin!());
    cmd.current_dir(cwd)
        .args([
            "-i",
            filename.to_str().unwrap(),
            "-o",
            &format!("./{}", filename.to_str().unwrap()),
        ])
        .assert()
        .failure()
        .code(1);
}

impl Write for TestTempFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        File::options().append(true).open(&self.path)?.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        File::options().append(true).open(&self.path)?.flush()
    }
}
