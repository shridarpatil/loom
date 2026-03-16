<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { loom } from "@/utils/call";

const props = defineProps<{
  widget: {
    label: string;
    doctype: string;
    filters?: unknown[];
    route?: string;
    color?: string;
  };
}>();

const router = useRouter();
const count = ref<number | null>(null);
const loading = ref(true);

onMounted(async () => {
  try {
    const countRes = await loom.resource(props.widget.doctype).getList({
      fields: ["id"],
      filters: props.widget.filters,
      limit: 10000,
    });
    count.value = countRes.data.length;
  } catch {
    count.value = 0;
  } finally {
    loading.value = false;
  }
});

function navigate() {
  if (props.widget.route) {
    router.push(props.widget.route);
  } else {
    router.push(`/app/${props.widget.doctype}`);
  }
}
</script>

<template>
  <button
    class="bg-white border border-border rounded-lg px-4 py-3 text-left hover:border-primary-300 transition-all"
    @click="navigate"
  >
    <div class="text-xl font-semibold" :style="widget.color ? { color: widget.color } : {}">
      <span v-if="loading" class="inline-block w-6 h-5 bg-surface-raised rounded animate-pulse" />
      <span v-else>{{ count }}</span>
    </div>
    <div class="text-[12px] text-text-muted">{{ widget.label }}</div>
  </button>
</template>
