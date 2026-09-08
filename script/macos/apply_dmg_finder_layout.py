#!/usr/bin/env python3
"""Write Finder window layout onto a mounted DMG.

Finder AppleScript on macOS Tahoe (26) APFS volumes does not persist
backgroundType or iconSize, so create-dmg leaves a default grey window even
when the PNG is on the disk. This writes the same .DS_Store records dmgbuild
uses: a volume-relative alias (backgroundType 2) and no pBBk bookmark, which
Tahoe treats as stale and then drops the picture.
"""

from __future__ import annotations

import argparse
from pathlib import Path

from ds_store import DSStore
from mac_alias import Alias


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mount-dir", required=True, type=Path)
    parser.add_argument("--app-name", required=True)
    parser.add_argument("--background", required=True)
    parser.add_argument("--window-origin-x", type=int, default=200)
    parser.add_argument("--window-origin-y", type=int, default=120)
    parser.add_argument("--window-width", type=int, default=640)
    parser.add_argument("--window-height", type=int, default=428)
    parser.add_argument("--icon-size", type=int, default=128)
    parser.add_argument("--app-x", type=int, default=160)
    parser.add_argument("--app-y", type=int, default=200)
    parser.add_argument("--applications-x", type=int, default=480)
    parser.add_argument("--applications-y", type=int, default=200)
    args = parser.parse_args()

    mount_dir = args.mount_dir.resolve()
    background_path = (mount_dir / args.background).resolve()
    if not background_path.is_file():
        raise SystemExit(f"DMG background not found: {background_path}")

    alias = Alias.for_file(str(background_path))
    window_bounds = (
        f"{{{{{args.window_origin_x}, {args.window_origin_y}}}, "
        f"{{{args.window_width}, {args.window_height}}}}}"
    )

    bwsp = {
        "ShowStatusBar": False,
        "WindowBounds": window_bounds,
        "ContainerShowSidebar": False,
        "PreviewPaneVisibility": False,
        "SidebarWidth": 0,
        "ShowTabView": False,
        "ShowToolbar": False,
        "ShowPathbar": False,
        "ShowSidebar": False,
    }
    # Finder does not stretch the PNG. Extra space in a larger window (typical
    # when the DMG opens as a tab) is filled with this color — match the art's
    # charcoal so it does not letterbox in white.
    charcoal = 25 / 255.0
    icvp = {
        "viewOptionsVersion": 1,
        "backgroundType": 2,
        "backgroundColorRed": charcoal,
        "backgroundColorGreen": charcoal,
        "backgroundColorBlue": 28 / 255.0,
        "backgroundImageAlias": alias.to_bytes(),
        "gridOffsetX": 0.0,
        "gridOffsetY": 0.0,
        "gridSpacing": 100.0,
        "arrangeBy": "none",
        "showIconPreview": True,
        "showItemInfo": False,
        "labelOnBottom": True,
        "textSize": 12.0,
        "iconSize": float(args.icon_size),
        "scrollPositionX": 0.0,
        "scrollPositionY": 0.0,
    }

    dsstore_path = mount_dir / ".DS_Store"
    with DSStore.open(str(dsstore_path), "w+") as d:
        d["."]["vSrn"] = ("long", 1)
        d["."]["bwsp"] = bwsp
        d["."]["icvp"] = icvp
        d["."]["icvl"] = (b"type", b"icnv")
        d[args.app_name]["Iloc"] = (args.app_x, args.app_y)
        d["Applications"]["Iloc"] = (args.applications_x, args.applications_y)


if __name__ == "__main__":
    main()
