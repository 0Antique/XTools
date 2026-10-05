import { defineStore } from 'pinia';
import { ref } from 'vue';
import { errorText } from '../lib/native';

export const useUiStore = defineStore('ui', () => {
  const notice = ref('');
  const noticeKind = ref<'success' | 'error'>('success');
  let timer: ReturnType<typeof setTimeout> | undefined;
  function notify(message: string, kind: 'success' | 'error' = 'success') {
    if (timer) clearTimeout(timer);
    notice.value = message;
    noticeKind.value = kind;
    timer = setTimeout(() => { notice.value = ''; }, kind === 'error' ? 8000 : 4000);
  }
  function fail(error: unknown) { notify(errorText(error), 'error'); }
  return { notice, noticeKind, notify, fail };
});
