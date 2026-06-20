<template>
  <div>
    <a-spin :spinning="loading">
      <a-row :gutter="16">
        <a-col :span="5"><stat-card title="今日工具调用" :value="summary?.today ?? 0" /></a-col>
        <a-col :span="5"><stat-card title="高危及以上" :value="summary?.high_risk ?? 0" color="#d4380d" /></a-col>
        <a-col :span="5"><stat-card title="被阻止" :value="summary?.blocked ?? 0" color="#cf1322" /></a-col>
        <a-col :span="5"><stat-card title="活跃 MCP Server" :value="summary?.active_servers ?? 0" /></a-col>
        <a-col :span="4"><stat-card title="事件总数" :value="summary?.total ?? 0" /></a-col>
      </a-row>

      <a-row :gutter="16" style="margin-top: 16px">
        <a-col :span="9">
          <a-card title="风险等级分布" size="small">
            <e-chart v-if="hasEvents" :option="pieOption" height="300px" />
            <a-empty v-else description="暂无事件" />
          </a-card>
        </a-col>
        <a-col :span="15">
          <a-card title="最近 10 条风险事件" size="small">
            <a-table
              :data-source="summary?.recent ?? []"
              :columns="columns"
              :pagination="false"
              row-key="id"
              size="small"
            >
              <template #bodyCell="{ column, record }">
                <template v-if="column.key === 'level'">
                  <a-tag :color="levelColor(record.risk_level)">
                    {{ levelText(record.risk_level) }} {{ record.risk_score }}
                  </a-tag>
                </template>
                <template v-else-if="column.key === 'decision'">
                  <a-tag :color="decisionColor(record.decision)">{{ decisionText(record.decision) }}</a-tag>
                </template>
                <template v-else-if="column.key === 'created_at'">
                  {{ shortTime(record.created_at) }}
                </template>
              </template>
            </a-table>
          </a-card>
        </a-col>
      </a-row>
    </a-spin>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { message } from "ant-design-vue";
import { api, type Summary } from "../api";
import { levelColor, levelText, decisionColor, decisionText, shortTime } from "../utils/format";
import EChart from "../components/EChart.vue";
import StatCard from "../components/StatCard.vue";

const loading = ref(false);
const summary = ref<Summary>();

const hasEvents = computed(() => (summary.value?.total ?? 0) > 0);

const columns = [
  { title: "时间", key: "created_at", dataIndex: "created_at", width: 140 },
  { title: "来源", dataIndex: "client_name", width: 110 },
  { title: "类型", dataIndex: "event_type", width: 110 },
  { title: "目标", dataIndex: "target", ellipsis: true },
  { title: "风险", key: "level", width: 110 },
  { title: "决策", key: "decision", width: 90 },
];

const pieOption = computed(() => {
  const lv = summary.value?.by_level ?? {};
  const palette: Record<string, string> = {
    低: "#52c41a",
    中: "#faad14",
    高: "#fa541c",
    严重: "#cf1322",
  };
  const data = [
    { name: "低", value: lv.Low ?? 0 },
    { name: "中", value: lv.Medium ?? 0 },
    { name: "高", value: lv.High ?? 0 },
    { name: "严重", value: lv.Critical ?? 0 },
  ].filter((d) => d.value > 0);
  return {
    tooltip: { trigger: "item" },
    legend: { bottom: 0 },
    series: [
      {
        type: "pie",
        radius: ["45%", "70%"],
        itemStyle: { borderRadius: 4, borderColor: "#fff", borderWidth: 2 },
        data: data.map((d) => ({ ...d, itemStyle: { color: palette[d.name] } })),
      },
    ],
  };
});

async function load() {
  loading.value = true;
  try {
    summary.value = await api.summary();
  } catch (e) {
    message.error((e as Error).message);
  } finally {
    loading.value = false;
  }
}

onMounted(load);
</script>
