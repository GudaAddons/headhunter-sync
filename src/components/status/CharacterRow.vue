<script setup lang="ts">
import {
    CircleAlertIcon,
    CircleCheckIcon,
    CircleDashedIcon,
    ClockIcon,
} from '@lucide/vue';
import { computed } from 'vue';
import { Badge } from '@/components/ui/badge';
import { ago, splitKey } from '@/lib/format';
import type { CharacterStatus } from '@/types';

const props = defineProps<{
    character: CharacterStatus;
}>();

const who = computed(() => splitKey(props.character.key));

const RESULTS = {
    sent: { icon: CircleCheckIcon, tone: 'text-emerald-400', text: 'Sent' },
    already_there: { icon: CircleCheckIcon, tone: 'text-emerald-400', text: 'Up to date' },
    retry: { icon: ClockIcon, tone: 'text-gold', text: 'Will try again' },
    claimed: { icon: CircleAlertIcon, tone: 'text-wanted', text: 'Not synced' },
    rejected: { icon: CircleAlertIcon, tone: 'text-wanted', text: 'Not synced' },
    error: { icon: CircleAlertIcon, tone: 'text-wanted', text: 'Not synced' },
} as const;

const result = computed(() =>
    props.character.result ? RESULTS[props.character.result.outcome] : null,
);
</script>

<template>
    <li class="flex items-start gap-3 py-2.5">
        <component
            :is="result?.icon ?? CircleDashedIcon"
            class="mt-0.5 size-4 shrink-0"
            :class="result?.tone ?? 'text-muted-foreground'"
        />
        <div class="flex min-w-0 flex-1 flex-col gap-0.5">
            <div class="flex items-center gap-2">
                <span class="truncate font-heading font-semibold tracking-wide">
                    {{ who.name }}
                </span>
                <span v-if="who.realm" class="text-xs text-muted-foreground">
                    {{ who.realm }}
                </span>
                <Badge
                    v-if="character.last_played"
                    variant="outline"
                    class="rounded-none border-gold/40 text-[0.65rem] text-gold uppercase"
                >
                    Played last
                </Badge>
            </div>
            <span class="text-xs text-muted-foreground">
                <template v-if="character.result">
                    {{ result?.text }} {{ ago(character.result.at) }}
                    <template v-if="character.result.records">
                        · {{ character.result.records }} records
                    </template>
                </template>
                <template v-else>Nothing sent yet</template>
            </span>
            <span
                v-if="character.result?.message"
                class="text-xs"
                :class="result?.tone"
            >
                {{ character.result.message }}
            </span>
        </div>
    </li>
</template>
