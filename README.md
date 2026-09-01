# What The F\*ck Is My PC Doing?

Windows Task Manager tells you Chrome is at 12%. This app tells you **why the machine feels like that**, and what to do about it.

Local. Private. Honest. No account. No cloud.

[https://github.com/osfv/WhatTheFuckIsMyPcDoing](https://github.com/osfv/WhatTheFuckIsMyPcDoing)

## Why this exists

Task Manager is a spreadsheet of processes. You still have to know what `dwm.exe` is, why RAM sits at 83% with "nothing open", or why the disk is busy when Explorer looks idle.

This is a local explainer:

- Plain-language headlines ("Your PC is chilling" vs "RAM is getting crowded")
- Click CPU or RAM and see **what is actually using it**, Chrome-style grouped
- Hardware inventory with real vendor marks, live VRAM, disk queue and active time
- Alerts that wait for a problem to stick around, so a 2-second spike does not scream
- Discord Rich Presence: friends see live CPU/RAM and a GitHub button (off until you turn it on)
- Everything stays on the PC. History is SQLite in app data.

## Stack

**Tauri 2 + React 19 + TypeScript + Vite + Tailwind v4 + Rust.** Telemetry, analysis, tray, and Discord IPC live in Rust. The UI is a dark desktop shell.

## Run

```bash
npm install
npm run tauri dev
```

Windows. Needs a working Rust toolchain for the Tauri side.

## Privacy

No telemetry to us. Discord presence is opt-in and only talks to the Discord app on this machine. Kill process is a last resort and asks first.

## License

MIT
