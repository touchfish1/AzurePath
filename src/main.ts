import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import router from "./router";
import "./style.css";

const app = createApp(App);
app.use(createPinia());
app.use(router);

// Global Vue error handler
app.config.errorHandler = (err, _instance, info) => {
  console.error("[Global Vue Error]", err, info);
  // Attempt to show a toast if toast store is available
  try {
    import("@/stores/toast").then(({ useToastStore }) => {
      const toast = useToastStore();
      toast.error(`应用错误: ${err}`);
    });
  } catch {
    // Toast store not available
  }
};

// Window-level unhandled rejection handler
window.addEventListener("unhandledrejection", (event) => {
  console.error("[Unhandled Promise Rejection]", event.reason);
});

// Window-level error handler
window.addEventListener("error", (event) => {
  console.error("[Window Error]", event.error ?? event.message);
});

app.mount("#root");
