# HeadHunter Sync

*Your reports, ridden to the bounty board.*

A small desktop app for the **HeadHunter - Wanted: Dead or Alive** WoW addon. It reads the
addon's saved data (`WTF\Account\<account>\SavedVariables\HeadHunter.lua`) and uploads your
deaths, catches, duels and bounty to the HeadHunter website. It lives in the system tray and
syncs on its own: when the game saves (logout, `/reload`, quitting), when it starts, and on a
timer you choose. "Sync now" is always there too.

Supports **Classic Era** (`_classic_era_`) and **WoW Forever** (`_classic_beta_` while in beta).

Design and tickets: `head-hunter-web/docs/app/` (plan.md, tickets.md).

## Builds

| Command | What it is |
|---|---|
| `npm run tauri:dev` | Develop against the local website (`http://localhost`, Laravel Sail) |
| `npm run build:local` | Installer "HeadHunter Sync (Local)" for the local website; installs next to the production app |
| `npm run build:prod` | Installer "HeadHunter Sync" for the production website. Needs `HEADHUNTER_API_URL=https://...`; there is no production website yet, so it stops with a message |
| `npm test` | Rust unit tests (Lua reader, payload mapping, settings, installs, API messages) |

Installers land in `src-tauri/target/release/bundle/` (`nsis/*.exe`, `msi/*.msi`).

Needs Node 20+, Rust (rustup, MSVC toolchain on Windows) and WebView2 (built into Windows 10/11).

## How it works

- `src-tauri/src/installs.rs` finds the WoW folder (Blizzard registry key, common paths, or the
  folder you pick in Settings) and each account's `HeadHunter.lua`.
- `lua.rs` reads the file; `payload.rs` turns it into the website's upload format, one upload per
  character, only what is new since the last sync; demo and simulated data never leave the PC.
- `api.rs` signs in (one token per PC, kept in the Windows Credential Manager) and uploads;
  `sync.rs` runs the schedule, retries when offline and remembers what was sent (`state.json`
  in the app's data folder).
- The window (Vue 3 + shadcn-vue, in the website's look) shows sign in, status and settings.

The app only reads the HeadHunter saved file and talks to the HeadHunter website. Nothing else on
the PC is touched.
