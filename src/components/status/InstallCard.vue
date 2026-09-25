<script setup lang="ts">
import { FolderIcon } from '@lucide/vue';
import CharacterRow from '@/components/status/CharacterRow.vue';
import GameDataLine from '@/components/status/GameDataLine.vue';
import { Badge } from '@/components/ui/badge';
import { CLIENT_LABELS, folderName } from '@/lib/format';
import type { InstallStatus } from '@/types';

defineProps<{
    install: InstallStatus;
}>();
</script>

<template>
    <article class="frame-gold flex flex-col gap-3 rounded-md bg-card/90 p-5">
        <header class="flex items-start gap-3">
            <FolderIcon class="mt-1 size-5 shrink-0 text-gold" />
            <div class="flex min-w-0 flex-1 flex-col gap-0.5">
                <h2 class="font-bold tracking-wider text-gold uppercase">
                    {{ CLIENT_LABELS[install.client] }}
                </h2>
                <span
                    class="truncate text-xs text-muted-foreground"
                    :title="install.path"
                >
                    {{ folderName(install.path) }} ·
                    {{
                        install.addon_version
                            ? `HeadHunter ${install.addon_version}`
                            : 'HeadHunter addon not installed'
                    }}
                </span>
            </div>
            <Badge
                v-if="!install.settings.enabled"
                variant="secondary"
                class="rounded-none"
            >
                Off
            </Badge>
        </header>

        <GameDataLine v-if="install.download" :download="install.download" />

        <p
            v-if="!install.accounts.length"
            class="text-sm text-muted-foreground"
        >
            No HeadHunter data yet. Play with the addon, then log out: the game
            saves it then.
        </p>

        <section
            v-for="account in install.accounts"
            :key="account.name"
            class="flex flex-col"
        >
            <span
                class="text-[0.7rem] tracking-[0.25em] text-muted-foreground uppercase"
            >
                Account {{ account.name }}
            </span>
            <p v-if="account.error" class="text-sm text-wanted">
                {{ account.error }}
            </p>
            <p
                v-else-if="!account.characters.length"
                class="py-2 text-sm text-muted-foreground"
            >
                Nothing to sync yet.
            </p>
            <ul v-else class="divide-y divide-border">
                <CharacterRow
                    v-for="character in account.characters"
                    :key="character.key"
                    :character="character"
                />
            </ul>
        </section>
    </article>
</template>
