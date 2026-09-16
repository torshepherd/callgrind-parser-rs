use std::{
    io::Write,
    process::{Command, Stdio},
};
fn run(text: &str, args: &[&str]) -> std::process::Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_callgrind2pprof"))
        .arg("-")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    c.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
    c.wait_with_output().unwrap()
}
#[test]
fn stdin_gzip_stdout_and_argument_failures() {
    let text = "events: Ir sysTime\n1 9007199254740993 1500\n";
    let r = run(text, &["--unit", "sysTime=microseconds"]);
    assert!(r.status.success());
    assert!(r.stdout.starts_with(&[31, 139]));
    let p = pprof_profile::read(r.stdout.as_slice(), 4096).unwrap();
    assert_eq!(p.sample[0].value, [9007199254740993, 1500]);
    for args in [
        vec!["--unit", "Ir"],
        vec!["--unit", "Ir=count", "--unit", "Ir=bytes"],
        vec!["--part", "1"],
        vec!["--max-input-bytes", "1"],
        vec!["--max-input-bytes", "0"],
        vec!["--unit", "missing=count"],
    ] {
        let r = run(text, &args);
        assert!(!r.status.success());
        assert!(r.stdout.is_empty());
    }
    assert!(!run("events: Ir\n1 -3\n", &[]).status.success());
}
#[test]
fn validation_precedes_creation_and_existing_output_is_preserved() {
    let dir = std::env::temp_dir().join(format!("callgrind2pprof-test-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("out.pb.gz");
    let args = ["-o", path.to_str().unwrap()];
    assert!(
        !run("events: Ir\n1 9223372036854775808\n", &args)
            .status
            .success()
    );
    assert!(!path.exists());
    assert!(run("events: Ir\n1 9\n", &args).status.success());
    let before = std::fs::read(&path).unwrap();
    assert!(!run("events: Ir\n1 10\n", &args).status.success());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
