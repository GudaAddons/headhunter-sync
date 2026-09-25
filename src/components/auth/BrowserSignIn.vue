<script setup lang="ts">
import { CircleAlertIcon, GlobeIcon, LoaderCircleIcon } from '@lucide/vue';
import { ref } from 'vue';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';

// Battle.net, Discord and Google accounts have no password: they sign in on the website
const props = defineProps<{
    start: () => Promise<unknown>;
    cancel: () => Promise<unknown>;
}>();

const emit = defineEmits<{
    signedIn: [];
}>();

const waiting = ref(false);
const error = ref<string | null>(null);

async function signIn(): Promise<void> {
    waiting.value = true;
    error.value = null;
    try {
        await props.start();
        emit('signedIn');
    } catch (e) {
        error.value = String(e);
    } finally {
        waiting.value = false;
    }
}
</script>

<template>
    <div class="grid gap-3">
        <Alert v-if="error" variant="destructive" class="bg-card/90">
            <CircleAlertIcon />
            <AlertDescription>{{ error }}</AlertDescription>
        </Alert>

        <template v-if="waiting">
            <p
                class="flex items-center justify-center gap-2 text-sm text-muted-foreground"
            >
                <LoaderCircleIcon class="size-4 animate-spin text-gold" />
                Finish signing in in your browser...
            </p>
            <Button variant="outline" size="lg" @click="cancel">Cancel</Button>
        </template>
        <Button v-else variant="gold" size="lg" @click="signIn">
            <GlobeIcon />
            Sign in with browser
        </Button>
        <p class="text-center text-xs text-muted-foreground">
            Also for accounts made with Battle.net, Discord or Google.
        </p>
    </div>
</template>
