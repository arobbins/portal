const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
for (const input of document.querySelectorAll("[data-detect-time-zone]")) {
  input.value = zone;
}
