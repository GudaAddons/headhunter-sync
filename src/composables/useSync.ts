import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { onMounted, onUnmounted, ref } from 'vue';
import type {
    Settings,
    Status,
    UpdateInfo,
    UploadInfo,
    User,
} from '@/types';

/** The app's state from the Rust side, refreshed whenever a sync moves on. */
export function useSync() {
    const status = ref<Status | null>(null);
    const update = ref<UpdateInfo | null>(null);
    const stops: (() => void)[] = [];
    let timer: number | undefined;

    async function refresh(): Promise<void> {
        status.value = await invoke<Status>('get_status');
    }

    onMounted(async () => {
        await refresh();
        stops.push(await listen('status-changed', () => refresh()));
        stops.push(
            await listen<UpdateInfo>('update-available', (event) => {
                update.value = event.payload;
            }),
        );
        // Keeps "next sync in" and "x min ago" current
        timer = window.setInterval(refresh, 30_000);
        update.value = await invoke<UpdateInfo | null>('check_update').catch(
            () => null,
        );
    });

    onUnmounted(() => {
        stops.forEach((stop) => stop());
        window.clearInterval(timer);
    });

    return {
        status,
        refresh,
        update,
        installUpdate: () => invoke('install_update'),
        signIn: (email: string, password: string) => invoke<User>('sign_in', { email, password }),
        signInWithBrowser: () => invoke<User>('sign_in_with_browser'),
        cancelBrowserSignIn: () => invoke('cancel_browser_sign_in'),
        signOut: () => invoke('sign_out'),
        syncNow: () => invoke('sync_now'),
        getSettings: () => invoke<Settings>('get_settings'),
        saveSettings: (settings: Settings) => invoke('save_settings', { settings }),
        recentUploads: () => invoke<UploadInfo[]>('recent_uploads'),
    };
}
