use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "export-tournament",
    about = "Write the embedded tournament.json to a local file"
)]
struct Cli {
    /// Where to write the tournament.json file.
    #[arg(long, short)]
    output: PathBuf,

    /// Tournament year.
    #[arg(long, default_value = "2026")]
    year: u16,
}

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    let json = seismic_march_madness::tournament_json(cli.year)
        .ok_or_else(|| eyre::eyre!("no embedded tournament data for year {}", cli.year))?;
    std::fs::write(&cli.output, json)?;
    println!("wrote tournament.json ({}) to {}", cli.year, cli.output.display());
    Ok(())
}
