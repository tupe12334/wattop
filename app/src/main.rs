//! Menu-bar app: shows the chosen power stats in the macOS menu bar.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{
    ActivationPolicy, AppHandle, Manager, RunEvent, Wry,
    image::Image,
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use wattop::{smc, telemetry::Sample};

mod cli_link;

const TRAY_ID: &str = "wattop";
const NO_DATA_TITLE: &str = "⚡ n/a";

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(rename_all = "snake_case")]
enum Stat {
    Input,
    Load,
    BatteryNet,
    BatteryPct,
    Charger,
    Temp,
    TimeLeft,
    LowPower,
}

type Selection = BTreeSet<Stat>;

impl Stat {
    /// Menu order and title order.
    const ALL: [Stat; 8] = [
        Stat::Input,
        Stat::Load,
        Stat::BatteryNet,
        Stat::BatteryPct,
        Stat::Charger,
        Stat::Temp,
        Stat::TimeLeft,
        Stat::LowPower,
    ];

    fn label(self) -> &'static str {
        match self {
            Stat::Input => "Input watts",
            Stat::Load => "Load watts",
            Stat::BatteryNet => "Battery net",
            Stat::BatteryPct => "Battery %",
            Stat::Charger => "Charger port",
            Stat::Temp => "Temperature",
            Stat::TimeLeft => "Time left",
            Stat::LowPower => "Low Power",
        }
    }

    fn glyph(self, s: &Sample) -> String {
        match self {
            Stat::Input => format!("⚡{:.0}W", s.input_w),
            Stat::Load => format!("◆{:.0}W", s.load_w),
            Stat::BatteryNet if s.battery_w > 0.0 => format!("▲{:.0}W", s.battery_w),
            Stat::BatteryNet => format!("▼{:.0}W", -s.battery_w),
            Stat::BatteryPct => format!("{}%", s.percent),
            Stat::Charger if !s.plugged_in => "🔋".into(),
            Stat::Charger => s
                .power_ports
                .first()
                .map_or("—".into(), |p| p.replacen(" port ", " ", 1)),
            Stat::Temp => format!("{:.0}°", s.temp_c),
            Stat::TimeLeft => s
                .minutes_left
                .map_or("--:--".into(), |m| format!("{}:{:02}", m / 60, m % 60)),
            // Empty when off, so the title shows nothing for this stat.
            Stat::LowPower if s.low_power == Some(true) => "LPM".into(),
            Stat::LowPower => String::new(),
        }
    }
}

/// Menu-bar title: the selected stats in menu order, two spaces apart.
fn title(s: &Sample, sel: &Selection) -> String {
    if sel.is_empty() {
        return "⚡".into();
    }
    Stat::ALL
        .iter()
        .filter(|st| sel.contains(st))
        .map(|st| st.glyph(s))
        .filter(|g| !g.is_empty())
        .collect::<Vec<_>>()
        .join("  ")
}

/// The five disabled readout lines at the top of the menu.
fn info_lines(r: &Result<Sample, String>) -> [String; 5] {
    let s = match r {
        Ok(s) => s,
        Err(e) => {
            return [
                format!("Error: {e}"),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ];
        }
    };
    let low_power = match s.low_power {
        Some(true) => "On",
        Some(false) => "Off",
        None => "—",
    };
    let word = if s.battery_w > 0.0 {
        "charging"
    } else if s.battery_w < 0.0 {
        "draining"
    } else {
        "holding"
    };
    let source = if s.plugged_in {
        let port = s.power_ports.first().map_or("no port", String::as_str);
        let charger = s.charger_name.as_deref().unwrap_or("charger");
        format!("Source {port} · {charger} ({} W)", s.adapter_watts)
    } else {
        "Source battery".into()
    };
    let eta = s
        .minutes_left
        .map_or("--".into(), |m| format!("{}h {:02}m", m / 60, m % 60));
    [
        format!("Input {:.1} W · Load {:.1} W", s.input_w, s.load_w),
        format!("Battery {}% {word} {:+.1} W", s.percent, s.battery_w),
        source,
        format!("Temp {:.1} °C · ETA {eta}", s.temp_c),
        format!("Low Power Mode: {low_power}"),
    ]
}

/// Menu items that change every tick. Managed as app state.
struct Items {
    info: Vec<MenuItem<Wry>>,
    /// Result of the last "Install command-line tool" click.
    note: MenuItem<Wry>,
    checks: Vec<CheckMenuItem<Wry>>,
    login: CheckMenuItem<Wry>,
}

/// Latest sample and the selected stats. Managed as app state behind a Mutex.
struct Shared {
    sel: Selection,
    last: Result<Sample, String>,
}

fn default_selection() -> Selection {
    [Stat::Input, Stat::BatteryNet, Stat::BatteryPct].into()
}

fn load_selection(app: &AppHandle) -> Selection {
    app.path()
        .app_config_dir()
        .ok()
        .and_then(|d| std::fs::read_to_string(d.join("selection.json")).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(default_selection)
}

fn save_selection(app: &AppHandle, sel: &Selection) {
    let Ok(dir) = app.path().app_config_dir() else {
        return;
    };
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(sel) {
        let _ = std::fs::write(dir.join("selection.json"), json);
    }
}

/// Pushes the latest sample to the title, the info lines and the check marks.
fn refresh(app: &AppHandle) {
    let (title_text, info, sel) = {
        let st = app.state::<Mutex<Shared>>();
        let g = st.lock().unwrap_or_else(|e| e.into_inner());
        let title_text = match &g.last {
            Ok(s) => title(s, &g.sel),
            Err(_) => NO_DATA_TITLE.into(),
        };
        (title_text, info_lines(&g.last), g.sel.clone())
    };
    let items = app.state::<Items>();
    for (item, text) in items.info.iter().zip(info) {
        let _ = item.set_text(text);
    }
    for (st, check) in Stat::ALL.iter().zip(&items.checks) {
        let _ = check.set_checked(sel.contains(st));
    }
    let _ = items
        .login
        .set_checked(app.autolaunch().is_enabled().unwrap_or(false));
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_title(Some(title_text));
    }
}

/// Runs the bundled TUI in iTerm2 when present, else `wattop` from PATH in Terminal.
fn open_dashboard() {
    let _ = match cli_link::bundled_cli() {
        Some(p) => {
            let cmd = p
                .display()
                .to_string()
                .replace('\\', "\\\\")
                .replace('"', "\\\"");
            Command::new("osascript")
                .args(["-e", &format!(r#"tell application "iTerm2" to create window with default profile command "{cmd}""#)])
                .spawn()
        }
        None => Command::new("osascript")
            .args(["-e", r#"tell application "Terminal" to do script "wattop""#])
            .spawn(),
    };
}

fn on_menu(app: &AppHandle, id: &str) {
    let stat = id
        .strip_prefix("stat:")
        .and_then(|i| i.parse::<usize>().ok())
        .and_then(|i| Stat::ALL.get(i).copied());
    if let Some(st) = stat {
        {
            let state = app.state::<Mutex<Shared>>();
            let mut g = state.lock().unwrap_or_else(|e| e.into_inner());
            if !g.sel.remove(&st) {
                g.sel.insert(st);
            }
            save_selection(app, &g.sel);
        }
        refresh(app);
        return;
    }
    match id {
        "login" => {
            let al = app.autolaunch();
            let _ = if al.is_enabled().unwrap_or(false) {
                al.disable()
            } else {
                al.enable()
            };
            refresh(app);
        }
        "open" => open_dashboard(),
        "install_cli" => {
            let msg = cli_link::install();
            let _ = app.state::<Items>().note.set_text(msg);
        }
        "quit" => app.exit(0),
        _ => {}
    }
}

fn setup(app: &mut tauri::App) -> tauri::Result<()> {
    app.set_activation_policy(ActivationPolicy::Accessory);
    let handle = app.handle().clone();
    let sel = load_selection(&handle);

    let info = (0..5)
        .map(|i| MenuItem::with_id(app, format!("info{i}"), "Loading…", false, None::<&str>))
        .collect::<tauri::Result<Vec<_>>>()?;
    let checks = Stat::ALL
        .iter()
        .enumerate()
        .map(|(i, st)| {
            CheckMenuItem::with_id(
                app,
                format!("stat:{i}"),
                st.label(),
                true,
                sel.contains(st),
                None::<&str>,
            )
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let login_on = app.autolaunch().is_enabled().unwrap_or(false);
    let login = CheckMenuItem::with_id(
        app,
        "login",
        "Launch at login",
        true,
        login_on,
        None::<&str>,
    )?;
    let note = MenuItem::with_id(app, "note", "", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open terminal dashboard", true, None::<&str>)?;
    let install_cli = MenuItem::with_id(
        app,
        "install_cli",
        "Install command-line tool",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit Wattop", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let check_refs: Vec<&dyn IsMenuItem<Wry>> = checks.iter().map(|c| c as _).collect();
    let show = Submenu::with_items(app, "Show in menu bar", true, &check_refs)?;

    let mut top: Vec<&dyn IsMenuItem<Wry>> = info.iter().map(|i| i as _).collect();
    top.extend([
        &note as &dyn IsMenuItem<Wry>,
        &sep,
        &show,
        &sep,
        &open,
        &install_cli,
        &login,
        &quit,
    ]);
    let menu = Menu::with_items(app, &top)?;

    app.manage(Mutex::new(Shared {
        sel,
        last: Err("loading".into()),
    }));
    app.manage(Items {
        info,
        note,
        checks,
        login,
    });

    let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, ev| on_menu(app, ev.id.as_ref()))
        .build(app)?;

    // Same sampling as the TUI: the SMC is opened once and read every tick.
    let smc = smc::Smc::open().ok();
    std::thread::spawn(move || {
        loop {
            let r = wattop::sample(smc.as_ref()).map_err(|e| format!("{e:#}"));
            handle
                .state::<Mutex<Shared>>()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .last = r;
            refresh(&handle);
            std::thread::sleep(Duration::from_secs(1));
        }
    });
    refresh(app.handle());
    Ok(())
}

fn main() -> tauri::Result<()> {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| Ok(setup(app)?))
        .build(tauri::generate_context!())?;
    app.run(|_, event| {
        // Tray-only: keep running when no window exists.
        if let RunEvent::ExitRequested {
            api, code: None, ..
        } = event
        {
            api.prevent_exit();
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Sample {
        Sample {
            percent: 9,
            plugged_in: true,
            input_w: 58.4,
            load_w: 42.0,
            battery_w: 16.4,
            temp_c: 30.7,
            minutes_left: Some(235),
            power_ports: vec!["USB-C port 2".into()],
            ..Default::default()
        }
    }

    fn sel(stats: &[Stat]) -> Selection {
        stats.iter().copied().collect()
    }

    #[test]
    fn title_charging_with_defaults() {
        assert_eq!(title(&sample(), &default_selection()), "⚡58W  ▲16W  9%");
    }

    #[test]
    fn title_draining() {
        let s = Sample {
            battery_w: -2.4,
            ..sample()
        };
        assert_eq!(title(&s, &default_selection()), "⚡58W  ▼2W  9%");
    }

    #[test]
    fn title_unplugged_shows_battery_icon_for_port() {
        let s = Sample {
            plugged_in: false,
            battery_w: -5.0,
            ..sample()
        };
        assert_eq!(title(&s, &sel(&[Stat::Charger])), "🔋");
    }

    #[test]
    fn title_all_stats_in_menu_order() {
        let all: Vec<Stat> = Stat::ALL.to_vec();
        assert_eq!(
            title(&sample(), &sel(&all)),
            "⚡58W  ◆42W  ▲16W  9%  USB-C 2  31°  3:55"
        );
    }

    #[test]
    fn title_nothing_selected() {
        assert_eq!(title(&sample(), &Selection::new()), "⚡");
    }

    #[test]
    fn title_low_power_shows_lpm_only_when_on() {
        let off = sample();
        assert_eq!(title(&off, &sel(&[Stat::LowPower])), "");
        let on = Sample {
            low_power: Some(true),
            ..sample()
        };
        assert_eq!(title(&on, &sel(&[Stat::LowPower])), "LPM");
        assert_eq!(
            title(&on, &sel(&[Stat::Input, Stat::LowPower])),
            "⚡58W  LPM"
        );
    }
}
