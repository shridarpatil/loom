import { ref } from "vue";

export interface Toast {
  id: number;
  message: string;
  type: "success" | "error" | "warning" | "info";
  removing?: boolean;
}

const toasts = ref<Toast[]>([]);
let nextId = 1;

export function useToast() {
  function show(message: string, type: Toast["type"] = "info", duration = 4000) {
    const id = nextId++;
    toasts.value.push({ id, message, type });
    if (duration > 0) {
      setTimeout(() => remove(id), duration);
    }
  }

  function remove(id: number) {
    const toast = toasts.value.find((t) => t.id === id);
    if (toast) {
      toast.removing = true;
      setTimeout(() => {
        toasts.value = toasts.value.filter((t) => t.id !== id);
      }, 200);
    }
  }

  return {
    toasts,
    show,
    success: (msg: string) => show(msg, "success"),
    error: (msg: string) => show(msg, "error", 6000),
    warning: (msg: string) => show(msg, "warning"),
    info: (msg: string) => show(msg, "info"),
    remove,
  };
}
