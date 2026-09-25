<script setup lang="ts">
import { CircleAlertIcon } from '@lucide/vue';
import { openUrl } from '@tauri-apps/plugin-opener';
import { ref } from 'vue';
import BrowserSignIn from '@/components/auth/BrowserSignIn.vue';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

const props = defineProps<{
    apiUrl: string;
    signIn: (email: string, password: string) => Promise<unknown>;
    signInWithBrowser: () => Promise<unknown>;
    cancelBrowserSignIn: () => Promise<unknown>;
}>();

const emit = defineEmits<{
    signedIn: [];
}>();

const email = ref('');
const password = ref('');
const busy = ref(false);
const error = ref<string | null>(null);

async function submit(): Promise<void> {
    busy.value = true;
    error.value = null;
    try {
        await props.signIn(email.value, password.value);
        password.value = '';
        emit('signedIn');
    } catch (e) {
        error.value = String(e);
    } finally {
        busy.value = false;
    }
}
</script>

<template>
    <section class="frame-gold flex flex-col gap-5 rounded-md bg-card/90 p-6">
        <header class="flex flex-col items-center gap-2 text-center">
            <h1 class="text-xl font-bold tracking-[0.15em] text-gold uppercase">
                Sign in
            </h1>
            <div class="divider-gold w-28" />
            <p class="text-sm text-muted-foreground">
                Use your HeadHunter website account. Your reports ride to the
                bounty board on their own after that.
            </p>
        </header>

        <BrowserSignIn
            :start="signInWithBrowser"
            :cancel="cancelBrowserSignIn"
            @signed-in="emit('signedIn')"
        />

        <div
            class="flex items-center gap-3 text-xs tracking-widest text-muted-foreground uppercase"
        >
            <span class="h-px flex-1 bg-border" />
            or with email
            <span class="h-px flex-1 bg-border" />
        </div>

        <Alert v-if="error" variant="destructive" class="bg-card/90">
            <CircleAlertIcon />
            <AlertDescription>{{ error }}</AlertDescription>
        </Alert>

        <form class="grid gap-4" @submit.prevent="submit">
            <div class="grid gap-1.5">
                <Label for="email">Email</Label>
                <Input
                    id="email"
                    v-model="email"
                    type="email"
                    autocomplete="email"
                    required
                />
            </div>
            <div class="grid gap-1.5">
                <Label for="password">Password</Label>
                <Input
                    id="password"
                    v-model="password"
                    type="password"
                    autocomplete="current-password"
                    required
                />
            </div>
            <Button type="submit" variant="outline" size="lg" :disabled="busy">
                {{ busy ? 'Signing in...' : 'Sign in' }}
            </Button>
        </form>

        <p class="text-center text-sm text-muted-foreground">
            No account yet?
            <button
                type="button"
                class="text-gold hover:underline"
                @click="openUrl(`${apiUrl}/register`)"
            >
                Create one on the website
            </button>
        </p>
        <p class="text-center text-xs text-muted-foreground/70">{{ apiUrl }}</p>
    </section>
</template>
