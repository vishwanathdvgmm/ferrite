/**
 * Ferrite Shared Layout Loader
 * Fetches header and footer partials and injects them into the page.
 * Also highlights the current page in the navigation.
 */
document.addEventListener("DOMContentLoaded", async () => {
  const VERSION = "3";

  // Load header
  const headerSlot = document.getElementById("header-slot");
  if (headerSlot) {
    try {
      const res = await fetch(`/partials/header.html?v=${VERSION}`);
      if (res.ok) {
        headerSlot.innerHTML = await res.text();
        highlightActiveNav();
        initMobileToggle();
      }
    } catch (e) {
      console.warn("Failed to load header partial:", e);
    }
  }

  // Load footer
  const footerSlot = document.getElementById("footer-slot");
  if (footerSlot) {
    try {
      const res = await fetch(`/partials/footer.html?v=${VERSION}`);
      if (res.ok) {
        footerSlot.innerHTML = await res.text();
      }
    } catch (e) {
      console.warn("Failed to load footer partial:", e);
    }
  }
});

/**
 * Highlight the active nav link based on the current URL path.
 */
function highlightActiveNav() {
  const path = window.location.pathname.replace(/\/+$/, "") || "/";
  const navItems = document.querySelectorAll(".nav-item, .logo");

  navItems.forEach((link) => {
    const href = link.getAttribute("href");
    if (!href) return;

    const linkPath = href.replace(/\/+$/, "") || "/";
    if (path === linkPath) {
      link.classList.add("active-nav");
    } else {
      link.classList.remove("active-nav");
    }
  });
}

/**
 * Initialize mobile navigation toggle.
 */
function initMobileToggle() {
  const navToggle = document.querySelector(".nav-toggle");
  const navLinksContainer = document.querySelector(".nav-links");

  if (navToggle && navLinksContainer) {
    navToggle.addEventListener("click", () => {
      navLinksContainer.classList.toggle("open");
    });
  }
}
