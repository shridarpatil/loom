<script setup lang="ts">
import ShortcutWidget from "./ShortcutWidget.vue";
import CountWidget from "./CountWidget.vue";
import ListWidget from "./ListWidget.vue";

defineProps<{
  widgets: Array<{
    type: "shortcut" | "count" | "list";
    [key: string]: unknown;
  }>;
}>();
</script>

<template>
  <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
    <template v-for="(widget, i) in widgets" :key="i">
      <ShortcutWidget v-if="widget.type === 'shortcut'" :widget="widget as any" />
      <CountWidget v-if="widget.type === 'count'" :widget="widget as any" />
      <ListWidget v-if="widget.type === 'list'" :widget="widget as any" class="col-span-full lg:col-span-2" />
    </template>
  </div>
</template>
