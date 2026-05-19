import { onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { useThemeStore } from "@/stores/theme";

const ROUTE_ORDER = [
  "dashboard",
  "ping",
  "traceroute",
  "port-scan",
  "dns",
  "chat",
  "clipboard",
  "network-sniffer",
  "files",
];

export function useKeyboardShortcuts() {
  const router = useRouter();
  const themeStore = useThemeStore();

  function handleKeyDown(e: KeyboardEvent) {
    // Skip if focus is inside an editable element
    const target = e.target as HTMLElement;
    if (
      target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.isContentEditable
    ) {
      return;
    }

    if (!e.ctrlKey || e.shiftKey || e.altKey || e.metaKey) return;

    // Ctrl+1 through Ctrl+9: navigate to routes
    const num = parseInt(e.key, 10);
    if (num >= 1 && num <= ROUTE_ORDER.length) {
      e.preventDefault();
      router.push({ name: ROUTE_ORDER[num - 1] });
      return;
    }

    switch (e.key.toLowerCase()) {
      case "t":
        e.preventDefault();
        themeStore.toggleTheme();
        break;
      case "d":
        e.preventDefault();
        router.push({ name: "dashboard" });
        break;
      case "f":
        e.preventDefault();
        router.push({ name: "files" });
        break;
    }
  }

  onMounted(() => {
    window.addEventListener("keydown", handleKeyDown);
  });

  onUnmounted(() => {
    window.removeEventListener("keydown", handleKeyDown);
  });
}
