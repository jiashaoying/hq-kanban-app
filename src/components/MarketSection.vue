<script setup lang="ts">
import type { MarketGroup } from '../types/market'
import IndexCard from './IndexCard.vue'

defineProps<{ group: MarketGroup }>()
</script>

<template>
  <div class="mb-6">
    <div class="flex items-center gap-2 mb-3">
      <h2 class="text-base font-semibold text-gray-300">{{ group.label }}</h2>
      <div class="flex-1 h-px bg-gray-700"></div>
    </div>
    <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
      <IndexCard v-for="index in group.indices" :key="index.code" :data="index" />
    </div>
  </div>
</template>
