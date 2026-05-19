<script setup lang="ts">
import { ref, onMounted } from "vue";
import { X, Plus, Trash2 } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { useTargetGroupStore } from "@/stores/targetGroup";
import { useToastStore } from "@/stores/toast";
import type { TargetGroup } from "@/lib/tauri";

const emit = defineEmits<{
  close: [];
}>();

const store = useTargetGroupStore();
const toast = useToastStore();

const editingGroup = ref<TargetGroup | null>(null);
const groupName = ref("");
const targetsText = ref("");
const saving = ref(false);

onMounted(async () => {
  await store.loadGroups();
});

function startEdit(group: TargetGroup) {
  editingGroup.value = group;
  groupName.value = group.name;
  targetsText.value = group.targets.join("\n");
}

function cancelEdit() {
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

async function removeGroup(id: string) {
  try {
    await store.removeGroup(id);
    toast.add("success", "分组已删除");
  } catch (e) {
    toast.add("error", String(e));
  }
}
</script>

<template>
  <!-- Management dialog -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/45"
    @click.self="emit('close')"
  >
    <div class="flex w-full max-w-lg max-h-[80vh] flex-col rounded-2xl bg-paper shadow-2xl overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-paper-deep/50 px-4 md:px-6 py-4">
        <h2 class="text-base font-semibold text-ink">管理目标分组</h2>
        <button
          class="flex h-7 w-7 items-center justify-center rounded-md bg-paper-deep/30 text-ink-faint transition-colors hover:bg-paper-deep/60"
          @click="emit('close')"
        >
          <X class="h-4 w-4" />
        </button>
      </div>

      <!-- Group list -->
      <div class="flex-1 overflow-y-auto px-4 md:px-6 py-4">
        <!-- Existing groups -->
        <div
          v-for="g in store.groups"
          :key="g.id"
          class="group flex items-center justify-between rounded-lg border border-paper-deep/30 px-3 py-2.5 mb-2 hover:border-paper-deep/60"
        >
          <div class="min-w-0 flex-1">
            <p class="text-sm font-medium text-ink truncate">{{ g.name }}</p>
            <p class="text-xs text-ink-faint">{{ g.targets.length }} 个目标</p>
          </div>
          <div class="flex items-center gap-1 shrink-0">
            <button
              class="rounded-md p-1.5 text-ink-faint transition-colors hover:bg-paper-deep/30 hover:text-ink"
              title="编辑"
              @click="startEdit(g)"
            >
              <svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
              </svg>
            </button>
            <button
              class="rounded-md p-1.5 text-ink-faint transition-colors hover:bg-red-50 hover:text-red-600"
              title="删除"
              @click="removeGroup(g.id)"
            >
              <Trash2 class="h-3.5 w-3.5" />
            </button>
          </div>
        </div>

        <div v-if="store.groups.length === 0" class="py-8 text-center text-sm text-ink-faint">
          暂无分组，点击下方按钮创建
        </div>
      </div>

      <!-- Footer -->
      <div class="border-t border-paper-deep/50 px-4 md:px-6 py-3">
        <Button size="sm" class="w-full" @click="startEdit({ id: '', name: '', targets: [], createdAt: '', updatedAt: '' })">
          <Plus class="mr-1 h-3.5 w-3.5" />
          新建分组
        </Button>
      </div>
    </div>
  </div>

  <!-- Edit/Create dialog -->
  <div
    v-if="editingGroup !== null"
    class="fixed inset-0 z-[60] flex items-center justify-center bg-black/45"
    @click.self="cancelEdit"
  >
    <div class="flex w-full max-w-lg flex-col rounded-2xl bg-paper shadow-2xl overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-paper-deep/50 px-4 md:px-6 py-4">
        <h2 class="text-base font-semibold text-ink">
          {{ editingGroup.id ? "编辑分组" : "新建分组" }}
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
</template>
