use assert_cmd::{Command, cargo, pkg_name};
use assertables::assert_contains;
use std::io::Write;
use svgdx::Result;
use svgdx::cli::{CliAction, parse_args};
use tempfile::NamedTempFile;

/// Create a `Config` object set up given a command line string.
///
/// The string is parsed using `shlex::split()`, so values containing
/// spaces or quotes should be quoted or escaped appropriately.
pub fn from_cmdline(args: &str) -> Result<CliAction> {
    let args = shlex::split(args).unwrap_or_default();
    parse_args(args.into_iter()) //.map_err(Error::from_err)
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

    let mut tmpfile = NamedTempFile::new().expect("could not create tmpfile");
    write!(tmpfile, r#"<svg><rect xy="0" wh="1"/></svg>"#).expect("tmpfile write failed");
    let config = from_cmdline(&format!(
        "{} {}",
        pkg_name!(),
        tmpfile.path().to_str().unwrap(),
    ))
    .expect("cmdline should be valid");
    svgdx::cli::run(config).expect("run failed");

    let mut tmpfile = NamedTempFile::new().expect("could not create tmpfile");
    write!(tmpfile, r#"<svg><rect xy="0" wh="1"/></svg>"#).expect("tmpfile write failed");
    let outfile = NamedTempFile::new().expect("could not create outfile");
    let config = from_cmdline(&format!(
        "{} -o {} {}",
        pkg_name!(),
        outfile.path().to_str().unwrap(),
        tmpfile.path().to_str().unwrap(),
    ))
    .expect("cmdline should be valid");
    svgdx::cli::run(config).expect("run failed");
}

#[test]
fn test_cmdline_same_file() {
    let mut tmpfile = NamedTempFile::new().expect("could not create tmpfile");
    write!(tmpfile, r#"<svg><rect xy="0" wh="1"/></svg>"#).expect("tmpfile write failed");

    let cwd = tmpfile.path().parent().unwrap();
    let filename = tmpfile.path().file_name().unwrap();

    // Different files: should succeed
    let mut cmd = Command::new(cargo::cargo_bin!());
    cmd.current_dir(cwd)
        .args([
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
            filename.to_str().unwrap(),
            "-o",
            &format!("./{}", filename.to_str().unwrap()),
        ])
        .assert()
        .failure()
        .code(1);
}
