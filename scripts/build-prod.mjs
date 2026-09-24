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

const result = spawnSync('npx', ['tauri', 'build'], {
    stdio: 'inherit',
    shell: true,
    env: { ...process.env, HEADHUNTER_BUILD: 'prod' },
});
process.exit(result.status ?? 1);
