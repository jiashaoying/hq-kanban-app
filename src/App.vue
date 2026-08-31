<script setup lang="ts">
import TitleBar from './components/TitleBar.vue'
import MarketSection from './components/MarketSection.vue'
import { useMarketData } from './composables/useMarketData'

const { marketGroups, loading, lastUpdateTime, refresh } = useMarketData()
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
      <MarketSection
        v-for="group in marketGroups"
        :key="group.key"
        :group="group"
      />

      <!-- 空状态 -->
      <div v-if="marketGroups.every(g => g.indices.length === 0) && !loading" class="flex items-center justify-center h-full">
        <div class="text-center text-gray-500">
          <div class="text-lg mb-2">暂无数据</div>
          <div class="text-sm">正在获取行情数据...</div>
        </div>
      </div>
    </div>
  </div>
</template>
