<script setup lang="ts">
import { computed } from 'vue';
import { Checkbox } from '@/components/ui/checkbox';
import { Label } from '@/components/ui/label';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import type { Settings } from '@/types';

const settings = defineModel<Settings>({ required: true });

const INTERVALS = [
    { value: '0', label: 'Off' },
    { value: '15', label: 'Every 15 minutes' },
    { value: '30', label: 'Every 30 minutes' },
    { value: '60', label: 'Every hour' },
    { value: '180', label: 'Every 3 hours' },
];

const interval = computed({
    get: () => String(settings.value.interval_minutes),
    set: (value: string) => {
        settings.value = { ...settings.value, interval_minutes: Number(value) };
    },
});

function toggle(key: 'sync_on_change' | 'sync_on_start' | 'start_with_system', value: boolean | 'indeterminate'): void {
    settings.value = { ...settings.value, [key]: value === true };
}
</script>

<template>
    <section class="flex flex-col gap-3">
        <h3 class="text-sm font-bold tracking-[0.2em] text-gold uppercase">
            When to sync
        </h3>
        <Label class="flex items-start gap-2.5 font-normal">
            <Checkbox
                :model-value="settings.sync_on_change"
                @update:model-value="(v) => toggle('sync_on_change', v)"
            />
            <span>
                When the game saves
                <span class="block text-xs text-muted-foreground">
                    At logout, /reload or quitting, a few seconds after.
                </span>
            </span>
        </Label>
        <Label class="flex items-center gap-2.5 font-normal">
            <Checkbox
                :model-value="settings.sync_on_start"
                @update:model-value="(v) => toggle('sync_on_start', v)"
            />
            When HeadHunter Sync starts
        </Label>
        <div class="flex items-center gap-3">
            <span class="text-sm">Also</span>
            <Select v-model="interval">
                <SelectTrigger class="min-w-44 bg-card/85" aria-label="Timed sync">
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    <SelectItem
                        v-for="option in INTERVALS"
                        :key="option.value"
                        :value="option.value"
                    >
                        {{ option.label }}
                    </SelectItem>
                </SelectContent>
            </Select>
        </div>
        <Label class="flex items-center gap-2.5 font-normal">
            <Checkbox
                :model-value="settings.start_with_system"
                @update:model-value="(v) => toggle('start_with_system', v)"
            />
            Start with Windows (in the tray)
        </Label>
    </section>
</template>
