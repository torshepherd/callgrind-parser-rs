use lasso::{Rodeo, RodeoResolver, Spur};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::sync::Arc;

/// A profile-local string identity, unrelated to IDs in compressed input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StringId(pub(crate) Spur);

/// Identity of a function within one decoded profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionId(pub(crate) u32);

/// Function identity includes its object and defining file. An inline source
/// file belongs to a Location, and must not split this function into two nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Function {
    pub object: Option<StringId>,
    pub file: Option<StringId>,
    pub name: Option<StringId>,
}

/// The mutable dictionary retained by a streaming decoder (not its records).
#[derive(Debug, Default)]
pub struct Symbols {
    pub(crate) strings: Rodeo,
    pub(crate) functions: Vec<Function>,
    pub(crate) function_ids: HashMap<Function, FunctionId>,
}

impl Symbols {
    pub fn resolve(&self, id: StringId) -> Option<&str> {
        self.strings.try_resolve(&id.0)
    }

    pub fn function(&self, id: FunctionId) -> Option<&Function> {
        self.functions.get(id.0 as usize)
    }

    pub fn string_count(&self) -> usize {
        self.strings.len()
    }

    pub fn functions(&self) -> &[Function] {
        &self.functions
    }

    pub(crate) fn freeze(self) -> ProfileSymbols {
        ProfileSymbols {
            strings: Arc::new(self.strings.into_resolver()),
            functions: self.functions.into(),
        }
    }
}

/// Read-only, owned dictionaries. The construction-time hash maps are released.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSymbols {
    strings: Arc<RodeoResolver>,
    functions: Arc<[Function]>,
}

impl ProfileSymbols {
    pub fn resolve(&self, id: StringId) -> Option<&str> {
        self.strings.try_resolve(&id.0)
    }

    pub fn function(&self, id: FunctionId) -> Option<&Function> {
        self.functions.get(id.0 as usize)
    }

    pub fn string_count(&self) -> usize {
        self.strings.len()
    }

    pub fn functions(&self) -> &[Function] {
        &self.functions
    }
}

/// Columns in the containing part's declared order; never more than three.
/// SmallVec keeps all supported position combinations inline.
pub type Positions = SmallVec<[u64; 3]>;

/// Dense values in event order. No event-name map or spare capacity per row.
pub type Costs = Box<[u64]>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PositionKind {
    Instruction,
    BasicBlock,
    Line,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub function: FunctionId,
    /// File responsible for this cost; can differ from Function.file.
    pub file: Option<StringId>,
    pub positions: Positions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    /// Exclusive cost; repeated positions are retained, not silently merged.
    Cost { location: Location, costs: Costs },
    /// An aggregate edge, not an observed stack or an additional self cost.
    Call {
        source: Location,
        target: Location,
        count: u64,
        costs: Costs,
    },
    Jump {
        source: Location,
        target: Location,
        executed: u64,
        /// None for unconditional jumps; Some(taken) for conditional jumps.
        taken: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedRecord {
    /// One-based input line; associations point to their calls/jump/jcnd line.
    pub line: usize,
    pub record: Record,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    pub pid: Option<u64>,
    pub thread: Option<u64>,
    pub part: Option<u64>,
    pub command: Option<String>,
    /// Preserve arbitrary description kinds, punctuation, and repetition.
    pub descriptions: Vec<(String, String)>,
    /// Unknown header fields are retained for producer extensions.
    pub extensions: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventTerm {
    pub coefficient: u64,
    pub event: StringId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDefinition {
    pub name: StringId,
    /// Parsed linear expression; resolution/evaluation belongs to analysis.
    pub terms: Option<Vec<EventTerm>>,
    pub long_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartHeader {
    pub metadata: Metadata,
    pub positions: Vec<PositionKind>,
    pub events: Vec<StringId>,
    pub event_definitions: Vec<EventDefinition>,
    /// May exceed the sum of emitted self costs. Never replace it with totals.
    pub summary: Option<Costs>,
}

impl Default for PartHeader {
    fn default() -> Self {
        Self {
            metadata: Metadata::default(),
            positions: vec![PositionKind::Line],
            events: Vec::new(),
            event_definitions: Vec::new(),
            summary: None,
        }
    }
}

impl PartHeader {
    pub fn position(&self, values: &Positions, kind: PositionKind) -> Option<u64> {
        self.positions
            .iter()
            .position(|k| *k == kind)
            .and_then(|index| values.get(index).copied())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub header: PartHeader,
    pub records: Vec<SpannedRecord>,
    pub totals: Option<Costs>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHeader {
    pub has_format_marker: bool,
    pub version: u64,
    pub creator: Option<String>,
}

impl Default for FileHeader {
    fn default() -> Self {
        Self {
            has_format_marker: false,
            version: 1,
            creator: None,
        }
    }
}

/// Owned semantic data for UI/analysis. It does not borrow the input text and
/// does not claim to represent full stacks, inline chains, or binary mappings
/// that Callgrind did not record. IDs are local to these dictionaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub header: FileHeader,
    pub symbols: ProfileSymbols,
    pub parts: Vec<Part>,
}

/// Events emitted without retaining previously decoded cost/association rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseEvent {
    PartStart(PartHeader),
    Record(SpannedRecord),
    PartEnd { totals: Option<Costs> },
}
