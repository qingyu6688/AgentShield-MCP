<template>
  <a-card size="small">
    <a-space style="margin-bottom: 12px">
      <a-segmented v-model:value="format" :options="formatOptions" />
      <a-button type="primary" :loading="loading" @click="generate">生成</a-button>
      <a-button :disabled="!content" @click="download">下载</a-button>
    </a-space>

    <a-empty v-if="!content && !loading" description="点击「生成」从当前审计数据生成报告" />

    <iframe v-else-if="format === 'html'" :srcdoc="content" class="report-frame"></iframe>
    <pre v-else class="report-text">{{ content }}</pre>
  </a-card>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { message } from "ant-design-vue";
import { api } from "../api";

const format = ref("markdown");
const formatOptions = [
  { label: "Markdown", value: "markdown" },
  { label: "JSON", value: "json" },
  { label: "HTML", value: "html" },
];
const content = ref("");
const loading = ref(false);

async function generate() {
  loading.value = true;
  try {
    content.value = (await api.report(format.value)).content;
  } catch (e) {
    message.error((e as Error).message);
  } finally {
    loading.value = false;
  }
}

function download() {
  const ext = format.value === "markdown" ? "md" : format.value;
  const mime =
    format.value === "html" ? "text/html" : format.value === "json" ? "application/json" : "text/markdown";
  const blob = new Blob([content.value], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `agentshield-report.${ext}`;
  a.click();
  URL.revokeObjectURL(url);
}
</script>

<style scoped>
.report-text {
  background: #f6f7f9;
  border-radius: 6px;
  padding: 16px;
  max-height: calc(100vh - 220px);
  overflow: auto;
  font-size: 13px;
  line-height: 1.6;
}
.report-frame {
  width: 100%;
  height: calc(100vh - 220px);
  border: 1px solid #eef0f3;
  border-radius: 6px;
  background: #fff;
}
</style>
