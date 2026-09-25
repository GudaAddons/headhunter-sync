<script setup lang="ts">
import { CircleAlertIcon, ClockIcon, DownloadIcon } from '@lucide/vue';
import { computed } from 'vue';
import { ago } from '@/lib/format';
import type { DownloadResult } from '@/types';

const props = defineProps<{
    download: DownloadResult;
}>();

const RESULTS = {
    updated: { icon: DownloadIcon, tone: 'text-emerald-400' },
    unchanged: { icon: DownloadIcon, tone: 'text-emerald-400' },
    retry: { icon: ClockIcon, tone: 'text-gold' },
    error: { icon: CircleAlertIcon, tone: 'text-wanted' },
} as const;

const result = computed(() => RESULTS[props.download.outcome]);
</script>

<template>
    <p class="flex items-start gap-2 text-xs text-muted-foreground">
        <component
            :is="result.icon"
            class="mt-px size-3.5 shrink-0"
            :class="result.tone"
        />
        <span>
            <template v-if="download.written_at">
                Website data in game {{ ago(download.written_at) }} ·
                {{ download.wanted }} WANTED
            </template>
            <template v-else>No website data in the game yet</template>
            <template v-if="download.message">
                · <span :class="result.tone">{{ download.message }}</span>
            </template>
        </span>
    </p>
</template>
