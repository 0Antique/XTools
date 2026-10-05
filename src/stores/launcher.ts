import { defineStore } from 'pinia';
import { ref } from 'vue';
import { command, desktop, errorText } from '../lib/native';
import type { SearchResult } from '../types';

export const useLauncherStore = defineStore('launcher', () => {
  const query = ref('');
  const results = ref<SearchResult[]>([]);
  const selected = ref(0);
  const loading = ref(false);
  const error = ref('');
  let request = 0;
  function invalidate() {
    request += 1;
    results.value = [];
    selected.value = 0;
    loading.value = desktop;
    error.value = '';
  }
  async function search() {
    const current = ++request;
    error.value = '';
    selected.value = 0;
    if (!desktop) return;
    loading.value = true;
    try {
      const items = await command<SearchResult[]>('search', { query: query.value });
      if (current === request) results.value = items;
    } catch (e) {
      if (current === request) { error.value = errorText(e); results.value = []; }
    } finally { if (current === request) loading.value = false; }
  }
  return { query, results, selected, loading, error, search, invalidate };
});
