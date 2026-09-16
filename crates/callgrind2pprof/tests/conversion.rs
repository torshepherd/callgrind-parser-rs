use callgrind_parser::{Record, parse_profile};
use callgrind2pprof::{Options, Report, convert};
use pprof_profile::{proto, string};
use std::collections::BTreeMap;

fn run(text: &str) -> Report {
    convert(&parse_profile(text).unwrap(), &Options::default()).unwrap()
}
fn labels(p: &proto::Profile, s: &proto::Sample) -> BTreeMap<String, String> {
    s.label
        .iter()
        .map(|l| {
            (
                string(p, l.key).unwrap().into(),
                string(p, l.str).unwrap().into(),
            )
        })
        .collect()
}
fn gzip(r: &Report) -> Vec<u8> {
    let mut bytes = Vec::new();
    r.write(&mut bytes).unwrap();
    bytes
}

#[test]
fn exact_multicolumn_self_costs_ignore_calls_jumps_and_summary() {
    let r = run(
        "events: Ir Dr\nsummary: 999 999\nob=app\nfl=a.c\nfn=main\n1 9007199254740993 4\ncfn=foo\ncalls=0 2\n1 80 90\njump=40 2\n1\nfn=foo\n2 7 9\ntotals: 9007199254741000 13\n",
    );
    assert_eq!(r.totals(), [9007199254741000, 13]);
    let p = r.profile();
    assert_eq!(p.sample.len(), 2);
    assert!(p.sample.iter().all(|s| s.location_id.len() == 1));
    assert!(p.mapping.is_empty());
    assert!(
        p.location
            .iter()
            .all(|l| l.address == 0 && l.mapping_id == 0)
    );
    assert_eq!(
        p.sample.iter().map(|s| s.value[0]).sum::<i64>(),
        9007199254741000
    );
    assert_eq!(string(p, p.default_sample_type).unwrap(), "Ir");
    assert_eq!(pprof_profile::read(gzip(&r).as_slice(), 4096).unwrap(), *p);
}
#[test]
fn repeated_locations_aggregate_deterministically() {
    let a = run("events: Ir\nfl=a.c\nfn=f\n2 4\n1 3\n2 7\n");
    let b = run("events: Ir\nfl=a.c\nfn=f\n2 7\n2 4\n1 3\n");
    assert_eq!(a.profile().sample.len(), 2);
    assert_eq!(gzip(&a), gzip(&b));
    assert_eq!(a.profile().sample[1].value, [11]);
}
#[test]
fn object_and_full_path_collisions_remain_distinct() {
    let r = run(
        "events: Ir\nob=/a/app\nfl=/a/same.c\nfn=same\n1 2\nob=/b/app\nfl=/a/same.c\nfn=same\n1 3\nfl=/b/same.c\nfn=same\n1 4\n",
    );
    let p = r.profile();
    assert_eq!(p.function.len(), 3);
    assert_eq!(
        p.function
            .iter()
            .map(|f| f.name)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
    assert_eq!(
        p.sample
            .iter()
            .map(|s| labels(p, s)["callgrind.object"].clone())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        2
    );
}
#[test]
fn inline_attribution_preserves_defining_file_without_inventing_inline_stack() {
    let r = run("events: Ir\nfl=main.c\nfn=main\n1 2\nfi=inline.h\n10 3\nfe=main.c\n2 5\n");
    let p = r.profile();
    assert_eq!(p.function.len(), 2);
    let s = p.sample.iter().find(|s| s.value == [3]).unwrap();
    let l = labels(p, s);
    assert_eq!(l["callgrind.defining_file"], "v:main.c");
    assert_eq!(l["callgrind.source_file"], "v:inline.h");
    assert!(p.location.iter().all(|l| l.line.len() == 1));
    assert!(
        p.function
            .iter()
            .any(|f| string(p, f.filename).unwrap() == "inline.h")
    );
}
#[test]
fn original_positions_are_labels_not_runtime_addresses() {
    let r = run("positions: instr bb line\nevents: Ir\nfn=f\n0xffffffffffffffff 0x99 0 7\n");
    let p = r.profile();
    let l = labels(p, &p.sample[0]);
    assert_eq!(l["callgrind.instruction"], u64::MAX.to_string());
    assert_eq!(l["callgrind.basic_block"], "153");
    assert_eq!(l["callgrind.line"], "0");
    assert_eq!(p.location[0].address, 0);
    assert_eq!(p.location[0].line[0].line, 0);
    let r = run("positions: instr\nevents: Ir\nfn=f\n0x40 1\n");
    assert!(!labels(r.profile(), &r.profile().sample[0]).contains_key("callgrind.line"));
}
#[test]
fn unknown_and_literal_unknown_names_are_distinct() {
    let r = run("events: Ir\n1 2\nfn=???\n1 3\nfn=<unknown>\n1 4\n");
    assert_eq!(r.profile().function.len(), 3);
    assert!(
        r.profile()
            .sample
            .iter()
            .any(|s| !labels(r.profile(), s).contains_key("callgrind.function"))
    );
}
#[test]
fn units_are_explicit_and_never_scaled() {
    let p = parse_profile("events: Ir sysTime Weird\n1 2 1500 8\n").unwrap();
    let r = convert(&p, &Options::default()).unwrap();
    assert_eq!(
        r.profile()
            .sample_type
            .iter()
            .map(|t| string(r.profile(), t.unit).unwrap())
            .collect::<Vec<_>>(),
        ["count", "callgrind_raw", "callgrind_raw"]
    );
    let options = Options {
        units: BTreeMap::from([("sysTime".into(), "nanoseconds".into())]),
        ..Default::default()
    };
    let r = convert(&p, &options).unwrap();
    assert_eq!(r.profile().sample[0].value, [2, 1500, 8]);
    assert_eq!(
        string(r.profile(), r.profile().sample_type[1].unit).unwrap(),
        "nanoseconds"
    );
    for (event, unit) in [("missing", "count"), ("Ir", ""), ("Ir", "bad\nunit")] {
        assert!(
            convert(
                &p,
                &Options {
                    units: BTreeMap::from([(event.into(), unit.into())]),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
#[test]
fn multipart_requires_selection_and_keeps_local_event_order() {
    let p = parse_profile(
        "part: 1\nevents: Ir Dr\n1 2 3\ntotals: 2 3\npart: 2\nevents: Dr Ir\n1 5 7\ntotals: 5 7\n",
    )
    .unwrap();
    assert!(convert(&p, &Options::default()).is_err());
    let r = convert(
        &p,
        &Options {
            part: Some(1),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(r.part(), 1);
    assert_eq!(r.totals(), [5, 7]);
    assert_eq!(
        string(r.profile(), r.profile().sample_type[0].r#type).unwrap(),
        "Dr"
    );
    assert!(
        convert(
            &p,
            &Options {
                part: Some(2),
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn rejects_signed_range_overflow_in_rows_aggregation_total_and_lines() {
    for body in [
        "1 9223372036854775808\n",
        "1 9223372036854775807\n1 1\n",
        "1 9223372036854775807\n2 1\n",
        "9223372036854775808 1\n",
    ] {
        assert!(
            convert(
                &parse_profile(&format!("events: Ir\n{body}")).unwrap(),
                &Options::default()
            )
            .is_err()
        );
    }
    assert_eq!(
        run("events: Ir\n1 9223372036854775807\n").totals(),
        [i64::MAX as u64]
    );
}
#[test]
fn zero_rows_and_empty_profile_keep_types_and_zero_totals() {
    let r = run("events: Ir Dr\n1 0 0\ntotals: 0 0\n");
    assert_eq!(r.totals(), [0, 0]);
    assert!(r.profile().sample.is_empty());
    assert_eq!(r.profile().sample_type.len(), 2);
    assert!(run("events: Ir\n").profile().sample.is_empty());
}
#[test]
fn rejects_mismatched_totals_and_invalid_mutated_model() {
    assert!(
        convert(
            &parse_profile("events: Ir\n1 3\ntotals: 4\n").unwrap(),
            &Options::default()
        )
        .is_err()
    );
    let mut p = parse_profile("events: Ir\n1 3\n").unwrap();
    let Record::Cost { costs, .. } = &mut p.parts[0].records[0].record else {
        panic!()
    };
    *costs = vec![3, 4].into_boxed_slice();
    assert!(convert(&p, &Options::default()).is_err());
    p.parts.clear();
    assert!(convert(&p, &Options::default()).is_err());
}
#[test]
fn location_limits_and_output_failures() {
    let p = parse_profile("events: Ir\n1 3\n2 4\n").unwrap();
    for limit in [0, 1] {
        assert!(
            convert(
                &p,
                &Options {
                    max_locations: limit,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    struct Fail;
    impl std::io::Write for Fail {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("test"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert!(
        convert(&p, &Options::default())
            .unwrap()
            .write(Fail)
            .is_err()
    );
}
