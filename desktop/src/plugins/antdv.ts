import type { App } from "vue";
import {
  Badge,
  Button,
  Card,
  Col,
  Descriptions,
  Drawer,
  Empty,
  Input,
  InputNumber,
  Layout,
  Menu,
  Row,
  Segmented,
  Select,
  Space,
  Spin,
  Switch,
  Table,
  Tag,
} from "ant-design-vue";

const components = [
  Badge,
  Button,
  Card,
  Col,
  Descriptions,
  Drawer,
  Empty,
  Input,
  InputNumber,
  Layout,
  Menu,
  Row,
  Segmented,
  Select,
  Space,
  Spin,
  Switch,
  Table,
  Tag,
];

export function setupAntd(app: App): void {
  components.forEach((component) => app.use(component));
}
