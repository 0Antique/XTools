<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch, nextTick } from 'vue';
import { AlertTriangle, LoaderCircle } from 'lucide-vue-next';
const props = defineProps<{ open: boolean; title: string; message: string; confirmText?: string; busy?: boolean; danger?: boolean }>();
const emit = defineEmits<{ cancel: []; confirm: [] }>();
const cancelButton = ref<HTMLButtonElement>();
let previousFocus: HTMLElement | null = null;
watch(() => props.open, async (open) => {
  if (open) { previousFocus = document.activeElement as HTMLElement | null; await nextTick(); cancelButton.value?.focus(); }
  else previousFocus?.focus();
}, { immediate: true });
function keydown(event: KeyboardEvent) {
  if (!props.open) return;
  if (event.key === 'Escape') { event.preventDefault(); event.stopImmediatePropagation(); if (!props.busy) emit('cancel'); }
  if (event.key === 'Tab') {
    const buttons = Array.from(document.querySelectorAll<HTMLButtonElement>('.confirm-dialog button:not(:disabled)'));
    if (buttons.length === 0) { event.preventDefault(); return; }
    const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const next = event.shiftKey ? (index <= 0 ? buttons.length - 1 : index - 1) : (index + 1) % buttons.length;
    event.preventDefault(); buttons[next]?.focus();
  }
}
onMounted(() => document.addEventListener('keydown', keydown, true));
onUnmounted(() => document.removeEventListener('keydown', keydown, true));
</script>
<template>
  <Teleport to="body"><div v-if="open" class="modal-backdrop" @mousedown.self="!busy && emit('cancel')"><section class="confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="confirm-title" aria-describedby="confirm-message"><div class="dialog-icon" :class="{ danger }"><AlertTriangle :size="23" /></div><h2 id="confirm-title">{{ title }}</h2><p id="confirm-message">{{ message }}</p><slot /><div class="dialog-actions"><button ref="cancelButton" class="button" :disabled="busy" @click="emit('cancel')">取消</button><button class="button" :class="danger ? 'button-danger' : 'button-primary'" :disabled="busy" @click="emit('confirm')"><LoaderCircle v-if="busy" class="spinning" :size="15" />{{ confirmText || '确认' }}</button></div></section></div></Teleport>
</template>
