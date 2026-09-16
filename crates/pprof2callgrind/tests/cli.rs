use pprof_profile::proto::*;
use std::{
    io::Write,
    process::{Command, Stdio},
};

fn profile(value: i64) -> Vec<u8> {
    let p = Profile {
        string_table: vec!["".into(), "samples".into()],
        sample_type: vec![ValueType { r#type: 1, unit: 0 }],
        location: vec![Location {
            id: 1,
            ..Default::default()
        }],
        sample: vec![Sample {
            location_id: vec![1],
            value: vec![value],
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut bytes = Vec::new();
    pprof_profile::write(&p, &mut bytes).unwrap();
    bytes
}
fn run(input: &[u8], args: &[&str]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_pprof2callgrind"))
        .arg("-")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}
#[test]
fn stdin_stdout_and_errors() {
    for mode in ["graph", "tree"] {
        let out = run(&profile(9007199254740993), &["--mode", mode]);
        assert!(out.status.success());
        let p = callgrind_parser::parse_profile(std::str::from_utf8(&out.stdout).unwrap()).unwrap();
        assert_eq!(p.parts[0].totals.as_deref(), Some(&[9007199254740993][..]));
        assert!(String::from_utf8(out.stderr).unwrap().contains("unknown"));
    }
    for (bytes, args) in [
        (profile(-1), vec![]),
        (vec![255], vec![]),
        (profile(1), vec!["--max-input-bytes", "1"]),
    ] {
        let out = run(&bytes, &args);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
    }
}
#[test]
fn refuses_overwrite_and_validates_before_creating_output() {
    let dir = std::env::temp_dir().join(format!("pprof2callgrind-test-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("output");
    let pathstr = path.to_str().unwrap();
    assert!(!run(&profile(-1), &["-o", pathstr]).status.success());
    assert!(!path.exists());
    assert!(run(&profile(9), &["-o", pathstr]).status.success());
    let original = std::fs::read(&path).unwrap();
    assert!(!run(&profile(10), &["-o", pathstr]).status.success());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(&dir).unwrap();
}
