<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { ContextQueue, type RenameContext } from '../lib/rename-context';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { Files, FolderPlus, Upload, Undo2, X, ArrowRight, FolderPen, LoaderCircle, AlertCircle, CheckCircle2, RotateCcw, Hash, ChevronDown } from 'lucide-vue-next';
import TitleBar from '../components/TitleBar.vue';
import DesktopNotice from '../components/DesktopNotice.vue';
import ConfirmDialog from '../components/ConfirmDialog.vue';
import { command, desktop, errorText, hideWindow } from '../lib/native';
import { useUiStore } from '../stores/ui';
import type { RenamePreview, RenameRules } from '../types';

const ui = useUiStore();
const defaults = (): RenameRules => ({ prefix: '', suffix: '', find: '', replace: '', deleteChars: '', caseMode: 'keep', numbering: false, numberStart: 1, numberStep: 1, numberWidth: 3, numberPosition: 'suffix', numberSeparator: '_' });
const rules = reactive<RenameRules>(defaults());
const paths = ref<string[]>([]);
const previews = ref<RenamePreview[]>([]);
const previewInputPaths = ref<string[]>([]);
const previewing = ref(false);
const error = ref('');
const busy = ref(false);
const dragging = ref(false);
const historyAvailable = ref(false);
const confirming = ref<'execute' | 'undo' | undefined>();
const changes = computed(() => previews.value.filter((row) => !row.error && row.oldName !== row.newName).length);
const errors = computed(() => previews.value.filter((row) => !!row.error).length);
const canExecute = computed(() => desktop && !busy.value && !previewing.value && !error.value && errors.value === 0 && previews.value.length === paths.value.length && changes.value > 0);
let previewTimer: ReturnType<typeof setTimeout> | undefined;
let previewRevision = 0;
let unlisten: UnlistenFn | undefined;
let disposed = false;
let contextListener: UnlistenFn | undefined;
const contextQueue = new ContextQueue();
function applyContext() {
  const context = contextQueue.take(busy.value);
  if (!context) return;
  confirming.value = undefined;
  if (context.status === "ready" && context.paths.length) {
    ++previewRevision;
    previews.value = []; previewInputPaths.value = [];
    Object.assign(rules, defaults());
    paths.value = [...context.paths];
    ui.notify(`已带入资源管理器选中的 ${context.paths.length} 个项目`);
  } else if (context.message) ui.notify(context.message, "error");
}
async function readContext() {
  try {
    const context = await command<RenameContext | null>("get_rename_context");
    if (!disposed && context && contextQueue.receive(context)) {
      await command("ack_rename_context", { requestId: context.requestId });
      applyContext();
    }
  } catch (e) { ui.fail(e); }
}

function addPaths(items: string[]) {
  if (busy.value) return;
  const existing = new Set(paths.value.map((path) => path.toLocaleLowerCase()));
  const unique = items.filter((path) => { const key = path.toLocaleLowerCase(); if (existing.has(key)) return false; existing.add(key); return true; });
  paths.value = [...paths.value, ...unique];
}
async function select(directory: boolean) {
  try {
    const selection = await command<string[]>('select_rename_paths', { directory });
    if (selection) addPaths(Array.isArray(selection) ? selection : [selection]);
  } catch (e) { ui.fail(e); }
}
function schedulePreview() {
  const revision = ++previewRevision;
  if (previewTimer) clearTimeout(previewTimer);
  error.value = '';
  previewing.value = paths.value.length > 0;
  if (!paths.value.length) { previews.value = []; return; }
  previewTimer = setTimeout(() => { void preview(revision); }, 60);
}
async function preview(revision: number) {
  const inputPaths = [...paths.value];
  try {
    const rows = await command<RenamePreview[]>('preview_rename', { paths: inputPaths, rules: { ...rules } });
    if (revision === previewRevision) { previews.value = rows; previewInputPaths.value = inputPaths; }
  } catch (e) { if (revision === previewRevision) { error.value = errorText(e); previews.value = []; } }
  finally { if (revision === previewRevision) previewing.value = false; }
}
function removePreviewRow(index: number) {
  const originalPath = previewInputPaths.value[index];
  if (originalPath !== undefined) paths.value = paths.value.filter((path) => path !== originalPath);
}
watch([paths, rules], schedulePreview, { deep: true, flush: 'sync' });
async function refreshHistory() {
  if (!desktop) return;
  try { historyAvailable.value = await command<boolean>('rename_history_available'); }
  catch (e) { ui.fail(e); }
}
async function execute() {
  if (!canExecute.value) { confirming.value = undefined; return; }
  busy.value = true;
  const updatedPaths = previews.value.map((row) => row.newPath);
  try {
    const count = await command<number>('execute_rename', { paths: [...paths.value], rules: { ...rules } });
    confirming.value = undefined;
    paths.value = updatedPaths;
    Object.assign(rules, defaults());
    await refreshHistory();
    ui.notify(`已重命名 ${count} 个项目，可撤销上一次操作`);
  } catch (e) { ui.fail(e); confirming.value = undefined; schedulePreview(); }
  finally { busy.value = false; applyContext(); }
}
async function undo() {
  busy.value = true;
  try { const count = await command<number>('undo_rename'); confirming.value = undefined; paths.value = []; await refreshHistory(); ui.notify(`已还原 ${count} 个项目的名称`); }
  catch (e) { ui.fail(e); confirming.value = undefined; }
  finally { busy.value = false; applyContext(); }
}
function keydown(event: KeyboardEvent) {
  if (event.key === 'Escape' && !confirming.value && !busy.value) { event.preventDefault(); void hideWindow().catch(ui.fail); }
}
onMounted(async () => {
  document.addEventListener('keydown', keydown);
  await refreshHistory();
  if (desktop) {
    const listenerContext = await listen("rename-context-changed", () => { void readContext(); });
    if (disposed) listenerContext(); else contextListener = listenerContext;
    await readContext();
    const listener = await getCurrentWindow().onDragDropEvent((event) => {
      if (event.payload.type === 'enter' || event.payload.type === 'over') dragging.value = true;
      else dragging.value = false;
      if (event.payload.type === 'drop') addPaths(event.payload.paths);
    }).catch((e) => { ui.fail(e); return undefined; });
    if (disposed) listener?.(); else unlisten = listener;
  }
});
onUnmounted(() => { disposed = true; document.removeEventListener('keydown', keydown); if (previewTimer) clearTimeout(previewTimer); ++previewRevision; unlisten?.(); contextListener?.(); });
</script>

<template>
  <TitleBar title="批量重命名" standalone />
  <div class="rename-body" :class="{ dragging }" @dragover.prevent @drop.prevent="!desktop && ui.notify('拖入文件需要在 XTools 桌面版中使用。', 'error')">
    <DesktopNotice v-if="!desktop" />
    <div class="rename-heading"><div><h1>批量重命名</h1></div><button class="button button-small" :disabled="!desktop || busy || !historyAvailable" @click="confirming = 'undo'"><Undo2 :size="15" />撤销上一次</button></div>
    <div class="rename-workspace"><aside class="rename-rules scroll-area"><div class="rule-heading"><h2>命名规则</h2><button class="icon-button" title="重置规则" aria-label="重置规则" :disabled="busy" @click="Object.assign(rules, defaults())"><RotateCcw :size="15" /></button></div><fieldset :disabled="busy || !desktop"><div class="rule-group"><h3><span class="rule-number">01</span>添加文本</h3><label class="field-label">前缀<input v-model="rules.prefix" type="text" placeholder="例如：旅行_" /></label><label class="field-label">后缀<input v-model="rules.suffix" type="text" placeholder="例如：_精选" /></label></div><div class="rule-group"><h3><span class="rule-number">02</span>查找与替换</h3><label class="field-label">查找<input v-model="rules.find" type="text" placeholder="原名称中的文字" /></label><label class="field-label">替换为<input v-model="rules.replace" type="text" placeholder="留空即删除匹配内容" /></label></div><div class="rule-group"><h3><span class="rule-number">03</span>字符与大小写</h3><label class="field-label">删除指定字符<input v-model="rules.deleteChars" type="text" placeholder="例如：空格、括号等" /></label><label class="field-label">大小写<select v-model="rules.caseMode"><option value="keep">保持原样</option><option value="lower">全部小写</option><option value="upper">全部大写</option><option value="title">单词首字母大写</option></select></label></div><div class="rule-group numbering-group"><label class="numbering-toggle"><span><span class="rule-number">04</span>自动编号</span><input v-model="rules.numbering" type="checkbox" /></label><div v-if="rules.numbering" class="numbering-fields"><div class="three-fields"><label class="field-label">起始值<input v-model.number="rules.numberStart" type="number" min="0" max="2147483647" step="1" /></label><label class="field-label">步长<input v-model.number="rules.numberStep" type="number" min="1" max="2147483647" step="1" /></label><label class="field-label">位数<input v-model.number="rules.numberWidth" type="number" min="1" max="12" step="1" /></label></div><div class="two-fields"><label class="field-label">位置<select v-model="rules.numberPosition"><option value="prefix">名称前</option><option value="suffix">名称后</option></select></label><label class="field-label">分隔符<input v-model="rules.numberSeparator" type="text" placeholder="_" /></label></div><p class="field-note"><Hash :size="12" />按照列表顺序依次编号</p></div></div></fieldset></aside>
    <section class="rename-preview-panel"><div class="preview-toolbar"><span><strong>{{ paths.length }}</strong> 个项目<span class="preview-quiet"> · 不递归</span></span><div class="preview-toolbar-actions"><button class="button button-small" :disabled="!desktop || busy" @click="select(false)"><Files :size="14" />添加文件</button><button class="button button-small" :disabled="!desktop || busy" @click="select(true)"><FolderPlus :size="14" />文件夹</button><button v-if="paths.length" class="icon-button" title="移除所有项目" aria-label="移除所有项目" :disabled="busy" @click="paths = []"><X :size="16" /></button></div></div>
      <div v-if="!paths.length" class="rename-dropzone"><div class="dropzone-icon"><Upload :size="29" :stroke-width="1.4" /></div><h2>把文件或文件夹拖到这里</h2><p>支持混合添加，只修改所选项目的名称</p><div class="dropzone-buttons"><button class="button" :disabled="!desktop || busy" @click="select(false)">选择文件</button><button class="button" :disabled="!desktop || busy" @click="select(true)">选择文件夹</button></div><span class="dropzone-note">文件扩展名会保留 · 子目录内容不受影响</span></div>
      <template v-else><div class="preview-table-head"><span>原名称</span><ArrowRight :size="13" /><span>新名称</span><span /></div><div class="preview-table scroll-area"><div v-if="error" class="inline-error" role="alert"><AlertCircle :size="16" />{{ error }}<button class="text-button" @click="schedulePreview">重试</button></div><template v-if="previews.length"><div v-for="(row, index) in previews" :key="row.oldPath" class="preview-row" :class="{ 'preview-error': row.error, 'preview-changed': row.oldName !== row.newName }"><span class="old-name" :title="row.oldPath">{{ row.oldName }}</span><ArrowRight :size="13" class="rename-arrow" /><div class="new-name"><span :title="row.newPath">{{ row.newName }}</span><span v-if="row.error" class="row-error"><AlertCircle :size="11" />{{ row.error }}</span></div><button class="icon-button" aria-label="从列表移除此项目" :disabled="busy" @click="removePreviewRow(index)"><X :size="13" /></button></div></template><div v-else-if="previewing" class="preview-loading"><LoaderCircle :size="20" class="spinning" />正在检查名称与冲突…</div><div v-else-if="!error" class="preview-loading">等待预览</div></div><div class="preview-status"><span v-if="previewing"><LoaderCircle :size="14" class="spinning" />正在更新预览…</span><span v-else-if="errors" class="error-text"><AlertCircle :size="14" />{{ errors }} 个项目有冲突，请调整规则</span><span v-else-if="changes"><CheckCircle2 :size="14" />{{ changes }} 个项目将使用新名称</span><span v-else>调整左侧规则，查看新名称</span></div></template>
    </section></div>
    <div v-if="dragging" class="drag-overlay"><Upload :size="35" /><h2>松开以添加项目</h2><p>文件与文件夹都可以</p><ChevronDown :size="25" /></div>
  </div>
  <footer class="rename-footer"><span><FolderPen :size="15" />实时检查冲突，支持撤销上一次操作</span><button class="button button-primary" :disabled="!canExecute" @click="confirming = 'execute'"><LoaderCircle v-if="busy" :size="15" class="spinning" /><span>{{ busy ? '正在处理…' : `执行重命名${changes ? ` · ${changes} 项` : ''}` }}</span><ArrowRight :size="15" /></button></footer>
  <ConfirmDialog :open="confirming === 'execute'" title="应用这些新名称？" :message="`即将重命名 ${changes} 个文件或文件夹。项目内容不会被修改，完成后可撤销上一次操作。`" confirm-text="执行重命名" :busy="busy" @cancel="confirming = undefined" @confirm="execute" />
  <ConfirmDialog :open="confirming === 'undo'" title="撤销上一次重命名？" message="还原最近一次成功操作的名称。如果原名称已被其他项目占用，操作将停止并提示冲突。" confirm-text="还原名称" :busy="busy" @cancel="confirming = undefined" @confirm="undo" />
</template>
