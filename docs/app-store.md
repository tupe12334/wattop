# Mac App Store readiness: Wattop

Checked on 2026-11-07 against the code in `app/` and the root crate `wattop`.

- App: Wattop, bundle id `io.github.tupe12334.wattop`, Tauri 2 (`tauri` 2.12.5 in `Cargo.lock`), menu-bar only, Apple silicon only.
- Status key: **Done**, **Missing** (code or file work needed), **Blocked** (needs the sandbox spike result or an owner action).
- **Unverified** means the Apple page I could read had only a title, or it returned an error. Check these before you rely on them.

## Summary: open items

**Missing**

- App Sandbox entitlement (also Blocked by the spike)
- Hardened runtime, signed with a real identity
- Entitlements file
- Privacy manifest (`PrivacyInfo.xcprivacy`)
- `LSApplicationCategoryType` and `LSUIElement` (no `Info.plist`)
- `bundle.category` is `DeveloperTool`, should be `Utility`
- Copyright and build number (`CFBundleVersion`)
- Launch at login uses a LaunchAgent, must move to SMAppService
- Apple Events entitlement and usage text (the `osascript` calls)
- `~/.local/bin` link writes outside the container
- Sidecar signing with `com.apple.security.inherit`
- Sidecar contains `install-app`, which downloads and runs code
- `minimumSystemVersion` is 10.15
- Export compliance key
- Privacy policy page and support URL
- App Store screenshots, description, keywords, subtitle, age rating, review notes
- Universal binary decision

**Blocked**

- No private APIs: the SMC user client (`src/smc.rs`). Result: see "Sandbox spike results" below.
- Owner actions: Developer Program enrollment, App ID and app record, certificates, provisioning profile.

## 1. Distribution options

| | Mac App Store | Developer ID + notarization (outside the store) |
| --- | --- | --- |
| Sandbox | Required. Guideline 2.4.5(i): "They must be appropriately sandboxed, and follow macOS File System Documentation." ([guidelines](https://developer.apple.com/app-store/review/guidelines/)) | Not stated on the Developer ID page I could read. Unverified. ([Developer ID](https://developer.apple.com/developer-id/)) |
| Apple account | Apple Developer Program membership ([Tauri MAS steps](https://v2.tauri.app/distribute/app-store/)) | Same membership |
| App signing certificate | **Apple Distribution** ([Tauri signing](https://v2.tauri.app/distribute/sign/macos/)) | **Developer ID Application** ([Tauri signing](https://v2.tauri.app/distribute/sign/macos/), [Developer ID](https://developer.apple.com/developer-id/)) |
| Installer certificate | **Mac Installer Distribution**. The older name is "3rd Party Mac Developer Installer". An Apple forum thread says both names refer to the same identity. ([forum 751198](https://developer.apple.com/forums/thread/751198)) | Not needed for a DMG or zip |
| Provisioning profile | Mac App Store Connect profile for the App ID, embedded as `embedded.provisionprofile` ([Tauri MAS steps](https://v2.tauri.app/distribute/app-store/)) | Not needed |
| Notarization | Not part of the store path | Recommended by Apple. Tauri says a free account cannot notarize. Credentials: App Store Connect API key or Apple ID ([Tauri signing](https://v2.tauri.app/distribute/sign/macos/)) |
| Review | Apple reviews each release | No store review |
| User warning | None from the store install | Signing lets Gatekeeper verify the developer. Tauri says a free account cannot notarize, so the app still shows as unverified. Exact Gatekeeper behavior: unverified. |
| Delivery | App Store Connect upload (`.pkg`) | Release files (`.dmg`, `.zip`) |

Notes:

- The store path needs the sandbox. The sandbox is the largest task (see section 2).
- The Developer ID path avoids the sandbox. Its release files still need real signing and notarization. Today the release job uses ad-hoc signing (`signingIdentity: "-"`) and skips notarization. See `.github/workflows/release.yml` and `docs/releasing.md`.
- Pick the path after the sandbox spike in section 5. If the SMC read fails in the sandbox, use Developer ID.

## 2. Checklist

| # | Item | Status | Evidence in the current code | Needed | Source |
| --- | --- | --- | --- | --- | --- |
| 1 | App Sandbox entitlement | Blocked | No entitlements file. `tauri.conf.json` has no `entitlements` key. Four code paths conflict with the sandbox: the LaunchAgent (row 11), the `~/.local/bin` link (row 13), Apple Events (row 12), and SMC IOKit calls (row 9). | Set `com.apple.security.app-sandbox` to `true`. Pass the spike (section 5). | [Guidelines 2.4.5(i)](https://developer.apple.com/app-store/review/guidelines/). App Sandbox page: unverified (title only). |
| 2 | Hardened runtime | Missing | `tauri.conf.json` has no `hardenedRuntime` key. Tauri default is `true`. Signing is ad-hoc (`"signingIdentity": "-"`), so no real identity is applied. | Sign with a real identity. Check the built app with `codesign -dv --verbose=4`. | [Tauri config](https://v2.tauri.app/reference/config/). Store requirement: unverified. |
| 3 | Entitlements file | Missing | No `*.entitlements` file in `app/`. | Add `app/Wattop.entitlements` (proposal in section 4.1). | [Tauri MAS steps](https://v2.tauri.app/distribute/app-store/), [Tauri config](https://v2.tauri.app/reference/config/) (`bundle.macOS.entitlements`) |
| 4 | Privacy manifest (required-reason APIs) | Missing | No `PrivacyInfo.xcprivacy`. Reasons found in the code: **System Boot Time**. Rust std `Instant` uses `mach_absolute_time` on Apple ([std `sys/time/unix.rs`](https://github.com/rust-lang/rust/blob/stable/library/std/src/sys/time/unix.rs)). sysinfo 0.40.1 reads `KERN_BOOTTIME` and calls `mach_absolute_time` ([sysinfo `unix/apple/system.rs`](https://docs.rs/crate/sysinfo/0.40.1/source/src/unix/apple/system.rs)). No file-timestamp, disk-space, or user-defaults calls found in `src/` or `app/src/`. No `NSUserDefaults` in the tao, wry, or tauri sources. Other dependencies: unverified. | Add the manifest. Declare `NSPrivacyAccessedAPICategorySystemBootTime` with reason `35F9.1`. Verify the reason code on Apple's list. Check the built binary with `nm`. | [Apple news: approved reasons](https://developer.apple.com/news/upcoming-requirements/?id=03012024a) (quoted: approved reasons are needed "to upload a new or updated app"). Category list and codes: unverified (page title only). Reason `35F9.1` came from a [forum summary](https://developer.apple.com/forums/thread/749799). |
| 5 | `LSApplicationCategoryType` | Missing | No `Info.plist` in `app/`. | Add `public.app-category.utilities` through `bundle.macOS.infoPlist`. | [Tauri config](https://v2.tauri.app/reference/config/) (`infoPlist`). Key definition: unverified (title only). |
| 6 | `bundle.category` in `tauri.conf.json` | Missing | `"category": "DeveloperTool"`. | Change to `"Utility"`. | [Tauri config](https://v2.tauri.app/reference/config/) (accepted values), [Tauri MAS steps](https://v2.tauri.app/distribute/app-store/) (example `"Utility"`). |
| 7 | 1024 px icon and `.icns` | Done | `app/icons/icon.icns` has a 1024 x 1024 image (`icon_512x512@2x`, checked with `iconutil`). `app/icons/icon.png` is 512 x 512. | None. | Local check. |
| 8 | Copyright and versioning | Missing | No `bundleVersion`, so Tauri uses the package version `0.2.0` for `CFBundleVersion`. No copyright string. | Keep `CFBundleShortVersionString` = `0.2.0`. Set `bundleVersion` in the store config to a number that grows each upload (for example `"2"`). Add `NSHumanReadableCopyright` through the Info.plist merge. | [Tauri config](https://v2.tauri.app/reference/config/) (`bundleVersion` maps to `CFBundleVersion`). "Build number must grow": unverified (title only). |
| 9 | No private APIs | Blocked | `src/smc.rs` links IOKit and calls `IOServiceOpen` and `IOConnectCallStructMethod` on `AppleSMC`. Other reads spawn `ioreg`, `pmset`, and `sysctl`. | Pending spike (section 5). If the SMC user client is refused or not public, the store build cannot read sensors. | [Guideline 2.5.1](https://developer.apple.com/app-store/review/guidelines/): "Apps may only use public APIs and must run on the currently shipping OS." Spike: see "Sandbox spike results" below. |
| 10 | No self-updating or downloading code | Done (app); Missing (sidecar) | The app has no updater plugin (`app/Cargo.toml`). No network calls in `app/src/` or `src/`. The CLI `install-app` downloads a `.app.zip` with `curl` and runs `ditto`, `xattr`, and `open` (`src/install_app.rs`). That CLI ships inside the bundle as the sidecar. | Build the store sidecar without `install-app`, or remove it from the store build. | [Guideline 2.5.2](https://developer.apple.com/app-store/review/guidelines/): apps "may not download, install, or execute code which introduces or changes features or functionality of the app." |
| 11 | Launch at login via SMAppService | Missing | `tauri-plugin-autostart` with `MacosLauncher::LaunchAgent` writes a plist under `~/Library/LaunchAgents` ([plugin source](https://docs.rs/crate/tauri-plugin-autostart/2.7.0/source/src/lib.rs)). That path is outside the container. | Replace with `SMAppService.mainApp` register and unregister. Remove the plugin from the store build. The toggle is user-controlled, which meets consent. | [Guideline 2.4.5(iii)](https://developer.apple.com/app-store/review/guidelines/): no auto-launch "without consent." [Tauri autostart](https://v2.tauri.app/plugin/autostart/) shows only `LaunchAgent`. SMAppService page: unverified (timed out). |
| 12 | Apple Events usage | Missing | `app/src/main.rs` runs `osascript` to tell iTerm2 or Terminal to run `wattop`. | Add `com.apple.security.automation.apple-events` and `NSAppleEventsUsageDescription`, or remove the item from the store build. | Entitlement and key pages: unverified (title only). |
| 13 | Writes outside the container (`~/.local/bin`) | Missing | `app/src/cli_link.rs` creates `$HOME/.local/bin/wattop`. A sandboxed `HOME` is the container, and writes outside it are denied. | Remove "Install command-line tool" from the store build. | [Guideline 2.5.2](https://developer.apple.com/app-store/review/guidelines/): no reads or writes "outside the designated container area." |
| 14 | Sidecar signing (`com.apple.security.inherit`) | Missing | `tauri.conf.json`: `"externalBin": ["binaries/wattop"]`. No sidecar entitlements. | Sign the sidecar with the sandbox and inherit entitlements (proposal in section 4.1). Check the built bundle. | Inherit page returned HTTP 404 (unverified). Sandbox page: unverified. |
| 15 | Minimum macOS version | Missing | `"minimumSystemVersion": "10.15"`. README says macOS 10.15 or later. The app is Apple silicon only. | Set `11.0`. Apple silicon shipped with macOS 11 (from Apple product history; unverified here). Set `13.0` if SMAppService is used (unverified). | [Tauri config](https://v2.tauri.app/reference/config/) (key and default). |
| 16 | Export compliance | Missing | No `Info.plist`. The app does no custom encryption. The only network call is HTTPS in the CLI `install-app`, through `curl`. | After the owner confirms, add `ITSAppUsesNonExemptEncryption` = `false`. | [Tauri MAS steps](https://v2.tauri.app/distribute/app-store/) (key in `Info.plist`). Key definition: unverified (title only). |
| 17 | Privacy policy URL | Missing | None. The app writes one local settings file and makes no network calls. | Owner hosts the policy page. Add the URL in App Store Connect. Add a menu item that opens it, so the link is "within the app". | [Guideline 5.1.1(i)](https://developer.apple.com/app-store/review/guidelines/): link "in the App Store Connect metadata field and within the app." |
| 18 | Support URL | Missing | None. | Owner sets a support URL. | App Store Connect metadata: unverified. |
| 19 | App Store screenshots | Missing | No store screenshots in the repo. | Make at least one. Mac sizes: 1280 x 800, 1440 x 900, 2560 x 1600, 2880 x 1800. PNG or JPEG, no alpha. Up to 10. | [Apple screenshot specifications](https://developer.apple.com/help/app-store-connect/reference/app-information/screenshot-specifications). |
| 20 | Description, keywords, subtitle | Missing | None. Drafts in section 3. | Owner pastes the drafts into App Store Connect. | Draft basis: [README](../README.md). |
| 21 | Age rating | Missing | None. | Owner answers the App Store Connect questionnaire. | Age rating page: unverified (not fetched). |
| 22 | Review notes | Missing | None. Draft in section 3. | Paste into App Review notes. | None. |
| 23 | Menu-bar-only UI (`LSUIElement`) | Missing | Set only at run time: `app.set_activation_policy(ActivationPolicy::Accessory)` in `setup()`. No `LSUIElement` in a plist. | Add `LSUIElement` = `true` through `infoPlist`. Keep the run-time call. | [Tauri config](https://v2.tauri.app/reference/config/) (`infoPlist`). Key definition: unverified (title only). Behavior described in `docs/menu-bar-app.md`. |
| 24 | Universal binary | Missing (decision) | Apple silicon only: `app/binaries/wattop-aarch64-apple-darwin`. Tauri's default build is universal, which also needs `wattop-x86_64-apple-darwin`. Store requirement for universal: unverified (search found none). Apple says macOS Tahoe is the last macOS release for Intel Macs. | Owner decides. Apple silicon only is simpler if App Review accepts it. | [Tauri MAS steps](https://v2.tauri.app/distribute/app-store/) (default is universal). [Tom's Hardware](https://www.tomshardware.com/software/macos/macos-tahoe-is-the-last-release-for-intel-macs-apple-silicon-exclusivity-will-mark-end-of-hackintoshes). |
| 25 | Bundle id matches App Store Connect | Blocked | `identifier` is `io.github.tupe12334.wattop`. No App Store Connect record yet. | Owner creates the App ID and the app record with the same id. | [Tauri MAS steps](https://v2.tauri.app/distribute/app-store/) (steps 2 and 4). |
| 26 | Certificates and provisioning profile | Blocked | None in the repo. | Owner creates the certificates and the profile (section 6). | [Tauri MAS steps](https://v2.tauri.app/distribute/app-store/), [Tauri signing](https://v2.tauri.app/distribute/sign/macos/). |
| 27 | Developer Program membership | Blocked | Not in the repo. | Owner enrolls. | [Tauri MAS steps](https://v2.tauri.app/distribute/app-store/) (step 1). |

## 3. Proposed App Store listing text

Plain and accurate to the store build. If the store build keeps the terminal dashboard, add the line marked **[if kept]**.

- **Name:** Wattop
- **Subtitle (28 of 30 characters):** Power stats in your menu bar
- **Promotional text:** See live charger input, battery and load watts in your menu bar. Choose the stats you want to see.
- **Description:**

  Wattop shows how your MacBook uses power. The stats appear in the menu bar.

  Choose what the menu bar shows:
  - Input watts from the charger
  - Load watts used by the system
  - Battery net watts, charging or draining
  - Battery percentage
  - Charger port
  - Temperature
  - Time left
  - Low Power Mode

  Click the menu to see live readouts: input and load, battery state, power source, temperature, time left, and Low Power Mode.

  Turn on "Launch at login" to start Wattop when you sign in.

  **[if kept]** Open the terminal dashboard in iTerm2 or Terminal.

  Wattop needs an Apple silicon MacBook with a battery. On a desktop Mac, it shows an error.

  Wattop reads power data from your Mac once per second. It needs no administrator password.

- **Keywords (79 of 100 characters):** `battery,watts,power,charger,energy,laptop,temperature,macbook,monitor,low power`
- **Review notes:**

  Wattop is a menu-bar app. It has no window and no Dock icon (`LSUIElement`). Click the lightning icon (⚡) in the menu bar to see the stats.

  To test, use an Apple silicon MacBook with a battery. Plug in and unplug the charger to see the input and battery values change.

  Power data comes from the battery entry in the IORegistry and from the power sensors, read once per second. The app saves one settings file in its container.

  "Launch at login" is off by default. Turn it on from the menu to start the app at login.

  **[if kept]** "Install command-line tool" and "Open terminal dashboard" use Apple Events to open Terminal or iTerm2.

## 4. Build and submit (Tauri 2)

Sources: [Tauri Mac App Store guide](https://v2.tauri.app/distribute/app-store/), [Tauri config reference](https://v2.tauri.app/reference/config/). Run the commands from the repo root unless a line says `cd app`. The workspace target folder is `target/` at the repo root.

### 4.1 Proposed files (not created)

**`app/tauri.appstore.conf.json`.** Merge it with `--config`. Tauri merges objects. How it merges arrays is unverified, so keep `externalBin` in the base file.

```json
{
  "bundle": {
    "category": "Utility",
    "macOS": {
      "minimumSystemVersion": "11.0",
      "bundleVersion": "2",
      "signingIdentity": "<Apple Distribution: NAME (TEAM_ID)>",
      "entitlements": "./Wattop.entitlements",
      "infoPlist": "./Info.plist",
      "hardenedRuntime": true,
      "files": {
        "embedded.provisionprofile": "<path to the Mac App Store Connect profile>"
      }
    }
  }
}
```

**`app/Wattop.entitlements`.** Sandbox and team id, from the Tauri MAS guide. Add `apple-events` only if row 12 keeps the `osascript` calls.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.app-sandbox</key>
  <true/>
  <key>com.apple.application-identifier</key>
  <string>$TEAM_ID.io.github.tupe12334.wattop</string>
  <key>com.apple.developer.team-identifier</key>
  <string>$TEAM_ID</string>
  <!-- Only if row 12 keeps osascript: -->
  <!-- <key>com.apple.security.automation.apple-events</key><true/> -->
</dict>
</plist>
```

**Sidecar entitlements (name proposed: `app/wattop-cli.entitlements`).** Sandbox plus inherit. Whether Tauri applies this file to `externalBin` is unverified. Check the built bundle.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.app-sandbox</key>
  <true/>
  <key>com.apple.security.inherit</key>
  <true/>
</dict>
</plist>
```

**`app/Info.plist`.** Merged into the default plist through `infoPlist`.

```xml
<key>LSUIElement</key>
<true/>
<key>LSApplicationCategoryType</key>
<string>public.app-category.utilities</string>
<key>ITSAppUsesNonExemptEncryption</key>
<false/>
<key>NSHumanReadableCopyright</key>
<string>Copyright 2026 <holder name></string>
```

Add `NSAppleEventsUsageDescription` only if row 12 keeps the `osascript` calls. The copyright holder is a placeholder. The owner sets the name before submit.

**`PrivacyInfo.xcprivacy`.** Structure from Apple's privacy manifest format. The reason code and the placement inside the bundle are unverified. Check both against Apple's pages and a real build.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>NSPrivacyTracking</key>
  <false/>
  <key>NSPrivacyTrackingDomains</key>
  <array/>
  <key>NSPrivacyCollectedDataTypes</key>
  <array/>
  <key>NSPrivacyAccessedAPITypes</key>
  <array>
    <dict>
      <key>NSPrivacyAccessedAPIType</key>
      <string>NSPrivacyAccessedAPICategorySystemBootTime</string>
      <key>NSPrivacyAccessedAPITypeReasons</key>
      <array>
        <string>35F9.1</string>
      </array>
    </dict>
  </array>
</dict>
</plist>
```

### 4.2 Steps

1. Build the sidecar for Apple silicon. Remove `install-app` from the store build first (row 10). That is a code change, not a flag in the repo today.

   ```sh
   cargo build --release -p wattop --target aarch64-apple-darwin
   cp target/aarch64-apple-darwin/release/wattop app/binaries/wattop-aarch64-apple-darwin
   ```

2. Build the app with the store config merged in. Run from `app/`.

   ```sh
   cd app
   cargo tauri build --bundles app --target aarch64-apple-darwin --config tauri.appstore.conf.json
   cd ..
   ```

   Expected output: `target/aarch64-apple-darwin/release/bundle/macos/Wattop.app`.

3. Check the signature and entitlements.

   ```sh
   codesign -dv --verbose=4 target/aarch64-apple-darwin/release/bundle/macos/Wattop.app
   codesign -d --entitlements :- target/aarch64-apple-darwin/release/bundle/macos/Wattop.app
   ```

4. Find the exact installer identity name. Use that name in step 5.

   ```sh
   security find-identity -v
   ```

5. Sign the installer package with the Mac Installer Distribution identity.

   ```sh
   xcrun productbuild --sign "<exact name from step 4>" --component target/aarch64-apple-darwin/release/bundle/macos/Wattop.app /Applications Wattop.pkg
   ```

6. Validate and upload with an App Store Connect API key. Use `altool` as the Tauri guide shows. Transporter is the other option ([Apple upload page](https://developer.apple.com/help/app-store-connect/manage-builds/upload-builds)).

   ```sh
   xcrun altool --validate-app -f Wattop.pkg --type macos --apiKey "$APPLE_API_KEY_ID" --apiIssuer "$APPLE_API_ISSUER"
   xcrun altool --upload-app -f Wattop.pkg --type macos --apiKey "$APPLE_API_KEY_ID" --apiIssuer "$APPLE_API_ISSUER"
   ```

   The API key must be saved as `AuthKey_<KEY_ID>.p8` in one of the folders listed in the Tauri guide.

7. In App Store Connect, add the build to the version, fill the metadata, and submit for review.

**Universal binary.** Only if the owner chooses it (row 24). Add `app/binaries/wattop-x86_64-apple-darwin`, build with `--target universal-apple-darwin`, and check the sidecar name. The x86_64 Rust target is installed on this Mac. Passing `--target aarch64-apple-darwin` builds arm64 only. Without it, Tauri's default is universal.

## 5. Sandbox spike (see "Sandbox spike results" below)

Run the signed, sandboxed build. Record each result in this table. Tests are against the current code.

| # | Test | Expected if the store build works | Result |
| --- | --- | --- | --- |
| 1 | App starts and the menu-bar title shows a value | Yes | see "Sandbox spike results" below |
| 2 | `smc::Smc::open()` (AppleSMC user client) | Opens. If not, row 9 is a failure. | see "Sandbox spike results" below |
| 3 | `ioreg -rn AppleSmartBattery -a` as a child process | Output returned | see "Sandbox spike results" below |
| 4 | `pmset -g` as a child process (Low Power Mode) | Output returned | see "Sandbox spike results" below |
| 5 | `sysctl hw.model` (CLI) | Output returned | see "Sandbox spike results" below |
| 6 | sysinfo process list (top power users, CLI) | Process list returned | see "Sandbox spike results" below |
| 7 | `osascript` Apple Event to Terminal or iTerm2 | Runs after the consent prompt | see "Sandbox spike results" below |
| 8 | Launch at login: SMAppService against LaunchAgent write | SMAppService works. LaunchAgent write is denied. | see "Sandbox spike results" below |
| 9 | "Install command-line tool" writes `~/.local/bin/wattop` | Denied. Remove the item from the store build. | see "Sandbox spike results" below |
| 10 | Settings file in the container | Written | see "Sandbox spike results" below |

## 6. Next actions for the owner

- Enroll in the Apple Developer Program. Only the Account Holder can create some certificates ([Developer ID page](https://developer.apple.com/developer-id/) states this for Developer ID).
- Create the App ID `io.github.tupe12334.wattop` and the app record in App Store Connect. Use the same bundle id.
- Create the certificates: Apple Distribution and Mac Installer Distribution. Add Developer ID Application only if the Developer ID path is chosen.
- Create a Mac App Store Connect provisioning profile for that App ID. Keep the profile file out of git.
- Create an App Store Connect API key with Developer access. Save the `.p8` file where the Tauri guide says.
- Host a privacy policy page. Choose a support URL.
- Answer the age rating questionnaire and the export compliance question. Confirm the copyright holder name.
- Decide: keep or drop the CLI sidecar and `install-app` in the store build. Keep or drop Apple Events items.
- Decide: Apple silicon only, or universal.
- Decide the store path after the spike (section 1).
- Make the screenshots and approve the listing text.

## 7. Sources

Apple, read:

- [App Review Guidelines](https://developer.apple.com/app-store/review/guidelines/): 2.4.5, 2.5.1, 2.5.2, 5.1.1, 5.1.2 (quoted). Sections 2.4.5(iii) and 2.5.4 appear in the same page.
- [Developer ID](https://developer.apple.com/developer-id/): Developer ID certificates are for software distributed outside the Mac App Store.
- [Privacy manifest approved reasons (news)](https://developer.apple.com/news/upcoming-requirements/?id=03012024a): approved reasons are needed to upload.
- [Screenshot specifications](https://developer.apple.com/help/app-store-connect/reference/app-information/screenshot-specifications): Mac sizes, formats, up to 10 per device size.
- [Upload builds](https://developer.apple.com/help/app-store-connect/manage-builds/upload-builds): Xcode, Transporter, `altool`, App Store Connect API.

Apple, not readable (title only, timed out, or 404). Every claim that depends on these is marked unverified:

- [Privacy manifest files](https://developer.apple.com/documentation/bundleresources/privacy-manifest-files)
- [Describing use of required reason API](https://developer.apple.com/documentation/bundleresources/describing-use-of-required-reason-api)
- [App Sandbox](https://developer.apple.com/documentation/security/app-sandbox)
- [Configuring the macOS App Sandbox](https://developer.apple.com/documentation/xcode/configuring-the-macos-app-sandbox)
- [com.apple.security.inherit](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.inherit) (HTTP 404)
- [com.apple.security.automation.apple-events](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.automation.apple-events)
- [SMAppService](https://developer.apple.com/documentation/servicemanagement/smappservice) (timed out)
- [LSUIElement](https://developer.apple.com/documentation/bundleresources/information-property-list/lsuielement)
- [ITSAppUsesNonExemptEncryption](https://developer.apple.com/documentation/bundleresources/information-property-list/itsappusesnonexemptencryption)
- [CFBundleVersion](https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundleversion)
- [LSApplicationCategoryType](https://developer.apple.com/documentation/bundleresources/information-property-list/lsapplicationcategorytype)
- [Notarizing macOS software](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)

Tauri 2, read:

- [Mac App Store guide](https://v2.tauri.app/distribute/app-store/)
- [macOS code signing](https://v2.tauri.app/distribute/sign/macos/) (does not cover `entitlements` or `hardenedRuntime`)
- [Configuration reference](https://v2.tauri.app/reference/config/)
- [Autostart plugin](https://v2.tauri.app/plugin/autostart/)

Other:

- [Apple forum 751198](https://developer.apple.com/forums/thread/751198): installer identity names.
- [Apple forum 749799](https://developer.apple.com/forums/thread/749799): forum summary of reason `35F9.1`. Not an Apple list.
- [Tom's Hardware, macOS Tahoe last Intel release](https://www.tomshardware.com/software/macos/macos-tahoe-is-the-last-release-for-intel-macs-apple-silicon-exclusivity-will-mark-end-of-hackintoshes).

Local source files:

- Rust std: `/Users/ofek/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/sys/time/unix.rs`
- sysinfo 0.40.1: `/Users/ofek/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sysinfo-0.40.1/src/unix/apple/system.rs`
- tauri-plugin-autostart 2.7.0: `/Users/ofek/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-plugin-autostart-2.7.0/src/lib.rs`

## Sandbox spike results

A test app ran with only `com.apple.security.app-sandbox` (ad-hoc signed, macOS 26, Mac16,1, 2026-11-07).

| Data source | Works in the sandbox? | Needed entitlement | App Review risk |
|---|---|---|---|
| SMC `PDTR`/`PSTR` (`IOServiceOpen` on AppleSMC) | No. Error `0xe00002e2` | `com.apple.security.temporary-exception.iokit-user-client-class` = `["AppleSMCClient"]` | High |
| `ioreg -rn AppleSmartBattery -a` (spawn) | Yes | None | Low |
| `IORegistryEntryCreateCFProperties` (in-process) | Yes | None | Low |
| `pmset -g` (spawn) | Yes | None | Low |
| `sysctlbyname("hw.model")` | Yes | None | Low |

Verdict: an App Store build is possible with these limits.

- Drop the SMC live watts in the App Store build. Use the IORegistry `PowerTelemetryData` values. They update about every 20 s.
- Select the data source with a Cargo feature. The direct-download build keeps the SMC.
- Remove "Install command-line tool". It writes outside the container.
- Replace the LaunchAgent login item with `SMAppService`.
- Remove "Open terminal dashboard", or show the dashboard in the app.

Not tested yet: the plugged-in readings, sidecar signing with `com.apple.security.inherit`, and `SMAppService`.
