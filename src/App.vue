<script setup lang="ts">
import { onMounted, watch } from 'vue';
import SignInForm from '@/components/auth/SignInForm.vue';
import AppFooter from '@/components/layout/AppFooter.vue';
import AppHeader from '@/components/layout/AppHeader.vue';
import UpdateBanner from '@/components/layout/UpdateBanner.vue';
import ChooseWowFolder from '@/components/settings/ChooseWowFolder.vue';
import SettingsPanel from '@/components/settings/SettingsPanel.vue';
import InstallCard from '@/components/status/InstallCard.vue';
import RecentUploads from '@/components/status/RecentUploads.vue';
import SyncOverview from '@/components/status/SyncOverview.vue';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useSync } from '@/composables/useSync';
import { language, systemLanguage } from '@/lib/i18n';

const sync = useSync();
const { status } = sync;

// The Rust side decides the language; "auto" needs the system's, which the window knows
onMounted(() => void sync.setSystemLanguage(systemLanguage()));
watch(
    () => status.value?.language,
    (value) => {
        if (value) {
            language.value = value;
            document.documentElement.lang = value.replace('_', '-');
        }
    },
    { immediate: true },
);

async function signOut(): Promise<void> {
    await sync.signOut();
    await sync.refresh();
}
</script>

<template>
    <div v-if="status" class="bg-leather flex h-full flex-col">
        <AppHeader
            :build="status.build"
            :user="status.user"
            @sign-out="signOut"
        />

        <main class="flex flex-1 flex-col gap-4 overflow-y-auto p-4">
            <UpdateBanner
                v-if="sync.update.value"
                :update="sync.update.value"
                :install="sync.installUpdate"
            />
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
                    <TabsTrigger value="status">{{ $t('Status') }}</TabsTrigger>
                    <TabsTrigger value="settings">{{ $t('Settings') }}</TabsTrigger>
                </TabsList>

                <TabsContent value="status" class="flex flex-col gap-4">
                    <SyncOverview :status="status" @sync-now="sync.syncNow" />
                    <InstallCard
                        v-for="install in status.installs"
                        :key="install.path"
                        :install="install"
                    />
                    <div
                        v-if="!status.installs.length"
                        class="frame-gold flex flex-col gap-3 rounded-md bg-card/90 p-5 text-sm"
                    >
                        <p>
                            {{
                                $t(
                                    'No WoW Forever folder found. Choose your World of Warcraft folder.',
                                )
                            }}
                        </p>
                        <ChooseWowFolder
                            :add="sync.addWowFolder"
                            @added="sync.refresh"
                        />
                    </div>
                    <RecentUploads :load="sync.recentUploads" />
                </TabsContent>

                <TabsContent value="settings">
                    <SettingsPanel
                        :installs="status.installs"
                        :load="sync.getSettings"
                        :save="sync.saveSettings"
                        :add-wow-folder="sync.addWowFolder"
                        @saved="sync.refresh"
                    />
                </TabsContent>
            </Tabs>
        </main>

        <AppFooter :version="status.version" :api-url="status.api_url" />
    </div>
</template>
