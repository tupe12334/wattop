# wattop

A terminal power dashboard for macOS laptops, styled like a hybrid car's energy monitor.

It shows where your watts go: from the charger, into the system and in or out of the battery. It updates live.

- **Energy flow:** animated flow between CHARGER, SYSTEM and BATTERY.
- **Battery cell:** the charge level and the state: CHARGING, HOLDING or DISCHARGING.
- **Readouts:** input, load, net battery watts, charger rating, temperature and time left.
- **Warning:** shows when the charger is too weak for the load.
- **History:** a 5-minute graph of input, load and battery watts.
- **Top power users:** the processes that use the most CPU.

## Install

```sh
cargo install --git https://github.com/tupe12334/wattop
wattop
```

Press `q` or `Esc` to quit.

## How it works

wattop reads the `AppleSmartBattery` entry from the IORegistry (`ioreg -rn AppleSmartBattery -a`). It uses the `PowerTelemetryData` fields `SystemPowerIn`, `SystemLoad` and `BatteryPower`. It needs no root access. It works only on Apple laptops. Desktop Macs have no battery entry.

## License

MIT
