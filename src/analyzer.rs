use std::collections::HashMap;
use crate::parser::{Program, SubDef, FuncDef, Stmt, Expr, QbType, BinOp, UnOp};
use anyhow::Result;

// ── Symbol table ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name:   String,
    pub ty:     QbType,
    pub dims:   usize,      // 0 = scalar, 1+ = array rank
    pub shared: bool,
}

#[derive(Debug, Default)]
pub struct Scope {
    pub symbols: HashMap<String, Symbol>,
}

impl Scope {
    pub fn insert(&mut self, sym: Symbol) {
        self.symbols.insert(sym.name.clone(), sym);
    }

    #[allow(dead_code)]
    pub fn get(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }
}

// ── Analyzed program ──────────────────────────────────────────────────────────

/// Program with type and scope information attached — fed to the emitter
#[derive(Debug)]
pub struct AnalyzedProgram {
    pub global_scope: Scope,
    pub subs:         Vec<SubDef>,
    pub functions:    Vec<FuncDef>,
    pub main_body:    Vec<Stmt>,
    #[allow(dead_code)]
    pub labels:       Vec<String>,
    pub data_store:   Vec<String>,
    /// Label name → index in data_store where that label's DATA begins (for RESTORE label)
    pub data_labels:  HashMap<String, usize>,
    /// CONST declarations, constant-folded to f64 values, in declaration order
    pub consts:       Vec<(String, f64)>,
    /// CONST declarations with string values (name_upper → value)
    pub str_consts:   Vec<(String, String)>,
    /// TYPE definitions: type_name_lower → [(field_name_lower, QbType)]
    pub type_defs:    HashMap<String, Vec<(String, QbType)>>,
    /// QBC transpiler directives from `REM QBC …` lines (uppercased).
    pub directives:   Vec<String>,
}

// ── Analyzer ──────────────────────────────────────────────────────────────────

pub struct Analyzer {
    global_scope: Scope,
    labels:       Vec<String>,
    data_store:   Vec<String>,
    data_labels:  HashMap<String, usize>,
    consts:       Vec<(String, f64)>,
    str_consts:   Vec<(String, String)>,
    const_table:  HashMap<String, f64>,
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            global_scope: Scope::default(),
            labels:       Vec::new(),
            data_store:   Vec::new(),
            data_labels:  HashMap::new(),
            consts:       Vec::new(),
            str_consts:   Vec::new(),
            const_table:  HashMap::new(),
        }
    }

    pub fn analyze(&mut self, program: Program) -> Result<AnalyzedProgram> {
        self.collect_labels(&program.main_body);

        // Warn about duplicate numeric line-number labels.  In real QB/GW-BASIC the
        // second definition silently overwrites the first; the transpiler emits both,
        // which produces different (wrong) behaviour.
        {
            let mut seen = std::collections::HashSet::new();
            for label in &self.labels {
                // Numeric labels are line numbers (e.g. "10", "20"); named labels
                // (e.g. "MyLoop") cannot be duplicated — the parser would reject them.
                if label.chars().all(|c| c.is_ascii_digit()) && !seen.insert(label.as_str()) {
                    eprintln!(
                        "warning: duplicate line number {label} — \
                         in QB the second definition replaces the first, \
                         but the transpiler emits both statements"
                    );
                }
            }
        }

        self.collect_data(&program.main_body);
        self.collect_consts(&program.main_body);
        self.collect_globals(&program.main_body)?;
        // Promote any global that is referenced by a bare SHARED statement inside a SUB/FUNCTION
        self.promote_shared_globals(&program.subs, &program.functions);

        Ok(AnalyzedProgram {
            global_scope: std::mem::take(&mut self.global_scope),
            subs:         program.subs,
            functions:    program.functions,
            main_body:    program.main_body,
            labels:       std::mem::take(&mut self.labels),
            data_store:   std::mem::take(&mut self.data_store),
            data_labels:  std::mem::take(&mut self.data_labels),
            consts:       std::mem::take(&mut self.consts),
            str_consts:   std::mem::take(&mut self.str_consts),
            type_defs:    program.type_defs,
            directives:   program.directives,
        })
    }

    fn collect_labels(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            if let Stmt::Label(l) = stmt { self.labels.push(l.clone()); }
            match stmt {
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    self.collect_labels(then_body);
                    for (_, b) in elseif_branches { self.collect_labels(b); }
                    if let Some(b) = else_body { self.collect_labels(b); }
                }
                Stmt::For    { body, .. } => self.collect_labels(body),
                Stmt::While  { body, .. } => self.collect_labels(body),
                Stmt::Do     { body, .. } => self.collect_labels(body),
                Stmt::Select { cases, default, .. } => {
                    for c in cases { self.collect_labels(&c.body); }
                    if let Some(b) = default { self.collect_labels(b); }
                }
                Stmt::Block(inner) => self.collect_labels(inner),
                _ => {}
            }
        }
    }

    fn collect_data(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match stmt {
                Stmt::Label(label) => {
                    // Record data position at this label for RESTORE label support
                    let pos = self.data_store.len();
                    self.data_labels.insert(label.to_uppercase(), pos);
                }
                Stmt::Data(vals) => {
                    for v in vals {
                        match v {
                            Expr::StrLit(s)   => self.data_store.push(s.clone()),
                            Expr::IntLit(n)   => self.data_store.push(n.to_string()),
                            Expr::FloatLit(f) => self.data_store.push(f.to_string()),
                            Expr::UnOp { op: UnOp::Neg, operand } => {
                                match operand.as_ref() {
                                    Expr::IntLit(n)   => self.data_store.push(format!("-{n}")),
                                    Expr::FloatLit(f) => self.data_store.push(format!("-{f}")),
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Stmt::Block(inner) => self.collect_data(inner),
                _ => {}
            }
        }
    }

    fn collect_consts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            if let Stmt::Const { name, val } = stmt {
                let upper = name.to_uppercase();
                if let Expr::StrLit(s) = val {
                    // String constant — stored separately
                    self.str_consts.push((upper, s.clone()));
                } else if let Some(v) = self.fold_const(val) {
                    self.const_table.insert(upper.clone(), v);
                    self.consts.push((upper, v));
                }
            }
            if let Stmt::Block(inner) = stmt {
                self.collect_consts(inner);
            }
        }
    }

    /// Evaluate a constant expression to f64, or return None if not foldable.
    fn fold_const(&self, expr: &Expr) -> Option<f64> {
        match expr {
            Expr::IntLit(n)   => Some(*n as f64),
            Expr::FloatLit(f) => Some(*f),
            Expr::Var(lv) => {
                if let crate::parser::LValue::Scalar { name, .. } = lv {
                    self.const_table.get(&name.to_uppercase()).copied()
                } else { None }
            }
            Expr::UnOp { op, operand } => {
                let v = self.fold_const(operand)?;
                Some(match op {
                    UnOp::Neg => -v,
                    UnOp::Not => if v == 0.0 { -1.0 } else { 0.0 },
                })
            }
            Expr::BinOp { op, lhs, rhs } => {
                let l = self.fold_const(lhs)?;
                let r = self.fold_const(rhs)?;
                Some(match op {
                    BinOp::Add => l + r,
                    BinOp::Sub => l - r,
                    BinOp::Mul => l * r,
                    BinOp::Div => l / r,
                    BinOp::And => ((l as i64) & (r as i64)) as f64,
                    BinOp::Or  => ((l as i64) | (r as i64)) as f64,
                    _ => return None,
                })
            }
            _ => None,
        }
    }

    fn collect_globals(&mut self, stmts: &[Stmt]) -> Result<()> {
        for stmt in stmts {
            match stmt {
                Stmt::Dim(decl) | Stmt::ReDim(decl) => {
                    self.global_scope.insert(Symbol {
                        name:   decl.name.clone(),
                        ty:     decl.ty.clone(),
                        dims:   decl.dims.len(),
                        shared: decl.shared,
                    });
                }
                Stmt::Block(inner) => self.collect_globals(inner)?,
                _ => {}
            }
        }
        Ok(())
    }

    /// Walk all SUB/FUNCTION bodies for `SHARED name, arr()` declarations,
    /// then mark those global symbols as shared so they end up in GameState.
    fn promote_shared_globals(
        &mut self,
        subs: &[crate::parser::SubDef],
        fns:  &[crate::parser::FuncDef],
    ) {
        let mut shared_names: Vec<String> = Vec::new();
        for sub in subs  { self.collect_shared_decls(&sub.body,  &mut shared_names); }
        for f   in fns   { self.collect_shared_decls(&f.body,    &mut shared_names); }
        for name_lc in shared_names {
            // symbol keys keep original case — find by lowercase comparison
            let mut matched = false;
            for sym in self.global_scope.symbols.values_mut() {
                if sym.name.to_lowercase() == name_lc {
                    sym.shared = true;
                    matched = true;
                }
            }
            if !matched {
                // SHARED variable with no DIM in main scope (e.g. mandel's
                // ColorRange, an implicit variable set via a by-ref SUB param).
                // QB shares it module-wide, so synthesize a shared symbol — it
                // then becomes a GameState field and both scopes see one storage.
                // Arrays must be DIM'd to be SHARED, so an unmatched name is a
                // scalar; all numerics are f64, so QbType::Single is correct.
                self.global_scope.insert(Symbol {
                    name:   name_lc.clone(),
                    ty:     QbType::Single,
                    dims:   0,
                    shared: true,
                });
            }
        }
    }

    fn collect_shared_decls(&self, stmts: &[Stmt], names: &mut Vec<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::SharedDecl(ns) => names.extend(ns.iter().cloned()),
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    self.collect_shared_decls(then_body, names);
                    for (_, b) in elseif_branches { self.collect_shared_decls(b, names); }
                    if let Some(b) = else_body { self.collect_shared_decls(b, names); }
                }
                Stmt::For   { body, .. } |
                Stmt::While { body, .. } |
                Stmt::Do    { body, .. } => self.collect_shared_decls(body, names),
                Stmt::Select { cases, default, .. } => {
                    for c in cases { self.collect_shared_decls(&c.body, names); }
                    if let Some(b) = default { self.collect_shared_decls(b, names); }
                }
                Stmt::Block(inner) => self.collect_shared_decls(inner, names),
                _ => {}
            }
        }
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn analyze(program: Program) -> Result<AnalyzedProgram> {
    Analyzer::new().analyze(program)
}
