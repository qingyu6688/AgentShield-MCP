// 与 agentshield dashboard 后端通信的接口层。组件不直接写 fetch。

const BASE = "/api";

async function get<T>(path: string): Promise<T> {
  const resp = await fetch(BASE + path);
  if (!resp.ok) {
    throw new Error(`请求失败（${resp.status}）`);
  }
  return resp.json() as Promise<T>;
}

function qs(params: Record<string, string | number | undefined>): string {
  const parts = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== "")
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`);
  return parts.length ? `?${parts.join("&")}` : "";
}

export interface AuditEvent {
  id: string;
  session_id: string;
  client_name: string;
  server_name: string;
  tool_name: string;
  event_type: string;
  target?: string | null;
  arguments_json?: unknown;
  result_json?: unknown;
  risk_score: number;
  risk_level: string;
  decision: string;
  reason: string;
  created_at: string;
}

export interface Summary {
  total: number;
  today: number;
  blocked: number;
  high_risk: number;
  active_servers: number;
  by_level: Record<string, number>;
  recent: AuditEvent[];
}

export interface ServerInfo {
  name: string;
  transport: string;
  upstream: string;
  trust_level: number;
  enabled: boolean;
}

export interface MemoryEntry {
  server: string;
  tool: string;
  target: string;
}

export interface EventQuery {
  level?: string;
  server?: string;
  since?: string;
  until?: string;
  limit?: number;
}

export const api = {
  summary: () => get<Summary>("/summary"),
  events: (q: EventQuery = {}) => get<AuditEvent[]>("/events" + qs(q)),
  servers: () => get<{ servers: ServerInfo[] }>("/servers"),
  memory: () => get<{ allow: MemoryEntry[]; block: MemoryEntry[] }>("/memory"),
  report: (format: string) => get<{ content: string }>("/report" + qs({ format })),
};
