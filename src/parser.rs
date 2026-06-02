use crate::lexer::{Spanned, Token};
use crate::error::QbError;
use anyhow::Result;
use std::collections::HashMap;

// ── AST types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum QbType {
    Integer,    // %
    Single,     // ! or bare
    Double,     // #
    String,     // $
    UserType(String),
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub name:      String,
    pub ty:        QbType,
    pub dims:      Vec<Expr>,       // upper bound per dimension; empty = scalar
    pub dim_lower: Vec<Expr>,       // lower bound per dimension (parallel to dims; 0 if absent)
    pub shared:    bool,
}

#[derive(Debug)]
pub struct Program {
    pub subs:       Vec<SubDef>,
    pub functions:  Vec<FuncDef>,
    pub main_body:  Vec<Stmt>,
    /// TYPE definitions: type_name_lower → ordered [(field_name_lower, QbType)]
    pub type_defs:  HashMap<String, Vec<(String, QbType)>>,
    /// QBC transpiler directives from `REM QBC <directive>` lines (uppercased).
    pub directives: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SubDef {
    pub name:   String,
    pub params: Vec<VarDecl>,
    pub body:   Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct FuncDef {
    pub name:    String,
    pub params:  Vec<VarDecl>,
    pub ret_ty:  QbType,
    pub body:    Vec<Stmt>,
}

// ── Statements ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Stmt {
    Dim(VarDecl),
    ReDim(VarDecl),
    Let { var: LValue, expr: Expr },
    If {
        cond:            Expr,
        then_body:       Vec<Stmt>,
        elseif_branches: Vec<(Expr, Vec<Stmt>)>,
        else_body:       Option<Vec<Stmt>>,
    },
    For {
        var:  String,
        from: Expr,
        to:   Expr,
        step: Option<Expr>,
        body: Vec<Stmt>,
    },
    While { cond: Expr, body: Vec<Stmt> },
    Do    { kind: DoKind, body: Vec<Stmt> },
    Select {
        expr:    Expr,
        cases:   Vec<CaseBranch>,
        default: Option<Vec<Stmt>>,
    },
    Goto(String),
    Gosub(String),
    Return,
    Exit(ExitKind),
    Label(String),
    Print { args: Vec<PrintArg>, newline: bool },
    PrintUsing { fmt: Expr, args: Vec<Expr>, newline: bool },
    Input { prompt: Option<String>, vars: Vec<LValue> },
    Locate { row: Option<Expr>, col: Option<Expr>, cursor: Option<Expr> },
    Color  { fg:  Option<Expr>, bg:  Option<Expr> },
    /// CLS [arg]  — 0=full, 1=text-only, 2=viewport-only; default 0
    Cls(Option<Expr>),
    /// VIEW PRINT [top TO bot]  — set/reset text scrolling viewport
    ViewPrint { top: Option<Expr>, bot: Option<Expr> },
    Screen(Expr),
    Circle { x: Expr, y: Expr, r: Expr, color: Option<Expr>, step: bool },
    /// LINE [(x1,y1)]-(x2,y2)[,color][,B[F]]  — x1/y1=None means relative from gfx cursor.
    /// step1 = STEP on the first point (relative to cursor); step2 = STEP on the
    /// second point (relative to the FIRST point, per QB semantics).
    Line   { x1: Option<Expr>, y1: Option<Expr>, x2: Expr, y2: Expr, color: Option<Expr>, style: LineStyle, step1: bool, step2: bool },
    Pset   { x: Expr, y: Expr, color: Option<Expr>, preset: bool, step: bool },
    Paint  { x: Expr, y: Expr, fill: Expr, border: Option<Expr> },
    Play(Expr),
    Sound { freq: Expr, duration: Expr },
    Beep,
    Randomize(Option<Expr>),
    Palette { attr: Expr, color64: Expr },
    /// PALETTE USING arr(start) — remap all palette entries from array
    PaletteUsing(Expr),
    /// VIEW (x1,y1)-(x2,y2)[,fill[,border]] — define graphics viewport
    View { x1: Expr, y1: Expr, x2: Expr, y2: Expr, fill: Option<Expr>, border: Option<Expr> },
    /// WINDOW (x1,y1)-(x2,y2) — define logical coordinate window
    Window { x1: Expr, y1: Expr, x2: Expr, y2: Expr },
    /// SHARED name, name() inside a SUB/FUNCTION body
    SharedDecl(Vec<String>),
    /// PUT (x, y), array, PSET|XOR|...   — step = STEP on the point (cursor-relative)
    PutSprite { x: Expr, y: Expr, arr: LValue, xor_mode: bool, step: bool },
    /// GET (x1,y1)-(x2,y2), array   — step1/step2 as in Line
    GetSprite { x1: Expr, y1: Expr, x2: Expr, y2: Expr, arr: LValue, step1: bool, step2: bool },
    Swap(LValue, LValue),
    End,
    Stop,
    Call { name: String, args: Vec<Expr> },
    Data(Vec<Expr>),
    Read(Vec<LValue>),
    Restore(Option<String>),
    /// CONST name = expr  (module-level constant)
    Const { name: String, val: Expr },
    /// DEF FN single-line function  (DEF FnName(x) = expr)
    DefFn { name: String, params: Vec<VarDecl>, expr: Expr },
    /// Multiple statements from one source line (e.g. multi-var DIM)
    Block(Vec<Stmt>),

    // ── Error handling ────────────────────────────────────────────────────────
    /// ON ERROR GOTO label  (label="0" disables the handler)
    OnError { label: String },
    /// RESUME [NEXT]
    Resume { next: bool },

    // ── File I/O ──────────────────────────────────────────────────────────────
    /// OPEN path FOR mode AS [#]n [LEN = reclen]
    Open { path: Expr, mode: FileMode, file_num: Expr, rec_len: Option<Expr> },
    /// CLOSE [#n [, #m ...]]  — empty list means close all
    Close { file_nums: Vec<Expr> },
    /// FIELD [#]n, len AS var [, len AS var ...]
    Field { file_num: Expr, fields: Vec<(Expr, LValue)> },
    /// GET [#]n [, recnum]
    FileGet { file_num: Expr, record: Option<Expr> },
    /// PUT [#]n [, recnum]
    FilePut { file_num: Expr, record: Option<Expr> },
    /// LSET var = expr
    LSet { var: LValue, expr: Expr },
    /// RSET var = expr
    RSet { var: LValue, expr: Expr },
    /// PRINT #n, ...
    PrintFile { file_num: Expr, args: Vec<PrintArg>, newline: bool },
    /// INPUT #n, var [, var ...]
    InputFile { file_num: Expr, vars: Vec<LValue> },
    /// LINE INPUT #n, var$
    LineInputFile { file_num: Expr, var: LValue },
    /// WRITE #n, expr [, expr ...]
    WriteFile { file_num: Expr, args: Vec<Expr> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileMode { Input, Output, Append, Random, Binary }

#[derive(Debug, Clone)]
pub enum DoKind {
    WhilePre(Expr),
    UntilPre(Expr),
    WhilePost(Expr),
    UntilPost(Expr),
    Infinite,
}

#[derive(Debug, Clone)]
pub enum ExitKind { For, Do, Sub, Function }

#[derive(Debug, Clone)]
pub enum LineStyle { Plain, Box, FilledBox }

#[derive(Debug, Clone)]
pub struct CaseBranch {
    pub conditions: Vec<CaseCond>,
    pub body:       Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum CaseCond {
    Value(Expr),
    Range(Expr, Expr),
    Is(CmpOp, Expr),
}

#[derive(Debug, Clone)]
pub enum CmpOp { Eq, Ne, Lt, Le, Gt, Ge }

// ── Expressions ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Expr {
    IntLit(i32),
    FloatLit(f64),
    StrLit(String),
    Var(LValue),
    BinOp { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    UnOp  { op: UnOp,  operand: Box<Expr> },
    Call  { name: String, args: Vec<Expr> },
    Point { x: Box<Expr>, y: Box<Expr> },
}

#[derive(Debug, Clone)]
pub enum LValue {
    Scalar { name: String, ty: QbType },
    Index  { name: String, #[allow(dead_code)] ty: QbType, indices: Vec<Expr> },
    /// `arr(idx).field` — user-defined TYPE member access
    Field  { base: Box<LValue>, field: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, IntDiv, Pow, Mod,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or, Xor,
}

#[derive(Debug, Clone)]
pub enum UnOp { Neg, Not }

#[derive(Debug, Clone)]
pub enum PrintArg {
    Expr(Expr),
    Tab(Expr),
    Spc(Expr),
    /// Comma separator — advances to the next 14-column print zone.
    Comma,
}

// ── Parser ────────────────────────────────────────────────────────────────────

pub struct Parser {
    tokens:     Vec<Spanned>,
    pos:        usize,
    type_defs:  HashMap<String, Vec<(String, QbType)>>,
    directives: Vec<String>,
}

impl Parser {
    pub fn new(tokens: Vec<Spanned>) -> Self {
        Self { tokens, pos: 0, type_defs: HashMap::new(), directives: Vec::new() }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos].token
    }

    fn line(&self) -> u32 {
        self.tokens[self.pos].line
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos].token;
        if self.pos + 1 < self.tokens.len() { self.pos += 1; }
        t
    }

    fn expect(&mut self, expected: &Token) -> Result<()> {
        if self.peek() == expected {
            self.advance();
            Ok(())
        } else {
            Err(QbError::Parse {
                line: self.line(),
                msg: format!("expected {expected:?}, got {:?}", self.peek()),
            }.into())
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Token::Newline | Token::Colon) {
            self.advance();
        }
    }

    fn at_eol(&self) -> bool {
        matches!(self.peek(), Token::Newline | Token::Eof | Token::Colon | Token::Else)
    }

    /// The first non-Newline token after the current position.
    fn peek_next(&self) -> &Token {
        let mut i = self.pos + 1;
        while i < self.tokens.len() {
            if self.tokens[i].token != Token::Newline {
                return &self.tokens[i].token;
            }
            i += 1;
        }
        &Token::Eof
    }

    /// True when the current token is END followed by a block-closing keyword.
    /// Used to distinguish `END IF` / `END SUB` / etc. from standalone `END`.
    fn is_block_end(&self) -> bool {
        self.peek() == &Token::End
            && matches!(
                self.peek_next(),
                Token::If | Token::Sub | Token::Function
                    | Token::Select | Token::Type | Token::Eof
            )
    }

    // ── Top-level ─────────────────────────────────────────────────────────────

    pub fn parse_program(&mut self) -> Result<Program> {
        let mut main_body = Vec::new();
        let mut subs      = Vec::new();
        let mut functions = Vec::new();

        self.skip_newlines();

        while self.peek() != &Token::Eof {
            match self.peek() {
                Token::Sub      => subs.push(self.parse_sub()?),
                Token::Function => functions.push(self.parse_function()?),
                Token::Declare  => { self.parse_declare()?; }
                _ => {
                    if let Some(s) = self.parse_stmt()? {
                        main_body.push(s);
                    }
                }
            }
            self.skip_newlines();
        }

        Ok(Program {
            subs, functions, main_body,
            type_defs:  self.type_defs.clone(),
            directives: std::mem::take(&mut self.directives),
        })
    }

    fn parse_sub(&mut self) -> Result<SubDef> {
        self.expect(&Token::Sub)?;
        let name   = self.parse_ident()?;
        let params = self.parse_param_list()?;
        // Consume any trailing modifier on the header line (e.g. STATIC)
        while !self.at_eol() { self.advance(); }
        self.skip_newlines();
        let body   = self.parse_block_until(|t| matches!(t, Token::Eof))?;
        self.expect(&Token::End)?;
        self.expect(&Token::Sub)?;
        Ok(SubDef { name, params, body })
    }

    fn parse_function(&mut self) -> Result<FuncDef> {
        self.expect(&Token::Function)?;
        let (name, ret_ty) = self.parse_ident_with_sigil()?;
        let params = self.parse_param_list()?;
        // Consume any trailing modifier on the header line (e.g. STATIC)
        while !self.at_eol() { self.advance(); }
        self.skip_newlines();
        let body   = self.parse_block_until(|t| matches!(t, Token::Eof))?;
        self.expect(&Token::End)?;
        self.expect(&Token::Function)?;
        Ok(FuncDef { name, params, ret_ty, body })
    }

    fn parse_declare(&mut self) -> Result<()> {
        while !self.at_eol() { self.advance(); }
        Ok(())
    }

    // ── Statement dispatch ────────────────────────────────────────────────────

    fn parse_stmt(&mut self) -> Result<Option<Stmt>> {
        self.skip_newlines();
        // Collect QBC transpiler directives without emitting a Stmt.
        if let Token::QbcDirective(d) = self.peek().clone() {
            self.directives.push(d);
            self.advance();
            return Ok(None);
        }
        let stmt = match self.peek().clone() {
            Token::Newline | Token::Eof => return Ok(None),

            Token::Dim    => self.parse_dim(),
            Token::Common => self.parse_common(),
            Token::Static => self.parse_static(),
            Token::ReDim  => self.parse_redim(),
            Token::Let    => { self.advance(); self.parse_assign() }
            Token::If     => self.parse_if(),
            Token::For    => self.parse_for(),
            Token::While  => self.parse_while(),
            Token::Do     => self.parse_do(),
            Token::Select => self.parse_select(),
            Token::Goto   => { self.advance(); Ok(Stmt::Goto(self.parse_label()?)) }
            Token::Gosub  => { self.advance(); Ok(Stmt::Gosub(self.parse_label()?)) }
            Token::Return => { self.advance(); Ok(Stmt::Return) }
            Token::Exit   => self.parse_exit(),
            Token::Print  => self.parse_print(),
            Token::Input  => self.parse_input(),
            Token::Locate => self.parse_locate(),
            Token::Color  => self.parse_color(),
            Token::Cls    => {
                self.advance();
                // CLS [arg] — optional numeric argument (0, 1, or 2)
                let arg = if !self.at_eol() {
                    Some(self.parse_expr()?)
                } else { None };
                Ok(Stmt::Cls(arg))
            }
            Token::Screen => {
                self.advance();
                let mode = self.parse_expr()?;
                // SCREEN mode [, colorswitch [, apage [, vpage]]] — consume extra args
                while self.peek() == &Token::Comma {
                    self.advance();
                    if !self.at_eol() { self.parse_expr()?; }
                }
                Ok(Stmt::Screen(mode))
            }
            Token::Circle => self.parse_circle(),
            Token::Line   => self.parse_line_stmt(),
            Token::Pset | Token::Preset => self.parse_pset(),
            Token::Paint  => self.parse_paint(),
            Token::Play   => { self.advance(); Ok(Stmt::Play(self.parse_expr()?)) }
            Token::Sound  => self.parse_sound(),
            Token::Beep   => { self.advance(); Ok(Stmt::Beep) }
            Token::Randomize => self.parse_randomize(),
            Token::Swap   => self.parse_swap(),
            Token::End    => { self.advance(); Ok(Stmt::End) }
            Token::Stop   => { self.advance(); Ok(Stmt::Stop) }
            Token::Data   => self.parse_data(),
            Token::Read   => self.parse_read(),
            Token::Restore=> self.parse_restore(),
            Token::Call   => self.parse_call(),

            // OPTION BASE n — consume and ignore (1-indexed is our default)
            Token::Option_ => {
                while !self.at_eol() { self.advance(); }
                return Ok(None);
            }

            // ── gorilla.bas extras ────────────────────────────────────────────

            // TYPE name … END TYPE — parse field names and types
            Token::Type => {
                self.advance(); // consume TYPE
                // Read the type name — use advance_as_type_ident so keywords like
                // `Color` (tokenised as Token::Color) are accepted as type names.
                let type_name = if let Some(s) = self.advance_as_type_ident() {
                    s
                } else {
                    while !self.at_eol() { self.advance(); }
                    String::new()
                };
                while !self.at_eol() { self.advance(); } // consume rest of TYPE name line
                let mut fields: Vec<(String, QbType)> = Vec::new();
                loop {
                    self.skip_newlines();
                    if matches!(self.peek(), Token::Eof) { break; }
                    if self.peek() == &Token::End {
                        self.advance(); // END
                        if self.peek() == &Token::Type { self.advance(); } // TYPE
                        break;
                    }
                    // Parse "FieldName AS TypeName [* n]"
                    if let Token::Ident(fname) = self.peek().clone() {
                        self.advance();
                        let fname_lower = fname.to_lowercase();
                        let mut fty = QbType::Single;
                        if self.peek() == &Token::As {
                            self.advance();
                            fty = self.parse_type_name().unwrap_or(QbType::Single);
                            // STRING * n — consume the fixed-length qualifier
                            if self.peek() == &Token::Star {
                                self.advance();
                                if !self.at_eol() { let _ = self.parse_expr(); }
                            }
                        }
                        fields.push((fname_lower, fty));
                    }
                    while !self.at_eol() { self.advance(); } // consume rest of field line
                }
                if !type_name.is_empty() {
                    self.type_defs.insert(type_name, fields);
                }
                return Ok(None);
            }

            Token::Const  => self.parse_const(),
            Token::Def    => self.parse_def(),
            Token::DefInt | Token::DefSng | Token::DefDbl | Token::DefStr => {
                while !self.at_eol() { self.advance(); }
                return Ok(None);
            }
            Token::On => {
                self.advance(); // consume ON
                // ON ERROR GOTO label
                if matches!(self.peek(), Token::ErrorKw) {
                    self.advance(); // consume ERROR
                    if matches!(self.peek(), Token::Goto) { self.advance(); } // consume GOTO
                    let label = match self.peek().clone() {
                        Token::IntLit(n) => { self.advance(); n.to_string() }
                        Token::Ident(s)  => { self.advance(); s }
                        Token::IdentStr(s) | Token::IdentInt(s) => { self.advance(); s }
                        _ => "0".into(),
                    };
                    while !self.at_eol() { self.advance(); } // consume any trailing tokens
                    return Ok(Some(Stmt::OnError { label }));
                }
                // ON … other forms (ON … GOSUB, ON … GOTO) — skip
                while !self.at_eol() { self.advance(); }
                return Ok(None);
            }
            Token::Resume => {
                self.advance(); // consume RESUME
                // RESUME NEXT — skip to next statement after error site
                let next = matches!(self.peek(), Token::Ident(s) if s.eq_ignore_ascii_case("NEXT"));
                if next { self.advance(); }
                while !self.at_eol() { self.advance(); }
                return Ok(Some(Stmt::Resume { next }));
            }
            Token::Palette => {
                self.advance(); // consume PALETTE
                // PALETTE USING array — remap all palette entries from array
                if matches!(self.peek(), Token::Ident(s) if s.eq_ignore_ascii_case("USING")) {
                    self.advance(); // consume USING
                    let arr = self.parse_expr()?;
                    return Ok(Some(Stmt::PaletteUsing(arr)));
                }
                // Check if there's actually an argument (bare PALETTE resets)
                if self.at_eol() {
                    return Ok(None);
                }
                let attr = self.parse_expr()?;
                if self.peek() == &Token::Comma {
                    self.advance(); // consume comma
                    let color64 = self.parse_expr()?;
                    return Ok(Some(Stmt::Palette { attr, color64 }));
                }
                // PALETTE n (no second arg) — ignore
                return Ok(None);
            }
            Token::Poke => {
                while !self.at_eol() { self.advance(); }
                return Ok(None);
            }
            Token::Width => {
                while !self.at_eol() { self.advance(); }
                return Ok(None);
            }
            // KEY ON/OFF — IBM PC keyboard function-key display control; skip
            Token::Ident(ref s) if s.eq_ignore_ascii_case("KEY") => {
                while !self.at_eol() { self.advance(); }
                return Ok(None);
            }
            // FIELD #n, width AS Var$, ... — declare the string buffer variables
            Token::Ident(ref s) if s.eq_ignore_ascii_case("FIELD") => {
                // Skip up to the first comma (skips #n and optional RANDOM/binary info)
                while !self.at_eol() {
                    if matches!(self.peek(), Token::Comma) { self.advance(); break; }
                    self.advance();
                }
                // Parse "width AS VarName$" pairs
                let mut decls: Vec<Stmt> = Vec::new();
                while !self.at_eol() {
                    // skip width expression (anything up to AS or EOL)
                    while !self.at_eol() {
                        if matches!(self.peek(), Token::Ident(ref s) if s.eq_ignore_ascii_case("AS")) {
                            break;
                        }
                        self.advance();
                    }
                    if self.at_eol() { break; }
                    self.advance(); // consume AS
                    // FIELD vars may be lexed as IdentStr (IoDate$→IdentStr("IoDate")),
                    // IdentInt, IdentDbl, or plain Ident. Handle all sigil variants.
                    let (vname_raw, ty) = match self.peek().clone() {
                        Token::IdentStr(n) => (n.clone(), QbType::String),
                        Token::IdentInt(n) => (n.clone(), QbType::Integer),
                        Token::IdentDbl(n) => (n.clone(), QbType::Double),
                        Token::IdentSng(n) => (n.clone(), QbType::Single),
                        Token::Ident(n)    => (n.clone(), QbType::String), // undecorated → treat as string buffer
                        _ => { if matches!(self.peek(), Token::Comma) { self.advance(); } continue; }
                    };
                    self.advance(); // consume variable name
                    // Store the name WITHOUT sigil so rust_ident_typed can handle it uniformly
                    decls.push(Stmt::Dim(VarDecl {
                        name: vname_raw.clone(),
                        ty,
                        dims: Vec::new(),
                        dim_lower: Vec::new(),
                        shared: false,
                    }));
                    if matches!(self.peek(), Token::Comma) { self.advance(); }
                }
                if decls.is_empty() { return Ok(None); }
                if decls.len() == 1 { return Ok(Some(decls.remove(0))); }
                return Ok(Some(Stmt::Block(decls)));
            }

            // File I/O statements
            Token::Ident(ref s) if s.eq_ignore_ascii_case("OPEN") => {
                self.advance();
                return self.parse_open();
            }
            Token::Ident(ref s) if s.eq_ignore_ascii_case("CLOSE") => {
                self.advance();
                return self.parse_close();
            }
            Token::Ident(ref s) if s.eq_ignore_ascii_case("FIELD") => {
                self.advance();
                return self.parse_field();
            }
            Token::Ident(ref s) if s.eq_ignore_ascii_case("LSET") => {
                self.advance();
                let var = self.parse_lvalue()?;
                self.expect(&Token::Eq)?;
                let expr = self.parse_expr()?;
                return Ok(Some(Stmt::LSet { var, expr }));
            }
            Token::Ident(ref s) if s.eq_ignore_ascii_case("RSET") => {
                self.advance();
                let var = self.parse_lvalue()?;
                self.expect(&Token::Eq)?;
                let expr = self.parse_expr()?;
                return Ok(Some(Stmt::RSet { var, expr }));
            }
            Token::Ident(ref s) if s.eq_ignore_ascii_case("WRITE") => {
                self.advance();
                if self.peek() == &Token::Hash {
                    self.advance();
                    let file_num = self.parse_expr()?;
                    if self.peek() == &Token::Comma { self.advance(); }
                    let mut args = Vec::new();
                    while !self.at_eol() {
                        args.push(self.parse_expr()?);
                        if self.peek() == &Token::Comma { self.advance(); } else { break; }
                    }
                    return Ok(Some(Stmt::WriteFile { file_num, args }));
                }
                while !self.at_eol() { self.advance(); }
                return Ok(None);
            }
            // Unsupported file-related or OS statements — skip silently
            Token::Ident(ref s) if matches!(s.to_uppercase().as_str(),
                "SEEK" | "FLUSH" | "LOCK" | "UNLOCK" |
                "MKDIR" | "RMDIR" | "CHDIR" | "NAME" | "KILL") => {
                while !self.at_eol() { self.advance(); }
                return Ok(None);
            }
            Token::View => {
                self.advance(); // consume VIEW
                // VIEW PRINT [top TO bot] — text-mode scrolling viewport
                let is_view_print = matches!(self.peek(), Token::Print)
                    || matches!(self.peek(), Token::Ident(s) if s.eq_ignore_ascii_case("PRINT"));
                if is_view_print {
                    self.advance(); // consume PRINT
                    if self.at_eol() {
                        // bare VIEW PRINT → reset to full screen
                        return Ok(Some(Stmt::ViewPrint { top: None, bot: None }));
                    }
                    let top = self.parse_expr()?;
                    // expect TO
                    if matches!(self.peek(), Token::To) { self.advance(); }
                    let bot = self.parse_expr()?;
                    return Ok(Some(Stmt::ViewPrint { top: Some(top), bot: Some(bot) }));
                }
                // VIEW (x1,y1)-(x2,y2) [,fill [,border]]
                self.expect(&Token::LParen)?;
                let x1 = self.parse_expr()?; self.expect(&Token::Comma)?;
                let y1 = self.parse_expr()?; self.expect(&Token::RParen)?;
                self.expect(&Token::Minus)?;
                self.expect(&Token::LParen)?;
                let x2 = self.parse_expr()?; self.expect(&Token::Comma)?;
                let y2 = self.parse_expr()?; self.expect(&Token::RParen)?;
                let mut fill = None;
                let mut border = None;
                if self.peek() == &Token::Comma {
                    self.advance();
                    if !self.at_eol() && self.peek() != &Token::Comma {
                        fill = Some(self.parse_expr()?);
                    }
                    if self.peek() == &Token::Comma {
                        self.advance();
                        if !self.at_eol() { border = Some(self.parse_expr()?); }
                    }
                }
                return Ok(Some(Stmt::View { x1, y1, x2, y2, fill, border }));
            }
            Token::Window => {
                self.advance(); // consume WINDOW
                self.expect(&Token::LParen)?;
                let x1 = self.parse_expr()?; self.expect(&Token::Comma)?;
                let y1 = self.parse_expr()?; self.expect(&Token::RParen)?;
                self.expect(&Token::Minus)?;
                self.expect(&Token::LParen)?;
                let x2 = self.parse_expr()?; self.expect(&Token::Comma)?;
                let y2 = self.parse_expr()?; self.expect(&Token::RParen)?;
                return Ok(Some(Stmt::Window { x1, y1, x2, y2 }));
            }
            Token::Shared => {
                // Bare SHARED inside a SUB/FUNCTION body — mark names as globally shared
                self.advance(); // consume SHARED
                let mut names = Vec::new();
                loop {
                    let name = match self.peek().clone() {
                        Token::IdentStr(n) | Token::IdentInt(n) |
                        Token::IdentDbl(n) | Token::IdentSng(n) |
                        Token::Ident(n) => { self.advance(); n }
                        _ => break,
                    };
                    // consume optional () for array declarations
                    if self.peek() == &Token::LParen {
                        self.advance();
                        if self.peek() == &Token::RParen { self.advance(); }
                    }
                    names.push(name.to_lowercase());
                    if self.peek() != &Token::Comma { break; }
                    self.advance();
                }
                if names.is_empty() { return Ok(None); }
                return Ok(Some(Stmt::SharedDecl(names)));
            }
            Token::Put => self.parse_put(),
            Token::Get => self.parse_get(),

            Token::Ident(_) | Token::IdentStr(_) |
            Token::IdentInt(_) | Token::IdentSng(_) |
            Token::IdentDbl(_) => self.parse_assign_or_call(),

            // Line-number label (legacy QB line numbers)
            Token::IntLit(n) => {
                let label = n.to_string();
                self.advance();
                return Ok(Some(Stmt::Label(label)));
            }

            other => Err(QbError::Parse {
                line: self.line(),
                msg: format!("unexpected token in statement: {other:?}"),
            }.into()),
        }?;

        // consume trailing newline / colon (multi-statement line)
        while matches!(self.peek(), Token::Newline | Token::Colon) {
            self.advance();
        }

        Ok(Some(stmt))
    }

    // ── Declaration parsers ───────────────────────────────────────────────────

    fn parse_dim(&mut self) -> Result<Stmt> {
        self.expect(&Token::Dim)?;
        let shared = if self.peek() == &Token::Shared {
            self.advance(); true
        } else { false };
        let first = Stmt::Dim(self.parse_var_decl(shared)?);
        if self.peek() != &Token::Comma {
            return Ok(first);
        }
        // Multiple declarations on one line: DIM SHARED a(n), b(n), ...
        let mut stmts = vec![first];
        while self.peek() == &Token::Comma {
            self.advance();
            stmts.push(Stmt::Dim(self.parse_var_decl(shared)?));
        }
        Ok(Stmt::Block(stmts))
    }

    /// `COMMON [SHARED] var[, var...]` — in a single-module program (no CHAIN)
    /// this is functionally `DIM SHARED`: the variables become module-level
    /// globals shared with every SUB/FUNCTION. Emitted as shared `Dim` decls.
    fn parse_common(&mut self) -> Result<Stmt> {
        self.expect(&Token::Common)?;
        if self.peek() == &Token::Shared { self.advance(); }
        // COMMON vars are always shared module-wide for our purposes.
        let first = Stmt::Dim(self.parse_var_decl(true)?);
        if self.peek() != &Token::Comma { return Ok(first); }
        let mut stmts = vec![first];
        while self.peek() == &Token::Comma {
            self.advance();
            stmts.push(Stmt::Dim(self.parse_var_decl(true)?));
        }
        Ok(Stmt::Block(stmts))
    }

    /// `STATIC var[, var...] [AS type]` inside a SUB/FUNCTION — a local that
    /// retains its value across calls. Modelled as a persistent module-level
    /// shared variable (a `GameState` field) by emitting `SharedDecl`, which the
    /// analyzer promotes (synthesizing the symbol if it isn't otherwise DIM'd).
    /// Caveat: same-named STATIC locals in different procedures would alias.
    fn parse_static(&mut self) -> Result<Stmt> {
        self.expect(&Token::Static)?;
        let mut names = Vec::new();
        loop {
            let decl = self.parse_var_decl(true)?;
            names.push(decl.name.to_lowercase());
            if self.peek() != &Token::Comma { break; }
            self.advance();
        }
        Ok(Stmt::SharedDecl(names))
    }

    fn parse_redim(&mut self) -> Result<Stmt> {
        self.expect(&Token::ReDim)?;
        if self.peek() == &Token::Preserve { self.advance(); }
        let first = Stmt::ReDim(self.parse_var_decl(false)?);
        if self.peek() != &Token::Comma { return Ok(first); }
        let mut stmts = vec![first];
        while self.peek() == &Token::Comma {
            self.advance();
            stmts.push(Stmt::ReDim(self.parse_var_decl(false)?));
        }
        Ok(Stmt::Block(stmts))
    }

    fn parse_var_decl(&mut self, shared: bool) -> Result<VarDecl> {
        let (name, mut ty) = self.parse_ident_with_sigil()?;
        let mut dims      = Vec::new();
        let mut dim_lower = Vec::new();
        if self.peek() == &Token::LParen {
            self.advance();
            if self.peek() != &Token::RParen {
                let (lo, hi) = self.parse_dim_bound()?;
                dim_lower.push(lo); dims.push(hi);
                while self.peek() == &Token::Comma {
                    self.advance();
                    let (lo, hi) = self.parse_dim_bound()?;
                    dim_lower.push(lo); dims.push(hi);
                }
            }
            self.expect(&Token::RParen)?;
        }
        if self.peek() == &Token::As {
            self.advance();
            ty = self.parse_type_name()?;
            // STRING * n — fixed-length string; consume and ignore the length
            if self.peek() == &Token::Star {
                self.advance();
                if !self.at_eol() { self.parse_expr()?; }
            }
        }
        Ok(VarDecl { name, ty, dims, dim_lower, shared })
    }

    /// Parse DEF FnName(params) = expr  or  DEF SEG / DEF SEG = n (skip).
    fn parse_def(&mut self) -> Result<Stmt> {
        self.advance(); // consume DEF
        // DEF FnXxx(x) = expr
        if let Token::Ident(raw) = self.peek().clone() {
            if raw.to_uppercase().starts_with("FN") {
                self.advance(); // consume FnXxx
                let mut params = Vec::new();
                if self.peek() == &Token::LParen {
                    self.advance();
                    while self.peek() != &Token::RParen && !self.at_eol() {
                        let (pname, pty) = self.parse_ident_with_sigil()?;
                        params.push(VarDecl { name: pname, ty: pty, dims: vec![], dim_lower: vec![], shared: false });
                        if self.peek() == &Token::Comma { self.advance(); }
                    }
                    self.expect(&Token::RParen)?;
                }
                self.expect(&Token::Eq)?;
                let expr = self.parse_expr()?;
                return Ok(Stmt::DefFn { name: raw, params, expr });
            }
        }
        // DEF SEG, DEF SEG = n, or anything else — skip rest of line
        while !self.at_eol() { self.advance(); }
        Ok(Stmt::Block(vec![]))  // emit as no-op block
    }

    /// Parse a single array dimension bound, handling both `n` and `low TO high`.
    /// Returns `(lower, upper)`. When no `TO` is present, lower = IntLit(0).
    fn parse_dim_bound(&mut self) -> Result<(Expr, Expr)> {
        let first = self.parse_expr()?;
        if self.peek() == &Token::To {
            self.advance();
            let upper = self.parse_expr()?;
            Ok((first, upper))
        } else {
            Ok((Expr::IntLit(0), first))
        }
    }

    fn parse_type_name(&mut self) -> Result<QbType> {
        // First check for plain identifiers (the common case).
        if let Token::Ident(s) = self.peek().clone() {
            self.advance();
            return Ok(match s.to_uppercase().as_str() {
                "INTEGER" => QbType::Integer,
                "LONG"    => QbType::Double,   // treat LONG as f64
                "SINGLE"  => QbType::Single,
                "DOUBLE"  => QbType::Double,
                "STRING"  => QbType::String,
                other     => QbType::UserType(other.to_string()),
            });
        }
        // Then try keyword-as-type-ident (e.g. `Col AS Color` where Color is
        // tokenised as Token::Color).
        if let Some(name_lc) = self.advance_as_type_ident() {
            // The keywords we handle in advance_as_type_ident that are also
            // built-in scalar type names:
            return Ok(match name_lc.to_uppercase().as_str() {
                "INTEGER" => QbType::Integer,
                "SINGLE"  => QbType::Single,
                "DOUBLE"  => QbType::Double,
                "STRING"  => QbType::String,
                other     => QbType::UserType(other.to_uppercase()),
            });
        }
        Err(QbError::Parse {
            line: self.line(),
            msg: format!("expected type name, got {:?}", self.peek()),
        }.into())
    }

    // ── Assignment / call ─────────────────────────────────────────────────────

    fn parse_assign(&mut self) -> Result<Stmt> {
        // LET already consumed
        let lval = self.parse_lvalue()?;
        self.expect(&Token::Eq)?;
        let expr = self.parse_expr()?;
        Ok(Stmt::Let { var: lval, expr })
    }

    fn parse_assign_or_call(&mut self) -> Result<Stmt> {
        let (name, ty) = self.parse_ident_with_sigil()?;

        if self.peek() == &Token::LParen {
            self.advance();
            let mut exprs = Vec::new();
            if self.peek() != &Token::RParen {
                exprs.push(self.parse_expr()?);
                while self.peek() == &Token::Comma {
                    self.advance();
                    exprs.push(self.parse_expr()?);
                }
            }
            self.expect(&Token::RParen)?;

            if self.peek() == &Token::Dot {
                // TYPE field assignment: arr(i).Field = val  (chained dots ok)
                let mut lv = LValue::Index { name, ty, indices: exprs };
                while self.peek() == &Token::Dot {
                    self.advance();
                    let (field, _) = self.parse_ident_with_sigil()?;
                    lv = LValue::Field { base: Box::new(lv), field };
                }
                self.expect(&Token::Eq)?;
                let expr = self.parse_expr()?;
                Ok(Stmt::Let { var: lv, expr })
            } else if self.peek() == &Token::Eq {
                // array assignment: arr(i) = val
                self.advance();
                let expr = self.parse_expr()?;
                Ok(Stmt::Let { var: LValue::Index { name, ty, indices: exprs }, expr })
            } else {
                // sub call with parenthesized args
                Ok(Stmt::Call { name, args: exprs })
            }
        } else if self.peek() == &Token::Eq {
            // scalar assignment
            self.advance();
            let expr = self.parse_expr()?;
            Ok(Stmt::Let { var: LValue::Scalar { name, ty }, expr })
        } else if self.peek() == &Token::Dot {
            // name.field = expr  — scalar TYPE field assignment (chained dots ok)
            let mut lv = LValue::Scalar { name, ty };
            while self.peek() == &Token::Dot {
                self.advance();
                let (field, _) = self.parse_ident_with_sigil()?;
                lv = LValue::Field { base: Box::new(lv), field };
            }
            self.expect(&Token::Eq)?;
            let expr = self.parse_expr()?;
            Ok(Stmt::Let { var: lv, expr })
        } else if self.peek() == &Token::Colon {
            // LabelName: — consume colon, treat as label definition
            self.advance();
            Ok(Stmt::Label(name))
        } else {
            // sub call without parens: SubName arg1, arg2
            let mut args = Vec::new();
            if !self.at_eol() {
                args.push(self.parse_expr()?);
                while self.peek() == &Token::Comma {
                    self.advance();
                    args.push(self.parse_expr()?);
                }
            }
            Ok(Stmt::Call { name, args })
        }
    }

    fn parse_lvalue(&mut self) -> Result<LValue> {
        let (name, ty) = self.parse_ident_with_sigil()?;
        let base = if self.peek() == &Token::LParen {
            self.advance();
            let mut indices = Vec::new();
            indices.push(self.parse_expr()?);
            while self.peek() == &Token::Comma {
                self.advance();
                indices.push(self.parse_expr()?);
            }
            self.expect(&Token::RParen)?;
            LValue::Index { name, ty, indices }
        } else {
            LValue::Scalar { name, ty }
        };
        // Handle TYPE member access: arr(i).Field  or  s.A.B.C  (chained dots)
        let mut lv = base;
        while self.peek() == &Token::Dot {
            self.advance();
            let (field, _) = self.parse_ident_with_sigil()?;
            lv = LValue::Field { base: Box::new(lv), field };
        }
        Ok(lv)
    }

    // ── Control flow ──────────────────────────────────────────────────────────

    /// Parse a sequence of colon-separated statements for a single-line IF clause
    /// (THEN or ELSE body). Stops at Newline, Eof, Else, or when parse_stmt
    /// crosses a source-line boundary (which it does by consuming trailing Newline).
    fn parse_single_line_body(&mut self) -> Vec<Stmt> {
        // Record the source line we're on. parse_stmt() consumes trailing Newline/Colon;
        // if it moves us to a higher source line we've gone past this IF's line.
        let allowed_line = self.line();
        let mut stmts = Vec::new();
        loop {
            // Consume colon separators between statements
            while matches!(self.peek(), Token::Colon) { self.advance(); }
            // Stop at end-of-line or ELSE keyword
            if matches!(self.peek(), Token::Newline | Token::Eof | Token::Else) { break; }
            // Stop if we've drifted onto a later source line (e.g. after parse_stmt
            // consumed a trailing Newline on behalf of the previous statement).
            if self.line() > allowed_line { break; }
            match self.parse_stmt() {
                Ok(Some(s)) => stmts.push(s),
                Ok(None)    => {}
                Err(_)      => break,
            }
            // parse_stmt may have consumed a trailing Newline, advancing to the next
            // source line.  If so, we're done with this single-line body.
            if self.line() > allowed_line { break; }
        }
        stmts
    }

    fn parse_if(&mut self) -> Result<Stmt> {
        self.expect(&Token::If)?;
        let cond = self.parse_expr()?;
        self.expect(&Token::Then)?;

        if self.at_eol() {
            // Multi-line IF
            self.skip_newlines();
            let then_body = self.parse_block_until(|t| {
                matches!(t, Token::ElseIf | Token::Else | Token::Eof)
            })?;

            let mut elseif_branches = Vec::new();
            let mut else_body = None;

            loop {
                match self.peek().clone() {
                    Token::ElseIf => {
                        self.advance();
                        let ec = self.parse_expr()?;
                        self.expect(&Token::Then)?;
                        self.skip_newlines();
                        let eb = self.parse_block_until(|t| {
                            matches!(t, Token::ElseIf | Token::Else | Token::Eof)
                        })?;
                        elseif_branches.push((ec, eb));
                    }
                    Token::Else => {
                        self.advance();
                        // ELSE IF (two-word form)
                        if self.peek() == &Token::If {
                            self.advance();
                            let ec = self.parse_expr()?;
                            self.expect(&Token::Then)?;
                            self.skip_newlines();
                            let eb = self.parse_block_until(|t| {
                                matches!(t, Token::ElseIf | Token::Else | Token::Eof)
                            })?;
                            elseif_branches.push((ec, eb));
                        } else {
                            self.skip_newlines();
                            let eb = self.parse_block_until(|t| matches!(t, Token::Eof))?;
                            else_body = Some(eb);
                            break;
                        }
                    }
                    _ => break,
                }
            }

            self.expect(&Token::End)?;
            self.expect(&Token::If)?;
            Ok(Stmt::If { cond, then_body, elseif_branches, else_body })
        } else {
            // Single-line IF: IF cond THEN [line_number | stmt[:stmt...]] [ELSE ...]
            // Old BASIC: bare integer after THEN means GOTO that line.
            let then_body = if let Token::IntLit(n) = self.peek().clone() {
                let label = n.to_string();
                self.advance();
                vec![Stmt::Goto(label)]
            } else {
                self.parse_single_line_body()
            };

            let else_body = if self.peek() == &Token::Else {
                self.advance();
                // Same rule applies after ELSE
                let stmts = if let Token::IntLit(n) = self.peek().clone() {
                    let label = n.to_string();
                    self.advance();
                    vec![Stmt::Goto(label)]
                } else {
                    self.parse_single_line_body()
                };
                Some(stmts)
            } else {
                None
            };

            Ok(Stmt::If { cond, then_body, elseif_branches: Vec::new(), else_body })
        }
    }

    fn parse_for(&mut self) -> Result<Stmt> {
        self.expect(&Token::For)?;
        let (var, _ty) = self.parse_ident_with_sigil()?;
        self.expect(&Token::Eq)?;
        let from = self.parse_expr()?;
        self.expect(&Token::To)?;
        let to   = self.parse_expr()?;
        let step = if self.peek() == &Token::Step {
            self.advance();
            Some(self.parse_expr()?)
        } else { None };
        self.skip_newlines();
        let body = self.parse_block_until(|t| matches!(t, Token::Next))?;
        self.expect(&Token::Next)?;
        // optional variable name after NEXT
        if matches!(self.peek(), Token::Ident(_) | Token::IdentInt(_) |
                    Token::IdentSng(_) | Token::IdentDbl(_) | Token::IdentStr(_)) {
            self.advance();
        }
        Ok(Stmt::For { var, from, to, step, body })
    }

    fn parse_while(&mut self) -> Result<Stmt> {
        self.expect(&Token::While)?;
        let cond = self.parse_expr()?;
        self.skip_newlines();
        let body = self.parse_block_until(|t| matches!(t, Token::Wend))?;
        self.expect(&Token::Wend)?;
        Ok(Stmt::While { cond, body })
    }

    fn parse_do(&mut self) -> Result<Stmt> {
        self.expect(&Token::Do)?;

        let (pre_while, cond_pre) = match self.peek().clone() {
            Token::While => { self.advance(); (Some(true),  Some(self.parse_expr()?)) }
            Token::Until => { self.advance(); (Some(false), Some(self.parse_expr()?)) }
            _            => (None, None),
        };

        self.skip_newlines();
        let body = self.parse_block_until(|t| matches!(t, Token::Loop))?;
        self.expect(&Token::Loop)?;

        let (post_while, cond_post) = match self.peek().clone() {
            Token::While => { self.advance(); (Some(true),  Some(self.parse_expr()?)) }
            Token::Until => { self.advance(); (Some(false), Some(self.parse_expr()?)) }
            _            => (None, None),
        };

        let kind = match (pre_while, cond_pre, post_while, cond_post) {
            (Some(true),  Some(c), _, _) => DoKind::WhilePre(c),
            (Some(false), Some(c), _, _) => DoKind::UntilPre(c),
            (_, _, Some(true),  Some(c)) => DoKind::WhilePost(c),
            (_, _, Some(false), Some(c)) => DoKind::UntilPost(c),
            _                            => DoKind::Infinite,
        };

        Ok(Stmt::Do { kind, body })
    }

    fn parse_select(&mut self) -> Result<Stmt> {
        self.expect(&Token::Select)?;
        self.expect(&Token::Case)?;
        let expr = self.parse_expr()?;
        self.skip_newlines();

        let mut cases   = Vec::new();
        let mut default = None;

        while !self.is_block_end() && self.peek() != &Token::Eof {
            self.skip_newlines();
            if self.is_block_end() || self.peek() == &Token::Eof { break; }
            self.expect(&Token::Case)?;

            if self.peek() == &Token::Else {
                self.advance();
                self.skip_newlines();
                let body = self.parse_block_until(|t| matches!(t, Token::Case | Token::Eof))?;
                default = Some(body);
            } else {
                let conditions = self.parse_case_conditions()?;
                self.skip_newlines();
                let body = self.parse_block_until(|t| matches!(t, Token::Case | Token::Eof))?;
                cases.push(CaseBranch { conditions, body });
            }
        }

        self.expect(&Token::End)?;
        self.expect(&Token::Select)?;
        Ok(Stmt::Select { expr, cases, default })
    }

    fn parse_case_conditions(&mut self) -> Result<Vec<CaseCond>> {
        let mut conds = Vec::new();
        loop {
            let cond = if self.peek() == &Token::Is {
                self.advance();
                let op = match self.peek().clone() {
                    Token::Eq => { self.advance(); CmpOp::Eq }
                    Token::Ne => { self.advance(); CmpOp::Ne }
                    Token::Lt => { self.advance(); CmpOp::Lt }
                    Token::Le => { self.advance(); CmpOp::Le }
                    Token::Gt => { self.advance(); CmpOp::Gt }
                    Token::Ge => { self.advance(); CmpOp::Ge }
                    other => return Err(QbError::Parse {
                        line: self.line(),
                        msg: format!("expected comparison op after IS, got {other:?}"),
                    }.into()),
                };
                CaseCond::Is(op, self.parse_expr()?)
            } else {
                let a = self.parse_expr()?;
                if self.peek() == &Token::To {
                    self.advance();
                    CaseCond::Range(a, self.parse_expr()?)
                } else {
                    CaseCond::Value(a)
                }
            };
            conds.push(cond);
            if self.peek() == &Token::Comma { self.advance(); } else { break; }
        }
        Ok(conds)
    }

    fn parse_exit(&mut self) -> Result<Stmt> {
        self.expect(&Token::Exit)?;
        match self.peek().clone() {
            Token::For      => { self.advance(); Ok(Stmt::Exit(ExitKind::For)) }
            Token::Do       => { self.advance(); Ok(Stmt::Exit(ExitKind::Do)) }
            Token::Sub      => { self.advance(); Ok(Stmt::Exit(ExitKind::Sub)) }
            Token::Function => { self.advance(); Ok(Stmt::Exit(ExitKind::Function)) }
            other => Err(QbError::Parse {
                line: self.line(),
                msg: format!("expected FOR/DO/SUB/FUNCTION after EXIT, got {other:?}"),
            }.into()),
        }
    }

    // ── I/O ───────────────────────────────────────────────────────────────────

    fn parse_print(&mut self) -> Result<Stmt> {
        self.expect(&Token::Print)?;

        // PRINT #n, ... — file output
        if self.peek() == &Token::Hash {
            self.advance();
            let file_num = self.parse_expr()?;
            if self.peek() == &Token::Comma { self.advance(); }
            let (args, newline) = self.parse_print_args()?;
            return Ok(Stmt::PrintFile { file_num, args, newline });
        }

        // PRINT USING fmt$; arg1; arg2 ...
        if matches!(self.peek(), Token::Ident(s) if s.eq_ignore_ascii_case("USING")) {
            self.advance(); // consume USING
            let fmt = self.parse_expr()?;
            // expect ; separator
            if self.peek() == &Token::Semicolon { self.advance(); }
            let mut args = Vec::new();
            let mut newline = true;
            while !self.at_eol() {
                match self.peek() {
                    Token::Semicolon => {
                        self.advance();
                        newline = false;
                        if !self.at_eol() { newline = true; }
                    }
                    Token::Comma => { self.advance(); if self.at_eol() { newline = false; } }
                    _ => { args.push(self.parse_expr()?); newline = true; }
                }
            }
            return Ok(Stmt::PrintUsing { fmt, args, newline });
        }

        let (args, newline) = self.parse_print_args()?;
        Ok(Stmt::Print { args, newline })
    }

    /// Parse the argument list for PRINT / PRINT #n — shared helper.
    fn parse_print_args(&mut self) -> Result<(Vec<PrintArg>, bool)> {
        let mut args    = Vec::new();
        let mut newline = true;

        while !self.at_eol() {
            match self.peek().clone() {
                Token::Semicolon => {
                    self.advance();
                    newline = false;
                    if !self.at_eol() { newline = true; }
                }
                Token::Comma => {
                    self.advance();
                    // Trailing comma = no newline; mid comma = push a zone separator
                    if self.at_eol() {
                        newline = false;
                    } else {
                        // Comma advances cursor to next 14-column print zone
                        args.push(PrintArg::Comma);
                    }
                }
                Token::Ident(ref s) if s.to_uppercase() == "TAB" => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    let e = self.parse_expr()?;
                    self.expect(&Token::RParen)?;
                    args.push(PrintArg::Tab(e));
                    newline = true;
                }
                Token::Ident(ref s) if s.to_uppercase() == "SPC" => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    let e = self.parse_expr()?;
                    self.expect(&Token::RParen)?;
                    args.push(PrintArg::Spc(e));
                    newline = true;
                }
                _ => {
                    args.push(PrintArg::Expr(self.parse_expr()?));
                    newline = true;
                }
            }
        }
        Ok((args, newline))
    }

    fn parse_input(&mut self) -> Result<Stmt> {
        self.expect(&Token::Input)?;

        // INPUT #n, var [, var ...] — file input
        if self.peek() == &Token::Hash {
            self.advance();
            let file_num = self.parse_expr()?;
            if self.peek() == &Token::Comma { self.advance(); }
            let mut vars = Vec::new();
            vars.push(self.parse_lvalue()?);
            while self.peek() == &Token::Comma {
                self.advance();
                vars.push(self.parse_lvalue()?);
            }
            return Ok(Stmt::InputFile { file_num, vars });
        }

        // Optional prompt string
        let prompt = if let Token::StrLit(s) = self.peek().clone() {
            let s = s.clone();
            self.advance();
            if matches!(self.peek(), Token::Semicolon | Token::Comma) { self.advance(); }
            Some(s)
        } else if self.peek() == &Token::Semicolon {
            self.advance(); // suppress ?
            None
        } else {
            None
        };

        let mut vars = Vec::new();
        vars.push(self.parse_lvalue()?);
        while self.peek() == &Token::Comma {
            self.advance();
            vars.push(self.parse_lvalue()?);
        }
        Ok(Stmt::Input { prompt, vars })
    }

    // ── File I/O parsers ──────────────────────────────────────────────────────

    /// OPEN path FOR mode AS [#]n [LEN = reclen]
    fn parse_open(&mut self) -> Result<Option<Stmt>> {
        let path = self.parse_expr()?;
        // FOR mode
        if !matches!(self.peek(), Token::For) {
            while !self.at_eol() { self.advance(); }
            return Ok(None);
        }
        self.advance(); // FOR
        let mode = match self.peek().clone() {
            Token::Input  => { self.advance(); FileMode::Input }
            Token::Ident(s) if s.to_uppercase() == "OUTPUT" => { self.advance(); FileMode::Output }
            Token::Ident(s) if s.to_uppercase() == "APPEND" => { self.advance(); FileMode::Append }
            Token::Ident(s) if s.to_uppercase() == "RANDOM" => { self.advance(); FileMode::Random }
            Token::Ident(s) if s.to_uppercase() == "BINARY" => { self.advance(); FileMode::Binary }
            _ => { while !self.at_eol() { self.advance(); } return Ok(None); }
        };
        // AS [#]n
        if !matches!(self.peek(), Token::As) {
            while !self.at_eol() { self.advance(); }
            return Ok(None);
        }
        self.advance(); // AS
        if self.peek() == &Token::Hash { self.advance(); }
        let file_num = self.parse_expr()?;
        // optional LEN = reclen
        let rec_len = if matches!(self.peek(), Token::Ident(s) if s.to_uppercase() == "LEN") {
            self.advance();
            if self.peek() == &Token::Eq { self.advance(); }
            Some(self.parse_expr()?)
        } else { None };
        Ok(Some(Stmt::Open { path, mode, file_num, rec_len }))
    }

    /// CLOSE [#n [, #m ...]]
    fn parse_close(&mut self) -> Result<Option<Stmt>> {
        let mut file_nums = Vec::new();
        while !self.at_eol() {
            if self.peek() == &Token::Hash { self.advance(); }
            file_nums.push(self.parse_expr()?);
            if self.peek() == &Token::Comma { self.advance(); } else { break; }
        }
        Ok(Some(Stmt::Close { file_nums }))
    }

    /// FIELD [#]n, len AS var [, len AS var ...]
    fn parse_field(&mut self) -> Result<Option<Stmt>> {
        if self.peek() == &Token::Hash { self.advance(); }
        let file_num = self.parse_expr()?;
        let mut fields = Vec::new();
        while self.peek() == &Token::Comma {
            self.advance();
            let len = self.parse_expr()?;
            self.expect(&Token::As)?;
            let var = self.parse_lvalue()?;
            fields.push((len, var));
        }
        Ok(Some(Stmt::Field { file_num, fields }))
    }

    fn parse_locate(&mut self) -> Result<Stmt> {
        self.expect(&Token::Locate)?;
        let row = if !matches!(self.peek(), Token::Comma | Token::Newline | Token::Eof) {
            Some(self.parse_expr()?)
        } else { None };
        let col = if self.peek() == &Token::Comma {
            self.advance();
            if !self.at_eol() && self.peek() != &Token::Comma {
                Some(self.parse_expr()?)
            } else { None }
        } else { None };
        // 3rd arg = cursor visibility (0 = hide, non-zero = show); may be omitted.
        let cursor = if self.peek() == &Token::Comma {
            self.advance();
            if !self.at_eol() && self.peek() != &Token::Comma {
                Some(self.parse_expr()?)
            } else { None }
        } else { None };
        // 4th/5th args = cursor scan-line start/stop — parsed and discarded
        // (no effect in the windowed runtime).
        while self.peek() == &Token::Comma {
            self.advance();
            if !self.at_eol() && self.peek() != &Token::Comma { self.parse_expr()?; }
        }
        Ok(Stmt::Locate { row, col, cursor })
    }

    fn parse_color(&mut self) -> Result<Stmt> {
        self.expect(&Token::Color)?;
        let fg = if !matches!(self.peek(), Token::Comma | Token::Newline | Token::Eof) {
            Some(self.parse_expr()?)
        } else { None };
        let bg = if self.peek() == &Token::Comma {
            self.advance();
            if !self.at_eol() && self.peek() != &Token::Comma {
                Some(self.parse_expr()?)
            } else { None }
        } else { None };
        // COLOR fg, bg, border — consume any extra args (e.g. border color in text mode)
        while self.peek() == &Token::Comma {
            self.advance();
            if !self.at_eol() { self.parse_expr()?; }
        }
        Ok(Stmt::Color { fg, bg })
    }

    // ── Graphics ──────────────────────────────────────────────────────────────

    /// Consume an optional leading `STEP` keyword before a `(x,y)` coordinate
    /// pair (relative-coordinate marker). Returns true if STEP was present.
    fn opt_step(&mut self) -> bool {
        if self.peek() == &Token::Step { self.advance(); true } else { false }
    }

    fn parse_circle(&mut self) -> Result<Stmt> {
        self.expect(&Token::Circle)?;
        let step = self.opt_step();
        self.expect(&Token::LParen)?;
        let x = self.parse_expr()?;
        self.expect(&Token::Comma)?;
        let y = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::Comma)?;
        let r = self.parse_expr()?;
        let color = if self.peek() == &Token::Comma {
            self.advance();
            if !self.at_eol() && self.peek() != &Token::Comma {
                Some(self.parse_expr()?)
            } else { None }
        } else { None };
        // skip start, end, aspect args
        while self.peek() == &Token::Comma {
            self.advance();
            if !self.at_eol() && self.peek() != &Token::Comma { self.parse_expr()?; }
        }
        Ok(Stmt::Circle { x, y, r, color, step })
    }

    fn parse_line_stmt(&mut self) -> Result<Stmt> {
        self.expect(&Token::Line)?;
        // LINE INPUT "prompt"; var$ — different from LINE (x,y)-(x,y)
        if self.peek() == &Token::Input {
            self.advance();
            // LINE INPUT #n, var$  — file I/O
            if self.peek() == &Token::Hash {
                self.advance();
                let file_num = self.parse_expr()?;
                if self.peek() == &Token::Comma { self.advance(); }
                let var = self.parse_lvalue()?;
                return Ok(Stmt::LineInputFile { file_num, var });
            }
            let prompt = if let Token::StrLit(s) = self.peek().clone() {
                let s = s.clone(); self.advance();
                if matches!(self.peek(), Token::Semicolon | Token::Comma) { self.advance(); }
                Some(s)
            } else { None };
            let mut vars = Vec::new();
            vars.push(self.parse_lvalue()?);
            return Ok(Stmt::Input { prompt, vars });
        }
        // LINE -(x2,y2) — relative form: no opening (x1,y1) before the dash.
        // STEP before the first pair marks it cursor-relative.
        let mut step1 = false;
        let (x1, y1) = if self.peek() == &Token::Minus {
            (None, None)
        } else {
            step1 = self.opt_step();
            self.expect(&Token::LParen)?;
            let x1 = self.parse_expr()?;
            self.expect(&Token::Comma)?;
            let y1 = self.parse_expr()?;
            self.expect(&Token::RParen)?;
            (Some(x1), Some(y1))
        };
        self.expect(&Token::Minus)?;
        let step2 = self.opt_step();
        self.expect(&Token::LParen)?;
        let x2 = self.parse_expr()?;
        self.expect(&Token::Comma)?;
        let y2 = self.parse_expr()?;
        self.expect(&Token::RParen)?;

        let mut color = None;
        let mut style = LineStyle::Plain;

        if self.peek() == &Token::Comma {
            self.advance();
            if !self.at_eol() && self.peek() != &Token::Comma {
                color = Some(self.parse_expr()?);
            }
            if self.peek() == &Token::Comma {
                self.advance();
                if let Token::Ident(s) = self.peek().clone() {
                    match s.to_uppercase().as_str() {
                        "BF" => { self.advance(); style = LineStyle::FilledBox; }
                        "B"  => { self.advance(); style = LineStyle::Box; }
                        _    => {}
                    }
                }
            }
        }

        Ok(Stmt::Line { x1, y1, x2, y2, color, style, step1, step2 })
    }

    fn parse_pset(&mut self) -> Result<Stmt> {
        let preset = self.peek() == &Token::Preset;
        self.advance(); // consume PSET or PRESET
        let step = self.opt_step();
        self.expect(&Token::LParen)?;
        let x = self.parse_expr()?;
        self.expect(&Token::Comma)?;
        let y = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        let color = if self.peek() == &Token::Comma {
            self.advance();
            Some(self.parse_expr()?)
        } else { None };
        Ok(Stmt::Pset { x, y, color, preset, step })
    }

    fn parse_paint(&mut self) -> Result<Stmt> {
        self.expect(&Token::Paint)?;
        self.expect(&Token::LParen)?;
        let x = self.parse_expr()?;
        self.expect(&Token::Comma)?;
        let y = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        // PAINT (x,y) — fill color is optional; defaults to fg color
        let fill = if self.peek() == &Token::Comma {
            self.advance();
            if !self.at_eol() && self.peek() != &Token::Comma {
                self.parse_expr()?
            } else {
                Expr::IntLit(-1) // default: use fg color
            }
        } else {
            Expr::IntLit(-1) // no comma at all — use fg color
        };
        let border = if self.peek() == &Token::Comma {
            self.advance();
            if !self.at_eol() { Some(self.parse_expr()?) } else { None }
        } else { None };
        Ok(Stmt::Paint { x, y, fill, border })
    }

    // ── Sound / misc ──────────────────────────────────────────────────────────

    fn parse_sound(&mut self) -> Result<Stmt> {
        self.expect(&Token::Sound)?;
        let freq     = self.parse_expr()?;
        self.expect(&Token::Comma)?;
        let duration = self.parse_expr()?;
        Ok(Stmt::Sound { freq, duration })
    }

    fn parse_randomize(&mut self) -> Result<Stmt> {
        self.expect(&Token::Randomize)?;
        if self.at_eol() { return Ok(Stmt::Randomize(None)); }
        // RANDOMIZE TIMER is a common idiom — map to randomize_timer()
        if let Token::Ident(s) = self.peek().clone() {
            if s.to_uppercase() == "TIMER" {
                self.advance();
                return Ok(Stmt::Randomize(None));
            }
        }
        Ok(Stmt::Randomize(Some(self.parse_expr()?)))
    }

    fn parse_swap(&mut self) -> Result<Stmt> {
        self.expect(&Token::Swap)?;
        let a = self.parse_lvalue()?;
        self.expect(&Token::Comma)?;
        let b = self.parse_lvalue()?;
        Ok(Stmt::Swap(a, b))
    }

    // ── DATA / READ / RESTORE ─────────────────────────────────────────────────

    fn parse_data(&mut self) -> Result<Stmt> {
        self.expect(&Token::Data)?;
        let mut vals = Vec::new();
        loop {
            if self.at_eol() { break; }
            // Empty element: DATA foo,,bar — a bare comma means empty string
            if self.peek() == &Token::Comma {
                vals.push(Expr::StrLit(String::new()));
                self.advance();
                continue;
            }
            // Quoted string or numeric literal — parse normally
            let is_numeric = matches!(self.peek(),
                Token::IntLit(_) | Token::FloatLit(_) | Token::Minus | Token::Plus);
            let is_quoted = matches!(self.peek(), Token::StrLit(_));
            if is_numeric || is_quoted {
                vals.push(self.parse_expr()?);
            } else {
                // Unquoted string element: collect raw tokens until next comma or EOL.
                // QB DATA treats everything between commas as a string when not numeric.
                let mut s = String::new();
                while !self.at_eol() && self.peek() != &Token::Comma {
                    if !s.is_empty() { s.push(' '); }
                    s.push_str(&self.advance().to_data_string());
                }
                vals.push(Expr::StrLit(s.trim().to_string()));
            }
            if self.peek() == &Token::Comma { self.advance(); } else { break; }
        }
        Ok(Stmt::Data(vals))
    }

    fn parse_read(&mut self) -> Result<Stmt> {
        self.expect(&Token::Read)?;
        let mut vars = Vec::new();
        vars.push(self.parse_lvalue()?);
        while self.peek() == &Token::Comma {
            self.advance();
            vars.push(self.parse_lvalue()?);
        }
        Ok(Stmt::Read(vars))
    }

    fn parse_restore(&mut self) -> Result<Stmt> {
        self.expect(&Token::Restore)?;
        let label = if !self.at_eol() { Some(self.parse_label()?) } else { None };
        Ok(Stmt::Restore(label))
    }

    fn parse_call(&mut self) -> Result<Stmt> {
        self.expect(&Token::Call)?;
        let name = self.parse_ident()?;
        let args = if self.peek() == &Token::LParen {
            self.advance();
            let mut args = Vec::new();
            if self.peek() != &Token::RParen {
                args.push(self.parse_expr()?);
                while self.peek() == &Token::Comma {
                    self.advance();
                    args.push(self.parse_expr()?);
                }
            }
            self.expect(&Token::RParen)?;
            args
        } else {
            Vec::new()
        };
        Ok(Stmt::Call { name, args })
    }

    // ── gorilla.bas extras ────────────────────────────────────────────────────

    fn parse_const(&mut self) -> Result<Stmt> {
        self.expect(&Token::Const)?;
        let (name, _ty) = self.parse_ident_with_sigil()?;
        self.expect(&Token::Eq)?;
        let val = self.parse_expr()?;
        // CONST can have multiple declarations on one line: CONST A = 1, B = 2
        if self.peek() != &Token::Comma {
            return Ok(Stmt::Const { name, val });
        }
        let mut stmts = vec![Stmt::Const { name, val }];
        while self.peek() == &Token::Comma {
            self.advance();
            let (n2, _) = self.parse_ident_with_sigil()?;
            self.expect(&Token::Eq)?;
            let v2 = self.parse_expr()?;
            stmts.push(Stmt::Const { name: n2, val: v2 });
        }
        Ok(Stmt::Block(stmts))
    }

    /// PUT (x, y), array_var [, mode]  — sprite blit to screen
    /// PUT #n [, record] — file I/O (stub: skip)
    fn parse_put(&mut self) -> Result<Stmt> {
        self.expect(&Token::Put)?;
        // File-mode PUT: PUT #filenum [, record]
        if self.peek() == &Token::Hash {
            self.advance();
            let file_num = self.parse_expr()?;
            let record = if self.peek() == &Token::Comma {
                self.advance();
                if self.at_eol() { None } else { Some(self.parse_expr()?) }
            } else { None };
            // QB random-access record variable: `PUT #n, rec, var`. Parsed and
            // ignored (see parse_get) — without a FIELD layout the record bytes
            // aren't assembled from a TYPE variable.
            if self.peek() == &Token::Comma {
                self.advance();
                if !self.at_eol() { self.parse_expr()?; }
            }
            return Ok(Stmt::FilePut { file_num, record });
        }
        let step = self.opt_step();
        self.expect(&Token::LParen)?;
        let x = self.parse_expr()?;
        self.expect(&Token::Comma)?;
        let y = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::Comma)?;
        let arr = self.parse_lvalue()?;
        // Optional mode: PSET, XOR, AND, OR, PRESET — consume only the mode token
        let mut xor_mode = false;
        if self.peek() == &Token::Comma {
            self.advance();
            match self.peek() {
                Token::Xor  => { self.advance(); xor_mode = true; }
                Token::Pset | Token::Preset => { self.advance(); } // PSET = default, PRESET = invert
                Token::And | Token::Or      => { self.advance(); } // AND/OR blend modes
                _ => {}  // no mode or unrecognized — leave tokens for the rest of the line
            }
        }
        Ok(Stmt::PutSprite { x, y, arr, xor_mode, step })
    }

    /// GET (x1,y1)-(x2,y2), array_var  — capture screen region to array
    /// GET #n [, record]              — file I/O
    fn parse_get(&mut self) -> Result<Stmt> {
        self.expect(&Token::Get)?;
        // File-mode GET: GET #filenum [, record]
        if self.peek() == &Token::Hash {
            self.advance();
            let file_num = self.parse_expr()?;
            let record = if self.peek() == &Token::Comma {
                self.advance();
                if self.at_eol() { None } else { Some(self.parse_expr()?) }
            } else { None };
            // QB random-access record variable: `GET #n, rec, var`. Without a
            // FIELD layout the runtime can't map record bytes onto a TYPE
            // variable, so the variable is parsed and ignored (the record buffer
            // is still read; the target keeps its current/default value).
            if self.peek() == &Token::Comma {
                self.advance();
                if !self.at_eol() { self.parse_expr()?; }
            }
            return Ok(Stmt::FileGet { file_num, record });
        }
        let step1 = self.opt_step();
        self.expect(&Token::LParen)?;
        let x1 = self.parse_expr()?;
        self.expect(&Token::Comma)?;
        let y1 = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::Minus)?;
        let step2 = self.opt_step();
        self.expect(&Token::LParen)?;
        let x2 = self.parse_expr()?;
        self.expect(&Token::Comma)?;
        let y2 = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::Comma)?;
        let arr = self.parse_lvalue()?;
        Ok(Stmt::GetSprite { x1, y1, x2, y2, arr, step1, step2 })
    }

    // ── Expression parser (recursive descent, QB precedence) ─────────────────

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_xor()
    }

    fn parse_xor(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_or()?;
        while self.peek() == &Token::Xor {
            self.advance();
            let rhs = self.parse_or()?;
            lhs = Expr::BinOp { op: BinOp::Xor, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_and()?;
        while self.peek() == &Token::Or {
            self.advance();
            let rhs = self.parse_and()?;
            lhs = Expr::BinOp { op: BinOp::Or, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_not()?;
        while self.peek() == &Token::And {
            self.advance();
            let rhs = self.parse_not()?;
            lhs = Expr::BinOp { op: BinOp::And, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_not(&mut self) -> Result<Expr> {
        if self.peek() == &Token::Not {
            self.advance();
            let operand = self.parse_not()?;
            Ok(Expr::UnOp { op: UnOp::Not, operand: Box::new(operand) })
        } else {
            self.parse_compare()
        }
    }

    fn parse_compare(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_add()?;
        loop {
            let op = match self.peek() {
                Token::Eq => BinOp::Eq,
                Token::Ne => BinOp::Ne,
                Token::Lt => BinOp::Lt,
                Token::Le => BinOp::Le,
                Token::Gt => BinOp::Gt,
                Token::Ge => BinOp::Ge,
                _         => break,
            };
            self.advance();
            let rhs = self.parse_add()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Token::Plus  => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _            => break,
            };
            self.advance();
            let rhs = self.parse_mul()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_intdiv()?;
        loop {
            let op = match self.peek() {
                Token::Star  => BinOp::Mul,
                Token::Slash => BinOp::Div,
                _            => break,
            };
            self.advance();
            let rhs = self.parse_intdiv()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_intdiv(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_mod()?;
        while self.peek() == &Token::Backslash {
            self.advance();
            let rhs = self.parse_mod()?;
            lhs = Expr::BinOp { op: BinOp::IntDiv, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_mod(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_negate()?;
        while self.peek() == &Token::Mod {
            self.advance();
            let rhs = self.parse_negate()?;
            lhs = Expr::BinOp { op: BinOp::Mod, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_negate(&mut self) -> Result<Expr> {
        if self.peek() == &Token::Minus {
            self.advance();
            let operand = self.parse_negate()?;
            Ok(Expr::UnOp { op: UnOp::Neg, operand: Box::new(operand) })
        } else if self.peek() == &Token::Plus {
            self.advance(); // unary + is a no-op
            self.parse_negate()
        } else {
            self.parse_pow()
        }
    }

    fn parse_pow(&mut self) -> Result<Expr> {
        let base = self.parse_primary()?;
        if self.peek() == &Token::Caret {
            self.advance();
            let exp = self.parse_negate()?; // right-associative
            Ok(Expr::BinOp { op: BinOp::Pow, lhs: Box::new(base), rhs: Box::new(exp) })
        } else {
            Ok(base)
        }
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        match self.peek().clone() {
            Token::IntLit(n) => { self.advance(); Ok(Expr::IntLit(n)) }
            Token::FloatLit(f) => { self.advance(); Ok(Expr::FloatLit(f)) }
            Token::StrLit(s)   => { self.advance(); Ok(Expr::StrLit(s)) }

            Token::LParen => {
                self.advance();
                let e = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(e)
            }

            Token::Ident(_) | Token::IdentStr(_) | Token::IdentInt(_) |
            Token::IdentSng(_) | Token::IdentDbl(_) => {
                // Extract name and sigil separately so we can reconstruct
                // the full name (with $/#/%) for function call lookups
                let (sigil, name, lv_ty) = match self.peek().clone() {
                    Token::Ident(s)    => ("",  s, QbType::Single),
                    Token::IdentStr(s) => ("$", s, QbType::String),
                    Token::IdentInt(s) => ("%", s, QbType::Integer),
                    Token::IdentSng(s) => ("!", s, QbType::Single),
                    Token::IdentDbl(s) => ("#", s, QbType::Double),
                    _ => unreachable!(),
                };
                self.advance();
                let upper = name.to_uppercase();

                // Built-ins with no parens
                match upper.as_str() {
                    "TIMER" => return Ok(Expr::Call { name: "TIMER".into(), args: vec![] }),
                    "INKEY" => return Ok(Expr::Call { name: "INKEY$".into(), args: vec![] }),
                    // ERR — QB system variable holding the last error number
                    "ERR"   => return Ok(Expr::Call { name: "ERR".into(), args: vec![] }),
                    _ => {}
                }

                // File I/O built-in functions: EOF(n), LOF(n)
                if matches!(upper.as_str(), "EOF" | "LOF") {
                    self.expect(&Token::LParen)?;
                    let arg = self.parse_expr()?;
                    self.expect(&Token::RParen)?;
                    return Ok(Expr::Call { name: upper, args: vec![arg] });
                }

                // Binary type-conversion functions: MKD$/MKI$/MKS$/MKL$, CVD/CVI/CVS/CVL
                if matches!(upper.as_str(),
                    "MKD" | "MKI" | "MKS" | "MKL" |
                    "CVD" | "CVI" | "CVS" | "CVL") {
                    self.expect(&Token::LParen)?;
                    let arg = self.parse_expr()?;
                    self.expect(&Token::RParen)?;
                    return Ok(Expr::Call { name: upper, args: vec![arg] });
                }

                // POINT(x, y) — special form
                if upper == "POINT" {
                    self.expect(&Token::LParen)?;
                    let x = self.parse_expr()?;
                    self.expect(&Token::Comma)?;
                    let y = self.parse_expr()?;
                    self.expect(&Token::RParen)?;
                    return Ok(Expr::Point { x: Box::new(x), y: Box::new(y) });
                }

                // RND — optional dummy arg
                if upper == "RND" {
                    if self.peek() == &Token::LParen {
                        self.advance();
                        if self.peek() != &Token::RParen { self.parse_expr()?; }
                        self.expect(&Token::RParen)?;
                    }
                    return Ok(Expr::Call { name: "RND".into(), args: vec![] });
                }

                if self.peek() == &Token::LParen {
                    // function call or array index — both parse the same way
                    self.advance();
                    let mut args = Vec::new();
                    if self.peek() != &Token::RParen {
                        args.push(self.parse_expr()?);
                        while self.peek() == &Token::Comma {
                            self.advance();
                            args.push(self.parse_expr()?);
                        }
                    }
                    self.expect(&Token::RParen)?;
                    // Check for TYPE member access: arr(i).Field  (chained dots ok)
                    if self.peek() == &Token::Dot {
                        let mut lv = LValue::Index { name, ty: lv_ty, indices: args };
                        while self.peek() == &Token::Dot {
                            self.advance();
                            let (field, _) = self.parse_ident_with_sigil()?;
                            lv = LValue::Field { base: Box::new(lv), field };
                        }
                        return Ok(Expr::Var(lv));
                    }
                    // Use name+sigil so emitter's rust_fn_name() can match "LEFT$" etc.
                    let full = format!("{name}{sigil}");
                    Ok(Expr::Call { name: full, args })
                } else if self.peek() == &Token::Dot {
                    // scalar.field — user-defined TYPE member access (chained dots ok)
                    let mut lv = LValue::Scalar { name, ty: lv_ty };
                    while self.peek() == &Token::Dot {
                        self.advance();
                        let (field, _) = self.parse_ident_with_sigil()?;
                        lv = LValue::Field { base: Box::new(lv), field };
                    }
                    Ok(Expr::Var(lv))
                } else {
                    Ok(Expr::Var(LValue::Scalar { name, ty: lv_ty }))
                }
            }

            other => Err(QbError::Parse {
                line: self.line(),
                msg: format!("expected expression, got {other:?}"),
            }.into()),
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn parse_ident(&mut self) -> Result<String> {
        match self.peek().clone() {
            Token::Ident(s) => { self.advance(); Ok(s) }
            other => Err(QbError::Parse {
                line: self.line(),
                msg: format!("expected identifier, got {other:?}"),
            }.into())
        }
    }

    /// Like `parse_ident` but also accepts keyword tokens (e.g. `Color`, `Screen`)
    /// when they appear in a user-defined-type name or field-type-name position.
    /// Returns the lowercase ASCII name of the token.
    fn advance_as_type_ident(&mut self) -> Option<String> {
        use Token::*;
        let name = match self.peek() {
            Ident(s)    => s.to_lowercase(),
            IdentStr(s) => s.to_lowercase(),
            IdentInt(s) => s.to_lowercase(),
            IdentSng(s) => s.to_lowercase(),
            IdentDbl(s) => s.to_lowercase(),
            // Keywords that QBasic programs sometimes use as TYPE names
            Color    => "color".into(),
            Screen   => "screen".into(),
            Line     => "line".into(),
            Type     => "type".into(),
            Play     => "play".into(),
            Sound    => "sound".into(),
            Input    => "input".into(),
            Read     => "read".into(),
            Data     => "data".into(),
            _ => return None,
        };
        self.advance();
        Some(name)
    }

    fn parse_ident_with_sigil(&mut self) -> Result<(String, QbType)> {
        match self.peek().clone() {
            Token::IdentStr(s) => { self.advance(); Ok((s, QbType::String))  }
            Token::IdentInt(s) => { self.advance(); Ok((s, QbType::Integer)) }
            Token::IdentDbl(s) => { self.advance(); Ok((s, QbType::Double))  }
            Token::IdentSng(s) => { self.advance(); Ok((s, QbType::Single))  }
            Token::Ident(s)    => { self.advance(); Ok((s, QbType::Single))  }
            other => Err(QbError::Parse {
                line: self.line(),
                msg: format!("expected identifier, got {other:?}"),
            }.into())
        }
    }

    fn parse_label(&mut self) -> Result<String> {
        match self.peek().clone() {
            Token::Ident(s)  => { self.advance(); Ok(s) }
            Token::IntLit(n) => { self.advance(); Ok(n.to_string()) }
            other => Err(QbError::Parse {
                line: self.line(),
                msg: format!("expected label, got {other:?}"),
            }.into())
        }
    }

    fn parse_param_list(&mut self) -> Result<Vec<VarDecl>> {
        if self.peek() != &Token::LParen { return Ok(vec![]); }
        self.advance(); // consume (
        let mut params = Vec::new();
        while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
            let (name, mut ty) = self.parse_ident_with_sigil()?;
            let mut dims = Vec::new();
            if self.peek() == &Token::LParen {
                self.advance();
                self.expect(&Token::RParen)?;
                dims.push(Expr::IntLit(0)); // array param placeholder
            }
            if self.peek() == &Token::As {
                self.advance();
                ty = self.parse_type_name()?;
            }
            params.push(VarDecl { name, ty, dims, dim_lower: vec![], shared: false });
            if self.peek() == &Token::Comma { self.advance(); } else { break; }
        }
        self.expect(&Token::RParen)?;
        Ok(params)
    }

    fn parse_block_until<F>(&mut self, end: F) -> Result<Vec<Stmt>>
    where F: Fn(&Token) -> bool
    {
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            // END followed by a block-closing keyword is always a terminator,
            // regardless of the predicate — prevents standalone END (program stop)
            // from being consumed by parse_stmt inside a nested block.
            if self.is_block_end() { break; }
            if end(self.peek()) || self.peek() == &Token::Eof { break; }
            if let Some(s) = self.parse_stmt()? { stmts.push(s); }
        }
        Ok(stmts)
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn parse(tokens: Vec<Spanned>) -> Result<Program> {
    Parser::new(tokens).parse_program()
}
