<script setup lang="ts">
import { CircleCheckIcon } from '@lucide/vue';
import { onMounted, ref, watch } from 'vue';
import InstallOptions from '@/components/settings/InstallOptions.vue';
import LanguageSelect from '@/components/settings/LanguageSelect.vue';
import ScreenshotOptions from '@/components/settings/ScreenshotOptions.vue';
import SyncTiming from '@/components/settings/SyncTiming.vue';
import WowFolders from '@/components/settings/WowFolders.vue';
import { Separator } from '@/components/ui/separator';
import type { InstallSettings, InstallStatus, Settings } from '@/types';

const props = defineProps<{
    installs: InstallStatus[];
    load: () => Promise<Settings>;
    save: (settings: Settings) => Promise<unknown>;
    addWowFolder: (picked: string) => Promise<string>;
}>();

/** Milliseconds a change waits before it is saved, so typing or quick clicks save once. */
const SAVE_DELAY = 400;

const emit = defineEmits<{
    saved: [];
}>();

const settings = ref<Settings | null>(null);
const saved = ref(false);
const error = ref<string | null>(null);

let timer: number | undefined;
let loading = true;

async function reload(): Promise<void> {
    loading = true;
    settings.value = await props.load();
}

onMounted(reload);

/** Every change saves on its own, a moment after the last one. */
watch(
    settings,
    () => {
        if (loading) {
            loading = false;
            return;
        }
        window.clearTimeout(timer);
        timer = window.setTimeout(submit, SAVE_DELAY);
    },
    { deep: true },
);

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
    error.value = null;
    try {
        await props.save(settings.value);
        saved.value = true;
        window.setTimeout(() => (saved.value = false), 2500);
        emit('saved');
    } catch (e) {
        error.value = String(e);
    }
}
</script>

<template>
    <section
        v-if="settings"
        class="frame-gold flex flex-col gap-5 rounded-md bg-card/90 p-5"
    >
        <WowFolders
            v-model="settings.wow_folders"
            :add="addWowFolder"
            @added="reload"
        />
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
            {{
                $t(
                    'No WoW Forever folder found. Choose your World of Warcraft folder.',
                )
            }}
        </p>
        <Separator />
        <SyncTiming v-model="settings" />
        <Separator />
        <ScreenshotOptions v-model="settings.delete_screenshots" />
        <Separator />
        <LanguageSelect v-model="settings.language" />
        <p v-if="error" class="text-sm text-wanted">{{ error }}</p>
        <div class="flex h-5 items-center gap-3">
            <span
                v-if="saved"
                class="flex items-center gap-1 text-sm text-emerald-400"
            >
                <CircleCheckIcon class="size-4" />
                {{ $t('Saved') }}
            </span>
        </div>
    </section>
</template>
