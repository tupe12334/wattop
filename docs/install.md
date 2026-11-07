# Install wattop

wattop has two parts. The terminal dashboard is the `wattop` command. The menu-bar app is `Wattop.app`. Each install path can give you one or both parts.

## Choose a path

| Path | You get | Command-line tool location | App location |
| --- | --- | --- | --- |
| [Release download](#path-1-release-download-dmg-or-zip) (.dmg or .zip) | The app and the bundled CLI | Link it with the menu item **Install command-line tool**, to `~/.local/bin/wattop` | `/Applications/Wattop.app` |
| [cargo install from crates.io](#path-2-cargo-install-from-cratesio) | The CLI, then the app with `wattop install-app` | `~/.cargo/bin/wattop` | `/Applications/Wattop.app` |
| [cargo install from GitHub](#path-3-cargo-install-from-github) | Same as path 2, from the `main` branch | `~/.cargo/bin/wattop` | `/Applications/Wattop.app` |
| [Build from source](#path-4-build-from-source) | The app and the CLI | `target/release/wattop`, or `cargo install --path .` | `target/release/bundle/macos/Wattop.app` |

**Requirements**

- An Apple Silicon Mac laptop. Desktop Macs have no battery data.
- macOS 10.15 or later. Use a recent macOS.
- Rust stable, for the three cargo paths. Install it with [rustup](https://rustup.rs).
- For path 4, also `tauri-cli` version 2. See [CONTRIBUTING: Setup](../CONTRIBUTING.md#setup).

Use the path that matches what you need. Most users want path 1 or path 2.

## Path 1: Release download (.dmg or .zip)

The release has two files for each version. The `.dmg` is for drag-and-drop install. The `.zip` holds the same app.

1. Open the [Releases page](https://github.com/tupe12334/wattop/releases). Pick the latest `wattop-v...` release.
2. Download `Wattop-wattop-v<version>-aarch64.dmg`.
3. Open the `.dmg`. Drag **Wattop** onto **Applications**.
4. Start the app. See [First launch](#first-launch).
5. Link the command-line tool. See [Get the CLI after a .dmg install](#get-the-cli-after-a-dmg-install).

You should see:

- A `⚡` item in the menu bar. It appears after the app starts.
- `/Applications/Wattop.app` on disk.

Check the app:

```sh
ls -d /Applications/Wattop.app
```

Expected output:

```text
/Applications/Wattop.app
```

## Path 2: cargo install from crates.io

This path installs the CLI first. Then it downloads the matching app.

1. Install the CLI:

   ```sh
   cargo install wattop
   ```

2. Install the menu-bar app:

   ```sh
   wattop install-app
   ```

3. Check the CLI:

   ```sh
   wattop --version
   ```

   You should see `wattop` and the version number, for example `wattop 0.2.0`.

4. Check the app. The app opens after the install. You should see a `⚡` item in the menu bar.

`wattop install-app` downloads `Wattop-wattop-v<version>-aarch64.app.zip` from the GitHub release. The version is the CLI version. It installs to `/Applications`. If that folder is not writable, it uses `~/Applications`.

Flags:

- `--version X` installs the app for another release. A leading `v` is allowed.
- `--no-open` installs the app and does not open it.

## Path 3: cargo install from GitHub

Use this path to get the `main` branch before a release.

1. Install the CLI from the repository:

   ```sh
   cargo install --git https://github.com/tupe12334/wattop wattop
   ```

2. Install the menu-bar app:

   ```sh
   wattop install-app
   ```

`install-app` uses the version in the source `Cargo.toml`. That version must have a GitHub release. If it does not, the download fails. Pick a released version:

```sh
wattop install-app --version 0.2.0
```

Replace `0.2.0` with a version that has a release. See the [Releases page](https://github.com/tupe12334/wattop/releases).

## Path 4: Build from source

Use this path to build both parts from a clone.

1. Clone the repository:

   ```sh
   git clone https://github.com/tupe12334/wattop
   cd wattop
   ```

2. Build the CLI and copy it into the app folder. The app bundles this file as its sidecar:

   ```sh
   cargo build --release -p wattop
   mkdir -p app/binaries
   cp target/release/wattop app/binaries/wattop-aarch64-apple-darwin
   ```

3. Install `tauri-cli` version 2, if you do not have it:

   ```sh
   cargo install tauri-cli --version "^2" --locked
   ```

4. Build the app:

   ```sh
   cd app
   cargo tauri build --bundles app
   cd ..
   ```

5. Copy the app to `/Applications`:

   ```sh
   cp -R target/release/bundle/macos/Wattop.app /Applications/
   ```

6. Put the CLI on your path. Choose one:

   ```sh
   cargo install --path .
   ```

   Or link the bundled CLI with the menu item **Install command-line tool**.

You should see `target/release/bundle/macos/Wattop.app` after step 4.

The app is not notarized. Use [First launch](#first-launch) before you open it.

## First launch

Notarized releases open with no warning. Their .dmg and app are signed with a Developer ID certificate and notarized by Apple.

Older releases, and unsigned builds, are ad-hoc signed. macOS blocks them on first launch with "Apple could not verify" or "can't be verified". This is not a damaged app. Use the steps below for those builds.

**If you used `wattop install-app`:** the command removes the quarantine flag. You do not need these steps.

**If you used the .dmg or the .zip of an older or unsigned build:** use one of these steps.

1. Right-click `Wattop.app` in `/Applications`. Choose **Open**. Click **Open** in the dialog.

   Or:

2. Try to open the app. Then open **System Settings**, go to **Privacy & Security**, and click **Open Anyway** for Wattop.

If macOS says the app is damaged, or you want to skip the dialogs, remove the quarantine flag:

```sh
xattr -dr com.apple.quarantine /Applications/Wattop.app
```

You should see the `⚡` item in the menu bar after the app starts.

## Get the CLI after a .dmg install

The .dmg app includes the `wattop` command. The app does not put it on your path. Link it:

1. Click the `⚡` item in the menu bar.
2. Click **Install command-line tool**.
3. Read the line under the menu title. You should see `CLI linked at ~/.local/bin/wattop`.

If you see `CLI linked; ~/.local/bin is not on PATH`, add the folder to your path. Use zsh:

```sh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
```

Open a new terminal window. Then check:

```sh
wattop --version
```

You should see `wattop` and the version number.

## Start at login

1. Click the `⚡` item in the menu bar.
2. Click **Launch at login**. A check mark appears.

To stop it, click **Launch at login** again. The check mark goes away.

## Update

Update each path as follows.

| Path | Command or step |
| --- | --- |
| Release download | Download the new `.dmg`. Drag Wattop to **Applications** again. Replace the old app. |
| cargo from crates.io | `cargo install wattop --force && wattop install-app` |
| cargo from GitHub | `cargo install --git https://github.com/tupe12334/wattop wattop --force && wattop install-app` |
| Build from source | Pull the new code. Repeat [Path 4](#path-4-build-from-source). |

`wattop install-app` quits the running app before it replaces the app.

## Uninstall

1. Turn off **Launch at login** in the menu. See [Start at login](#start-at-login).
2. Remove the menu-bar app. Use one of these:

   ```sh
   wattop uninstall-app
   ```

   Or drag `Wattop.app` from `/Applications` to the Trash.

   `wattop uninstall-app` removes `/Applications/Wattop.app` and `~/Applications/Wattop.app`. It does not remove the `~/.local/bin/wattop` link.

3. Remove the CLI from cargo:

   ```sh
   cargo uninstall wattop
   ```

   If you used the .dmg path, skip this step. You did not install the CLI with cargo.

4. Remove the command-line link. Only if you used **Install command-line tool**:

   ```sh
   rm ~/.local/bin/wattop
   ```

5. Remove the saved settings. This step is optional. It removes the selected menu-bar stats:

   ```sh
   rm -rf "$HOME/Library/Application Support/io.github.tupe12334.wattop"
   ```

## Troubleshooting

### No `⚡` text in the menu bar

1. Check that the app runs:

   ```sh
   pgrep -fl wattop-app
   ```

   You should see a line with the process id and `wattop-app`. If you see nothing, open `Wattop.app`.

2. Check the stats. Click the `⚡` item. Open **Show in menu bar**. Check at least one stat. With no stat checked, the title shows only `⚡`.

3. Check for hidden items. macOS hides menu bar items when the bar is full. Quit other menu bar apps, or hold ⌘ and drag other items to make room.

4. If the title shows `⚡ n/a`, sampling failed. Click the item and read the `Error:` line.

### "no AppleSmartBattery entry (desktop Mac?)"

wattop needs a laptop battery. A desktop Mac has no battery entry. The terminal dashboard shows the error in its top panel. The menu-bar app shows `Error: ...` in the first line of the menu.

### Intel Mac: "the menu-bar app is built for Apple Silicon (aarch64) only"

The app builds are Apple Silicon only. `wattop install-app` stops on an Intel Mac. The release files are named `aarch64`. Use an Apple Silicon Mac for the app.

### `install-app` fails: "download failed; is there a release for vX?"

The download failed. The release for your CLI version may not exist.

1. Open the [Releases page](https://github.com/tupe12334/wattop/releases). Find a `wattop-v...` release.
2. Install that version:

   ```sh
   wattop install-app --version 0.2.0
   ```

3. Check your network connection if the release exists.

### `install-app` fails: "neither /Applications nor ~/Applications is writable"

Make one of these folders writable. Or drag the app in by hand, using [Path 1](#path-1-release-download-dmg-or-zip).

## Not implemented

These steps are not in the code. Do not expect them to happen:

- `wattop install-app` does not link the CLI into `~/.local/bin`. Only the menu item does that.
- `wattop uninstall-app` does not remove the `~/.local/bin/wattop` link or the saved settings.
