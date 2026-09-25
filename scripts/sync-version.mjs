// package.json holds the version (tauri.conf.json reads it); `npm version <x.y.z>` runs
// this to copy it into Cargo.toml and Cargo.lock, which the app shows and sends.
import { readFileSync, writeFileSync } from 'node:fs';

const { version } = JSON.parse(readFileSync('package.json', 'utf8'));

function replace(path, pattern, replacement) {
    const text = readFileSync(path, 'utf8');
    if (!pattern.test(text)) {
        console.error(`No version found in ${path}`);
        process.exit(1);
    }
    writeFileSync(path, text.replace(pattern, replacement));
}

replace('src-tauri/Cargo.toml', /^version = ".*"$/m, `version = "${version}"`);
replace(
    'src-tauri/Cargo.lock',
    /(name = "headhunter-sync"\r?\nversion = )".*"/,
    `$1"${version}"`,
);
console.log(`Version ${version} in Cargo.toml and Cargo.lock`);
