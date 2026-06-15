import { invoke } from '@tauri-apps/api/core'
import { eventBus } from './eventBus'

export async function ipc<T = unknown>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  try {
    return await invoke<T>(command, args)
  } catch (err) {
    console.error(`[IPC Error] ${command}:`, err)
    eventBus.emit('ipc:error', { command, error: err })
    throw err
  }
}
