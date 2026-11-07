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

wattop reads live input power (`PDTR`) and system power (`PSTR`) from the SMC every second. It reads the battery state, the charger details and the active port from the IORegistry (`AppleSmartBattery` and the `PortDescription` entries). If an SMC value fails its checks, wattop uses the IORegistry `PowerTelemetryData` value. That value updates only about every 20 s. wattop needs no root access. It works only on Apple laptops, because desktop Macs have no battery entry.

## License

MIT
