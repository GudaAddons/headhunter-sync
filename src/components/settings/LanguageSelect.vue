<script setup lang="ts">
import { computed } from 'vue';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import { trans } from '@/lib/i18n';

const language = defineModel<string>({ required: true });

// Each language in its own words, so a player can find theirs
const options = computed(() => [
    { value: 'auto', label: trans('Same as the system') },
    { value: 'en', label: 'English' },
    { value: 'zh_CN', label: '中文' },
]);
</script>

<template>
    <section class="flex flex-col gap-3">
        <h3 class="text-sm font-bold tracking-[0.2em] text-gold uppercase">
            {{ $t('Language') }}
        </h3>
        <Select v-model="language">
            <SelectTrigger
                class="w-full max-w-60 bg-card/85"
                :aria-label="$t('Language')"
            >
                <SelectValue />
            </SelectTrigger>
            <SelectContent>
                <SelectItem
                    v-for="option in options"
                    :key="option.value"
                    :value="option.value"
                    :lang="option.value === 'zh_CN' ? 'zh-CN' : undefined"
                >
                    {{ option.label }}
                </SelectItem>
            </SelectContent>
        </Select>
    </section>
</template>
