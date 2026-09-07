pub mod fake;
pub mod host;

use anyhow::Result;
use host::Host;

/// The single entry point tests drive. The picker it will open arrives with the
/// tickets that follow; for now it reports that there is nothing to open yet.
pub fn run(_host: &dyn Host) -> Result<()> {
    println!(
        "vibestation {} — no picker yet; see specs/001-vibestation-v1/README.md",
        env!("CARGO_PKG_VERSION")
    );
    Ok(())
}
