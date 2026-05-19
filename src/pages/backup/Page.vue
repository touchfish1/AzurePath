<script setup lang="ts">
import { ref, onMounted } from "vue";
import {
  backupAllData,
  listBackups,
  restoreBackup,
  deleteBackup,
  type BackupInfo,
} from "@/lib/tauri";
import { useToastStore } from "@/stores/toast";
import { HardDrive, Download, Trash2, RefreshCw, RotateCw, AlertTriangle } from "lucide-vue-next";

const toast = useToastStore();

const backups = ref<BackupInfo[]>([]);
const loading = ref(false);
const restoring = ref<string | null>(null);
const deleting = ref<string | null>(null);
const confirmPath = ref<string | null>(null);
const confirmAction = ref<"restore" | "delete" | null>(null);

async function fetchBackups() {
  try {
    backups.value = await listBackups();
  } catch (e) {
    toast.error(`获取备份列表失败: ${e}`);
  }
}

async function handleCreateBackup() {
  loading.value = true;
  try {
    const path = await backupAllData();
    toast.success(`备份已创建: ${path}`);
    await fetchBackups();
  } catch (e) {
    toast.error(`创建备份失败: ${e}`);
  } finally {
    loading.value = false;
  }
}

function confirmRestore(path: string) {
  confirmPath.value = path;
  confirmAction.value = "restore";
}

function confirmDelete(path: string) {
  confirmPath.value = path;
  confirmAction.value = "delete";
}

async function executeRestore() {
  if (!confirmPath.value) return;
  restoring.value = confirmPath.value;
  try {
    const result = await restoreBackup(confirmPath.value);
    toast.success(`恢复完成: ${result}`);
  } catch (e) {
    toast.error(`恢复失败: ${e}`);
  } finally {
    restoring.value = null;
    confirmPath.value = null;
    confirmAction.value = null;
  }
}

async function executeDelete() {
  if (!confirmPath.value) return;
  deleting.value = confirmPath.value;
  try {
    await deleteBackup(confirmPath.value);
    toast.success("备份已删除");
    await fetchBackups();
  } catch (e) {
    toast.error(`删除失败: ${e}`);
  } finally {
    deleting.value = null;
    confirmPath.value = null;
    confirmAction.value = null;
  }
}

function cancelConfirm() {
  confirmPath.value = null;
  confirmAction.value = null;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "-";
  try {
    const d = new Date(dateStr);
    return d.toLocaleString("zh-CN");
  } catch {
    return dateStr;
  }
}

onMounted(() => {
  fetchBackups();
});
</script>

<template>
  <div class="flex h-full flex-col p-6">
    <!-- Header -->
    <div class="mb-4 flex items-center justify-between">
      <div class="flex items-center gap-3">
        <HardDrive class="h-5 w-5 text-ink-soft" />
        <h1 class="text-lg font-semibold text-ink">数据备份</h1>
      </div>
      <div class="flex items-center gap-2">
        <button
          class="inline-flex h-8 items-center gap-1.5 rounded-lg border border-paper-deep bg-paper px-3 text-xs text-ink-soft transition-colors hover:bg-paper-deep hover:text-ink"
          @click="fetchBackups"
        >
          <RefreshCw class="h-3.5 w-3.5" />
          刷新
        </button>
        <button
          class="inline-flex h-8 items-center gap-1.5 rounded-lg bg-bamboo px-4 text-xs font-medium text-white transition-colors hover:bg-bamboo/90 disabled:opacity-50"
          :disabled="loading"
          @click="handleCreateBackup"
        >
          <RotateCw v-if="loading" class="h-3.5 w-3.5 animate-spin" />
          <HardDrive v-else class="h-3.5 w-3.5" />
          {{ loading ? "备份中..." : "创建备份" }}
        </button>
      </div>
    </div>

    <!-- Confirmation dialog -->
    <div
      v-if="confirmPath && confirmAction"
      class="mb-4 rounded-xl border border-yellow-500/30 bg-yellow-500/5 p-4"
    >
      <div class="flex items-start gap-3">
        <AlertTriangle class="mt-0.5 h-5 w-5 shrink-0 text-yellow-500" />
        <div class="flex-1">
          <p class="text-sm font-medium text-ink">
            {{ confirmAction === "restore" ? "确认恢复备份？" : "确认删除备份？" }}
          </p>
          <p class="mt-1 text-xs text-ink-soft">
            {{ confirmAction === "restore"
              ? "恢复操作将覆盖当前数据（设置、配置等）。此操作不可撤销。"
              : "删除后无法恢复此备份文件。" }}
          </p>
          <p class="mt-1 text-xs font-mono text-ink-faint">
            {{ confirmPath }}
          </p>
          <div class="mt-3 flex items-center gap-2">
            <button
              class="inline-flex h-7 items-center gap-1 rounded-lg px-3 text-xs font-medium text-white transition-colors"
              :class="confirmAction === 'restore' ? 'bg-yellow-500 hover:bg-yellow-500/90' : 'bg-red-500 hover:bg-red-500/90'"
              :disabled="restoring === confirmPath || deleting === confirmPath"
              @click="confirmAction === 'restore' ? executeRestore() : executeDelete()"
            >
              <RotateCw
                v-if="(confirmAction === 'restore' && restoring === confirmPath) || (confirmAction === 'delete' && deleting === confirmPath)"
                class="h-3 w-3 animate-spin"
              />
              {{ confirmAction === "restore" ? "确认恢复" : "确认删除" }}
            </button>
            <button
              class="inline-flex h-7 items-center rounded-lg border border-paper-deep bg-paper px-3 text-xs text-ink-soft transition-colors hover:bg-paper-deep"
              @click="cancelConfirm"
            >
              取消
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Backup list -->
    <div class="flex-1 overflow-hidden rounded-xl border border-paper-deep bg-paper">
      <!-- Empty state -->
      <div
        v-if="backups.length === 0"
        class="flex h-full flex-col items-center justify-center gap-3 text-center"
      >
        <HardDrive class="h-10 w-10 text-ink-faint" />
        <p class="text-sm text-ink-soft">暂无备份</p>
        <p class="text-xs text-ink-faint">点击上方"创建备份"按钮开始</p>
      </div>

      <!-- Table -->
      <div v-else class="h-full overflow-auto">
        <table class="w-full text-left text-xs">
          <thead class="sticky top-0 bg-paper-warm">
            <tr class="border-b border-paper-deep">
              <th class="px-4 py-2.5 font-medium text-ink-soft">文件名</th>
              <th class="px-4 py-2.5 font-medium text-ink-soft">大小</th>
              <th class="px-4 py-2.5 font-medium text-ink-soft">创建时间</th>
              <th class="px-4 py-2.5 font-medium text-ink-soft">操作</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-paper-deep/50">
            <tr
              v-for="backup in backups"
              :key="backup.path"
              class="transition-colors hover:bg-paper-warm/50"
            >
              <td class="max-w-[300px] truncate px-4 py-2.5 font-medium text-ink">
                {{ backup.name }}
              </td>
              <td class="whitespace-nowrap px-4 py-2.5 text-ink-soft">
                {{ formatSize(backup.size) }}
              </td>
              <td class="whitespace-nowrap px-4 py-2.5 text-ink-soft">
                {{ formatDate(backup.created) }}
              </td>
              <td class="px-4 py-2.5">
                <div class="flex items-center gap-1.5">
                  <button
                    class="inline-flex h-7 items-center gap-1 rounded-lg border border-paper-deep bg-paper px-2.5 text-xs text-ink-soft transition-colors hover:bg-bamboo/10 hover:text-bamboo disabled:opacity-50"
                    :disabled="restoring === backup.path"
                    @click="confirmRestore(backup.path)"
                  >
                    <Download class="h-3 w-3" />
                    恢复
                  </button>
                  <button
                    class="inline-flex h-7 items-center gap-1 rounded-lg border border-paper-deep bg-paper px-2.5 text-xs text-ink-soft transition-colors hover:bg-danger-bg hover:text-red-600 disabled:opacity-50"
                    :disabled="deleting === backup.path"
                    @click="confirmDelete(backup.path)"
                  >
                    <Trash2 class="h-3 w-3" />
                    删除
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>
