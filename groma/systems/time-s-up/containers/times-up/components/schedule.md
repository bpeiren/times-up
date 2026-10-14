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

Holds each weekday's classes, the default 10-minute break of classes of 100 minutes or more, and the orange and red warning thresholds of each block. Given a weekday and a time it returns the state to show: idle, lead-in before a class, running block with zone and time left, break, or just ended.
