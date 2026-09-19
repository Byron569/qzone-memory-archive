const blockedKeys = new Set(["F1"]);
const blockedCtrlKeys = new Set(["+", "-", "=", "0", "u"]);
const blockedDevtoolKeys = new Set(["i", "j", "c"]);

export function installAppGuards() {
  document.addEventListener("contextmenu", (event) => event.preventDefault(), { capture: true });

  document.addEventListener("keydown", (event) => {
    const key = event.key.toLowerCase();
    const blocksZoomOrSource = (event.ctrlKey || event.metaKey) && blockedCtrlKeys.has(key);
    const blocksDevtools = (event.ctrlKey || event.metaKey) && event.shiftKey && blockedDevtoolKeys.has(key);
    if (blockedKeys.has(event.key) || blocksZoomOrSource || blocksDevtools) {
      event.preventDefault();
      event.stopImmediatePropagation();
    }
  }, { capture: true });

  document.addEventListener("wheel", (event) => {
    if (event.ctrlKey || event.metaKey) event.preventDefault();
  }, { capture: true, passive: false });

  document.addEventListener("touchmove", (event) => {
    if (event.touches.length > 1) event.preventDefault();
  }, { capture: true, passive: false });

  document.addEventListener("gesturestart", (event) => event.preventDefault(), { capture: true });
}
