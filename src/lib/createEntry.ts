import { invoke } from '@tauri-apps/api/core';

export async function createNewEntry(): Promise<number> {
  // Provide default values for title and body
  return invoke<number>('create_entry', { request: { title: '', body: '' } });
}