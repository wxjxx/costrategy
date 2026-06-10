import { createApp } from "vue";
import { createPinia } from "pinia";
import { VueQueryPlugin } from "@tanstack/vue-query";
import ElementPlus from "element-plus";
import zhCn from "element-plus/es/locale/lang/zh-cn";
import "element-plus/dist/index.css";
import "./styles/main.css";
import App from "./App.vue";
import { router } from "./router";
import { setUnauthorizedRedirectHandler } from "@/api/client";

setUnauthorizedRedirectHandler((path) => {
  if (router.currentRoute.value.path !== path) {
    void router.replace(path);
  }
});

createApp(App)
  .use(createPinia())
  .use(VueQueryPlugin)
  .use(router)
  .use(ElementPlus, { locale: zhCn })
  .mount("#app");
