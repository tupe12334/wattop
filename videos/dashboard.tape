# Records the real wattop dashboard. Run from the repo root:
#   cargo build --release && vhs videos/dashboard.tape
Output videos/out/dashboard.mp4
Output videos/out/dashboard.gif

Set Shell "bash"
Set FontSize 15
Set Width 1400
Set Height 900
Set Padding 16
Set Theme "Dracula"

Hide
Type `export PS1='$ ' PATH="$PWD/target/release:$PATH" && clear`
Enter
Show

Type "wattop"
Sleep 600ms
Enter
Sleep 14s
Screenshot docs/media/dashboard.png
Sleep 2s
Type "q"
Sleep 500ms
