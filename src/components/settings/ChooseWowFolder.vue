<script setup lang="ts">
import { FolderPlusIcon } from '@lucide/vue';
import { open } from '@tauri-apps/plugin-dialog';
import { ref } from 'vue';
import { Button } from '@/components/ui/button';
import { trans } from '@/lib/i18n';

const props = defineProps<{
    add: (picked: string) => Promise<string>;
}>();

const emit = defineEmits<{
    added: [folder: string];
}>();

const error = ref<string | null>(null);

/** Any folder near the WoW folder will do: the Rust side finds the WoW folder and saves it. */
async function choose(): Promise<void> {
    const picked = await open({
        directory: true,
        title: trans('Choose your World of Warcraft folder'),
    });
    if (typeof picked !== 'string') {
        return;
    }
    error.value = null;
    try {
        emit('added', await props.add(picked));
    } catch (e) {
        error.value = String(e);
    }
}
</script>

<template>
    <div class="flex flex-col gap-2">
        <Button variant="outline" class="self-start" @click="choose">
            <FolderPlusIcon />
            {{ $t('Choose WoW folder') }}
        </Button>
        <p v-if="error" class="text-sm text-wanted">{{ error }}</p>
    </div>
</template>
