// ================================================================
// Ferrite v3.2.0 — Browser Tree-Walk Interpreter
// A faithful JS port of the Rust lexer → parser → interpreter pipeline.
// This runs entirely client-side, no external backend needed.
// ================================================================

// ── Token Types ─────────────────────────────────────────────────

const TK = Object.freeze({
  // Keywords
  Fun: "Fun",
  Keep: "Keep",
  Param: "Param",
  Constant: "Constant",
  Group: "Group",
  Enum: "Enum",
  Import: "Import",
  From: "From",
  Take: "Take",
  As: "As",
  If: "If",
  Elif: "Elif",
  Else: "Else",
  While: "While",
  For: "For",
  In: "In",
  Return: "Return",
  Stop: "Stop",
  Skip: "Skip",
  Match: "Match",
  Case: "Case",
  Default: "Default",
  Infer: "Infer",
  Train: "Train",
  Trait: "Trait",
  Impl: "Impl",
  Pub: "Pub",
  True: "True",
  False: "False",
  SelfKw: "SelfKw",
  // Literals & Ident
  Ident: "Ident",
  IntLit: "IntLit",
  FloatLit: "FloatLit",
  StringLit: "StringLit",
  // Operators
  Plus: "+",
  Minus: "-",
  Star: "*",
  Slash: "/",
  Percent: "%",
  Eq: "=",
  EqEq: "==",
  BangEq: "!=",
  At: "@",
  Lt: "<",
  Gt: ">",
  LtEq: "<=",
  GtEq: ">=",
  And: "&&",
  Or: "||",
  Bang: "!",
  Arrow: "->",
  FatArrow: "=>",
  // Delimiters
  LParen: "(",
  RParen: ")",
  LBrace: "{",
  RBrace: "}",
  LBracket: "[",
  RBracket: "]",
  Comma: ",",
  Colon: ":",
  Semicolon: ";",
  Dot: ".",
  // Special
  EOF: "EOF",
});

const KEYWORDS = {
  fun: TK.Fun,
  keep: TK.Keep,
  param: TK.Param,
  constant: TK.Constant,
  group: TK.Group,
  enum: TK.Enum,
  import: TK.Import,
  from: TK.From,
  take: TK.Take,
  as: TK.As,
  if: TK.If,
  elif: TK.Elif,
  else: TK.Else,
  while: TK.While,
  for: TK.For,
  in: TK.In,
  return: TK.Return,
  stop: TK.Stop,
  skip: TK.Skip,
  match: TK.Match,
  case: TK.Case,
  default: TK.Default,
  infer: TK.Infer,
  train: TK.Train,
  trait: TK.Trait,
  impl: TK.Impl,
  pub: TK.Pub,
  true: TK.True,
  false: TK.False,
  self: TK.SelfKw,
};

// ── Lexer ───────────────────────────────────────────────────────

class Lexer {
  constructor(source) {
    this.src = source;
    this.pos = 0;
    this.line = 1;
    this.col = 1;
  }

  peek() {
    return this.pos < this.src.length ? this.src[this.pos] : null;
  }
  advance() {
    const ch = this.src[this.pos++];
    if (ch === "\n") {
      this.line++;
      this.col = 1;
    } else {
      this.col++;
    }
    return ch;
  }

  tokenize() {
    const tokens = [];
    while (this.pos < this.src.length) {
      this.skipWhitespaceAndComments();
      if (this.pos >= this.src.length) break;
      const ch = this.peek();

      // String literal
      if (ch === '"') {
        tokens.push(this.readString());
        continue;
      }
      // Number
      if (this.isDigit(ch)) {
        tokens.push(this.readNumber());
        continue;
      }
      // Ident / Keyword
      if (this.isAlpha(ch) || ch === "_") {
        tokens.push(this.readIdent());
        continue;
      }
      // Operators and delimiters
      tokens.push(this.readSymbol());
    }
    tokens.push({ kind: TK.EOF, line: this.line, col: this.col });
    return tokens;
  }

  skipWhitespaceAndComments() {
    while (this.pos < this.src.length) {
      const ch = this.peek();
      if (ch === " " || ch === "\t" || ch === "\r" || ch === "\n") {
        this.advance();
      } else if (
        ch === "/" &&
        this.pos + 1 < this.src.length &&
        this.src[this.pos + 1] === "/"
      ) {
        while (this.pos < this.src.length && this.peek() !== "\n")
          this.advance();
      } else {
        break;
      }
    }
  }

  readString() {
    const line = this.line,
      col = this.col;
    this.advance(); // opening "
    let s = "";
    while (this.pos < this.src.length && this.peek() !== '"') {
      if (this.peek() === "\\") {
        this.advance();
        const esc = this.advance();
        if (esc === "n") s += "\n";
        else if (esc === "t") s += "\t";
        else if (esc === "\\") s += "\\";
        else if (esc === '"') s += '"';
        else s += esc;
      } else {
        s += this.advance();
      }
    }
    if (this.pos < this.src.length) this.advance(); // closing "
    return { kind: TK.StringLit, value: s, line, col };
  }

  readNumber() {
    const line = this.line,
      col = this.col;
    let numStr = "";
    while (this.pos < this.src.length && this.isDigit(this.peek()))
      numStr += this.advance();
    if (
      this.peek() === "." &&
      this.pos + 1 < this.src.length &&
      this.isDigit(this.src[this.pos + 1])
    ) {
      numStr += this.advance(); // .
      while (this.pos < this.src.length && this.isDigit(this.peek()))
        numStr += this.advance();
      return { kind: TK.FloatLit, value: parseFloat(numStr), line, col };
    }
    return { kind: TK.IntLit, value: parseInt(numStr, 10), line, col };
  }

  readIdent() {
    const line = this.line,
      col = this.col;
    let id = "";
    while (
      this.pos < this.src.length &&
      (this.isAlphaNum(this.peek()) || this.peek() === "_")
    ) {
      id += this.advance();
    }
    if (KEYWORDS[id] !== undefined) {
      return { kind: KEYWORDS[id], line, col };
    }
    return { kind: TK.Ident, value: id, line, col };
  }

  readSymbol() {
    const line = this.line,
      col = this.col;
    const ch = this.advance();
    const next = this.peek();
    switch (ch) {
      case "+":
        return { kind: TK.Plus, line, col };
      case "*":
        return { kind: TK.Star, line, col };
      case "/":
        return { kind: TK.Slash, line, col };
      case "%":
        return { kind: TK.Percent, line, col };
      case "@":
        return { kind: TK.At, line, col };
      case "(":
        return { kind: TK.LParen, line, col };
      case ")":
        return { kind: TK.RParen, line, col };
      case "{":
        return { kind: TK.LBrace, line, col };
      case "}":
        return { kind: TK.RBrace, line, col };
      case "[":
        return { kind: TK.LBracket, line, col };
      case "]":
        return { kind: TK.RBracket, line, col };
      case ",":
        return { kind: TK.Comma, line, col };
      case ":":
        return { kind: TK.Colon, line, col };
      case ";":
        return { kind: TK.Semicolon, line, col };
      case ".":
        return { kind: TK.Dot, line, col };
      case "-":
        if (next === ">") {
          this.advance();
          return { kind: TK.Arrow, line, col };
        }
        return { kind: TK.Minus, line, col };
      case "=":
        if (next === "=") {
          this.advance();
          return { kind: TK.EqEq, line, col };
        }
        if (next === ">") {
          this.advance();
          return { kind: TK.FatArrow, line, col };
        }
        return { kind: TK.Eq, line, col };
      case "!":
        if (next === "=") {
          this.advance();
          return { kind: TK.BangEq, line, col };
        }
        return { kind: TK.Bang, line, col };
      case "<":
        if (next === "=") {
          this.advance();
          return { kind: TK.LtEq, line, col };
        }
        return { kind: TK.Lt, line, col };
      case ">":
        if (next === "=") {
          this.advance();
          return { kind: TK.GtEq, line, col };
        }
        return { kind: TK.Gt, line, col };
      case "&":
        if (next === "&") {
          this.advance();
          return { kind: TK.And, line, col };
        }
        throw new FerriteError(`Unexpected character '&'`, line, col);
      case "|":
        if (next === "|") {
          this.advance();
          return { kind: TK.Or, line, col };
        }
        throw new FerriteError(`Unexpected character '|'`, line, col);
      default:
        throw new FerriteError(`Unexpected character '${ch}'`, line, col);
    }
  }

  isDigit(ch) {
    return ch >= "0" && ch <= "9";
  }
  isAlpha(ch) {
    return (ch >= "a" && ch <= "z") || (ch >= "A" && ch <= "Z") || ch === "_";
  }
  isAlphaNum(ch) {
    return this.isDigit(ch) || this.isAlpha(ch);
  }
}

// ── Error ───────────────────────────────────────────────────────

class FerriteError extends Error {
  constructor(msg, line, col) {
    super(line != null ? `[Line ${line}] ${msg}` : msg);
    this.ferriteLine = line;
    this.ferriteCol = col;
  }
}

// ── AST Node Types ──────────────────────────────────────────────
// We use plain objects with a `type` field instead of classes.

// ── Parser ──────────────────────────────────────────────────────

class Parser {
  constructor(tokens) {
    this.tokens = tokens;
    this.pos = 0;
  }

  current() {
    return this.tokens[this.pos];
  }
  peek() {
    return this.tokens[this.pos];
  }
  peekKind() {
    return this.tokens[this.pos].kind;
  }

  advance() {
    return this.tokens[this.pos++];
  }
  expect(kind) {
    const t = this.advance();
    if (t.kind !== kind) {
      throw new FerriteError(
        `Expected '${kind}' but got '${t.kind}'`,
        t.line,
        t.col,
      );
    }
    return t;
  }
  match(kind) {
    if (this.peekKind() === kind) {
      this.advance();
      return true;
    }
    return false;
  }

  parse() {
    const decls = [];
    while (this.peekKind() !== TK.EOF) {
      decls.push(this.parseTopDecl());
    }
    return { type: "Program", decls };
  }

  parseTopDecl() {
    const tk = this.peekKind();
    if (tk === TK.Import) return this.parseImport();
    if (tk === TK.Constant) return this.parseConstant();
    if (tk === TK.Fun) return this.parseFuncDecl();
    if (tk === TK.Group) return this.parseGroupDecl();
    if (tk === TK.Enum) return this.parseEnumDecl();
    if (tk === TK.Trait) return this.parseTraitDecl();
    if (tk === TK.Impl) return this.parseImplBlock();
    // Top-level statements (script mode) — wrap as ExprStmt
    return this.parseStmt();
  }

  parseImport() {
    this.expect(TK.Import);
    const path = this.expect(TK.StringLit);
    this.expect(TK.Semicolon);
    return { type: "Import", path: path.value };
  }

  parseConstant() {
    this.expect(TK.Constant);
    const name = this.expect(TK.Ident).value;
    this.expect(TK.Colon);
    this.skipType();
    this.expect(TK.Eq);
    const value = this.parseExpr();
    this.expect(TK.Semicolon);
    return { type: "Constant", name, value };
  }

  parseFuncDecl() {
    this.expect(TK.Fun);
    const name = this.expect(TK.Ident).value;
    this.expect(TK.LParen);
    const params = this.parseParamList();
    this.expect(TK.RParen);
    let returnType = null;
    if (this.match(TK.Arrow)) {
      returnType = this.parseTypeExpr();
    }
    const body = this.parseBlock();
    return { type: "FuncDecl", name, params, returnType, body };
  }

  parseParamList() {
    const params = [];
    while (this.peekKind() !== TK.RParen && this.peekKind() !== TK.EOF) {
      // Handle `self` parameter
      if (this.peekKind() === TK.SelfKw) {
        this.advance();
        params.push({ name: "self", ty: null });
      } else {
        const name = this.expect(TK.Ident).value;
        this.expect(TK.Colon);
        const ty = this.parseTypeExpr();
        params.push({ name, ty });
      }
      if (!this.match(TK.Comma)) break;
    }
    return params;
  }

  parseGroupDecl() {
    this.expect(TK.Group);
    const name = this.expect(TK.Ident).value;
    if (this.peekKind() === TK.Lt) this.skipGenerics();
    this.expect(TK.LBrace);
    const fields = [];
    const methods = [];
    while (this.peekKind() !== TK.RBrace && this.peekKind() !== TK.EOF) {
      if (this.peekKind() === TK.Fun) {
        methods.push(this.parseFuncDecl());
      } else {
        const fname = this.expect(TK.Ident).value;
        this.expect(TK.Colon);
        const fty = this.parseTypeExpr();
        this.expect(TK.Semicolon);
        fields.push({ name: fname, ty: fty });
      }
    }
    this.expect(TK.RBrace);
    return { type: "GroupDecl", name, fields, methods };
  }

  parseEnumDecl() {
    this.expect(TK.Enum);
    const name = this.expect(TK.Ident).value;
    if (this.peekKind() === TK.Lt) this.skipGenerics();
    this.expect(TK.LBrace);
    const variants = [];
    while (this.peekKind() !== TK.RBrace && this.peekKind() !== TK.EOF) {
      const vname = this.expect(TK.Ident).value;
      const vfields = [];
      if (this.match(TK.LParen)) {
        while (this.peekKind() !== TK.RParen && this.peekKind() !== TK.EOF) {
          vfields.push(this.parseTypeExpr());
          if (!this.match(TK.Comma)) break;
        }
        this.expect(TK.RParen);
      }
      this.expect(TK.Semicolon);
      variants.push({ name: vname, fields: vfields });
    }
    this.expect(TK.RBrace);
    return { type: "EnumDecl", name, variants };
  }

  parseTraitDecl() {
    this.expect(TK.Trait);
    const name = this.expect(TK.Ident).value;
    if (this.peekKind() === TK.Lt) this.skipGenerics();
    this.expect(TK.LBrace);
    const methods = [];
    while (this.peekKind() !== TK.RBrace && this.peekKind() !== TK.EOF) {
      this.expect(TK.Fun);
      const mname = this.expect(TK.Ident).value;
      this.expect(TK.LParen);
      const mparams = this.parseParamList();
      this.expect(TK.RParen);
      let retType = null;
      if (this.match(TK.Arrow)) retType = this.parseTypeExpr();
      this.expect(TK.Semicolon);
      methods.push({ name: mname, params: mparams, returnType: retType });
    }
    this.expect(TK.RBrace);
    return { type: "TraitDecl", name, methods };
  }

  parseImplBlock() {
    this.expect(TK.Impl);
    let traitName = null;
    const firstName = this.expect(TK.Ident).value;
    let targetType = firstName;
    if (this.match(TK.For)) {
      traitName = firstName;
      targetType = this.expect(TK.Ident).value;
    }
    this.expect(TK.LBrace);
    const methods = [];
    while (this.peekKind() !== TK.RBrace && this.peekKind() !== TK.EOF) {
      methods.push(this.parseFuncDecl());
    }
    this.expect(TK.RBrace);
    return { type: "ImplBlock", traitName, targetType, methods };
  }

  parseStmt() {
    const tk = this.peekKind();
    if (tk === TK.Keep || tk === TK.Param) return this.parseVarDecl();
    if (tk === TK.Fun) return this.parseFuncDecl();
    if (tk === TK.Group) return this.parseGroupDecl();
    if (tk === TK.Enum) return this.parseEnumDecl();
    if (tk === TK.Trait) return this.parseTraitDecl();
    if (tk === TK.Impl) return this.parseImplBlock();
    if (tk === TK.Import) return this.parseImport();
    if (tk === TK.Constant) return this.parseConstant();
    // Expression statement
    const expr = this.parseExpr();
    const hasSemi = this.match(TK.Semicolon);
    return { type: "ExprStmt", expr, hasSemi };
  }

  parseVarDecl() {
    const declKind = this.advance().kind; // Keep or Param
    const name = this.expect(TK.Ident).value;
    let ty = null;
    if (this.match(TK.Colon)) {
      ty = this.parseTypeExpr();
    }
    this.expect(TK.Eq);
    const value = this.parseExpr();
    this.expect(TK.Semicolon);
    return { type: declKind === TK.Keep ? "Keep" : "Param", name, ty, value };
  }

  parseBlock() {
    this.expect(TK.LBrace);
    const stmts = [];
    while (this.peekKind() !== TK.RBrace && this.peekKind() !== TK.EOF) {
      stmts.push(this.parseStmt());
    }
    this.expect(TK.RBrace);
    return { type: "Block", stmts };
  }

  // ── Expression Parsing (Pratt-style precedence climbing) ──────

  parseExpr() {
    return this.parseAssign();
  }

  parseAssign() {
    let left = this.parseOr();
    if (this.peekKind() === TK.Eq) {
      this.advance();
      const value = this.parseAssign();
      return { type: "Assign", target: left, value };
    }
    return left;
  }

  parseOr() {
    let left = this.parseAnd();
    while (this.match(TK.Or)) {
      const right = this.parseAnd();
      left = { type: "BinOp", op: "||", left, right };
    }
    return left;
  }

  parseAnd() {
    let left = this.parseEquality();
    while (this.match(TK.And)) {
      const right = this.parseEquality();
      left = { type: "BinOp", op: "&&", left, right };
    }
    return left;
  }

  parseEquality() {
    let left = this.parseComparison();
    while (this.peekKind() === TK.EqEq || this.peekKind() === TK.BangEq) {
      const op = this.advance().kind;
      const right = this.parseComparison();
      left = { type: "BinOp", op, left, right };
    }
    return left;
  }

  parseComparison() {
    let left = this.parseAddSub();
    while ([TK.Lt, TK.Gt, TK.LtEq, TK.GtEq].includes(this.peekKind())) {
      const op = this.advance().kind;
      const right = this.parseAddSub();
      left = { type: "BinOp", op, left, right };
    }
    return left;
  }

  parseAddSub() {
    let left = this.parseMulDiv();
    while (this.peekKind() === TK.Plus || this.peekKind() === TK.Minus) {
      const op = this.advance().kind;
      const right = this.parseMulDiv();
      left = { type: "BinOp", op, left, right };
    }
    return left;
  }

  parseMulDiv() {
    let left = this.parseMatMul();
    while ([TK.Star, TK.Slash, TK.Percent].includes(this.peekKind())) {
      const op = this.advance().kind;
      const right = this.parseMatMul();
      left = { type: "BinOp", op, left, right };
    }
    return left;
  }

  parseMatMul() {
    let left = this.parseUnary();
    while (this.peekKind() === TK.At) {
      this.advance();
      const right = this.parseUnary();
      left = { type: "BinOp", op: "@", left, right };
    }
    return left;
  }

  parseUnary() {
    if (this.peekKind() === TK.Minus) {
      this.advance();
      const operand = this.parseUnary();
      return { type: "UnaryOp", op: "-", operand };
    }
    if (this.peekKind() === TK.Bang) {
      this.advance();
      const operand = this.parseUnary();
      return { type: "UnaryOp", op: "!", operand };
    }
    return this.parsePostfix();
  }

  parsePostfix() {
    let expr = this.parsePrimary();
    while (true) {
      if (this.peekKind() === TK.LParen) {
        // Function call
        this.advance();
        const args = [];
        while (this.peekKind() !== TK.RParen && this.peekKind() !== TK.EOF) {
          args.push(this.parseExpr());
          if (!this.match(TK.Comma)) break;
        }
        this.expect(TK.RParen);
        expr = { type: "Call", callee: expr, args };
      } else if (this.peekKind() === TK.Dot) {
        this.advance();
        const field = this.expect(TK.Ident).value;
        expr = { type: "FieldAccess", object: expr, field };
      } else if (this.peekKind() === TK.LBracket) {
        this.advance();
        const index = this.parseExpr();
        this.expect(TK.RBracket);
        expr = { type: "IndexAccess", object: expr, index };
      } else {
        break;
      }
    }
    return expr;
  }

  parsePrimary() {
    const t = this.peek();

    // Literals
    if (t.kind === TK.IntLit) {
      this.advance();
      return { type: "Lit", litType: "int", value: t.value };
    }
    if (t.kind === TK.FloatLit) {
      this.advance();
      return { type: "Lit", litType: "float", value: t.value };
    }
    if (t.kind === TK.StringLit) {
      this.advance();
      return { type: "Lit", litType: "string", value: t.value };
    }
    if (t.kind === TK.True) {
      this.advance();
      return { type: "Lit", litType: "bool", value: true };
    }
    if (t.kind === TK.False) {
      this.advance();
      return { type: "Lit", litType: "bool", value: false };
    }

    // Grouped expression
    if (t.kind === TK.LParen) {
      this.advance();
      // Could be a lambda: (params) => expr
      // Peek ahead to see if this looks like a lambda
      const expr = this.parseExpr();
      this.expect(TK.RParen);
      // Check for fat arrow (lambda)
      if (this.peekKind() === TK.FatArrow) {
        this.advance();
        const body = this.parseExpr();
        // expr should be param-like, but for simplicity we just return it
        return { type: "Lambda", params: [], body };
      }
      return expr;
    }

    // List literal
    if (t.kind === TK.LBracket) {
      this.advance();
      const elements = [];
      while (this.peekKind() !== TK.RBracket && this.peekKind() !== TK.EOF) {
        elements.push(this.parseExpr());
        if (!this.match(TK.Comma)) break;
      }
      this.expect(TK.RBracket);
      return { type: "ListLit", elements };
    }

    // Block expression
    if (t.kind === TK.LBrace) {
      return { type: "BlockExpr", block: this.parseBlock() };
    }

    // If expression
    if (t.kind === TK.If) return this.parseIfExpr();

    // While expression
    if (t.kind === TK.While) return this.parseWhileExpr();

    // For expression
    if (t.kind === TK.For) return this.parseForExpr();

    // Match expression
    if (t.kind === TK.Match) return this.parseMatchExpr();

    // Return
    if (t.kind === TK.Return) {
      this.advance();
      let value = null;
      if (
        this.peekKind() !== TK.Semicolon &&
        this.peekKind() !== TK.RBrace &&
        this.peekKind() !== TK.EOF
      ) {
        value = this.parseExpr();
      }
      return { type: "Return", value };
    }

    // Stop / Skip
    if (t.kind === TK.Stop) {
      this.advance();
      return { type: "Stop" };
    }
    if (t.kind === TK.Skip) {
      this.advance();
      return { type: "Skip" };
    }

    // Infer block
    if (t.kind === TK.Infer) {
      this.advance();
      return { type: "InferBlock", block: this.parseBlock() };
    }

    // Train block
    if (t.kind === TK.Train) {
      this.advance();
      return { type: "TrainBlock", block: this.parseBlock() };
    }

    // Self keyword
    if (t.kind === TK.SelfKw) {
      this.advance();
      return { type: "Ident", name: "self" };
    }

    // Identifier — may be followed by { for group literal
    if (t.kind === TK.Ident) {
      this.advance();
      const name = t.value;

      // Check for group literal: Name { field: val, ... }
      // But only if NOT followed by something that looks like a block statement
      if (this.peekKind() === TK.LBrace && /^[A-Z]/.test(name)) {
        // Save position and try to parse as group literal
        const savedPos = this.pos;
        try {
          this.advance(); // consume {
          const fields = [];
          // Check if first thing is Ident Colon (group literal pattern)
          if (
            this.peekKind() === TK.Ident &&
            this.pos + 1 < this.tokens.length &&
            this.tokens[this.pos + 1].kind === TK.Colon
          ) {
            while (
              this.peekKind() !== TK.RBrace &&
              this.peekKind() !== TK.EOF
            ) {
              const fname = this.expect(TK.Ident).value;
              this.expect(TK.Colon);
              const fval = this.parseExpr();
              fields.push({ name: fname, value: fval });
              if (!this.match(TK.Comma)) break;
            }
            this.expect(TK.RBrace);
            return { type: "GroupLiteral", name, fields };
          } else {
            // Not a group literal, restore position
            this.pos = savedPos;
          }
        } catch (e) {
          this.pos = savedPos;
        }
      }

      return { type: "Ident", name };
    }

    throw new FerriteError(`Unexpected token '${t.kind}'`, t.line, t.col);
  }

  parseIfExpr() {
    this.expect(TK.If);
    const condition = this.parseExpr();
    const thenBlock = this.parseBlock();
    const elifBranches = [];
    while (this.peekKind() === TK.Elif) {
      this.advance();
      const elifCond = this.parseExpr();
      const elifBlock = this.parseBlock();
      elifBranches.push({ condition: elifCond, block: elifBlock });
    }
    let elseBlock = null;
    if (this.match(TK.Else)) {
      elseBlock = this.parseBlock();
    }
    return { type: "If", condition, thenBlock, elifBranches, elseBlock };
  }

  parseWhileExpr() {
    this.expect(TK.While);
    const condition = this.parseExpr();
    const body = this.parseBlock();
    return { type: "While", condition, body };
  }

  parseForExpr() {
    this.expect(TK.For);
    const varName = this.expect(TK.Ident).value;
    this.expect(TK.In);
    const iterable = this.parseExpr();
    const body = this.parseBlock();
    return { type: "For", var: varName, iterable, body };
  }

  parseMatchExpr() {
    this.expect(TK.Match);
    const subject = this.parseExpr();
    this.expect(TK.LBrace);
    const cases = [];
    while (this.peekKind() !== TK.RBrace && this.peekKind() !== TK.EOF) {
      if (this.peekKind() === TK.Case) {
        this.advance();
        const pattern = this.parsePattern();
        let guard = null;
        if (this.peekKind() === TK.If) {
          this.advance();
          guard = this.parseExpr();
        }
        this.expect(TK.FatArrow);
        const body = this.parseBlock();
        cases.push({ pattern, guard, body });
      } else if (this.peekKind() === TK.Default) {
        this.advance();
        this.expect(TK.FatArrow);
        const body = this.parseBlock();
        cases.push({ pattern: { type: "Wildcard" }, guard: null, body });
      } else {
        throw new FerriteError(
          `Expected 'case' or 'default' in match`,
          this.peek().line,
          this.peek().col,
        );
      }
    }
    this.expect(TK.RBrace);
    return { type: "Match", subject, cases };
  }

  parsePattern() {
    const t = this.peek();
    if (t.kind === TK.IntLit) {
      this.advance();
      return { type: "LitPattern", litType: "int", value: t.value };
    }
    if (t.kind === TK.FloatLit) {
      this.advance();
      return { type: "LitPattern", litType: "float", value: t.value };
    }
    if (t.kind === TK.StringLit) {
      this.advance();
      return { type: "LitPattern", litType: "string", value: t.value };
    }
    if (t.kind === TK.True) {
      this.advance();
      return { type: "LitPattern", litType: "bool", value: true };
    }
    if (t.kind === TK.False) {
      this.advance();
      return { type: "LitPattern", litType: "bool", value: false };
    }

    if (t.kind === TK.Ident) {
      this.advance();
      const name = t.value;
      // Constructor pattern: Name(sub_patterns)
      if (this.peekKind() === TK.LParen) {
        this.advance();
        const subPatterns = [];
        while (this.peekKind() !== TK.RParen && this.peekKind() !== TK.EOF) {
          subPatterns.push(this.parsePattern());
          if (!this.match(TK.Comma)) break;
        }
        this.expect(TK.RParen);
        return { type: "Constructor", name, fields: subPatterns };
      }
      // If the name starts with uppercase, treat as unit constructor pattern (e.g. Cpu, None)
      if (/^[A-Z]/.test(name)) {
        return { type: "Constructor", name, fields: [] };
      }
      // Variable binding (lowercase)
      return { type: "Binding", name };
    }

    throw new FerriteError(
      `Unexpected pattern token '${t.kind}'`,
      t.line,
      t.col,
    );
  }

  // ── Type Parsing (just enough to skip types, we don't type-check in JS) ──

  parseTypeExpr() {
    return this.skipType();
  }

  skipType() {
    // Consume tokens that form a type expression, return a string representation
    let depth = 0;
    let typeStr = "";
    while (this.peekKind() !== TK.EOF) {
      const tk = this.peekKind();
      if (tk === TK.Lt) {
        depth++;
        typeStr += "<";
        this.advance();
        continue;
      }
      if (tk === TK.Gt) {
        if (depth > 0) {
          depth--;
          typeStr += ">";
          this.advance();
          continue;
        }
        break;
      }
      if (
        depth === 0 &&
        (tk === TK.Eq ||
          tk === TK.LBrace ||
          tk === TK.Semicolon ||
          tk === TK.RParen ||
          tk === TK.Comma ||
          tk === TK.RBrace ||
          tk === TK.FatArrow ||
          tk === TK.Arrow)
      ) {
        break;
      }
      // Handle -> inside types at depth 0 as function return type separator (not a type token)
      if (depth === 0 && tk === TK.Arrow) break;

      typeStr +=
        this.advance().kind === TK.Ident
          ? this.tokens[this.pos - 1].value
          : this.tokens[this.pos - 1].kind;
    }
    return typeStr;
  }

  skipGenerics() {
    if (this.peekKind() !== TK.Lt) return;
    this.advance();
    let depth = 1;
    while (depth > 0 && this.peekKind() !== TK.EOF) {
      if (this.peekKind() === TK.Lt) depth++;
      if (this.peekKind() === TK.Gt) depth--;
      this.advance();
    }
  }
}

// ── Control Flow Signals ────────────────────────────────────────

class ReturnSignal {
  constructor(value) {
    this.value = value;
  }
}
class StopSignal {}
class SkipSignal {}

// ── Runtime Values ──────────────────────────────────────────────

function displayValue(val) {
  if (val === null || val === undefined) return "()";
  if (typeof val === "number") return val.toString();
  if (typeof val === "boolean") return val.toString();
  if (typeof val === "string") return val;
  if (val._type === "func") return `<fun ${val.name}>`;
  if (val._type === "closure") return `<closure>`;
  if (val._type === "builtin") return `<builtin ${val.name}>`;
  if (val._type === "group") {
    const fields = Object.entries(val.fields)
      .map(([k, v]) => `${k}: ${displayValue(v)}`)
      .join(", ");
    return `${val.name} { ${fields} }`;
  }
  if (val._type === "enum") {
    if (val.values.length === 0) return `${val.enumName}::${val.variant}`;
    return `${val.enumName}::${val.variant}(${val.values.map(displayValue).join(", ")})`;
  }
  if (val._type === "list") {
    return `[${val.items.map(displayValue).join(", ")}]`;
  }
  if (val._type === "tensor") {
    return `Tensor(${val.data.length}, shape=[${val.shape.join(", ")}])`;
  }
  if (val._type === "bound_method") return `<bound method>`;
  return String(val);
}

// ── Environment ─────────────────────────────────────────────────

class Environment {
  constructor(parent = null) {
    this.vars = new Map();
    this.parent = parent;
  }
  declare(name, value) {
    this.vars.set(name, value);
  }
  get(name) {
    if (this.vars.has(name)) return this.vars.get(name);
    if (this.parent) return this.parent.get(name);
    return undefined;
  }
  set(name, value) {
    if (this.vars.has(name)) {
      this.vars.set(name, value);
      return true;
    }
    if (this.parent) return this.parent.set(name, value);
    return false;
  }
  child() {
    return new Environment(this);
  }
}

// ── Interpreter ─────────────────────────────────────────────────

class FerriteInterpreter {
  constructor(outputFn) {
    this.output = outputFn;
    this.env = new Environment();
    this.stepCount = 0;
    this.maxSteps = 100000; // prevent infinite loops
    this.initBuiltins();
  }

  initBuiltins() {
    const self = this;
    this.env.declare("println", {
      _type: "builtin",
      name: "println",
      call(args) {
        self.output(args.map(displayValue).join(" "));
        return null;
      },
    });
    this.env.declare("print", {
      _type: "builtin",
      name: "print",
      call(args) {
        self.output(args.map(displayValue).join(""));
        return null;
      },
    });
    this.env.declare("str", {
      _type: "builtin",
      name: "str",
      call(args) {
        return displayValue(args[0]);
      },
    });
    this.env.declare("int", {
      _type: "builtin",
      name: "int",
      call(args) {
        return parseInt(displayValue(args[0]));
      },
    });
    this.env.declare("float", {
      _type: "builtin",
      name: "float",
      call(args) {
        return parseFloat(displayValue(args[0]));
      },
    });
    this.env.declare("len", {
      _type: "builtin",
      name: "len",
      call(args) {
        const v = args[0];
        if (typeof v === "string") return v.length;
        if (v && v._type === "list") return v.items.length;
        return 0;
      },
    });
    this.env.declare("range", {
      _type: "builtin",
      name: "range",
      call(args) {
        const start = args.length >= 2 ? args[0] : 0;
        const end = args.length >= 2 ? args[1] : args[0];
        const step = args.length >= 3 ? args[2] : 1;
        const items = [];
        for (let i = start; step > 0 ? i < end : i > end; i += step)
          items.push(i);
        return { _type: "list", items };
      },
    });
    this.env.declare("List", {
      _type: "builtin",
      name: "List",
      call() {
        return { _type: "list", items: [] };
      },
    });
    this.env.declare("zeros", {
      _type: "builtin",
      name: "zeros",
      call(args) {
        const shape = args;
        const size = shape.reduce((a, b) => a * b, 1);
        return {
          _type: "tensor",
          data: new Array(Math.min(size, 1000)).fill(0.0),
          shape,
        };
      },
    });
    this.env.declare("ones", {
      _type: "builtin",
      name: "ones",
      call(args) {
        const shape = args;
        const size = shape.reduce((a, b) => a * b, 1);
        return {
          _type: "tensor",
          data: new Array(Math.min(size, 1000)).fill(1.0),
          shape,
        };
      },
    });
    this.env.declare("rand", {
      _type: "builtin",
      name: "rand",
      call(args) {
        const shape = args;
        const size = shape.reduce((a, b) => a * b, 1);
        const data = [];
        for (let i = 0; i < Math.min(size, 1000); i++)
          data.push(parseFloat(Math.random().toFixed(4)));
        return { _type: "tensor", data, shape };
      },
    });
    this.env.declare("assert", {
      _type: "builtin",
      name: "assert",
      call(args) {
        if (!args[0])
          throw new FerriteError("Assertion failed: " + (args[1] || ""));
        return null;
      },
    });
    this.env.declare("input", {
      _type: "builtin",
      name: "input",
      call() {
        return ""; /* no stdin in browser */
      },
    });
    this.env.declare("exit", {
      _type: "builtin",
      name: "exit",
      call() {
        throw new ReturnSignal(null);
      },
    });
    this.env.declare("abs", {
      _type: "builtin",
      name: "abs",
      call(args) {
        return Math.abs(args[0]);
      },
    });
    this.env.declare("sqrt", {
      _type: "builtin",
      name: "sqrt",
      call(args) {
        return Math.sqrt(args[0]);
      },
    });
    this.env.declare("pow", {
      _type: "builtin",
      name: "pow",
      call(args) {
        return Math.pow(args[0], args[1]);
      },
    });
    this.env.declare("shape", {
      _type: "builtin",
      name: "shape",
      call(args) {
        const v = args[0];
        if (v && v._type === "tensor") return `(${v.shape.join(", ")})`;
        return "()";
      },
    });
  }

  run(program) {
    this.stepCount = 0;
    // Pass 1: Register all top-level declarations
    for (const decl of program.decls) {
      this.registerDecl(decl);
    }

    // Pass 2: Execute top-level statements
    let lastVal = null;
    for (const decl of program.decls) {
      if (decl.type === "ExprStmt") {
        lastVal = this.evalNode(decl.expr);
      } else if (decl.type === "Keep" || decl.type === "Param") {
        const val = this.evalNode(decl.value);
        this.env.declare(decl.name, val);
      } else if (decl.type === "Constant") {
        const val = this.evalNode(decl.value);
        this.env.declare(decl.name, val);
      }
    }

    // Check for main function
    const mainFn = this.env.get("main");
    if (mainFn && mainFn._type === "func") {
      lastVal = this.callFunction(mainFn, []);
    }

    return lastVal;
  }

  registerDecl(decl) {
    switch (decl.type) {
      case "FuncDecl":
        this.env.declare(decl.name, {
          _type: "func",
          name: decl.name,
          params: decl.params,
          body: decl.body,
          closure: this.env,
        });
        break;
      case "GroupDecl":
        // Store group definition for construction
        this.env.declare(`__group_${decl.name}`, decl);
        for (const method of decl.methods) {
          const qualName = `${decl.name}::${method.name}`;
          this.env.declare(qualName, {
            _type: "func",
            name: method.name,
            params: method.params,
            body: method.body,
            closure: this.env,
            selfType: decl.name,
          });
        }
        break;
      case "EnumDecl":
        for (const variant of decl.variants) {
          if (variant.fields.length === 0) {
            // Unit variant — register as a value
            this.env.declare(variant.name, {
              _type: "enum",
              enumName: decl.name,
              variant: variant.name,
              values: [],
            });
          } else {
            // Variant constructor
            this.env.declare(variant.name, {
              _type: "builtin",
              name: `enum_${decl.name}::${variant.name}`,
              enumName: decl.name,
              variantName: variant.name,
              arity: variant.fields.length,
              call(args) {
                return {
                  _type: "enum",
                  enumName: decl.name,
                  variant: variant.name,
                  values: args,
                };
              },
            });
          }
        }
        break;
      case "ImplBlock":
        for (const method of decl.methods) {
          const qualName = `${decl.targetType}::${method.name}`;
          this.env.declare(qualName, {
            _type: "func",
            name: method.name,
            params: method.params,
            body: method.body,
            closure: this.env,
            selfType: decl.targetType,
          });
          // Also register by simple name for fallback
          if (!this.env.get(method.name)) {
            this.env.declare(method.name, {
              _type: "func",
              name: method.name,
              params: method.params,
              body: method.body,
              closure: this.env,
              selfType: decl.targetType,
            });
          }
        }
        break;
      case "TraitDecl":
      case "Import":
        // No-op in browser interpreter
        break;
    }
  }

  checkSteps() {
    if (++this.stepCount > this.maxSteps) {
      throw new FerriteError(
        "Execution limit exceeded (possible infinite loop)",
      );
    }
  }

  evalNode(node) {
    this.checkSteps();
    if (!node) return null;

    switch (node.type) {
      case "Lit":
        return node.value;

      case "Ident": {
        const val = this.env.get(node.name);
        if (val === undefined)
          throw new FerriteError(`Undefined variable '${node.name}'`);
        return val;
      }

      case "BinOp":
        return this.evalBinOp(node);

      case "UnaryOp": {
        const operand = this.evalNode(node.operand);
        if (node.op === "-")
          return typeof operand === "number" ? -operand : -operand;
        if (node.op === "!") return !this.isTruthy(operand);
        return operand;
      }

      case "Call":
        return this.evalCall(node);

      case "FieldAccess":
        return this.evalFieldAccess(node);

      case "IndexAccess": {
        const obj = this.evalNode(node.object);
        const idx = this.evalNode(node.index);
        if (obj && obj._type === "list") return obj.items[idx];
        if (typeof obj === "string") return obj[idx] || "";
        throw new FerriteError(`Cannot index into ${displayValue(obj)}`);
      }

      case "Assign": {
        const value = this.evalNode(node.value);
        if (node.target.type === "Ident") {
          if (!this.env.set(node.target.name, value)) {
            throw new FerriteError(`Undefined variable '${node.target.name}'`);
          }
          return value;
        }
        if (node.target.type === "FieldAccess") {
          const obj = this.evalNode(node.target.object);
          if (obj && obj._type === "group") {
            obj.fields[node.target.field] = value;
            return value;
          }
        }
        if (node.target.type === "IndexAccess") {
          const obj = this.evalNode(node.target.object);
          const idx = this.evalNode(node.target.index);
          if (obj && obj._type === "list") {
            obj.items[idx] = value;
            return value;
          }
        }
        return value;
      }

      case "GroupLiteral": {
        const fields = {};
        for (const f of node.fields) {
          fields[f.name] = this.evalNode(f.value);
        }
        return { _type: "group", name: node.name, fields };
      }

      case "ListLit": {
        const items = node.elements.map((e) => this.evalNode(e));
        return { _type: "list", items };
      }

      case "BlockExpr":
        return this.execBlock(node.block);

      case "If":
        return this.evalIf(node);

      case "While":
        return this.evalWhile(node);

      case "For":
        return this.evalFor(node);

      case "Match":
        return this.evalMatch(node);

      case "InferBlock":
      case "TrainBlock":
        return this.execBlock(node.block);

      case "Return":
        throw new ReturnSignal(node.value ? this.evalNode(node.value) : null);

      case "Stop":
        throw new StopSignal();

      case "Skip":
        throw new SkipSignal();

      case "Lambda":
        return {
          _type: "closure",
          params: node.params,
          body: node.body,
          closure: this.env,
        };

      default:
        return null;
    }
  }

  evalBinOp(node) {
    const lval = this.evalNode(node.left);
    // Short-circuit for && and ||
    if (node.op === "&&")
      return this.isTruthy(lval) ? this.evalNode(node.right) : lval;
    if (node.op === "||")
      return this.isTruthy(lval) ? lval : this.evalNode(node.right);

    const rval = this.evalNode(node.right);

    // String concatenation
    if (typeof lval === "string" || typeof rval === "string") {
      if (node.op === "+") return displayValue(lval) + displayValue(rval);
      if (node.op === TK.EqEq) return lval === rval;
      if (node.op === TK.BangEq) return lval !== rval;
    }

    // Numeric operations
    if (typeof lval === "number" && typeof rval === "number") {
      switch (node.op) {
        case "+":
          return lval + rval;
        case "-":
          return lval - rval;
        case "*":
          return lval * rval;
        case "/":
          if (rval === 0) throw new FerriteError("Division by zero");
          return Number.isInteger(lval) && Number.isInteger(rval)
            ? Math.trunc(lval / rval)
            : lval / rval;
        case "%":
          if (rval === 0) throw new FerriteError("Modulo by zero");
          return lval % rval;
        case TK.EqEq:
          return lval === rval;
        case TK.BangEq:
          return lval !== rval;
        case "<":
          return lval < rval;
        case ">":
          return lval > rval;
        case "<=":
          return lval <= rval;
        case ">=":
          return lval >= rval;
      }
    }

    // Boolean operations
    if (typeof lval === "boolean" && typeof rval === "boolean") {
      if (node.op === TK.EqEq) return lval === rval;
      if (node.op === TK.BangEq) return lval !== rval;
    }

    // Tensor matmul
    if (
      node.op === "@" &&
      lval &&
      lval._type === "tensor" &&
      rval &&
      rval._type === "tensor"
    ) {
      return this.tensorMatMul(lval, rval);
    }

    // Equality for enums/groups
    if (node.op === TK.EqEq) return this.valuesEqual(lval, rval);
    if (node.op === TK.BangEq) return !this.valuesEqual(lval, rval);

    throw new FerriteError(
      `Cannot apply operator '${node.op}' to ${displayValue(lval)} and ${displayValue(rval)}`,
    );
  }

  tensorMatMul(a, b) {
    if (a.shape.length !== 2 || b.shape.length !== 2)
      throw new FerriteError("MatMul requires 2D tensors");
    if (a.shape[1] !== b.shape[0])
      throw new FerriteError(`Shape mismatch: (${a.shape}) @ (${b.shape})`);
    const [m, k] = a.shape;
    const n = b.shape[1];
    const data = new Array(m * n).fill(0);
    for (let i = 0; i < m; i++) {
      for (let j = 0; j < n; j++) {
        let sum = 0;
        for (let p = 0; p < k; p++)
          sum += (a.data[i * k + p] || 0) * (b.data[p * n + j] || 0);
        data[i * n + j] = parseFloat(sum.toFixed(4));
      }
    }
    return { _type: "tensor", data, shape: [m, n] };
  }

  valuesEqual(a, b) {
    if (a === b) return true;
    if (a === null || b === null) return false;
    if (a._type === "enum" && b._type === "enum") {
      return (
        a.enumName === b.enumName &&
        a.variant === b.variant &&
        a.values.length === b.values.length &&
        a.values.every((v, i) => this.valuesEqual(v, b.values[i]))
      );
    }
    return false;
  }

  evalCall(node) {
    // Method call: obj.method(args) — FieldAccess as callee
    if (node.callee.type === "FieldAccess") {
      const obj = this.evalNode(node.callee.object);
      const methodName = node.callee.field;
      const args = node.args.map((a) => this.evalNode(a));

      // List methods
      if (obj && obj._type === "list") {
        if (methodName === "push") {
          obj.items.push(args[0]);
          return null;
        }
        if (methodName === "pop") {
          return obj.items.pop();
        }
        if (methodName === "len") {
          return obj.items.length;
        }
        if (methodName === "contains") {
          return obj.items.some((i) => this.valuesEqual(i, args[0]));
        }
        if (methodName === "map") {
          const fn = args[0];
          return {
            _type: "list",
            items: obj.items.map((item) => this.callFunction(fn, [item])),
          };
        }
        if (methodName === "filter") {
          const fn = args[0];
          return {
            _type: "list",
            items: obj.items.filter((item) =>
              this.isTruthy(this.callFunction(fn, [item])),
            ),
          };
        }
      }

      // String methods
      if (typeof obj === "string") {
        if (methodName === "len") return obj.length;
        if (methodName === "contains") return obj.includes(args[0]);
        if (methodName === "split")
          return { _type: "list", items: obj.split(args[0] || " ") };
        if (methodName === "trim") return obj.trim();
        if (methodName === "to_upper") return obj.toUpperCase();
        if (methodName === "to_lower") return obj.toLowerCase();
      }

      // Tensor methods
      if (obj && obj._type === "tensor") {
        if (methodName === "shape") return `(${obj.shape.join(", ")})`;
        if (methodName === "reshape")
          return { _type: "tensor", data: obj.data, shape: args };
      }

      // Group method dispatch
      if (obj && obj._type === "group") {
        const qualName = `${obj.name}::${methodName}`;
        const method = this.env.get(qualName);
        if (method && method._type === "func") {
          return this.callFunction(method, [obj, ...args]);
        }
      }

      // Enum methods
      if (obj && obj._type === "enum") {
        const qualName = `${obj.enumName}::${methodName}`;
        const method = this.env.get(qualName);
        if (method && method._type === "func") {
          return this.callFunction(method, [obj, ...args]);
        }
      }

      throw new FerriteError(
        `No method '${methodName}' on ${displayValue(obj)}`,
      );
    }

    // Regular function call
    const callee = this.evalNode(node.callee);
    const args = node.args.map((a) => this.evalNode(a));

    return this.callFunction(callee, args);
  }

  callFunction(fn, args) {
    if (!fn) throw new FerriteError("Cannot call null");

    // Builtin
    if (fn._type === "builtin") {
      return fn.call(args);
    }

    // User function
    if (fn._type === "func") {
      const callEnv = fn.closure ? fn.closure.child() : this.env.child();
      for (let i = 0; i < fn.params.length; i++) {
        const paramName = fn.params[i].name;
        callEnv.declare(paramName, args[i] !== undefined ? args[i] : null);
      }
      const prevEnv = this.env;
      this.env = callEnv;
      let result = null;
      try {
        result = this.execBlock(fn.body);
      } catch (e) {
        if (e instanceof ReturnSignal) {
          result = e.value;
        } else {
          this.env = prevEnv;
          throw e;
        }
      }
      this.env = prevEnv;
      return result;
    }

    // Closure
    if (fn._type === "closure") {
      const callEnv = fn.closure.child();
      for (let i = 0; i < fn.params.length; i++) {
        callEnv.declare(
          fn.params[i].name,
          args[i] !== undefined ? args[i] : null,
        );
      }
      const prevEnv = this.env;
      this.env = callEnv;
      let result = null;
      try {
        result = this.evalNode(fn.body);
      } catch (e) {
        if (e instanceof ReturnSignal) result = e.value;
        else {
          this.env = prevEnv;
          throw e;
        }
      }
      this.env = prevEnv;
      return result;
    }

    throw new FerriteError(`Cannot call ${displayValue(fn)}`);
  }

  evalFieldAccess(node) {
    const obj = this.evalNode(node.object);
    if (obj && obj._type === "group") {
      if (node.field in obj.fields) return obj.fields[node.field];
      // Check for method — return a bound method
      const qualName = `${obj.name}::${node.field}`;
      const method = this.env.get(qualName);
      if (method) return { _type: "bound_method", receiver: obj, method };
      throw new FerriteError(`No field '${node.field}' on ${obj.name}`);
    }
    if (obj && obj._type === "enum") {
      if (node.field === "variant") return obj.variant;
    }
    if (obj && obj._type === "tensor") {
      if (node.field === "shape") return `(${obj.shape.join(", ")})`;
    }
    if (typeof obj === "string") {
      if (node.field === "len") return obj.length;
    }
    if (obj && obj._type === "list") {
      if (node.field === "len") return obj.items.length;
    }
    throw new FerriteError(
      `Cannot access field '${node.field}' on ${displayValue(obj)}`,
    );
  }

  execBlock(block) {
    let lastVal = null;
    const prevEnv = this.env;
    this.env = this.env.child();
    try {
      for (const stmt of block.stmts) {
        lastVal = this.execStmt(stmt);
      }
    } finally {
      this.env = prevEnv;
    }
    return lastVal;
  }

  execStmt(stmt) {
    this.checkSteps();
    switch (stmt.type) {
      case "Keep":
      case "Param": {
        const val = this.evalNode(stmt.value);
        this.env.declare(stmt.name, val);
        return null;
      }
      case "Constant": {
        const val = this.evalNode(stmt.value);
        this.env.declare(stmt.name, val);
        return null;
      }
      case "ExprStmt":
        return this.evalNode(stmt.expr);
      case "FuncDecl":
        this.env.declare(stmt.name, {
          _type: "func",
          name: stmt.name,
          params: stmt.params,
          body: stmt.body,
          closure: this.env,
        });
        return null;
      case "GroupDecl":
      case "EnumDecl":
      case "TraitDecl":
      case "ImplBlock":
        this.registerDecl(stmt);
        return null;
      case "Import":
        return null;
      default:
        return this.evalNode(stmt);
    }
  }

  evalIf(node) {
    const condVal = this.evalNode(node.condition);
    if (this.isTruthy(condVal)) return this.execBlock(node.thenBlock);
    for (const elif of node.elifBranches) {
      if (this.isTruthy(this.evalNode(elif.condition)))
        return this.execBlock(elif.block);
    }
    if (node.elseBlock) return this.execBlock(node.elseBlock);
    return null;
  }

  evalWhile(node) {
    let lastVal = null;
    while (this.isTruthy(this.evalNode(node.condition))) {
      this.checkSteps();
      try {
        lastVal = this.execBlock(node.body);
      } catch (e) {
        if (e instanceof StopSignal) break;
        if (e instanceof SkipSignal) continue;
        throw e;
      }
    }
    return lastVal;
  }

  evalFor(node) {
    const iterable = this.evalNode(node.iterable);
    let items = [];
    if (iterable && iterable._type === "list") items = iterable.items;
    else if (typeof iterable === "string")
      items = iterable.split("").map((ch) => ch);
    else throw new FerriteError("For-in requires a list or string");

    let lastVal = null;
    for (const item of items) {
      this.checkSteps();
      const prevEnv = this.env;
      this.env = this.env.child();
      this.env.declare(node.var, item);
      try {
        lastVal = this.execBlock(node.body);
      } catch (e) {
        if (e instanceof StopSignal) {
          this.env = prevEnv;
          break;
        }
        if (e instanceof SkipSignal) {
          this.env = prevEnv;
          continue;
        }
        this.env = prevEnv;
        throw e;
      }
      this.env = prevEnv;
    }
    return lastVal;
  }

  evalMatch(node) {
    const subject = this.evalNode(node.subject);
    for (const c of node.cases) {
      const prevEnv = this.env;
      this.env = this.env.child();
      if (this.matchPattern(c.pattern, subject)) {
        if (c.guard) {
          if (!this.isTruthy(this.evalNode(c.guard))) {
            this.env = prevEnv;
            continue;
          }
        }
        const result = this.execBlock(c.body);
        this.env = prevEnv;
        return result;
      }
      this.env = prevEnv;
    }
    return null;
  }

  matchPattern(pattern, value) {
    switch (pattern.type) {
      case "Wildcard":
        return true;
      case "Binding":
        this.env.declare(pattern.name, value);
        return true;
      case "LitPattern":
        return value === pattern.value;
      case "Constructor": {
        if (!value || value._type !== "enum" || value.variant !== pattern.name)
          return false;
        if (pattern.fields.length !== value.values.length) return false;
        for (let i = 0; i < pattern.fields.length; i++) {
          if (!this.matchPattern(pattern.fields[i], value.values[i]))
            return false;
        }
        return true;
      }
      default:
        return false;
    }
  }

  isTruthy(val) {
    if (val === null || val === undefined) return false;
    if (typeof val === "boolean") return val;
    if (typeof val === "number") return val !== 0;
    if (typeof val === "string") return val.length > 0;
    if (val._type === "list") return val.items.length > 0;
    return true;
  }
}

// ── Public API ──────────────────────────────────────────────────

window.FerriteEngine = {
  run(code, outputFn) {
    try {
      const lexer = new Lexer(code);
      const tokens = lexer.tokenize();
      const parser = new Parser(tokens);
      const ast = parser.parse();
      const interp = new FerriteInterpreter(outputFn);
      interp.run(ast);
      return { success: true };
    } catch (e) {
      if (e instanceof FerriteError) {
        return { success: false, error: e.message };
      }
      return { success: false, error: `Internal Error: ${e.message}` };
    }
  },
};
