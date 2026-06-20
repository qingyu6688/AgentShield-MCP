<template>
  <a-card size="small" :body-style="{ padding: '16px 18px' }">
    <div class="stat-title">{{ title }}</div>
    <div class="stat-value" :style="{ color: color }">{{ value }}</div>
  </a-card>
</template>

<script setup lang="ts">
defineProps<{ title: string; value: number | string; color?: string }>();
</script>

<style scoped>
.stat-title {
  color: #8a8f99;
  font-size: 13px;
}
.stat-value {
  font-size: 30px;
  font-weight: 700;
  line-height: 1.3;
  margin-top: 4px;
}
</style>
