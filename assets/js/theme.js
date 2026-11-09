const THEMES = ["light", "dark", "system"];

function themeFromCookie() {
  const match = document.cookie.match(/(?:^|;\s*)theme=([a-z]+)/);
  return match && THEMES.includes(match[1]) ? match[1] : null;
}

function applyTheme(theme) {
  document.documentElement.dataset.theme = theme;
  document.querySelector("meta[name=color-scheme]").content =
    theme === "system" ? "light dark" : theme;
  for (const option of document.querySelectorAll(".a-theme-switcher__option")) {
    option.setAttribute("aria-pressed", String(option.value === theme));
  }
}

function syncThemeFromCookie() {
  const theme = themeFromCookie();
  if (theme) applyTheme(theme);
}

window.setTheme = (theme) => {
  if (!THEMES.includes(theme)) return;
  document.cookie = `theme=${theme}; path=/; max-age=31536000; samesite=lax`;
  applyTheme(theme);
};

syncThemeFromCookie();
document.addEventListener("DOMContentLoaded", syncThemeFromCookie);
window.addEventListener("pageshow", (event) => {
  if (event.persisted) syncThemeFromCookie();
});
