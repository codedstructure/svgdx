use assert_cmd::{crate_name, Command};
use std::io::Write;
use svgd::Config;
use tempfile::NamedTempFile;

#[test]
fn test_cmdline_no_args() {
    let mut cmd = Command::cargo_bin(crate_name!()).unwrap();
    cmd.assert().failure().code(2);
}

#[test]
fn test_cmdline_help() {
    let mut cmd = Command::cargo_bin(crate_name!()).unwrap();
    let output = String::from_utf8(cmd.arg("-h").assert().success().get_output().stdout.clone())
        .expect("non-UTF8");
    assert!(output.contains("Usage"));
}

#[test]
fn test_cmdline_config() {
    let config = Config::from_args(&format!("{} --help", crate_name!()));
    assert!(config.is_err());

    let mut tmpfile = NamedTempFile::new().expect("could not create tmpfile");
    write!(tmpfile, r#"<svg><rect xy="0" wh="1"/></svg>"#).expect("tmpfile write failed");
    let config = Config::from_args(&format!(
        "{} -i {}",
        crate_name!(),
        tmpfile.path().to_str().unwrap()
    ))
    .expect("cmdline should be valid");
    svgd::run(config).expect("run failed");

    let mut tmpfile = NamedTempFile::new().expect("could not create tmpfile");
    write!(tmpfile, r#"<svg><rect xy="0" wh="1"/></svg>"#).expect("tmpfile write failed");
    let outfile = NamedTempFile::new().expect("could not create outfile");
    let config = Config::from_args(&format!(
        "{} -i {} -o {}",
        crate_name!(),
        tmpfile.path().to_str().unwrap(),
        outfile.path().to_str().unwrap()
    ))
    .expect("cmdline should be valid");
    svgd::run(config).expect("run failed");
}
