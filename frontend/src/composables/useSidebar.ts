import { ref } from "vue";

export interface SidebarItem {
  name: string;
  route: string;
  module?: string;
  icon?: string;
}

export interface SidebarSection {
  label: string;
  items: SidebarItem[];
  collapsed?: boolean;
}

const sections = ref<SidebarSection[]>([]);
const pinnedItems = ref<SidebarItem[]>([]);
const hiddenItems = ref<Set<string>>(new Set());
const loading = ref(false);
const loaded = ref(false);

export function useSidebar() {
  async function load() {
    if (loaded.value) return;
    loading.value = true;
    try {
      const res = await fetch("/api/sidebar", { credentials: "include" });
      if (res.ok) {
        const data = await res.json();
        sections.value = data.data?.sections || [];
        pinnedItems.value = data.data?.pinned || [];
        hiddenItems.value = new Set(data.data?.hidden || []);
      }
    } catch {
      // Fallback: empty sidebar
    } finally {
      loading.value = false;
      loaded.value = true;
    }
  }

  async function refresh() {
    loaded.value = false;
    await load();
  }

  async function pin(item: SidebarItem) {
    if (pinnedItems.value.find((p) => p.name === item.name)) return;
    pinnedItems.value = [...pinnedItems.value, item];
    await saveUserSidebar();
  }

  async function unpin(name: string) {
    pinnedItems.value = pinnedItems.value.filter((p) => p.name !== name);
    await saveUserSidebar();
  }

  async function hide(name: string) {
    hiddenItems.value.add(name);
    hiddenItems.value = new Set(hiddenItems.value);
    await saveUserSidebar();
  }

  async function unhide(name: string) {
    hiddenItems.value.delete(name);
    hiddenItems.value = new Set(hiddenItems.value);
    await saveUserSidebar();
  }

  async function saveUserSidebar() {
    try {
      await fetch("/api/settings/sidebar", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({
          pinned: pinnedItems.value,
          hidden: Array.from(hiddenItems.value),
        }),
      });
    } catch {
      // Silent fail
    }
  }

  return { sections, pinnedItems, hiddenItems, loading, loaded, load, refresh, pin, unpin, hide, unhide };
}
