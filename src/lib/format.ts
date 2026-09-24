const relative = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });

/** "3 minutes ago" from Unix seconds */
export function ago(unixSeconds: number | null): string {
    if (!unixSeconds) {
        return 'never';
    }
    const seconds = Math.round(unixSeconds - Date.now() / 1000);
    const steps: [Intl.RelativeTimeFormatUnit, number][] = [
        ['day', 86_400],
        ['hour', 3_600],
        ['minute', 60],
    ];
    for (const [unit, size] of steps) {
        if (Math.abs(seconds) >= size) {
            return relative.format(Math.round(seconds / size), unit);
        }
    }
    return 'just now';
}

/** "in 42 min" */
export function soon(seconds: number | null): string | null {
    if (seconds === null) {
        return null;
    }
    const minutes = Math.max(1, Math.round(seconds / 60));
    return minutes >= 60 ? `in ${Math.round(minutes / 60)} h` : `in ${minutes} min`;
}

/** "Name-Realm" shows as "Name" with the realm apart */
export function splitKey(key: string): { name: string; realm: string | null } {
    const at = key.indexOf('-');
    return at > 0 ? { name: key.slice(0, at), realm: key.slice(at + 1) } : { name: key, realm: null };
}

export const CLIENT_LABELS = {
    era: 'Classic Era',
    forever: 'WoW Forever (Beta)',
} as const;

/** The last folder of a path, e.g. "_classic_era_" */
export function folderName(path: string): string {
    return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}
