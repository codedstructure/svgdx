use assert_cmd::{crate_name, Command};
use assertables::{assert_contains, assert_contains_as_result};
use std::io::Write;
use svgdx::cli::Config;
use tempfile::NamedTempFile;

#[test]
fn test_cmdline_bad_args() {
    let mut cmd = Command::cargo_bin(crate_name!()).unwrap();
    // -w without an input file should fail
    cmd.arg("-w").assert().failure().code(2);
}

#[test]
fn test_cmdline_help() {
    let mut cmd = Command::cargo_bin(crate_name!()).unwrap();
    let output = String::from_utf8(cmd.arg("-h").assert().success().get_output().stdout.clone())
        .expect("non-UTF8");
    assert_contains!(output, "Usage");
}

#[test]
fn test_cmdline_config() {
    let config = Config::from_cmdline(&format!("{} --help", crate_name!()));
    assert!(config.is_err());

    let mut tmpfile = NamedTempFile::new().expect("could not create tmpfile");
    write!(tmpfile, r#"<svg><rect xy="0" wh="1"/></svg>"#).expect("tmpfile write failed");
    let config = Config::from_cmdline(&format!(
        "{} {}",
        crate_name!(),
        tmpfile.path().to_str().unwrap(),
    ))
    .expect("cmdline should be valid");
    svgdx::cli::run(config).expect("run failed");

    let mut tmpfile = NamedTempFile::new().expect("could not create tmpfile");
    write!(tmpfile, r#"<svg><rect xy="0" wh="1"/></svg>"#).expect("tmpfile write failed");
    let outfile = NamedTempFile::new().expect("could not create outfile");
    let config = Config::from_cmdline(&format!(
        "{} -o {} {}",
        crate_name!(),
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
    let mut cmd = Command::cargo_bin(crate_name!()).unwrap();
    cmd.current_dir(cwd)
        .args([
            filename.to_str().unwrap(),
            "-o",
            &format!("./{}x", filename.to_str().unwrap()),
        ])
        .assert()
        .success();

    // Same file (even with different 'spelling') - should fail.
    let mut cmd = Command::cargo_bin(crate_name!()).unwrap();
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
