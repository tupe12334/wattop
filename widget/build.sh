#!/bin/sh
# Builds the Power Flow widget, a WidgetKit extension the menu-bar app embeds.
# Output: widget/build/WattopWidget.appex. Needs the Xcode command line tools.
#
#   WIDGET_BUNDLE_ID      bundle id of the extension. It must start with the host app's id.
#   WIDGET_SIGN_IDENTITY  codesign identity. Falls back to APPLE_SIGNING_IDENTITY, then to "-" (ad-hoc).
set -eu
cd "$(dirname "$0")"

BUNDLE_ID=${WIDGET_BUNDLE_ID:-io.github.tupe12334.wattop.widget}
IDENTITY=${WIDGET_SIGN_IDENTITY:-${APPLE_SIGNING_IDENTITY:--}}
VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' ../Cargo.toml | head -n 1)
APPEX=build/WattopWidget.appex

rm -rf "$APPEX"
mkdir -p "$APPEX/Contents/MacOS"

# An app extension starts in NSExtensionMain, not in main. Without that entry point the
# process exits at launch and macOS lists no widget.
xcrun swiftc -O -parse-as-library -application-extension \
    -target arm64-apple-macos14.0 -module-name WattopWidget \
    -Xlinker -e -Xlinker _NSExtensionMain \
    Sources/*.swift -o "$APPEX/Contents/MacOS/WattopWidget"

sed -e "s/@BUNDLE_ID@/$BUNDLE_ID/" -e "s/@VERSION@/$VERSION/" Info.plist > "$APPEX/Contents/Info.plist"

# macOS only loads a widget that has the App Sandbox entitlement.
if [ "$IDENTITY" = "-" ]; then
    codesign --force --sign - --entitlements WattopWidget.entitlements "$APPEX"
else
    codesign --force --sign "$IDENTITY" --options runtime --timestamp \
        --entitlements WattopWidget.entitlements "$APPEX"
fi

echo "Built widget/$APPEX ($BUNDLE_ID $VERSION)"
