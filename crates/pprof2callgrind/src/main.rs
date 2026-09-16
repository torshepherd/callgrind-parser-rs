use clap::Parser;
use pprof2callgrind::{Mode, Options, convert};
use std::{
    fs::File,
    io::{self, BufReader, BufWriter},
    path::PathBuf,
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    version,
    about = "Convert pprof protobuf/gzip to exact-integer Callgrind"
)]
struct Args {
    /// Input profile, or '-' for stdin. Legacy text pprof is not accepted.
    profile: PathBuf,
    /// New output file (refuses overwrite); omit or use '-' for stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = Mode::Graph)]
    mode: Mode,
    /// Maximum compressed AND decoded protobuf size.
    #[arg(long, default_value_t = pprof_profile::DEFAULT_MAX_BYTES)]
    max_input_bytes: u64,
    #[arg(long, default_value_t = 1_000_000)]
    max_nodes: usize,
    #[arg(long, default_value_t = 16_384)]
    max_depth: usize,
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let p = if args.profile.as_os_str() == "-" {
        pprof_profile::read(io::stdin().lock(), args.max_input_bytes)?
    } else {
        pprof_profile::read(
            BufReader::new(File::open(&args.profile)?),
            args.max_input_bytes,
        )?
    };
    let report = convert(
        &p,
        &Options {
            mode: args.mode,
            max_nodes: args.max_nodes,
            max_depth: args.max_depth,
        },
    )?;
    for warning in report.warnings() {
        eprintln!("warning: {warning}");
    }
    if let Some(path) = args.output.filter(|p| p.as_os_str() != "-") {
        let file = File::options().write(true).create_new(true).open(path)?;
        report.write(BufWriter::new(file))?;
    } else {
        report.write(io::stdout().lock())?;
    }
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("pprof2callgrind: {error}");
            ExitCode::FAILURE
        }
    }
}
