<template>
  <a-layout style="height: 100vh">
    <a-layout-sider :width="220" theme="dark">
      <div class="brand">
        <span class="brand-mark">AgentShield</span>
        <span class="brand-sub">MCP 安全防火墙</span>
      </div>
      <a-menu
        theme="dark"
        mode="inline"
        :selected-keys="[route.name as string]"
        @click="onMenu"
      >
        <a-menu-item key="dashboard">总览</a-menu-item>
        <a-menu-item key="live">实时事件</a-menu-item>
        <a-menu-item key="events">审计日志</a-menu-item>
        <a-menu-item key="servers">MCP Server</a-menu-item>
        <a-menu-item key="memory">确认记忆</a-menu-item>
        <a-menu-item key="reports">报告</a-menu-item>
      </a-menu>
    </a-layout-sider>

    <a-layout>
      <a-layout-header class="header">
        <span class="header-title">{{ route.meta.title }}</span>
        <span class="muted">本地优先 · 数据不出本机</span>
      </a-layout-header>
      <a-layout-content class="content">
        <router-view />
      </a-layout-content>
    </a-layout>
  </a-layout>
</template>

<script setup lang="ts">
import { useRoute, useRouter } from "vue-router";

const route = useRoute();
const router = useRouter();

function onMenu({ key }: { key: string }) {
  router.push({ name: key });
}
</script>

<style scoped>
.brand {
  padding: 18px 20px;
  color: #fff;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.brand-mark {
  font-size: 18px;
  font-weight: 700;
  letter-spacing: 0.3px;
}
.brand-sub {
  font-size: 12px;
  color: #9aa4b2;
}
.header {
  background: #fff;
  padding: 0 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid #eef0f3;
}
.header-title {
  font-size: 16px;
  font-weight: 600;
}
.content {
  padding: 24px;
  overflow: auto;
}
</style>
