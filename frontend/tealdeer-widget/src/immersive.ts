import { createApp } from "vue";
import ImmersiveView from "./components/ImmersiveView.vue";
import i18n from "./locales";
import "./styles/theme.css";

createApp(ImmersiveView).use(i18n).mount("#app");
