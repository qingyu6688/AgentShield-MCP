<template>
  <a-spin :spinning="loading">
    <a-row :gutter="16">
      <a-col :span="12">
        <a-card size="small">
          <template #title><a-tag color="green">始终允许</a-tag>白名单</template>
          <a-table :data-source="allow" :columns="columns" :pagination="false" row-key="key" size="small">
            <template #emptyText><a-empty description="暂无" /></template>
          </a-table>
        </a-card>
      </a-col>
      <a-col :span="12">
        <a-card size="small">
          <template #title><a-tag color="red">永久拉黑</a-tag>黑名单</template>
          <a-table :data-source="block" :columns="columns" :pagination="false" row-key="key" size="small">
            <template #emptyText><a-empty description="暂无" /></template>
          </a-table>
        </a-card>
      </a-col>
    </a-row>
    <p class="muted" style="margin-top: 12px">
      这些记忆由终端确认时选择「始终允许 / 永久拉黑」生成；始终允许不会覆盖明确的阻断策略。可在 .agentshield/decisions.json 编辑。
    </p>
  </a-spin>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { message } from "ant-design-vue";
import { api, type MemoryEntry } from "../api";

const loading = ref(false);
const allow = ref<(MemoryEntry & { key: string })[]>([]);
const block = ref<(MemoryEntry & { key: string })[]>([]);

const columns = [
  { title: "Server", dataIndex: "server", width: 120 },
  { title: "工具", dataIndex: "tool", width: 160 },
  { title: "目标", dataIndex: "target", ellipsis: true },
];

function withKey(list: MemoryEntry[]) {
  return list.map((e, i) => ({ ...e, key: `${e.server}|${e.tool}|${e.target}|${i}` }));
}

async function load() {
  loading.value = true;
  try {
    const m = await api.memory();
    allow.value = withKey(m.allow);
    block.value = withKey(m.block);
  } catch (e) {
    message.error((e as Error).message);
  } finally {
    loading.value = false;
  }
}

onMounted(load);
</script>
