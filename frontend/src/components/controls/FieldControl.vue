<script setup lang="ts">
import type { DocFieldMeta } from "@/utils/call";
import LinkControl from "./LinkControl.vue";
import TableControl from "./TableControl.vue";
import AttachControl from "./AttachControl.vue";

const props = defineProps<{
  field: DocFieldMeta;
  modelValue: unknown;
  readOnly?: boolean;
  computedReqd?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: unknown];
}>();

function onInput(event: Event) {
  const target = event.target as HTMLInputElement;
  const { fieldtype } = props.field;

  if (fieldtype === "Int") {
    emit("update:modelValue", target.value ? parseInt(target.value) : null);
  } else if (
    fieldtype === "Float" ||
    fieldtype === "Currency" ||
    fieldtype === "Percent"
  ) {
    emit("update:modelValue", target.value ? parseFloat(target.value) : null);
  } else if (fieldtype === "Check") {
    emit("update:modelValue", (target as HTMLInputElement).checked ? 1 : 0);
  } else {
    emit("update:modelValue", target.value);
  }
}

function inputType(): string {
  switch (props.field.fieldtype) {
    case "Int":
    case "Float":
    case "Currency":
    case "Percent":
      return "number";
    case "Date":
      return "date";
    case "Datetime":
      return "datetime-local";
    case "Time":
      return "time";
    case "Password":
      return "password";
    default:
      return "text";
  }
}

function isTextArea(): boolean {
  return ["Text", "SmallText", "LongText", "Code", "JSON", "TextEditor", "HTMLEditor"].includes(
    props.field.fieldtype,
  );
}

function isSelect(): boolean {
  return props.field.fieldtype === "Select";
}

function isCheck(): boolean {
  return props.field.fieldtype === "Check";
}

function isLink(): boolean {
  return props.field.fieldtype === "Link";
}

function isColor(): boolean {
  return props.field.fieldtype === "Color";
}

function isTable(): boolean {
  return props.field.fieldtype === "Table";
}

function isAttach(): boolean {
  return props.field.fieldtype === "Attach" || props.field.fieldtype === "AttachImage";
}

function isDynamicLink(): boolean {
  return props.field.fieldtype === "DynamicLink";
}

function selectOptions(): string[] {
  if (!props.field.options) return [];
  return props.field.options.split("\n").filter((o) => o.trim());
}

function isRequired(): boolean {
  return props.computedReqd ?? props.field.reqd ?? false;
}

const inputClass = "w-full h-8 px-2.5 text-[13px] border border-border rounded-md bg-white text-text placeholder-text-light focus:outline-none focus:ring-1 focus:ring-primary-500/30 focus:border-primary-400 disabled:bg-surface-raised disabled:text-text-muted disabled:cursor-not-allowed transition-colors";
</script>

<template>
  <div>
    <!-- Link field -->
    <LinkControl
      v-if="isLink()"
      :field="field"
      :model-value="modelValue"
      :read-only="readOnly"
      @update:model-value="emit('update:modelValue', $event)"
    />

    <!-- Table (child table) -->
    <TableControl
      v-else-if="isTable()"
      :field="field"
      :model-value="modelValue"
      :read-only="readOnly"
      @update:model-value="emit('update:modelValue', $event)"
    />

    <!-- Attach / AttachImage -->
    <AttachControl
      v-else-if="isAttach()"
      :field="field"
      :model-value="modelValue"
      :read-only="readOnly"
      :is-image="field.fieldtype === 'AttachImage'"
      @update:model-value="emit('update:modelValue', $event)"
    />

    <!-- DynamicLink — renders as a text input (the target DocType comes from another field) -->
    <template v-else-if="isDynamicLink()">
      <label class="block text-[12px] font-medium text-text-muted mb-1">
        {{ field.label || field.fieldname }}
        <span v-if="isRequired()" class="text-danger">*</span>
      </label>
      <input
        type="text"
        :value="(modelValue as string) ?? ''"
        :disabled="readOnly"
        :placeholder="field.options ? `Link to ${field.options}` : 'Dynamic Link'"
        :class="inputClass"
        @input="onInput"
      />
    </template>

    <!-- Check / Boolean -->
    <label v-else-if="isCheck()" class="inline-flex items-center gap-2 cursor-pointer select-none group py-0.5">
      <div class="relative">
        <input
          type="checkbox"
          :checked="!!modelValue"
          :disabled="readOnly"
          class="peer sr-only"
          @change="onInput"
        />
        <div class="w-4 h-4 rounded border border-border-strong bg-white peer-checked:bg-primary-600 peer-checked:border-primary-600 peer-focus-visible:ring-1 peer-focus-visible:ring-primary-500/30 transition-colors flex items-center justify-center">
          <svg class="w-2.5 h-2.5 text-white opacity-0 peer-checked:opacity-100" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
            <path stroke-linecap="round" stroke-linejoin="round" d="M4.5 12.75l6 6 9-13.5" />
          </svg>
        </div>
      </div>
      <span class="text-[13px] text-text">
        {{ field.label || field.fieldname }}
        <span v-if="isRequired()" class="text-danger text-[11px]">*</span>
      </span>
    </label>

    <!-- Other fields -->
    <template v-else>
      <label class="block text-[12px] font-medium text-text-muted mb-1">
        {{ field.label || field.fieldname }}
        <span v-if="isRequired()" class="text-danger">*</span>
      </label>

      <!-- Select -->
      <div v-if="isSelect()" class="relative">
        <select
          :value="(modelValue as string) ?? ''"
          :disabled="readOnly"
          :class="[inputClass, 'appearance-none pr-7']"
          @change="onInput"
        >
          <option value="" class="text-text-light">Select...</option>
          <option v-for="opt in selectOptions()" :key="opt" :value="opt">
            {{ opt }}
          </option>
        </select>
        <svg class="absolute right-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-light pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" />
        </svg>
      </div>

      <!-- Color -->
      <div v-else-if="isColor()" class="flex items-center gap-1.5">
        <input
          type="color"
          :value="(modelValue as string) || '#000000'"
          :disabled="readOnly"
          class="w-8 h-8 p-0.5 border border-border rounded-md cursor-pointer disabled:cursor-not-allowed"
          @input="onInput"
        />
        <input
          type="text"
          :value="(modelValue as string) ?? ''"
          :disabled="readOnly"
          placeholder="#000000"
          :class="[inputClass, 'font-mono']"
          @input="onInput"
        />
      </div>

      <!-- Code / JSON textarea -->
      <textarea
        v-else-if="field.fieldtype === 'Code' || field.fieldtype === 'JSON'"
        :value="(modelValue as string) ?? ''"
        :disabled="readOnly"
        rows="8"
        spellcheck="false"
        class="w-full px-3 py-2 text-[12px] font-mono leading-5 border border-border rounded-md bg-gray-50 text-text placeholder-text-light focus:outline-none focus:ring-1 focus:ring-primary-500/30 focus:border-primary-400 disabled:bg-surface-raised disabled:text-text-muted disabled:cursor-not-allowed transition-colors resize-y"
        @input="onInput"
      />

      <!-- Regular textarea -->
      <textarea
        v-else-if="isTextArea()"
        :value="(modelValue as string) ?? ''"
        :disabled="readOnly"
        rows="3"
        class="w-full px-2.5 py-1.5 text-[13px] border border-border rounded-md bg-white text-text placeholder-text-light focus:outline-none focus:ring-1 focus:ring-primary-500/30 focus:border-primary-400 disabled:bg-surface-raised disabled:text-text-muted disabled:cursor-not-allowed transition-colors resize-y"
        @input="onInput"
      />

      <!-- Standard input -->
      <input
        v-else
        :type="inputType()"
        :value="(modelValue as string) ?? ''"
        :disabled="readOnly"
        :placeholder="field.description || ''"
        :step="field.fieldtype === 'Float' || field.fieldtype === 'Currency' || field.fieldtype === 'Percent' ? 'any' : undefined"
        :class="inputClass"
        @input="onInput"
      />

      <p v-if="field.description" class="mt-1 text-[11px] text-text-light leading-relaxed">
        {{ field.description }}
      </p>
    </template>
  </div>
</template>
