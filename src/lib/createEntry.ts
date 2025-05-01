import { invoke } from '@tauri-apps/api/core';

export async function createNewEntry(): Promise<number> {
  return invoke<number>('create_entry');
}