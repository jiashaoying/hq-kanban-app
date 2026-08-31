<script setup lang="ts">
import { ref, onMounted } from 'vue'
import type { WebviewWindow } from '@tauri-apps/api/webviewWindow'

defineProps<{
  lastUpdateTime: string
  loading: boolean
}>()

const emit = defineEmits<{ refresh: [] }>()

const isDesktop = ref(false)
const appWindow = ref<WebviewWindow | null>(null)

onMounted(async () => {
  const ua = navigator.userAgent.toLowerCase()
  const isMobile = /iphone|ipad|ipod|android/.test(ua)
  isDesktop.value = !isMobile && window.innerWidth >= 768

  if (isDesktop.value) {
    const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow')
    appWindow.value = getCurrentWebviewWindow()
  }
})

async function startDrag(event: MouseEvent) {
  if (!appWindow.value) return
  if (event.detail === 2) return
  if (event.button !== 0) return
  await appWindow.value.startDragging()
}

async function minimize() {
  if (!appWindow.value) return
  await appWindow.value.minimize()
}

async function close() {
  if (!appWindow.value) return
  await appWindow.value.close()
}
</script>

<template>
  <div
    class="flex items-center justify-between px-4 h-10 bg-gray-900 border-b border-gray-700 select-none"
    @mousedown="isDesktop ? startDrag($event) : undefined"
  >
    <!-- 左侧：标题 -->
    <div class="flex items-center gap-2">
      <span class="text-sm font-semibold text-gray-200">大盘行情看板</span>
      <!-- 自动刷新脉动指示器 -->
      <span v-if="loading" class="w-2 h-2 rounded-full bg-blue-500 animate-pulse"></span>
    </div>

    <!-- 右侧：操作区 -->
    <div class="flex items-center gap-3">
      <!-- 最后更新时间 -->
      <span class="text-xs text-gray-500">{{ lastUpdateTime }}</span>

      <!-- 刷新按钮 -->
      <button
        @mousedown.stop
        @click="emit('refresh')"
        class="text-xs text-gray-400 hover:text-gray-200 transition-colors px-2 py-1 rounded hover:bg-gray-700"
      >
        刷新
      </button>

      <!-- 窗口控制按钮（仅桌面） -->
      <template v-if="isDesktop">
        <button @mousedown.stop @click="minimize" class="text-gray-400 hover:text-gray-200 px-2 py-1 hover:bg-gray-700 rounded">
          <svg class="w-3 h-3" viewBox="0 0 12 12" fill="currentColor"><rect x="1" y="5" width="10" height="2" /></svg>
        </button>
        <button @mousedown.stop @click="close" class="text-gray-400 hover:text-red-400 px-2 py-1 hover:bg-gray-700 rounded">
          <svg class="w-3 h-3" viewBox="0 0 12 12"><path d="M2 2l8 8M10 2l-8 8" stroke="currentColor" stroke-width="1.5" fill="none"/></svg>
        </button>
      </template>
    </div>
  </div>
</template>
