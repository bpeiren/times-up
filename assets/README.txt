TIME'S UP - a class-hour timer for your screen
==============================================

A small bar that stays on top of your slides and shows how much of the current class is left.
It fills from the left: the lesson colour, then the warning colour for the last 10 minutes, then the final colour for the last 5.
A 100 minute class shows two blocks with a break in the middle.

Getting started
---------------
1. Put times-up.exe in any folder (a USB stick works too) and start it. There is nothing to
   install. The first time, it writes times-up.toml next to it: a commented sample timetable.
2. You will not see the bar yet: it only shows during class times (and 5 minutes before), and the
   sample timetable has morning classes only. A small icon appears in the notification area
   (maybe behind the ^ arrow) to show that the app is running. Open times-up.toml, next to the
   exe, in any text editor (Notepad will do) and set the classes for each weekday. Save the file:
   the bar picks up the change by itself.

The bar
-------
- It appears 5 minutes before a class and disappears shortly after the last class ends.
- The text shows the time left in the current block, centred in that block, or "Break mm:ss" on the
  break.
- Drag it to move it. Drag its edges or corners to resize it. Its place and size are remembered.
- Drag the break, or the thin warning and final lines, to change them for the class that is running.
  While you drag, a small time at the bottom of the bar shows when it starts (and ends). Such a
  change is forgotten when the class is over.
- Right-click the bar, or click the icon in the notification area:
    Save as default for this slot   keeps those changes for this class from now on
    Settings...                     opens the Settings window: the week, with classes you can
                                    drag, resize, add (+) and remove (Delete),
                                    and the break and warning times of the selected class;
                                    its General tab sets the colours, the lead-in and the
                                    default break and warning minutes
    Start with Windows              starts the bar when you sign in (switch on again if you move
                                    the folder)
    Quit

Upgrading
---------
Replace times-up.exe with the new one. Your times-up.toml stays as it is and keeps working.

Settings
--------
Everything is in times-up.toml next to the exe; nothing is stored anywhere else (apart from the
optional "Start with Windows" entry). Lines starting with # are comments. Wrong values are ignored
and replaced by their defaults. Colours look like "#2ecc71" (or "#2ecc7180" with transparency).

Time's up is free software under the MIT license (see LICENSE).

Needs Windows 11, or Windows 10 version 1607 or later, 64-bit (only tried on Windows 11). If the
folder is read-only the bar still works but cannot remember its place.
