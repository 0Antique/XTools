<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { CheckCircle2, AlertCircle, X } from 'lucide-vue-next';
import LauncherView from './views/LauncherView.vue';
import ClipboardView from './views/ClipboardView.vue';
import RenameView from './views/RenameView.vue';
import SettingsView from './views/SettingsView.vue';
import { desktop } from './lib/native';
import { useUiStore } from './stores/ui';
import { useSettingsStore } from './stores/settings';

type View = 'launcher' | 'clipboard' | 'rename' | 'settings';
const initial = new URLSearchParams(location.search).get('view');
const launcherWindow = initial !== 'rename' && initial !== 'settings';
const view = ref<View>(initial === 'rename' || initial === 'settings' || initial === 'clipboard' ? initial : 'launcher');
const ui = useUiStore();
const settings = useSettingsStore();
const unlisteners: UnlistenFn[] = [];
let disposed = false;
onMounted(async () => {
  if (!desktop) return;
  await settings.load().catch(ui.fail);
  for (const [event, handler] of [
    ['launcher-shown', () => { if (launcherWindow) view.value = 'launcher'; }],
    ['tool-opened', (event: { payload: unknown }) => { const payload = event.payload as { id?: string } | string; const id = typeof payload === 'string' ? payload : payload?.id; if (launcherWindow && id === 'clipboard') view.value = 'clipboard'; }],
    ['settings-changed', () => { void settings.load().catch(ui.fail); }],
    ['scan-error', (event: { payload: unknown }) => { ui.fail(event.payload); }],
  ] as const) {
    const unlisten = await listen(event, handler).catch((e) => { ui.fail(e); return undefined; });
    if (unlisten) { if (disposed) unlisten(); else unlisteners.push(unlisten); }
  }
});
onUnmounted(() => { disposed = true; unlisteners.forEach((fn) => fn()); });
</script>

<template>
  <main class="window-shell" :class="[`view-${view}`, { 'browser-preview': !desktop }]">
    <LauncherView v-if="view === 'launcher'" @clipboard="view = 'clipboard'" />
    <ClipboardView v-else-if="view === 'clipboard'" @back="view = 'launcher'" />
    <RenameView v-else-if="view === 'rename'" />
    <SettingsView v-else />
  </main>
  <div v-if="ui.notice" class="toast" :class="ui.noticeKind" role="status"><AlertCircle v-if="ui.noticeKind === 'error'" :size="18" /><CheckCircle2 v-else :size="18" /><span>{{ ui.notice }}</span><button class="icon-button" aria-label="关闭提示" @click="ui.notice = ''"><X :size="15" /></button></div>
</template>
