//! Annotation analysis and rendering over the production Callgrind parser.
//!
//! Inclusive function costs follow the annotator's incoming-edge policy, not
//! recursive graph summation or KCachegrind's cycle presentation. Raw self and
//! edge costs are always retained separately. See this crate's README.
mod render;
pub use render::{render, render_tsv};

use callgrind_parser::{FunctionId, Part, PositionKind, Profile, ProfileSymbols, Record, StringId};
use clap::{Parser, ValueEnum};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

pub type Costs = Vec<u128>;
pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum YesNo {
    Yes,
    No,
}
impl YesNo {
    pub fn enabled(self) -> bool {
        self == Self::Yes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Tree {
    None,
    Caller,
    Calling,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Text,
    Tsv,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Grouping {
    /// Keep the defining-file function identity across inline attributions.
    Function,
    /// Split functions by attribution file, like the Perl annotator.
    Source,
}

/// A Rust annotator. Cost parsing is exclusively provided by callgrind-parser.
#[derive(Parser, Clone, Debug)]
#[command(version, about, name = "callgrind-annotate")]
pub struct Options {
    /// Recorded events to display, in the requested order.
    #[arg(long)]
    pub show: Option<String>,
    /// Descending sort events; optional per-event thresholds: Ir:99,Dr:90.
    #[arg(long)]
    pub sort: Option<String>,
    /// Percentage cutoff (0..100); 100 displays every function.
    #[arg(long, default_value = "99", value_parser = percentage)]
    pub threshold: f64,
    #[arg(long, value_enum, default_value = "yes")]
    pub show_percs: YesNo,
    /// Automatically annotate source files contributing to selected functions.
    #[arg(long, value_enum, default_value = "yes")]
    pub auto: YesNo,
    #[arg(long, default_value_t = 8)]
    pub context: usize,
    #[arg(long, value_enum, default_value = "no")]
    pub inclusive: YesNo,
    #[arg(long, value_enum, default_value = "none")]
    pub tree: Tree,
    /// Additional source search directories (repeatable).
    #[arg(short = 'I', long = "include")]
    pub include: Vec<PathBuf>,
    /// Select a zero-based input part index; required for multipart profiles.
    #[arg(long)]
    pub part: Option<usize>,
    /// Machine output has hex-encoded names and exact integer counters.
    #[arg(long, value_enum, default_value = "text")]
    pub format: Format,
    #[arg(long, value_enum, default_value = "function")]
    pub grouping: Grouping,
    #[arg(default_value = "callgrind.out")]
    pub profile: PathBuf,
    /// Explicitly requested source files (missing files are errors).
    pub sources: Vec<PathBuf>,
}

fn percentage(text: &str) -> Result<f64> {
    let value: f64 = text
        .strip_suffix('%')
        .unwrap_or(text)
        .parse()
        .map_err(|_| "invalid percentage")?;
    if !value.is_finite() || !(0.0..=100.0).contains(&value) {
        return Err("percentage must be between 0 and 100".into());
    }
    Ok(value)
}

/// Full identity: neither basenames nor display placeholders are map keys.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Identity {
    pub object: Option<String>,
    pub file: Option<String>,
    pub name: Option<String>,
}

impl Identity {
    pub fn label(&self) -> String {
        format!(
            "{}:{}",
            self.file.as_deref().unwrap_or("???"),
            self.name.as_deref().unwrap_or("???")
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionCosts {
    pub self_costs: Costs,
    pub inclusive: Costs,
    incoming: Option<Costs>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    pub count: u128,
    pub costs: Costs,
}

pub type CallSite = (Option<String>, u64, Identity, Identity);

#[derive(Debug)]
pub struct Analysis {
    pub events: Vec<String>,
    pub functions: BTreeMap<Identity, FunctionCosts>,
    pub edges: BTreeMap<(Identity, Identity), Edge>,
    pub lines: BTreeMap<(Option<String>, u64), Costs>,
    pub call_sites: BTreeMap<CallSite, Edge>,
    pub function_files: BTreeMap<Identity, BTreeSet<String>>,
    pub self_totals: Costs,
    pub program_totals: Costs,
    pub totals_calculated: bool,
    pub warnings: Vec<String>,
}

pub(crate) fn add(target: &mut [u128], source: &[u128]) -> Result<()> {
    if target.len() != source.len() {
        return Err("cost vector width mismatch".into());
    }
    for (a, b) in target.iter_mut().zip(source) {
        *a = a.checked_add(*b).ok_or("cost aggregation overflow")?;
    }
    Ok(())
}

fn resolve(symbols: &ProfileSymbols, id: Option<StringId>) -> Result<Option<String>> {
    id.map(|id| {
        symbols
            .resolve(id)
            .map(str::to_owned)
            .ok_or_else(|| "invalid string ID".into())
    })
    .transpose()
}

fn identity(symbols: &ProfileSymbols, id: FunctionId) -> Result<Identity> {
    let f = symbols.function(id).ok_or("invalid function ID")?;
    Ok(Identity {
        object: resolve(symbols, f.object)?,
        file: resolve(symbols, f.file)?,
        name: resolve(symbols, f.name)?,
    })
}

pub fn select_part(profile: &Profile, index: Option<usize>) -> Result<&Part> {
    let index = match (index, profile.parts.len()) {
        (Some(i), _) => i,
        (None, 1) => 0,
        (None, 0) => return Err("profile contains no parts".into()),
        (None, _) => {
            return Err(
                "multipart profile: select one input part with --part INDEX (zero-based)".into(),
            );
        }
    };
    profile
        .parts
        .get(index)
        .ok_or_else(|| format!("part index {index} is out of range"))
}

impl Analysis {
    pub fn build(profile: &Profile, part: &Part) -> Result<Self> {
        Self::build_grouped(profile, part, Grouping::Function)
    }

    pub fn build_grouped(profile: &Profile, part: &Part, grouping: Grouping) -> Result<Self> {
        let events = part
            .header
            .events
            .iter()
            .map(|id| {
                profile
                    .symbols
                    .resolve(*id)
                    .map(str::to_owned)
                    .ok_or_else(|| "invalid event ID".to_string())
            })
            .collect::<Result<Vec<_>>>()?;
        if events.is_empty() {
            return Err("part has no recorded events".into());
        }
        let width = events.len();
        let mut a = Self {
            events,
            functions: BTreeMap::new(),
            edges: BTreeMap::new(),
            lines: BTreeMap::new(),
            call_sites: BTreeMap::new(),
            function_files: BTreeMap::new(),
            self_totals: vec![0; width],
            program_totals: vec![0; width],
            totals_calculated: false,
            warnings: Vec::new(),
        };
        for row in &part.records {
            match &row.record {
                Record::Cost { location, costs } => {
                    let mut key = identity(&profile.symbols, location.function)?;
                    if grouping == Grouping::Source {
                        key.file = resolve(&profile.symbols, location.file)?;
                    }
                    let costs: Costs = costs.iter().copied().map(u128::from).collect();
                    add(&mut a.self_totals, &costs)?;
                    let f = a
                        .functions
                        .entry(key.clone())
                        .or_insert_with(|| FunctionCosts {
                            self_costs: vec![0; width],
                            inclusive: vec![0; width],
                            incoming: None,
                        });
                    add(&mut f.self_costs, &costs)?;
                    add(&mut f.inclusive, &costs)?;
                    let file = resolve(&profile.symbols, location.file)?;
                    if let Some(file) = &file {
                        a.function_files
                            .entry(key)
                            .or_default()
                            .insert(file.clone());
                    }
                    if let Some(line) = part
                        .header
                        .position(&location.positions, PositionKind::Line)
                    {
                        add(
                            a.lines
                                .entry((file, line))
                                .or_insert_with(|| vec![0; width]),
                            &costs,
                        )?;
                    }
                }
                Record::Call {
                    source,
                    target,
                    count,
                    costs,
                } => {
                    let mut caller = identity(&profile.symbols, source.function)?;
                    let mut callee = identity(&profile.symbols, target.function)?;
                    if grouping == Grouping::Source {
                        caller.file = resolve(&profile.symbols, source.file)?;
                        callee.file = resolve(&profile.symbols, target.file)?;
                    }
                    let costs: Costs = costs.iter().copied().map(u128::from).collect();
                    for key in [&caller, &callee] {
                        a.functions
                            .entry(key.clone())
                            .or_insert_with(|| FunctionCosts {
                                self_costs: vec![0; width],
                                inclusive: vec![0; width],
                                incoming: None,
                            });
                    }
                    add(&mut a.functions.get_mut(&caller).unwrap().inclusive, &costs)?;
                    add(
                        a.functions
                            .get_mut(&callee)
                            .unwrap()
                            .incoming
                            .get_or_insert_with(|| vec![0; width]),
                        &costs,
                    )?;
                    let edge = a
                        .edges
                        .entry((caller.clone(), callee.clone()))
                        .or_insert_with(|| Edge {
                            count: 0,
                            costs: vec![0; width],
                        });
                    edge.count = edge
                        .count
                        .checked_add(u128::from(*count))
                        .ok_or("call count overflow")?;
                    add(&mut edge.costs, &costs)?;
                    let file = resolve(&profile.symbols, source.file)?;
                    if let Some(file) = &file {
                        a.function_files
                            .entry(caller.clone())
                            .or_default()
                            .insert(file.clone());
                    }
                    if let Some(line) = part.header.position(&source.positions, PositionKind::Line)
                    {
                        let site = a
                            .call_sites
                            .entry((file, line, caller, callee))
                            .or_insert_with(|| Edge {
                                count: 0,
                                costs: vec![0; width],
                            });
                        site.count = site
                            .count
                            .checked_add(u128::from(*count))
                            .ok_or("call count overflow")?;
                        add(&mut site.costs, &costs)?;
                    }
                }
                Record::Jump { .. } => {}
            }
        }
        for function in a.functions.values_mut() {
            if let Some(incoming) = function.incoming.take() {
                function.inclusive = incoming;
            }
        }
        let summary = part
            .header
            .summary
            .as_ref()
            .filter(|v| v.iter().any(|v| *v != 0));
        let totals = part.totals.as_ref().filter(|v| v.iter().any(|v| *v != 0));
        if let Some(declared) = summary.or(totals) {
            if declared.len() != width {
                return Err("declared totals width mismatch".into());
            }
            a.program_totals = declared.iter().copied().map(u128::from).collect();
        } else {
            a.program_totals = a.self_totals.clone();
            a.totals_calculated = true;
        }
        if let Some(totals) = &part.totals
            && totals.iter().copied().map(u128::from).collect::<Vec<_>>() != a.self_totals
        {
            a.warnings.push(
                "declared totals differ from summed self costs (declarations preserved)".into(),
            );
        }
        Ok(a)
    }

    pub fn costs(&self, key: &Identity, inclusive: bool) -> &[u128] {
        let f = &self.functions[key];
        if inclusive {
            &f.inclusive
        } else {
            &f.self_costs
        }
    }
}

#[derive(Debug)]
pub struct Selection {
    pub show: Vec<usize>,
    pub sort: Vec<usize>,
    pub thresholds: Vec<f64>,
    pub functions: Vec<Identity>,
}

impl Selection {
    pub fn build(a: &Analysis, options: &Options) -> Result<Self> {
        let lookup = |name: &str| {
            a.events.iter().position(|s| s == name).ok_or_else(|| format!(
            "unknown or unrecorded event {name:?}; available: {} (derived formulas are not evaluated)", a.events.join(",")))
        };
        let show = if let Some(show) = &options.show {
            show.split(',').map(lookup).collect::<Result<Vec<_>>>()?
        } else {
            (0..a.events.len()).collect()
        };
        let mut sort = Vec::new();
        let mut specified = Vec::new();
        if let Some(spec) = &options.sort {
            for item in spec.split(',') {
                let (event, cutoff) = match item.split_once(':') {
                    Some((event, cutoff)) => (event, Some(percentage(cutoff)?)),
                    None => (item, None),
                };
                sort.push(lookup(event)?);
                specified.push(cutoff);
            }
        } else {
            sort = (0..a.events.len()).collect();
            specified.resize(sort.len(), None);
        }
        if show.iter().collect::<BTreeSet<_>>().len() != show.len()
            || sort.iter().collect::<BTreeSet<_>>().len() != sort.len()
        {
            return Err("duplicate show/sort event".into());
        }
        let per_event = specified.iter().any(Option::is_some);
        let thresholds = if per_event {
            specified
                .into_iter()
                .map(|t| t.unwrap_or(0.0))
                .collect::<Vec<_>>()
        } else {
            let mut t = vec![0.0; sort.len()];
            t[0] = options.threshold;
            t
        };
        let mut functions = a.functions.keys().cloned().collect::<Vec<_>>();
        functions.sort_by(|x, y| {
            compare_costs(
                a.costs(x, options.inclusive.enabled()),
                a.costs(y, options.inclusive.enabled()),
                &sort,
            )
            .then_with(|| x.label().cmp(&y.label()))
            .then_with(|| x.cmp(y))
        });
        let mut selected = Vec::new();
        let mut progress = vec![0u128; sort.len()];
        for key in functions {
            if (per_event || options.threshold < 100.0)
                && sort.iter().enumerate().all(|(i, event)| {
                    let denominator = a.program_totals[*event].max(1) as f64;
                    progress[i] as f64 * 100.0 / denominator >= thresholds[i]
                })
            {
                break;
            }
            let costs = a.costs(&key, options.inclusive.enabled());
            for (i, event) in sort.iter().enumerate() {
                progress[i] = if options.inclusive.enabled() {
                    a.program_totals[*event].saturating_sub(costs[*event])
                } else {
                    progress[i]
                        .checked_add(costs[*event])
                        .ok_or("threshold aggregation overflow")?
                };
            }
            selected.push(key);
        }
        Ok(Self {
            show,
            sort,
            thresholds,
            functions: selected,
        })
    }
}

pub(crate) fn compare_costs(a: &[u128], b: &[u128], sort: &[usize]) -> std::cmp::Ordering {
    sort.iter()
        .map(|i| b[*i].cmp(&a[*i]))
        .find(|c| !c.is_eq())
        .unwrap_or(std::cmp::Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_add_rejects_overflow_and_bad_width() {
        assert!(
            add(&mut [u128::MAX], &[1])
                .unwrap_err()
                .contains("overflow")
        );
        assert!(add(&mut [0], &[1, 2]).unwrap_err().contains("width"));
    }
}
