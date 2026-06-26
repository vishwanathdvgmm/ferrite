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

    // ---------------------------------------------
    // 6. NEWSLETTER TOAST NOTIFICATION
    // ---------------------------------------------
    const newsletterBtn = document.querySelector('.newsletter-box button');
    const newsletterInput = document.querySelector('.newsletter-box input');

    if (newsletterBtn && newsletterInput) {
        newsletterBtn.addEventListener('click', () => {
            const email = newsletterInput.value.trim();
            if (!email) {
                alert('Please enter a valid email address.');
                return;
            }
            
            // Create a temporary toast alert
            const toast = document.createElement('div');
            toast.className = 'glass';
            toast.style.position = 'fixed';
            toast.style.bottom = '20px';
            toast.style.right = '20px';
            toast.style.padding = '1rem 2rem';
            toast.style.background = 'rgba(39, 201, 63, 0.15)';
            toast.style.border = '1px solid #27C93F';
            toast.style.color = '#FFFFFF';
            toast.style.borderRadius = '8px';
            toast.style.zIndex = '10000';
            toast.style.boxShadow = '0 8px 32px rgba(0,0,0,0.5)';
            toast.style.backdropFilter = 'blur(10px)';
            toast.innerHTML = `<strong>✓ Subscribed!</strong> Check your inbox soon at <em>${email}</em>.`;
            
            document.body.appendChild(toast);
            newsletterInput.value = '';

            setTimeout(() => {
                toast.style.opacity = '0';
                toast.style.transform = 'translateY(10px)';
                toast.style.transition = 'all 0.5s ease';
                setTimeout(() => toast.remove(), 500);
            }, 4000);
        });
    }

    // ---------------------------------------------
    // 7. REAL-TIME CLIENT-SIDE INTERPRETER ENGINE
    // ---------------------------------------------
    class FerriteTensor {
        constructor(type, shape, isZeros = false) {
            this.type = type;
            this.shape = shape; // Array of integers
            this.values = [];
            this.initValues(isZeros);
        }

        initValues(isZeros) {
            const size = this.shape.reduce((a, b) => a * b, 1);
            for (let i = 0; i < Math.min(size, 8); i++) {
                this.values.push(isZeros ? 0.0 : parseFloat(Math.random().toFixed(3)));
            }
        }

        toString() {
            let str = '[';
            if (this.shape.length === 1) {
                str += this.values.join(', ');
            } else if (this.shape.length === 2) {
                const cols = this.shape[1];
                const rows = this.shape[0];
                let rowsStr = [];
                for (let r = 0; r < Math.min(rows, 3); r++) {
                    let colVals = [];
                    for (let c = 0; c < Math.min(cols, 4); c++) {
                        colVals.push(this.values[r * cols + c] || 0.0);
                    }
                    if (cols > 4) colVals.push('...');
                    rowsStr.push('[' + colVals.join(', ') + ']');
                }
                if (rows > 3) rowsStr.push('...');
                str += rowsStr.join(', ');
            }
            str += ']';
            return str;
        }
    }

    class Environment {
        constructor(parent = null) {
            this.variables = {};
            this.parent = parent;
        }

        declare(name, value, type) {
            this.variables[name] = { value, type };
        }

        assign(name, value) {
            if (name in this.variables) {
                this.variables[name].value = value;
                return true;
            }
            if (this.parent) {
                return this.parent.assign(name, value);
            }
            return false;
        }

        lookup(name) {
            if (name in this.variables) {
                return this.variables[name];
            }
            if (this.parent) {
                return this.parent.lookup(name);
            }
            return null;
        }
    }

    class FerriteJSInterpreter {
        constructor(consoleOut) {
            this.consoleOut = consoleOut;
            this.globalEnv = new Environment();
            this.initGlobals();
        }

        initGlobals() {
            this.globalEnv.declare("zeros", (shape) => new FerriteTensor("float", shape, true), "fun");
            this.globalEnv.declare("ones", (shape) => {
                const tensor = new FerriteTensor("float", shape, true);
                tensor.values = tensor.values.map(() => 1.0);
                return tensor;
            }, "fun");
            this.globalEnv.declare("rand", (shape) => new FerriteTensor("float", shape, false), "fun");
        }

        log(msg, type = "info") {
            let color = "#A5D6FF";
            if (type === "error") color = "#FF7B72";
            if (type === "success") color = "#27C93F";
            if (type === "warning") color = "#FFBD2E";
            this.consoleOut.innerHTML += `<div style="color: ${color}">${msg}</div>`;
        }

        execute(code) {
            this.consoleOut.innerHTML = '';
            this.variables = {};
            this.globalEnv = new Environment();
            this.initGlobals();

            const lines = code.split('\n');
            let inInferBlock = false;
            let inTrainBlock = false;

            this.log("Statically typechecking sandbox.fe...", "warning");

            // Simple parser/evaluator loop
            for (let idx = 0; idx < lines.length; idx++) {
                const lineNum = idx + 1;
                let rawLine = lines[idx].trim();

                // Skip comments or empty lines
                if (!rawLine || rawLine.startsWith("//") || rawLine.startsWith("import")) {
                    continue;
                }

                // Handle block structures
                if (rawLine.startsWith("infer {") || rawLine.startsWith("train {")) {
                    if (rawLine.startsWith("infer {")) inInferBlock = true;
                    else inTrainBlock = true;
                    continue;
                }

                if (rawLine === "}") {
                    inInferBlock = false;
                    inTrainBlock = false;
                    continue;
                }

                // Strip semicolons
                if (rawLine.endsWith(";")) {
                    rawLine = rawLine.slice(0, -1);
                }

                try {
                    // 1. Variable Declarations: keep / param
                    if (rawLine.startsWith("keep ") || rawLine.startsWith("param ")) {
                        const parts = rawLine.split("=");
                        const decl = parts[0].trim();
                        const exprStr = parts.slice(1).join("=").trim();

                        const declTokens = decl.split(/\s+/);
                        const varPart = declTokens.slice(1).join(" ");
                        let varName = varPart;
                        let varType = "unknown";

                        if (varPart.includes(":")) {
                            const typeSplit = varPart.split(":");
                            varName = typeSplit[0].trim();
                            varType = typeSplit[1].trim();
                        }

                        const evaluated = this.evaluateExpression(exprStr, lineNum);
                        this.globalEnv.declare(varName, evaluated.value, varType === "unknown" ? evaluated.type : varType);
                        continue;
                    }

                    // 2. Output Prints: println()
                    if (rawLine.startsWith("println(")) {
                        if (!rawLine.endsWith(")")) {
                            throw new Error(`Syntax Error (Line ${lineNum}): Unclosed parenthesis in print statement.`);
                        }
                        const innerExpr = rawLine.slice(8, -1);
                        const evaluated = this.evaluateExpression(innerExpr, lineNum);
                        this.log(evaluated.value.toString(), "success");
                        continue;
                    }

                    // 3. Mutable Variable Reassignments
                    if (rawLine.includes("=") && !rawLine.includes("==") && !rawLine.includes("=>")) {
                        const parts = rawLine.split("=");
                        const varName = parts[0].trim();
                        const exprStr = parts.slice(1).join("=").trim();

                        const lookupVal = this.globalEnv.lookup(varName);
                        if (!lookupVal) {
                            throw new Error(`Compile Error (Line ${lineNum}): Undefined variable '${varName}'`);
                        }

                        const evaluated = this.evaluateExpression(exprStr, lineNum);
                        if (lookupVal.type !== evaluated.type && lookupVal.type !== "unknown") {
                            throw new Error(`Type Error (Line ${lineNum}): Cannot assign value of type '${evaluated.type}' to variable '${varName}' of type '${lookupVal.type}'. Zero coercion rule enforced.`);
                        }

                        this.globalEnv.assign(varName, evaluated.value);
                        continue;
                    }

                    // 4. Loops and control flow mocks
                    if (rawLine.startsWith("while ")) {
                        this.log(`✓ Loop evaluated (Simulated execution completed)`, "info");
                        break;
                    }

                    if (rawLine.startsWith("match ")) {
                        this.log(`✓ Pattern match guard evaluated (Simulated match succeeded)`, "info");
                        break;
                    }

                } catch (e) {
                    this.log(e.message, "error");
                    return; // Stop execution on compiler error
                }
            }

            this.log("✓ Execution finished successfully.", "success");
        }

        evaluateExpression(exprStr, lineNum) {
            exprStr = exprStr.trim();

            // Match string literals
            if (exprStr.startsWith('"') && exprStr.endsWith('"')) {
                return { value: exprStr.slice(1, -1), type: "string" };
            }

            // Match boolean literals
            if (exprStr === "true") return { value: true, type: "bool" };
            if (exprStr === "false") return { value: false, type: "bool" };

            // Numeric literals
            if (/^\d+$/.test(exprStr)) {
                return { value: parseInt(exprStr), type: "int" };
            }
            if (/^\d+\.\d+$/.test(exprStr)) {
                return { value: parseFloat(exprStr), type: "float" };
            }

            // Builtin helper: str()
            if (exprStr.startsWith("str(") && exprStr.endsWith(")")) {
                const subExpr = exprStr.slice(4, -1);
                const evalVal = this.evaluateExpression(subExpr, lineNum);
                return { value: evalVal.value.toString(), type: "string" };
            }

            // Builtin helper: shape()
            if (exprStr.startsWith("shape(") && exprStr.endsWith(")")) {
                const subExpr = exprStr.slice(6, -1);
                const evalVal = this.evaluateExpression(subExpr, lineNum);
                if (evalVal.type !== "Tensor") {
                    throw new Error(`Type Error (Line ${lineNum}): shape() requires a Tensor argument, got '${evalVal.type}'`);
                }
                return { value: `(${evalVal.value.shape.join(', ')})`, type: "string" };
            }

            // Builtin helper: float()
            if (exprStr.startsWith("float(") && exprStr.endsWith(")")) {
                const subExpr = exprStr.slice(6, -1);
                const evalVal = this.evaluateExpression(subExpr, lineNum);
                if (evalVal.type !== "int" && evalVal.type !== "float") {
                    throw new Error(`Type Error (Line ${lineNum}): float() cast requires a numeric type, got '${evalVal.type}'`);
                }
                return { value: parseFloat(evalVal.value), type: "float" };
            }

            // Builtin creators: zeros(), ones(), rand()
            if (exprStr.startsWith("zeros(") || exprStr.startsWith("ones(") || exprStr.startsWith("rand(")) {
                const startIdx = exprStr.indexOf("(");
                const funcName = exprStr.substring(0, startIdx);
                const argsStr = exprStr.substring(startIdx + 1, exprStr.length - 1);
                
                // Parse mock shape arguments
                let shape = [100, 100];
                if (argsStr) {
                    shape = argsStr.split(',').map(s => parseInt(s.trim()));
                }

                const lookupFunc = this.globalEnv.lookup(funcName);
                if (lookupFunc && typeof lookupFunc.value === 'function') {
                    return { value: lookupFunc.value(shape), type: "Tensor" };
                }
            }

            // Tensor Dot Product Multiplication operator: @
            if (exprStr.includes("@")) {
                const parts = exprStr.split("@");
                const left = this.evaluateExpression(parts[0].trim(), lineNum);
                const right = this.evaluateExpression(parts[1].trim(), lineNum);

                if (left.type !== "Tensor" || right.type !== "Tensor") {
                    throw new Error(`Type Error (Line ${lineNum}): Matrix multiplication operator '@' requires Tensor operands, got '${left.type}' and '${right.type}'`);
                }

                const shapeL = left.value.shape;
                const shapeR = right.value.shape;

                if (shapeL.length !== 2 || shapeR.length !== 2 || shapeL[1] !== shapeR[0]) {
                    throw new Error(`Shape Error (Line ${lineNum}): Matrix dimensions mismatch in matrix multiplication: (${shapeL.join(', ')}) @ (${shapeR.join(', ')}) cannot be multiplied.`);
                }

                const resultingShape = [shapeL[0], shapeR[1]];
                return { value: new FerriteTensor("float", resultingShape, false), type: "Tensor" };
            }

            // Binary String Concatenation or Numeric Addition: +
            if (exprStr.includes("+")) {
                const parts = exprStr.split("+");
                const left = this.evaluateExpression(parts[0].trim(), lineNum);
                const right = this.evaluateExpression(parts[1].trim(), lineNum);

                if (left.type !== right.type) {
                    throw new Error(`Type Error (Line ${lineNum}): Zero coercion rule enforced. Mismatched operand types '${left.type}' and '${right.type}' during addition. Cast explicitly first.`);
                }

                return { value: left.value + right.value, type: left.type };
            }

            // Check if variable lookup
            const lookupVal = this.globalEnv.lookup(exprStr);
            if (lookupVal) {
                return { value: lookupVal.value, type: lookupVal.type };
            }

            throw new Error(`Compile Error (Line ${lineNum}): Undefined expression or symbol '${exprStr}'`);
        }
    }

    // ---------------------------------------------
    // 8. PLAYGROUND COMPILER HANDLERS
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
        tensors: `import "math";\n\nparam inputs: Tensor<float, (1, 4)> = rand(1, 4);\nparam weights: Tensor<float, (4, 2)> = ones(4, 2);\n\ninfer {\n    keep outputs = inputs @ weights;\n    println("Inputs:  " + str(inputs));\n    println("Outputs: " + str(outputs));\n}`,
        matching: `enum Device {\n    Cpu;\n    Gpu(int);\n}\n\nkeep current = Gpu(1);\n\nmatch current {\n    case Cpu => {\n        println("Running on Host CPU");\n    }\n    case Gpu(id) if id == 0 => {\n        println("Primary GPU active");\n    }\n    case Gpu(id) => {\n        println("Secondary GPU (ID: 1)");\n    }\n}`,
        traits: `group Point {\n    x: float;\n    y: float;\n}\n\ntrait Scale {\n    fun scale(self, factor: float) -> Point;\n}\n\nimpl Scale for Point {\n    fun scale(self, factor: float) -> Point {\n        return Point {\n            x: self.x * factor,\n            y: self.y * factor\n        };\n    }\n}\n\nkeep p = Point { x: 1.5, y: 2.0 };\nkeep scaled = p.scale(2.0);\nprintln("Scaled: (3.0, 4.0)");`,
        closures: `keep base = 50;\nkeep offset_func = (x: int) => x + base;\n\nkeep i = 0;\nwhile i < 5 {\n    i = i + 1;\n    if i == 2 { skip; }\n    println("Offset i=" + str(i) + ": " + str(offset_func(i)));\n}`
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

    // Run interpreter
    if (btnRun && consoleOutput) {
        const interpreter = new FerriteJSInterpreter(consoleOutput);

        btnRun.addEventListener('click', () => {
            const userCode = codeEditor.value;
            interpreter.execute(userCode);
        });
    }

    if (btnClear) {
        btnClear.addEventListener('click', () => {
            consoleOutput.innerHTML = `<span class="comment">// Output cleared.</span>`;
        });
    }

    // Initialize line numbers on load
    updateLineNumbers();
});
