# Changelog

All notable changes to this project will be documented in this file.

## [0.8.8] - 2026-07-15

### ♻️ Refactor

- *(cli)* Soft-only update reminder: print new version banner with releases URL, no interactive `[y/N]` install prompt

## [0.8.7] - 2026-07-15

### 🐛 Bug Fixes

- *(proxy)* Prevent EMFILE (`Too many open files`) under high concurrency by reusing `RequestClient`, caching hot-path settings, skipping rules directory fingerprints on cache hits, and capping concurrent accepts so localhost self-service stays available

### 🧪 Testing

- *(proxy)* Add EMFILE resilience coverage for shared client reuse and concurrent localhost health checks

## [0.8.6] - 2026-06-29

### 🚀 Features

- *(daemon)* Support custom restart params and hide PID/uptime on stopped status

## [0.8.5] - 2026-06-26

### 🐛 Bug Fixes

- *(auth)* Allow static assets (JS/CSS/images) to load without authentication so the login page can render properly
- *(cert)* Fix `cert uninstall` failing with "No matching lynxProxy certificate found" when root.pem was regenerated — now removes all lynxProxy certificates from the System Keychain regardless of fingerprint match

### ⚙️ Miscellaneous Tasks

- *(release)* Add release skill to `.github/skills/release/` to standardize the release workflow

## [0.8.4] - 2026-06-26

### 🐛 Bug Fixes

- *(auth)* Allow static assets (JS/CSS/images) to load without authentication so the login page can render properly
- *(cert)* Fix `cert uninstall` failing with "No matching lynxProxy certificate found" when root.pem was regenerated — now removes all lynxProxy certificates from the System Keychain regardless of fingerprint match

## [0.8.3] - 2026-06-26

### 🚀 Features

- *(cli)* Auto-check for new version on every command; interactive update prompt when newer release is available
- *(cli)* Use `lynx-cli-update` (cargo-dist updater) when updating

## [0.8.2] - 2026-06-25

### 🐛 Bug Fixes

- *(daemon)* Fix `lynx start -u -p` failing with "invalid type: map, expected a sequence" by making `/api/base_info/address` a public endpoint
- *(daemon)* Show detailed running instance info (PID, port, auth, data dir) when daemon is already running

## [0.8.1] - 2026-06-25

### 🐛 Bug Fixes

- *(daemon)* Forward `-u`/`-p` auth credentials to the subprocess in daemon mode
- *(storage)* Remove unused `ConnectType` enum and `connect_type` field from `GeneralSetting`
- *(docs)* Remove stale `--connect-type` CLI option from README

## [0.7.1] - 2026-06-17

### 🐛 Bug Fixes

- *(cli)* Install Lynx root CA into macOS System Keychain and fix cert status detection
- *(docs)* Update README and Chinese README for System Keychain install flow

## [0.7.0] - 2026-06-16

### 🚀 Features

- Rules schema + project config

## [0.6.0] - 2026-06-16

### 🚀 Features

- Add Android ADB proxy with drawer UI and lazy platform-tools install
- Add API Studio workbench and replace api_debug persistence ([#63](https://github.com/xin2017338/lynx-proxy/issues/63))
- *(network)* Persist traffic filter history on backend via WebSocket ([#65](https://github.com/xin2017338/lynx-proxy/issues/65))

### 🐛 Bug Fixes

- *(api-studio)* Address PR review for draft saves, timestamps, and a11y
- *(ui)* Resolve TypeScript errors blocking release build
- *(ui)* Read WS connection state via storeToRefs in filter history

## [0.5.1] - 2026-06-04

### 🐛 Bug Fixes

- *(ui)* Use hash routing for refresh-safe Web UI (v0.5.1)

## [0.5.0] - 2026-06-04

### 🐛 Bug Fixes

- *(ci)* Sync ui package-lock and use Node 22 for release build
- *(ci)* Use npm install in release UI build

### 📚 Documentation

- Enhance README and README.zh-CN with detailed rule matching and action descriptions

## [0.4.8] - 2025-08-03

### 🚀 Features

- Support custom columns ([#54](https://github.com/xin2017338/lynx-proxy/issues/54))

## [0.4.7] - 2025-08-01

### 🐛 Bug Fixes

- 在其上下文中，该请求的地址无效

## [0.4.6] - 2025-08-01

### 🐛 Bug Fixes

- Window ip bug ([#49](https://github.com/xin2017338/lynx-proxy/issues/49))
- Window ip bug ([#50](https://github.com/xin2017338/lynx-proxy/issues/50))

## [0.4.5] - 2025-08-01

### 🚀 Features

- Support sse

### 🐛 Bug Fixes

- Hex layout

## [0.4.4] - 2025-07-29

### 🚜 Refactor

- Remove ssl check

## [0.4.3] - 2025-07-29

### 🚀 Features

- *(cli)* Support localhost only mode

## [0.4.2] - 2025-07-29

### 🐛 Bug Fixes

- Tree bug

### 🎨 Styling

- Ui

## [0.4.1] - 2025-07-27

### 🚀 Features

- Support switch connect type

## [0.4.0] - 2025-07-26

### 🚀 Features

- Better content modify
- Api debug tree ([#46](https://github.com/xin2017338/lynx-proxy/issues/46))

## [0.3.8] - 2025-07-26

### 🚀 Features

- Pwa support
- Filter template
- *(apiDebug)* 添加响应覆盖功能按钮并优化布局样式

### 🚜 Refactor

- *(apiDebug)* 优化请求历史记录的分页和状态管理

## [0.3.7] - 2025-07-25

### 🚀 Features

- General setting ([#44](https://github.com/xin2017338/lynx-proxy/issues/44))

### 💼 Other

- Ui layout ([#43](https://github.com/xin2017338/lynx-proxy/issues/43))

### 🚜 Refactor

- Setting config ([#45](https://github.com/xin2017338/lynx-proxy/issues/45))

## [0.3.6] - 2025-07-15

### 🚀 Features

- Batch handler rule ([#34](https://github.com/xin2017338/lynx-proxy/issues/34))
- Add import and export rules
- Supports using SSE to retrieve request logs. ([#41](https://github.com/xin2017338/lynx-proxy/issues/41))

### 🐛 Bug Fixes

- Compression bug ([#36](https://github.com/xin2017338/lynx-proxy/issues/36))
- Network request traceid error ([#39](https://github.com/xin2017338/lynx-proxy/issues/39))
- Proxy forward ui ([#40](https://github.com/xin2017338/lynx-proxy/issues/40))
- Scroll does not reach the bottom
- Dead lock

## [0.3.3] - 2025-07-01

### 🚀 Features

- Support request delay ([#30](https://github.com/xin2017338/lynx-proxy/issues/30))

## [0.3.1] - 2025-07-01

### 🚀 Features

- Support client proxy ([#32](https://github.com/xin2017338/lynx-proxy/issues/32))

## [0.3.0] - 2025-06-27

### 🚀 Features

- *(core)* Support api request ([#27](https://github.com/xin2017338/lynx-proxy/issues/27))

## [0.2.5] - 2025-06-21

### 🚀 Features

- Better display of status time ([#28](https://github.com/xin2017338/lynx-proxy/issues/28))
- Support inject script ([#29](https://github.com/xin2017338/lynx-proxy/issues/29))

## [0.2.4] - 2025-06-13

### 🚀 Features

- Pwa and one click add block rule ([#23](https://github.com/xin2017338/lynx-proxy/issues/23))
- Support jaeger log ([#25](https://github.com/xin2017338/lynx-proxy/issues/25))

## [0.2.3] - 2025-06-10

### 🐛 Bug Fixes

- Daemons don't behave as expected with empty bodies   ([#20](https://github.com/xin2017338/lynx-proxy/issues/20))

### 🚜 Refactor

- Refactor Network UI ([#16](https://github.com/xin2017338/lynx-proxy/issues/16))

## [0.2.2] - 2025-06-08

### 🚀 Features

- The command line supports daemons ([#15](https://github.com/xin2017338/lynx-proxy/issues/15))

## [0.2.1] - 2025-06-05

### 🐛 Bug Fixes

- Dark mode bug

## [0.2.0] - 2025-06-05

### 🚀 Features

- Proxy Interception Support ([#8](https://github.com/xin2017338/lynx-proxy/issues/8))

## [0.1.7] - 2025-05-26

### 🐛 Bug Fixes

- Cli start error

## [0.1.6] - 2025-05-26

### 🐛 Bug Fixes

- Table style and websocket log ([#6](https://github.com/xin2017338/lynx-proxy/issues/6))
- Record time error ([#7](https://github.com/xin2017338/lynx-proxy/issues/7))
- Record time error
- Test case

## [0.1.5] - 2025-05-22

### 🚀 Features

- Support a more user-friendly experience for rule config and network tree  ([#39](https://github.com/xin2017338/lynx-proxy/issues/39))
- Support html,css,js,font,video,image,font content preview ([#41](https://github.com/xin2017338/lynx-proxy/issues/41))
- Filter support and limit the number of size ([#42](https://github.com/xin2017338/lynx-proxy/issues/42))
- Websocket support ([#43](https://github.com/xin2017338/lynx-proxy/issues/43))
- Add some layer ([#46](https://github.com/xin2017338/lynx-proxy/issues/46))
- Add axum and swagger ([#47](https://github.com/xin2017338/lynx-proxy/issues/47))
- Add request session event ([#2](https://github.com/xin2017338/lynx-proxy/issues/2))

### 🐛 Bug Fixes

- A lot of bugs

### 🚜 Refactor

- Refactoring everything ([#44](https://github.com/xin2017338/lynx-proxy/issues/44))

## [0.1.4] - 2025-02-17

### 🐛 Bug Fixes

- Unable to create dir on startup  ([#36](https://github.com/xin2017338/lynx-proxy/issues/36))
- Http1.1, http 1.0 proxy request and lose some header ([#34](https://github.com/xin2017338/lynx-proxy/issues/34))

## [0.1.3] - 2025-02-15

### 🐛 Bug Fixes

- Window local ip ([#33](https://github.com/xin2017338/lynx-proxy/issues/33))
- *(ui)* Clear request log and content ui bug in request tree struce  ([#35](https://github.com/xin2017338/lynx-proxy/issues/35))

## [0.1.2] - 2025-02-14

### 🚜 Refactor

- Use include dir replace static dir ([#32](https://github.com/xin2017338/lynx-proxy/issues/32))

## [0.1.1] - 2025-02-14

### 🐛 Bug Fixes

- Ui assert not found

## [0.1.0] - 2025-02-13

### 🚀 Features

- Rule support
- Add rule group
- Support tariui ([#5](https://github.com/xin2017338/lynx-proxy/issues/5))
- *(lynx-core)* Support glob match model 
- Support more access ip
- Support certificate download and install doc
- Fetch request log in the app context ([#13](https://github.com/xin2017338/lynx-proxy/issues/13))
- Support clear request log ([#16](https://github.com/xin2017338/lynx-proxy/issues/16))
- Support ssl capture switch and ssl capture rule ([#18](https://github.com/xin2017338/lynx-proxy/issues/18))
- Support better default config dir and support specifying dir ([#21](https://github.com/xin2017338/lynx-proxy/issues/21))

### 🐛 Bug Fixes

- Parse request log to json

<!-- generated by git-cliff -->
