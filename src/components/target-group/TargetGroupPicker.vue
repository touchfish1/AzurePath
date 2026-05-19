<script setup lang="ts">
import { ref, onMounted } from "vue";
import { Layers, Plus } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { useTargetGroupStore } from "@/stores/targetGroup";
import TargetGroupEditor from "@/components/target-group/TargetGroupEditor.vue";

defineProps<{
  modelValue: string | null;
  allowNone?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string | null];
}>();

const store = useTargetGroupStore();
const showEditor = ref(false);

onMounted(() => {
  store.loadGroups();
});

function selectGroup(id: string | null) {
  emit("update:modelValue", id);
}
</script>

<template>
  <div class="flex items-center gap-2">
    <div class="flex-1 relative">
      <Layers
        class="absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-ink-faint pointer-events-none"
      />
      <select
        :value="modelValue ?? ''"
        class="w-full appearance-none rounded-lg border border-paper-deep bg-paper-warm/50 pl-9 pr-8 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
        @change="selectGroup(($event.target as HTMLSelectElement).value || null)"
      >
        <option value="">-- 不使用分组 --</option>
        <option v-for="g in store.groups" :key="g.id" :value="g.id">
          {{ g.name }} ({{ g.targets.length }})
        </option>
      </select>
      <div class="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-ink-faint">
        <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </div>
    </div>
    <Button variant="ghost" size="sm" @click="showEditor = true" title="管理目标组">
      <Plus class="h-3.5 w-3.5" />
    </Button>
  </div>

  <TargetGroupEditor v-if="showEditor" @close="showEditor = false" />
</template>
