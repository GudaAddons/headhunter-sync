<script setup lang="ts">
import { CircleAlertIcon, RefreshCwIcon } from '@lucide/vue';
import { computed } from 'vue';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { ago, soon } from '@/lib/format';
import type { Status } from '@/types';

const props = defineProps<{
    status: Status;
}>();

const emit = defineEmits<{
    syncNow: [];
}>();

const next = computed(() => soon(props.status.next_run_in));
</script>

<template>
    <section class="frame-gold flex flex-col gap-4 rounded-md bg-card/90 p-5">
        <div class="flex items-center gap-4">
            <div class="flex flex-1 flex-col gap-0.5">
                <span
                    class="font-heading text-xs font-semibold tracking-[0.3em] text-gold uppercase"
                >
                    Last sync
                </span>
                <span class="text-lg">
                    {{ status.syncing ? 'Syncing now...' : ago(status.last_run) }}
                </span>
                <span v-if="next" class="text-xs text-muted-foreground">
                    Next timed sync {{ next }}
                </span>
            </div>
            <Button
                variant="gold"
                size="lg"
                :disabled="status.syncing"
                @click="emit('syncNow')"
            >
                <RefreshCwIcon :class="{ 'animate-spin': status.syncing }" />
                Sync now
            </Button>
        </div>
        <Alert v-if="status.problem" variant="destructive" class="bg-card/90">
            <CircleAlertIcon />
            <AlertDescription>{{ status.problem }}</AlertDescription>
        </Alert>
    </section>
</template>
