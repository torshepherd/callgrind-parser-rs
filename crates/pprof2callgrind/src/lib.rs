//! Convert pprof's observed stacks to Callgrind without floating-point scaling.
//!
//! **Experimental — documentation status:** This crate is experimental. Its
//! documentation is fully LLM-generated and may contain errors or outdated claims.
//! The documentation will receive a review and cleanup pass before the 1.0 release.
use callgrind_writer::{Event, Function, Location, Writer};
use pprof_profile::{Error, proto, string};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Mode {
    #[default]
    Graph,
    Tree,
}
#[derive(Debug, Clone)]
pub struct Options {
    pub mode: Mode,
    pub max_nodes: usize,
    pub max_depth: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            mode: Mode::Graph,
            max_nodes: 1_000_000,
            max_depth: 16_384,
        }
    }
}

// Graph identity intentionally merges duplicate location IDs with identical
// mapping/function/address/line. Column and source metadata are not extra costs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Frame {
    mapping: u64,
    function: u64,
    unknown: u64,
    address: u64,
    line: u64,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub function: Function,
    pub instruction: u64,
    pub line: u64,
    pub self_costs: Vec<u64>,
}
impl Node {
    fn location(&self) -> Location<'_> {
        Location {
            function: &self.function,
            instruction: self.instruction,
            line: self.line,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Edge {
    pub source: usize,
    pub target: usize,
    pub costs: Vec<u64>,
}
#[derive(Debug, Clone)]
pub struct Report {
    events: Vec<Event>,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    totals: Vec<u64>,
    warnings: Vec<String>,
    mode: Mode,
}

fn add(to: &mut [u64], from: &[u64]) -> Result<(), Error> {
    for (a, b) in to.iter_mut().zip(from) {
        *a = a
            .checked_add(*b)
            .ok_or_else(|| Error("Callgrind cost aggregation exceeds u64".into()))?;
    }
    Ok(())
}

struct Catalog<'a> {
    keys: Vec<Frame>,
    locations: BTreeMap<u64, Vec<usize>>,
    functions: BTreeMap<u64, &'a proto::Function>,
    mappings: BTreeMap<u64, &'a proto::Mapping>,
}
impl<'a> Catalog<'a> {
    fn new(p: &'a proto::Profile, options: &Options) -> Result<Self, Error> {
        let functions = p.function.iter().map(|f| (f.id, f)).collect();
        let mappings = p.mapping.iter().map(|m| (m.id, m)).collect();
        let mut raw = BTreeMap::new();
        let mut unique = BTreeSet::new();
        for location in &p.location {
            if location.line.len() > options.max_depth {
                return Err(Error("inline depth exceeds max-depth".into()));
            }
            let frames = if location.line.is_empty() {
                vec![Frame {
                    mapping: location.mapping_id,
                    function: 0,
                    unknown: location.id,
                    address: location.address,
                    line: 0,
                }]
            } else {
                location
                    .line
                    .iter()
                    .map(|l| {
                        Ok(Frame {
                            mapping: location.mapping_id,
                            function: l.function_id,
                            unknown: 0,
                            address: location.address,
                            line: u64::try_from(l.line).map_err(|_| {
                                Error(
                                    "negative source line cannot be represented in Callgrind"
                                        .into(),
                                )
                            })?,
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?
            };
            unique.extend(frames.iter().cloned());
            if unique.len() > options.max_nodes {
                return Err(Error("location catalog exceeds max-nodes".into()));
            }
            raw.insert(location.id, frames);
        }
        let keys: Vec<_> = unique.into_iter().collect();
        let indices: BTreeMap<_, _> = keys.iter().enumerate().map(|(i, k)| (k, i)).collect();
        let locations = raw
            .into_iter()
            .map(|(id, frames)| (id, frames.iter().map(|f| indices[f]).collect()))
            .collect();
        Ok(Self {
            keys,
            locations,
            functions,
            mappings,
        })
    }
    fn node(
        &self,
        p: &proto::Profile,
        frame: usize,
        costs: Vec<u64>,
        context: Option<usize>,
    ) -> Result<Node, Error> {
        let key = &self.keys[frame];
        let object = self
            .mappings
            .get(&key.mapping)
            .map(|m| string(p, m.filename))
            .transpose()?
            .unwrap_or("");
        let (file, name) = if key.function == 0 {
            (
                "",
                format!("<unknown> [pprof:l{}:m{}]", key.unknown, key.mapping),
            )
        } else {
            let f = self.functions[&key.function];
            let name = match string(p, f.name)? {
                "" => string(p, f.system_name)?,
                name => name,
            };
            (
                string(p, f.filename)?,
                format!(
                    "{} [pprof:f{}:m{}]",
                    if name.is_empty() { "<unknown>" } else { name },
                    key.function,
                    key.mapping
                ),
            )
        };
        let name = context.map_or_else(|| name.clone(), |id| format!("{name} [ctx:{id}]"));
        Ok(Node {
            function: Function {
                object: object.into(),
                file: file.into(),
                name,
            },
            instruction: key.address,
            line: key.line,
            self_costs: costs,
        })
    }
    fn stack(&self, sample: &proto::Sample, max: usize) -> Result<Vec<usize>, Error> {
        let mut stack = Vec::new();
        for id in sample.location_id.iter().rev() {
            for frame in self.locations[id].iter().rev() {
                if stack.len() == max {
                    return Err(Error("expanded stack exceeds max-depth".into()));
                }
                stack.push(*frame);
            }
        }
        Ok(stack)
    }
}

#[derive(Debug)]
struct TreeNode {
    frame: usize,
    flat: Vec<u64>,
    cumulative: Vec<u64>,
    children: BTreeMap<usize, usize>,
}

/// Build and validate the entire conversion before any output file is opened.
pub fn convert(p: &proto::Profile, options: &Options) -> Result<Report, Error> {
    pprof_profile::validate(p)?;
    if options.max_nodes == 0 || options.max_depth == 0 {
        return Err(Error("limits must be positive".into()));
    }
    let width = p.sample_type.len();
    let events = p
        .sample_type
        .iter()
        .enumerate()
        .map(|(i, t)| {
            Ok(Event {
                name: format!("E{i}"),
                description: format!(
                    "pprof value[{i}] type={} unit={}",
                    string(p, t.r#type)?,
                    string(p, t.unit)?
                ),
            })
        })
        .collect::<Result<_, Error>>()?;
    let catalog = Catalog::new(p, options)?;
    let mut totals = vec![0; width];
    let mut flat: BTreeMap<usize, Vec<u64>> = BTreeMap::new();
    let mut edges: BTreeMap<(usize, usize), Vec<u64>> = BTreeMap::new();
    let mut tree: Vec<TreeNode> = Vec::new();
    let mut roots: BTreeMap<usize, usize> = BTreeMap::new();
    for (i, sample) in p.sample.iter().enumerate() {
        let values = sample
            .value
            .iter()
            .map(|v| {
                u64::try_from(*v)
                    .map_err(|_| Error(format!("sample {i}: negative sample value is unsupported")))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        if values.iter().all(|v| *v == 0) {
            continue;
        }
        let stack = catalog.stack(sample, options.max_depth)?;
        if stack.is_empty() {
            return Err(Error(format!(
                "sample {i}: nonzero value has no stack location"
            )));
        }
        add(&mut totals, &values)?;
        match options.mode {
            Mode::Graph => {
                for frame in &stack {
                    flat.entry(*frame).or_insert_with(|| vec![0; width]);
                }
                add(
                    flat.get_mut(stack.last().expect("nonempty"))
                        .expect("inserted"),
                    &values,
                )?;
                // One contribution per distinct edge per sample; unlike the
                // upstream graph, retain identical-node recursive edges too.
                let distinct: BTreeSet<_> = stack.windows(2).map(|p| (p[0], p[1])).collect();
                for pair in distinct {
                    add(edges.entry(pair).or_insert_with(|| vec![0; width]), &values)?;
                }
            }
            Mode::Tree => {
                let mut parent: Option<usize> = None;
                for frame in stack {
                    let existing = if let Some(id) = parent {
                        tree[id].children.get(&frame).copied()
                    } else {
                        roots.get(&frame).copied()
                    };
                    let id = if let Some(id) = existing {
                        id
                    } else {
                        if tree.len() >= options.max_nodes {
                            return Err(Error("context tree exceeds max-nodes".into()));
                        }
                        let id = tree.len();
                        tree.push(TreeNode {
                            frame,
                            flat: vec![0; width],
                            cumulative: vec![0; width],
                            children: BTreeMap::new(),
                        });
                        if let Some(parent) = parent {
                            tree[parent].children.insert(frame, id);
                        } else {
                            roots.insert(frame, id);
                        }
                        id
                    };
                    add(&mut tree[id].cumulative, &values)?;
                    parent = Some(id);
                }
                add(&mut tree[parent.expect("nonempty")].flat, &values)?;
            }
        }
    }
    let mut nodes = Vec::new();
    let mut out_edges = Vec::new();
    match options.mode {
        Mode::Graph => {
            let mut remap = BTreeMap::new();
            for (id, costs) in flat {
                remap.insert(id, nodes.len());
                nodes.push(catalog.node(p, id, costs, None)?);
            }
            for ((a, b), costs) in edges {
                out_edges.push(Edge {
                    source: remap[&a],
                    target: remap[&b],
                    costs,
                });
            }
        }
        Mode::Tree => {
            // Canonical iterative preorder, independent of input sample order.
            let mut todo: Vec<_> = roots.values().rev().map(|id| (*id, None)).collect();
            while let Some((id, parent)) = todo.pop() {
                let node = &tree[id];
                let new_id = nodes.len();
                nodes.push(catalog.node(p, node.frame, node.flat.clone(), Some(new_id))?);
                if let Some(source) = parent {
                    out_edges.push(Edge {
                        source,
                        target: new_id,
                        costs: node.cumulative.clone(),
                    });
                }
                todo.extend(
                    node.children
                        .values()
                        .rev()
                        .map(|child| (*child, Some(new_id))),
                );
            }
        }
    }
    let mut warnings = vec!["Call counts are unknown (calls=0); legacy Perl callgrind_annotate misclassifies these edges. Addresses retain pprof semantics, not Valgrind relocation semantics.".into()];
    if p.sample.iter().any(|s| !s.label.is_empty()) {
        warnings.push(
            "Sample labels are not represented; samples differing only in labels are aggregated."
                .into(),
        );
    }
    if p.drop_frames != 0 || p.keep_frames != 0 {
        warnings.push("drop_frames/keep_frames directives are not applied: stored stacks are exported unchanged.".into());
    }
    if p.location
        .iter()
        .any(|l| l.is_folded || l.line.iter().any(|line| line.column != 0))
    {
        warnings.push("Columns and is_folded metadata are not represented in Callgrind.".into());
    }
    warnings.push("Mapping ranges/offsets/build IDs, timing/period, comments and separate system names are not serialized; value types/units are retained without scaling.".into());
    Ok(Report {
        events,
        nodes,
        edges: out_edges,
        totals,
        warnings,
        mode: options.mode,
    })
}

impl Report {
    pub fn events(&self) -> &[Event] {
        &self.events
    }
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
    pub fn totals(&self) -> &[u64] {
        &self.totals
    }
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
    pub fn mode(&self) -> Mode {
        self.mode
    }
    /// Serialize only the graph/tree already built; targets use absolute PCs
    /// and explicit callee objects. All costs retain their original units.
    pub fn write(&self, output: impl Write) -> io::Result<()> {
        let descriptions = vec![
            format!("pprof2callgrind mode={:?}; identity suffixes preserve pprof function/mapping IDs; percent-encoded names", self.mode),
            "All value columns unscaled; sampled-edge weights, not invocation counts; calls=0 means unknown".into(),
        ];
        let mut writer = Writer::new(output, &self.events, &descriptions)?;
        for n in &self.nodes {
            writer.self_cost(n.location(), &n.self_costs)?;
        }
        for e in &self.edges {
            writer.call(
                self.nodes[e.source].location(),
                self.nodes[e.target].location(),
                0,
                &e.costs,
            )?;
        }
        writer.finish()?;
        Ok(())
    }
}
