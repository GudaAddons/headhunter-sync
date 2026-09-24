<script setup lang="ts">
import { FolderPlusIcon, XIcon } from '@lucide/vue';
import { open } from '@tauri-apps/plugin-dialog';
import { Button } from '@/components/ui/button';

const folders = defineModel<string[]>({ required: true });

async function add(): Promise<void> {
    const picked = await open({
        directory: true,
        title: 'Pick your World of Warcraft folder (or its _classic_era_ folder)',
    });
    if (typeof picked === 'string' && !folders.value.includes(picked)) {
        folders.value = [...folders.value, picked];
    }
}

function remove(folder: string): void {
    folders.value = folders.value.filter((f) => f !== folder);
}
</script>

<template>
    <section class="flex flex-col gap-3">
        <div class="flex flex-col gap-0.5">
            <h3 class="text-sm font-bold tracking-[0.2em] text-gold uppercase">
                World of Warcraft folder
            </h3>
            <p class="text-xs text-muted-foreground">
                Found on its own from the Battle.net install. If your game is
                somewhere else, add its folder here.
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
                    :aria-label="`Remove ${folder}`"
                    @click="remove(folder)"
                >
                    <XIcon />
                </Button>
            </li>
        </ul>
        <Button variant="outline" class="self-start" @click="add">
            <FolderPlusIcon />
            Add a WoW folder
        </Button>
    </section>
</template>
