# Changelog

## [0.4.1](https://github.com/tupe12334/wattop/compare/wattop-v0.4.0...wattop-v0.4.1) (2026-11-12)


### Bug Fixes

* keep 0 W power-out ports out of the charger ports ([7d5d909](https://github.com/tupe12334/wattop/commit/7d5d909518bd17c7b942facb1c1ca9f164d453ea))

## [0.4.0](https://github.com/tupe12334/wattop/compare/wattop-v0.3.4...wattop-v0.4.0) (2026-11-12)


### Features

* show USB-C port power out, e.g. a charging phone ([eb22146](https://github.com/tupe12334/wattop/commit/eb22146f410c914ab80b5ec3be87e368a3816b15))

## [0.3.4](https://github.com/tupe12334/wattop/compare/wattop-v0.3.3...wattop-v0.3.4) (2026-11-08)


### Bug Fixes

* hide charger-too-weak warning while holding, build docs.rs for macOS ([276739e](https://github.com/tupe12334/wattop/commit/276739e4a081e839eb7f6bc165cb039caecd33ee))

## [0.3.3](https://github.com/tupe12334/wattop/compare/wattop-v0.3.2...wattop-v0.3.3) (2026-11-08)


### Bug Fixes

* hold state at full charge instead of flipping charge/discharge ([60757e4](https://github.com/tupe12334/wattop/commit/60757e496315f9a7182a67a3041a6d9e1b4091e7))

## [0.3.2](https://github.com/tupe12334/wattop/compare/wattop-v0.3.1...wattop-v0.3.2) (2026-11-07)


### Bug Fixes

* dmg background shows in Finder, light readable design ([e60b643](https://github.com/tupe12334/wattop/commit/e60b6431dbfc443578b6f9955f393282622abe05))


### Documentation

* Mac App Store readiness checklist and sandbox spike results ([6ed237f](https://github.com/tupe12334/wattop/commit/6ed237f4be3c326602b467b1c76f53d2e5749b11))

## [0.3.1](https://github.com/tupe12334/wattop/compare/wattop-v0.3.0...wattop-v0.3.1) (2026-11-07)


### Bug Fixes

* seal app bundle with ad-hoc signature, laid-out dmg ([3b0a003](https://github.com/tupe12334/wattop/commit/3b0a0036f23ec4c07fb5e2c70d2909fdd76804f3))

## [0.3.0](https://github.com/tupe12334/wattop/compare/wattop-v0.2.0...wattop-v0.3.0) (2026-11-07)


### Features

* battery bar as ▰▱ segments with forward/backward pulse ([d8cecd0](https://github.com/tupe12334/wattop/commit/d8cecd03e97a2ff00dfd89fbd158e72bd4704ff6))
* every install path gets the menu-bar app ([b18c325](https://github.com/tupe12334/wattop/commit/b18c32554ddcce249ba76d1c33cb86876aee921f))


### Documentation

* install guide, how it works, menu-bar app, releasing, contributing ([d139546](https://github.com/tupe12334/wattop/commit/d139546d2d7a2d229c14594bb0778f9f7646e39d))

## [0.2.0](https://github.com/tupe12334/wattop/compare/wattop-v0.1.1...wattop-v0.2.0) (2026-11-07)


### Features

* animated battery cell, pulse forward on charge, backward on drain ([e887340](https://github.com/tupe12334/wattop/commit/e887340ead542190121324b7937969ce68473bdc))
* bus-topology energy flow, cleaner port outline ([0bbc1f9](https://github.com/tupe12334/wattop/commit/0bbc1f943c47be839aad10592dca80454dc9ee3d))
* hybrid-car style terminal power dashboard for macOS ([39382b1](https://github.com/tupe12334/wattop/commit/39382b1d9ce4afdea02b88e064446009d336f8eb))
* live SMC power readings, adapter name fallback, adaptive layout ([c362df4](https://github.com/tupe12334/wattop/commit/c362df4a069a71d0c47c5f9748d7904993b651b4))
* menu-bar app, Mac port outline, clearer chart key ([b7101ff](https://github.com/tupe12334/wattop/commit/b7101ff0f250bcc0e6bbe4549a7b73df22fe66c1))
* one-line equal-size energy flow, power-only port outline ([9cb7138](https://github.com/tupe12334/wattop/commit/9cb71386f39e8da8ea57ec7fed450b423b9f7c7e))
* show Low Power Mode in TUI readouts and menu-bar app ([671b771](https://github.com/tupe12334/wattop/commit/671b77189424b5f6984aed852f0dd247c969fbf7))


### Documentation

* explain SMC live readings ([e66b958](https://github.com/tupe12334/wattop/commit/e66b9580eb6a4164b5317602d5483e1fd8744fab))
* install from crates.io ([77bee42](https://github.com/tupe12334/wattop/commit/77bee42b21bb1005268804ea076600dab9f9cb75))
