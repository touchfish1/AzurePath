<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Layers, Plus, Trash2, Edit, X } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { useTargetGroupStore } from "@/stores/targetGroup";
import { useToastStore } from "@/stores/toast";
import type { TargetGroup } from "@/lib/tauri";

const store = useTargetGroupStore();
const toast = useToastStore();

const showEditor = ref(false);
const editingGroup = ref<TargetGroup | null>(null);
const groupName = ref("");
const targetsText = ref("");
const saving = ref(false);

onMounted(async () => {
  await store.loadGroups();
});

function startNew() {
  editingGroup.value = null;
  groupName.value = "";
  targetsText.value = "";
  showEditor.value = true;
}

function startEdit(group: TargetGroup) {
  editingGroup.value = group;
  groupName.value = group.name;
  targetsText.value = group.targets.join("\n");
  showEditor.value = true;
}

function cancelEdit() {
  showEditor.value = false;
  editingGroup.value = null;
  groupName.value = "";
  targetsText.value = "";
}

async function save() {
  const name = groupName.value.trim();
  if (!name) {
    toast.add("error", "请输入分组名称");
    return;
  }

  const targets = targetsText.value
    .split("\n")
    .map((t) => t.trim())
    .filter((t) => t.length > 0);

  if (targets.length === 0) {
    toast.add("error", "请输入至少一个目标地址");
    return;
  }

  saving.value = true;
  try {
    await store.saveGroup(editingGroup.value?.id ?? null, name, targets);
    toast.add("success", editingGroup.value ? "分组已更新" : "分组已创建");
    cancelEdit();
  } catch (e) {
    toast.add("error", String(e));
  } finally {
    saving.value = false;
  }
}

async function removeGroup(id: string, name: string) {
  if (confirm(`确定要删除分组「${name}」吗？`)) {
    try {
      await store.removeGroup(id);
      toast.add("success", "分组已删除");
    } catch (e) {
      toast.add("error", String(e));
    }
  }
}
</script>

<template>
  <div class="flex h-full flex-col p-4 md:p-6 space-y-4 md:space-y-6 animate-view-fade">
    <!-- Header -->
    <div class="flex items-start justify-between">
      <div>
        <h1 class="text-2xl font-display font-bold text-ink">目标分组</h1>
        <p class="mt-0.5 text-sm text-ink-faint">管理批量操作的目标地址分组</p>
      </div>
      <Button @click="startNew">
        <Plus class="mr-1.5 h-3.5 w-3.5" />
        新建分组
      </Button>
    </div>

    <!-- Loading -->
    <div
      v-if="store.loading"
      class="flex items-center justify-center py-16 text-sm text-ink-faint"
    >
      加载中...
    </div>

    <!-- Group list -->
    <div v-else-if="store.groups.length > 0" class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      <div
        v-for="g in store.groups"
        :key="g.id"
        class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm hover:shadow-md transition-shadow"
      >
        <div class="flex items-start justify-between mb-3">
          <div class="flex items-center gap-2 min-w-0">
            <Layers class="h-4 w-4 shrink-0 text-bamboo" />
            <h3 class="text-sm font-semibold text-ink truncate">{{ g.name }}</h3>
          </div>
          <div class="flex items-center gap-1 shrink-0">
            <button
              class="rounded-md p-1.5 text-ink-faint transition-colors hover:bg-paper-deep/30 hover:text-ink"
              title="编辑"
              @click="startEdit(g)"
            >
              <Edit class="h-3.5 w-3.5" />
            </button>
            <button
              class="rounded-md p-1.5 text-ink-faint transition-colors hover:bg-red-50 hover:text-red-600"
              title="删除"
              @click="removeGroup(g.id, g.name)"
            >
              <Trash2 class="h-3.5 w-3.5" />
            </button>
          </div>
        </div>

        <div class="mb-2">
          <span class="text-xs text-ink-faint">{{ g.targets.length }} 个目标</span>
        </div>

        <div class="flex flex-wrap gap-1.5">
          <span
            v-for="target in g.targets.slice(0, 6)"
            :key="target"
            class="inline-block rounded-md bg-paper-deep/30 px-2 py-0.5 text-xs font-mono text-ink-soft truncate max-w-[140px]"
            :title="target"
          >
            {{ target }}
          </span>
          <span
            v-if="g.targets.length > 6"
            class="inline-block rounded-md bg-paper-deep/20 px-2 py-0.5 text-xs text-ink-faint"
          >
            +{{ g.targets.length - 6 }}
          </span>
        </div>

        <div class="mt-3 text-xs text-ink-faint">
          更新于 {{ new Date(g.updatedAt).toLocaleString("zh-CN") }}
        </div>
      </div>
    </div>

    <!-- Empty state -->
    <div
      v-else
      class="flex items-center justify-center rounded-xl border border-dashed border-paper-deep/30 bg-paper-warm/20 py-16 text-sm text-ink-faint"
    >
      <div class="text-center max-w-sm">
        <Layers class="mx-auto h-10 w-10 mb-3 opacity-30" />
        <p class="font-medium text-ink-soft">暂无目标分组</p>
        <p class="mt-2 text-xs opacity-60 leading-relaxed">
          创建分组以便批量执行 Ping、端口扫描和路由追踪
        </p>
      </div>
    </div>

    <!-- Edit/Create dialog -->
    <div
      v-if="showEditor"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/45"
      @click.self="cancelEdit"
    >
      <div class="flex w-full max-w-lg flex-col rounded-2xl bg-paper shadow-2xl overflow-hidden">
        <!-- Header -->
        <div class="flex items-center justify-between border-b border-paper-deep/50 px-4 md:px-6 py-4">
          <h2 class="text-base font-semibold text-ink">
            {{ editingGroup ? "编辑分组" : "新建分组" }}
          </h2>
          <button
            class="flex h-7 w-7 items-center justify-center rounded-md bg-paper-deep/30 text-ink-faint transition-colors hover:bg-paper-deep/60"
            @click="cancelEdit"
          >
            <X class="h-4 w-4" />
          </button>
        </div>

        <!-- Body -->
        <div class="px-4 md:px-6 py-4 space-y-4">
          <div>
            <label class="mb-1 block text-xs font-medium text-ink-soft">分组名称</label>
            <input
              v-model="groupName"
              type="text"
              placeholder="例如：办公网络、服务器集群"
              class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
            />
          </div>
          <div>
            <label class="mb-1 block text-xs font-medium text-ink-soft">
              目标地址
              <span class="text-ink-faint font-normal">（每行一个 IP 或域名）</span>
            </label>
            <textarea
              v-model="targetsText"
              placeholder="192.168.1.1&#10;192.168.1.2&#10;10.0.0.1&#10;example.com"
              rows="8"
              class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm font-mono text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 resize-y"
            />
          </div>
        </div>

        <!-- Footer -->
        <div class="flex items-center justify-end gap-2 border-t border-paper-deep/50 px-4 md:px-6 py-3">
          <Button variant="ghost" size="sm" @click="cancelEdit">取消</Button>
          <Button size="sm" :disabled="saving" @click="save">
            {{ saving ? "保存中..." : "保存" }}
          </Button>
        </div>
      </div>
    </div>
  </div>
</template>
