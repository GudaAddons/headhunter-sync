// The production build: talks to the website at HEADHUNTER_API_URL (https only).
// There is no production website yet, so this stops until the address is set:
//   set HEADHUNTER_API_URL=https://example.com && npm run build:prod
import { spawnSync } from 'node:child_process';

const url = process.env.HEADHUNTER_API_URL ?? '';

if (!url.startsWith('https://')) {
    console.error(
        'HEADHUNTER_API_URL must be the website\'s https:// address for the production build.\n' +
            'There is no production website yet: use "npm run build:local" for now.',
    );
    process.exit(1);
}

// Updates are signed; installed apps accept only files signed with this key
if (!process.env.TAURI_SIGNING_PRIVATE_KEY && !process.env.TAURI_SIGNING_PRIVATE_KEY_PATH) {
    console.error(
        'Set TAURI_SIGNING_PRIVATE_KEY_PATH (and TAURI_SIGNING_PRIVATE_KEY_PASSWORD) to sign the update.\n' +
            'The key lives in %USERPROFILE%\\.tauri\\headhunter-sync.key; releases are normally built by GitHub Actions.',
    );
    process.exit(1);
}

const result = spawnSync('npx', ['tauri', 'build'], {
    stdio: 'inherit',
    shell: true,
    env: { ...process.env, HEADHUNTER_BUILD: 'prod' },
});
process.exit(result.status ?? 1);
