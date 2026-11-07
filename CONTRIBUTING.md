# Contributing

Thanks for your help. Keep each pull request small and focused.

Read first:

- [README](README.md) for what wattop does and how to install it.
- [How it works](docs/how-it-works.md) for the data sources.
- [Releasing](docs/releasing.md) for how versions ship.

## Setup

You need:

- Rust stable. Install with [rustup](https://rustup.rs).
- macOS on Apple Silicon. The live readings need an Apple laptop.
- `tauri-cli` version 2, only for the menu-bar app. See [Install: build from source](docs/install.md#path-4-build-from-source).

## Run the terminal dashboard

```sh
cargo run --release
```

Press `q` or `Esc` to quit.

## Run the checks

The checks need the app sidecar file. Build it first, with step 2 of [Install: build from source](docs/install.md#path-4-build-from-source). CI does the same before its checks.

CI runs these four commands. Run them in this order before you push. See [.github/workflows/ci.yml](.github/workflows/ci.yml).

```sh
cargo fmt --all --check
cargo clippy --workspace --release -- -D warnings
cargo test --workspace
cargo publish --dry-run -p wattop
```

## Build the menu-bar app

The app needs the CLI as a sidecar first. The full build steps are in [Install: build from source](docs/install.md#path-4-build-from-source). The sidecar is set in `externalBin` in [app/tauri.conf.json](app/tauri.conf.json). The release workflow builds it the same way.

Run the app build from `app/`:

```sh
cd app
cargo tauri build --bundles app
```

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org):

```text
feat: show Low Power Mode in the menu bar
fix: keep ioreg watts when the SMC read is zero on a plugged-in Mac
docs: explain the energy flow model
chore: update dependencies
```

Use `feat`, `fix` or `docs` when the change should appear in the changelog. See [Releasing](docs/releasing.md#versioning).

## Pull requests

- Keep one change in each pull request.
- Make sure the four check commands pass.
- Update the docs in `docs/` when you change behavior they describe.

## Add a Mac model to the port outline

The outline draws only verified layouts. Do not guess a layout.

1. Get the model id:

   ```sh
   sysctl -n hw.model
   ```

2. List the active ports. Only lines with `ConnectionActive` set to true count.

   ```sh
   ioreg -r -k PortDescription
   ```

3. Find the physical position of each power port. Plug a charger into each port and check which port name becomes active. Note the side and the slot. The `PortPos` comment in [src/silhouette.rs](src/silhouette.rs) defines slot numbers.

4. Add a table next to `MAC16_1` in [src/silhouette.rs](src/silhouette.rs). Use only `MagSafe` or `USB-C port N` names. Those are the only names [src/telemetry.rs](src/telemetry.rs) produces. Leave out other ports.

5. Add a match arm in `layout()` for the model id.

6. Add a test like `usb_c_2_is_left_slot_2_of_4_on_mac16_1`. Check the slot and side of one port.

7. Run the checks.

Open the pull request with the model id and the test Mac in the description.
