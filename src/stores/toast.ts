import { defineStore } from 'pinia'

interface Toast {
  id: number
  message: string
  type: 'success' | 'error' | 'info'
}

export const useToastStore = defineStore('toast', {
  state: () => ({ toasts: [] as Toast[], nextId: 0 }),
  actions: {
    add(type: 'success' | 'error' | 'info', message: string) {
      const id = this.nextId++
      this.toasts.push({ id, message, type })
      setTimeout(() => { this.toasts = this.toasts.filter(t => t.id !== id) }, 2500)
      return id
    },
    remove(id: number) {
      this.toasts = this.toasts.filter(t => t.id !== id)
    },
    success(message: string) { this.add('success', message) },
    error(message: string) { this.add('error', message) },
    info(message: string) { this.add('info', message) },
  }
})
