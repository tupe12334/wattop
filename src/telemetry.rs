//! Reads Mac power telemetry from the AppleSmartBattery IORegistry entry.

use anyhow::{Context, Result, anyhow};
use plist::{Dictionary, Value};
use std::process::Command;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sample {
    /// Battery state of charge, 0-100.
    pub percent: i64,
    pub charging: bool,
    pub plugged_in: bool,
    /// Charger rating in watts (0 when unplugged or unknown).
    pub adapter_watts: i64,
    /// Watts flowing in from the charger.
    pub input_w: f64,
    /// Watts the whole system consumes.
    pub load_w: f64,
    /// Watts into the battery (positive = charging, negative = draining).
    pub battery_w: f64,
    pub temp_c: f64,
    /// Minutes to full / empty, None when macOS has no estimate.
    pub minutes_left: Option<i64>,
    /// Adapter name, e.g. "20W USB-C Power Adapter".
    pub charger_name: Option<String>,
    /// Adapter output voltage in volts.
    pub charger_volts: f64,
    /// Adapter output current in amps.
    pub charger_amps: f64,
    /// Number of power sources macOS reports.
    pub power_sources: usize,
    /// Active power-capable ports, e.g. "USB-C port 2", "MagSafe".
    pub power_ports: Vec<String>,
    /// macOS Low Power Mode on the current power source, None when unknown.
    pub low_power: Option<bool>,
}

pub fn read() -> Result<Sample> {
    let out = Command::new("ioreg")
        .args(["-rn", "AppleSmartBattery", "-a"])
        .output()
        .context("run ioreg")?;
    let mut s = parse(&out.stdout)?;
    // Port lookup is best effort: a failure only hides the port name.
    s.power_ports = read_ports().unwrap_or_default();
    Ok(s)
}

/// Reads Low Power Mode from `pmset -g`. None when pmset fails or prints no such line.
pub fn read_low_power() -> Option<bool> {
    let out = Command::new("pmset").arg("-g").output().ok()?;
    parse_low_power(&String::from_utf8_lossy(&out.stdout))
}

/// Parses the `lowpowermode` line of `pmset -g` output. 1 is on, 0 is off.
// ponytail: pmset runs every tick (1 s). It is cheap, so no cache.
pub fn parse_low_power(text: &str) -> Option<bool> {
    text.lines().find_map(
        |line| match line.split_whitespace().collect::<Vec<_>>()[..] {
            ["lowpowermode", "1"] => Some(true),
            ["lowpowermode", "0"] => Some(false),
            _ => None,
        },
    )
}

pub fn parse(bytes: &[u8]) -> Result<Sample> {
    let root = Value::from_reader(std::io::Cursor::new(bytes)).context("parse ioreg plist")?;
    let dict = root
        .as_array()
        .and_then(|a| a.first())
        .and_then(Value::as_dictionary)
        .ok_or_else(|| anyhow!("no AppleSmartBattery entry (desktop Mac?)"))?;

    let telem = dict
        .get("PowerTelemetryData")
        .and_then(Value::as_dictionary);
    let mw = |key: &str| telem.map_or(0.0, |t| int(t, key) as f64 / 1000.0);
    let minutes = int(dict, "TimeRemaining");
    let adapter = dict.get("AdapterDetails").and_then(Value::as_dictionary);

    Ok(Sample {
        percent: int(dict, "CurrentCapacity"),
        charging: bool_(dict, "IsCharging"),
        plugged_in: bool_(dict, "ExternalConnected"),
        adapter_watts: adapter.map_or(0, |a| int(a, "Watts")),
        input_w: mw("SystemPowerIn"),
        load_w: mw("SystemLoad"),
        battery_w: mw("BatteryPower"),
        temp_c: int(dict, "Temperature") as f64 / 100.0,
        // ponytail: 65535 is macOS's "still calculating" marker.
        minutes_left: (minutes > 0 && minutes < 65535).then_some(minutes),
        charger_name: adapter.and_then(adapter_name),
        charger_volts: adapter.map_or(0.0, |a| int(a, "AdapterVoltage") as f64 / 1000.0),
        charger_amps: adapter.map_or(0.0, |a| int(a, "Current") as f64 / 1000.0),
        power_sources: dict
            .get("AppleRawAdapterDetails")
            .and_then(Value::as_array)
            .map_or(0, Vec::len),
        power_ports: Vec::new(),
        low_power: None,
    })
}

/// Replaces ioreg watts with live SMC values, all or nothing.
/// Battery net becomes input - load, because ioreg's BatteryPower lags too.
/// Keeps the ioreg values when either SMC value fails the checks.
pub fn apply_live_power(s: &mut Sample, input: Option<f32>, load: Option<f32>) {
    let valid = |v: Option<f32>| v.map(f64::from).filter(|v| v.is_finite() && *v >= 0.0);
    let (Some(i), Some(l)) = (valid(input), valid(load)) else {
        return;
    };
    // A plugged-in, rated charger cannot deliver 0 W. Treat that read as bad.
    if s.plugged_in && s.adapter_watts > 0 && i == 0.0 {
        return;
    }
    s.input_w = i;
    s.load_w = l;
    s.battery_w = i - l;
}

/// Best adapter label: Name, Manufacturer, Description, else the rating.
fn adapter_name(a: &Dictionary) -> Option<String> {
    ["Name", "Manufacturer", "Description"]
        .iter()
        .find_map(|k| {
            a.get(k)
                .and_then(Value::as_string)
                .filter(|s| !s.is_empty())
        })
        .map(str::to_owned)
        .or_else(|| (int(a, "Watts") > 0).then(|| format!("{} W USB-C charger", int(a, "Watts"))))
}

/// Lists active power-capable ports from the IORegistry.
pub fn read_ports() -> Result<Vec<String>> {
    let out = Command::new("ioreg")
        .args(["-r", "-k", "PortDescription", "-d1", "-a"])
        .output()
        .context("run ioreg ports")?;
    parse_ports(&out.stdout)
}

pub fn parse_ports(bytes: &[u8]) -> Result<Vec<String>> {
    let root =
        Value::from_reader(std::io::Cursor::new(bytes)).context("parse ioreg ports plist")?;
    let Some(list) = root.as_array() else {
        return Ok(Vec::new());
    };
    Ok(list
        .iter()
        .filter_map(Value::as_dictionary)
        .filter(|d| bool_(d, "ConnectionActive"))
        .filter_map(|d| d.get("PortDescription").and_then(Value::as_string))
        .filter_map(pretty_port)
        .collect())
}

/// "Port-USB-C@2" -> "USB-C port 2"; "Port-MagSafe 3@1" -> "MagSafe"; others -> None.
fn pretty_port(raw: &str) -> Option<String> {
    let (kind, num) = raw.strip_prefix("Port-")?.split_once('@')?;
    if kind == "USB-C" {
        Some(format!("USB-C port {num}"))
    } else if kind.starts_with("MagSafe") {
        Some("MagSafe".into())
    } else {
        None
    }
}

fn int(d: &Dictionary, key: &str) -> i64 {
    match d.get(key) {
        Some(Value::Integer(i)) => i
            .as_signed()
            .unwrap_or_else(|| i.as_unsigned().unwrap_or(0) as i64),
        _ => 0,
    }
}

fn bool_(d: &Dictionary, key: &str) -> bool {
    d.get(key).and_then(Value::as_boolean).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array><dict>
  <key>CurrentCapacity</key><integer>8</integer>
  <key>IsCharging</key><true/>
  <key>ExternalConnected</key><true/>
  <key>Temperature</key><integer>3067</integer>
  <key>TimeRemaining</key><integer>65535</integer>
  <key>AdapterDetails</key><dict>
    <key>Watts</key><integer>20</integer>
    <key>Name</key><string>20W USB-C Power Adapter</string>
    <key>AdapterVoltage</key><integer>9000</integer>
    <key>Current</key><integer>2220</integer>
  </dict>
  <key>AppleRawAdapterDetails</key><array><dict/><dict/></array>
  <key>PowerTelemetryData</key><dict>
    <key>BatteryPower</key><integer>-663</integer>
    <key>SystemLoad</key><integer>7632</integer>
    <key>SystemPowerIn</key><integer>6969</integer>
  </dict>
</dict></array></plist>"#;

    const PORTS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array>
  <dict><key>PortDescription</key><string>Port-USB-C@2</string><key>ConnectionActive</key><true/></dict>
  <dict><key>PortDescription</key><string>Port-USB-C@4</string><key>ConnectionActive</key><false/></dict>
  <dict><key>PortDescription</key><string>Port-HDMI@3</string><key>ConnectionActive</key><true/></dict>
  <dict><key>PortDescription</key><string>Port-SD Card@1</string><key>ConnectionActive</key><true/></dict>
</array></plist>"#;

    #[test]
    fn parses_real_ioreg_shape() {
        let s = parse(FIXTURE.as_bytes()).unwrap();
        assert_eq!(s.percent, 8);
        assert!(s.charging && s.plugged_in);
        assert_eq!(s.adapter_watts, 20);
        assert!((s.input_w - 6.969).abs() < 1e-9);
        assert!((s.load_w - 7.632).abs() < 1e-9);
        assert!((s.battery_w + 0.663).abs() < 1e-9);
        assert!((s.temp_c - 30.67).abs() < 1e-9);
        assert_eq!(s.minutes_left, None);
        assert_eq!(s.charger_name.as_deref(), Some("20W USB-C Power Adapter"));
        assert!((s.charger_volts - 9.0).abs() < 1e-9);
        assert!((s.charger_amps - 2.22).abs() < 1e-9);
        assert_eq!(s.power_sources, 2);
    }

    #[test]
    fn live_power_needs_a_valid_pair() {
        let mut s = Sample {
            plugged_in: true,
            adapter_watts: 60,
            input_w: 1.0,
            load_w: 2.0,
            battery_w: -1.0,
            ..Default::default()
        };
        apply_live_power(&mut s, Some(58.5), Some(-30.7)); // negative load: rejected
        assert_eq!(s.load_w, 2.0);
        apply_live_power(&mut s, Some(0.0), Some(35.0)); // 0 W on a 60 W charger: rejected
        assert_eq!(s.input_w, 1.0);
        apply_live_power(&mut s, Some(58.5), Some(35.0));
        assert_eq!((s.input_w, s.load_w, s.battery_w), (58.5, 35.0, 23.5));
    }

    const PMSET_LPM_ON: &str = "System-wide power settings:\nCurrently in use:\n standby              1\n disksleep            10\n hibernatemode        3\n lowpowermode         1\n womp                 1\n";
    const PMSET_LPM_OFF: &str = "System-wide power settings:\nCurrently in use:\n standby              1\n disksleep            10\n hibernatemode        3\n lowpowermode         0\n womp                 1\n";
    const PMSET_NO_LPM: &str = "System-wide power settings:\nCurrently in use:\n standby              1\n disksleep            10\n womp                 1\n";

    #[test]
    fn parses_low_power_from_pmset() {
        assert_eq!(parse_low_power(PMSET_LPM_ON), Some(true));
        assert_eq!(parse_low_power(PMSET_LPM_OFF), Some(false));
        assert_eq!(parse_low_power(PMSET_NO_LPM), None);
    }

    #[test]
    fn parses_only_active_power_ports() {
        assert_eq!(
            parse_ports(PORTS.as_bytes()).unwrap(),
            vec!["USB-C port 2".to_string()]
        );
    }
}
