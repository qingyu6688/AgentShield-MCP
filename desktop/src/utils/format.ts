// 展示层通用格式化：风险等级 / 决策 的颜色与文案。

export function levelColor(level: string): string {
  switch (level) {
    case "Low":
      return "green";
    case "Medium":
      return "gold";
    case "High":
      return "volcano";
    case "Critical":
      return "red";
    default:
      return "default";
  }
}

export function decisionColor(decision: string): string {
  switch (decision) {
    case "Allow":
      return "green";
    case "Log":
      return "blue";
    case "Confirm":
      return "gold";
    case "Block":
      return "red";
    case "Sandbox":
      return "purple";
    default:
      return "default";
  }
}

const LEVEL_CN: Record<string, string> = {
  Low: "低",
  Medium: "中",
  High: "高",
  Critical: "严重",
};
const DECISION_CN: Record<string, string> = {
  Allow: "放行",
  Log: "记录",
  Confirm: "确认",
  Block: "阻止",
  Sandbox: "沙箱",
};

export function levelText(level: string): string {
  return LEVEL_CN[level] ?? level;
}
export function decisionText(decision: string): string {
  return DECISION_CN[decision] ?? decision;
}

/** 把 RFC3339 时间转成本地可读的 'MM-DD HH:mm:ss'。 */
export function shortTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const p = (n: number) => String(n).padStart(2, "0");
  return `${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}
