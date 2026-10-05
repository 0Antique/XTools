<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { Search, X, Settings, ArrowUp, ArrowDown, CornerDownLeft, LoaderCircle, AppWindow, Sparkles } from 'lucide-vue-next';
import TitleBar from '../components/TitleBar.vue';
import AppIcon from '../components/AppIcon.vue';
import DesktopNotice from '../components/DesktopNotice.vue';
import { command, desktop, hideWindow, hotkeyLabel } from '../lib/native';
import { useLauncherStore } from '../stores/launcher';
import { useSettingsStore } from '../stores/settings';
import { useUiStore } from '../stores/ui';
import type { SearchResult, ToolId } from '../types';

const emit = defineEmits<{ clipboard: [] }>();
const store = useLauncherStore();
const settings = useSettingsStore();
const ui = useUiStore();
const searchInput = ref<HTMLInputElement>();
const launching = ref(false);
const tools: SearchResult[] = [
  { id: 'clipboard', name: '剪贴板', kind: 'tool', subtitle: '文字、图片与文件历史' },
  { id: 'color', name: '屏幕取色', kind: 'tool', subtitle: '放大像素，捕捉颜色' },
  { id: 'rename', name: '批量重命名', kind: 'tool', subtitle: '统一命名，实时预览' },
];
const recent = computed(() => settings.settings.showRecent ? store.results.filter((item) => item.kind === 'application').slice(0, 8) : []);
const entries = computed(() => store.query.trim() ? store.results : [...recent.value, ...tools]);
let timer: ReturnType<typeof setTimeout> | undefined;
const unlisteners: UnlistenFn[] = [];
let disposed = false;
watch(() => store.query, () => { store.invalidate(); if (timer) clearTimeout(timer); timer = setTimeout(() => { void store.search(); }, 10); }, { flush: 'sync' });
watch(() => settings.settings.showRecent, () => { void store.search(); });
watch(() => store.selected, async () => { await nextTick(); document.querySelector('[data-selected="true"]')?.scrollIntoView({ block: 'nearest' }); });

async function reset() {
  store.query = '';
  store.selected = 0;
  await nextTick();
  searchInput.value?.focus();
  await store.search();
}
async function openTool(id: ToolId) {
  if (!desktop) { ui.notify('请在 XTools 桌面版中打开此工具。', 'error'); return; }
  try {
    await command('open_tool', { id });
    if (id === 'clipboard') emit('clipboard');
  } catch (e) { ui.fail(e); }
}
async function activate(item?: SearchResult) {
  if (!item || launching.value) return;
  if (item.kind === 'tool') { await openTool(item.id as ToolId); return; }
  launching.value = true;
  try { await command('launch_application', { id: item.id }); }
  catch (e) { ui.fail(e); }
  finally { launching.value = false; }
}
function keydown(event: KeyboardEvent) {
  if (event.isComposing || event.ctrlKey || event.metaKey || event.altKey) return;
  if (event.key === 'Escape') { event.preventDefault(); void hideWindow().catch(ui.fail); }
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    event.preventDefault();
    const count = entries.value.length;
    if (count) store.selected = (store.selected + (event.key === 'ArrowDown' ? 1 : -1) + count) % count;
  }
  if (event.key === 'Enter' && !(event.target instanceof HTMLButtonElement)) { event.preventDefault(); void activate(entries.value[store.selected]); }
}
onMounted(async () => {
  document.addEventListener('keydown', keydown);
  await reset();
  if (desktop) {
    for (const event of ['launcher-shown', 'applications-updated']) {
      const unlisten = await listen(event, () => { void (event === 'launcher-shown' ? reset() : store.search()); }).catch((e) => { ui.fail(e); return undefined; });
      if (unlisten) { if (disposed) unlisten(); else unlisteners.push(unlisten); }
    }
  }
});
onUnmounted(() => { disposed = true; document.removeEventListener('keydown', keydown); if (timer) clearTimeout(timer); unlisteners.forEach((fn) => fn()); });
</script>

<template>
  <TitleBar />
  <div class="launcher-search"><Search class="search-icon" :size="25" :stroke-width="1.7" /><input ref="searchInput" v-model="store.query" type="text" placeholder="搜索应用或 XTools 功能" aria-label="搜索应用或 XTools 功能" autocomplete="off" spellcheck="false" /><LoaderCircle v-if="store.loading || launching" class="spinning muted" :size="20" /><button v-else-if="store.query" class="icon-button" aria-label="清空搜索" @click="store.query = ''; searchInput?.focus()"><X :size="19" /></button><kbd v-else class="launch-hotkey">{{ hotkeyLabel(settings.settings.hotkey) }}</kbd></div>
  <div class="launcher-body scroll-area">
    <DesktopNotice v-if="!desktop" />
    <div v-if="store.error" class="inline-error" role="alert">{{ store.error }}<button class="text-button" @click="store.search">重试</button></div>
    <template v-if="!store.query.trim()">
      <section v-if="settings.settings.showRecent" class="recent-section"><div class="section-label"><span>最近使用</span><span class="section-caption">从这里继续</span></div><div v-if="recent.length" class="recent-grid"><button v-for="(item, index) in recent" :key="item.id" class="recent-card" :class="{ selected: store.selected === index }" :data-selected="store.selected === index" :disabled="launching" @click="activate(item)" @mouseenter="store.selected = index"><AppIcon :item="item" /><span class="recent-name" :title="item.name">{{ item.name }}</span></button></div><div v-else class="recent-empty"><AppWindow :size="24" :stroke-width="1.5" /><div><p>{{ desktop ? '常用应用，从第一次启动开始' : '最近使用将在桌面版中显示' }}</p><span>{{ desktop ? '搜索并打开应用，最近使用会自动出现在这里。' : '应用索引保存在本机，预览不会访问系统数据。' }}</span></div></div></section>
      <section class="tools-section"><div class="section-label"><span>XTools 工具</span><span class="section-caption">让琐事变简单</span></div><div class="tools-grid"><button v-for="(item, index) in tools" :key="item.id" class="tool-card" :class="{ selected: store.selected === recent.length + index }" :data-selected="store.selected === recent.length + index" @click="activate(item)" @mouseenter="store.selected = recent.length + index"><AppIcon :item="item" /><span class="tool-name">{{ item.name }}</span><span class="tool-description">{{ item.subtitle }}</span><span class="tool-alias">{{ item.id === 'clipboard' ? 'cb' : item.id === 'color' ? 'color' : 'rn' }} <CornerDownLeft :size="11" /></span></button></div></section>
      <p class="launcher-note"><Sparkles :size="13" /> 中文、拼音和首字母，都能找到你的应用</p>
    </template>
    <template v-else>
      <div class="section-label search-result-label"><span>{{ store.results.length ? '搜索结果' : '搜索' }}</span><span class="section-caption">{{ store.results.length }} 个结果</span></div>
      <div v-if="!store.results.length && !store.loading" class="empty-state"><Search :size="32" :stroke-width="1.4" /><h3>{{ desktop ? '没有找到匹配项' : '在桌面版中搜索应用' }}</h3><p>{{ desktop ? '试试应用名称、拼音，或 cb、color、rename。' : '此预览不包含模拟的应用或剪贴板记录。' }}</p></div>
      <div class="search-results"><button v-for="(item, index) in store.results" :key="`${item.kind}-${item.id}`" class="result-row" :class="{ selected: store.selected === index }" :data-selected="store.selected === index" :disabled="launching" @mouseenter="store.selected = index" @click="activate(item)"><AppIcon :item="item" /><span class="result-copy"><strong>{{ item.name }}</strong><span>{{ item.subtitle || (item.kind === 'tool' ? 'XTools 内置工具' : '应用程序') }}</span></span><span v-if="index === 0" class="best-match">最佳匹配</span><CornerDownLeft v-if="store.selected === index" class="muted" :size="17" /></button></div>
    </template>
  </div>
  <footer class="launcher-footer"><div class="keyboard-hints"><span><kbd><ArrowUp :size="11" /></kbd><kbd><ArrowDown :size="11" /></kbd> 选择</span><span><kbd>Enter</kbd> 打开</span><span><kbd>Esc</kbd> 隐藏</span></div><button class="footer-settings" @click="openTool('settings')"><Settings :size="15" /><span>设置</span></button></footer>
</template>
