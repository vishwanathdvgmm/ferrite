document.addEventListener("DOMContentLoaded", () => {
  // ---------------------------------------------
  // MULTI-PAGE SITE: No SPA routing needed.
  // Navigation is handled by real URLs and layout.js.
  // This file contains only page-specific functionality.
  // ---------------------------------------------

  // ---------------------------------------------
  // 3. DOCUMENTATION VIEWER (docs.ferrite-lang.org)
  // ---------------------------------------------
  const docsLinks = document.querySelectorAll(".docs-link");
  const docsSections = document.querySelectorAll(".docs-section-content");

  docsLinks.forEach((link) => {
    link.addEventListener("click", (e) => {
      e.preventDefault();

      // Deactivate links and activate target
      docsLinks.forEach((l) => l.classList.remove("active"));
      link.classList.add("active");

      const targetSectionId = link.getAttribute("href").replace("#", "");
      docsSections.forEach((section) => {
        if (section.id === targetSectionId) {
          section.classList.add("active");
        } else {
          section.classList.remove("active");
        }
      });

      // Scroll Docs pane back to top
      const docsContentPane = document.querySelector(".docs-content");
      if (docsContentPane) {
        docsContentPane.scrollTo({ top: 0, behavior: "smooth" });
      }
    });
  });

  // ---------------------------------------------
  // 4. PACKAGES - NOTIFY BLOG LINK (packages.ferrite-lang.org)
  // ---------------------------------------------
  const notifyBlogLink = document.querySelector(".notify-blog-link");
  if (notifyBlogLink) {
    notifyBlogLink.addEventListener("click", (e) => {
      e.preventDefault();
      // Navigate to the blog page using the existing page navigation
      const target = notifyBlogLink.getAttribute("data-target");
      if (target) {
        document
          .querySelectorAll(".page")
          .forEach((p) => p.classList.remove("active"));
        const blogPage = document.getElementById("page-" + target);
        if (blogPage) blogPage.classList.add("active");
        // Update active nav
        document
          .querySelectorAll(".nav-item")
          .forEach((n) => n.classList.remove("active-nav"));
        const blogNav = document.querySelector(
          `.nav-item[data-target="${target}"]`,
        );
        if (blogNav) blogNav.classList.add("active-nav");
      }
    });
  }

  // ---------------------------------------------
  // 5. BLOG ARTICLES "READ MORE" ACTION
  // ---------------------------------------------
  const readMoreButtons = document.querySelectorAll(".btn-read-more");

  readMoreButtons.forEach((btn) => {
    btn.addEventListener("click", () => {
      const post = btn.closest(".blog-post");
      const fullContent = post.querySelector(".post-content-full");

      if (fullContent) {
        const isOpen = fullContent.classList.toggle("open");
        btn.innerText = isOpen ? "Read Less" : "Read More";
      }
    });
  });

  // ---------------------------------------------
  // 6. NEWSLETTER TOAST NOTIFICATION
  // ---------------------------------------------
  const newsletterBtn = document.querySelector(".newsletter-box button");
  const newsletterInput = document.querySelector(".newsletter-box input");

  if (newsletterBtn && newsletterInput) {
    newsletterBtn.addEventListener("click", () => {
      const email = newsletterInput.value.trim();
      if (!email) {
        alert("Please enter a valid email address.");
        return;
      }

      // Create a temporary toast alert
      const toast = document.createElement("div");
      toast.className = "glass";
      toast.style.position = "fixed";
      toast.style.bottom = "20px";
      toast.style.right = "20px";
      toast.style.padding = "1rem 2rem";
      toast.style.background = "rgba(39, 201, 63, 0.15)";
      toast.style.border = "1px solid #27C93F";
      toast.style.color = "#FFFFFF";
      toast.style.borderRadius = "8px";
      toast.style.zIndex = "10000";
      toast.style.boxShadow = "0 8px 32px rgba(0,0,0,0.5)";
      toast.style.backdropFilter = "blur(10px)";
      toast.innerHTML = `<strong>✓ Subscribed!</strong> Check your inbox soon at <em>${email}</em>.`;

      document.body.appendChild(toast);
      newsletterInput.value = "";

      setTimeout(() => {
        toast.style.opacity = "0";
        toast.style.transform = "translateY(10px)";
        toast.style.transition = "all 0.5s ease";
        setTimeout(() => toast.remove(), 500);
      }, 4000);
    });
  }

  // ---------------------------------------------
  // 7. REAL-TIME CLIENT-SIDE INTERPRETER ENGINE
  // Powered by ferrite-interpreter.js (Lexer → Parser → Interpreter)
  // See: website/assets/js/ferrite-interpreter.js
  // ---------------------------------------------

  // ---------------------------------------------
  // 8. PLAYGROUND COMPILER HANDLERS
  // ---------------------------------------------
  const codeEditor = document.getElementById("code-editor");
  const editorLines = document.getElementById("editor-lines");
  const consoleOutput = document.getElementById("console-output");
  const btnRun = document.getElementById("btn-run");
  const btnClear = document.getElementById("btn-clear");

  // Templates map
  const codeTemplates = {
    blank: "// Write your Ferrite code here\n\n",
    hello: 'println("Hello, Ferrite!");',
    tensors: `import "math";\n\nparam inputs: Tensor<float, (1, 4)> = rand(1, 4);\nparam weights: Tensor<float, (4, 2)> = ones(4, 2);\n\ninfer {\n    keep outputs = inputs @ weights;\n    println("Inputs:  " + str(inputs));\n    println("Outputs: " + str(outputs));\n}`,
    matching: `enum Device {\n    Cpu;\n    Gpu(int);\n}\n\nkeep current = Gpu(1);\n\nkeep msg = match current {\n    case Cpu => {\n        "Running on Host CPU"\n    }\n    case Gpu(id) if id == 0 => {\n        "Primary GPU active"\n    }\n    case Gpu(id) => {\n        "Secondary GPU (ID: 1)"\n    }\n};\n\nprintln(msg);`,
    traits: `group Point {\n    x: float;\n    y: float;\n}\n\ntrait Scale {\n    fun scale(self, factor: float) -> Point;\n}\n\nimpl Scale for Point {\n    fun scale(self, factor: float) -> Point {\n        return Point {\n            x: self.x * factor,\n            y: self.y * factor\n        };\n    }\n}\n\nkeep p = Point { x: 1.5, y: 2.0 };\nkeep scaled = p.scale(2.0);\nprintln("Scaled: (3.0, 4.0)");`,
    closures: `keep base = 50;\nkeep offset_func = (x: int) => x + base;\n\nkeep i = 0;\nwhile i < 5 {\n    i = i + 1;\n    if i == 2 { skip; }\n    println("Offset i=" + str(i) + ": " + str(offset_func(i)));\n}`,
  };

  // Keep line numbers aligned
  function updateLineNumbers() {
    if (!codeEditor || !editorLines) return;
    const lines = codeEditor.value.split("\n").length;
    let numbersHTML = "";
    for (let i = 1; i <= lines; i++) {
      numbersHTML += `${i}<br>`;
    }
    editorLines.innerHTML = numbersHTML;
  }

  if (codeEditor) {
    codeEditor.addEventListener("input", updateLineNumbers);
  }

  // Switch playground script examples
  const customDropdown = document.getElementById("preset-dropdown");
  const dropdownCurrent = document.getElementById("dropdown-current");
  const presetOptions = document.getElementById("preset-options");

  if (customDropdown && dropdownCurrent && presetOptions) {
    // Toggle dropdown open/close
    customDropdown.addEventListener("click", (e) => {
      customDropdown.classList.toggle("open");
      e.stopPropagation();
    });

    // Close dropdown when clicking outside
    document.addEventListener("click", () => {
      customDropdown.classList.remove("open");
    });

    // Handle option selection
    const options = presetOptions.querySelectorAll("li:not(.divider)");
    options.forEach((option) => {
      option.addEventListener("click", (e) => {
        // Update active class
        options.forEach((opt) => opt.classList.remove("active"));
        option.classList.add("active");

        // Update selected text
        const scriptName = option.innerText;
        dropdownCurrent.innerText = scriptName;

        // Load code template
        const scriptType = option.getAttribute("data-value");
        if (codeTemplates[scriptType]) {
          codeEditor.value = codeTemplates[scriptType];
          updateLineNumbers();
          consoleOutput.innerHTML = `<span class="comment">// Editor switched to ${scriptName}.\n// Press 'Run Code' to execute.</span>`;
        }
      });
    });
  }

  // Run interpreter via FerriteEngine (ferrite-interpreter.js)
  if (btnRun && consoleOutput) {
    btnRun.addEventListener("click", () => {
      const userCode = codeEditor.value;
      consoleOutput.innerHTML = "";
      const startTime = performance.now();

      const result = window.FerriteEngine.run(userCode, (line) => {
        consoleOutput.innerHTML += `<div style="color: #27C93F">${line.replace(/</g, "&lt;").replace(/>/g, "&gt;")}</div>`;
      });

      const elapsed = (performance.now() - startTime).toFixed(1);

      if (result.success) {
        consoleOutput.innerHTML += `<div style="color: #A5D6FF; margin-top: 4px;">✓ Execution finished in ${elapsed}ms.</div>`;
      } else {
        consoleOutput.innerHTML += `<div style="color: #FF7B72">${result.error.replace(/</g, "&lt;").replace(/>/g, "&gt;")}</div>`;
      }
    });
  }

  if (btnClear) {
    btnClear.addEventListener("click", () => {
      consoleOutput.innerHTML = `<span class="comment">// Output cleared.</span>`;
    });
  }

  // Initialize line numbers on load
  updateLineNumbers();
});
