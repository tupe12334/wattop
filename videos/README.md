# Videos

This folder holds the setup for the README video and dashboard screenshot. They are real recordings of the `wattop` binary.

## Record again

1. Install [VHS](https://github.com/charmbracelet/vhs).
2. Run `cargo build --release`.
3. Run `vhs videos/dashboard.tape` from the repo root.

VHS writes `videos/out/dashboard.mp4` and `videos/out/dashboard.gif`. It also writes `docs/media/dashboard.png`.

## Publish

Upload the MP4 as a GitHub user attachment. Drag it into an issue or PR comment, then copy the URL. Put the bare URL on its own line in the README. GitHub shows it as a video player.

The `out/` folder is ignored by git.
