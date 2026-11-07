#!/usr/bin/env python3
"""Build the Wattop .dmg with dmgbuild, without the pBBk background bookmark.

dmgbuild writes a pBBk bookmark for the background image next to the icvp
background alias. On macOS 26 Finder then shows the default grey window, not
the background. Without pBBk, Finder draws the background from the icvp alias.
Pinned dmgbuild version: 1.6.5 (this wrapper patches dmgbuild.core).

Run from the repo root:
    python3 app/dmg/build_dmg.py -s app/dmg/settings.py -D app=<Wattop.app> "Wattop" out.dmg
"""

import sys

import dmgbuild.core as core
from dmgbuild.__main__ import main


class _NoBookmark:
    """Stands in for mac_alias.Bookmark. Returns None, so dmgbuild writes no pBBk record."""

    @staticmethod
    def for_file(path):
        return None


core.Bookmark = _NoBookmark

if __name__ == "__main__":
    sys.exit(main())
