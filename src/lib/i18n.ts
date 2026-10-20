import type { App } from 'vue';
import { ref } from 'vue';
import ja from '@/locales/ja.json';
import ko from '@/locales/ko.json';
import ru from '@/locales/ru.json';
import zhCN from '@/locales/zh_CN.json';

/**
 * The app's languages, like the website: the English text is the key (`$t('Sync now')`
 * in templates, `trans('Sync now')` in code, `:name` for values), and
 * each other language's src/locales/<language>.json (zh_CN, ko, ru, ja) has its text
 * for every key. The Rust side reads the same files for the tray and notifications, and
 * decides the language (Status.language).
 */
const TEXTS: Record<string, Record<string, string>> = { zh_CN: zhCN, ko, ru, ja };

export const language = ref('en');

export function trans(key: string, values: Record<string, string | number> = {}): string {
    const text = TEXTS[language.value]?.[key] ?? key;
    return Object.entries(values)
        .sort(([a], [b]) => b.length - a.length)
        .reduce((out, [name, value]) => out.split(`:${name}`).join(String(value)), text);
}

/** The system's language for the "auto" setting, e.g. "zh-CN". */
export function systemLanguage(): string {
    return navigator.languages?.[0] ?? navigator.language ?? 'en';
}

/** For Intl dates: "zh-CN", "ko", "en"… */
export function intlLocale(): string {
    return language.value.replace('_', '-');
}

export const i18n = {
    install(app: App): void {
        app.config.globalProperties.$t = trans;
    },
};

declare module 'vue' {
    interface ComponentCustomProperties {
        $t: typeof trans;
    }
}
