<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { loom } from "@/utils/call";

const props = defineProps<{
  widget: {
    label: string;
    doctype: string;
    filters?: unknown[];
    fields?: string[];
    limit?: number;
    order_by?: string;
  };
}>();

const router = useRouter();
const rows = ref<Record<string, unknown>[]>([]);
const loading = ref(true);

onMounted(async () => {
  try {
    const res = await loom.resource(props.widget.doctype).getList({
      fields: props.widget.fields || ["id"],
      filters: props.widget.filters,
      limit: props.widget.limit || 5,
      order_by: props.widget.order_by || "modified desc",
    });
    rows.value = res.data;
  } catch {
    rows.value = [];
  } finally {
    loading.value = false;
  }
});
</script>

<template>
  <div class="bg-white border border-border rounded-lg overflow-hidden">
    <div class="px-4 py-2.5 border-b border-border bg-surface-muted/30 flex items-center justify-between">
      <h3 class="text-[12px] font-semibold text-text-muted uppercase tracking-wide">{{ widget.label }}</h3>
      <button
        class="text-[11px] text-primary-600 hover:text-primary-700 font-medium"
        @click="router.push(`/app/${widget.doctype}`)"
      >View All</button>
    </div>
    <div v-if="loading" class="px-4 py-3 space-y-2">
      <div v-for="i in 3" :key="i" class="h-4 bg-surface-raised rounded animate-pulse" :style="{ width: `${60 + i * 15}%` }" />
    </div>
    <div v-else-if="rows.length === 0" class="px-4 py-6 text-center text-[12px] text-text-light">
      No records
    </div>
    <div v-else>
      <button
        v-for="row in rows"
        :key="String(row.id)"
        class="w-full flex items-center gap-2 px-4 py-2 text-left border-b border-border last:border-0 hover:bg-surface-muted/40 transition-colors text-[13px]"
        @click="router.push(`/app/${widget.doctype}/${row.id}`)"
      >
        <span class="text-primary-600 font-medium truncate">{{ row.id }}</span>
        <span v-if="row.status" class="ml-auto text-[11px] text-text-muted">{{ row.status }}</span>
      </button>
    </div>
  </div>
</template>
