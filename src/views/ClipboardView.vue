<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { ArrowLeft, Search, X, Star, Trash2, Clipboard, FileText, Folder, Copy, LoaderCircle, CornerDownLeft } from 'lucide-vue-next';
import TitleBar from '../components/TitleBar.vue';
import DesktopNotice from '../components/DesktopNotice.vue';
import ConfirmDialog from '../components/ConfirmDialog.vue';
import { asset, command, desktop, hideWindow } from '../lib/native';
import { useClipboardStore } from '../stores/clipboard';
import { useUiStore } from '../stores/ui';
import type { ClipboardItem } from '../types';

const emit = defineEmits<{ back: [] }>();
const store = useClipboardStore();
const ui = useUiStore();
const input = ref<HTMLInputElement>();
const selected = ref(0);
const favoritesOnly = ref(false);
const visible = computed(() => favoritesOnly.value ? store.items.filter((item) => item.isFavorite) : store.items);
const busy = ref(false);
const failedImages = ref(new Set<number>());
const deleting = ref<ClipboardItem>();
const clearing = ref(false);
const includeFavorites = ref(false);
let timer: ReturnType<typeof setTimeout> | undefined;
let unlisten: UnlistenFn | undefined;
let disposed = false;
watch(() => store.query, () => { store.invalidate(); selected.value = 0; if (timer) clearTimeout(timer); timer = setTimeout(() => { void store.load(); }, 70); }, { flush: 'sync' });
watch(visible, () => { selected.value = Math.max(0, Math.min(selected.value, visible.value.length - 1)); });
watch(selected, async () => { await nextTick(); document.querySelector('[data-selected="true"]')?.scrollIntoView({ block: 'nearest' }); });
function dateLabel(time: number) {
  const date = new Date(time < 100000000000 ? time * 1000 : time);
  const today = new Date();
  const prefix = date.toDateString() === today.toDateString() ? '今天' : date.toLocaleDateString('zh-CN', { month: 'numeric', day: 'numeric' });
  return `${prefix} ${date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })}`;
}
async function copy(item?: ClipboardItem) {
  if (!item || busy.value) return;
  busy.value = true;
  try { await command('copy_clipboard', { id: item.id }); await hideWindow(); }
  catch (e) { ui.fail(e); }
  finally { busy.value = false; }
}
async function favorite(item: ClipboardItem) {
  if (busy.value) return;
  busy.value = true;
  try { await command('favorite_clipboard', { id: item.id, favorite: !item.isFavorite }); await store.load(); }
  catch (e) { ui.fail(e); }
  finally { busy.value = false; }
}
async function deleteItem() {
  if (!deleting.value || busy.value) return;
  busy.value = true;
  try { await command('delete_clipboard', { id: deleting.value.id }); deleting.value = undefined; await store.load(); ui.notify('已删除这条记录'); }
  catch (e) { ui.fail(e); }
  finally { busy.value = false; }
}
async function clearItems() {
  busy.value = true;
  try { await command('clear_clipboard', { includeFavorites: includeFavorites.value }); clearing.value = false; await store.load(); ui.notify('剪贴板历史已清空'); }
  catch (e) { ui.fail(e); }
  finally { busy.value = false; }
}
function keydown(event: KeyboardEvent) {
  if (deleting.value || clearing.value || event.isComposing || event.ctrlKey || event.metaKey || event.altKey) return;
  if (event.key === 'Escape') { event.preventDefault(); void hideWindow().catch(ui.fail); }
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    event.preventDefault();
    if (visible.value.length) selected.value = (selected.value + (event.key === 'ArrowDown' ? 1 : -1) + visible.value.length) % visible.value.length;
  }
  if (event.key === 'Enter' && !(event.target instanceof HTMLButtonElement)) { event.preventDefault(); void copy(visible.value[selected.value]); }
}
onMounted(async () => {
  document.addEventListener('keydown', keydown);
  input.value?.focus();
  await store.load();
  if (desktop) {
    const listener = await listen('clipboard-changed', () => { void store.load(); }).catch((e) => { ui.fail(e); return undefined; });
    if (disposed) listener?.(); else unlisten = listener;
  }
});
onUnmounted(() => { disposed = true; document.removeEventListener('keydown', keydown); if (timer) clearTimeout(timer); unlisten?.(); });
</script>

<template>
  <TitleBar title="剪贴板" />
  <div class="clipboard-search"><button class="icon-button back-button" aria-label="返回启动器" @click="emit('back')"><ArrowLeft :size="21" /></button><Search :size="21" class="muted" /><input ref="input" v-model="store.query" placeholder="搜索剪贴板历史" aria-label="搜索剪贴板历史" autocomplete="off" /><LoaderCircle v-if="store.loading" class="spinning muted" :size="18" /><button v-else-if="store.query" class="icon-button" aria-label="清空搜索" @click="store.query = ''"><X :size="18" /></button></div>
  <div class="clipboard-toolbar"><div class="segmented-control"><button :class="{ active: !favoritesOnly }" @click="favoritesOnly = false">全部记录</button><button :class="{ active: favoritesOnly }" @click="favoritesOnly = true"><Star :size="13" /> 收藏</button></div><span class="small muted">{{ visible.length }} 条</span><button class="text-button subtle-danger" :disabled="!desktop || busy || !store.items.length" @click="clearing = true; includeFavorites = false"><Trash2 :size="14" />清空历史</button></div>
  <div class="clipboard-body scroll-area"><DesktopNotice v-if="!desktop" /><div v-if="store.error" class="inline-error" role="alert">{{ store.error }}<button class="text-button" @click="store.load">重试</button></div><div v-if="!visible.length && !store.loading" class="empty-state clipboard-empty"><Clipboard :size="37" :stroke-width="1.3" /><h3>{{ favoritesOnly ? '收藏你的常用内容' : store.query ? '没有找到这条记录' : '你的剪贴板，从这里开始' }}</h3><p>{{ favoritesOnly ? '点击记录旁的星标，随时找回重要内容。' : store.query ? '试试其他关键词，或查看全部记录。' : '复制文字、图片或文件，历史会自动保存在本机。' }}</p></div><div class="clipboard-list"><article v-for="(item, index) in visible" :key="item.id" class="clipboard-row" :class="{ selected: selected === index, 'image-row': item.itemType === 'image' }" :data-selected="selected === index" @mouseenter="selected = index"><button class="clipboard-content" :disabled="busy" :aria-label="`复制${item.preview}`" @click="copy(item)"><div class="clipboard-type-icon" :class="{ 'image-preview': item.itemType === 'image' }"><span v-if="item.itemType === 'image' && failedImages.has(item.id)" class="image-failure">图片已失效<br />请重新复制</span><img v-else-if="item.itemType === 'image' && item.dataPath" @error="failedImages.add(item.id)" :src="asset(item.dataPath)" alt="剪贴板图片预览" loading="lazy" /><Folder v-else-if="item.itemType === 'files'" :size="21" :stroke-width="1.5" /><FileText v-else :size="21" :stroke-width="1.5" /></div><div class="clipboard-text"><p :title="item.preview">{{ item.preview || (item.itemType === 'image' ? '图片' : '空文本') }}</p><span>{{ item.itemType === 'image' ? '图片' : item.itemType === 'files' ? '文件 / 文件夹' : '文本' }}<span class="meta-dot">·</span>{{ dateLabel(item.createdAt) }}</span></div></button><div class="clipboard-actions"><button class="icon-button favorite-button" :class="{ favorited: item.isFavorite }" :aria-label="item.isFavorite ? '取消收藏' : '收藏'" :title="item.isFavorite ? '取消收藏' : '收藏'" :disabled="busy" @click="favorite(item)"><Star :size="17" :fill="item.isFavorite ? 'currentColor' : 'none'" /></button><button class="icon-button row-action" aria-label="复制并隐藏 XTools" title="复制并隐藏 XTools" :disabled="busy" @click="copy(item)"><Copy :size="16" /></button><button class="icon-button row-action delete-button" aria-label="删除记录" title="删除记录" :disabled="busy" @click="deleting = item"><Trash2 :size="16" /></button></div></article></div></div>
  <footer class="launcher-footer"><div class="keyboard-hints"><span><kbd>↑ ↓</kbd> 选择</span><span><kbd>Enter</kbd> 复制并隐藏</span><span><kbd>Esc</kbd> 隐藏</span></div><span class="footer-quiet"><CornerDownLeft :size="13" />复制后自行粘贴</span></footer>
  <ConfirmDialog :open="!!deleting" title="删除这条记录？" message="这条剪贴板历史将从 XTools 中永久删除。" confirm-text="删除记录" danger :busy="busy" @cancel="deleting = undefined" @confirm="deleteItem" />
  <ConfirmDialog :open="clearing" title="清空剪贴板历史？" :message="includeFavorites ? '所有记录和收藏将永久删除，此操作无法撤销。' : '普通历史将永久删除，收藏的内容会保留。'" confirm-text="清空历史" danger :busy="busy" @cancel="clearing = false" @confirm="clearItems"><label class="checkbox-label dialog-checkbox"><input v-model="includeFavorites" type="checkbox" :disabled="busy" />同时删除收藏记录</label></ConfirmDialog>
</template>
