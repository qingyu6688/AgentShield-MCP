<template>
  <div ref="el" :style="{ width: '100%', height: height }"></div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from "vue";
import * as echarts from "echarts";

const props = defineProps<{ option: echarts.EChartsOption; height?: string }>();
const height = props.height ?? "320px";

const el = ref<HTMLDivElement>();
let chart: echarts.ECharts | null = null;

function render() {
  if (chart && props.option) {
    chart.setOption(props.option, true);
  }
}
function resize() {
  chart?.resize();
}

onMounted(() => {
  chart = echarts.init(el.value!);
  render();
  window.addEventListener("resize", resize);
});

watch(
  () => props.option,
  () => render(),
  { deep: true },
);

onBeforeUnmount(() => {
  window.removeEventListener("resize", resize);
  chart?.dispose();
});
</script>
