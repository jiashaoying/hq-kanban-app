<script setup lang="ts">
import { ref, computed, defineAsyncComponent } from 'vue'
import TitleBar from './components/TitleBar.vue'
import MarketSection from './components/MarketSection.vue'
import { useMarketData } from './composables/useMarketData'

// 详情页异步加载，lightweight-charts 拆入独立 chunk，首屏看板零体积增量
const IndexDetail = defineAsyncComponent(() => import('./components/IndexDetail.vue'))

const { indices, marketGroups, loading, lastUpdateTime, refresh } = useMarketData()

// 视图切换：null = 看板，否则显示对应指数详情
const selectedCode = ref<string | null>(null)
const selectedIndex = computed(() => indices.value.find(i => i.code === selectedCode.value) ?? null)
</script>

<template>
  <div class="h-screen flex flex-col bg-gray-900 text-white overflow-hidden"
       style="padding-top: env(safe-area-inset-top); padding-bottom: env(safe-area-inset-bottom);">
    <!-- 自定义标题栏 -->
    <TitleBar
      :last-update-time="lastUpdateTime"
      :loading="loading"
      @refresh="refresh"
    />

    <!-- 主内容区 -->
    <div class="flex-1 overflow-y-auto p-4">
      <!-- 行情看板 -->
      <template v-if="!selectedCode">
        <MarketSection
          v-for="group in marketGroups"
          :key="group.key"
          :group="group"
          @select="selectedCode = $event.code"
        />

        <!-- 空状态 -->
        <div v-if="marketGroups.every(g => g.indices.length === 0) && !loading" class="flex items-center justify-center h-full">
          <div class="text-center text-gray-500">
            <div class="text-lg mb-2">暂无数据</div>
            <div class="text-sm">正在获取行情数据...</div>
          </div>
        </div>
      </template>

      <!-- 指数详情 -->
      <IndexDetail
        v-else-if="selectedIndex"
        :data="selectedIndex"
        @back="selectedCode = null"
      />

      <!-- 轮询数据未到/指数暂缺时的占位（含返回按钮：iOS 无 Esc，避免被困在占位页） -->
      <div v-else class="flex flex-col items-center justify-center h-full gap-4">
        <div class="text-center text-gray-500 text-sm">正在加载指数数据...</div>
        <button
          class="w-8 h-8 flex items-center justify-center rounded-md border border-gray-700 text-gray-300 hover:text-white hover:bg-gray-800 transition-colors"
          title="返回看板"
          @click="selectedCode = null"
        >
          ←
        </button>
      </div>
    </div>
  </div>
</template>
