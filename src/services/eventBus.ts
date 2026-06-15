type Handler = (...args: unknown[]) => void

const handlers: Record<string, Set<Handler>> = {}

export const eventBus = {
  on(event: string, handler: Handler): () => void {
    if (!handlers[event]) handlers[event] = new Set()
    handlers[event].add(handler)
    return () => handlers[event]?.delete(handler)
  },

  once(event: string, handler: Handler): () => void {
    const wrapper: Handler = (...args) => {
      handler(...args)
      off()
    }
    const off = eventBus.on(event, wrapper)
    return off
  },

  emit(event: string, ...args: unknown[]): void {
    const set = handlers[event]
    if (set) set.forEach(fn => fn(...args))
  },

  off(event: string, handler: Handler): void {
    handlers[event]?.delete(handler)
  },

  clear(event?: string): void {
    if (event) delete handlers[event]
    else Object.keys(handlers).forEach(k => delete handlers[k])
  },

  listeners(event: string): number {
    return handlers[event]?.size ?? 0
  }
}
