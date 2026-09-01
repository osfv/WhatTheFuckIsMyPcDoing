<p align="center">
  <img src="docs/icon.png" width="72" height="72" alt="WTF pulse icon" />
</p>

<h1 align="center">What The F\*ck Is My PC Doing?</h1>

<p align="center">
  Local. Private. Honest.<br />
  Windows Task Manager tells you Chrome is at 12%.<br />
  This tells you why the machine feels like that.
</p>

<p align="center">
  <img src="docs/screenshots/overview.png" alt="Overview: RAM at 80%, Cursor using 3.9 GB, and a plain-language recommendation" width="980" />
</p>

<p align="center">
  <img src="docs/screenshots/hardware.png" alt="Hardware inventory with Intel, NVIDIA, ASUS, and LG marks. Serials blurred." width="980" />
</p>

## Inside

<img src="docs/icons/chat.svg" width="16" height="16" alt="" /> **Headlines, not a spreadsheet.** "Your PC is chilling" vs "RAM is getting crowded."

<img src="docs/icons/cpu.svg" width="16" height="16" alt="" /> **Click CPU, RAM, GPU, or VRAM.** See what is actually using it, Chrome grouped.

<img src="docs/icons/heartbeat.svg" width="16" height="16" alt="" /> **Hardware with real vendor marks.** Live VRAM, disk queue, active time. Not a WMI dump in a list view.

<img src="docs/icons/pulse.svg" width="16" height="16" alt="" /> **Alerts that wait.** A 2-second spike does not scream.

<img src="docs/icons/lock.svg" width="16" height="16" alt="" /> **Stays on the machine.** SQLite in app data. No account. Discord presence is off until you turn it on.

## Run

```bash
npm install
npm run tauri dev
```

Windows. Rust toolchain required for the Tauri side.

**Tauri 2 + React 19 + TypeScript + Vite + Tailwind v4 + Rust.** Telemetry, analysis, tray, and Discord IPC live in Rust.

## License

MIT
