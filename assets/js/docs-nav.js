(() => {
  const OFFSET = 120;

  function setUp() {
    const navs = document.querySelectorAll("[data-docs-nav]");
    if (navs.length === 0) return;

    const links = [...document.querySelectorAll("[data-docs-nav] a[href^='#']")];
    const ids = [...new Set(links.map((link) => link.hash.slice(1)))];
    const sections = ids.map((id) => document.getElementById(id)).filter(Boolean);
    if (sections.length === 0) return;

    let frame = 0;
    const update = () => {
      frame = 0;
      const atBottom = window.innerHeight + window.scrollY >= document.body.scrollHeight - 2;
      let current = sections[0];
      for (const section of sections) {
        if (section.getBoundingClientRect().top <= OFFSET) current = section;
      }
      if (atBottom) current = sections[sections.length - 1];
      for (const link of links) {
        if (link.hash === `#${current.id}`) {
          link.setAttribute("aria-current", "location");
        } else {
          link.removeAttribute("aria-current");
        }
      }
    };
    const schedule = () => {
      if (frame === 0) frame = requestAnimationFrame(update);
    };

    window.addEventListener("scroll", schedule, { passive: true });
    window.addEventListener("hashchange", schedule);
    update();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", setUp);
  } else {
    setUp();
  }
})();
