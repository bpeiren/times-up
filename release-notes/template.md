## Time's up {{VERSION}}

A small always-on-top class-hour timer for Windows. A bar on screen fills with the lesson colour, then
the warning colour, then the final colour as the class hour runs out.

### Download

`{{ZIP}}`: unzip anywhere (a USB stick works) and start `times-up.exe`. There is nothing to install.

**First launch:** you will not see the bar until a class is running (or about to start). The first
start writes a commented `times-up.toml` next to the exe. Click the small icon in the notification
area (maybe behind the `^` arrow) or right-click the bar and choose *Settings...* to lay out your own
timetable.

### Requirements

- Windows 10 version 1607 or later, 64-bit. Only tried on Windows 11 so far; please open an issue if
  it does not work for you.
- No runtime and no installer. The exe only uses Windows DLLs.

### Please note

- The exe is **not code-signed**, so Windows SmartScreen may warn when you run it for the first time
  (*More info* > *Run anyway*).

### Upgrading

Unzip over the old folder, or just replace `times-up.exe`. The zip contains no `times-up.toml`, so your
timetable and settings stay as they are. Quit Time's up first, or Windows will not let the exe be
replaced.

### What's changed

{{CHANGES}}
