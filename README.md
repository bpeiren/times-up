# Time's up

A small always-on-top class-hour timer for Windows. A bar on screen fills green, then orange, then
red as the class hour runs out, so a teacher can keep track of time at a glance. A 100 minute class
shows two blocks with a break in the middle.

Written in Rust against the Win32 API (`windows` crate): one tiny exe, no installer, no runtime, and
all settings in a text file next to it, so it is portable (a USB stick works).

## For teachers

Download the zip, put `times-up.exe` in any folder and start it; it writes `times-up.toml`, a
commented sample timetable, next to itself. To upgrade, replace `times-up.exe`; your own
`times-up.toml` stays untouched. The `README.txt` in the zip explains the rest; in short:

- The bar appears 5 minutes before a class and hides shortly after the last class ends. **Outside
  class time you will not see it**: the sample timetable only has morning classes (08:30 to
  12:05), so edit it to your own timetable first.
- A small icon in the notification area (maybe behind the `^` arrow) shows that the app is running,
  also when the bar is hidden. Click it for the same menu as a right-click on the bar.
- The bar fills from the left in green; the last 10 minutes of a block are orange and the last 5 are
  red. Thin lines on the unfilled part show where the colours change.
- The text shows the time left in the current block, centred in that block; `Break mm:ss` during a
  break, centred on the break; or `Starts in mm:ss` before class.
- Drag the bar to move it and its edges or corners to resize it; its place and size are remembered.
- Drag the break, or an orange or red line, to change it for the class that is running. While you
  drag, a small time at the bottom of the bar shows when that zone starts (`09:35`) or when the
  break starts and ends (`09:45-09:55`). The change is forgotten when the class is over, unless you
  right-click and choose *Save as default for this slot*.
- Right-click the bar or click the tray icon: *Save as default for this slot*, *Settings...*,
  *Start with Windows*, *Quit*. *Settings...* opens a window of its own (one at a time, resizable;
  its place and size are remembered). It shows the week: click a class to select it, drag it to
  move it or drag its edges to resize it (5 minute steps, never over a neighbour), use the *+*
  button of a day to add a class, and *Remove class* or the Delete key to remove the selected one.
  The selected class is drawn large below the week: drag the break or the orange and red lines,
  hover for times and durations, *Add break* (click where it should start, Esc cancels) and *Remove
  break*. Changes are saved to `times-up.toml` when you let go of the mouse. If you change the
  break or the warning times of the class that is running while you have changed it on the bar
  too, a banner asks: *Replace* (Enter) puts the saved settings on the bar, *Save only* (Esc)
  keeps your temporary changes on the bar until the class ends. It asks once per running class;
  closing the window counts as *Save only*.

Requirements: Windows 11, or Windows 10 version 1607 or later (sharp scaling on monitors with
different DPI needs 1703), 64-bit. It has only been tried on Windows 11. Nothing else needs to be
installed: the exe imports Windows system DLLs only (a test checks this).

### Settings: `times-up.toml`

The file next to the exe holds everything. It is created with a commented sample when it is missing
([`assets/times-up.sample.toml`](assets/times-up.sample.toml)), and edits are picked up while the
app runs. A wrong value is reported, replaced by its default, and never stops the timer.

```toml
[general]
lead_in_minutes = 5          # the bar appears this long before a class starts
linger_seconds = 10          # ...and stays this long after the last class ends
default_break_minutes = 10   # break in the middle of classes of 100 minutes or more
orange_minutes = 10          # the bar turns orange this long before the end of a block
red_minutes = 5              # ...and red this long before the end

[colors]                     # #RRGGBB or #RRGGBBAA
green = "#2ecc71"
break = "#bdc3c7"            # also orange, red, track, text

[monday]                     # one section per weekday: monday ... friday
classes = [
  { start = "08:30", end = "09:20" },                 # 50 minutes: one block
  { start = "10:25", end = "12:05" },                 # 100 minutes: two blocks and a break
  { start = "13:00", end = "14:40", break_start = 40, break_len = 20, orange = [10, 8], red = [5, 4] },
]
```

Per class, `break_start` and `break_len` (minutes; `break_len = 0` means no break) and `orange` and
`red` (one value, or one per block) are optional. The app writes the `[window]` section and, for
*Save as default for this slot*, those per-class keys itself, keeping your comments.

### Measured

On Windows 11, 2560x1080 at 100 % scaling, release build, a class in progress (`.\scripts\package.ps1
-MeasureSeconds 30`):

| | |
|---|---|
| `times-up.exe` | 549,376 bytes (about 0.5 MB) |
| zip (exe + sample config + readme) | 290,944 bytes |
| Memory, working set | 12.4 MB, of which only 2.1 MB is private; the rest is shared Windows DLL pages |
| CPU, idle | 0.52 % of one core (one redraw per second) |
| Disk and registry | nothing outside its own folder, except the optional *Start with Windows* entry |

## For developers

### Setup (Windows 10/11 x64)

```powershell
winget install --id Rustlang.Rustup -e
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
rustup default stable-x86_64-pc-windows-msvc
rustup component add clippy rustfmt rust-analyzer
```

Open a new terminal afterwards so `~/.cargo/bin` is on `PATH`.

### Build, test, package

```powershell
cargo run                    # debug build, uses times-up.toml next to target\debug\times-up.exe
cargo test                   # unit tests, window tests (they create real windows) and the exe dependency test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
.\scripts\package.ps1        # release build, dependency check, dist\times-up-<version>-win-x64.zip, smoke test
.\scripts\package.ps1 -MeasureSeconds 30   # ...and measure idle memory and CPU
```

The release profile is tuned for size (`opt-level = "z"`, LTO, `panic = "abort"`, stripped).
`.cargo/config.toml` links the C runtime statically; without it the exe would import
`VCRUNTIME140.dll` and need the Visual C++ Redistributable on a clean PC.
`tests/exe_dependencies.rs` fails if that ever happens again.

### How it is built

No graphics library: a small software canvas draws the bar and a layered window shows it with
per-pixel transparency; text comes from GDI. Windows code is kept apart from plain, unit-tested
logic.

| Module | Responsibility |
|---|---|
| `schedule` | Weekdays, classes, breaks, warning zones; what to show at a given moment |
| `config` | `times-up.toml`: reading with per-value fallback, in-place saving that keeps comments |
| `bar` | Turns the schedule state into the bar and its label, and paints it |
| `canvas` | The pixel buffer: anti-aliased rounded rectangles, text masks |
| `interaction` | Hit testing, resizing, keeping the window on a connected monitor |
| `editing` | Dragging the break and the warning zones, with clamping |
| `win::window` | The borderless, transparent, always-on-top window and its mouse handling |
| `win::app` | Start-up, the per-second work, saving, menu commands |
| `store` | The one owner of the loaded settings: the file poll and every edit go through it, views redraw when its revision changes, and the bar's temporary changes are kept apart from the saved timetable |
| `settings`, `strings` | The Settings window's layout and frame as plain geometry; every UI text in one place |
| `banner` | The Replace / Save only question for the running class: geometry and painting |
| `timetable`, `detail` | The week overview and the detail bar of the selected class: editing rules, geometry and painting, without Win32 |
| `win::settings_window` | The Settings window: a resizable window whose contents we draw |
| `win::menu`, `win::autostart` | The right-click menu and the *Start with Windows* registry entry |
| `icon`, `win::tray` | The notification-area icon, drawn at run time, and its click handling |
| `win::text`, `win::monitors`, `win::surface` | GDI text, monitor areas, showing the canvas |

### Commit messages

[Conventional Commits](https://www.conventionalcommits.org/) (`feat(bar): ...`, `fix(window): ...`),
so the changelog can be generated from the history. The details are in `AGENTS.md`.

### Changelog

[`CHANGELOG.md`](CHANGELOG.md) is generated with [git-cliff](https://git-cliff.org) (`cliff.toml`) and
committed only when a version is released: `winget install --id orhun.git-cliff -e`, then
`git cliff --unreleased` to preview what is coming. The release steps are in `AGENTS.md`.

### License

[MIT](LICENSE), copyright (c) 2026 Bart Peiren. The zip contains the license text.

### Project management

Work is tracked with Backlog.md (`backlog/`) and the architecture, curated as C4 documents, lives
in `groma/` ([Groma](https://github.com/MrLesk/Groma.md)).
See `AGENTS.md` for the workflow.
