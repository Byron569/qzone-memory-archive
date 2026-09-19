import { defineStore } from "pinia";
import { computed, ref } from "vue";

export const useAppStore = defineStore("app", () => {
  const darkMode = ref(false);
  const sidebarCollapsed = ref(true);
  const themeIcon = computed(() => (darkMode.value ? "pi pi-sun" : "pi pi-moon"));

  function toggleTheme() {
    darkMode.value = !darkMode.value;
  }

  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value;
  }

  return { darkMode, sidebarCollapsed, themeIcon, toggleTheme, toggleSidebar };
});
