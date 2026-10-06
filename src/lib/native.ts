import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core';

export const desktop = isTauri();

export function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  if (!desktop) return Promise.reject(new Error('请在 XTools 桌面应用中使用此功能。'));
  return invoke<T>(name, args);
}

export function asset(path?: string): string | undefined {
  if (!desktop || !path) return undefined;
  return convertFileSrc(path);
}

export function errorText(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  return '操作未完成，请重试。';
}

export async function hideWindow(): Promise<void> {
  if (desktop) await command('hide_current_window');
}

export function hotkeyLabel(hotkey: string): string {
  return hotkey.replace(/CommandOrControl|Control/gi, 'Ctrl').replace(/Super|Meta/gi, 'Win').replace(/\+/g, ' + ');
}
