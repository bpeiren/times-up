---
type: C4 Component
title: Schedule and class state
status: stable
groma:
  id: schedule
  parent: times-up
  code:
    - scanner: rust
      file: src/schedule.rs
  group: Timetable
description: Decides what to show at a given moment
---

Holds each weekday's classes. A class has a list of breaks (any number that fit, none touching) and so one more block than breaks, each block with its own orange and red warning thresholds; classes of 100 minutes or more start with one centred 10-minute break. Given a weekday and a time it returns the state to show: idle, lead-in before a class, running block with zone and time left, break, or just ended.
