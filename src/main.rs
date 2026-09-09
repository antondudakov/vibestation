use clap::Parser;
use vibestation::host::RealHost;

/// One picker for your tmux sessions and git projects.
#[derive(Parser)]
#[command(name = "vibestation", version = VERSION, about, long_about = None)]
struct Cli {}

/// `0.1.0 (build 43)` for a binary from `scripts/build.sh`, plain `0.1.0` for
/// a local `cargo build`. The build number is the commit count, stamped in at
/// compile time, so a shipped binary can be traced to the commit that made it —
/// which its own commit hash cannot do, since the binary is part of that commit.
///
/// Composed by the build script rather than here because clap wants a
/// `&'static str` and a const cannot format one.
const VERSION: &str = match option_env!("VIBESTATION_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

fn main() {
    Cli::parse();
    if let Err(e) = vibestation::run(&RealHost) {
        eprintln!("vibestation: {e:#}");
        std::process::exit(1);
    }
}
