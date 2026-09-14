use callgrind_annotate::{Analysis, Options, Selection, render, render_tsv, select_part};
use callgrind_parser::parse_profile;
use clap::Parser;

#[test]
fn explicit_source_grouping_preserves_raw_function_view() {
    let p = parse_profile(include_str!(
        "../../../tests/reference/fixtures/inline-source.callgrind"
    ))
    .unwrap();
    let original = p.clone();
    let grouped =
        Analysis::build_grouped(&p, &p.parts[0], callgrind_annotate::Grouping::Source).unwrap();
    assert_eq!(grouped.functions.len(), 3);
    let pieces: Vec<_> = grouped
        .functions
        .iter()
        .filter(|(k, _)| k.name.as_deref() == Some("main"))
        .map(|(k, v)| (k.file.as_deref().unwrap(), v.self_costs.clone()))
        .collect();
    assert_eq!(pieces, [("/inline.h", vec![3]), ("/main.c", vec![7])]);
    let raw = Analysis::build(&p, &p.parts[0]).unwrap();
    assert_eq!(raw.functions.len(), 2);
    assert_eq!(raw.self_totals, grouped.self_totals);
    assert_eq!(p, original);
}

const SIMPLE: &str = "events: Ir Dr\nfl=main.c\nfn=main\n1 2 1\ncfn=work\ncalls=3 2\n1 7 4\nfn=work\n2 7 4\ntotals: 9 5\n";

fn options(args: &[&str]) -> Options {
    Options::try_parse_from(
        ["annotate", "--auto=no"]
            .into_iter()
            .chain(args.iter().copied()),
    )
    .unwrap()
}
fn analysis(input: &str) -> Analysis {
    let p = parse_profile(input).unwrap();
    Analysis::build(&p, &p.parts[0]).unwrap()
}
fn values(a: &Analysis, name: &str, inclusive: bool) -> Vec<u128> {
    let key = a
        .functions
        .keys()
        .find(|k| k.name.as_deref() == Some(name))
        .unwrap();
    a.costs(key, inclusive).to_vec()
}
fn names(s: &Selection) -> Vec<&str> {
    s.functions
        .iter()
        .map(|k| k.name.as_deref().unwrap())
        .collect()
}

#[test]
fn self_and_inclusive_are_separate() {
    let a = analysis(SIMPLE);
    assert_eq!(a.self_totals, [9, 5]);
    assert_eq!(values(&a, "main", false), [2, 1]);
    assert_eq!(values(&a, "main", true), [9, 5]);
    assert_eq!(values(&a, "work", true), [7, 4]);
    assert_eq!(a.edges.values().next().unwrap().count, 3);
    assert_eq!(a.edges.values().next().unwrap().costs, [7, 4]);
}
#[test]
fn zero_count_keeps_cost_and_incoming_policy() {
    let a = analysis(&SIMPLE.replace("calls=3", "calls=0"));
    assert_eq!(a.self_totals, [9, 5]);
    assert_eq!(values(&a, "main", false), [2, 1]);
    assert_eq!(values(&a, "work", true), [7, 4]);
    assert_eq!(a.edges.values().next().unwrap().count, 0);
}
#[test]
fn recursive_annotator_policy_is_not_kcachegrind_policy() {
    let a = analysis(include_str!(
        "../../callgrind-parser/tests/fixtures/recursive-cost.callgrind"
    ));
    assert_eq!(values(&a, "recur", false), [7]);
    assert_eq!(values(&a, "recur", true), [12]);
    assert_eq!(values(&a, "main", true), [9]);
    assert_eq!(a.self_totals, [9]);
}
#[test]
fn mutual_cycle_does_not_recurse_or_double_count_self() {
    let a = analysis(include_str!(
        "../../../tests/reference/fixtures/cycle.callgrind"
    ));
    assert_eq!(values(&a, "A", true), [8]);
    assert_eq!(values(&a, "B", true), [5]);
    assert_eq!(a.self_totals, [9]);
    assert_eq!(a.edges.len(), 3);
}
#[test]
fn repeated_rows_edges_and_call_sites_sum() {
    let a = analysis(
        "events: Ir\nfl=a\nfn=f\n1 2\n1 3\ncfn=g\ncalls=2 2\n1 5\ncfn=g\ncalls=3 2\n1 7\n",
    );
    assert_eq!(values(&a, "f", false), [5]);
    assert_eq!(values(&a, "f", true), [17]);
    assert_eq!(a.lines[&(Some("a".into()), 1)], [5]);
    assert_eq!(a.edges.values().next().unwrap().count, 5);
    assert_eq!(a.call_sites.values().next().unwrap().costs, [12]);
}
#[test]
fn different_callers_on_same_line_are_retained() {
    let a =
        analysis("events: Ir\nfl=a\nfn=f\ncfn=g\ncalls=1 3\n1 2\nfn=h\ncfn=g\ncalls=1 3\n1 5\n");
    assert_eq!(a.call_sites.len(), 2);
    assert_eq!(values(&a, "g", true), [7]);
}
#[test]
fn inline_attribution_does_not_split_functions() {
    let a = analysis(include_str!(
        "../../../tests/reference/fixtures/inline-source.callgrind"
    ));
    assert_eq!(a.functions.len(), 2);
    assert_eq!(values(&a, "main", false), [10]);
    assert_eq!(a.lines[&(Some("/inline.h".into()), 10)], [3]);
}
#[test]
fn full_objects_and_paths_are_identity() {
    let a = analysis(include_str!(
        "../../../tests/reference/fixtures/basename-collision.callgrind"
    ));
    assert_eq!(a.functions.len(), 2);
    assert_eq!(a.self_totals, [5]);
    let b = analysis("events: Ir\nob=one\nfl=a\nfn=f\n1 2\nob=two\nfn=f\n1 3\n");
    assert_eq!(b.functions.len(), 2);
}
#[test]
fn missing_names_and_literal_unknown_remain_distinct() {
    let a = analysis("events: Ir\n1 2\nfl=???\nfn=???\n1 3\n");
    assert_eq!(a.functions.len(), 2);
}
#[test]
fn summary_precedence_and_fallback_are_independent_of_inclusive() {
    let input =
        "events: Ir\nsummary: 20\nfl=a\nfn=f\n1 2\ncfn=g\ncalls=1 2\n1 7\nfn=g\n2 7\ntotals: 9\n";
    assert_eq!(analysis(input).program_totals, [20]);
    assert_eq!(
        analysis(&input.replace("summary: 20", "summary: 0")).program_totals,
        [9]
    );
    let a = analysis(
        &input
            .replace("summary: 20\n", "")
            .replace("totals: 9\n", ""),
    );
    assert_eq!(a.program_totals, [9]);
    assert!(a.totals_calculated);
    assert_eq!(values(&a, "f", true), [9]);
}
#[test]
fn zero_declarations_use_safe_self_fallback() {
    let a = analysis("events: Ir\nsummary: 0\nfl=a\nfn=f\n1 3\ntotals: 0\n");
    assert_eq!(a.program_totals, [3]);
    assert!(a.totals_calculated);
    assert_eq!(a.warnings.len(), 1);
}
#[test]
fn totals_mismatch_is_reported_not_silently_rewritten() {
    let a = analysis(&SIMPLE.replace("totals: 9 5", "totals: 99 55"));
    assert_eq!(a.program_totals, [99, 55]);
    assert_eq!(a.self_totals, [9, 5]);
    assert_eq!(a.warnings.len(), 1);
}
#[test]
fn aggregation_exceeds_u64_without_losing_integer_precision() {
    let a = analysis("events: Ir\nfl=a\nfn=f\n1 18446744073709551615\n1 18446744073709551615\n");
    assert_eq!(a.self_totals, [36893488147419103230]);
    let o = options(&["--threshold=100"]);
    let s = Selection::build(&a, &o).unwrap();
    assert!(render_tsv(&a, &s, &o).contains("36893488147419103230"));
}
#[test]
fn sorting_show_order_and_ties_are_deterministic() {
    let a = analysis("events: Ir Dr\nfl=a\nfn=z\n1 5 2\nfn=b\n1 5 3\nfn=a\n1 5 3\n");
    let s = Selection::build(&a, &options(&["--threshold=100", "--show=Dr,Ir"])).unwrap();
    assert_eq!(names(&s), ["a", "b", "z"]);
    assert_eq!(s.show, [1, 0]);
    let s = Selection::build(&a, &options(&["--threshold=100", "--sort=Ir"])).unwrap();
    assert_eq!(names(&s), ["a", "b", "z"]);
}
#[test]
fn exclusive_threshold_boundaries() {
    let a = analysis("events: Ir\nfl=a\nfn=a\n1 80\nfn=b\n1 19\nfn=c\n1 1\n");
    assert_eq!(
        names(&Selection::build(&a, &options(&[])).unwrap()),
        ["a", "b"]
    );
    assert_eq!(
        names(&Selection::build(&a, &options(&["--threshold=80"])).unwrap()),
        ["a"]
    );
    assert_eq!(
        names(&Selection::build(&a, &options(&["--threshold=100"])).unwrap()),
        ["a", "b", "c"]
    );
    assert!(
        Selection::build(&a, &options(&["--threshold=0"]))
            .unwrap()
            .functions
            .is_empty()
    );
}
#[test]
fn inclusive_threshold_matches_annotator_cutoff_rule() {
    let a = analysis("events: Ir\nsummary: 100\nfl=a\nfn=a\n1 90\nfn=b\n1 9\nfn=c\n1 1\n");
    assert_eq!(
        names(&Selection::build(&a, &options(&["--inclusive=yes", "--threshold=90"])).unwrap()),
        ["a", "b"]
    );
}
#[test]
fn per_event_thresholds_override_global_threshold() {
    let a = analysis("events: Ir Dr\nfl=a\nfn=a\n1 80 1\nfn=b\n1 19 9\nfn=c\n1 1 0\n");
    let s = Selection::build(&a, &options(&["--threshold=0", "--sort=Ir:80,Dr:90"])).unwrap();
    assert_eq!(names(&s), ["a", "b"]);
}
#[test]
fn invalid_options_and_events_fail() {
    for arg in [
        "--threshold=nan",
        "--threshold=101",
        "--threshold=-1",
        "--show-percs=maybe",
        "--tree=wrong",
    ] {
        assert!(Options::try_parse_from(["annotate", arg]).is_err());
    }
    let a = analysis(SIMPLE);
    for arg in [
        "--show=Unknown",
        "--show=",
        "--show=Ir,Ir",
        "--sort=Ir,Ir",
        "--sort=Ir:nan",
        "--sort=Dr:101",
    ] {
        assert!(Selection::build(&a, &options(&[arg])).is_err());
    }
}
#[test]
fn multipart_requires_selection_and_uses_local_event_order() {
    let p = parse_profile(include_str!(
        "../../../tests/reference/fixtures/event-remapping.callgrind"
    ))
    .unwrap();
    assert!(select_part(&p, None).is_err());
    assert!(select_part(&p, Some(2)).is_err());
    let a = Analysis::build(&p, select_part(&p, Some(1)).unwrap()).unwrap();
    assert_eq!(a.events, ["Dr", "Ir", "Dw"]);
    assert_eq!(a.self_totals, [3, 5, 7]);
}
#[test]
fn jumps_do_not_contribute_to_totals_and_instruction_only_has_no_source_lines() {
    let a = analysis("positions: instr\nevents: Ir\nfl=a\nfn=f\n0x10 2\njump=4 0x20\n0x10\n");
    assert_eq!(a.self_totals, [2]);
    assert!(a.lines.is_empty());
}
#[test]
fn rendering_tree_shows_zero_count_edge() {
    let p = parse_profile(&SIMPLE.replace("calls=3", "calls=0")).unwrap();
    let a = Analysis::build(&p, &p.parts[0]).unwrap();
    let o = options(&["--tree=both", "--threshold=100"]);
    let s = Selection::build(&a, &o).unwrap();
    let text = render(&p, &p.parts[0], &a, &s, &o).unwrap();
    assert!(text.contains(">   main.c:work (0x)"));
    assert!(text.contains("< main.c:main (0x)"));
    assert!(text.contains("PROGRAM TOTALS"));
}
#[test]
fn invalid_mutated_model_width_is_an_error() {
    let mut p = parse_profile(SIMPLE).unwrap();
    p.parts[0].header.events.pop();
    assert!(Analysis::build(&p, &p.parts[0]).is_err());
}
