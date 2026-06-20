<template>
  <a-card size="small">
    <template #title>
      <a-space>
        <span>实时事件</span>
        <a-badge :status="polling ? 'processing' : 'default'" :text="polling ? '自动刷新中（1.5s）' : '已暂停'" />
      </a-space>
    </template>
    <template #extra>
      <a-switch v-model:checked="polling" checked-children="开" un-checked-children="停" @change="togglePoll" />
    </template>

    <a-table
      :data-source="events"
      :columns="columns"
      :pagination="false"
      row-key="id"
      size="small"
      :scroll="{ y: 'calc(100vh - 260px)' }"
    >
      <template #bodyCell="{ column, record }">
        <template v-if="column.key === 'level'">
          <a-tag :color="levelColor(record.risk_level)">{{ levelText(record.risk_level) }} {{ record.risk_score }}</a-tag>
        </template>
        <template v-else-if="column.key === 'decision'">
          <a-tag :color="decisionColor(record.decision)">{{ decisionText(record.decision) }}</a-tag>
        </template>
        <template v-else-if="column.key === 'created_at'">{{ shortTime(record.created_at) }}</template>
      </template>
    </a-table>
  </a-card>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from "vue";
import { api, type AuditEvent } from "../api";
import { levelColor, levelText, decisionColor, decisionText, shortTime } from "../utils/format";

const events = ref<AuditEvent[]>([]);
const polling = ref(true);
let timer: number | undefined;

const columns = [
  { title: "时间", key: "created_at", dataIndex: "created_at", width: 140 },
  { title: "来源", dataIndex: "client_name", width: 110 },
  { title: "Server", dataIndex: "server_name", width: 110 },
  { title: "类型", dataIndex: "event_type", width: 110 },
  { title: "目标", dataIndex: "target", ellipsis: true },
  { title: "风险", key: "level", width: 110 },
  { title: "决策", key: "decision", width: 80 },
];

async function tick() {
  try {
    events.value = await api.events({ limit: 100 });
  } catch {
    // 实时流静默失败，等下次轮询
  }
}

function start() {
  stop();
  timer = window.setInterval(tick, 1500);
}
function stop() {
  if (timer) window.clearInterval(timer);
  timer = undefined;
}
function togglePoll(on: boolean) {
  if (on) start();
  else stop();
}

onMounted(() => {
  tick();
  start();
});
onBeforeUnmount(stop);
</script>
