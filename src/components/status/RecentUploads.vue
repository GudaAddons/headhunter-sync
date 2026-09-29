<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { ago } from '@/lib/format';
import type { UploadInfo } from '@/types';

const props = defineProps<{
    load: () => Promise<UploadInfo[]>;
}>();

const uploads = ref<UploadInfo[]>([]);
const error = ref<string | null>(null);

const STATUS = {
    pending: 'Waiting on the website',
    parsed: 'On the board',
    rejected: 'Refused',
} as Record<string, string>;

/** The list is newest first, so the first upload of each character is its latest. */
function latestPerCharacter(list: UploadInfo[]): UploadInfo[] {
    const seen = new Set<string>();
    return list.filter((upload) => {
        const key = String(upload.character_id ?? upload.character);
        if (seen.has(key)) {
            return false;
        }
        seen.add(key);
        return true;
    });
}

function sentAgo(upload: UploadInfo): string {
    return upload.created_at ? ago(Date.parse(upload.created_at) / 1000) : '';
}

onMounted(async () => {
    try {
        uploads.value = latestPerCharacter(await props.load());
    } catch (e) {
        error.value = String(e);
    }
});
</script>

<template>
    <section class="frame-gold flex flex-col gap-2 rounded-md bg-card/90 p-5">
        <h2 class="text-sm font-bold tracking-[0.2em] text-gold uppercase">
            On the website
        </h2>
        <p v-if="error" class="text-sm text-muted-foreground">{{ error }}</p>
        <p v-else-if="!uploads.length" class="text-sm text-muted-foreground">
            No uploads yet.
        </p>
        <ul v-else class="divide-y divide-border text-sm">
            <li
                v-for="upload in uploads"
                :key="upload.id"
                class="flex items-center justify-between gap-3 py-1.5"
            >
                <span class="truncate">
                    {{ upload.character ?? '?' }}
                    <span v-if="upload.realm" class="text-muted-foreground">
                        - {{ upload.realm }}
                    </span>
                </span>
                <span
                    class="shrink-0 text-right"
                    :class="
                        upload.status === 'rejected'
                            ? 'text-wanted'
                            : 'text-muted-foreground'
                    "
                    :title="upload.error ?? ''"
                >
                    {{ STATUS[upload.status] ?? upload.status }}
                    <template v-if="sentAgo(upload)">
                        · {{ sentAgo(upload) }}
                    </template>
                </span>
            </li>
        </ul>
    </section>
</template>
