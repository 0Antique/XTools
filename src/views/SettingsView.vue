<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { Keyboard, Power, Clipboard, LayoutGrid, RefreshCw, LoaderCircle, ShieldCheck, Check, FolderSearch, AlertCircle } from 'lucide-vue-next';
import TitleBar from '../components/TitleBar.vue';
import BrandMark from '../components/BrandMark.vue';
import DesktopNotice from '../components/DesktopNotice.vue';
import { command, desktop, hideWindow, hotkeyLabel } from '../lib/native';
import { useSettingsStore } from '../stores/settings';
import { useUiStore } from '../stores/ui';
import type { Settings } from '../types';

const store = useSettingsStore();
const ui = useUiStore();
const recording = ref(false);
const candidate = ref('');
const rescanning = ref(false);
const scanResult = ref('');
const historyLimit = ref(store.settings.clipboardLimit);
const warnings = ref<string[]>([]);
const disabled = computed(() => !desktop || !store.loaded || store.saving);
watch(() => store.settings.clipboardLimit, (value) => { historyLimit.value = value; });
async function save(patch: Partial<Settings>) {
  try { await store.save(patch); ui.notify('设置已生效'); }
  catch (e) { ui.fail(e); historyLimit.value = store.settings.clipboardLimit; }
  finally { await loadWarnings(); }
}
async function loadWarnings() {
  if (!desktop) return;
  try { const messages = await command<string[]>('get_runtime_warnings'); warnings.value = Array.isArray(messages) ? messages : []; }
  catch (e) { ui.fail(e); }
}
async function saveLimit() {
  const value = Number(historyLimit.value);
  if (!Number.isInteger(value) || value < 1 || value > 10000) { ui.notify('历史数量请输入 1 至 10000 之间的整数。', 'error'); historyLimit.value = store.settings.clipboardLimit; return; }
  if (value !== store.settings.clipboardLimit) await save({ clipboardLimit: value });
}
async function rescan() {
  rescanning.value = true;
  scanResult.value = '';
  try { const count = await command<number>('rescan_applications'); scanResult.value = `已更新 ${count} 个应用`; ui.notify(scanResult.value); }
  catch (e) { ui.fail(e); }
  finally { rescanning.value = false; await loadWarnings(); }
}
async function recordHotkey(event: KeyboardEvent) {
  event.preventDefault();
  event.stopPropagation();
  if (event.key === 'Escape') { recording.value = false; candidate.value = ''; return; }
  if (event.repeat || event.isComposing || ['Control', 'Alt', 'Shift', 'Meta'].includes(event.key)) return;
  if (!event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey) { ui.notify('请按住 Ctrl、Alt、Shift 或 Win，再按一个按键。', 'error'); return; }
  let key = event.code;
  if (/^Key[A-Z]$/.test(key)) key = key.slice(3);
  else if (/^Digit\d$/.test(key)) key = key.slice(5);
  else if (key === 'ArrowUp') key = 'Up';
  else if (key === 'ArrowDown') key = 'Down';
  else if (key === 'ArrowLeft') key = 'Left';
  else if (key === 'ArrowRight') key = 'Right';
  if (!key) { ui.notify('未识别该按键，请重试。', 'error'); return; }
  const modifiers = [event.ctrlKey ? 'Control' : '', event.altKey ? 'Alt' : '', event.shiftKey ? 'Shift' : '', event.metaKey ? 'Super' : ''].filter(Boolean);
  candidate.value = [...modifiers, key].join('+');
  recording.value = false;
  await save({ hotkey: candidate.value });
  candidate.value = '';
}
function keydown(event: KeyboardEvent) {
  if (recording.value) { void recordHotkey(event); return; }
  if (event.key === 'Escape') { event.preventDefault(); void hideWindow().catch(ui.fail); }
}
onMounted(() => { document.addEventListener('keydown', keydown, true); void loadWarnings(); });
onUnmounted(() => document.removeEventListener('keydown', keydown, true));
</script>

<template>
  <TitleBar title="设置" standalone />
  <div class="settings-body scroll-area"><DesktopNotice v-if="!desktop" /><div v-if="warnings.length" class="runtime-warning" role="status"><AlertCircle :size="17" /><div><p v-for="warning in warnings" :key="warning">{{ warning }}</p></div></div><div class="page-heading"><h1>按你的习惯工作</h1><p>简单的设置，让 XTools 更顺手。</p></div>
    <section class="settings-section"><h2>通用</h2><div class="setting-row"><div class="setting-icon"><Power :size="20" /></div><div class="setting-copy"><h3>开机自动启动</h3><p>登录 Windows 后，安静地在托盘中待命</p></div><button class="toggle" :class="{ on: store.settings.autostart }" role="switch" :aria-checked="store.settings.autostart" aria-label="开机自动启动" :disabled="disabled" @click="save({ autostart: !store.settings.autostart })"><span><Check v-if="store.settings.autostart" :size="10" /></span></button></div><div class="setting-row"><div class="setting-icon"><Keyboard :size="20" /></div><div class="setting-copy"><h3>呼出快捷键</h3><p>{{ recording ? '按下组合键，Esc 取消录制' : '再次按下同一快捷键可隐藏启动器' }}</p></div><button class="hotkey-recorder" :class="{ recording }" :disabled="disabled" @click="recording = !recording; candidate = ''">{{ recording ? '等待按键…' : hotkeyLabel(candidate || store.settings.hotkey) }}<Keyboard :size="14" /></button></div></section>
    <section class="settings-section"><h2>剪贴板</h2><div class="setting-row"><div class="setting-icon"><Clipboard :size="20" /></div><div class="setting-copy"><h3>最大历史数量</h3><p>只统计普通记录；收藏内容始终保留</p></div><div class="number-with-unit"><input v-model.number="historyLimit" type="number" min="1" max="10000" step="1" aria-label="最大剪贴板历史数量" :disabled="disabled" @change="saveLimit" /><span>条</span></div></div></section>
    <section class="settings-section"><h2>启动器</h2><div class="setting-row"><div class="setting-icon"><LayoutGrid :size="20" /></div><div class="setting-copy"><h3>显示最近使用</h3><p>显示最近通过 XTools 启动的 8 个应用</p></div><button class="toggle" :class="{ on: store.settings.showRecent }" role="switch" :aria-checked="store.settings.showRecent" aria-label="显示最近使用" :disabled="disabled" @click="save({ showRecent: !store.settings.showRecent })"><span><Check v-if="store.settings.showRecent" :size="10" /></span></button></div><div class="setting-row"><div class="setting-icon"><FolderSearch :size="20" /></div><div class="setting-copy"><h3>应用索引</h3><p>{{ scanResult || '扫描开始菜单、桌面和已安装应用' }}</p></div><button class="button button-small" :disabled="disabled || rescanning" @click="rescan"><LoaderCircle v-if="rescanning" :size="14" class="spinning" /><RefreshCw v-else :size="14" />{{ rescanning ? '正在扫描' : '重新扫描' }}</button></div></section>
    <section class="about-section"><BrandMark class="about-mark" /><div><h3>XTools <span>0.2.0</span></h3><p>专注日常效率，所有数据留在你的电脑。</p><span class="privacy-note"><ShieldCheck :size="13" />离线使用 · 无账号 · 无遥测</span></div></section>
  </div>
  <footer class="settings-footer"><span v-if="store.saving"><LoaderCircle :size="13" class="spinning" />正在应用设置…</span><span v-else><Check :size="13" />更改即时生效</span><span>Windows 10 / 11 x64</span></footer>
</template>
