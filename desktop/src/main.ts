import { createApp } from "vue";
import "ant-design-vue/dist/reset.css";
import App from "./App.vue";
import { setupAntd } from "./plugins/antdv";
import router from "./router";
import "./style.css";

const app = createApp(App);

setupAntd(app);
app.use(router).mount("#app");
