use callgrind_annotate::{Analysis, Options, Selection, render};
use callgrind_parser::parse_profile;
use clap::Parser;
use std::path::PathBuf;

fn output(args: &[&str]) -> Result<String, String> {
    let p = parse_profile(include_str!("fixtures/source.callgrind")).unwrap();
    let a = Analysis::build(&p, &p.parts[0]).unwrap();
    let mut o = Options::try_parse_from(
        ["annotate", "--threshold=100", "--show-percs=no"]
            .into_iter()
            .chain(args.iter().copied()),
    )
    .unwrap();
    o.include
        .push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"));
    let s = Selection::build(&a, &o).unwrap();
    render(&p, &p.parts[0], &a, &s, &o)
}

#[test]
fn auto_source_context_calls_and_unknown_lines() {
    let out = output(&["--context=0"]).unwrap();
    assert!(out.contains("Auto-annotated source:"));
    assert!(out.contains("-- lines 3-4"));
    assert!(out.contains("3  int x = work();") || out.contains("3    int x = work();"));
    assert!(out.contains("=> source.c:work (2x)"));
    assert!(out.contains("=> source.c:work (0x)"));
    assert!(out.contains("<bogus line 99>"));
    assert!(out.contains("<counts for unidentified lines in source.c>"));
    assert!(out.contains("10  events annotated"));
    assert!(!out.contains("// source fixture"));
}
#[test]
fn context_merges_overlapping_ranges_once() {
    let out = output(&["--context=8"]).unwrap();
    assert_eq!(out.matches("// source fixture").count(), 1);
    assert_eq!(out.matches("return 7;").count(), 1);
}
#[test]
fn explicit_sources_work_with_auto_disabled() {
    let out = output(&["--auto=no", "profile.callgrind", "source.c"]).unwrap();
    assert!(out.contains("User-annotated source:"));
}
#[test]
fn explicit_missing_source_is_an_error() {
    assert!(
        output(&[
            "--auto=no",
            "profile.callgrind",
            "definitely-nonexistent-source.c"
        ])
        .unwrap_err()
        .contains("not opened")
    );
}
#[test]
fn auto_no_does_not_read_sources() {
    let out = output(&["--auto=no"]).unwrap();
    assert!(!out.contains("-annotated source:"));
}
#[test]
fn huge_context_does_not_overflow() {
    let out = output(&["--context", &usize::MAX.to_string()]).unwrap();
    assert!(out.contains("-- lines 1-10"));
}
#[test]
fn automatic_missing_sources_are_reported_without_failing() {
    let p =
        parse_profile("events: Ir\nfl=/nonexistent-callgrind-test/source.c\nfn=f\n1 2\n").unwrap();
    let a = Analysis::build(&p, &p.parts[0]).unwrap();
    let o = Options::try_parse_from(["annotate"]).unwrap();
    let s = Selection::build(&a, &o).unwrap();
    assert!(
        render(&p, &p.parts[0], &a, &s, &o)
            .unwrap()
            .contains("could not be found")
    );
}
