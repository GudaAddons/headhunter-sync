/** Mirrors the Rust structs sent to the window (src-tauri/src/sync.rs, store.rs, api.rs). */

export type Client = 'era' | 'forever';

export type User = {
    name: string;
    email: string | null;
};

export type UpdateInfo = {
    version: string;
    notes: string | null;
};

export type SyncResult = {
    at: number;
    outcome: 'sent' | 'already_there' | 'claimed' | 'rejected' | 'retry' | 'error';
    message: string | null;
    records: number;
};

export type InstallSettings = {
    enabled: boolean;
    region: string | null;
    realm_type: string;
};

export type CharacterStatus = {
    key: string;
    last_played: boolean;
    result: SyncResult | null;
};

export type AccountStatus = {
    name: string;
    characters: CharacterStatus[];
    error: string | null;
};

export type DownloadResult = {
    at: number;
    outcome: 'updated' | 'unchanged' | 'retry' | 'error';
    message: string | null;
    written_at: number | null;
    wanted: number;
};

export type InstallStatus = {
    path: string;
    client: Client;
    addon_version: string | null;
    settings: InstallSettings;
    accounts: AccountStatus[];
    download: DownloadResult | null;
};

export type Status = {
    build: string;
    api_url: string;
    version: string;
    user: User | null;
    syncing: boolean;
    last_run: number | null;
    next_run_in: number | null;
    problem: string | null;
    installs: InstallStatus[];
};

export type Settings = {
    wow_folders: string[];
    installs: Record<string, InstallSettings>;
    sync_on_change: boolean;
    sync_on_start: boolean;
    interval_minutes: number;
    start_with_system: boolean;
    user_name: string | null;
    email: string | null;
};

export type UploadInfo = {
    id: number;
    status: string;
    character: string | null;
    created_at: string | null;
    error: string | null;
};
