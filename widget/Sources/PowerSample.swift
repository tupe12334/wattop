// Reads Mac power telemetry from the IORegistry, in process.
// The widget runs in the App Sandbox, which refuses the SMC, so the watts are the
// `PowerTelemetryData` values. See "Sandbox spike results" in docs/app-store.md.
// Keep the fields and rules here the same as `Sample` in src/telemetry.rs.

import Foundation
import IOKit

/// One power reading.
struct PowerSample {
    /// Which way the battery is going, as the UI shows it.
    enum Motion {
        case charging, draining, holding
    }

    /// One port that delivers power to a device.
    struct PortOut {
        let name: String
        let watts: Double
    }

    /// Battery watts within this band count as holding.
    static let epsW = 0.05

    /// Battery state of charge, 0-100.
    var percent: Int
    var pluggedIn: Bool
    var fullyCharged: Bool
    /// Charger rating in watts. 0 when unplugged or unknown.
    var adapterWatts: Int
    /// Watts flowing in from the charger.
    var inputW: Double
    /// Watts the whole system consumes.
    var loadW: Double
    /// Watts into the battery. Positive charges, negative drains.
    var batteryW: Double
    /// Minutes to full or empty. Nil when macOS has no estimate.
    var minutesLeft: Int?
    /// Active charger ports, e.g. "USB-C port 2", "MagSafe".
    var ports: [String]
    /// Ports that power a device, such as a phone.
    var portOut: [PortOut]

    /// The net way the battery goes: gain, loss or hold. The widget shows it once, as the
    /// tint of its background, and the legs of the loop follow it.
    /// - With no charger plugged in, the battery is the only source: a loss, whatever the
    ///   watts say. For some seconds after the charger is unplugged, the IORegistry still
    ///   gives the old charge watts.
    /// - A full battery on a charger holds: macOS lets it cover load peaks and then tops it
    ///   up, so the raw watts sign flips every few seconds.
    var motion: Motion {
        if !pluggedIn { return .draining }
        if fullyCharged || percent >= 100 || abs(batteryW) <= Self.epsW { return .holding }
        return batteryW > 0 ? .charging : .draining
    }

    /// Net gain: the charger fills the battery.
    var charges: Bool { motion == .charging }

    /// Net loss: the battery feeds the system, and any device. A charger may still be
    /// plugged in and cover part of the load.
    var drains: Bool { motion == .draining }

    /// Watts the battery gives. Without a drain reading, the system load stands in.
    var drainW: Double { batteryW < -Self.epsW ? -batteryW : loadW }

    /// Watts that leave the Mac through ports that power a device.
    var portOutW: Double { portOut.reduce(0) { $0 + $1.watts } }

    /// A port powers a device, such as a phone.
    var powersDevice: Bool { portOutW > Self.epsW }

    /// The charger delivers power. A plugged-in charger can deliver none: macOS then reads
    /// 0 W in while the battery drains, for example while a USB-C charger renegotiates after
    /// a device was plugged into another port. Without an input reading, a battery that
    /// does not drain means the charger carries the system.
    var supplies: Bool { pluggedIn && (inputW > Self.epsW || batteryW >= -Self.epsW) }

    /// The battery feeds a device: a port powers one and the battery gives power. The system
    /// only takes power, so the device draws from the battery, not through the system. With
    /// a charger that covers everything, the device takes its power from the charger.
    var feedsDevice: Bool { powersDevice && drains }

    /// Watts the battery sends to devices. It can give no more than it gives in all.
    var deviceFromBatteryW: Double { feedsDevice ? min(portOutW, drainW) : 0 }
}

extension PowerSample {
    /// Reads the current sample. Nil on a Mac without a battery.
    static func read() -> PowerSample? {
        let service = IOServiceGetMatchingService(
            kIOMainPortDefault, IOServiceMatching("AppleSmartBattery"))
        guard service != 0 else { return nil }
        defer { IOObjectRelease(service) }
        var props: Unmanaged<CFMutableDictionary>?
        guard
            IORegistryEntryCreateCFProperties(service, &props, kCFAllocatorDefault, 0)
                == KERN_SUCCESS,
            let battery = props?.takeRetainedValue() as? [String: Any]
        else { return nil }
        return PowerSample(battery: battery, activePorts: activePorts())
    }

    /// Builds a sample from the AppleSmartBattery properties and the active port names.
    init(battery: [String: Any], activePorts: [String]) {
        let telemetry = battery["PowerTelemetryData"] as? [String: Any] ?? [:]
        let adapter = battery["AdapterDetails"] as? [String: Any] ?? [:]
        let watts = { (key: String) in Double(int(telemetry, key)) / 1000 }
        let minutes = int(battery, "TimeRemaining")
        let out = (battery["PowerOutDetails"] as? [[String: Any]] ?? []).map {
            PortOut(name: "USB-C port \(int($0, "PortIndex"))", watts: Double(int($0, "Watts")) / 1000)
        }
        percent = int(battery, "CurrentCapacity")
        pluggedIn = battery["ExternalConnected"] as? Bool ?? false
        fullyCharged = battery["FullyCharged"] as? Bool ?? false
        adapterWatts = int(adapter, "Watts")
        inputW = watts("SystemPowerIn")
        loadW = watts("SystemLoad")
        batteryW = watts("BatteryPower")
        // 65535 is macOS's "still calculating" marker.
        minutesLeft = minutes > 0 && minutes < 65535 ? minutes : nil
        // A port in `PowerOutDetails` powers a device, even at 0 W. It is not a charger port.
        ports = activePorts.filter { port in !out.contains { $0.name == port } }
        portOut = out.filter { $0.watts > 0 }
    }
}

/// An integer property. IORegistry numbers are unsigned, so a negative value arrives wrapped.
private func int(_ dict: [String: Any], _ key: String) -> Int {
    (dict[key] as? NSNumber)?.intValue ?? 0
}

/// Lists the power-capable ports that have a live connection.
private func activePorts() -> [String] {
    var iterator: io_iterator_t = 0
    let matching = [kIOPropertyExistsMatchKey: "PortDescription"] as CFDictionary
    guard IOServiceGetMatchingServices(kIOMainPortDefault, matching, &iterator) == KERN_SUCCESS
    else { return [] }
    defer { IOObjectRelease(iterator) }
    var ports: [String] = []
    while case let entry = IOIteratorNext(iterator), entry != 0 {
        defer { IOObjectRelease(entry) }
        let property = { (key: String) in
            IORegistryEntryCreateCFProperty(entry, key as CFString, kCFAllocatorDefault, 0)?
                .takeRetainedValue()
        }
        guard property("ConnectionActive") as? Bool == true,
            let raw = property("PortDescription") as? String,
            let name = prettyPort(raw)
        else { continue }
        ports.append(name)
    }
    return ports.sorted()
}

/// "Port-USB-C@2" becomes "USB-C port 2". "Port-MagSafe 3@1" becomes "MagSafe". Others are nil.
func prettyPort(_ raw: String) -> String? {
    guard raw.hasPrefix("Port-"), let at = raw.firstIndex(of: "@") else { return nil }
    let kind = raw[raw.index(raw.startIndex, offsetBy: 5)..<at]
    let number = raw[raw.index(after: at)...]
    if kind == "USB-C" {
        return "USB-C port \(number)"
    }
    return kind.hasPrefix("MagSafe") ? "MagSafe" : nil
}
