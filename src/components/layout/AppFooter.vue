<script setup lang="ts">
import { openUrl } from '@tauri-apps/plugin-opener';
import { computed } from 'vue';

const props = defineProps<{
    version: string;
    apiUrl: string;
}>();

const host = computed(() => props.apiUrl.replace(/^https?:\/\//, ''));
</script>

<template>
    <footer
        class="px-4 py-2 text-center text-[0.7rem] text-muted-foreground/70"
    >
        HeadHunter Sync {{ version }} ·
        <button
            type="button"
            class="cursor-pointer underline-offset-2 hover:text-gold hover:underline"
            :title="`Open ${apiUrl}`"
            @click="openUrl(apiUrl)"
        >
            {{ host }}
        </button>
    </footer>
</template>
