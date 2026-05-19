<script setup lang="ts">
import { FileText } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { useToastStore } from "@/stores/toast";
import { generateHtmlReport, type ReportColumn } from "@/lib/report";
import { save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";

const toast = useToastStore();

interface Props {
  title: string;
  columns: ReportColumn[];
  rows: Record<string, unknown>[];
}

const props = defineProps<Props>();

async function exportReport() {
  if (props.rows.length === 0) {
    toast.add("warning", "没有数据可导出");
    return;
  }

  try {
    const path = await save({
      defaultPath: `${props.title.replace(/[\\/:*?"<>|]/g, "_")}_${formatTimestamp()}.html`,
      filters: [{ name: "HTML 报告", extensions: ["html"] }],
    });

    if (!path) return; // User cancelled

    const timestamp = new Date().toLocaleString("zh-CN", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });

    const html = generateHtmlReport({
      title: props.title,
      columns: props.columns,
      rows: props.rows,
      timestamp,
    });

    await invoke("save_report", { path, content: html });
    toast.add("success", "报告已导出");
  } catch (e) {
    toast.add("error", `导出失败: ${String(e)}`);
  }
}

function formatTimestamp(): string {
  const now = new Date();
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  const h = String(now.getHours()).padStart(2, "0");
  const mi = String(now.getMinutes()).padStart(2, "0");
  return `${y}${m}${d}_${h}${mi}`;
}
</script>

<template>
  <Button variant="secondary" size="sm" @click="exportReport">
    <FileText class="mr-1.5 h-3.5 w-3.5" />
    导出报告
  </Button>
</template>
