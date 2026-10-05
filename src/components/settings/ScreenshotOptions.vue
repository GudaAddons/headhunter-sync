<script setup lang="ts">
import { Checkbox } from '@/components/ui/checkbox';
import { Label } from '@/components/ui/label';

const deleteAfterUpload = defineModel<boolean>({ required: true });
</script>

<template>
    <section class="flex flex-col gap-3">
        <h3 class="text-sm font-bold tracking-[0.2em] text-gold uppercase">
            {{ $t('Screenshots') }}
        </h3>
        <Label class="flex items-start gap-2.5 font-normal">
            <Checkbox
                :model-value="deleteAfterUpload"
                @update:model-value="(v) => (deleteAfterUpload = v === true)"
            />
            <span>
                {{ $t('Delete HeadHunter screenshots after upload') }}
                <span class="block text-xs text-muted-foreground">
                    {{ $t('Only the pictures HeadHunter took, once the website has them.') }}
                </span>
            </span>
        </Label>
    </section>
</template>
