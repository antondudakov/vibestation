pub mod config;
pub mod fake;
pub mod host;

use anyhow::Result;
use host::Host;

/// The single entry point tests drive. The picker it will open arrives with the
/// tickets that follow; for now it settles the config and reports what it found.
pub fn run(host: &dyn Host) -> Result<()> {
    let config = config::load_or_init(host)?;
    println!(
        "vibestation {} — {} configured; no picker yet, see specs/001-vibestation-v1/README.md",
        env!("CARGO_PKG_VERSION"),
        config
            .projects_dirs
            .iter()
            .map(|dir| dir.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(())
}
