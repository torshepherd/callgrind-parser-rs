//! Toolchain smoke test, not a settled command-line interface.
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
struct Arguments {
    profile: PathBuf,
}

#[test]
fn clap_derives_argument_parsing_without_starting_a_process() {
    let args = Arguments::try_parse_from(["annotate", "profile.callgrind"]).unwrap();
    assert_eq!(args.profile, PathBuf::from("profile.callgrind"));
}
