import { ref } from "vue";

export interface SavedView {
  name: string;
  is_default?: boolean;
  filters: unknown[];
  columns: string[];
  sort_field: string;
  sort_order: string;
}

export function useListView(doctype: () => string) {
  const views = ref<SavedView[]>([]);
  const activeView = ref<string>("");
  const loaded = ref(false);

  function settingsKey() {
    return `list_view:${doctype()}`;
  }

  async function load() {
    try {
      const res = await fetch(`/api/settings/${encodeURIComponent(settingsKey())}`, {
        credentials: "include",
      });
      if (res.ok) {
        const data = await res.json();
        views.value = data.data?.views || [];
        const defaultView = views.value.find((v) => v.is_default);
        if (defaultView) activeView.value = defaultView.name;
      }
    } catch {
      views.value = [];
    } finally {
      loaded.value = true;
    }
  }

  async function saveViews() {
    try {
      await fetch(`/api/settings/${encodeURIComponent(settingsKey())}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ views: views.value }),
      });
    } catch {
      // Silent fail
    }
  }

  function addView(view: SavedView) {
    views.value = [...views.value, view];
    activeView.value = view.name;
    saveViews();
  }

  function removeView(name: string) {
    views.value = views.value.filter((v) => v.name !== name);
    if (activeView.value === name) activeView.value = "";
    saveViews();
  }

  function getActiveView(): SavedView | undefined {
    return views.value.find((v) => v.name === activeView.value);
  }

  function setActiveView(name: string) {
    activeView.value = name;
  }

  return { views, activeView, loaded, load, saveViews, addView, removeView, getActiveView, setActiveView };
}
