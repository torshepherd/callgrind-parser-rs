use callgrind_parser::{Record, parse_profile};
use pprof_profile::proto::*;
use pprof2callgrind::{Mode, Options, convert};

fn fixture(stacks: &[(&[u64], i64)]) -> Profile {
    Profile {
        string_table: vec![
            "", "work", "count", "main", "A", "B", "M", "X", "Y", "app", "lib", "file.c",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        sample_type: vec![ValueType { r#type: 1, unit: 2 }],
        mapping: vec![
            Mapping {
                id: 1,
                filename: 9,
                ..Default::default()
            },
            Mapping {
                id: 2,
                filename: 10,
                ..Default::default()
            },
        ],
        function: (1..=6)
            .map(|id| Function {
                id,
                name: id as i64 + 2,
                filename: 11,
                ..Default::default()
            })
            .collect(),
        location: (1..=6)
            .map(|id| Location {
                id,
                mapping_id: if id == 6 { 2 } else { 1 },
                address: id * 256,
                line: vec![Line {
                    function_id: id,
                    line: id as i64 * 10,
                    column: 0,
                }],
                is_folded: false,
            })
            .collect(),
        sample: stacks
            .iter()
            .map(|(s, v)| Sample {
                location_id: s.to_vec(),
                value: vec![*v],
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}
fn output(p: &Profile, mode: Mode) -> String {
    let report = convert(
        p,
        &Options {
            mode,
            ..Default::default()
        },
    )
    .unwrap();
    let mut bytes = Vec::new();
    report.write(&mut bytes).unwrap();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn exact_multicolumn_costs_and_cross_object_targets() {
    let mut p = fixture(&[(&[1], 10), (&[6, 1], 9007199254740993)]);
    p.sample_type.push(ValueType { r#type: 1, unit: 2 });
    p.sample[0].value.push(1500);
    p.sample[1].value.push(2500);
    for mode in [Mode::Graph, Mode::Tree] {
        let report = convert(
            &p,
            &Options {
                mode,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(report.totals(), [9007199254741003, 4000]);
        let parsed = parse_profile(&output(&p, mode)).unwrap();
        assert_eq!(
            parsed.parts[0].totals.as_deref(),
            Some(&[9007199254741003, 4000][..])
        );
        let calls: Vec<_> = parsed.parts[0]
            .records
            .iter()
            .filter_map(|r| match &r.record {
                Record::Call {
                    source,
                    target,
                    count,
                    costs,
                } => Some((source, target, count, costs)),
                _ => None,
            })
            .collect();
        assert_eq!(calls.len(), 1);
        let (s, t, n, c) = calls[0];
        assert_eq!(*n, 0);
        assert_eq!(&**c, [9007199254740993, 2500]);
        assert_eq!(&s.positions[..], [256, 10]);
        assert_eq!(&t.positions[..], [1536, 60]);
        let f = parsed.symbols.function(t.function).unwrap();
        assert_eq!(parsed.symbols.resolve(f.object.unwrap()), Some("lib"));
    }
}

#[test]
fn context_tree_distinguishes_ambiguous_graphs() {
    let a = fixture(&[(&[5, 4, 2, 1], 4), (&[6, 4, 3, 1], 4)]);
    let b = fixture(&[(&[5, 4, 3, 1], 4), (&[6, 4, 2, 1], 4)]);
    assert_eq!(output(&a, Mode::Graph), output(&b, Mode::Graph));
    assert_ne!(output(&a, Mode::Tree), output(&b, Mode::Tree));
    let mut reversed = a.clone();
    reversed.sample.reverse();
    reversed.location.reverse();
    reversed.function.reverse();
    for mode in [Mode::Graph, Mode::Tree] {
        assert_eq!(output(&a, mode), output(&reversed, mode));
    }
}

#[test]
fn recursion_retains_self_edge_and_deduplicates_sample_edges() {
    let p = fixture(&[(&[2, 2, 2, 1], 7)]);
    let g = convert(&p, &Options::default()).unwrap();
    assert_eq!(g.nodes().len(), 2);
    assert_eq!(g.edges().len(), 2);
    assert!(g.edges().iter().any(|e| e.source == e.target));
    assert!(g.edges().iter().all(|e| e.costs == [7]));
    let t = convert(
        &p,
        &Options {
            mode: Mode::Tree,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(t.nodes().len(), 4);
    assert_eq!(t.edges().len(), 3);
    let p = fixture(&[(&[2, 3, 2, 3, 2, 1], 7)]);
    let g = convert(&p, &Options::default()).unwrap();
    assert_eq!(g.edges().len(), 3);
    assert!(g.edges().iter().all(|e| e.costs == [7]));
}

#[test]
fn inline_frames_expand_leaf_first() {
    let mut p = fixture(&[(&[2, 1], 9)]);
    p.location[1].line.insert(
        0,
        Line {
            function_id: 3,
            line: 33,
            column: 0,
        },
    );
    let g = convert(&p, &Options::default()).unwrap();
    assert_eq!(g.nodes().len(), 3);
    assert_eq!(g.edges().len(), 2);
    assert!(
        g.nodes()
            .iter()
            .any(|n| n.function.name.starts_with("B ") && n.self_costs == [9] && n.line == 33)
    );
}

#[test]
fn unknown_frames_and_duplicate_names_keep_identity() {
    let mut p = fixture(&[(&[1], 3), (&[2], 4)]);
    p.function[1].name = p.function[0].name;
    let g = convert(&p, &Options::default()).unwrap();
    assert_ne!(g.nodes()[0].function.name, g.nodes()[1].function.name);
    for l in &mut p.location {
        l.line.clear();
        l.address = 0;
    }
    let g = convert(&p, &Options::default()).unwrap();
    assert_eq!(g.nodes().len(), 2);
    assert_ne!(g.nodes()[0].function.name, g.nodes()[1].function.name);
}

#[test]
fn duplicate_locations_merge_and_labels_warn() {
    let mut p = fixture(&[(&[1], 3), (&[2], 4)]);
    p.location[1] = p.location[0].clone();
    p.location[1].id = 2;
    p.sample[0].label.push(Label {
        key: 1,
        str: 2,
        ..Default::default()
    });
    let g = convert(&p, &Options::default()).unwrap();
    assert_eq!(g.nodes().len(), 1);
    assert_eq!(g.nodes()[0].self_costs, [7]);
    assert!(g.warnings().iter().any(|w| w.contains("labels")));
}

#[test]
fn invalid_costs_stacks_positions_and_overflow_fail() {
    for p in [
        fixture(&[(&[1], -1)]),
        fixture(&[(&[], 1)]),
        fixture(&[(&[1], i64::MAX), (&[1], i64::MAX), (&[1], 2)]),
    ] {
        assert!(convert(&p, &Options::default()).is_err());
    }
    let mut p = fixture(&[(&[1], 1)]);
    p.location[0].line[0].line = -1;
    assert!(convert(&p, &Options::default()).is_err());
    let p = fixture(&[(&[], 0)]);
    assert_eq!(convert(&p, &Options::default()).unwrap().totals(), [0]);
    parse_profile(&output(&p, Mode::Graph)).unwrap();
}

#[test]
fn limits_fail_without_recursion_or_partial_output() {
    let p = fixture(&[(&[1, 1, 1], 1)]);
    assert!(
        convert(
            &p,
            &Options {
                max_depth: 2,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        convert(
            &p,
            &Options {
                max_nodes: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        convert(
            &p,
            &Options {
                max_depth: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    let p = fixture(&[(&[1, 1, 1, 1, 1, 1, 1], 1)]);
    assert!(
        convert(
            &p,
            &Options {
                mode: Mode::Tree,
                max_nodes: 6,
                ..Default::default()
            }
        )
        .is_err()
    );
}
