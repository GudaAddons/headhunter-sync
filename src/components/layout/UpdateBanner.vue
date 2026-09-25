<script setup lang="ts">
import { CircleAlertIcon, DownloadIcon, LoaderCircleIcon } from '@lucide/vue';
import { ref } from 'vue';
import { Button } from '@/components/ui/button';
import type { UpdateInfo } from '@/types';

const props = defineProps<{
    update: UpdateInfo;
    install: () => Promise<unknown>;
}>();

const installing = ref(false);
const error = ref<string | null>(null);

async function run(): Promise<void> {
    installing.value = true;
    error.value = null;
    try {
        // The app restarts on the new version when this succeeds
        await props.install();
    } catch (e) {
        error.value = String(e);
        installing.value = false;
    }
}
</script>

<template>
    <section
        class="frame-gold flex flex-col gap-2 rounded-md bg-card/90 px-4 py-3"
    >
        <div class="flex items-center gap-3">
            <DownloadIcon class="size-5 shrink-0 text-gold" />
            <p class="min-w-0 flex-1 text-sm">
                <span class="font-heading font-semibold tracking-wide"
                    >Version {{ update.version }}</span
                >
                is ready.
            </p>
            <Button
                variant="gold"
                size="sm"
                :disabled="installing"
                @click="run"
            >
                <LoaderCircleIcon v-if="installing" class="animate-spin" />
                {{ installing ? 'Installing...' : 'Install and restart' }}
            </Button>
        </div>
        <p
            v-if="update.notes"
            class="text-xs whitespace-pre-line text-muted-foreground"
        >
            {{ update.notes }}
        </p>
        <p
            v-if="error"
            class="flex items-center gap-2 text-xs text-destructive"
        >
            <CircleAlertIcon class="size-4 shrink-0" />
            {{ error }}
        </p>
    </section>
</template>
