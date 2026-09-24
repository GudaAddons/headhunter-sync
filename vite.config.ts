import { fileURLToPath, URL } from 'node:url';
import process from 'node:process';
import tailwindcss from '@tailwindcss/vite';
import vue from '@vitejs/plugin-vue';
import { defineConfig } from 'vite';

const host = process.env.TAURI_DEV_HOST;

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig(() => ({
    plugins: [vue(), tailwindcss()],
    resolve: {
        alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
    },
    // Tauri shows Rust errors; a fixed port for its dev window
    clearScreen: false,
    server: {
        port: 1420,
        strictPort: true,
        host: host || false,
        hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
        watch: { ignored: ['**/src-tauri/**'] },
    },
}));
