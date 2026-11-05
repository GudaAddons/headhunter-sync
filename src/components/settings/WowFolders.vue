<script setup lang="ts">
import { XIcon } from '@lucide/vue';
import ChooseWowFolder from '@/components/settings/ChooseWowFolder.vue';
import { Button } from '@/components/ui/button';

const folders = defineModel<string[]>({ required: true });

defineProps<{
    add: (picked: string) => Promise<string>;
}>();

const emit = defineEmits<{
    added: [folder: string];
}>();

function remove(folder: string): void {
    folders.value = folders.value.filter((f) => f !== folder);
}
</script>

<template>
    <section class="flex flex-col gap-3">
        <div class="flex flex-col gap-0.5">
            <h3 class="text-sm font-bold tracking-[0.2em] text-gold uppercase">
                {{ $t('World of Warcraft folder') }}
            </h3>
            <p class="text-xs text-muted-foreground">
                {{
                    $t(
                        'Found on its own from the Battle.net install. If your game is somewhere else, add its folder here.',
                    )
                }}
            </p>
        </div>
        <ul v-if="folders.length" class="flex flex-col gap-1.5">
            <li
                v-for="folder in folders"
                :key="folder"
                class="flex items-center gap-2 rounded-md border border-border bg-black/20 px-3 py-1.5 text-sm"
            >
                <span class="flex-1 truncate" :title="folder">{{ folder }}</span>
                <Button
                    variant="ghost"
                    size="icon-sm"
                    :aria-label="$t('Remove :folder', { folder })"
                    @click="remove(folder)"
                >
                    <XIcon />
                </Button>
            </li>
        </ul>
        <ChooseWowFolder :add="add" @added="(folder) => emit('added', folder)" />
    </section>
</template>
