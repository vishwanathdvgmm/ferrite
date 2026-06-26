document.addEventListener('DOMContentLoaded', () => {
    // ---------------------------------------------
    // 1. SPA ROUTING ENGINE (SUBDOMAINS)
    // ---------------------------------------------
    const pages = document.querySelectorAll('.page');
    const navLinks = document.querySelectorAll('.nav-item, .logo, .footer-nav-link');
    const navLinksContainer = document.querySelector('.nav-links');
    const navToggle = document.querySelector('.nav-toggle');

    // Menu toggle for mobile
    if (navToggle) {
        navToggle.addEventListener('click', () => {
            navLinksContainer.classList.toggle('open');
        });
    }

    function switchPage(targetPageId) {
        // Hide mobile menu if open
        if (navLinksContainer) {
            navLinksContainer.classList.remove('open');
        }

        // Deactivate all pages and activate the chosen one
        let matched = false;
        pages.forEach(page => {
            if (page.id === `page-${targetPageId}`) {
                page.classList.add('active');
                matched = true;
            } else {
                page.classList.remove('active');
            }
        });

        if (!matched && targetPageId === 'home') {
            document.getElementById('page-home').classList.add('active');
        }

        // Update active class on navbar items
        navLinks.forEach(link => {
            if (link.getAttribute('data-target') === targetPageId) {
                link.classList.add('active-nav');
            } else {
                link.classList.remove('active-nav');
            }
        });

        // Scroll to top smoothly on page transition
        window.scrollTo({ top: 0, behavior: 'smooth' });
    }

    // Hash listener
    function handleHashChange() {
        const hash = window.location.hash.replace('#', '') || 'home';
        switchPage(hash);
    }

    window.addEventListener('hashchange', handleHashChange);
    
    // Initial routing load
    handleHashChange();

    // ---------------------------------------------
    // 2. INTERACTIVE PLAYGROUND (play.ferrite-lang.org)
    // ---------------------------------------------
    const codeEditor = document.getElementById('code-editor');
    const editorLines = document.getElementById('editor-lines');
    const consoleOutput = document.getElementById('console-output');
    const btnRun = document.getElementById('btn-run');
    const btnClear = document.getElementById('btn-clear');
    const exampleItems = document.querySelectorAll('.example-item');

    // Templates map
    const codeTemplates = {
        hello: 'println("Hello, Ferrite!");',
        tensors: `import "math";\n\nparam inputs: Tensor<float, (1, 4)> = rand();\nparam weights: Tensor<float, (4, 2)> = ones();\n\ninfer {\n    keep outputs = inputs @ weights;\n    println("Inputs:  " + str(inputs));\n    println("Outputs: " + str(outputs));\n}`,
        matching: `enum Device {\n    Cpu;\n    Gpu(int);\n}\n\nkeep current = Gpu(1);\n\nmatch current {\n    case Cpu => {\n        println("Running on Host CPU");\n    }\n    case Gpu(id) if id == 0 => {\n        println("Primary GPU active");\n    }\n    case Gpu(id) => {\n        println("Secondary GPU (ID: " + str(id) + ")");\n    }\n}`,
        traits: `group Point {\n    x: float;\n    y: float;\n}\n\ntrait Scale {\n    fun scale(self, factor: float) -> Point;\n}\n\nimpl Scale for Point {\n    fun scale(self, factor: float) -> Point {\n        return Point {\n            x: self.x * factor,\n            y: self.y * factor\n        };\n    }\n}\n\nkeep p = Point { x: 1.5, y: 2.0 };\nkeep scaled = p.scale(2.0);\nprintln("Scaled: (" + str(scaled.x) + ", " + str(scaled.y) + ")");`,
        closures: `keep base = 50;\nkeep offset_func = (x: int) => x + base;\n\nkeep i = 0;\nwhile i < 5 {\n    i = i + 1;\n    if i == 2 { skip; }\n    println("Offset i=" + str(i) + ": " + str(offset_func(i)));\n}`
    };

    // Pre-configured expected simulator logs
    const outputLogs = {
        hello: `✓ Type check passed\nExecuting...\n\nHello, Ferrite!`,
        tensors: `✓ Dynamic shape matches verified: (1, 4) @ (4, 2) => (1, 2)\nExecuting...\n\nInputs:  [[0.384, 0.912, 0.518, 0.207]]\nOutputs: [[2.021, 2.021]]`,
        matching: `✓ Pattern match exhaustive verification success\nExecuting...\n\nSecondary GPU (ID: 1)`,
        traits: `✓ Static type implementation bindings succeeded\nExecuting...\n\nScaled: (3.0, 4.0)`,
        closures: `✓ Lexical capture checks cleared\nExecuting...\n\nOffset i=1: 51\nOffset i=3: 53\nOffset i=4: 54\nOffset i=5: 55`
    };

    // Keep line numbers aligned
    function updateLineNumbers() {
        if (!codeEditor || !editorLines) return;
        const lines = codeEditor.value.split('\n').length;
        let numbersHTML = '';
        for (let i = 1; i <= lines; i++) {
            numbersHTML += `${i}<br>`;
        }
        editorLines.innerHTML = numbersHTML;
    }

    if (codeEditor) {
        codeEditor.addEventListener('input', updateLineNumbers);
    }

    // Switch playground script examples
    exampleItems.forEach(item => {
        item.addEventListener('click', () => {
            exampleItems.forEach(i => i.classList.remove('active'));
            item.classList.add('active');

            const scriptType = item.getAttribute('data-example');
            if (codeTemplates[scriptType]) {
                codeEditor.value = codeTemplates[scriptType];
                updateLineNumbers();
                consoleOutput.innerHTML = `<span class="comment">// Editor switched to example ${item.innerText}.\n// Press 'Run Code' to execute.</span>`;
            }
        });
    });

    // Run compiler simulator
    if (btnRun) {
        btnRun.addEventListener('click', () => {
            consoleOutput.innerHTML = `<span style="color: var(--accent-primary)">Analyzing script and resolving imports...</span>\n`;
            
            setTimeout(() => {
                consoleOutput.innerHTML += `<span style="color: var(--syn-type)">Running syntax checks and resolving Tensor graphs...</span>\n`;
                
                setTimeout(() => {
                    const currentCode = codeEditor.value.trim();
                    let responseOutput = '';
                    
                    // Simple logic to match code to templates
                    if (currentCode.includes('println("Hello, Ferrite!");')) {
                        responseOutput = outputLogs.hello;
                    } else if (currentCode.includes('Tensor<float, (1, 4)>')) {
                        responseOutput = outputLogs.tensors;
                    } else if (currentCode.includes('case Gpu(id)')) {
                        responseOutput = outputLogs.matching;
                    } else if (currentCode.includes('impl Scale for Point')) {
                        responseOutput = outputLogs.traits;
                    } else if (currentCode.includes('offset_func')) {
                        responseOutput = outputLogs.closures;
                    } else {
                        // User modified the code
                        // Basic local print statement interpreter!
                        const matches = currentCode.match(/println\((["'])(.*?)\1\);/g);
                        if (matches && matches.length > 0) {
                            let parsedLogs = '';
                            matches.forEach(m => {
                                const cleanVal = m.replace(/println\((["'])/, '').replace(/(["'])\);/, '');
                                parsedLogs += cleanVal + '\n';
                            });
                            responseOutput = `✓ Dynamic validation passed\nExecuting...\n\n${parsedLogs.trim()}`;
                        } else {
                            responseOutput = `✓ Type check passed.\nExecuting...\n\n[Process terminated with code 0]`;
                        }
                    }
                    consoleOutput.innerHTML = responseOutput;
                }, 600);
            }, 400);
        });
    }

    if (btnClear) {
        btnClear.addEventListener('click', () => {
            consoleOutput.innerHTML = `<span class="comment">// Output cleared.</span>`;
        });
    }

    // Initialize lines
    updateLineNumbers();

    // ---------------------------------------------
    // 3. DOCUMENTATION VIEWER (docs.ferrite-lang.org)
    // ---------------------------------------------
    const docsLinks = document.querySelectorAll('.docs-link');
    const docsSections = document.querySelectorAll('.docs-section-content');

    docsLinks.forEach(link => {
        link.addEventListener('click', (e) => {
            e.preventDefault();
            
            // Deactivate links and activate target
            docsLinks.forEach(l => l.classList.remove('active'));
            link.classList.add('active');

            const targetSectionId = link.getAttribute('href').replace('#', '');
            docsSections.forEach(section => {
                if (section.id === targetSectionId) {
                    section.classList.add('active');
                } else {
                    section.classList.remove('active');
                }
            });

            // Scroll Docs pane back to top
            const docsContentPane = document.querySelector('.docs-content');
            if (docsContentPane) {
                docsContentPane.scrollTo({ top: 0, behavior: 'smooth' });
            }
        });
    });

    // ---------------------------------------------
    // 4. PACKAGES SEARCH REGISTRY (packages.ferrite-lang.org)
    // ---------------------------------------------
    const packageSearch = document.getElementById('package-search');
    const packageCards = document.querySelectorAll('.pkg-card');

    if (packageSearch) {
        packageSearch.addEventListener('input', (e) => {
            const query = e.target.value.toLowerCase().trim();
            
            packageCards.forEach(card => {
                const pkgName = card.querySelector('.pkg-name').innerText.toLowerCase();
                const pkgDesc = card.querySelector('.pkg-desc').innerText.toLowerCase();
                
                if (pkgName.includes(query) || pkgDesc.includes(query)) {
                    card.style.display = 'block';
                } else {
                    card.style.display = 'none';
                }
            });
        });
    }

    // ---------------------------------------------
    // 5. BLOG ARTICLES "READ MORE" ACTION
    // ---------------------------------------------
    const readMoreButtons = document.querySelectorAll('.btn-read-more');

    readMoreButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            const post = btn.closest('.blog-post');
            const fullContent = post.querySelector('.post-content-full');
            
            if (fullContent) {
                const isOpen = fullContent.classList.toggle('open');
                btn.innerText = isOpen ? 'Read Less' : 'Read More';
            }
        });
    });
});
