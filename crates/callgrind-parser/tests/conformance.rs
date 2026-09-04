mod support;

use callgrind_parser::{
    CallRecord, Context, CostRecord, Description, EventDefinition, JumpRecord, ParseErrorKind,
    PositionKind, Record, parse_profile,
};
use support::ProfileBuilder;

fn only_part(input: &str) -> callgrind_parser::Part {
    let mut profile = parse_profile(input).expect("fixture should parse");
    assert_eq!(profile.parts.len(), 1, "fixture should contain one part");
    profile.parts.remove(0)
}

fn assert_error(input: &str, kind: ParseErrorKind, line: usize) {
    let error = parse_profile(input).expect_err("fixture should be rejected");
    assert_eq!(error.kind, kind);
    assert_eq!(error.line, line);
}

#[test]
#[ignore = "not implemented: profile defaults and basic cost rows"]
fn minimal_profile_can_omit_marker_and_defaults_to_version_one_and_line_positions() {
    let input = ProfileBuilder::new(&["Ir"])
        .without_marker()
        .body("fl=main.c")
        .body("fn=main")
        .cost(&["15"], &[90])
        .build();

    let profile = parse_profile(&input).unwrap();
    assert!(!profile.has_format_marker);
    assert_eq!(profile.version, 1);
    assert_eq!(profile.creator, None);

    let part = &profile.parts[0];
    assert_eq!(part.positions, vec![PositionKind::Line]);
    assert_eq!(part.events, vec!["Ir".to_owned()]);
    assert_eq!(
        part.records,
        vec![Record::Cost(CostRecord {
            context: Context {
                object: None,
                file: Some("main.c".to_owned()),
                function: Some("main".to_owned()),
            },
            positions: vec![15],
            costs: vec![90],
        })]
    );
}

#[test]
#[ignore = "not implemented: comments and blank lines"]
fn ignores_comments_and_blank_lines() {
    let input = concat!(
        "# callgrind format\n",
        "# producer comment\n",
        "events: Ir\n",
        "\n",
        "# body comment\n",
        "\n",
        "fn=main\n",
        "1 3\n",
    );

    let profile = parse_profile(input).unwrap();
    assert!(profile.has_format_marker);
    assert_eq!(profile.parts.len(), 1);
    assert_eq!(profile.parts[0].records.len(), 1);
}

#[test]
#[ignore = "not implemented: part metadata and totals"]
fn parses_profile_and_part_metadata() {
    let input = ProfileBuilder::new(&["Ir", "Dr"])
        .version(1)
        .creator("fixture-generator 1.0")
        .header("pid", "42")
        .header("thread", "7")
        .header("part", "3")
        .header("cmd", "./demo --flag value")
        .header("desc", "Trigger: Program termination")
        .header("desc", "custom kind: arbitrary: value")
        .header("summary", "10 4")
        .body("fl=demo.c")
        .body("fn=main")
        .cost(&["8"], &[10, 4])
        .body("totals: 10 4")
        .build();

    let profile = parse_profile(&input).unwrap();
    assert_eq!(profile.version, 1);
    assert_eq!(profile.creator.as_deref(), Some("fixture-generator 1.0"));

    let part = &profile.parts[0];
    assert_eq!(part.metadata.pid, Some(42));
    assert_eq!(part.metadata.thread, Some(7));
    assert_eq!(part.metadata.part, Some(3));
    assert_eq!(part.metadata.command.as_deref(), Some("./demo --flag value"));
    assert_eq!(
        part.metadata.descriptions,
        vec![
            Description {
                kind: "Trigger".to_owned(),
                value: "Program termination".to_owned(),
            },
            Description {
                kind: "custom kind".to_owned(),
                value: "arbitrary: value".to_owned(),
            },
        ]
    );
    assert_eq!(part.summary, Some(vec![10, 4]));
    assert_eq!(part.totals, Some(vec![10, 4]));
}

#[test]
#[ignore = "not implemented: event definitions"]
fn parses_long_and_inherited_event_definitions() {
    let input = ProfileBuilder::new(&["Ir", "Dr"])
        .header("event", "Ir : Instruction Fetches")
        .header("event", "Total = Ir + 2 Dr : Weighted total")
        .body("fn=main")
        .cost(&["1"], &[4, 2])
        .build();

    let part = only_part(&input);
    assert_eq!(
        part.event_definitions,
        vec![
            EventDefinition {
                name: "Ir".to_owned(),
                formula: None,
                long_name: Some("Instruction Fetches".to_owned()),
            },
            EventDefinition {
                name: "Total".to_owned(),
                formula: Some("Ir + 2 Dr".to_owned()),
                long_name: Some("Weighted total".to_owned()),
            },
        ]
    );
}

#[test]
#[ignore = "not implemented: cost width normalization"]
fn zero_fills_omitted_trailing_event_costs() {
    let input = ProfileBuilder::new(&["Ir", "Dr", "Dw"])
        .body("fl=main.c")
        .body("fn=main")
        .body("15 90 14")
        .build();

    let part = only_part(&input);
    let Record::Cost(cost) = &part.records[0] else {
        panic!("expected a cost record");
    };
    assert_eq!(cost.positions, vec![15]);
    assert_eq!(cost.costs, vec![90, 14, 0]);
}

#[test]
#[ignore = "not implemented: position kinds and hexadecimal numbers"]
fn parses_all_position_kinds_and_hexadecimal_numbers() {
    let input = ProfileBuilder::new(&["Ir"])
        .positions(&["instr", "bb", "line"])
        .body("fn=main")
        .body("0x10 0x2 7 0xA")
        .build();

    let part = only_part(&input);
    assert_eq!(
        part.positions,
        vec![
            PositionKind::Instruction,
            PositionKind::BasicBlock,
            PositionKind::Line,
        ]
    );
    let Record::Cost(cost) = &part.records[0] else {
        panic!("expected a cost record");
    };
    assert_eq!(cost.positions, vec![16, 2, 7]);
    assert_eq!(cost.costs, vec![10]);
}

#[test]
#[ignore = "not implemented: subposition compression"]
fn resolves_relative_subpositions_per_column() {
    let input = ProfileBuilder::new(&["ticks"])
        .positions(&["instr", "line"])
        .body("fn=func")
        .body("0x80001234 90 1")
        .body("+3 * 5")
        .body("+1 +1 6")
        .build();

    let part = only_part(&input);
    let positions = part
        .records
        .iter()
        .map(|record| match record {
            Record::Cost(cost) => cost.positions.clone(),
            _ => panic!("expected only cost records"),
        })
        .collect::<Vec<_>>();

    assert_eq!(part.positions, vec![PositionKind::Instruction, PositionKind::Line]);
    assert_eq!(
        positions,
        vec![
            vec![0x8000_1234, 90],
            vec![0x8000_1237, 90],
            vec![0x8000_1238, 91],
        ]
    );
}

#[test]
#[ignore = "not implemented: name compression"]
fn resolves_name_compression_in_separate_namespaces() {
    let input = ProfileBuilder::new(&["Ir"])
        .body("fl=(1) src/main.c")
        .body("fn=(1) main")
        .body("fl=(1)")
        .body("fn=(1)")
        .cost(&["7"], &[12])
        .build();

    let part = only_part(&input);
    let Record::Cost(cost) = &part.records[0] else {
        panic!("expected a cost record");
    };
    assert_eq!(
        cost.context,
        Context {
            object: None,
            file: Some("src/main.c".to_owned()),
            function: Some("main".to_owned()),
        }
    );
}

#[test]
#[ignore = "not implemented: calls and called contexts"]
fn parses_call_associations_and_cfl_alias() {
    let input = ProfileBuilder::new(&["Ir"])
        .body("fl=caller.c")
        .body("fn=main")
        .body("cfl=callee.c")
        .body("cfn=work")
        .body("calls=3 20")
        .body("16 400")
        .build();

    let part = only_part(&input);
    assert_eq!(
        part.records,
        vec![Record::Call(CallRecord {
            caller: Context {
                object: None,
                file: Some("caller.c".to_owned()),
                function: Some("main".to_owned()),
            },
            callee: Context {
                object: None,
                file: Some("callee.c".to_owned()),
                function: Some("work".to_owned()),
            },
            count: 3,
            target_positions: vec![20],
            source_positions: vec![16],
            costs: vec![400],
        })]
    );
}

#[test]
#[ignore = "not implemented: jump associations"]
fn parses_unconditional_and_conditional_jumps() {
    let input = ProfileBuilder::new(&["Ir"])
        .body("fl=main.c")
        .body("fn=main")
        .body("10 1")
        .body("jump=4 20")
        .body("jcnd=10 7 30")
        .build();

    let part = only_part(&input);
    let context = Context {
        object: None,
        file: Some("main.c".to_owned()),
        function: Some("main".to_owned()),
    };
    assert_eq!(
        part.records,
        vec![
            Record::Cost(CostRecord {
                context: context.clone(),
                positions: vec![10],
                costs: vec![1],
            }),
            Record::Jump(JumpRecord {
                context: context.clone(),
                executed: 4,
                taken: None,
                target_positions: vec![20],
            }),
            Record::Jump(JumpRecord {
                context,
                executed: 10,
                taken: Some(7),
                target_positions: vec![30],
            }),
        ]
    );
}

#[test]
#[ignore = "not implemented: multiple profile parts"]
fn parses_multiple_parts() {
    let input = concat!(
        "# callgrind format\n",
        "creator: unit-test\n",
        "part: 1\n",
        "events: Ir\n",
        "fn=first\n",
        "1 10\n",
        "part: 2\n",
        "events: Ir\n",
        "fn=second\n",
        "2 20\n",
    );

    let profile = parse_profile(input).unwrap();
    assert_eq!(profile.parts.len(), 2);
    assert_eq!(profile.parts[0].metadata.part, Some(1));
    assert_eq!(profile.parts[1].metadata.part, Some(2));
}

#[test]
#[ignore = "not implemented: required events validation"]
fn rejects_a_part_without_events() {
    assert_error("# callgrind format\nfl=main.c\nfn=main\n1 1\n", ParseErrorKind::MissingEvents, 2);
}

#[test]
#[ignore = "not implemented: format version validation"]
fn rejects_an_unsupported_version() {
    assert_error(
        "# callgrind format\nversion: 2\nevents: Ir\nfn=main\n1 1\n",
        ParseErrorKind::UnsupportedVersion,
        2,
    );
}

#[test]
#[ignore = "not implemented: compression reference validation"]
fn rejects_an_unknown_name_compression_id() {
    assert_error(
        "# callgrind format\nevents: Ir\n\nfl=(7)\nfn=main\n1 1\n",
        ParseErrorKind::UnknownNameId,
        4,
    );
}

#[test]
#[ignore = "not implemented: integer overflow validation"]
fn rejects_a_number_larger_than_u64() {
    assert_error(
        "# callgrind format\nevents: Ir\n\nfn=main\n1 18446744073709551616\n",
        ParseErrorKind::NumberOverflow,
        5,
    );
}

#[test]
#[ignore = "not implemented: malformed number validation"]
fn rejects_a_malformed_number() {
    assert_error(
        "# callgrind format\nevents: Ir\n\nfn=main\n1 nope\n",
        ParseErrorKind::InvalidNumber,
        5,
    );
}

#[test]
#[ignore = "not implemented: call association pairing"]
fn rejects_a_call_without_its_mandatory_cost_row() {
    assert_error(
        "# callgrind format\nevents: Ir\n\ncfn=work\ncalls=1 20\n",
        ParseErrorKind::MissingAssociationCost,
        5,
    );
}

#[test]
#[ignore = "not implemented: positions ordering validation"]
fn rejects_positions_out_of_defined_order() {
    assert_error(
        "# callgrind format\npositions: line instr\nevents: Ir\n\nfn=main\n1 0x10 1\n",
        ParseErrorKind::InvalidPositionOrder,
        2,
    );
}

#[test]
#[ignore = "not implemented: cost width validation"]
fn rejects_more_costs_than_declared_events() {
    assert_error(
        "# callgrind format\nevents: Ir\n\nfn=main\n1 2 3\n",
        ParseErrorKind::InvalidCostWidth,
        5,
    );
}
