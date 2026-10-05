import { defineStore } from 'pinia';
import { ref } from 'vue';
import { command, desktop, errorText } from '../lib/native';
import type { ClipboardItem } from '../types';

export const useClipboardStore = defineStore('clipboard', () => {
  const query = ref('');
  const items = ref<ClipboardItem[]>([]);
  const loading = ref(false);
  const error = ref('');
  let request = 0;
  function invalidate() {
    request += 1;
    items.value = [];
    loading.value = desktop;
    error.value = '';
  }
  async function load() {
    const current = ++request;
    error.value = '';
    if (!desktop) return;
    loading.value = true;
    try {
      const rows = await command<ClipboardItem[]>('list_clipboard', { query: query.value });
      if (current === request) items.value = rows;
    } catch (e) { if (current === request) error.value = errorText(e); }
    finally { if (current === request) loading.value = false; }
  }
  return { query, items, loading, error, load, invalidate };
});
