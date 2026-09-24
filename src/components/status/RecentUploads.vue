<script setup lang="ts">
import { onMounted, ref } from 'vue';
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

onMounted(async () => {
    try {
        uploads.value = (await props.load()).slice(0, 8);
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
                <span class="truncate">{{ upload.character ?? '?' }}</span>
                <span
                    :class="
                        upload.status === 'rejected'
                            ? 'text-wanted'
                            : 'text-muted-foreground'
                    "
                    :title="upload.error ?? ''"
                >
                    {{ STATUS[upload.status] ?? upload.status }}
                </span>
            </li>
        </ul>
    </section>
</template>
