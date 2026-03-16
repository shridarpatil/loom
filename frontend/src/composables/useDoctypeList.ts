import { ref } from "vue";
import { loom } from "@/utils/call";

const doctypes = ref<string[]>([]);
const loading = ref(true);

async function refresh() {
  loading.value = true;
  try {
    const res = await loom.resource("DocType").getList({
      fields: ["name", "module"],
      order_by: "name asc",
      limit: 100,
    });
    doctypes.value = (res.data as Array<{ name: string }>)
      .map((d) => d.name)
      .sort();
  } catch {
    doctypes.value = [];
  } finally {
    loading.value = false;
  }
}

// Initial load
refresh();

export function useDoctypeList() {
  return { doctypes, loading, refresh };
}
