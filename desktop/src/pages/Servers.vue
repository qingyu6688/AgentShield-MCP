<template>
  <a-card size="small" title="已配置的 MCP Server">
    <a-table
      :data-source="servers"
      :columns="columns"
      :loading="loading"
      :pagination="false"
      row-key="name"
      size="small"
    >
      <template #bodyCell="{ column, record }">
        <template v-if="column.key === 'transport'">
          <a-tag :color="record.transport === 'http' ? 'blue' : 'default'">{{ record.transport }}</a-tag>
        </template>
        <template v-else-if="column.key === 'enabled'">
          <a-badge :status="record.enabled ? 'success' : 'default'" :text="record.enabled ? '启用' : '禁用'" />
        </template>
        <template v-else-if="column.key === 'trust_level'">
          <a-tag>{{ record.trust_level }} · {{ trustName(record.trust_level) }}</a-tag>
        </template>
      </template>
      <template #emptyText>
        <a-empty description="尚未配置 MCP Server，用 agentshield mcp add 添加" />
      </template>
    </a-table>
  </a-card>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { message } from "ant-design-vue";
import { api, type ServerInfo } from "../api";

const loading = ref(false);
const servers = ref<ServerInfo[]>([]);

const columns = [
  { title: "名称", dataIndex: "name", width: 140 },
  { title: "传输", key: "transport", width: 90 },
  { title: "上游", dataIndex: "upstream", ellipsis: true },
  { title: "信任等级", key: "trust_level", width: 150 },
  { title: "状态", key: "enabled", width: 100 },
];

const TRUST = ["Blocked", "Read Only", "Confirm Write", "Trusted", "Sandboxed", "Admin"];
function trustName(n: number): string {
  return TRUST[n] ?? "?";
}

async function load() {
  loading.value = true;
  try {
    servers.value = (await api.servers()).servers;
  } catch (e) {
    message.error((e as Error).message);
  } finally {
    loading.value = false;
  }
}

onMounted(load);
</script>
