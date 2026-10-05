<script setup lang="ts">
import { X } from 'lucide-vue-next';
import BrandMark from './BrandMark.vue';
import { hideWindow } from '../lib/native';
import { useUiStore } from '../stores/ui';
defineProps<{ title?: string; standalone?: boolean }>();
const ui = useUiStore();
</script>

<template>
  <header class="titlebar" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region><BrandMark class="brand-symbol" /> <span data-tauri-drag-region>XTools</span><span v-if="title" class="title-divider">/</span><span v-if="title" class="window-title" data-tauri-drag-region>{{ title }}</span></div>
    <div class="titlebar-right"><span class="local-label"><span class="status-dot" /> 完全本地</span><button class="icon-button window-close" :aria-label="standalone ? '关闭窗口' : '隐藏 XTools'" @click="hideWindow().catch(ui.fail)"><X :size="17" /></button></div>
  </header>
</template>
