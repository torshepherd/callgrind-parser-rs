#[derive(Debug)]
pub struct ProfileBuilder {
    marker: bool,
    version: Option<u64>,
    creator: Option<String>,
    header_lines: Vec<String>,
    positions: Option<Vec<String>>,
    events: Vec<String>,
    body_lines: Vec<String>,
}

impl ProfileBuilder {
    pub fn new(events: &[&str]) -> Self {
        Self {
            marker: true,
            version: None,
            creator: None,
            header_lines: Vec::new(),
            positions: None,
            events: events.iter().map(|event| (*event).to_owned()).collect(),
            body_lines: Vec::new(),
        }
    }

    pub fn version(mut self, version: u64) -> Self {
        self.version = Some(version);
        self
    }

    pub fn without_marker(mut self) -> Self {
        self.marker = false;
        self
    }

    pub fn creator(mut self, creator: &str) -> Self {
        self.creator = Some(creator.to_owned());
        self
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.header_lines.push(format!("{key}: {value}"));
        self
    }

    pub fn positions(mut self, positions: &[&str]) -> Self {
        self.positions = Some(
            positions
                .iter()
                .map(|position| (*position).to_owned())
                .collect(),
        );
        self
    }

    pub fn body(mut self, line: &str) -> Self {
        self.body_lines.push(line.to_owned());
        self
    }

    pub fn cost(mut self, positions: &[&str], costs: &[u64]) -> Self {
        let mut columns = positions
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>();
        columns.extend(costs.iter().map(ToString::to_string));
        self.body_lines.push(columns.join(" "));
        self
    }

    pub fn build(self) -> String {
        let mut lines = Vec::new();

        if self.marker {
            lines.push("# callgrind format".to_owned());
        }
        if let Some(version) = self.version {
            lines.push(format!("version: {version}"));
        }
        if let Some(creator) = self.creator {
            lines.push(format!("creator: {creator}"));
        }

        lines.extend(self.header_lines);
        if let Some(positions) = self.positions {
            lines.push(format!("positions: {}", positions.join(" ")));
        }
        lines.push(format!("events: {}", self.events.join(" ")));
        lines.push(String::new());
        lines.extend(self.body_lines);

        format!("{}\n", lines.join("\n"))
    }
}

#[test]
fn renders_a_readable_single_part_fixture() {
    let profile = ProfileBuilder::new(&["Ir", "Dr"])
        .version(1)
        .creator("unit-test")
        .positions(&["instr", "line"])
        .body("fl=main.c")
        .body("fn=main")
        .cost(&["0x10", "7"], &[4, 2])
        .build();

    assert_eq!(
        profile,
        concat!(
            "# callgrind format\n",
            "version: 1\n",
            "creator: unit-test\n",
            "positions: instr line\n",
            "events: Ir Dr\n",
            "\n",
            "fl=main.c\n",
            "fn=main\n",
            "0x10 7 4 2\n",
        )
    );
}
