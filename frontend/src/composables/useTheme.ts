import { ref } from "vue";

export interface Theme {
  brand_name: string;
  logo_url: string;
  primary_color: string;
  font_family: string;
  radius: string;
}

const theme = ref<Theme>({
  brand_name: "Loom",
  logo_url: "",
  primary_color: "#4f46e5",
  font_family: "Inter",
  radius: "0.375rem",
});

const loaded = ref(false);

function applyTheme(t: Theme) {
  const root = document.documentElement.style;

  if (t.primary_color) {
    // Generate shades from the primary color using HSL manipulation
    const hex = t.primary_color;
    root.setProperty("--color-primary-600", hex);

    // Compute lighter/darker variants
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);

    const lighten = (amt: number) => {
      const lr = Math.min(255, r + (255 - r) * amt);
      const lg = Math.min(255, g + (255 - g) * amt);
      const lb = Math.min(255, b + (255 - b) * amt);
      return `#${Math.round(lr).toString(16).padStart(2, "0")}${Math.round(lg).toString(16).padStart(2, "0")}${Math.round(lb).toString(16).padStart(2, "0")}`;
    };
    const darken = (amt: number) => {
      const dr = Math.max(0, Math.round(r * (1 - amt)));
      const dg = Math.max(0, Math.round(g * (1 - amt)));
      const db = Math.max(0, Math.round(b * (1 - amt)));
      return `#${dr.toString(16).padStart(2, "0")}${dg.toString(16).padStart(2, "0")}${db.toString(16).padStart(2, "0")}`;
    };

    root.setProperty("--color-primary-50", lighten(0.92));
    root.setProperty("--color-primary-100", lighten(0.84));
    root.setProperty("--color-primary-200", lighten(0.7));
    root.setProperty("--color-primary-300", lighten(0.55));
    root.setProperty("--color-primary-400", lighten(0.35));
    root.setProperty("--color-primary-500", lighten(0.15));
    root.setProperty("--color-primary-700", darken(0.15));
    root.setProperty("--color-primary-800", darken(0.3));
    root.setProperty("--color-primary-900", darken(0.45));
  }

  if (t.font_family) {
    root.setProperty("--font-family", `"${t.font_family}", ui-sans-serif, system-ui, sans-serif`);
    document.body.style.fontFamily = `"${t.font_family}", ui-sans-serif, system-ui, sans-serif`;
  }

  if (t.radius) {
    root.setProperty("--radius", t.radius);
  }
}

export function useTheme() {
  async function load() {
    try {
      const res = await fetch("/api/config/theme");
      if (res.ok) {
        const data = await res.json();
        if (data.data) {
          theme.value = { ...theme.value, ...data.data };
          applyTheme(theme.value);
        }
      }
    } catch {
      // Use defaults
    } finally {
      loaded.value = true;
    }
  }

  async function save(newTheme: Partial<Theme>) {
    theme.value = { ...theme.value, ...newTheme };
    applyTheme(theme.value);
    await fetch("/api/config/theme", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      credentials: "include",
      body: JSON.stringify(theme.value),
    });
  }

  return { theme, loaded, load, save };
}
