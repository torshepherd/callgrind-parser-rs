use crate::{Analysis, Identity, Options, Result, Selection, Tree, add, compare_costs};
use callgrind_parser::{Part, Profile};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

const RULE: &str =
    "--------------------------------------------------------------------------------\n";
fn safe(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
fn grouped(value: u128) -> String {
    let digits = value.to_string();
    let mut result = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
}
fn cell(value: Option<u128>, total: u128, percs: bool) -> String {
    let mut s = value.map(grouped).unwrap_or_else(|| ".".into());
    if percs {
        match value {
            Some(n) if n != 0 => {
                let percent = if total == 0 {
                    0.0
                } else {
                    n as f64 / total as f64 * 100.0
                };
                if percent < 99.995 {
                    write!(s, " ({percent:5.2}%)").unwrap();
                } else {
                    write!(s, " ({percent:5.1}%)").unwrap();
                }
            }
            _ => s.push_str("         "),
        }
    }
    s
}
struct Table<'a> {
    a: &'a Analysis,
    s: &'a Selection,
    widths: Vec<usize>,
    percs: bool,
}
impl<'a> Table<'a> {
    fn new(
        a: &'a Analysis,
        s: &'a Selection,
        options: &Options,
        rows: impl Iterator<Item = &'a [u128]>,
    ) -> Self {
        let percs = options.show_percs.enabled();
        let mut widths: Vec<_> = s.show.iter().map(|i| a.events[*i].len()).collect();
        for row in rows {
            for (j, i) in s.show.iter().enumerate() {
                widths[j] = widths[j].max(cell(Some(row[*i]), a.program_totals[*i], percs).len());
            }
        }
        Self {
            a,
            s,
            widths,
            percs,
        }
    }
    fn header(&self, out: &mut String, suffix: &str) {
        out.push_str(RULE);
        for (j, i) in self.s.show.iter().enumerate() {
            write!(
                out,
                "{:<width$} ",
                self.a.events[*i],
                width = self.widths[j]
            )
            .unwrap();
        }
        writeln!(out, "{suffix}").unwrap();
        out.push_str(RULE);
    }
    fn row(&self, out: &mut String, values: Option<&[u128]>, suffix: &str) {
        for (j, i) in self.s.show.iter().enumerate() {
            let c = cell(values.map(|v| v[*i]), self.a.program_totals[*i], self.percs);
            write!(out, "{c:>width$} ", width = self.widths[j]).unwrap();
        }
        writeln!(out, " {suffix}").unwrap();
    }
}
fn function_label(key: &Identity) -> String {
    let mut label = safe(&key.label());
    if let Some(object) = key.object.as_ref().filter(|s| !s.is_empty()) {
        write!(label, " [{}]", safe(object)).unwrap();
    }
    label
}
pub fn render(
    profile: &Profile,
    part: &Part,
    a: &Analysis,
    s: &Selection,
    options: &Options,
) -> Result<String> {
    let mut out = String::new();
    out.push_str(RULE);
    write!(
        out,
        "Profile data file '{}'",
        safe(&options.profile.display().to_string())
    )
    .unwrap();
    if let Some(creator) = &profile.header.creator {
        write!(out, " (creator: {})", safe(creator)).unwrap();
    }
    out.push('\n');
    out.push_str(RULE);
    for (key, value) in &part.header.metadata.descriptions {
        writeln!(out, "{}: {}", safe(key), safe(value)).unwrap();
    }
    let metadata = &part.header.metadata;
    let mut target = metadata
        .command
        .as_deref()
        .map(safe)
        .unwrap_or_else(|| "(unknown)".into());
    if let Some(pid) = metadata.pid {
        write!(target, " (PID {pid}").unwrap();
        if let Some(p) = metadata.part {
            write!(target, ", part {p}").unwrap();
        }
        if let Some(thread) = metadata.thread {
            write!(target, ", thread {thread}").unwrap();
        }
        target.push(')');
    }
    writeln!(out, "Profiled target:  {target}").unwrap();
    writeln!(out, "Events recorded:  {}", a.events.join(" ")).unwrap();
    writeln!(
        out,
        "Events shown:     {}",
        s.show
            .iter()
            .map(|i| a.events[*i].as_str())
            .collect::<Vec<_>>()
            .join(" ")
    )
    .unwrap();
    writeln!(
        out,
        "Event sort order: {}",
        s.sort
            .iter()
            .map(|i| a.events[*i].as_str())
            .collect::<Vec<_>>()
            .join(" ")
    )
    .unwrap();
    writeln!(
        out,
        "Thresholds:       {}",
        s.thresholds
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    )
    .unwrap();
    writeln!(
        out,
        "Include dirs:     {}",
        options
            .include
            .iter()
            .map(|p| safe(&p.display().to_string()))
            .collect::<Vec<_>>()
            .join(", ")
    )
    .unwrap();
    writeln!(
        out,
        "User annotated:   {}",
        options
            .sources
            .iter()
            .map(|p| safe(&p.display().to_string()))
            .collect::<Vec<_>>()
            .join(", ")
    )
    .unwrap();
    writeln!(
        out,
        "Auto-annotation:  {}\n",
        if options.auto.enabled() { "on" } else { "off" }
    )
    .unwrap();
    let summary = Table::new(a, s, options, std::iter::once(a.program_totals.as_slice()));
    summary.header(&mut out, "");
    summary.row(
        &mut out,
        Some(&a.program_totals),
        if a.totals_calculated {
            "PROGRAM TOTALS (calculated)"
        } else {
            "PROGRAM TOTALS"
        },
    );
    out.push('\n');
    let table = Table::new(
        a,
        s,
        options,
        a.functions
            .keys()
            .map(|k| a.costs(k, options.inclusive.enabled()))
            .chain(a.edges.values().map(|e| e.costs.as_slice())),
    );
    table.header(&mut out, " file:function");
    for key in &s.functions {
        if options.tree != Tree::None {
            out.push('\n');
        }
        if matches!(options.tree, Tree::Caller | Tree::Both) {
            tree_rows(&mut out, a, s, &table, key, true);
        }
        let marker = if options.tree != Tree::None {
            "*  "
        } else {
            ""
        };
        table.row(
            &mut out,
            Some(a.costs(key, options.inclusive.enabled())),
            &format!("{marker}{}", function_label(key)),
        );
        if matches!(options.tree, Tree::Calling | Tree::Both) {
            tree_rows(&mut out, a, s, &table, key, false);
        }
    }
    out.push('\n');
    source_annotations(&mut out, a, s, options)?;
    Ok(out)
}
fn tree_rows(
    out: &mut String,
    a: &Analysis,
    s: &Selection,
    table: &Table<'_>,
    key: &Identity,
    incoming: bool,
) {
    let mut edges: Vec<_> = a
        .edges
        .iter()
        .filter_map(|((caller, callee), edge)| {
            if incoming && callee == key {
                Some((caller, edge))
            } else if !incoming && caller == key {
                Some((callee, edge))
            } else {
                None
            }
        })
        .collect();
    edges.sort_by(|(x, a), (y, b)| {
        compare_costs(&a.costs, &b.costs, &s.sort).then_with(|| x.cmp(y))
    });
    for (other, edge) in edges {
        let label = format!(
            "{} {} ({}x){}",
            if incoming { "<" } else { ">  " },
            safe(&other.label()),
            grouped(edge.count),
            other
                .object
                .as_ref()
                .map(|o| format!(" [{}]", safe(o)))
                .unwrap_or_default()
        );
        table.row(out, Some(&edge.costs), &label);
    }
}
fn locate(path: &Path, include: &[PathBuf]) -> Result<(PathBuf, String)> {
    let relative = path.strip_prefix(Path::new("/")).unwrap_or(path);
    let candidates =
        std::iter::once(path.to_owned()).chain(include.iter().map(|dir| dir.join(relative)));
    let mut errors = Vec::new();
    for candidate in candidates {
        match fs::read_to_string(&candidate) {
            Ok(text) => return Ok((candidate, text)),
            Err(e) => errors.push(format!("{}: {e}", candidate.display())),
        }
    }
    Err(errors.join("; "))
}
fn source_annotations(
    out: &mut String,
    a: &Analysis,
    s: &Selection,
    options: &Options,
) -> Result<()> {
    let mut files: BTreeMap<String, bool> = BTreeMap::new();
    if options.auto.enabled() {
        for function in &s.functions {
            if let Some(names) = a.function_files.get(function) {
                for name in names
                    .iter()
                    .filter(|name| !name.is_empty() && *name != "???")
                {
                    files.insert(name.clone(), false);
                }
            }
        }
    }
    for path in &options.sources {
        files.insert(
            path.to_str().ok_or("source filename is not UTF-8")?.into(),
            true,
        );
    }
    let mut missing = Vec::new();
    let mut annotated = vec![0; a.events.len()];
    let mut did_annotations = false;
    for (file, explicit) in files {
        let (opened, contents) = match locate(Path::new(&file), &options.include) {
            Ok(result) => result,
            Err(e) if explicit => return Err(format!("source file {file:?} not opened: {e}")),
            Err(_) => {
                missing.push(file);
                continue;
            }
        };
        out.push_str(RULE);
        writeln!(
            out,
            "-- {}-annotated source: {}",
            if explicit { "User" } else { "Auto" },
            safe(&opened.display().to_string())
        )
        .unwrap();
        out.push_str(RULE);
        let rows: BTreeMap<_, _> = a
            .lines
            .iter()
            .filter(|((name, _), _)| name.as_deref() == Some(file.as_str()))
            .map(|((_, line), costs)| (*line, costs.as_slice()))
            .collect();
        let mut sites = a
            .call_sites
            .iter()
            .filter(|((name, _, _, _), _)| name.as_deref() == Some(file.as_str()))
            .collect::<Vec<_>>();
        sites.sort_by(|(x, a), (y, b)| {
            x.1.cmp(&y.1)
                .then_with(|| compare_costs(&a.costs, &b.costs, &s.sort))
                .then_with(|| x.cmp(y))
        });
        if rows.is_empty() && sites.is_empty() {
            writeln!(
                out,
                "  No information has been collected for {}\n",
                safe(&file)
            )
            .unwrap();
            continue;
        }
        did_annotations = true;
        let table = Table::new(
            a,
            s,
            options,
            rows.values()
                .copied()
                .chain(sites.iter().map(|(_, e)| e.costs.as_slice())),
        );
        table.header(out, " source");
        let text: Vec<_> = contents.lines().collect();
        let interesting: BTreeSet<_> = rows
            .keys()
            .copied()
            .chain(sites.iter().map(|(key, _)| key.1))
            .filter(|n| *n != 0)
            .collect();
        let mut intervals: Vec<(usize, usize)> = Vec::new();
        for &line in &interesting {
            let Ok(line) = usize::try_from(line) else {
                continue;
            };
            if line > text.len() {
                continue;
            }
            let start = line.saturating_sub(options.context).max(1);
            let end = line.saturating_add(options.context).min(text.len());
            if let Some(previous) = intervals
                .last_mut()
                .filter(|p| p.1.saturating_add(1) >= start)
            {
                previous.1 = previous.1.max(end);
            } else {
                intervals.push((start, end));
            }
        }
        for (start, end) in intervals {
            writeln!(out, "-- lines {start}-{end}").unwrap();
            for line in start..=end {
                let costs = rows.get(&(line as u64)).copied();
                table.row(out, costs, &format!("{line:>6}  {}", safe(text[line - 1])));
                if let Some(costs) = costs {
                    add(&mut annotated, costs)?;
                }
                for ((_, _, _, target), edge) in
                    sites.iter().filter(|(key, _)| key.1 == line as u64)
                {
                    table.row(
                        out,
                        Some(&edge.costs),
                        &format!("=> {} ({}x)", safe(&target.label()), grouped(edge.count)),
                    );
                }
            }
        }
        for line in interesting.into_iter().filter(|n| *n > text.len() as u64) {
            table.row(
                out,
                rows.get(&line).copied(),
                &format!("<bogus line {line}>"),
            );
            for ((_, _, _, target), edge) in sites.iter().filter(|(key, _)| key.1 == line) {
                table.row(
                    out,
                    Some(&edge.costs),
                    &format!("=> {} ({}x)", safe(&target.label()), grouped(edge.count)),
                );
            }
        }
        if let Some(costs) = rows.get(&0) {
            table.row(
                out,
                Some(costs),
                &format!("<counts for unidentified lines in {}>", safe(&file)),
            );
        }
        out.push('\n');
    }
    if !missing.is_empty() {
        out.push_str(RULE);
        out.push_str("The following files chosen for auto-annotation could not be found:\n");
        out.push_str(RULE);
        for file in missing {
            writeln!(out, "  {}", safe(&file)).unwrap();
        }
        out.push('\n');
    }
    if did_annotations {
        let table = Table::new(a, s, options, std::iter::once(annotated.as_slice()));
        table.header(out, "");
        table.row(out, Some(&annotated), "events annotated");
    }
    Ok(())
}
fn hex(value: &Option<String>) -> String {
    value
        .as_deref()
        .unwrap_or("")
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn key_fields(key: &Identity) -> String {
    format!(
        "{}\t{}\t{}",
        hex(&key.object),
        hex(&key.file),
        hex(&key.name)
    )
}
fn vector(costs: &[u128], show: &[usize]) -> String {
    show.iter()
        .map(|i| costs[*i].to_string())
        .collect::<Vec<_>>()
        .join(",")
}
/// Exact counters in selected event order. F contains the selected self or
/// inclusive policy; E/S always contain raw edge/source quantities. Names use
/// hex UTF-8, with absent names represented by empty strings at this boundary.
/// This is a report, not a lossless serialization of Profile.
pub fn render_tsv(a: &Analysis, s: &Selection, options: &Options) -> String {
    let mut out = format!(
        "P\t0\t{}\nT\t0\t{}\n",
        s.show
            .iter()
            .map(|i| hex(&Some(a.events[*i].clone())))
            .collect::<Vec<_>>()
            .join(","),
        vector(&a.program_totals, &s.show)
    );
    for key in &s.functions {
        writeln!(
            out,
            "F\t0\t{}\t{}",
            key_fields(key),
            vector(a.costs(key, options.inclusive.enabled()), &s.show)
        )
        .unwrap();
    }
    for ((caller, callee), edge) in &a.edges {
        writeln!(
            out,
            "E\t0\t{}\t{}\t{}\t{}",
            key_fields(caller),
            key_fields(callee),
            edge.count,
            vector(&edge.costs, &s.show)
        )
        .unwrap();
    }
    for ((file, line), costs) in &a.lines {
        writeln!(
            out,
            "S\t0\t{}\t{line}\t{}",
            hex(file),
            vector(costs, &s.show)
        )
        .unwrap();
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_integer_formatting() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(1234567), "1,234,567");
        assert_eq!(
            grouped(u128::MAX),
            "340,282,366,920,938,463,463,374,607,431,768,211,455"
        );
    }
    #[test]
    fn percent_edges_and_control_characters() {
        assert_eq!(cell(Some(0), 0, true), "0         ");
        assert_eq!(cell(Some(1), 0, true), "1 ( 0.00%)");
        assert_eq!(cell(Some(1), 1, true), "1 (100.0%)");
        assert_eq!(safe("a\x1b\nb"), "a\\u{1b}\\nb");
    }
}
