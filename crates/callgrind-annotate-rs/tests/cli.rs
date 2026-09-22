use std::process::Command;

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_callgrind-annotate-rs"))
}
fn fixture() -> String {
    format!(
        "{}/tests/fixtures/source.callgrind",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[test]
fn help_and_version() {
    for arg in ["--help", "--version"] {
        let out = command().arg(arg).output().unwrap();
        assert!(out.status.success());
        assert!(
            String::from_utf8(out.stdout)
                .unwrap()
                .contains("callgrind-annotate-rs")
        );
    }
}
#[test]
fn text_and_tsv_are_real_reports() {
    let text = command()
        .args(["--auto=no", "--threshold=100", &fixture()])
        .output()
        .unwrap();
    assert!(text.status.success());
    let text = String::from_utf8(text.stdout).unwrap();
    assert!(text.contains("PROGRAM TOTALS"));
    assert!(text.contains("source.c:main"));
    assert!(!text.contains("stub"));
    let tsv = command()
        .args(["--format=tsv", "--threshold=100", &fixture()])
        .output()
        .unwrap();
    assert!(tsv.status.success());
    let tsv = String::from_utf8(tsv.stdout).unwrap();
    assert!(tsv.starts_with("P\t0\t4972\nT\t0\t15\n"));
    assert!(tsv.contains("E\t0\t"));
    assert!(tsv.contains("S\t0\t"));
}
#[test]
fn bad_arguments_and_missing_inputs_return_nonzero() {
    for args in [vec!["--tree=bad"], vec!["no-such-callgrind-profile"]] {
        let out = command().args(args).output().unwrap();
        assert!(!out.status.success());
        assert!(!out.stderr.is_empty());
    }
}
#[test]
fn unsupported_event_fails_before_report_output() {
    let out = command()
        .args(["--show=missing", &fixture()])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .starts_with("callgrind-annotate-rs: unknown or unrecorded event")
    );
}
#[test]
fn multipart_requires_part_and_explicit_selection_succeeds() {
    let path = format!(
        "{}/../../tests/reference/fixtures/event-remapping.callgrind",
        env!("CARGO_MANIFEST_DIR")
    );
    assert!(!command().arg(&path).output().unwrap().status.success());
    let out = command()
        .args(["--auto=no", "--part=1", &path])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("Events recorded:  Dr Ir Dw")
    );
}
