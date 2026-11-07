//! Power sampling shared by the terminal dashboard and the menu-bar app.

pub mod smc;
pub mod telemetry;

/// Reads one sample from the IORegistry, then replaces watts with live SMC values when they are valid.
pub fn sample(smc: Option<&smc::Smc>) -> anyhow::Result<telemetry::Sample> {
    let mut s = telemetry::read()?;
    if let Some(smc) = smc {
        telemetry::apply_live_power(&mut s, smc.read_f32("PDTR").ok(), smc.read_f32("PSTR").ok());
    }
    Ok(s)
}
