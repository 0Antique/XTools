<script setup lang="ts">
import { X, Pin, Settings } from 'lucide-vue-next';
import { onMounted, ref } from 'vue';
import BrandMark from './BrandMark.vue';
import { command, desktop, hideWindow } from '../lib/native';
import { useUiStore } from '../stores/ui';
defineProps<{ title?: string; standalone?: boolean }>();
const ui = useUiStore();
const pinned = ref(false);
const updating = ref(false);
onMounted(() => { if (desktop) void command<boolean>('get_window_pin').then(value => { pinned.value = value; }).catch(ui.fail); });
async function togglePin() {
  if (updating.value) return;
  updating.value = true;
  try { pinned.value = await command<boolean>('set_window_pin', { pinned: !pinned.value }); }
  catch (e) { ui.fail(e); }
  finally { updating.value = false; }
}
async function openSettings() {
  try { await command('open_tool', { id: 'settings' }); }
  catch (e) { ui.fail(e); }
}
</script>

<template>
  <header class="titlebar" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region><BrandMark class="brand-symbol" /> <span data-tauri-drag-region>XTools</span><span v-if="title" class="title-divider">/</span><span v-if="title" class="window-title" data-tauri-drag-region>{{ title }}</span></div>
    <div class="titlebar-right"><button v-if="!standalone" class="titlebar-settings" aria-label="设置" title="设置" :disabled="!desktop" @click="openSettings"><Settings :size="15" /><span>设置</span></button><button class="icon-button pin-button" :class="{ pinned }" :aria-pressed="pinned" :aria-label="pinned ? '取消悬浮' : '开启悬浮'" :title="pinned ? '取消悬浮' : '开启悬浮'" :disabled="!desktop || updating" @click="togglePin"><Pin :size="17" /></button><button class="icon-button window-close" :aria-label="standalone ? '关闭窗口' : '隐藏 XTools'" @click="hideWindow().catch(ui.fail)"><X :size="17" /></button></div>
  </header>
</template>
