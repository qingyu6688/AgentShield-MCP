<template>
  <div ref="el" :style="{ width: '100%', height: height }"></div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from "vue";
import { PieChart } from "echarts/charts";
import { LegendComponent, TooltipComponent } from "echarts/components";
import { init, use, type ECharts, type EChartsOption } from "echarts/core";
import { CanvasRenderer } from "echarts/renderers";

use([CanvasRenderer, LegendComponent, PieChart, TooltipComponent]);

const props = defineProps<{ option: EChartsOption; height?: string }>();
const height = props.height ?? "320px";

const el = ref<HTMLDivElement>();
let chart: ECharts | null = null;

function render() {
  if (chart && props.option) {
    chart.setOption(props.option, true);
  }
}
function resize() {
  chart?.resize();
}

onMounted(() => {
  chart = init(el.value!);
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
