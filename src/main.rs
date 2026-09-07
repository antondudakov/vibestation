use clap::Parser;
use vibestation::host::RealHost;

/// One picker for your tmux sessions and git projects.
#[derive(Parser)]
#[command(name = "vibestation", version, about, long_about = None)]
struct Cli {}

fn main() {
    Cli::parse();
    if let Err(e) = vibestation::run(&RealHost) {
        eprintln!("vibestation: {e:#}");
        std::process::exit(1);
    }
}
