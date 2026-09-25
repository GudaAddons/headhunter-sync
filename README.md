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

## Releasing an update

Production builds check `https://github.com/GudaAddons/headhunter-sync/releases/latest/download/latest.json`
on start and every 6 hours, and offer "Install and restart". Local builds never update.

1. `npm version 0.2.0` (sets package.json, Cargo.toml and Cargo.lock, commits and tags `v0.2.0`).
2. `git push --follow-tags`. GitHub Actions (`.github/workflows/release.yml`) builds on Windows,
   signs the update and publishes the release with `latest.json`.

The first release is the current version, so tag it directly: `git tag v0.1.0 && git push --follow-tags`.

Dry run: Actions > Release > "Run workflow" builds and signs the same way but makes only a draft
release `dry-run-v<version>` (against `https://dry-run.invalid` while the variable is unset).
Installed apps never see drafts. Delete the draft and its tag after checking the files.

The website's download button (`HEADHUNTER_APP_WINDOWS_URL`) points to
`https://github.com/GudaAddons/headhunter-sync/releases/latest`; file names carry the version,
so a direct file link would break with each release.

Repository settings the workflow needs: variable `HEADHUNTER_API_URL` (https), secrets
`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. The key pair lives in
`%USERPROFILE%\.tauri\headhunter-sync.key` (+ `.password`, `.pub`); the public key is in
`tauri.conf.json`. Keep a backup of the private key: without it, installed apps can never update again.

## How it works

- `src-tauri/src/installs.rs` finds the WoW folder (Blizzard registry key, common paths, or the
  folder you pick in Settings) and each account's `HeadHunter.lua`.
- `lua.rs` reads the file; `payload.rs` turns it into the website's upload format, one upload per
  character, only what is new since the last sync; demo and simulated data never leave the PC.
- `api.rs` signs in (one token per PC, kept in the Windows Credential Manager) and uploads;
  `sync.rs` runs the schedule, retries when offline and remembers what was sent (`state.json`
  in the app's data folder).
- `download.rs` brings the website's WANTED and Duels lists and your own records back: after each
  sync it writes `Interface\AddOns\HeadHunter_Data\` (a `.toc` and `Data.lua`), a small addon the
  game loads at the next login, so WoW Forever gets its data back too.
- The window (Vue 3 + shadcn-vue, in the website's look) shows sign in, status and settings.

The app only reads the HeadHunter saved file, writes the `HeadHunter_Data` addon folder and talks to
the HeadHunter website. Nothing else on the PC is touched.
