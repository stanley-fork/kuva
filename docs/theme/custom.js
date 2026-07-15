// kuva docs — sidebar category collapse.
//
// mdBook's built-in expand/collapse (the `expanded` class toggled by
// toc.js) only applies to *nested* chapters, i.e. a chapter with its own
// `<ol class="section">` of sub-pages (like "kuva CLI" and its 56
// subcommands). The `# Category` part-titles introduced by the SUMMARY.md
// reorg are flat siblings in the same list with no nesting to hook into, so
// there's nothing built-in to toggle. This adds that behaviour: each
// part-title becomes a clickable header that shows/hides the chapter-items
// between it and the next part-title.
//
// Categories start collapsed, except the one containing the current page
// (so navigating to a plot page doesn't hide it), and previously-opened
// categories are remembered across page loads via localStorage so browsing
// several pages in the same category doesn't require re-opening it each time.
(function () {
  const STORAGE_KEY = "kuva-sidebar-open-categories";

  function initCategoryCollapse() {
    const sidebar = document.querySelector("mdbook-sidebar-scrollbox");
    if (!sidebar) {
      return;
    }
    const chapterList = sidebar.querySelector("ol.chapter");
    if (!chapterList || chapterList.dataset.kuvaInit === "true") {
      return;
    }
    chapterList.dataset.kuvaInit = "true";

    const groups = [];
    let currentGroup = null;

    Array.from(chapterList.children).forEach((li) => {
      if (li.classList.contains("part-title")) {
        currentGroup = { title: li.textContent.trim(), titleLi: li, items: [] };
        groups.push(currentGroup);
        li.classList.add("kuva-toggle");
      } else if (
        currentGroup &&
        li.classList.contains("chapter-item") &&
        li.querySelector("a")
      ) {
        currentGroup.items.push(li);
      }
    });

    let openState = {};
    try {
      openState = JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}");
    } catch (e) {
      openState = {};
    }

    function setGroupOpen(group, open, persist) {
      group.items.forEach((li) => li.classList.toggle("kuva-collapsed", !open));
      group.titleLi.classList.toggle("kuva-open", open);
      if (persist) {
        openState[group.title] = open;
        try {
          localStorage.setItem(STORAGE_KEY, JSON.stringify(openState));
        } catch (e) {
          // localStorage unavailable (private browsing, etc.) — collapse
          // state just won't persist across page loads, which is fine.
        }
      }
    }

    groups.forEach((group) => {
      const hasActivePage = group.items.some((li) => li.querySelector("a.active"));
      const open = hasActivePage || openState[group.title] === true;
      setGroupOpen(group, open, false);

      group.titleLi.addEventListener("click", () => {
        setGroupOpen(group, !group.titleLi.classList.contains("kuva-open"), true);
      });
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initCategoryCollapse);
  } else {
    initCategoryCollapse();
  }
})();
