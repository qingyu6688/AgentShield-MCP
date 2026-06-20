import { createRouter, createWebHashHistory } from "vue-router";

// 用 hash 路由，便于静态托管在任意路径下而无需服务端 rewrite
const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/dashboard" },
    {
      path: "/dashboard",
      name: "dashboard",
      component: () => import("../pages/Dashboard.vue"),
      meta: { title: "总览" },
    },
    {
      path: "/events",
      name: "events",
      component: () => import("../pages/AuditLogs.vue"),
      meta: { title: "审计日志" },
    },
    {
      path: "/live",
      name: "live",
      component: () => import("../pages/LiveEvents.vue"),
      meta: { title: "实时事件" },
    },
    {
      path: "/servers",
      name: "servers",
      component: () => import("../pages/Servers.vue"),
      meta: { title: "MCP Server" },
    },
    {
      path: "/memory",
      name: "memory",
      component: () => import("../pages/Memory.vue"),
      meta: { title: "确认记忆" },
    },
    {
      path: "/reports",
      name: "reports",
      component: () => import("../pages/Reports.vue"),
      meta: { title: "报告" },
    },
  ],
});

export default router;
