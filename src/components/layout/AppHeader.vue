<script setup lang="ts">
import { CrosshairIcon, LogOutIcon } from '@lucide/vue';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type { User } from '@/types';

defineProps<{
    build: string;
    user: User | null;
}>();

const emit = defineEmits<{
    signOut: [];
}>();
</script>

<template>
    <header
        class="bg-wood flex items-center gap-3 px-5 py-3 shadow-[0_4px_18px_oklch(0_0_0/70%)]"
    >
        <span
            class="grid size-9 place-items-center rounded-full border border-gold/60 bg-black/40 text-gold"
        >
            <CrosshairIcon class="size-5" />
        </span>
        <div class="flex flex-col leading-none">
            <span
                class="font-heading text-lg font-extrabold tracking-[0.2em] text-gold uppercase"
            >
                HeadHunter
            </span>
            <span class="font-western text-xs tracking-wider text-stone-300">
                Sync
            </span>
        </div>
        <Badge
            v-if="build !== 'prod'"
            variant="outline"
            class="rounded-none border-gold/40 text-gold uppercase"
        >
            {{ build }}
        </Badge>
        <div v-if="user" class="ml-auto flex min-w-0 items-center gap-1">
            <span class="truncate text-sm text-stone-300">
                {{ user.name }}
            </span>
            <Button
                variant="ghost"
                size="sm"
                class="text-stone-300 hover:text-gold"
                title="Sign out"
                @click="emit('signOut')"
            >
                <LogOutIcon />
                Sign out
            </Button>
        </div>
    </header>
</template>
