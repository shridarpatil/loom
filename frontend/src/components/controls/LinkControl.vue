<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount } from "vue";
import type { DocFieldMeta } from "@/utils/call";
import { loom } from "@/utils/call";

const props = defineProps<{
  field: DocFieldMeta;
  modelValue: unknown;
  readOnly?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: unknown];
}>();

const inputValue = ref(String(props.modelValue ?? ""));
const options = ref<string[]>([]);
const showDropdown = ref(false);
const highlightIndex = ref(-1);
const containerRef = ref<HTMLElement | null>(null);
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

watch(
  () => props.modelValue,
  (val) => {
    inputValue.value = String(val ?? "");
  },
);

async function fetchOptions(term: string) {
  if (!props.field.options) return;
  try {
    const res = await loom.resource(props.field.options).getList({
      search_term: term || undefined,
      fields: ["id"],
      limit: 10,
    });
    options.value = res.data.map((d) => String(d.id));
  } catch {
    options.value = [];
  }
}

function onInput(e: Event) {
  const value = (e.target as HTMLInputElement).value;
  inputValue.value = value;
  highlightIndex.value = -1;
  showDropdown.value = true;

  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => fetchOptions(value), 200);
}

function selectOption(option: string) {
  inputValue.value = option;
  emit("update:modelValue", option);
  showDropdown.value = false;
  highlightIndex.value = -1;
}

function onFocus() {
  showDropdown.value = true;
  fetchOptions(inputValue.value);
}

function onBlur() {
  // Delay to allow click on dropdown
  setTimeout(() => {
    showDropdown.value = false;
    // Commit current input value
    if (inputValue.value !== String(props.modelValue ?? "")) {
      emit("update:modelValue", inputValue.value || null);
    }
  }, 200);
}

function onKeydown(e: KeyboardEvent) {
  if (!showDropdown.value || options.value.length === 0) {
    if (e.key === "ArrowDown") {
      showDropdown.value = true;
      fetchOptions(inputValue.value);
    }
    return;
  }

  switch (e.key) {
    case "ArrowDown":
      e.preventDefault();
      highlightIndex.value = Math.min(highlightIndex.value + 1, options.value.length - 1);
      break;
    case "ArrowUp":
      e.preventDefault();
      highlightIndex.value = Math.max(highlightIndex.value - 1, 0);
      break;
    case "Enter":
      e.preventDefault();
      if (highlightIndex.value >= 0 && highlightIndex.value < options.value.length) {
        selectOption(options.value[highlightIndex.value]);
      }
      break;
    case "Escape":
      showDropdown.value = false;
      highlightIndex.value = -1;
      break;
  }
}

function onClickOutside(e: MouseEvent) {
  if (containerRef.value && !containerRef.value.contains(e.target as Node)) {
    showDropdown.value = false;
  }
}

onMounted(() => document.addEventListener("click", onClickOutside));
onBeforeUnmount(() => document.removeEventListener("click", onClickOutside));
</script>

<template>
  <div ref="containerRef">
    <label class="block text-[12px] font-medium text-text-muted mb-1">
      {{ field.label || field.fieldname }}
      <span v-if="field.reqd" class="text-danger">*</span>
    </label>
    <div class="relative">
      <div class="relative">
        <svg class="absolute left-2 top-1/2 -translate-y-1/2 w-3 h-3 text-text-light pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <path stroke-linecap="round" stroke-linejoin="round" d="M13.19 8.688a4.5 4.5 0 0 1 1.242 7.244l-4.5 4.5a4.5 4.5 0 0 1-6.364-6.364l1.757-1.757m9.86-2.439a4.5 4.5 0 0 0-1.242-7.244l-4.5-4.5a4.5 4.5 0 0 0-6.364 6.364L4.34 8.374" />
        </svg>
        <input
          type="text"
          :value="inputValue"
          :disabled="readOnly"
          :placeholder="`Select ${field.options || ''}...`"
          autocomplete="off"
          class="w-full h-8 pl-7 pr-2.5 text-[13px] border border-border rounded-md bg-white text-text placeholder-text-light focus:outline-none focus:ring-1 focus:ring-primary-500/30 focus:border-primary-400 disabled:bg-surface-raised disabled:text-text-muted disabled:cursor-not-allowed transition-colors"
          @input="onInput"
          @focus="onFocus"
          @blur="onBlur"
          @keydown="onKeydown"
        />
      </div>

      <!-- Dropdown -->
      <div
        v-if="showDropdown && options.length > 0"
        class="absolute z-50 mt-0.5 w-full bg-white border border-border rounded-md shadow-md max-h-40 overflow-y-auto"
      >
        <button
          v-for="(option, i) in options"
          :key="option"
          type="button"
          :class="[
            'w-full text-left px-2.5 py-1.5 text-[13px] transition-colors',
            i === highlightIndex
              ? 'bg-primary-50 text-primary-700'
              : 'text-text hover:bg-surface-muted',
            option === String(modelValue ?? '') ? 'font-medium' : '',
          ]"
          @mousedown.prevent="selectOption(option)"
          @mouseenter="highlightIndex = i"
        >
          {{ option }}
        </button>
      </div>

      <!-- No results -->
      <div
        v-if="showDropdown && options.length === 0 && inputValue"
        class="absolute z-50 mt-0.5 w-full bg-white border border-border rounded-md shadow-md px-2.5 py-1.5 text-[13px] text-text-muted"
      >
        No results
      </div>
    </div>
    <p v-if="field.description" class="mt-1 text-[11px] text-text-light leading-relaxed">
      {{ field.description }}
    </p>
  </div>
</template>
