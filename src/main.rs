//! Dynamic (subprocess) entrypoint for the bazarr plugin.
//!
//! Service-backend only — deploy/backup/restore of the Bazarr instance
//! itself. No `#[orca_tool]` surface to force-link.
plugin_toolkit::instrument::bootstrap!();
use plugin_toolkit::plugin::Plugin;
fn main() -> plugin_toolkit::anyhow::Result<()> {
    Plugin::named("bazarr")
        .version(env!("CARGO_PKG_VERSION"))
        .service(bazarr::BazarrServiceBackend::new("bazarr"))
        .serve()
}
