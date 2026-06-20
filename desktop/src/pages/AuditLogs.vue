<template>
  <a-card size="small">
    <a-space style="margin-bottom: 12px" wrap>
      <a-select
        v-model:value="filters.level"
        placeholder="风险等级"
        style="width: 130px"
        allow-clear
        :options="levelOptions"
      />
      <a-input v-model:value="filters.server" placeholder="server 名" style="width: 150px" allow-clear />
      <a-input v-model:value="filters.since" placeholder="起始 YYYY-MM-DD" style="width: 170px" allow-clear />
      <a-input v-model:value="filters.until" placeholder="结束 YYYY-MM-DD" style="width: 170px" allow-clear />
      <a-input-number v-model:value="filters.limit" :min="1" :max="1000" style="width: 110px" />
      <a-button type="primary" @click="load">查询</a-button>
      <a-button @click="reset">重置</a-button>
    </a-space>

    <a-table
      :data-source="events"
      :columns="columns"
      :loading="loading"
      :pagination="{ pageSize: 20, showSizeChanger: false }"
      row-key="id"
      size="small"
    >
      <template #bodyCell="{ column, record }">
        <template v-if="column.key === 'level'">
          <a-tag :color="levelColor(record.risk_level)">{{ levelText(record.risk_level) }} {{ record.risk_score }}</a-tag>
        </template>
        <template v-else-if="column.key === 'decision'">
          <a-tag :color="decisionColor(record.decision)">{{ decisionText(record.decision) }}</a-tag>
        </template>
        <template v-else-if="column.key === 'created_at'">{{ shortTime(record.created_at) }}</template>
        <template v-else-if="column.key === 'detail'">
          <a @click="openDetail(record)">详情</a>
        </template>
      </template>
    </a-table>

    <a-drawer v-model:open="detailOpen" title="事件详情" width="540">
      <template v-if="current">
        <a-descriptions :column="1" size="small" bordered>
          <a-descriptions-item label="时间">{{ current.created_at }}</a-descriptions-item>
          <a-descriptions-item label="来源">{{ current.client_name }}</a-descriptions-item>
          <a-descriptions-item label="Server">{{ current.server_name }}</a-descriptions-item>
          <a-descriptions-item label="工具">{{ current.tool_name }}</a-descriptions-item>
          <a-descriptions-item label="类型">{{ current.event_type }}</a-descriptions-item>
          <a-descriptions-item label="目标">{{ current.target ?? "-" }}</a-descriptions-item>
          <a-descriptions-item label="风险">
            <a-tag :color="levelColor(current.risk_level)">{{ levelText(current.risk_level) }} {{ current.risk_score }}/100</a-tag>
          </a-descriptions-item>
          <a-descriptions-item label="决策">
            <a-tag :color="decisionColor(current.decision)">{{ decisionText(current.decision) }}</a-tag>
          </a-descriptions-item>
          <a-descriptions-item label="原因">{{ current.reason }}</a-descriptions-item>
        </a-descriptions>
        <div class="block-title">参数（已脱敏）</div>
        <pre class="json">{{ pretty(current.arguments_json) }}</pre>
        <div class="block-title">结果</div>
        <pre class="json">{{ pretty(current.result_json) }}</pre>
      </template>
    </a-drawer>
  </a-card>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from "vue";
import { message } from "ant-design-vue";
import { api, type AuditEvent, type EventQuery } from "../api";
import { levelColor, levelText, decisionColor, decisionText, shortTime } from "../utils/format";

const loading = ref(false);
const events = ref<AuditEvent[]>([]);
const detailOpen = ref(false);
const current = ref<AuditEvent>();

const filters = reactive<EventQuery>({ level: undefined, server: "", since: "", until: "", limit: 100 });

const levelOptions = [
  { value: "low", label: "低" },
  { value: "medium", label: "中" },
  { value: "high", label: "高" },
  { value: "critical", label: "严重" },
];

const columns = [
  { title: "时间", key: "created_at", dataIndex: "created_at", width: 140 },
  { title: "来源", dataIndex: "client_name", width: 110 },
  { title: "Server", dataIndex: "server_name", width: 110 },
  { title: "类型", dataIndex: "event_type", width: 110 },
  { title: "目标", dataIndex: "target", ellipsis: true },
  { title: "风险", key: "level", width: 110 },
  { title: "决策", key: "decision", width: 80 },
  { title: "", key: "detail", width: 60 },
];

function pretty(v: unknown): string {
  if (v === null || v === undefined) return "（无）";
  return JSON.stringify(v, null, 2);
}

function openDetail(record: AuditEvent) {
  current.value = record;
  detailOpen.value = true;
}

async function load() {
  loading.value = true;
  try {
    events.value = await api.events({ ...filters });
  } catch (e) {
    message.error((e as Error).message);
  } finally {
    loading.value = false;
  }
}

function reset() {
  filters.level = undefined;
  filters.server = "";
  filters.since = "";
  filters.until = "";
  filters.limit = 100;
  load();
}

onMounted(load);
</script>

<style scoped>
.block-title {
  margin: 16px 0 6px;
  font-weight: 600;
}
.json {
  background: #f6f7f9;
  border-radius: 6px;
  padding: 10px;
  max-height: 220px;
  overflow: auto;
  font-size: 12px;
}
</style>
