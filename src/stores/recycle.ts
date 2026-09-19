import { defineStore } from "pinia";
import { ref } from "vue";

// Kept in memory only. The temporary QQ signature is never persisted to disk.
export const useRecycleSessionStore = defineStore("recycle-session", () => {
  const pwd2sig = ref("");
  const ownerUin = ref("");

  function setVerified(token: string, uin: string) {
    pwd2sig.value = token;
    ownerUin.value = uin;
  }

  function clear() {
    pwd2sig.value = "";
    ownerUin.value = "";
  }

  return { pwd2sig, ownerUin, setVerified, clear };
});
