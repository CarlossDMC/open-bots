// Applies the stored theme before the first paint so the window never flashes the wrong theme.
// Keep the storage key and fallback in sync with src/lib/theme.ts (covered by theme-init.test.ts).
(function () {
  var preference = "dark";
  try {
    var stored = window.localStorage.getItem("open-bots.theme");
    if (stored === "system" || stored === "light" || stored === "dark") preference = stored;
  } catch {
    // Storage can be unavailable; keep the default theme.
  }
  var systemDark =
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches;
  var dark = preference === "dark" || (preference === "system" && systemDark);
  document.documentElement.classList.toggle("dark", dark);
  document.documentElement.style.colorScheme = dark ? "dark" : "light";
})();
