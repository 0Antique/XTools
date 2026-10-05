import { defineStore } from 'pinia';
import { ref } from 'vue';
import { command, desktop } from '../lib/native';
import type { Settings } from '../types';

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<Settings>({ hotkey: 'Alt+Space', autostart: true, clipboardLimit: 100, showRecent: true });
  const loaded = ref(false);
  const saving = ref(false);
  async function load() {
    if (!desktop) return;
    settings.value = await command<Settings>('get_settings');
    loaded.value = true;
  }
  async function save(patch: Partial<Settings>) {
    if (saving.value) return;
    saving.value = true;
    try {
      settings.value = await command<Settings>('save_settings', { settings: { ...settings.value, ...patch } });
    } finally { saving.value = false; }
  }
  return { settings, loaded, saving, load, save };
});
