import { createApp } from "vue";
import PrimeVue from "primevue/config";
import Aura from "@primevue/themes/aura";
import "primeicons/primeicons.css";
import App from "./App.vue";
import router from "./router";
import { pinia } from "./stores";
import { installAppGuards } from "./utils/appGuards";
import "./styles/main.css";

installAppGuards();
const app = createApp(App);

app.use(pinia);
app.use(router);
app.use(PrimeVue, {
  theme: {
    preset: Aura,
    options: {
      darkModeSelector: ".app-dark",
      cssLayer: false,
    },
  },
});

app.mount("#app");
