<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { AppWindow, Clipboard, Palette, FolderPen, Settings } from 'lucide-vue-next';
import { asset } from '../lib/native';
import type { SearchResult } from '../types';
const props = defineProps<{ item: SearchResult }>();
const failed = ref(false);
const src = computed(() => asset(props.item.iconPath));
const toolIcons = { clipboard: Clipboard, color: Palette, rename: FolderPen, settings: Settings };
const icon = computed(() => props.item.kind === 'tool' ? toolIcons[props.item.id as keyof typeof toolIcons] ?? AppWindow : AppWindow);
watch(src, () => { failed.value = false; });
</script>

<template><span class="app-icon" :class="item.kind === 'tool' ? `tool-icon-${item.id}` : ''"><img v-if="src && !failed" :src="src" alt="" @error="failed = true" /><component :is="icon" v-else :size="25" :stroke-width="1.6" /></span></template>
