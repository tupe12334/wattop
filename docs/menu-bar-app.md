# Menu-bar app

The menu-bar app shows power stats in the macOS menu bar. It has no window. Its code is in [app/src/main.rs](../app/src/main.rs).

Install steps are in [Install](install.md).

## The menu

- **Menu-bar title:** the selected stats, in menu order, separated by two spaces. It shows `⚡` when no stat is selected. It shows `⚡ n/a` when sampling fails.
- **Five info lines:** input and load, battery state, source, temperature and time left, and Low Power Mode. They are disabled, so they only display.
- **Note line:** a disabled line. It shows the result of **Install command-line tool**.
- **Show in menu bar:** a submenu with one check item per stat.
- **Open terminal dashboard:** opens the terminal dashboard. See [Open the terminal dashboard](#open-the-terminal-dashboard).
- **Install command-line tool:** links the bundled `wattop` to `~/.local/bin/wattop`. See `install()` in [app/src/cli_link.rs](../app/src/cli_link.rs).
- **Launch at login:** a check item. See [Launch at login](#launch-at-login).
- **Quit Wattop:** quits the app.

## Selectable stats

The stats are the `Stat` enum in [app/src/main.rs](../app/src/main.rs). The menu and the title use the same order.

| Stat | Menu label |
| --- | --- |
| `Input` | Input watts |
| `Load` | Load watts |
| `BatteryNet` | Battery net |
| `BatteryPct` | Battery % |
| `Charger` | Charger port |
| `Temp` | Temperature |
| `TimeLeft` | Time left |
| `LowPower` | Low Power |

The first selection is `Input`, `BatteryNet` and `BatteryPct`. See `default_selection()` in [app/src/main.rs](../app/src/main.rs).

## Where the selection is stored

The selection saves to `selection.json` in the app config directory. The file holds a JSON list of stat names. Tauri sets the directory from the app identifier, `io.github.tupe12334.wattop`. See `load_selection()` and `save_selection()` in [app/src/main.rs](../app/src/main.rs).

On macOS the path is `~/Library/Application Support/io.github.tupe12334.wattop/selection.json`.

If the file is missing or invalid, the default selection applies.

## Launch at login

The app uses `tauri-plugin-autostart` with the `LaunchAgent` launcher. The check item turns the login item on and off. See `setup()` in [app/src/main.rs](../app/src/main.rs).

## Open the terminal dashboard

The item looks for a `wattop` binary next to the app executable.

- If the binary exists, the item opens it in iTerm2.
- If not, the item runs `wattop` from `PATH` in Terminal.

See `open_dashboard()` in [app/src/main.rs](../app/src/main.rs).

## Quit

The **Quit Wattop** item calls `app.exit(0)`. The app keeps running when no window exists. It uses the accessory activation policy, so it has no Dock icon.
