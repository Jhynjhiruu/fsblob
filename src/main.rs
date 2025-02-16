use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use clap_num::maybe_hex;
use fsblob::{build_fs, extract_fs};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build an FS blob from files
    Build(BuildArgs),

    /// Extract files from an FS blob
    Extract(ExtractArgs),
}

#[derive(Args)]
struct BuildArgs {
    /// Files to build into the FS blob
    files: Vec<String>,

    /// Output file
    #[arg(short, long)]
    outfile: PathBuf,

    /// Size to pad to
    #[arg(short, long, value_parser = maybe_hex::<usize>)]
    pad: Option<usize>,
}

#[derive(Args)]
struct ExtractArgs {
    /// Input file
    #[arg(short, long)]
    infile: PathBuf,

    /// Output folder
    #[arg(short, long, default_value_t = String::from("fs"))]
    outdir: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build(args) => build_fs(args.files, args.outfile, args.pad),
        Commands::Extract(args) => extract_fs(args.infile, args.outdir.into()),
    }
}
