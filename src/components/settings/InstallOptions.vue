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
import { CLIENT_LABELS, folderName } from '@/lib/format';
import type { InstallSettings, InstallStatus } from '@/types';

const props = defineProps<{
    install: InstallStatus;
}>();

const options = defineModel<InstallSettings>({ required: true });

const AUTO = 'auto';

// The Forever beta runs in the US only (until release on 2026-11-04)
const regions = computed(() =>
    props.install.client === 'forever'
        ? [{ value: 'us', label: 'Americas & Oceania' }]
        : [
              { value: 'us', label: 'Americas & Oceania' },
              { value: 'eu', label: 'Europe' },
              { value: 'kr', label: 'Korea' },
              { value: 'tw', label: 'Taiwan' },
              { value: 'cn', label: 'China' },
          ],
);

const REALM_TYPES = [
    { value: 'pvp', label: 'PvP' },
    { value: 'pve', label: 'PvE' },
    { value: 'roleplay', label: 'Roleplay' },
    { value: 'hardcore', label: 'Hardcore' },
];

const region = computed({
    get: () => options.value.region ?? AUTO,
    set: (value: string) => {
        options.value = { ...options.value, region: value === AUTO ? null : value };
    },
});

const realmType = computed({
    get: () => options.value.realm_type,
    set: (value: string) => {
        options.value = { ...options.value, realm_type: value };
    },
});
</script>

<template>
    <article class="flex flex-col gap-3 rounded-md border border-border bg-black/20 p-4">
        <Label class="flex items-center gap-2.5">
            <Checkbox
                :model-value="options.enabled"
                @update:model-value="(v) => (options = { ...options, enabled: v === true })"
            />
            <span class="font-heading tracking-wide text-gold uppercase">
                {{ CLIENT_LABELS[install.client] }}
            </span>
            <span class="truncate text-xs font-normal text-muted-foreground">
                {{ folderName(install.path) }}
            </span>
        </Label>
        <div class="grid gap-3 sm:grid-cols-2">
            <div class="grid gap-1">
                <span class="text-xs text-muted-foreground">Region</span>
                <Select v-model="region" :disabled="!options.enabled">
                    <SelectTrigger class="w-full bg-card/85" aria-label="Region">
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        <SelectItem :value="AUTO">From the addon</SelectItem>
                        <SelectItem
                            v-for="option in regions"
                            :key="option.value"
                            :value="option.value"
                        >
                            {{ option.label }}
                        </SelectItem>
                    </SelectContent>
                </Select>
            </div>
            <div v-if="install.client === 'forever'" class="grid gap-1">
                <span class="text-xs text-muted-foreground">World</span>
                <Select v-model="realmType" :disabled="!options.enabled">
                    <SelectTrigger class="w-full bg-card/85" aria-label="World">
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        <SelectItem
                            v-for="option in REALM_TYPES"
                            :key="option.value"
                            :value="option.value"
                        >
                            {{ option.label }}
                        </SelectItem>
                    </SelectContent>
                </Select>
            </div>
        </div>
    </article>
</template>
