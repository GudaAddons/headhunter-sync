<script setup lang="ts">
import SignInForm from '@/components/auth/SignInForm.vue';
import AppHeader from '@/components/layout/AppHeader.vue';
import SettingsPanel from '@/components/settings/SettingsPanel.vue';
import InstallCard from '@/components/status/InstallCard.vue';
import RecentUploads from '@/components/status/RecentUploads.vue';
import SyncOverview from '@/components/status/SyncOverview.vue';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useSync } from '@/composables/useSync';

const sync = useSync();
const { status } = sync;

async function signOut(): Promise<void> {
    await sync.signOut();
    await sync.refresh();
}
</script>

<template>
    <div v-if="status" class="bg-leather flex h-full flex-col">
        <AppHeader :build="status.build" :user="status.user" />

        <main class="flex-1 overflow-y-auto p-4">
            <SignInForm
                v-if="!status.user"
                :api-url="status.api_url"
                :sign-in="sync.signIn"
                :sign-in-with-browser="sync.signInWithBrowser"
                :cancel-browser-sign-in="sync.cancelBrowserSignIn"
                @signed-in="sync.refresh"
            />

            <Tabs v-else default-value="status" class="gap-4">
                <TabsList class="w-full">
                    <TabsTrigger value="status">Status</TabsTrigger>
                    <TabsTrigger value="settings">Settings</TabsTrigger>
                </TabsList>

                <TabsContent value="status" class="flex flex-col gap-4">
                    <SyncOverview :status="status" @sync-now="sync.syncNow" />
                    <InstallCard
                        v-for="install in status.installs"
                        :key="install.path"
                        :install="install"
                    />
                    <p
                        v-if="!status.installs.length"
                        class="frame-gold rounded-md bg-card/90 p-5 text-sm"
                    >
                        No Classic Era or WoW Forever folder found. Add your
                        World of Warcraft folder in Settings.
                    </p>
                    <RecentUploads :load="sync.recentUploads" />
                </TabsContent>

                <TabsContent value="settings">
                    <SettingsPanel
                        :installs="status.installs"
                        :load="sync.getSettings"
                        :save="sync.saveSettings"
                        @saved="sync.refresh"
                        @sign-out="signOut"
                    />
                </TabsContent>
            </Tabs>
        </main>

        <footer class="px-4 py-2 text-center text-[0.7rem] text-muted-foreground/70">
            HeadHunter Sync {{ status.version }} · {{ status.api_url }}
        </footer>
    </div>
</template>
