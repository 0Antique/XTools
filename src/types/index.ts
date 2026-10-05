export type ToolId = 'clipboard' | 'color' | 'rename' | 'settings';

export interface SearchResult {
  id: string;
  name: string;
  kind: 'application' | 'tool';
  iconPath?: string;
  subtitle?: string;
}

export interface Settings {
  hotkey: string;
  autostart: boolean;
  clipboardLimit: number;
  showRecent: boolean;
}

export interface ClipboardItem {
  id: number;
  itemType: 'text' | 'image' | 'files';
  textContent?: string;
  dataPath?: string;
  preview: string;
  createdAt: number;
  isFavorite: boolean;
}

export interface RenameRules {
  prefix: string;
  suffix: string;
  find: string;
  replace: string;
  deleteChars: string;
  caseMode: 'keep' | 'lower' | 'upper' | 'title';
  numbering: boolean;
  numberStart: number;
  numberStep: number;
  numberWidth: number;
  numberPosition: 'prefix' | 'suffix';
  numberSeparator: string;
}

export interface RenamePreview {
  oldPath: string;
  newPath: string;
  oldName: string;
  newName: string;
  error?: string;
}
