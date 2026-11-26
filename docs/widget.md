# Desktop widget

Wattop has a desktop widget named **Power Flow**. It is a WidgetKit extension inside `Wattop.app`. Its code is in [widget/](../widget).

The widget needs macOS 14 or later. On an older macOS the app works and the widget does not appear.

## The energy loop

Both sizes draw an energy loop, like the energy monitor of a hybrid car. The power port, SYSTEM and BATTERY are three stations on a ring. The charge level is in the middle of the ring.

A leg of the ring lights up when power moves along it. The colour names the source of the power: green comes from a charger, orange comes from the battery. Orange is also the drain colour of the terminal dashboard.

- **Port to system:** green, lit when a charger delivers power and the battery does not lose charge. On a net loss it stays dark, even when the charger covers part of the load: green beside orange reads as two answers to one question. A charger can also be plugged in and deliver none, for example while it renegotiates after a device was plugged into another port.
- **Port to battery:** green, lit on a net gain: the battery charges.
- **Battery to system:** orange, lit on a net loss: the battery drains. It is also lit when no charger is plugged in, because the battery is then the only source.
- **Battery to port:** orange, lit when a port powers a device while the battery drains. The power for the device comes out of the battery, not out of the system: the system only takes power.

A leg has no arrow. Bright pulses run along a lit leg, the way the power moves. See [Flow animation](#flow-animation).

For some seconds after a charger is unplugged, the IORegistry still gives the old charge watts. So the net motion follows the charger first and the watts second. A full battery on a charger holds, because macOS lets it cover load peaks and the watts sign flips every few seconds. The rules are `motion`, `charges` and `drains` in [widget/Sources/PowerSample.swift](../widget/Sources/PowerSample.swift). The terminal panel uses `flows()` in [src/flow.rs](../src/flow.rs), which follows the watts alone. See [Energy flow model](how-it-works.md#energy-flow-model).

## The power port station

The port is the one station where power moves both ways. Its symbol is always a plug. The lit leg and its pulses show which way the power moves:

- **Power in:** a charger feeds the Mac. The legs from the port are green.
- **Power out:** the battery feeds a device, such as a phone. The leg from the battery to the port is orange. The battery feeds the device whenever it drains, even with a charger plugged in that covers only part of the load.
- **A dim plug:** nothing is connected.

With a charger that covers everything and a device powered at the same time, the device takes its power from the charger inside the station, so no leg shows it. In the medium size, the port row then names the port the device is on, for example "Port 2 out".

## Width of a leg

The widget shows no watts. A number of watts is only right for a moment, and macOS reloads a widget at most every 20 seconds. See [When the widget reloads](#when-the-widget-reloads).

The width of the pulses on a lit leg follows the watts on it. The width grows in proportion to the watts, from 3 points near 0 W to 6.5 points at 100 W and more. The rule is `width` of `Leg` in [widget/Sources/LoopRing.swift](../widget/Sources/LoopRing.swift).

## Flow animation

Bright pulses run along a lit leg, from its source. A widget cannot run an animation of its own, so the motion is built from timeline entries:

- One reading becomes 12 entries, 2 seconds apart. Each entry turns the pulses 45 degrees further round the ring.
- A pulse is 84 degrees long, and a new one starts every 90 degrees. So the tail of a pulse ends just before the head of the pulse behind it.
- A pulse is as wide as its leg at the head. Its tail gets thinner and fades out. The shape shows the way the power moves, also in a still picture.
- Only the pulses have a colour. Under them is a thin neutral line, like an idle leg.
- The three legs drift. Each leg starts its pulses a third of a spacing after the leg before, and each leg has its own pace. So with two lit legs, one leads, then they run in step, then the other leads. Two legs come back in step about every half minute. With one pace, two lit legs move like one mirrored picture.
- A run is one unbroken motion. While a run goes on, a reload continues it: the first new entry is one step ahead of what the widget shows, and macOS animates to it. So a reload makes no stop in the motion.
- A run ends when a timeline plays out before the next reload, as it does when the widget is hidden. The next reload starts a new run. The pulses of each run are a separate view, so macOS fades to the new run. Without this, a widget that was hidden turns the pulses through every step it missed.
- A new run shows a still picture for 0.3 seconds, and then the motion starts.
- The widget keeps the start and the end of the run in its own defaults. See `FlowRun` in [widget/Sources/WattopWidget.swift](../widget/Sources/WattopWidget.swift).
- The 12 entries last 24 seconds. The app reloads the widget every 20 seconds while the user can see it. When the widget is hidden, the pulses stop on the last entry.

The settings are in `Flow` in [widget/Sources/PowerPresentation.swift](../widget/Sources/PowerPresentation.swift). The entries are made in `getTimeline` in [widget/Sources/WattopWidget.swift](../widget/Sources/WattopWidget.swift).

With **Reduce Motion** on, the legs are solid and nothing moves.

The cost is 12 drawn entries for each reload, not 1.

## One cue for each state

The tint of the background is the one sign of net gain or net loss. The lit legs show where the power moves, in the same colours. The widget has no state word, no arrow and no plus or minus sign. No text or symbol changes colour with the state.

Keep this rule when you change the widget. A second cue for the same state adds noise and no information.

VoiceOver cannot read the loop. It reads one sentence with the state and the numbers. See `spokenSummary` in [widget/Sources/PowerPresentation.swift](../widget/Sources/PowerPresentation.swift).

## Sizes

- **Small:** the loop. The charge level and the time left are in the middle.
- **Medium:** the loop with the charge level in the middle. Beside it is one row for each station. A row has the symbol and name of its station and one note: the port in use or the port a device is on, the time left, and the Mac itself.

## Background

The background is a system fill, washed from the bottom with the colour of the net motion, like the sky of the Weather widget: green on a net gain, orange on a net loss, no wash when the battery holds. See `NetBackground` in [widget/Sources/Views.swift](../widget/Sources/Views.swift).

The widget gives the background to macOS with `containerBackground`. macOS then draws the widget surface, dims widgets on the desktop, and applies the clear or tinted widget style that the user picks. Those styles drop the background, wash included. The legs still carry their colours then. Colours in the views are system styles such as `primary` and `secondary`, so they follow light and dark.

## Add the widget

1. Install `Wattop.app` in `/Applications`. See [Install](install.md).
2. Start the app one time.
3. Control-click the desktop. Choose **Edit Widgets**.
4. Search for **Wattop**. Drag **Power Flow** to the desktop or to Notification Center.

If the widget is not in the list, restart the widget service and look again:

```sh
killall chronod
```

## When the widget reloads

A widget cannot animate. macOS draws it from a saved picture of each reading. A widget also gets no event when it comes on screen. So the menu-bar app decides when the widget reloads.

The app looks at the window list once per second. It asks WidgetKit for a reload only while the user can see a desktop widget:

- It reloads when a desktop widget has been on screen for about one second. That is when the user shows the desktop, moves the window that covered it, changes Space, unlocks the screen or wakes the display. A swipe between Spaces shows the desktop for a moment. That does not count.
- It reloads every 20 seconds while a widget stays on screen. The IORegistry watts change about that often.
- It reloads at once when a charger is plugged in or unplugged, or when a port starts or stops to power a device, while a widget is on screen.
- It never reloads while every desktop widget is hidden, the screen is locked or the display sleeps.

The window list gives no widget names without the Screen Recording permission. So any desktop widget counts, not only Power Flow. A widget in Notification Center does not count.

The rule is `due()` in [app/src/widget.rs](../app/src/widget.rs). The window check and the WidgetKit call are in [app/swift/WidgetBridge.swift](../app/swift/WidgetBridge.swift). WidgetKit has no C API, so [app/build.rs](../app/build.rs) compiles that Swift file and links it into the app.

When the app does not run, the widget asks for a new reading every 15 minutes by itself. macOS can wait longer. The interval is `PowerProvider.refresh` in [widget/Sources/WattopWidget.swift](../widget/Sources/WattopWidget.swift).

The widget is not a live meter. For live numbers, use the terminal dashboard or the menu bar.

## Data source

macOS only loads a widget that runs in the App Sandbox. The sandbox refuses the SMC. See [Sandbox spike results](app-store.md#sandbox-spike-results).

The widget reads the `AppleSmartBattery` entry of the IORegistry in its own process. The watts are the `PowerTelemetryData` values. These values change about every 20 seconds. The charger port names come from the entries that have a `PortDescription` property.

See [widget/Sources/PowerSample.swift](../widget/Sources/PowerSample.swift). It follows the same rules as `Sample` in [src/telemetry.rs](../src/telemetry.rs). Change the two files together.

## Build

The build needs the Xcode command line tools. It does not need an Xcode project.

```sh
./widget/build.sh
```

The script writes `widget/build/WattopWidget.appex`. Tauri copies this folder into the app as `Contents/PlugIns/WattopWidget.appex`. The path is set in `bundle.macOS.files` in [app/tauri.conf.json](../app/tauri.conf.json). Run the script before `cargo tauri build`.

The script signs the extension with the App Sandbox entitlement in [widget/WattopWidget.entitlements](../widget/WattopWidget.entitlements). The signing identity is `WIDGET_SIGN_IDENTITY`, else `APPLE_SIGNING_IDENTITY`, else ad-hoc. An ad-hoc signed widget loads on the Mac that runs the app.

The bundle id of the extension must start with the bundle id of the app. The default is `io.github.tupe12334.wattop.widget`. Set `WIDGET_BUNDLE_ID` to change it.

## See every state

```sh
./widget/preview.sh
```

The script draws every state of the widget in both sizes. It writes `widget/build/states-dark.png` and `widget/build/states-light.png`. It compiles the real views, so the pictures follow the code. The states are the cases in [widget/Preview/States.swift](../widget/Preview/States.swift). Add a case there when the widget gains a state.

It also writes `widget/build/widget-dark.png` and `widget/build/widget-light.png`, the picture in the README. Copy them to `docs/media/` when the look of the widget changes.

The script draws a plain tile under each widget's own background, with its net tint. macOS supplies the real surface, and the script cannot show the dimmed, clear or tinted widget styles.

## Check a change

1. Build the app and copy it to `/Applications`. See [Install: build from source](install.md#path-4-build-from-source).
2. Start the app one time.
3. Open `/System/Library/CoreServices/WidgetKit Simulator.app`.
4. Choose **Wattop**, then **Power Flow**. The **Timeline** page draws the widget in each size with the current reading.

Keep only one copy of `Wattop.app` registered. A second copy, such as the one in `target/`, can hide the new widget.
