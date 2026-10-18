# Changelog

All notable changes to Time's up. This file is generated from the conventional commit messages with
[git-cliff](https://git-cliff.org); do not edit it by hand (see `AGENTS.md`).

## [0.0.2] - 2026-10-18

### Features

- **settings:** Add a Settings window opened from the menus ([68c4475](https://github.com/bpeiren/times-up/commit/68c4475579e178733c7b6906252d89309e336be3))
- **settings:** Edit the week by dragging classes in the Settings window ([c6199f7](https://github.com/bpeiren/times-up/commit/c6199f778eb251cc8e38c808e46354e09a0101bb))
- **settings:** Edit the selected class on a detail bar ([32c6a6b](https://github.com/bpeiren/times-up/commit/32c6a6bf72ce8f2a8a85416d82b3dc730097220a))
- **settings:** Ask what to do with temporary changes of the running class ([a0ea324](https://github.com/bpeiren/times-up/commit/a0ea324afb438964ad4b63d7822819f4c9926139))
- **settings:** Set colours, lead-in and default minutes on a General tab (#8) ([e6d8098](https://github.com/bpeiren/times-up/commit/e6d8098c83e8f3f2edd13bebc912809a47e0d3e7))
- **schedule:** Allow several breaks in a class (#10) ([3cae250](https://github.com/bpeiren/times-up/commit/3cae250787806c43d1ad6288059a73d0c26e95dd))

### Refactoring

- **store:** Route the live bar's config and file through a Store ([ee52949](https://github.com/bpeiren/times-up/commit/ee529499375d47aa57d47014bbd0398ca1c6a4c5))

### Build and packaging

- **package:** Ship the zip without a config file ([49cafcd](https://github.com/bpeiren/times-up/commit/49cafcd6d59dda8cff2736dcc99eec04821628b5))
- **changelog:** Set up git-cliff and generate CHANGELOG.md ([d47e3f3](https://github.com/bpeiren/times-up/commit/d47e3f35d75e8cf145928afe92f89e9f495542b0))

### Documentation

- Require conventional commit messages for the changelog ([63999e7](https://github.com/bpeiren/times-up/commit/63999e7a5a5153ca7216493e10b1371d667807f3))
- Drop the Co-Authored-By trailer from the commit convention ([a2a2433](https://github.com/bpeiren/times-up/commit/a2a2433c94aaa60348374768daaa59a69e694196))
- Forbid Claude Code attribution in pull request descriptions ([df7c02b](https://github.com/bpeiren/times-up/commit/df7c02b4d4b477d21826c73a55f2aed4800d20f5))
- Plan Backlog tasks on main and keep task progress in the pull request ([8827c8c](https://github.com/bpeiren/times-up/commit/8827c8ce7b670dbba13c383f6e9962c1bdcf793f))
- **changelog:** Commit the changelog only at releases ([1f75569](https://github.com/bpeiren/times-up/commit/1f755695a39205cd9ef50502039be231bac59fa5))

## [0.0.1] - 2026-10-14

### Features

- **schedule:** Add the schedule and block model ([8ce1ccd](https://github.com/bpeiren/times-up/commit/8ce1ccd69744d9f0a42d389167f331bc4a670583))
- **config:** Read and write times-up.toml next to the exe ([4ef477a](https://github.com/bpeiren/times-up/commit/4ef477a91c32e485d84d4375ea235a4bf1739740))
- **window:** Add a transparent always-on-top window and canvas ([55fe59f](https://github.com/bpeiren/times-up/commit/55fe59fb5c9857a2915f43e75ff0b4d6cdc2a3a1))
- **bar:** Show zones, break, countdown and lead-in ([4f880ba](https://github.com/bpeiren/times-up/commit/4f880ba91072b0b7895a6fbba34b596d57b508b9))
- **window:** Move, resize and persist the bar ([9d35d22](https://github.com/bpeiren/times-up/commit/9d35d228dc40e773b59a80b333464f6ce14d360f))
- **menu:** Add the right-click menu and Start with Windows ([3c419e6](https://github.com/bpeiren/times-up/commit/3c419e688f0ccffff55fe23d57d0ebcc9a130abd))
- **editing:** Drag the break and warning zones on the bar ([201184b](https://github.com/bpeiren/times-up/commit/201184bb4ae6743541d3fa06a93a080ab84f81ec))
- **tray:** Add a tray icon with the same menu ([c73d30b](https://github.com/bpeiren/times-up/commit/c73d30b2a3e5e46ab942d59d86b1ac94e8ab05a5))
- **bar:** Centre the countdown in its block and show drag times ([c987e97](https://github.com/bpeiren/times-up/commit/c987e9712bf204e9767d8786e5d2a81f7a67e6db))

### Bug fixes

- Rename the app to "Time's up" everywhere ([ff2f13b](https://github.com/bpeiren/times-up/commit/ff2f13b31c84d31ff250831870fb054ff50786a8))

### Build and packaging

- Add the portable release package ([e6349f4](https://github.com/bpeiren/times-up/commit/e6349f4a77b7ee389007a2b4cdb6f08b14db6736))

### Documentation

- Add the MIT license ([c4e7985](https://github.com/bpeiren/times-up/commit/c4e7985af4b4481b3f6c7a39a7de669c2a27f560))

[0.0.2]: https://github.com/bpeiren/times-up/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/bpeiren/times-up/releases/tag/v0.0.1

