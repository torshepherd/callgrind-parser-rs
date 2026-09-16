use callgrind2pprof::{Options, convert};
use clap::Parser;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, BufReader, BufWriter, Read},
    path::PathBuf,
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    version,
    about = "Export exact exclusive Callgrind costs as flat pprof samples"
)]
struct Args {
    /// Callgrind text input, or '-' for stdin.
    profile: PathBuf,
    /// New gzip output file (refuses overwrite); omit or '-' for stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Zero-based part index; mandatory for multipart input.
    #[arg(long)]
    part: Option<usize>,
    /// Declare units without scaling, e.g. --unit sysTime=microseconds.
    #[arg(long, value_name = "EVENT=UNIT")]
    unit: Vec<String>,
    #[arg(long,default_value_t=pprof_profile::DEFAULT_MAX_BYTES)]
    max_input_bytes: u64,
    #[arg(long, default_value_t = 1_000_000)]
    max_locations: usize,
}
fn input(
    reader: impl Read,
    limit: u64,
) -> Result<callgrind_parser::Profile, Box<dyn std::error::Error>> {
    let bound = limit
        .checked_add(1)
        .filter(|_| limit > 0)
        .ok_or("invalid byte limit")?;
    let mut bytes = Vec::new();
    reader.take(bound).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("input exceeds max-input-bytes".into());
    }
    Ok(callgrind_parser::parse_reader(bytes.as_slice())?)
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut units = BTreeMap::new();
    for value in args.unit {
        let (name, unit) = value
            .split_once('=')
            .filter(|(n, u)| !n.is_empty() && !u.is_empty())
            .ok_or("--unit requires EVENT=UNIT")?;
        if units.insert(name.into(), unit.into()).is_some() {
            return Err(format!("duplicate --unit for {name}").into());
        }
    }
    let profile = if args.profile.as_os_str() == "-" {
        input(io::stdin().lock(), args.max_input_bytes)?
    } else {
        input(
            BufReader::new(File::open(args.profile)?),
            args.max_input_bytes,
        )?
    };
    let report = convert(
        &profile,
        &Options {
            part: args.part,
            units,
            max_locations: args.max_locations,
        },
    )?;
    for warning in report.warnings() {
        eprintln!("warning: {warning}");
    }
    if let Some(path) = args.output.filter(|p| p.as_os_str() != "-") {
        let file = File::options().write(true).create_new(true).open(path)?;
        // Finish gzip, then explicitly flush BufWriter; Drop ignores errors.
        let mut output = BufWriter::new(file);
        report.write(&mut output)?;
        std::io::Write::flush(&mut output)?;
    } else {
        report.write(io::stdout().lock())?;
    }
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("callgrind2pprof: {e}");
            ExitCode::FAILURE
        }
    }
}
