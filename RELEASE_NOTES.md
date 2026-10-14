## Time's up 0.0.1 (first pre-release)

A small always-on-top class-hour timer for Windows. A bar on screen fills green, then orange, then
red as the class hour runs out. A 100 minute class shows two blocks with a break in the middle.

### Download

`times-up-0.0.1-win-x64.zip`: unzip anywhere (a USB stick works), keep `times-up.exe` and
`times-up.toml` together, and start `times-up.exe`. There is nothing to install.

**First launch:** you will not see the bar until a class is running (or about to start). The sample
`times-up.toml` only has morning classes. Click the small icon in the notification area (maybe behind
the `^` arrow), choose *Open config (Notepad)*, and enter your own timetable.

### What it does

- Per-weekday timetable in a text file next to the exe; edits are picked up while it runs.
- Fills from the left: green, orange for the last 10 minutes of a block, red for the last 5.
  Countdown text centred in the current block.
- Drag the bar to move it and its edges to resize it; place and size are remembered, and it moves
  back onto a connected monitor if its monitor is gone.
- Drag the break or the orange and red lines for the running class (a small time shows while you
  drag); *Save as default for this slot* keeps the change.
- Tray icon and right-click menu: Save as default, Open config, Start with Windows, Quit.
- Tiny: a 0.5 MB exe with no runtime or installer, about 12 MB of memory (2 MB private) and about
  0.5 % of one CPU core.

### Please note

- **Only tried on Windows 11.** It is meant for Windows 10 version 1607 or later too, but that has
  not been verified. Please open an issue if it does not work for you.
- The exe is **not code-signed**, so Windows SmartScreen may warn when you run it for the first time
  (*More info* > *Run anyway*).
- No exe icon or version resource yet.

### Checksum

SHA256 of `times-up-0.0.1-win-x64.zip`:

```
58c59d037593d4b55a11a07d14c71f45b0923ce742006f5504bd61c0dec33e92
```

Licensed under the [MIT license](LICENSE).
