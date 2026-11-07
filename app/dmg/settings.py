# dmgbuild settings for the Wattop release .dmg.
#
# Run from the repo root:
#   python3 app/dmg/build_dmg.py -s app/dmg/settings.py -D app=target/release/bundle/macos/Wattop.app "Wattop" out.dmg
#
# build_dmg.py wraps dmgbuild and drops its pBBk background bookmark, which stops Finder
# from showing the background on macOS 26. dmgbuild writes the Finder layout (.DS_Store)
# itself, so no Finder or GUI session is needed.

import os

app = defines["app"]  # noqa: F821 - defined by dmgbuild from -D app=...
app_name = os.path.basename(app)

format = "UDZO"
files = [app]
symlinks = {"Applications": "/Applications"}
icon = "app/icons/icon.icns"
background = "app/dmg/background.tiff"

window_rect = ((100, 100), (660, 400))
default_view = "icon-view"
show_status_bar = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
show_tab_view = False

icon_size = 128
text_size = 13
icon_locations = {
    app_name: (170, 200),
    "Applications": (490, 200),
}
