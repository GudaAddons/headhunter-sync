import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { onMounted, onUnmounted, ref } from 'vue';
import type { Settings, Status, UploadInfo, User } from '@/types';

/** The app's state from the Rust side, refreshed whenever a sync moves on. */
export function useSync() {
    const status = ref<Status | null>(null);
    let stop: (() => void) | undefined;
    let timer: number | undefined;

    async function refresh(): Promise<void> {
        status.value = await invoke<Status>('get_status');
    }

    onMounted(async () => {
        await refresh();
        stop = await listen('status-changed', () => refresh());
        // Keeps "next sync in" and "x min ago" current
        timer = window.setInterval(refresh, 30_000);
    });

    onUnmounted(() => {
        stop?.();
        window.clearInterval(timer);
    });

    return {
        status,
        refresh,
        signIn: (email: string, password: string) => invoke<User>('sign_in', { email, password }),
        signOut: () => invoke('sign_out'),
        syncNow: () => invoke('sync_now'),
        getSettings: () => invoke<Settings>('get_settings'),
        saveSettings: (settings: Settings) => invoke('save_settings', { settings }),
        recentUploads: () => invoke<UploadInfo[]>('recent_uploads'),
    };
}
