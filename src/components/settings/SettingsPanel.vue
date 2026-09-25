<script setup lang="ts">
import { CircleCheckIcon } from '@lucide/vue';
import { onMounted, ref } from 'vue';
import InstallOptions from '@/components/settings/InstallOptions.vue';
import SyncTiming from '@/components/settings/SyncTiming.vue';
import WowFolders from '@/components/settings/WowFolders.vue';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import type { InstallSettings, InstallStatus, Settings } from '@/types';

const props = defineProps<{
    installs: InstallStatus[];
    load: () => Promise<Settings>;
    save: (settings: Settings) => Promise<unknown>;
}>();

const emit = defineEmits<{
    saved: [];
}>();

const settings = ref<Settings | null>(null);
const saving = ref(false);
const saved = ref(false);
const error = ref<string | null>(null);

onMounted(async () => {
    settings.value = await props.load();
});

function optionsFor(install: InstallStatus): InstallSettings {
    return settings.value?.installs[install.path] ?? install.settings;
}

function setOptions(install: InstallStatus, options: InstallSettings): void {
    if (settings.value) {
        settings.value = {
            ...settings.value,
            installs: { ...settings.value.installs, [install.path]: options },
        };
    }
}

async function submit(): Promise<void> {
    if (!settings.value) {
        return;
    }
    saving.value = true;
    error.value = null;
    try {
        await props.save(settings.value);
        saved.value = true;
        window.setTimeout(() => (saved.value = false), 2500);
        emit('saved');
    } catch (e) {
        error.value = String(e);
    } finally {
        saving.value = false;
    }
}
</script>

<template>
    <section
        v-if="settings"
        class="frame-gold flex flex-col gap-5 rounded-md bg-card/90 p-5"
    >
        <WowFolders v-model="settings.wow_folders" />
        <div v-if="installs.length" class="flex flex-col gap-2">
            <InstallOptions
                v-for="install in installs"
                :key="install.path"
                :install="install"
                :model-value="optionsFor(install)"
                @update:model-value="(options) => setOptions(install, options)"
            />
        </div>
        <p v-else class="text-sm text-wanted">
            No Classic Era or WoW Forever folder found. Add your World of
            Warcraft folder above.
        </p>
        <Separator />
        <SyncTiming v-model="settings" />
        <p v-if="error" class="text-sm text-wanted">{{ error }}</p>
        <div class="flex items-center gap-3">
            <Button variant="gold" :disabled="saving" @click="submit">
                Save
            </Button>
            <span
                v-if="saved"
                class="flex items-center gap-1 text-sm text-emerald-400"
            >
                <CircleCheckIcon class="size-4" />
                Saved
            </span>
        </div>
    </section>
</template>
