mod support;

use callgrind_parser::*;
use support::ProfileBuilder;

fn cost(profile: &Profile, index: usize) -> (&Location, &[u64]) {
    let Record::Cost { location, costs } = &profile.parts[0].records[index].record else {
        panic!("expected self cost");
    };
    (location, costs)
}

fn text(profile: &Profile, id: Option<StringId>) -> Option<&str> {
    id.map(|id| profile.symbols.resolve(id).unwrap())
}

fn error(input: &str, kind: ParseErrorKind, line: usize) {
    let actual = parse_profile(input).unwrap_err();
    assert_eq!(actual.kind, kind, "{input}");
    assert_eq!(actual.line, line, "{input}");
    assert!(actual.to_string().contains(&format!("line {line}")));
}

#[test]
fn defaults_and_minimal_cost() {
    let input = ProfileBuilder::new(&["Ir"])
        .without_marker()
        .body("fl=main.c")
        .body("fn=main")
        .cost(&["15"], &[90])
        .build();
    let p = parse_profile(&input).unwrap();
    assert!(!p.header.has_format_marker);
    assert_eq!(p.header.version, 1);
    assert_eq!(p.header.creator, None);
    assert_eq!(p.parts[0].header.positions, [PositionKind::Line]);
    assert_eq!(p.symbols.resolve(p.parts[0].header.events[0]), Some("Ir"));
    let (location, values) = cost(&p, 0);
    assert_eq!(&location.positions[..], [15]);
    assert_eq!(values, [90]);
    let function = p.symbols.function(location.function).unwrap();
    assert_eq!(text(&p, function.object), None);
    assert_eq!(text(&p, function.file), Some("main.c"));
    assert_eq!(text(&p, function.name), Some("main"));
}

#[test]
fn comments_crlf_tabs_and_final_line_without_newline() {
    let p = parse_profile(
        "# callgrind format\r\n# comment\r\nevents:\t Ir Dr\r\n\r\nfn=main\r\n\t1\t3",
    )
    .unwrap();
    assert!(p.header.has_format_marker);
    assert_eq!(p.parts.len(), 1);
    assert_eq!(p.parts[0].records.len(), 1);
    assert_eq!(cost(&p, 0).1, [3, 0]);
}

#[test]
fn metadata_and_distinct_summary_and_totals() {
    let input = ProfileBuilder::new(&["Ir", "Dr"])
        .version(1)
        .creator("tests")
        .header("pid", "42")
        .header("thread", "7")
        .header("part", "3")
        .header("cmd", "./demo --flag value")
        .header("desc", "I1 cache: 32768 B, 64 B, 8-way")
        .header("desc", "custom kind: arbitrary: value")
        .header("custom-header", "keep me")
        .header("summary", "100 40")
        .body("fn=main")
        .cost(&["8"], &[10, 4])
        .body("totals: 10 4")
        .build();
    let p = parse_profile(&input).unwrap();
    assert_eq!(p.header.creator.as_deref(), Some("tests"));
    let h = &p.parts[0].header;
    assert_eq!(
        (h.metadata.pid, h.metadata.thread, h.metadata.part),
        (Some(42), Some(7), Some(3))
    );
    assert_eq!(h.metadata.command.as_deref(), Some("./demo --flag value"));
    assert_eq!(
        h.metadata.descriptions,
        [
            ("I1 cache".into(), "32768 B, 64 B, 8-way".into()),
            ("custom kind".into(), "arbitrary: value".into())
        ]
    );
    assert_eq!(
        h.metadata.extensions,
        [("custom-header".into(), "keep me".into())]
    );
    assert_eq!(h.summary.as_deref(), Some([100, 40].as_slice()));
    assert_eq!(p.parts[0].totals.as_deref(), Some([10, 4].as_slice()));
}

#[test]
fn compatible_version_zero() {
    assert_eq!(
        parse_profile("version: 0\nevents: Ir\n1 0\n")
            .unwrap()
            .header
            .version,
        0
    );
}

#[test]
fn event_definitions_before_and_after_events_with_typed_terms() {
    let p = parse_profile("event: Ir : Instruction Fetches\nevents: Ir Dr\nevent: Total = Ir + 2 Dr + 0x3*Ir : Weighted total\n1 4 2\n").unwrap();
    let definitions = &p.parts[0].header.event_definitions;
    assert_eq!(definitions.len(), 2);
    assert_eq!(definitions[0].terms, None);
    assert_eq!(
        definitions[0].long_name.as_deref(),
        Some("Instruction Fetches")
    );
    let terms = definitions[1].terms.as_ref().unwrap();
    let resolved: Vec<_> = terms
        .iter()
        .map(|t| (t.coefficient, p.symbols.resolve(t.event).unwrap()))
        .collect();
    assert_eq!(resolved, [(1, "Ir"), (2, "Dr"), (3, "Ir")]);
    assert_eq!(definitions[1].long_name.as_deref(), Some("Weighted total"));
}

#[test]
fn missing_trailing_costs_including_all_costs_are_zero() {
    let p = parse_profile("events: Ir Dr Dw\n15 90 14\n16\n").unwrap();
    assert_eq!(cost(&p, 0).1, [90, 14, 0]);
    assert_eq!(cost(&p, 1).1, [0, 0, 0]);
}

#[test]
fn repeated_positions_are_preserved_for_checked_aggregation() {
    let p = parse_profile("events: Ir\nfn=main\n1 10\n* 20\n").unwrap();
    assert_eq!(cost(&p, 0).0, cost(&p, 1).0);
    assert_eq!(cost(&p, 0).1, [10]);
    assert_eq!(cost(&p, 1).1, [20]);
}

#[test]
fn all_position_columns_and_hexadecimal_counters() {
    let input = ProfileBuilder::new(&["Ir"])
        .positions(&["instr", "bb", "line"])
        .body("0x10 0x2 7 0xA")
        .build();
    let p = parse_profile(&input).unwrap();
    let h = &p.parts[0].header;
    let (location, values) = cost(&p, 0);
    assert_eq!(
        h.positions,
        [
            PositionKind::Instruction,
            PositionKind::BasicBlock,
            PositionKind::Line
        ]
    );
    assert_eq!(&location.positions[..], [16, 2, 7]);
    assert!(!location.positions.spilled());
    assert_eq!(
        h.position(&location.positions, PositionKind::Instruction),
        Some(16)
    );
    assert_eq!(values, [10]);
}

#[test]
fn every_nonempty_position_combination() {
    for columns in [
        "instr",
        "bb",
        "line",
        "instr bb",
        "instr line",
        "bb line",
        "instr bb line",
    ] {
        let width = columns.split_whitespace().count();
        let values = vec!["0"; width].join(" ");
        let p = parse_profile(&format!("positions: {columns}\nevents: Ir\n{values} 9\n")).unwrap();
        assert_eq!(cost(&p, 0).0.positions.len(), width);
        assert_eq!(cost(&p, 0).1, [9]);
    }
}

#[test]
fn relative_subpositions_have_independent_columns() {
    let p = parse_profile(
        "positions: instr line\nevents: ticks\n0x80001234 90 1\n+3 * 5\n+1 +1 6\n-0x4 -2 7\n",
    )
    .unwrap();
    assert_eq!(&cost(&p, 1).0.positions[..], [0x80001237, 90]);
    assert_eq!(&cost(&p, 2).0.positions[..], [0x80001238, 91]);
    assert_eq!(&cost(&p, 3).0.positions[..], [0x80001234, 89]);
}

#[test]
fn relative_origin_is_zero_at_part_start() {
    let p = parse_profile("events: Ir\n+3 1\n").unwrap();
    assert_eq!(&cost(&p, 0).0.positions[..], [3]);
}

#[test]
fn zero_unknown_positions_are_not_missing_columns() {
    let p = parse_profile("events: Ir\n0 1\n").unwrap();
    let h = &p.parts[0].header;
    assert_eq!(
        h.position(&cost(&p, 0).0.positions, PositionKind::Line),
        Some(0)
    );
    assert_eq!(
        h.position(&cost(&p, 0).0.positions, PositionKind::Instruction),
        None
    );
}

#[test]
fn full_unsigned_range_is_preserved_for_later_checked_pprof_conversion() {
    let p =
        parse_profile("positions: instr\nevents: Ir\n0xffffffffffffffff 18446744073709551615\n")
            .unwrap();
    assert_eq!(&cost(&p, 0).0.positions[..], [u64::MAX]);
    assert_eq!(cost(&p, 0).1, [u64::MAX]);
}

#[test]
fn name_spaces_are_object_file_and_function_not_one_per_tag() {
    let p = parse_profile("events: Ir\nob=(1) app\nfl=(1) a.c\nfn=(1) main\n1 1\ncob=(1)\ncfl=(1)\ncfn=(1)\ncalls=2 1\n1 8\n").unwrap();
    let Record::Call { source, target, .. } = &p.parts[0].records[1].record else {
        panic!()
    };
    assert_eq!(source.function, target.function);
    let f = p.symbols.function(source.function).unwrap();
    assert_eq!(text(&p, f.object), Some("app"));
    assert_eq!(text(&p, f.file), Some("a.c"));
    assert_eq!(text(&p, f.name), Some("main"));
}

#[test]
fn forward_definitions_share_caller_and_callee_namespaces() {
    let p = parse_profile("events: Ir\nfl=(1) a.c\nfn=main\ncfi=(2) b.c\ncfn=(7) work\ncalls=3 20\n16 400\nfl=(2)\nfn=(7)\n20 400\n").unwrap();
    let Record::Call {
        source,
        target,
        count,
        costs,
    } = &p.parts[0].records[0].record
    else {
        panic!()
    };
    assert_eq!(*count, 3);
    assert_eq!(&costs[..], [400]);
    assert_eq!(&source.positions[..], [16]);
    assert_eq!(&target.positions[..], [20]);
    assert_eq!(target.function, cost(&p, 1).0.function);
    assert_eq!(text(&p, target.file), Some("b.c"));
}

#[test]
fn sparse_large_compression_ids_do_not_allocate_id_sized_vectors() {
    let p = parse_profile(
        "events: Ir\nfn=(18446744073709551615) main\nfn=(18446744073709551615)\n1 1\n",
    )
    .unwrap();
    assert_eq!(p.symbols.functions().len(), 1);
    assert_eq!(p.symbols.string_count(), 2);
}

#[test]
fn redefining_a_format_alias_does_not_rewrite_existing_rows() {
    let p =
        parse_profile("events: Ir\nfn=(1) first\n1 1\nfn=(1) second\n2 2\nfn=(1)\n3 3\n").unwrap();
    assert_ne!(cost(&p, 0).0.function, cost(&p, 1).0.function);
    assert_eq!(cost(&p, 1).0.function, cost(&p, 2).0.function);
    assert_eq!(
        text(&p, p.symbols.function(cost(&p, 0).0.function).unwrap().name),
        Some("first")
    );
}

#[test]
fn strings_are_interned_but_functions_are_qualified() {
    let p = parse_profile(
        "events: Ir\nob=a.so\nfl=x.c\nfn=work\n1 1\nob=b.so\nfn=work\n1 2\nfl=y.c\nfn=work\n1 3\n",
    )
    .unwrap();
    let ids: Vec<_> = (0..3).map(|i| cost(&p, i).0.function).collect();
    assert_ne!(ids[0], ids[1]);
    assert_ne!(ids[1], ids[2]);
    let names: Vec<_> = ids
        .iter()
        .map(|id| p.symbols.function(*id).unwrap().name)
        .collect();
    assert_eq!(names[0], names[1]);
    assert_eq!(names[1], names[2]);
}

#[test]
fn inline_files_do_not_change_function_identity_and_fn_restores_definition_file() {
    let p = parse_profile("events: Ir\nfl=(1) main.c\nfn=main\n10 1\nfi=(2) inline.h\n20 2\nfe=(1)\n11 3\nfi=(2)\nfn=other\n30 4\n").unwrap();
    assert_eq!(cost(&p, 0).0.function, cost(&p, 1).0.function);
    assert_eq!(cost(&p, 1).0.function, cost(&p, 2).0.function);
    assert_eq!(text(&p, cost(&p, 1).0.file), Some("inline.h"));
    assert_eq!(
        text(&p, p.symbols.function(cost(&p, 1).0.function).unwrap().file),
        Some("main.c")
    );
    assert_eq!(text(&p, cost(&p, 3).0.file), Some("main.c"));
}

#[test]
fn names_preserve_cpp_punctuation_unicode_and_unknown_markers() {
    let p = parse_profile(
        "events: Ir\nob=???\nfl=src/naïve code.cpp\nfn=(anonymous namespace)::f<int>(x = 1)\n1 2\n",
    )
    .unwrap();
    let f = p.symbols.function(cost(&p, 0).0.function).unwrap();
    assert_eq!(
        text(&p, f.name),
        Some("(anonymous namespace)::f<int>(x = 1)")
    );
    assert_eq!(text(&p, f.file), Some("src/naïve code.cpp"));
    assert_eq!(text(&p, f.object), Some("???"));
}

#[test]
fn absent_context_remains_absent() {
    let p = parse_profile("events: Ir\n1 2\n").unwrap();
    assert_eq!(
        p.symbols.function(cost(&p, 0).0.function).unwrap(),
        &Function {
            object: None,
            file: None,
            name: None
        }
    );
}

#[test]
fn callee_context_defaults_and_resets_between_calls() {
    let p = parse_profile("events: Ir\nob=app\nfl=main.c\nfn=main\ncob=lib.so\ncfi=lib.c\ncfn=foreign\ncalls=1 20\n10 8\ncfn=local\ncalls=2 30\n10 9\n").unwrap();
    let Record::Call { target: a, .. } = &p.parts[0].records[0].record else {
        panic!()
    };
    let Record::Call { target: b, .. } = &p.parts[0].records[1].record else {
        panic!()
    };
    assert_eq!(
        text(&p, p.symbols.function(a.function).unwrap().object),
        Some("lib.so")
    );
    assert_eq!(
        text(&p, p.symbols.function(b.function).unwrap().object),
        Some("app")
    );
    assert_eq!(text(&p, b.file), Some("main.c"));
}

#[test]
fn target_compression_uses_previous_source_and_does_not_advance_it() {
    let p = parse_profile("positions: instr line\nevents: Ir\nfn=main\n0x100 10 1\ncfn=work\ncalls=2 +0x20 +5\n+4 +1 9\n+1 * 2\n").unwrap();
    let Record::Call { source, target, .. } = &p.parts[0].records[1].record else {
        panic!()
    };
    assert_eq!(&target.positions[..], [0x120, 15]);
    assert_eq!(&source.positions[..], [0x104, 11]);
    assert_eq!(&cost(&p, 2).0.positions[..], [0x105, 11]);
}

#[test]
fn call_costs_are_not_self_costs_and_zero_calls_are_retained() {
    let p =
        parse_profile("events: Ir Dr\nfn=main\n1 5\ncfn=work\ncalls=0 20\n# comment\n\n1 900\n")
            .unwrap();
    assert_eq!(p.parts[0].records.len(), 2);
    assert_eq!(cost(&p, 0).1, [5, 0]);
    let Record::Call { count, costs, .. } = &p.parts[0].records[1].record else {
        panic!()
    };
    assert_eq!(*count, 0);
    assert_eq!(&costs[..], [900, 0]);
    assert_eq!(p.parts[0].records[1].line, 5);
}

#[test]
fn recursive_edges_and_cycles_are_preserved_without_expansion() {
    let p = parse_profile("events: Ir\nfn=A\ncfn=A\ncalls=2 1\n1 20\ncfn=B\ncalls=3 2\n1 30\nfn=B\ncfn=A\ncalls=4 1\n2 40\n").unwrap();
    let Record::Call {
        source: a,
        target: a2,
        ..
    } = &p.parts[0].records[0].record
    else {
        panic!()
    };
    let Record::Call {
        source: a3,
        target: b,
        ..
    } = &p.parts[0].records[1].record
    else {
        panic!()
    };
    let Record::Call {
        source: b2,
        target: a4,
        ..
    } = &p.parts[0].records[2].record
    else {
        panic!()
    };
    assert_eq!(a.function, a2.function);
    assert_eq!(a.function, a3.function);
    assert_eq!(a.function, a4.function);
    assert_eq!(b.function, b2.function);
    assert_ne!(a.function, b.function);
}

#[test]
fn real_valgrind_jumps_have_following_source_rows_and_taken_over_executed() {
    // Valgrind 3.26.0 callgrind/dump.c:fprint_jcc, not the stale manual grammar.
    let p = parse_profile("events: Ir\nfn=main\n10 1\njump=4 20\n10\njcnd=7/10 30\n11\n").unwrap();
    assert_eq!(p.parts[0].records.len(), 3);
    let Record::Jump {
        source,
        target,
        executed,
        taken,
    } = &p.parts[0].records[1].record
    else {
        panic!()
    };
    assert_eq!((*executed, *taken), (4, None));
    assert_eq!(&source.positions[..], [10]);
    assert_eq!(&target.positions[..], [20]);
    let Record::Jump {
        source,
        target,
        executed,
        taken,
    } = &p.parts[0].records[2].record
    else {
        panic!()
    };
    assert_eq!((*executed, *taken), (10, Some(7)));
    assert_eq!(&source.positions[..], [11]);
    assert_eq!(&target.positions[..], [30]);
}

#[test]
fn jump_targets_can_change_file_and_function_and_then_reset() {
    let p = parse_profile(
        "events: Ir\nfl=main.c\nfn=main\njfi=other.c\njfn=other\njump=1 20\n10\njump=2 30\n11\n",
    )
    .unwrap();
    let Record::Jump { source, target, .. } = &p.parts[0].records[0].record else {
        panic!()
    };
    assert_ne!(source.function, target.function);
    assert_eq!(text(&p, target.file), Some("other.c"));
    let Record::Jump { source, target, .. } = &p.parts[0].records[1].record else {
        panic!()
    };
    assert_eq!(source.function, target.function);
    assert_eq!(text(&p, target.file), Some("main.c"));
}

#[test]
fn multiple_parts_preserve_metadata_and_reset_layout_and_position_state() {
    let p = parse_profile("part: 1\nthread: 7\npositions: instr line\nevents: Ir Dr\nfn=(1) first\n0x100 10 3 4\npart: 2\nthread: 8\nevents: Dw\nfn=(1)\n+2 9\n").unwrap();
    assert_eq!(p.parts.len(), 2);
    assert_eq!(p.parts[0].header.metadata.thread, Some(7));
    assert_eq!(p.parts[1].header.metadata.thread, Some(8));
    assert_eq!(p.parts[1].header.positions, [PositionKind::Line]);
    let Record::Cost { location, costs } = &p.parts[1].records[0].record else {
        panic!()
    };
    assert_eq!(&location.positions[..], [2]);
    assert_eq!(&costs[..], [9]);
    assert_eq!(location.function, cost(&p, 0).0.function);
}

#[test]
fn header_only_profile_and_summary_padding() {
    let p = parse_profile("summary: 10\nevents: Ir Dr\n").unwrap();
    assert!(p.parts[0].records.is_empty());
    assert_eq!(
        p.parts[0].header.summary.as_deref(),
        Some([10, 0].as_slice())
    );
}

macro_rules! rejects {
    ($($name:ident: $input:expr, $kind:ident, $line:expr;)*) => {$ (
        #[test] fn $name() { error($input, ParseErrorKind::$kind, $line); }
    )*};
}

rejects! {
    empty_input: "", MissingEvents, 1;
    missing_events: "# callgrind format\nfl=main.c\n1 1\n", MissingEvents, 2;
    empty_events: "events: \n", MissingEvents, 1;
    duplicate_events_header: "events: Ir\nevents: Dr\n", DuplicateEvents, 2;
    duplicate_event_columns: "events: Ir Ir\n", DuplicateEvents, 1;
    unsupported_version: "version: 2\nevents: Ir\n", UnsupportedVersion, 1;
    misplaced_version: "events: Ir\nversion: 1\n", InvalidHeader, 2;
    duplicate_version: "version: 1\nversion: 1\n", InvalidHeader, 2;
    unknown_alias: "events: Ir\nfl=(7)\n", UnknownNameId, 2;
    alias_namespace_isolation: "events: Ir\nfl=(7) a.c\nfn=(7)\n", UnknownNameId, 3;
    malformed_alias: "events: Ir\nfn=(7 main\n", InvalidName, 2;
    alias_requires_separator: "events: Ir\nfn=(7)main\n", InvalidName, 2;
    compressed_mangled_context_is_explicitly_unsupported: "events: Ir\nfn=(1) f\nfn=(2) (1)'2\n", UnsupportedExtension, 3;
    numeric_overflow: "events: Ir\n1 18446744073709551616\n", NumberOverflow, 2;
    hexadecimal_overflow: "events: Ir\n1 0x10000000000000000\n", NumberOverflow, 2;
    bad_number: "events: Ir\n1 nope\n", InvalidNumber, 2;
    numeric_trailing_junk: "events: Ir\n1 12junk\n", InvalidNumber, 2;
    empty_hexadecimal: "events: Ir\n1 0x\n", InvalidNumber, 2;
    signed_cost: "events: Ir\n1 -1\n", InvalidNumber, 2;
    positive_signed_cost: "events: Ir\n1 +1\n", InvalidNumber, 2;
    overflowing_relative: "events: Ir\n18446744073709551615 1\n+1 1\n", NumberOverflow, 3;
    underflowing_relative: "events: Ir\n1 1\n-2 1\n", PositionUnderflow, 3;
    invalid_position_order: "positions: line instr\nevents: Ir\n", InvalidPositionOrder, 1;
    repeated_position_kind: "positions: line line\n", InvalidPositionOrder, 1;
    empty_positions: "positions: \n", InvalidPositionOrder, 1;
    unknown_position_kind: "positions: address\n", InvalidPositionOrder, 1;
    missing_position_column: "positions: instr line\nevents: Ir\n1\n", InvalidPositionWidth, 3;
    extra_event_column: "events: Ir\n1 2 3\n", InvalidCostWidth, 2;
    extra_summary_column: "summary: 1 2\nevents: Ir\n1 1\n", InvalidCostWidth, 1;
    unknown_body_tag: "events: Ir\nwat=oops\n", UnknownBodyLine, 2;
    malformed_body: "events: Ir\ngarbage\n", UnknownBodyLine, 2;
    missing_callee: "events: Ir\ncalls=1 20\n1 1\n", MissingCalledFunction, 2;
    call_without_cost: "events: Ir\ncfn=work\ncalls=1 20\n", MissingAssociationCost, 3;
    call_interrupted_by_context: "events: Ir\ncfn=work\ncalls=1 20\nfn=other\n", MissingAssociationCost, 3;
    call_interrupted_by_part: "events: Ir\ncfn=work\ncalls=1 20\npart: 2\n", MissingAssociationCost, 3;
    bad_call_target_width: "events: Ir\ncfn=work\ncalls=1 20 30\n", InvalidPositionWidth, 3;
    missing_jump_source: "events: Ir\njump=1 20\n", MissingAssociationCost, 2;
    jump_with_cost_columns: "events: Ir\njump=1 20\n10 2\n", InvalidCostWidth, 3;
    impossible_jump_counts: "events: Ir\njcnd=11/10 20\n10\n", InvalidJumpCounts, 2;
    stale_manual_jcnd_syntax: "events: Ir\njcnd=10 7 20\n", InvalidPositionWidth, 2;
    incomplete_expression: "event: Total = Ir +\nevents: Ir\n", InvalidEventDefinition, 1;
    unsupported_expression_operator: "event: Total = Ir - Dr\nevents: Ir Dr\n", InvalidEventDefinition, 1;
    overflowed_expression_coefficient: "event: Total = 18446744073709551616 Ir\n", NumberOverflow, 1;
    data_after_totals: "events: Ir\n1 2\ntotals: 2\n2 3\n", UnexpectedRecord, 4;
    duplicate_totals: "events: Ir\n1 2\ntotals: 2\ntotals: 2\n", InvalidHeader, 4;
    missing_events_in_second_part: "events: Ir\n1 2\npart: 2\nfn=other\n", MissingEvents, 4;
}
