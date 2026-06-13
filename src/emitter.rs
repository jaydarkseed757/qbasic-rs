use std::collections::{HashSet, HashMap};
use crate::analyzer::{AnalyzedProgram, Scope};
use crate::parser::*;
use anyhow::Result;

// ── QBC pragma directives ─────────────────────────────────────────────────────

/// Parsed result of all `REM QBC <directive>` lines in the source file.
#[derive(Default)]
struct QbcConfig {
    fullspeed: bool,
    fps:       Option<f64>,    // REM QBC FPS <n>
    pace:      Option<f64>,    // REM QBC PACE <n>  — sleep-paced watchable draw
    slowmo:    Option<f64>,    // REM QBC SLOWMO <n>
    title:     Option<String>, // REM QBC TITLE <text>
    scale:     Option<u32>,    // REM QBC SCALE <n>
}

/// Parse the directive strings collected by the lexer/parser into a QbcConfig.
/// Each string is the uppercased text after "QBC", e.g. "FULLSPEED", "FPS 30",
/// "PACE 20", "TITLE My Game", "SCALE 2".
fn parse_qbc_config(directives: &[String]) -> QbcConfig {
    let mut c = QbcConfig::default();
    for d in directives {
        let mut parts = d.splitn(2, ' ');
        let kw   = parts.next().unwrap_or("").trim();
        let rest = parts.next().unwrap_or("").trim();
        match kw {
            "FULLSPEED" => c.fullspeed = true,
            "FPS"    => { if let Ok(v) = rest.parse::<f64>() { c.fps    = Some(v); } }
            "PACE"   => { if let Ok(v) = rest.parse::<f64>() { c.pace   = Some(v); } }
            "SLOWMO" => { if let Ok(v) = rest.parse::<f64>() { c.slowmo = Some(v); } }
            "TITLE"  => { if !rest.is_empty() { c.title = Some(rest.to_string()); } }
            "SCALE"  => { if let Ok(v) = rest.parse::<u32>() { c.scale  = Some(v); } }
            _ => {}
        }
    }
    c
}

// ── Emitter ───────────────────────────────────────────────────────────────────

pub struct Emitter {
    out:          String,
    indent:       usize,
    /// DIM SHARED scalar/array names (lowercase) — accessed via __gs
    shared_names: HashSet<String>,
    /// Names originally declared with DIM SHARED (not just promoted via SHARED in a sub).
    /// Always visible in every sub without an explicit SHARED declaration.
    dim_shared_names: HashSet<String>,
    /// All DIM'd array names in global scope (lowercase)
    array_names:  HashSet<String>,
    /// User-defined SUB + FUNCTION names (lowercase, sigil-stripped)
    user_fns:     HashSet<String>,
    /// Local DIM'd array names for the current function body
    local_arrays: HashSet<String>,
    /// Counter for generated temp var names
    lift_counter: usize,
    /// True when emitting fn main() — affects &mut vs plain for __rt/__gs args
    in_main:      bool,
    /// Names of &mut String parameters in current function (lowercase)
    str_params:   HashSet<String>,
    /// Maps lowercase array base name → list of field names (from TYPE field accesses)
    typed_fields: HashMap<String, Vec<String>>,
    /// Maps lowercase type name → ordered [(field_name_lower, QbType)] from TYPE definitions
    type_defs: HashMap<String, Vec<(String, QbType)>>,
    /// Maps lowercase type name → ordered [(field_name_lower, FieldRepr)] — the
    /// on-disk byte layout used for random-access record (GET/PUT #n,rec,var) I/O.
    type_layouts: HashMap<String, Vec<(String, FieldRepr)>>,
    /// Array-field dims from TYPE bodies: type_name_lower → field_name_lower → upper_bound.
    type_field_dims: HashMap<String, HashMap<String, usize>>,
    /// Maps lowercase array/var name → type name (from `DIM x AS TypeName`)
    var_type_name: HashMap<String, String>,
    /// Names of simple (non-TYPE) array parameters for current function
    array_params: HashSet<String>,
    /// Names of numeric scalar parameters for current function (passed as &mut f64)
    numeric_params: HashSet<String>,
    /// Arrays promoted to GameState because they cross GOSUB function boundaries.
    /// These get a `name: Vec<f64>` field in GameState even if not DIM SHARED.
    promoted_arrays: HashSet<String>,
    /// Scalars promoted to GameState because they cross GOSUB function boundaries.
    /// Each entry is (rust_field_name, QbType). The shared_names key is the
    /// lowercase original name (already inserted into shared_names separately).
    promoted_scalars: Vec<(String, QbType)>,
    /// Track which REDIM'd local arrays have already been declared (`let mut name: Vec<T>`)
    /// so we don't emit duplicate declarations on the second REDIM in the same sub.
    redim_declared: HashSet<String>,
    /// Label name (uppercase) → DATA store index, for RESTORE label
    data_labels: HashMap<String, usize>,
    /// sub/fn name (lowercase) → ordered param list, for typed-array call-site expansion
    sub_params: HashMap<String, Vec<VarDecl>>,
    /// lowercase array name → number of dimensions (0 if not recorded)
    typed_array_dims: HashMap<String, usize>,
    /// lowercase array name → number of dimensions, for ALL arrays (plain + typed).
    /// Used by ERASE to emit the right loop-nesting depth.
    array_dims: HashMap<String, usize>,
    /// lowercase array name → lower bound per dimension (i64; 0 when not specified)
    array_lower: HashMap<String, Vec<i64>>,
    /// GOSUB blocks for the current sub being emitted: label_lower → body stmts.
    /// When non-empty, Stmt::Gosub emits inline via Rust labeled-loop blocks.
    current_sub_gosubs: HashMap<String, Vec<crate::parser::Stmt>>,
    /// When Some(label), we are inside an inline-emitted GOSUB block.
    /// Stmt::Return emits `break '__gosub_LABEL;` instead of `return;`.
    current_gosub_label: Option<String>,
    /// When emitting a FUNCTION body, this is the Rust variable name that
    /// holds the return value (always "__fn_ret").  Used by EXIT FUNCTION.
    current_fn_retvar: Option<String>,
    /// When emitting a FUNCTION body, this is the rust-ident of the function
    /// name (e.g. "factorial").  Assignments to this name in emit_lvalue are
    /// redirected to "__fn_ret" so the local var can't shadow recursive calls.
    current_fn_name_lc: Option<String>,
    /// Rust-ident names of user-defined SUBs (not FUNCTIONs).
    /// Used in Stmt::Call to pick emit_call_args_with_wb (SUB) vs plain args (FUNCTION).
    user_subs: HashSet<String>,
    /// lowercase name → QbType for DIM SHARED variables (arrays and scalars).
    /// Used by emit_lvalue to get the correct type when the AST LValue has a stale type.
    shared_types: HashMap<String, QbType>,
    /// When true, Stmt::Goto emits `{ __pc = N; continue '__sm; }` instead of a comment.
    sm_mode: bool,
    /// Parsed QBC pragma config — populated from prog.directives before emit_main().
    qbc: QbcConfig,
    /// Maps named GOTO target label (QB name, any case) → Rust loop label (e.g. "'_loop_0").
    /// Populated while emitting DO loops that have labels at their tail that are
    /// GOTO targets from inside the body.  Lets "GOTO SkipGuess" emit "continue '_loop_0;".
    named_loop_labels: HashMap<String, String>,
    /// Counter used to generate unique Rust loop labels.
    loop_label_counter: usize,
    /// File I/O: for each open file number, the FIELD layout as
    /// (rust_var_name, byte_offset, byte_length) entries.
    /// Populated by Stmt::Field; consumed by Stmt::FileGet / Stmt::FilePut.
    file_fields: HashMap<u8, Vec<(String, usize, usize)>>,
    /// ON ERROR GOTO target label (uppercase).  When non-empty, the emitter
    /// appends an error-dispatch check after each fallible statement (OPEN,
    /// SCREEN) that jumps to this label if __rt.error_pending is set.
    on_error_label: String,
    /// True once a GameState struct has been emitted. When the struct would be
    /// empty AND the program has no SUB/FUNCTION/GOSUB-fn/DEF-FN (all of which
    /// take `__gs: &mut GameState`), it is suppressed and the `__gs` binding in
    /// main is skipped. Set by emit_game_state, read when emitting main.
    gamestate_emitted: bool,
    /// Bare-lowercase names of local string arrays (non-shared, `DIM name(...) AS STRING`).
    /// Used by emit_lvalue to emit the correct `_s`-suffixed name for string array accesses
    /// that were parsed without the `$` sigil.
    local_string_arrays: HashSet<String>,
    /// Bare-lowercase names of scalars that have an explicit local `DIM name`
    /// inside the current SUB/FUNCTION body.  Cleared on entry to each sub/fn.
    /// Used by emit_lvalue to detect the case where a local integer `B` and a
    /// DIM SHARED string `B$` share the same base name: the local DIM shadows
    /// the shared string for numeric accesses in that scope.
    local_dim_names: HashSet<String>,
    /// Collected ON KEY(n) GOSUB target bindings from all scopes.
    /// key_num → target label (lowercase).  Populated during emit; used to emit
    /// `fn __handle_key_event` before fn main().
    on_key_gosubs: Vec<(f64, String)>,
    /// ON TIMER(secs) GOSUB target — interval + target label (lowercase).
    on_timer_gosub: Option<(f64, String)>,
}

impl Emitter {
    pub fn new() -> Self {
        Self {
            out:          String::new(),
            indent:       0,
            shared_names: HashSet::new(),
            dim_shared_names: HashSet::new(),
            array_names:  HashSet::new(),
            user_fns:     HashSet::new(),
            local_arrays: HashSet::new(),
            lift_counter: 0,
            in_main:      true,
            str_params:   HashSet::new(),
            typed_fields: HashMap::new(),
            type_defs:       HashMap::new(),
            type_layouts:    HashMap::new(),
            type_field_dims: HashMap::new(),
            var_type_name: HashMap::new(),
            array_params: HashSet::new(),
            numeric_params: HashSet::new(),
            promoted_arrays: HashSet::new(),
            promoted_scalars: Vec::new(),
            redim_declared: HashSet::new(),
            data_labels:  HashMap::new(),
            sub_params:   HashMap::new(),
            typed_array_dims: HashMap::new(),
            array_dims:   HashMap::new(),
            array_lower:  HashMap::new(),
            current_sub_gosubs: HashMap::new(),
            current_gosub_label: None,
            current_fn_retvar: None,
            current_fn_name_lc: None,
            user_subs: HashSet::new(),
            shared_types: HashMap::new(),
            sm_mode: false,
            qbc: QbcConfig::default(),
            named_loop_labels: HashMap::new(),
            loop_label_counter: 0,
            file_fields: HashMap::new(),
            on_error_label: String::new(),
            gamestate_emitted: true,
            local_string_arrays: HashSet::new(),
            local_dim_names: HashSet::new(),
            on_key_gosubs: Vec::new(),
            on_timer_gosub: None,
        }
    }

    /// "&mut __rt, &mut __gs" in main; "__rt, __gs" inside subs/fns (auto-reborrow).
    fn rt_args(&self) -> &'static str {
        if self.in_main { "&mut __rt, &mut __gs" } else { "__rt, __gs" }
    }

    /// Emit an ON ERROR dispatch check after a fallible statement.
    /// If a handler label is active and __rt.error_pending is set, clears
    /// the flag and calls the handler (if it was extracted as a gosub fn).
    fn emit_error_dispatch(&mut self) {
        if self.on_error_label.is_empty() { return; }
        let lbl = self.on_error_label.clone();
        let rust_lbl = rust_ident(&lbl);
        // Only dispatch if the handler label was extracted as a callable fn.
        if self.user_fns.contains(&rust_lbl) {
            let call_args = self.rt_args();
            self.line(&format!(
                "if __rt.error_pending {{ __rt.error_pending = false; {rust_lbl}({call_args}); }}"
            ));
        } else {
            // Handler not callable (state-machine only, or disabled) —
            // clear the error so execution continues gracefully.
            self.line("if __rt.error_pending { __rt.error_pending = false; }");
        }
    }

    /// Return the i-th dimension lower bound for a named array (0 when unset).
    fn arr_lo(&self, name_lc: &str, dim: usize) -> i64 {
        self.array_lower.get(name_lc)
            .and_then(|v| v.get(dim))
            .copied()
            .unwrap_or(0)
    }

    /// Format a single subscript dimension with lower-bound offset applied.
    /// `idx_expr` is already an emitted Rust expression string.
    #[allow(dead_code)]
    fn dim_sub(idx_expr: &str, lo: i64) -> String {
        if lo == 0 {
            format!("[({idx_expr}) as usize]")
        } else {
            format!("[({idx_expr} - {lo}.0) as usize]")
        }
    }

    /// Context-aware string-type check that can look up TYPE field types from type_defs
    /// and shared array types from shared_types.
    fn is_str_expr_ctx(&self, expr: &Expr) -> bool {
        if is_str_expr(expr) { return true; }
        if let Expr::Var(LValue::Scalar { name, .. }) = expr {
            let lc = name.to_lowercase();
            // String param declared with `AS STRING` (no sigil): name_s is in str_params.
            let rn_s = rust_ident_typed(name, &QbType::String);
            if self.str_params.contains(&rn_s) { return true; }
            // Shared scalar variable parsed with wrong type (e.g. `Available AS STRING` parsed
            // as Integer under DEFINT A-Z) — look up authoritative type from shared_types.
            // Exception: if the current scope has an explicit local DIM for this name with
            // a numeric type (e.g. DIM B AS INTEGER when B$ is DIM SHARED), the local
            // declaration shadows the shared string — do NOT treat as string.
            if !self.local_dim_names.contains(&lc) {
                if let Some(ty) = self.shared_types.get(&lc) {
                    if *ty == QbType::String { return true; }
                }
            }
        }
        // Shared or local array element accessed via Expr::Call (parser didn't add $ sigil):
        // e.g. OptionTitle(I) where OptionTitle is DIM SHARED AS STRING
        if let Expr::Call { name, .. } = expr {
            let lc = name.to_lowercase();
            let name_bare = name.trim_end_matches(['$', '%', '!', '#', '&']).to_lowercase();
            if let Some(ty) = self.shared_types.get(&lc) {
                if *ty == QbType::String { return true; }
            }
            if self.local_string_arrays.contains(&name_bare) { return true; }
        }
        // TYPE field access: Account.Title(x) → look up AccountType.title
        if let Expr::Var(LValue::Field { base, field }) = expr {
            let arr_name = match base.as_ref() {
                LValue::Index { name, .. } | LValue::Scalar { name, .. } => {
                    rust_ident(name)
                }
                _ => return false,
            };
            if let Some(type_name) = self.var_type_name.get(&arr_name) {
                if let Some(fields) = self.type_defs.get(type_name.as_str()) {
                    let flow = field.to_lowercase();
                    if let Some((_, ty)) = fields.iter().find(|(f, _)| *f == flow) {
                        return ty == &QbType::String;
                    }
                }
            }
        }
        false
    }

    /// Emit a call to a user-defined sub or function: `name(rt_args, user_args)`.
    fn user_call(&self, name: &str, args: &str) -> String {
        let sep = if args.is_empty() { "" } else { ", " };
        format!("{}({}{sep}{args})", name, self.rt_args())
    }

    fn line(&mut self, s: &str) {
        let pad = "    ".repeat(self.indent);
        self.out.push_str(&pad);
        self.out.push_str(s);
        self.out.push('\n');
    }

    fn blank(&mut self) { self.out.push('\n'); }
    fn indent(&mut self)  { self.indent += 1; }
    fn dedent(&mut self)  { self.indent = self.indent.saturating_sub(1); }

    // ── Top-level emit ────────────────────────────────────────────────────────

    pub fn emit(&mut self, prog: &AnalyzedProgram) -> Result<String> {
        // Populate name sets used throughout the emitter
        for sym in prog.global_scope.symbols.values() {
            let n = sym.name.to_lowercase();
            if sym.shared  {
                self.shared_names.insert(n.clone());
                self.shared_types.insert(n.clone(), sym.ty.clone());
            }
            if sym.dims > 0 { self.array_names.insert(n); }
        }
        // Collect DIM SHARED names (declared with the SHARED keyword on the DIM itself,
        // not promoted later by SHARED-in-sub).  These are always visible in every SUB
        // without needing an explicit SHARED declaration in that sub's body.
        self.dim_shared_names = collect_dim_shared_names(&prog.main_body);
        // user_fns: sigil-stripped lowercase names (both bare and typed so $-returning
        // functions like Trim$ resolve to either "trim" or "trim_s" at call sites)
        for s in &prog.subs      {
            self.user_fns.insert(rust_ident(&s.name));
            self.user_subs.insert(rust_ident(&s.name));
        }
        for f in &prog.functions {
            self.user_fns.insert(rust_ident(&f.name));
            self.user_fns.insert(rust_ident_typed(&f.name, &f.ret_ty));
        }

        // Build the exclude set for main-body locals (globals + consts).
        // Use rust_ident so keyword-prefixed names like qb_true/qb_false are
        // excluded and collect_locals won't try to re-declare them.
        let mut globals: HashSet<String> = prog.global_scope.symbols
            .keys().map(|k| rust_ident(k)).collect();
        for (name, _) in &prog.consts { globals.insert(rust_ident(name)); }
        for (name, _) in &prog.str_consts { globals.insert(rust_ident(name)); }

        // Separate exclude set for sub/function bodies: only constants and user
        // sub/fn names.  Global scope variables that are NOT shared by a specific
        // sub are treated as locals in that sub (QB semantics: DIM without SHARED
        // does not implicitly expose a variable inside subs — only `SHARED x` in
        // the sub body does).  The per-sub shared_names are added below per-sub.
        let mut const_globals: HashSet<String> = HashSet::new();
        for (name, _) in &prog.consts    { const_globals.insert(rust_ident(name)); }
        for (name, _) in &prog.str_consts { const_globals.insert(rust_ident(name)); }
        for s in &prog.subs      { const_globals.insert(rust_ident(&s.name)); }
        for f in &prog.functions {
            const_globals.insert(rust_ident(&f.name));
            const_globals.insert(rust_ident_typed(&f.name, &f.ret_ty));
        }

        // Store parsed TYPE definitions (field names + types)
        self.type_defs       = prog.type_defs.clone();
        self.type_layouts    = prog.type_layouts.clone();
        self.type_field_dims = prog.type_field_dims.clone();

        // Build var_type_name: for every DIM/REDIM with AS UserType, record the type name
        collect_var_type_names(prog, &mut self.var_type_name);

        // Merge typed_fields from type_defs: for every known UserType var, populate its fields
        // Use flatten_type_fields so nested TYPEs are recursively expanded.
        for (var_lower, type_name) in &self.var_type_name.clone() {
            let flat = flatten_type_fields(type_name.as_str(), &self.type_defs.clone());
            if !flat.is_empty() {
                let field_names: Vec<String> = flat.into_iter().map(|(f, _)| f).collect();
                self.typed_fields.entry(var_lower.clone()).or_insert_with(|| field_names);
            }
        }

        // Pre-collect TYPE field names + dimension counts from all function bodies
        let (fields, access_dims) = collect_typed_array_fields(prog);
        // Merge (don't replace — type_defs-derived fields take precedence if complete)
        for (k, v) in fields { self.typed_fields.entry(k).or_insert(v); }
        self.typed_array_dims = access_dims;

        // Override/supplement with dim counts from global scope symbols
        for sym in prog.global_scope.symbols.values() {
            if matches!(&sym.ty, QbType::UserType(_)) && sym.dims > 0 {
                self.typed_array_dims.insert(sym.name.to_lowercase(), sym.dims);
            }
            // Record dimensionality of every array (plain + typed) for ERASE.
            if sym.dims > 0 {
                self.array_dims.insert(sym.name.to_lowercase(), sym.dims);
            }
        }

        // Build sub/fn → param list table for typed-array call-site expansion
        for s in &prog.subs      { self.sub_params.insert(rust_ident(&s.name), s.params.clone()); }
        for f in &prog.functions { self.sub_params.insert(rust_ident(&f.name), f.params.clone()); }

        // Collect ON KEY/TIMER GOSUB targets from all SUB/FUNCTION bodies.
        // These targets are labels in the main body that must be extracted as
        // gosub functions even if no explicit GOSUB statement references them.
        let mut event_gosub_targets: HashSet<String> = HashSet::new();
        for sub in &prog.subs {
            collect_event_gosub_targets_from_stmts(&sub.body, &mut event_gosub_targets);
        }
        for func in &prog.functions {
            collect_event_gosub_targets_from_stmts(&func.body, &mut event_gosub_targets);
        }

        // Extract GOSUB-target label blocks from main body
        let (main_stmts, gosub_fns) = extract_gosub_blocks(&prog.main_body, &event_gosub_targets);

        // Detect arrays that cross GOSUB function boundaries (declared in one scope,
        // used in another) and promote them to GameState so both scopes can access them.
        let cross = detect_cross_boundary_arrays(&main_stmts, &gosub_fns);
        for name in &cross {
            self.shared_names.insert(name.clone());
            self.promoted_arrays.insert(name.clone());
            // Also populate shared_types so emit_expr_inner and emit_game_state can use
            // the correct type (e.g. String → _s suffix, correct Vec element in GameState).
            if let Some(sym) = prog.global_scope.symbols.values()
                                    .find(|s| rust_ident(&s.name) == *name) {
                self.shared_types.insert(name.clone(), sym.ty.clone());
            }
        }

        // Collect all parameter names from named SUBs/FUNCTIONs so we can exclude
        // them from cross-boundary scalar promotion.  Variables passed explicitly as
        // &mut parameters are already shared via the parameter mechanism — promoting
        // them to GameState would cause a double-borrow of __gs at call sites.
        let sub_param_names: HashSet<String> = prog.subs.iter()
            .flat_map(|s| s.params.iter())
            .map(|p| p.name.to_lowercase())
            .collect();

        // CONST names are compile-time values, never variables — they must resolve
        // to the emitted `const` item, NOT be promoted into GameState. (e.g. a
        // program with `CONST TRUE = -1` referenced across scopes would otherwise
        // get a `qb_true` GameState field shadowing the const with a 0.0 default.)
        let const_names: HashSet<String> = prog.consts.iter().map(|(n, _)| n.to_lowercase())
            .chain(prog.str_consts.iter().map(|(n, _)| n.to_lowercase()))
            .collect();

        // Detect scalars that cross GOSUB function boundaries and promote them
        // to GameState so the GOSUB function can read the caller's local values.
        let cross_scalars = detect_cross_boundary_scalars(&main_stmts, &gosub_fns, &sub_param_names);
        for (name_lc, ty) in &cross_scalars {
            // Use the BARE name (not rust_ident_typed) so the GameState field
            // matches how emit_lvalue references a shared scalar (`__gs.{bare}`).
            // rust_ident_typed would suffix a string `A$` to `a_s`, leaving the
            // field orphaned while every reference emitted `__gs.a`.
            let rust_name = rust_ident(name_lc);
            // Avoid double-promotion if already DIM SHARED or promoted as array,
            // and never promote a CONST.
            if !self.shared_names.contains(name_lc.as_str())
               && !self.promoted_arrays.contains(rust_name.as_str())
               && !const_names.contains(name_lc.as_str())
            {
                self.shared_names.insert(name_lc.clone());
                self.shared_types.insert(name_lc.clone(), ty.clone());
                self.promoted_scalars.push((rust_name, ty.clone()));
            }
        }

        // Register gosub block names as user_fns so CALLs to them get __rt/__gs prepended
        for (label, _) in &gosub_fns {
            self.user_fns.insert(rust_ident(label));
        }
        // Register DEF FN names from main body
        for stmt in &prog.main_body {
            if let Stmt::DefFn { name, .. } = stmt {
                self.user_fns.insert(rust_ident(name));
            }
        }

        // Store data_labels for RESTORE label support
        self.data_labels = prog.data_labels.clone();

        // Pre-scan ALL DIM statements across the entire program to populate
        // array_lower BEFORE emitting any code.  Subs are emitted first, so
        // without this pass their shared-array accesses have no lower-bound info.
        collect_array_lower_bounds(&prog.main_body, &mut self.array_lower);
        for s in &prog.subs      { collect_array_lower_bounds(&s.body, &mut self.array_lower); }
        for f in &prog.functions { collect_array_lower_bounds(&f.body, &mut self.array_lower); }

        self.emit_header();
        self.emit_data_store(&prog.data_store);
        self.emit_consts(&prog.consts, &prog.str_consts);
        // Any of these emit a fn taking `__gs: &mut GameState`, so the struct
        // and the `__gs` binding must exist even when GameState has no fields.
        let has_fns = !prog.subs.is_empty()
            || !prog.functions.is_empty()
            || !gosub_fns.is_empty()
            || prog.main_body.iter().any(|s| matches!(s, Stmt::DefFn { .. }));
        self.emit_game_state(&prog.global_scope, has_fns);
        self.emit_subs(&prog.subs, &const_globals)?;
        self.emit_functions(&prog.functions, &const_globals)?;
        self.emit_def_fns(&prog.main_body, &globals)?;
        for (label, body) in &gosub_fns {
            self.emit_gosub_fn(label, body, &globals)?;
        }
        // Parse QBC pragma directives.
        self.qbc = parse_qbc_config(&prog.directives);

        // Emit key-event dispatch helper if any ON KEY(n) GOSUB bindings were collected.
        self.emit_key_event_helper()?;

        self.emit_main(&main_stmts, &globals)?;
        Ok(inline_single_use_tmps(&self.out))
    }

    fn emit_gosub_fn(&mut self, label: &str, body: &[Stmt], globals: &HashSet<String>) -> Result<()> {
        let fn_name = rust_ident(label);
        self.line(&format!("fn {fn_name}(__rt: &mut Runtime, __gs: &mut GameState) {{"));
        self.indent();
        self.in_main = false;
        self.str_params.clear();
        self.array_params.clear();
        self.numeric_params.clear();
        self.local_arrays = collect_local_array_names(body);
        let mut exclude = globals.clone();
        exclude.extend(self.shared_names.clone());
        self.emit_locals(body, &exclude)?;
        self.emit_stmts(body)?;
        self.dedent();
        self.line("}");
        self.blank();
        Ok(())
    }

    fn emit_def_fns(&mut self, body: &[Stmt], _globals: &HashSet<String>) -> Result<()> {
        for stmt in body {
            if let Stmt::DefFn { name, params, expr } = stmt {
                let fn_name = rust_ident(name);
                let plist: Vec<String> = params.iter()
                    .map(|p| format!("mut {}: f64", rust_ident(&p.name)))
                    .collect();
                let sep = if plist.is_empty() { "" } else { ", " };
                self.line(&format!(
                    "fn {fn_name}(__rt: &mut Runtime, __gs: &mut GameState{sep}{}) -> f64 {{",
                    plist.join(", ")
                ));
                self.indent();
                self.in_main = false;
                self.str_params.clear();
                self.array_params.clear();
                self.local_arrays = collect_local_array_names(&[]);
                let e = self.emit_expr(expr)?;
                self.line(&format!("{e}"));
                self.dedent();
                self.line("}");
                self.blank();
            }
        }
        Ok(())
    }

    fn emit_header(&mut self) {
        self.line("// Generated by qbc — QBasic to Rust transpiler");
        self.line("#![allow(non_snake_case, unused_variables, dead_code, unused_mut,");
        self.line("         unused_assignments, unused_parens, unreachable_code,");
        self.line("         non_upper_case_globals, const_item_mutation, clippy::all)]");
        self.line("use qbasic_runtime::*;");
        self.blank();
    }

    fn emit_data_store(&mut self, data: &[String]) {
        if data.is_empty() { return; }
        let items: Vec<String> = data.iter()
            .map(|s| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect();
        self.line(&format!("static __DATA: &[&str] = &[{}];", items.join(", ")));
        self.line("static __DATA_PTR: std::sync::atomic::AtomicUsize = \
                   std::sync::atomic::AtomicUsize::new(0);");
        self.blank();
    }

    fn emit_consts(&mut self, consts: &[(String, f64)], str_consts: &[(String, String)]) {
        for (name, val) in consts {
            self.line(&format!("const {}: f64 = {};", rust_ident(name), emit_f64_lit(*val)));
        }
        for (name, val) in str_consts {
            // String constants: emit as &str. Name may conflict with Rust keywords.
            let escaped = val.replace('\\', "\\\\").replace('"', "\\\"");
            self.line(&format!("const {}: &str = \"{escaped}\";", rust_ident(name)));
        }
        if !consts.is_empty() || !str_consts.is_empty() { self.blank(); }
    }

    // ── GameState struct — holds all DIM SHARED variables ─────────────────────

    /// Emit the `GameState` struct holding DIM SHARED + cross-GOSUB-promoted
    /// variables. `has_fns` is true when the program emits any
    /// SUB/FUNCTION/GOSUB-fn/DEF-FN — all of which take `__gs: &mut GameState`
    /// in their signature. When the struct would have no fields AND `has_fns`
    /// is false, nothing references `__gs`, so the struct (and the `__gs`
    /// binding in main) are suppressed entirely.
    fn emit_game_state(&mut self, scope: &Scope, has_fns: bool) {
        let mut shared: Vec<_> = scope.symbols.values()
            .filter(|s| s.shared)
            .collect();
        shared.sort_by_key(|s| &s.name);

        // Collect field declarations first so we can tell whether the struct
        // would be empty before committing to emitting it.
        let mut fields: Vec<String> = Vec::new();
        for sym in &shared {
            let name_bare = rust_ident(&sym.name);
            // For array fields, use typed name so string arrays get _s suffix
            let name = if sym.dims > 0 {
                rust_ident_typed(&sym.name, &sym.ty)
            } else {
                name_bare.clone()
            };
            if sym.dims > 0 {
                // Typed array: expand to one Vec field per TYPE member
                if let Some(tfields) = self.typed_fields.get(&name_bare).cloned() {
                    let ndims = self.typed_array_dims.get(&sym.name.to_lowercase())
                        .copied().unwrap_or(sym.dims);
                    // Try to look up field types from TYPE definitions (clone to release borrow)
                    let type_name = if let QbType::UserType(tn) = &sym.ty {
                        Some(tn.to_lowercase())
                    } else { None };
                    let field_types: Option<Vec<(String, QbType)>> = type_name.as_ref()
                        .and_then(|tn| self.type_defs.get(tn.as_str()))
                        .cloned();
                    for field in &tfields {
                        // Find the field type from type_defs if available
                        let elem_ty = field_types.as_ref()
                            .and_then(|fts| fts.iter().find(|(f, _)| f == field))
                            .map(|(_, t)| t.clone())
                            .unwrap_or(QbType::Single);
                        let rust_elem = qb_type_to_rust(&elem_ty);
                        // An array field inside the TYPE adds one more Vec<> level.
                        let is_array_field = type_name.as_ref()
                            .and_then(|tn| self.type_field_dims.get(tn))
                            .map_or(false, |fd| fd.contains_key(field.as_str()));
                        let total_dims = ndims + if is_array_field { 1 } else { 0 };
                        fields.push(format!("{name}__{field}: {},",
                            nested_vec_type(rust_elem, total_dims)));
                    }
                } else {
                    // Plain N-D array (1/2/3-D): Vec / Vec<Vec> / Vec<Vec<Vec>>.
                    let elem = qb_type_to_rust(&sym.ty);
                    fields.push(format!("{name}: {},", nested_vec_type(elem, sym.dims)));
                }
            } else {
                // Scalar UserType → expand to one GameState field per flattened member
                if let Some(tfields) = self.typed_fields.get(&name_bare).cloned() {
                    let type_name_opt = if let QbType::UserType(tn) = &sym.ty {
                        Some(tn.to_lowercase())
                    } else { None };
                    let field_types: Option<Vec<(String, QbType)>> = type_name_opt.clone()
                        .and_then(|tn| self.type_defs.get(tn.as_str()))
                        .cloned();
                    for field in &tfields {
                        let elem_ty = field_types.as_ref()
                            .and_then(|fts| fts.iter().find(|(f, _)| f == field))
                            .map(|(_, t)| t.clone())
                            .unwrap_or(QbType::Single);
                        let rust_ty = qb_type_to_rust(&elem_ty);
                        // Check if this field is an array inside the TYPE body
                        let is_array_field = type_name_opt.as_ref()
                            .and_then(|tn| self.type_field_dims.get(tn))
                            .map_or(false, |fd| fd.contains_key(field.as_str()));
                        if is_array_field {
                            fields.push(format!("{name}__{field}: Vec<{rust_ty}>,"));
                        } else {
                            fields.push(format!("{name}__{field}: {rust_ty},"));
                        }
                    }
                } else {
                    let ty = qb_type_to_rust(&sym.ty);
                    fields.push(format!("{name}: {ty},"));
                }
            }
        }
        // Promoted arrays: cross GOSUB boundary, not in global_scope as DIM SHARED
        let mut promoted: Vec<String> = self.promoted_arrays.iter()
            .filter(|n| !shared.iter().any(|s| rust_ident(&s.name) == **n))
            .cloned()
            .collect();
        promoted.sort();
        for name in &promoted {
            // Look up the original type and dimensionality from the scope symbols
            // so string arrays get _s suffix and multi-dim arrays get Vec<Vec<…>>.
            let (field_name, elem_ty_str, ndims) = scope.symbols.values()
                .find(|s| rust_ident(&s.name) == *name)
                .map(|sym| {
                    let field = rust_ident_typed(&sym.name, &sym.ty);
                    let elem  = qb_type_to_rust(&sym.ty);
                    (field, elem, sym.dims.max(1))
                })
                .unwrap_or_else(|| (name.clone(), "f64", 1));
            fields.push(format!("{field_name}: {},", nested_vec_type(elem_ty_str, ndims)));
        }

        // Promoted scalars: cross GOSUB boundary, not already in global_scope as DIM SHARED
        let mut ps = self.promoted_scalars.clone();
        ps.sort_by_key(|(n, _)| n.clone());
        let shared_rust_names: HashSet<String> =
            shared.iter().map(|s| rust_ident(&s.name)).collect();
        for (rust_name, ty) in &ps {
            if !shared_rust_names.contains(rust_name) {
                let rust_ty = qb_type_to_rust(ty);
                fields.push(format!("{rust_name}: {rust_ty},"));
            }
        }

        // Nothing in GameState and no function takes it → suppress entirely.
        if fields.is_empty() && !has_fns {
            self.gamestate_emitted = false;
            return;
        }
        self.gamestate_emitted = true;

        self.line("#[derive(Default)]");
        self.line("struct GameState {");
        self.indent();
        for f in &fields {
            self.line(f);
        }
        self.dedent();
        self.line("}");
        self.blank();
    }

    // ── SUB → fn ──────────────────────────────────────────────────────────────

    fn emit_subs(&mut self, subs: &[SubDef], globals: &HashSet<String>) -> Result<()> {
        for sub in subs {
            self.in_main = false;
            self.current_fn_retvar = None;  // not in a FUNCTION
            self.redim_declared.clear();

            // Per-sub shared_names: restrict to DIM SHARED globals plus this sub's
            // explicit SHARED declarations.  This prevents name collisions where a
            // module-level array and a local scalar share the same identifier but
            // only the sub that declares `SHARED arr` should see the array form.
            let sub_explicit = collect_sub_explicit_shared(&sub.body);
            let dim_shared   = self.dim_shared_names.clone();
            let full_shared  = self.shared_names.clone();
            let saved_shared = std::mem::replace(
                &mut self.shared_names,
                full_shared.iter()
                    .filter(|n| dim_shared.contains(*n) || sub_explicit.contains(*n))
                    .cloned()
                    .collect(),
            );

            self.setup_param_sets(&sub.params, true); // SUB: numeric scalars byref
            let params = self.emit_params(&sub.params, &sub.body);
            let sep    = if params.is_empty() { "" } else { ", " };
            self.line(&format!(
                "fn {}(__rt: &mut Runtime, __gs: &mut GameState{sep}{params}) {{",
                rust_ident(&sub.name)
            ));
            self.indent();

            // Extract GOSUB blocks from sub body so they can be inlined at call sites.
            // This gives GOSUB targets access to all the sub's local variables.
            let (inline_body, gosub_blocks) = extract_gosub_blocks(&sub.body, &HashSet::new());
            self.current_sub_gosubs.clear();
            for (label, body) in gosub_blocks {
                self.current_sub_gosubs.insert(label.to_lowercase(), body);
            }

            // Collect local arrays for disambiguation inside this body
            self.local_arrays = collect_local_array_names(&sub.body);

            // Track explicit local DIM declarations so emit_lvalue can distinguish
            // e.g. local integer `B` from DIM SHARED string `B$` (same base name).
            self.local_dim_names = collect_local_dim_names(&sub.body);
            self.local_string_arrays = collect_local_string_arrays(&sub.body);

            let mut exclude = globals.clone();
            for p in &sub.params {
                exclude.insert(rust_ident_typed(&p.name, &p.ty));
                exclude.insert(rust_ident(&p.name));
            }
            // For shared scalars, exclude by their typed Rust name so a locally
            // DIM'd numeric variable with the same base name as a shared string
            // variable (e.g. DIM B AS INTEGER when B$ is DIM SHARED) is NOT
            // suppressed from the locals list.  String shared vars get excluded
            // as "b_s"; a local integer "b" is then free to be declared locally.
            for name_lc in &self.shared_names {
                let sty = self.shared_types.get(name_lc).cloned().unwrap_or(QbType::Single);
                if sty == QbType::String {
                    exclude.insert(rust_ident_typed(name_lc, &sty));
                } else {
                    exclude.insert(name_lc.clone());
                }
            }
            // Exclude TYPE field arrays that are actually passed as params
            for (pname, fields) in self.typed_fields.clone().iter() {
                if sub.params.iter().any(|p| rust_ident(&p.name) == *pname && !p.dims.is_empty()) {
                    for field in fields {
                        exclude.insert(format!("{pname}__{field}"));
                    }
                }
            }
            // emit_locals on full body so it sees variables in gosub blocks too
            self.emit_locals(&sub.body, &exclude)?;
            self.emit_stmts(&inline_body)?;
            self.current_sub_gosubs.clear();
            self.current_gosub_label = None;
            self.dedent();
            self.line("}");
            self.blank();
            self.shared_names = saved_shared;
        }
        Ok(())
    }

    /// `byref_numerics`: true for SUBs (QB passes scalars by ref), false for FUNCTIONs
    /// (which return a value; callers pass args through emit_expr_inner by value).
    fn setup_param_sets(&mut self, params: &[VarDecl], byref_numerics: bool) {
        self.str_params.clear();
        self.array_params.clear();
        self.numeric_params.clear();
        for p in params {
            let lower = rust_ident_typed(&p.name, &p.ty);
            if p.ty == QbType::String {
                // Both SUBs and FUNCTIONs pass string scalars as &mut String
                self.str_params.insert(lower.clone());
            } else if p.dims.is_empty() {
                if let QbType::UserType(tn) = &p.ty {
                    // Scalar TYPE param — ALWAYS by reference, for both SUBs and
                    // FUNCTIONs (QB semantics). A FUNCTION that mutates a TYPE field
                    // and lets the caller read it back relies on this — e.g. torus's
                    // Inside() sets T.xc/T.yc and TileDraw uses them afterward.
                    let tn_lc = tn.to_lowercase();
                    let flat = flatten_type_fields(&tn_lc, &self.type_defs.clone());
                    let base = rust_ident(&p.name);
                    for (fname, _) in &flat {
                        self.numeric_params.insert(format!("{base}__{fname}"));
                    }
                } else if byref_numerics {
                    // Plain numeric scalar — QB SUB passes by reference. FUNCTIONs
                    // keep pass-by-value (return their result via the fn value), which
                    // is observationally identical unless they mutate-and-read-back.
                    self.numeric_params.insert(lower.clone());
                }
            }
            if !p.dims.is_empty() { self.array_params.insert(lower); }
        }
    }

    // ── FUNCTION → fn ─────────────────────────────────────────────────────────

    fn emit_functions(&mut self, funcs: &[FuncDef], globals: &HashSet<String>) -> Result<()> {
        for f in funcs {
            self.in_main = false;
            self.current_fn_retvar = None;
            self.redim_declared.clear();

            // Per-function shared_names: same restriction as emit_subs
            let fn_explicit = collect_sub_explicit_shared(&f.body);
            let dim_shared  = self.dim_shared_names.clone();
            let full_shared = self.shared_names.clone();
            let saved_shared = std::mem::replace(
                &mut self.shared_names,
                full_shared.iter()
                    .filter(|n| dim_shared.contains(*n) || fn_explicit.contains(*n))
                    .cloned()
                    .collect(),
            );

            self.setup_param_sets(&f.params, false); // FUNCTION: pass by value (return via fn result)
            let params = self.emit_params(&f.params, &f.body);
            let ret_ty = qb_type_to_rust(&f.ret_ty);
            let sep    = if params.is_empty() { "" } else { ", " };
            // Use rust_ident_typed so that Trim$→trim_s, Cvit$→cvit_s, etc.
            let fn_rust_name = rust_ident_typed(&f.name, &f.ret_ty);
            self.line(&format!(
                "fn {fn_rust_name}(__rt: &mut Runtime, __gs: &mut GameState{sep}{params}) -> {ret_ty} {{"
            ));
            self.indent();
            // QB FUNCTION returns by assigning to the function name.
            // Use "__fn_ret" as the local variable so recursive calls to the
            // same function don't get shadowed by the local binding.
            self.line(&format!("let mut __fn_ret: {ret_ty} = Default::default();"));
            self.current_fn_retvar = Some("__fn_ret".to_string());
            self.current_fn_name_lc = Some(fn_rust_name.clone());

            // Extract GOSUB blocks from function body for inline emission
            let (inline_body, gosub_blocks) = extract_gosub_blocks(&f.body, &HashSet::new());
            self.current_sub_gosubs.clear();
            for (label, body) in gosub_blocks {
                self.current_sub_gosubs.insert(label.to_lowercase(), body);
            }

            self.local_arrays = collect_local_array_names(&inline_body);

            // Track explicit local DIM declarations (same reason as emit_subs).
            self.local_dim_names = collect_local_dim_names(&inline_body);
            self.local_string_arrays = collect_local_string_arrays(&inline_body);

            let mut exclude = globals.clone();
            for p in &f.params {
                exclude.insert(rust_ident_typed(&p.name, &p.ty));
                exclude.insert(rust_ident(&p.name));
            }
            // Exclude the function name itself — it maps to __fn_ret, not a local
            exclude.insert(fn_rust_name.clone());
            exclude.insert(rust_ident(&f.name));
            // Also exclude "__fn_ret" so emit_locals doesn't re-declare it
            exclude.insert("__fn_ret".to_string());
            // Same type-aware exclusion as emit_subs: string shared vars excluded
            // by typed name so a local integer with the same base name can coexist.
            for name_lc in &self.shared_names {
                let sty = self.shared_types.get(name_lc).cloned().unwrap_or(QbType::Single);
                if sty == QbType::String {
                    exclude.insert(rust_ident_typed(name_lc, &sty));
                } else {
                    exclude.insert(name_lc.clone());
                }
            }
            self.emit_locals(&inline_body, &exclude)?;
            self.emit_stmts(&inline_body)?;
            self.current_sub_gosubs.clear();
            self.current_gosub_label = None;
            self.current_fn_retvar = None;
            self.current_fn_name_lc = None;
            self.line("__fn_ret");
            self.dedent();
            self.line("}");
            self.blank();
            self.shared_names = saved_shared;
        }
        Ok(())
    }

    // ── Key / Timer event dispatch helpers ───────────────────────────────────

    /// Emit `fn __handle_key_event` and `fn __handle_timer_event` if any
    /// ON KEY/TIMER GOSUB bindings were collected during emit.
    fn emit_key_event_helper(&mut self) -> Result<()> {
        if self.on_key_gosubs.is_empty() && self.on_timer_gosub.is_none() {
            return Ok(());
        }
        // QB predefined key numbers → the two-byte escape string that inkey() returns.
        // 11 = Up, 12 = Left, 13 = Right, 14 = Down (standard QB arrow-key traps).
        // 15-24 = user-defined; map them to common defaults where possible.
        let key_str = |n: f64| -> Option<String> {
            match n as u32 {
                11 => Some(r#""\x00H""#.into()),   // Up arrow
                12 => Some(r#""\x00K""#.into()),   // Left arrow
                13 => Some(r#""\x00M""#.into()),   // Right arrow
                14 => Some(r#""\x00P""#.into()),   // Down arrow
                _  => None,   // user-defined keys need KEY n,expr — not mapped
            }
        };

        let needs_gs = self.gamestate_emitted;
        let gs_param = if needs_gs { ", __gs: &mut GameState" } else { "" };
        let gs_arg   = if needs_gs { ", __gs" } else { "" };

        if !self.on_key_gosubs.is_empty() {
            self.line(&format!(
                "fn __handle_key_event(__k: &str, __rt: &mut Runtime{gs_param}) {{"
            ));
            self.indent();
            // Deduplicate by key string (keep first handler if multiple map same key).
            let mut arms: Vec<(String, String)> = Vec::new();
            for (n, tgt) in &self.on_key_gosubs {
                if let Some(ks) = key_str(*n) {
                    if !arms.iter().any(|(k, _)| k == &ks) {
                        arms.push((ks, rust_ident(tgt)));
                    }
                }
            }
            if arms.is_empty() {
                self.line("let _ = (__k, __rt);");
            } else {
                self.line("match __k {");
                self.indent();
                for (ks, fn_name) in &arms {
                    self.line(&format!("{ks} => {{ {fn_name}(__rt{gs_arg}); }}"));
                }
                self.line("_ => {}");
                self.dedent();
                self.line("}");
            }
            self.dedent();
            self.line("}");
        }

        if let Some((interval, tgt)) = &self.on_timer_gosub.clone() {
            let fn_name = rust_ident(tgt);
            let secs = interval;
            // Use a static AtomicU64 to store the last-fired timestamp as f64 bits.
            // This avoids needing a __last_timer parameter and the associated borrow issues.
            self.line("static __TIMER_LAST_FIRED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);");
            self.line(&format!(
                "fn __handle_timer_event(__rt: &mut Runtime{gs_param}) {{"
            ));
            self.indent();
            self.line("let __last = f64::from_bits(__TIMER_LAST_FIRED.load(std::sync::atomic::Ordering::Relaxed));");
            self.line("let __now = qb_timer();");
            self.line(&format!("if __now - __last >= {secs}.0 {{"));
            self.indent();
            self.line("__TIMER_LAST_FIRED.store(__now.to_bits(), std::sync::atomic::Ordering::Relaxed);");
            self.line(&format!("{fn_name}(__rt{gs_arg});"));
            self.dedent();
            self.line("}");
            self.dedent();
            self.line("}");
        }
        Ok(())
    }

    // ── main() ────────────────────────────────────────────────────────────────

    fn emit_main(&mut self, body: &[Stmt], globals: &HashSet<String>) -> Result<()> {
        self.in_main = true;
        self.str_params.clear();
        self.array_params.clear();
        self.numeric_params.clear();
        self.line("fn main() {");
        self.indent();

        // Emit Runtime constructor — use new_configured() when TITLE or SCALE are set.
        let needs_configured = self.qbc.title.is_some() || self.qbc.scale.is_some();
        if needs_configured {
            let title = self.qbc.title.as_deref().unwrap_or("QBasic");
            let scale = self.qbc.scale.unwrap_or(1) as usize;
            let win_w = 960 * scale;
            let win_h = 600 * scale;
            // Escape any backslashes or quotes in the title for Rust string literal.
            let title_escaped = title.replace('\\', "\\\\").replace('"', "\\\"");
            self.line(&format!("let mut __rt = Runtime::new_configured(\"{title_escaped}\", {win_w}, {win_h});"));
        } else {
            self.line("let mut __rt = Runtime::new();");
        }

        // Emit post-construction directive setters.
        if self.qbc.fullspeed { self.line("__rt.set_fullspeed(true);"); }
        if let Some(fps)    = self.qbc.fps    { self.line(&format!("__rt.set_fps({fps}.0);")); }
        if let Some(pace)   = self.qbc.pace   { self.line(&format!("__rt.set_pace({pace}.0);")); }
        if let Some(slowmo) = self.qbc.slowmo { self.line(&format!("__rt.set_slowmo({slowmo}.0);")); }

        if self.gamestate_emitted {
            self.line("let mut __gs = GameState::default();");
            self.blank();
        }

        self.local_arrays = collect_local_array_names(body);

        let mut exclude = globals.clone();
        exclude.extend(self.shared_names.clone());
        self.emit_locals(body, &exclude)?;

        // If main body contains numeric-label GOTOs, use a state-machine loop.
        // Named-label GOTOs (e.g. GOTO SkipGuess) are handled as Rust labeled
        // loop continues and must NOT activate the state machine.
        let has_goto = body.iter().any(|s| stmt_has_numeric_goto(s));
        if has_goto {
            self.emit_state_machine(body)?;
        } else {
            self.emit_stmts(body)?;
        }
        self.dedent();
        self.line("}");
        Ok(())
    }

    /// Emit a `__pc` state-machine loop for programs that use GOTO.
    fn emit_state_machine(&mut self, stmts: &[Stmt]) -> Result<()> {
        let blocks = flatten_to_blocks(stmts);
        if blocks.is_empty() {
            return Ok(());
        }

        // Hoist local array declarations before the loop so they're visible in all arms.
        let shared_names = self.shared_names.clone();
        let sm_arrays = collect_sm_local_arrays(stmts, &shared_names);
        for (name, elem_ty, ndims) in &sm_arrays {
            if *ndims >= 2 {
                self.line(&format!("let mut {name}: Vec<Vec<{elem_ty}>> = Vec::new();"));
            } else {
                self.line(&format!("let mut {name}: Vec<{elem_ty}> = Vec::new();"));
            }
        }

        // First pc is the first block's pc (or 0 for sentinel).
        let first_pc = blocks[0].0;

        self.line(&format!("let mut __pc: u32 = {first_pc};"));
        self.line("'__sm: loop {");
        self.indent();
        self.line("match __pc {");
        self.indent();

        self.sm_mode = true;

        for i in 0..blocks.len() {
            let (pc, ref block_stmts) = blocks[i];
            let next_pc = if i + 1 < blocks.len() {
                blocks[i + 1].0
            } else {
                u32::MAX
            };

            // Emit the match arm
            if pc == 0 {
                self.line("0 => {");
            } else {
                self.line(&format!("{pc} => {{"));
            }
            self.indent();
            // Emit the statements in this block
            let block_stmts = block_stmts.clone(); // clone to release borrow on blocks
            self.emit_stmts(&block_stmts)?;
            // Fall-through to next block
            if next_pc == u32::MAX {
                self.line("break '__sm;");
            } else {
                self.line(&format!("__pc = {next_pc}; continue '__sm;"));
            }
            self.dedent();
            self.line("}");
        }

        self.sm_mode = false;

        self.line("_ => break,");
        self.dedent();
        self.line("}"); // end match
        self.dedent();
        self.line("}"); // end loop
        Ok(())
    }

    fn emit_locals(&mut self, body: &[Stmt], exclude: &HashSet<String>) -> Result<()> {
        // When a typed-array DIM is present, emit_dim will emit `let mut arr__field`
        // with the correct size — so collect_locals must not also declare them.
        let mut combined_exclude = exclude.clone();

        // REDIM'd local arrays are declared inline by emit_redim() — exclude them
        // here so collect_locals doesn't also emit a scalar `let mut x: f64` for them.
        for rname in collect_redim_names(body) {
            combined_exclude.insert(rname);
        }
        for stmt in body {
            if let Stmt::Dim(d) = stmt {
                if matches!(&d.ty, QbType::UserType(_)) && !d.dims.is_empty() {
                    let lower = rust_ident(&d.name);
                    if let Some(fields) = self.typed_fields.get(&lower) {
                        for f in fields {
                            combined_exclude.insert(format!("{lower}__{f}"));
                        }
                    }
                }
            }
        }
        // Exclude shared typed array field names — they live in GameState, not as locals
        for (arr_name, fields) in self.typed_fields.clone().iter() {
            if self.shared_names.contains(arr_name.as_str()) {
                for field in fields {
                    combined_exclude.insert(format!("{arr_name}__{field}"));
                }
            }
        }
        // Exclude user FUNCTION names: a bare reference to a zero-arg FUNCTION is a
        // CALL (`X = StillWantsToPlay`), not a variable — never declare a local that
        // would shadow the fn. (Function names are reserved in QB.)
        combined_exclude.extend(self.user_fns.iter().cloned());
        let locals = collect_locals(body, &combined_exclude);
        for (name, ty) in &locals {
            // Disambiguate a scalar that shares its name with a local array.
            let name = self.local_scalar_name(name);
            match ty {
                QbType::UserType(s) if s == "vec_f64" => {
                    // Flattened TYPE field array — size unknown, use resizable Vec
                    self.line(&format!("let mut {name}: Vec<f64> = Vec::new();"));
                }
                QbType::UserType(s) if s == "vec_string" => {
                    // String array declared via REDIM inside sub
                    self.line(&format!("let mut {name}: Vec<String> = Vec::new();"));
                }
                QbType::String => {
                    self.line(&format!("let mut {name}: String = String::new();"));
                }
                _ => {
                    let rust_ty = qb_type_to_rust(ty);
                    self.line(&format!("let mut {name}: {rust_ty} = 0.0;"));
                }
            }
        }
        if !locals.is_empty() { self.blank(); }
        Ok(())
    }

    // ── Statement emitter ─────────────────────────────────────────────────────

    fn emit_stmts(&mut self, stmts: &[Stmt]) -> Result<()> {
        for stmt in stmts { self.emit_stmt(stmt)?; }
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Dim(decl)   => self.emit_dim(decl),
            Stmt::ReDim(decl) => self.emit_redim(decl),

            Stmt::Let { var, expr } => {
                // Detect whole-record TYPE array/scalar assignments and expand them.
                // Helper: get RHS typed-array info from either LValue::Index or Expr::Call
                let rhs_typed = if let Expr::Var(rhs_lv) = expr {
                    self.typed_array_index(rhs_lv)
                } else {
                    self.typed_array_call(expr)
                };
                // Also detect RHS scalar TYPE var (LValue::Scalar)
                let rhs_scalar_type_name: Option<String> = if let Expr::Var(LValue::Scalar { name, .. }) = expr {
                    let rn = rust_ident(name);
                    self.scalar_type_fields(&rn).map(|_| rn)
                } else { None };

                if let Some((lhs_arr, lhs_idx, fields)) = self.typed_array_index(var) {
                    // LHS is a typed array element: arr(i) = ?
                    if let Some((rhs_arr, rhs_idx, _)) = rhs_typed {
                        // arr(i) = arr(j) — field-by-field copy
                        let (la, li, rhs_a, ri, flds) =
                            (lhs_arr.clone(), lhs_idx.clone(), rhs_arr, rhs_idx, fields);
                        self.emit_typed_array_copy(&la, &li, &rhs_a, &ri, &flds);
                        return Ok(());
                    }
                    if let Some(rn) = rhs_scalar_type_name {
                        // arr(i) = scalar_type_var — copy scalar fields to array
                        let flds = self.scalar_type_fields(&rn).unwrap();
                        let (la, li, flds2) = (lhs_arr.clone(), lhs_idx.clone(), flds);
                        self.emit_scalar_to_typed_arr(&la, &li, &rn, &flds2);
                        return Ok(());
                    }
                    // Fall through — unknown RHS for typed array, emit plain (will likely fail)
                } else if let LValue::Scalar { name, .. } = var {
                    let rn = rust_ident(name);
                    if self.scalar_type_fields(&rn).is_some() {
                        // LHS is a scalar TYPE variable: scalar = typed_arr(i)?
                        if let Some((rhs_arr, rhs_idx, _)) = rhs_typed {
                            let flds = self.scalar_type_fields(&rn).unwrap();
                            let (rn2, ra, ri, flds2) = (rn, rhs_arr, rhs_idx, flds);
                            self.emit_typed_arr_to_scalar(&rn2, &ra, &ri, &flds2);
                            return Ok(());
                        }
                        // scalar = scalar TYPE var (whole-record copy, e.g.
                        // `OldBlock = CurBlock`) → per-field assignment. Field paths
                        // come from flatten_type_fields so they match the GameState
                        // field names (handles nested TYPEs).
                        if let Some(rhs_rn) = rhs_scalar_type_name {
                            if let Some(tn) = self.var_type_name.get(&rn).cloned() {
                                let fields: Vec<String> =
                                    flatten_type_fields(&tn, &self.type_defs.clone())
                                        .into_iter().map(|(f, _)| f).collect();
                                self.emit_scalar_type_copy(&rn, &rhs_rn, &fields);
                                return Ok(());
                            }
                        }
                    }
                }

                let lhs = self.emit_lvalue(var);
                // Use lift_expr so user-function calls with &mut String params
                // get proper temporary bindings hoisted before the call.
                let rhs = self.lift_expr(expr);
                match var {
                    // &mut String param (with $ sigil or AS STRING) → deref-assign with to_string()
                    LValue::Scalar { name, .. }
                        if self.str_params.contains(&rust_ident_typed(name, &QbType::String)) =>
                    {
                        self.line(&format!("*{lhs} = ({rhs}).to_string();"));
                    }
                    // String local → assign with to_string() (rhs may be &str literal)
                    LValue::Scalar { ty: QbType::String, .. } => {
                        self.line(&format!("{lhs} = ({rhs}).to_string();"));
                    }
                    // String array element → assign with to_string()
                    LValue::Index { ty: QbType::String, .. } => {
                        self.line(&format!("{lhs} = ({rhs}).to_string();"));
                    }
                    // Local string array accessed without $ sigil (DIM name(...) AS STRING)
                    LValue::Index { name, .. }
                        if self.local_string_arrays.contains(&name.to_lowercase()) =>
                    {
                        self.line(&format!("{lhs} = ({rhs}).to_string();"));
                    }
                    // TYPE field or other LValue: check if string type via context
                    _ if self.is_str_expr_ctx(&Expr::Var(var.clone())) => {
                        self.line(&format!("{lhs} = ({rhs}).to_string();"));
                    }
                    // Numeric or other — plain assignment
                    _ => {
                        self.line(&format!("{lhs} = {rhs};"));
                    }
                }
            }

            Stmt::If { cond, then_body, elseif_branches, else_body } => {
                let c = self.emit_cond_expr(cond)?;
                self.line(&format!("if {c} {{"));
                self.indent();
                self.emit_stmts(then_body)?;
                self.dedent();
                for (ec, eb) in elseif_branches {
                    let ec = self.emit_cond_expr(ec)?;
                    self.line(&format!("}} else if {ec} {{"));
                    self.indent();
                    self.emit_stmts(eb)?;
                    self.dedent();
                }
                if let Some(eb) = else_body {
                    self.line("} else {");
                    self.indent();
                    self.emit_stmts(eb)?;
                    self.dedent();
                }
                self.line("}");
            }

            Stmt::For { var, from, to, step, body } => {
                let v = rust_ident(var); // unique suffix for the __for_to_/__for_step_ temps
                // Reference form for the counter: `__gs.i` when it's a shared/
                // promoted variable (e.g. a GOSUB target reads the loop counter,
                // common in state-machine programs), `(*i)` for a byref param,
                // or the bare local `i`. Identical to `v` for plain locals.
                let vref = self.emit_lvalue(&LValue::Scalar { name: var.clone(), ty: QbType::Single });
                let f = self.emit_expr(from)?;
                let t = self.emit_expr(to)?;
                let s = step.as_ref().map(|e| self.emit_expr(e).unwrap())
                            .unwrap_or_else(|| "1.0".into());
                // FOR var is pre-declared by collect_locals (or promoted to GameState) — just assign.
                self.line(&format!("{vref} = {f};"));
                self.line(&format!("let __for_to_{v}: f64 = {t};"));
                self.line(&format!("let __for_step_{v}: f64 = {s};"));
                self.line(&format!(
                    "while (__for_step_{v} > 0.0 && {vref} <= __for_to_{v}) || \
                           (__for_step_{v} < 0.0 && {vref} >= __for_to_{v}) {{"
                ));
                self.indent();
                self.emit_stmts(body)?;
                self.line(&format!("{vref} += __for_step_{v};"));
                self.dedent();
                self.line("}");
            }

            Stmt::While { cond, body } => {
                let c = self.emit_cond_expr(cond)?;
                self.line(&format!("while {c} {{"));
                self.indent();
                self.emit_stmts(body)?;
                self.dedent();
                self.line("}");
            }

            Stmt::Do { kind, body } => self.emit_do(kind, body)?,

            Stmt::Select { expr, cases, default } => {
                let e = self.emit_expr(expr)?;
                // Clone String selectors so the original variable isn't moved.
                let clone_sel = if is_str_expr(expr) { ".clone()" } else { "" };
                self.line(&format!("let __sel = {e}{clone_sel};"));
                let mut first = true;
                for case in cases {
                    let cond = self.emit_case_cond(case)?;
                    if first {
                        self.line(&format!("if {cond} {{"));
                        first = false;
                    } else {
                        self.line(&format!("}} else if {cond} {{"));
                    }
                    self.indent();
                    self.emit_stmts(&case.body)?;
                    self.dedent();
                }
                if let Some(db) = default {
                    self.line("} else {");
                    self.indent();
                    self.emit_stmts(db)?;
                    self.dedent();
                }
                if !first { self.line("}"); }
            }

            Stmt::Goto(label)  => {
                if self.sm_mode {
                    let pc: u32 = label.parse().unwrap_or(u32::MAX);
                    self.line(&format!("{{ __pc = {pc}; continue '__sm; }}"));
                } else if let Some(rust_lbl) = self.named_loop_labels.get(&label.to_lowercase()).cloned() {
                    // Named GOTO to a label at the bottom of a DO loop → `continue`
                    self.line(&format!("continue {rust_lbl};"));
                } else {
                    self.line(&format!("// GOTO {label}"));
                }
            }
            Stmt::Gosub(label) => {
                let key = label.to_lowercase();
                if self.current_sub_gosubs.contains_key(&key) {
                    // Inline the GOSUB body using a Rust labeled loop.
                    // RETURN inside the body becomes `break '__gosub_LABEL;`.
                    let label_id = key.replace(|c: char| !c.is_alphanumeric(), "_");
                    let rust_label = format!("'__gosub_{label_id}");
                    self.line(&format!("{rust_label}: loop {{"));
                    self.indent();
                    // Clone body to satisfy borrow checker (current_sub_gosubs is on self)
                    let body = self.current_sub_gosubs[&key].clone();
                    let prev_label = self.current_gosub_label.take();
                    self.current_gosub_label = Some(label_id.clone());
                    self.emit_stmts(&body)?;
                    self.current_gosub_label = prev_label;
                    self.line(&format!("break {rust_label};"));
                    self.dedent();
                    self.line("}");
                } else {
                    let fn_name = rust_ident(label);
                    self.line(&format!("{fn_name}({});", self.rt_args()));
                }
            }
            Stmt::Return => {
                if let Some(label_id) = &self.current_gosub_label.clone() {
                    // Inside an inline GOSUB — break out of the labeled loop
                    self.line(&format!("break '__gosub_{label_id};"));
                } else {
                    self.line("return;");
                }
            }
            Stmt::Label(l)     => {
                if self.sm_mode && l.parse::<u32>().is_ok() {
                    // Numeric labels inside FOR/IF bodies in state-machine mode are comments.
                    // (Top-level labels are consumed by flatten_to_blocks.)
                    self.line(&format!("// line {l}"));
                } else if self.named_loop_labels.contains_key(&l.to_lowercase()) {
                    // This is a "continue target" label at the tail of a DO loop.
                    // The loop label handles it; emit only a comment.
                    self.line(&format!("// label: {l}  (loop continue target)"));
                } else {
                    self.line(&format!("// label: {l}"));
                }
            }
            Stmt::DefFn { .. } => { /* hoisted before main by emit_def_fns */ }

            Stmt::Exit(kind) => match kind {
                ExitKind::For      => self.line("break; // EXIT FOR"),
                ExitKind::Do       => self.line("break; // EXIT DO"),
                ExitKind::Sub      => self.line("return; // EXIT SUB"),
                ExitKind::Function => {
                    if let Some(rv) = self.current_fn_retvar.clone() {
                        self.line(&format!("return {rv}; // EXIT FUNCTION"));
                    } else {
                        self.line("return; // EXIT FUNCTION");
                    }
                }
            },

            // ── I/O ──────────────────────────────────────────────────────────
            Stmt::Print { args, newline } => self.emit_print(args, *newline)?,
            Stmt::Input { prompt, vars }  => self.emit_input(prompt, vars)?,
            Stmt::Locate { row, col, cursor } => {
                // Omitted args → None so the runtime leaves the cursor where it is
                // (QB semantics) rather than moving to (0,0).
                let opt = |e: &Option<Expr>, s: &mut Self| match e {
                    Some(x) => format!("Some({})", s.lift_expr(x)),
                    None    => "None".to_string(),
                };
                let r = opt(row, self);
                let c = opt(col, self);
                let cur = opt(cursor, self);
                self.line(&format!("__rt.locate({r}, {c}, {cur});"));
            }
            Stmt::Color { fg, bg } => {
                let f = fg.as_ref().map(|e| self.emit_expr(e).unwrap()).unwrap_or("7.0".into());
                let b = bg.as_ref()
                    .map(|e| format!("Some({})", self.emit_expr(e).unwrap()))
                    .unwrap_or("None".into());
                self.line(&format!("__rt.color({f}, {b});"));
            }
            Stmt::Cls(arg) => {
                let n = arg.as_ref()
                    .and_then(|e| self.emit_expr(e).ok())
                    .map(|s| format!("({s}) as u8"))
                    .unwrap_or_else(|| "0u8".into());
                self.line(&format!("__rt.cls({n});"));
            }
            Stmt::ViewPrint { top, bot } => {
                match (top, bot) {
                    (Some(t), Some(b)) => {
                        let t = self.emit_expr(t).unwrap_or_else(|_| "1.0".into());
                        let b = self.emit_expr(b).unwrap_or_else(|_| "25.0".into());
                        self.line(&format!("__rt.view_print(Some({t}), Some({b}));"));
                    }
                    _ => self.line("__rt.view_print(None, None);"),
                }
            }

            // ── Graphics ─────────────────────────────────────────────────────
            // lift_expr extracts user-fn calls to temp vars to avoid
            // double-borrowing __rt as both receiver and argument
            Stmt::Screen(e) => {
                let m = self.lift_expr(e);
                self.line(&format!("__rt.screen({m});"));
                self.emit_error_dispatch();
            }
            Stmt::Circle { x, y, r, color, step } => {
                let x = self.lift_expr(x);
                let y = self.lift_expr(y);
                let r = self.lift_expr(r);
                let c = color.as_ref().map(|e| self.lift_expr(e))
                             .unwrap_or_else(|| "__rt.fg_color as f64".into());
                if *step {
                    // STEP: center is relative to the current graphics cursor.
                    let tc = self.lift_counter; self.lift_counter += 1;
                    self.line(&format!("let __stx{tc} = __rt.cur_x() + ({x});"));
                    self.line(&format!("let __sty{tc} = __rt.cur_y() + ({y});"));
                    self.line(&format!("__rt.circle(__stx{tc}, __sty{tc}, {r}, {c});"));
                } else {
                    self.line(&format!("__rt.circle({x}, {y}, {r}, {c});"));
                }
            }
            Stmt::Line { x1, y1, x2, y2, color, style, step1, step2 } if !*step1 && !*step2 => {
                let x2 = self.lift_expr(x2);
                let y2 = self.lift_expr(y2);
                let c  = color.as_ref().map(|e| self.lift_expr(e))
                              .unwrap_or_else(|| "__rt.fg_color as f64".into());
                match (x1, y1) {
                    (Some(x1), Some(y1)) => {
                        let x1 = self.lift_expr(x1);
                        let y1 = self.lift_expr(y1);
                        match style {
                            LineStyle::Plain     => self.line(&format!("__rt.line({x1},{y1},{x2},{y2},{c});")),
                            LineStyle::Box       => self.line(&format!("__rt.line_box({x1},{y1},{x2},{y2},{c});")),
                            LineStyle::FilledBox => self.line(&format!("__rt.line_box_fill({x1},{y1},{x2},{y2},{c});")),
                        }
                    }
                    _ => {
                        // Relative form — LINE -(x2,y2): start from current graphics cursor
                        match style {
                            LineStyle::Plain     => self.line(&format!("__rt.line_to({x2},{y2},{c});")),
                            LineStyle::Box       => self.line(&format!("__rt.line_box_to({x2},{y2},{c});")),
                            LineStyle::FilledBox => self.line(&format!("__rt.line_box_fill_to({x2},{y2},{c});")),
                        }
                    }
                }
            }
            Stmt::Line { x1, y1, x2, y2, color, style, step1, step2 } => {
                // STEP path: resolve both points to absolute coords in temps, then
                // call the absolute line methods. First point STEP is relative to
                // the cursor; second point STEP is relative to the FIRST point.
                let tc = self.lift_counter; self.lift_counter += 1;
                match (x1, y1) {
                    (Some(x1e), Some(y1e)) => {
                        let x1v = self.lift_expr(x1e);
                        let y1v = self.lift_expr(y1e);
                        if *step1 {
                            self.line(&format!("let __lx1_{tc} = __rt.cur_x() + ({x1v});"));
                            self.line(&format!("let __ly1_{tc} = __rt.cur_y() + ({y1v});"));
                        } else {
                            self.line(&format!("let __lx1_{tc} = {x1v};"));
                            self.line(&format!("let __ly1_{tc} = {y1v};"));
                        }
                    }
                    _ => {
                        // Bare relative `LINE -(...)`: first point is the cursor.
                        self.line(&format!("let __lx1_{tc} = __rt.cur_x();"));
                        self.line(&format!("let __ly1_{tc} = __rt.cur_y();"));
                    }
                }
                let x2v = self.lift_expr(x2);
                let y2v = self.lift_expr(y2);
                if *step2 {
                    self.line(&format!("let __lx2_{tc} = __lx1_{tc} + ({x2v});"));
                    self.line(&format!("let __ly2_{tc} = __ly1_{tc} + ({y2v});"));
                } else {
                    self.line(&format!("let __lx2_{tc} = {x2v};"));
                    self.line(&format!("let __ly2_{tc} = {y2v};"));
                }
                let c = color.as_ref().map(|e| self.lift_expr(e))
                             .unwrap_or_else(|| "__rt.fg_color as f64".into());
                let args = format!("__lx1_{tc},__ly1_{tc},__lx2_{tc},__ly2_{tc},{c}");
                match style {
                    LineStyle::Plain     => self.line(&format!("__rt.line({args});")),
                    LineStyle::Box       => self.line(&format!("__rt.line_box({args});")),
                    LineStyle::FilledBox => self.line(&format!("__rt.line_box_fill({args});")),
                }
            }
            Stmt::View { x1, y1, x2, y2, fill, border } => {
                let x1 = self.lift_expr(x1);
                let y1 = self.lift_expr(y1);
                let x2 = self.lift_expr(x2);
                let y2 = self.lift_expr(y2);
                let f  = fill.as_ref().map(|e| self.lift_expr(e))
                             .unwrap_or_else(|| "-1.0".into());
                let b  = border.as_ref().map(|e| self.lift_expr(e))
                               .unwrap_or_else(|| "-1.0".into());
                self.line(&format!("__rt.set_view({x1},{y1},{x2},{y2},{f},{b});"));
            }
            Stmt::Window { x1, y1, x2, y2, screen } => {
                let x1 = self.lift_expr(x1);
                let y1 = self.lift_expr(y1);
                let x2 = self.lift_expr(x2);
                let y2 = self.lift_expr(y2);
                self.line(&format!("__rt.set_window({x1},{y1},{x2},{y2},{screen});"));
            }
            Stmt::Erase(names) => self.emit_erase(names),
            Stmt::PaletteUsing(arr) => {
                // PALETTE USING arr[(start_idx)] — pass a slice of the array.
                // The arg can be indexed (`PaletteArray(0)`) or a bare array name
                // (`PALETTE USING Colors`). Resolve to the array binding directly so
                // a bare name is NOT routed through scalar-name disambiguation
                // (which would append `__sc` and slice an f64).
                let arr_binding = |this: &Self, name: &str| -> String {
                    let lower = name.to_lowercase();
                    if this.shared_names.contains(&lower) {
                        let ty = this.shared_types.get(&lower).cloned().unwrap_or(QbType::Double);
                        format!("__gs.{}", rust_ident_typed(name, &ty))
                    } else {
                        rust_ident(name)
                    }
                };
                let is_array = |this: &Self, name: &str| -> bool {
                    let lower = name.to_lowercase();
                    this.local_arrays.contains(&rust_ident(name))
                        || this.array_names.contains(&lower)
                };
                let (arr_name, start_idx) = match arr {
                    // Indexed: PALETTE USING arr(start)
                    Expr::Call { name, args } if !args.is_empty() => {
                        (arr_binding(self, name), self.lift_expr(&args[0]))
                    }
                    // Bare array name (no subscript) → slice from the lower bound.
                    Expr::Var(LValue::Scalar { name, .. }) if is_array(self, name) => {
                        let lo = self.arr_lo(&name.to_lowercase(), 0);
                        (arr_binding(self, name), lo.to_string())
                    }
                    Expr::Call { name, args } if args.is_empty() && is_array(self, name) => {
                        let lo = self.arr_lo(&name.to_lowercase(), 0);
                        (arr_binding(self, name), lo.to_string())
                    }
                    // Fallback (other expr) — best effort, start at 0.
                    _ => (self.lift_expr(arr), "0".to_string()),
                };
                self.line(&format!("__rt.palette_using(&{arr_name}[({start_idx}) as usize..]);"));
            }
            Stmt::SharedDecl(_) => { /* analyzer hint only — no code to emit */ }
            Stmt::Paint { x, y, fill, border, step } => {
                let xv = self.lift_expr(x);
                let yv = self.lift_expr(y);
                let str_fill = is_str_expr(fill) || self.is_str_expr_ctx(fill);
                let (px, py) = if *step {
                    let tc = self.lift_counter; self.lift_counter += 1;
                    self.line(&format!("let __stpx{tc} = __rt.cur_x() + ({xv});"));
                    self.line(&format!("let __stpy{tc} = __rt.cur_y() + ({yv});"));
                    (format!("__stpx{tc}"), format!("__stpy{tc}"))
                } else {
                    (xv, yv)
                };
                if str_fill {
                    // PAINT (x,y), CHR$(n)[+...], border — pattern tiling flood fill.
                    let tc = self.lift_counter; self.lift_counter += 1;
                    let fv = self.lift_expr(fill);
                    self.line(&format!("let __pat{tc}: String = {fv};"));
                    let b = border.as_ref().map(|e| self.lift_expr(e))
                                  .unwrap_or_else(|| "-1.0".into());
                    self.line(&format!("__rt.paint_pattern({px}, {py}, &__pat{tc}, {b});"));
                } else {
                    let f = self.lift_expr(fill);
                    let b = border.as_ref().map(|e| self.lift_expr(e))
                                  .unwrap_or_else(|| f.clone());
                    self.line(&format!("__rt.paint({px}, {py}, {f}, {b});"));
                }
            }
            Stmt::Pset { x, y, color, preset, step } => {
                let x = self.lift_expr(x);
                let y = self.lift_expr(y);
                let default_color = if *preset {
                    "__rt.bg_color as f64".into()
                } else {
                    "__rt.fg_color as f64".into()
                };
                let c = color.as_ref().map(|e| self.lift_expr(e))
                             .unwrap_or(default_color);
                if *step {
                    // STEP: point is relative to the current graphics cursor.
                    let tc = self.lift_counter; self.lift_counter += 1;
                    self.line(&format!("let __stx{tc} = __rt.cur_x() + ({x});"));
                    self.line(&format!("let __sty{tc} = __rt.cur_y() + ({y});"));
                    self.line(&format!("__rt.pset(__stx{tc}, __sty{tc}, {c});"));
                } else {
                    self.line(&format!("__rt.pset({x}, {y}, {c});"));
                }
            }

            // ── Sound ─────────────────────────────────────────────────────────
            Stmt::Play(e)  => { let s = self.emit_expr(e)?; self.line(&format!("__rt.play(&{s});")); }

            // MID$(var$, pos[, len]) = val — in-place substring replacement.
            Stmt::MidAssign { var, pos, len, val } => {
                let v = self.emit_lvalue(var);
                let p = self.emit_expr_inline(pos);
                let rhs = self.emit_expr(val)?;
                if let Some(l) = len {
                    let ln = self.emit_expr_inline(l);
                    self.line(&format!("qb_mid_assign(&mut {v}, {p}, Some({ln}), &{rhs});"));
                } else {
                    self.line(&format!("qb_mid_assign(&mut {v}, {p}, None, &{rhs});"));
                }
            }
            Stmt::Poke { addr, val } => {
                let a = self.emit_expr_inline(addr);
                let v = self.emit_expr_inline(val);
                self.line(&format!("__rt.qb_poke({a}, {v});"));
            }

            Stmt::Sound { freq, duration } => {
                // Hoist both args to temps to prevent double-borrow when freq
                // or duration contains __rt calls (e.g. RND).
                let tc = self.lift_counter; self.lift_counter += 1;
                let f = self.lift_expr(freq);
                let d = self.lift_expr(duration);
                self.line(&format!("let __sf{tc}: f64 = {f};"));
                self.line(&format!("let __sd{tc}: f64 = {d};"));
                self.line(&format!("__rt.sound(__sf{tc}, __sd{tc});"));
            }
            Stmt::Beep => self.line("__rt.beep();"),

            Stmt::Randomize(seed) => {
                if let Some(s) = seed {
                    let s = self.emit_expr(s)?;
                    self.line(&format!("__rt.randomize({s});"));
                } else {
                    self.line("__rt.randomize(qb_timer());");
                }
            }

            Stmt::Palette { attr, color64 } => {
                let a = self.emit_expr(attr)?;
                let c = self.emit_expr(color64)?;
                self.line(&format!("__rt.palette({a}, {c});"));
            }

            Stmt::PaletteReset => {
                self.line("__rt.palette_reset();");
            }

            Stmt::PutSprite { x, y, arr, action, step } => {
                // Hoist coords to temps to avoid borrow conflicts when args contain __rt calls
                let xv = self.emit_expr(x)?;
                let yv = self.emit_expr(y)?;
                let tc = self.lift_counter; self.lift_counter += 1;
                if *step {
                    // STEP: position is relative to the current graphics cursor.
                    self.line(&format!("let __spx{tc} = __rt.cur_x() + ({xv});"));
                    self.line(&format!("let __spy{tc} = __rt.cur_y() + ({yv});"));
                } else {
                    self.line(&format!("let __spx{tc} = {xv};"));
                    self.line(&format!("let __spy{tc} = {yv};"));
                }
                let arr_name = self.sprite_arr_name(arr);
                let verb = match action {
                    PutAction::Pset   => "Pset",
                    PutAction::Preset => "Preset",
                    PutAction::And    => "And",
                    PutAction::Or     => "Or",
                    PutAction::Xor    => "Xor",
                };
                self.line(&format!(
                    "__rt.put_sprite(&{arr_name}, __spx{tc}, __spy{tc}, qbasic_runtime::PutAction::{verb});"
                ));
            }

            Stmt::GetSprite { x1, y1, x2, y2, arr, step1, step2 } => {
                // Hoist coords to temps to avoid borrow conflicts when args contain __rt calls
                let x1v = self.emit_expr(x1)?;
                let y1v = self.emit_expr(y1)?;
                let x2v = self.emit_expr(x2)?;
                let y2v = self.emit_expr(y2)?;
                let tc = self.lift_counter; self.lift_counter += 1;
                if *step1 {
                    // First point STEP: relative to the current graphics cursor.
                    self.line(&format!("let __sgx1_{tc} = __rt.cur_x() + ({x1v});"));
                    self.line(&format!("let __sgy1_{tc} = __rt.cur_y() + ({y1v});"));
                } else {
                    self.line(&format!("let __sgx1_{tc} = {x1v};"));
                    self.line(&format!("let __sgy1_{tc} = {y1v};"));
                }
                if *step2 {
                    // Second point STEP: relative to the FIRST point (QB semantics).
                    self.line(&format!("let __sgx2_{tc} = __sgx1_{tc} + ({x2v});"));
                    self.line(&format!("let __sgy2_{tc} = __sgy1_{tc} + ({y2v});"));
                } else {
                    self.line(&format!("let __sgx2_{tc} = {x2v};"));
                    self.line(&format!("let __sgy2_{tc} = {y2v};"));
                }
                let arr_name = self.sprite_arr_name(arr);
                self.line(&format!("__rt.get_sprite(__sgx1_{tc}, __sgy1_{tc}, __sgx2_{tc}, __sgy2_{tc}, &mut {arr_name});"));
            }

            Stmt::Swap(a, b) => {
                // Check if swapping typed array elements — if so, swap each field separately
                if let (Some((arr_a, idx_a, fields)), Some((arr_b, idx_b, _))) =
                    (self.typed_array_index(a), self.typed_array_index(b))
                {
                    let (aa, ia, ab, ib, flds) =
                        (arr_a.clone(), idx_a.clone(), arr_b.clone(), idx_b.clone(), fields);
                    self.emit_typed_array_swap(&aa, &ia, &ab, &ib, &flds);
                } else {
                    // Detect SWAP arr(i), arr(j) on the same Vec — use Vec::swap to
                    // avoid dual &mut on the same allocation (Rust E0499).
                    let same_shared_vec = match (a, b) {
                        (LValue::Index { name: na, indices: ia, .. },
                         LValue::Index { name: nb, indices: ib, .. })
                            if na.to_lowercase() == nb.to_lowercase()
                               && self.shared_names.contains(&na.to_lowercase()) =>
                        {
                            let i0 = self.emit_expr(&ia[0]).unwrap_or_default();
                            let i1 = self.emit_expr(&ib[0]).unwrap_or_default();
                            let arr = format!("__gs.{}", rust_ident(na));
                            Some((arr, i0, i1))
                        }
                        _ => None,
                    };
                    if let Some((arr, i0, i1)) = same_shared_vec {
                        self.line(&format!("{arr}.swap(({i0}) as usize, ({i1}) as usize);"));
                    } else {
                        let la = self.emit_lvalue(a);
                        let lb = self.emit_lvalue(b);
                        self.line(&format!("std::mem::swap(&mut {la}, &mut {lb});"));
                    }
                }
            }

            Stmt::Call { name, args } => {
                let fn_lower = rust_ident(name);  // sigil-stripped lowercase
                if fn_lower == "sleep" {
                    // SLEEP n → __rt.sleep(n) so present() is called first
                    let a = args.first()
                        .map(|e| self.emit_expr(e).unwrap())
                        .unwrap_or_else(|| "0.0".into());
                    self.line(&format!("__rt.sleep({a});"));
                } else if matches!(fn_lower.as_str(), "chain" | "shell" | "environ") {
                    // CHAIN loads another BASIC program — treat as program end
                    self.line(&format!("// STUB: {name}"));
                    self.line("__rt.quit();");
                } else if fn_lower == "draw" {
                    // DRAW "turtle-graphics-string" → runtime method
                    let s = args.first()
                        .map(|e| self.emit_expr(e).unwrap())
                        .unwrap_or_else(|| "String::new()".into());
                    self.line(&format!("__rt.draw(&({s}));"));
                } else if self.user_subs.contains(&fn_lower) {
                    // User-defined SUB — prepend __rt, __gs; string args by-ref
                    let (a, writebacks) = self.emit_call_args_with_wb(&fn_lower, args);
                    let call = self.user_call(&fn_lower, &a.join(", "));
                    self.line(&format!("{call};"));
                    // Write back any shared scalar temps that were hoisted to avoid
                    // double-borrow of __gs (shared field passed as &mut param).
                    for (gs_field, tmp) in &writebacks {
                        self.line(&format!("{gs_field} = {tmp};"));
                    }
                } else if self.user_fns.contains(&fn_lower) {
                    // User-defined FUNCTION called as statement — use same args as expr context
                    let (a, writebacks) = self.emit_call_args_with_wb(&fn_lower, args);
                    let call = self.user_call(&fn_lower, &a.join(", "));
                    self.line(&format!("{call};"));
                    for (gs_field, tmp) in &writebacks {
                        self.line(&format!("{gs_field} = {tmp};"));
                    }
                } else {
                    // Built-in or unknown SUB
                    let a: Vec<_> = args.iter()
                        .map(|e| self.emit_expr(e).unwrap())
                        .collect();
                    self.line(&format!("{}({});", fn_lower, a.join(", ")));
                }
            }

            Stmt::PrintUsing { fmt, args, newline } => {
                let f = self.emit_expr(fmt)?;
                // Hoist each argument and wrap in QbVal::Num / QbVal::Str
                let mut qb_vals: Vec<String> = Vec::new();
                for e in args {
                    let v = self.emit_expr(e).unwrap_or_else(|_| "0.0".into());
                    if self.is_str_expr_ctx(e) {
                        // Lift to a named String temp so we can take a &str of it
                        let sn = format!("__pu_s{}", self.lift_counter);
                        self.lift_counter += 1;
                        self.line(&format!("let {sn} = ({v}).to_string();"));
                        qb_vals.push(format!("QbVal::Str(&{sn})"));
                    } else {
                        qb_vals.push(format!("QbVal::Num({v})"));
                    }
                }
                let arr = format!("[{}]", qb_vals.join(", "));
                let tmp = format!("__pu{}", self.lift_counter);
                self.lift_counter += 1;
                self.line(&format!("let {tmp} = qb_print_using(&({f}), &{arr});"));
                if *newline {
                    self.line(&format!("__rt.println(&[{tmp}]);"));
                } else {
                    self.line(&format!("__rt.print(&[{tmp}]);"));
                }
            }

            Stmt::End | Stmt::Stop => self.line("__rt.quit();"),

            // ── Error handling ────────────────────────────────────────────────
            Stmt::OnGoto { expr, labels, is_gosub } => {
                // ON expr GOTO/GOSUB L1,L2,… — 1-based, rounded; 0/out-of-range
                // falls through. Reuse the Goto/Gosub emission per branch so the
                // state-machine (`__pc`) and inline-GOSUB-fn logic are shared.
                let e = self.lift_expr(expr);
                self.line(&format!("match qb_cint({e}) as i64 {{"));
                self.indent();
                for (i, label) in labels.iter().enumerate() {
                    self.line(&format!("{} => {{", i + 1));
                    self.indent();
                    // Use Gosub if: (a) explicitly ON GOSUB, or (b) the named
                    // label was extracted as a GOSUB fn (user_fns has it) — this
                    // handles the `ON x GOTO Named_label` → fn call rewrite.
                    // Numeric GOTO targets remain GOTOs (state machine arms).
                    let treat_as_gosub = *is_gosub ||
                        (label.parse::<u32>().is_err()
                         && self.user_fns.contains(&rust_ident(label)));
                    if treat_as_gosub {
                        self.emit_stmt(&Stmt::Gosub(label.clone()))?;
                    } else {
                        self.emit_stmt(&Stmt::Goto(label.clone()))?;
                    }
                    self.dedent();
                    self.line("}");
                }
                self.line("_ => {}");
                self.dedent();
                self.line("}");
            }
            Stmt::OnError { label } => {
                if label == "0" {
                    self.on_error_label = String::new();
                    // Disable handler: any pending error is now a hard stop
                } else {
                    self.on_error_label = label.to_uppercase();
                }
                // No runtime code needed — the dispatch is emitted inline by
                // emit_error_dispatch() after each fallible statement.
            }
            Stmt::OnKeyGosub { key_num, target } => {
                // Collect this binding; the dispatcher function is emitted after
                // all subs/functions, just before fn main().
                let target_lc = target.to_lowercase();
                if !self.on_key_gosubs.iter().any(|(_, t)| t == &target_lc) {
                    self.on_key_gosubs.push((*key_num, target_lc));
                }
            }
            Stmt::OnTimerGosub { interval, target } => {
                self.on_timer_gosub = Some((*interval, target.to_lowercase()));
            }
            Stmt::Resume { next } => {
                // Clear the error flag and continue (RESUME NEXT).
                // True RESUME (retry) would need to re-run the faulting
                // statement — not feasible without coroutine machinery, so
                // we treat RESUME identically to RESUME NEXT.
                self.line("__rt.error_pending = false;");
                let _ = next; // both forms just clear and fall through
            }

            Stmt::Const { name, val } => {
                // In the main body, consts are already emitted globally by emit_consts().
                // In subs/functions, emit them inline so the value is available.
                if self.in_main { return Ok(()); }
                let rname = rust_ident(name);
                match val {
                    Expr::StrLit(s) => {
                        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                        self.line(&format!("let {rname}: &str = \"{escaped}\";"));
                    }
                    _ => {
                        // Numeric constant: fold to literal if possible, else evaluate.
                        let val_s = match val {
                            Expr::IntLit(n)   => format!("{n}.0f64"),
                            Expr::FloatLit(f) => emit_f64_lit(*f),
                            other             => self.emit_expr(other).unwrap_or_else(|_| "0.0".into()),
                        };
                        self.line(&format!("let {rname}: f64 = {val_s};"));
                    }
                }
            }
            Stmt::Block(inner) => self.emit_stmts(inner)?,
            Stmt::Data(_)      => { /* collected in analyzer */ }

            // ── File I/O ──────────────────────────────────────────────────────
            Stmt::Open { path, mode, file_num, rec_len } => {
                let path_s = self.emit_expr(path).unwrap_or_else(|_| "String::new()".into());
                let fnum   = self.emit_expr(file_num).unwrap_or_else(|_| "1.0".into());
                let mode_s = match mode {
                    FileMode::Input  => "\"input\"",
                    FileMode::Output => "\"output\"",
                    FileMode::Append => "\"append\"",
                    FileMode::Random => "\"random\"",
                    FileMode::Binary => "\"binary\"",
                };
                if *mode == FileMode::Random {
                    let rlen = rec_len.as_ref()
                        .and_then(|e| self.emit_expr(e).ok())
                        .unwrap_or_else(|| "128.0".into());
                    self.line(&format!(
                        "__rt.open_random(&({path_s}).to_string(), ({fnum}) as u8, ({rlen}) as usize);"
                    ));
                } else {
                    self.line(&format!(
                        "__rt.open_seq(&({path_s}).to_string(), {mode_s}, ({fnum}) as u8);"
                    ));
                }
                self.emit_error_dispatch();
            }
            Stmt::Close { file_nums } => {
                if file_nums.is_empty() {
                    self.line("__rt.close_all();");
                } else {
                    for e in file_nums {
                        let n = self.emit_expr(e).unwrap_or_else(|_| "1.0".into());
                        self.line(&format!("__rt.close_file(({n}) as u8);"));
                    }
                }
            }
            Stmt::Field { file_num, fields } => {
                // Compute the static file number (must be a literal for field tracking)
                let fnum_e = self.emit_expr(file_num).unwrap_or_else(|_| "1.0".into());
                // Try to extract a constant u8 file number for compile-time field tracking
                let fnum_u8 = match file_num {
                    Expr::IntLit(n) => Some(*n as u8),
                    Expr::FloatLit(f) => Some(*f as u8),
                    _ => None,
                };
                // Emit field variable initialization to declared lengths
                let mut offset = 0usize;
                let mut field_entries: Vec<(String, usize, usize)> = Vec::new();
                for (len_expr, var) in fields {
                    let len_s = self.emit_expr(len_expr).unwrap_or_else(|_| "0.0".into());
                    let len_u = match len_expr {
                        Expr::IntLit(n)   => *n as usize,
                        Expr::FloatLit(f) => *f as usize,
                        _ => 0,
                    };
                    let var_name = self.emit_lvalue(var);
                    // Initialize the field variable to a string of the declared length
                    self.line(&format!("{var_name} = \" \".repeat(({len_s}) as usize);"));
                    field_entries.push((var_name, offset, len_u));
                    offset += len_u;
                }
                // Register with runtime for record-length tracking
                self.line(&format!("__rt.set_field(({fnum_e}) as u8, {offset});"));
                if let Some(n) = fnum_u8 {
                    self.file_fields.insert(n, field_entries);
                }
            }
            Stmt::FileGet { file_num, record, record_var } => {
                let fnum = self.emit_expr(file_num).unwrap_or_else(|_| "1.0".into());
                let rec  = record.as_ref()
                    .and_then(|e| self.emit_expr(e).ok())
                    .unwrap_or_else(|| "0.0".into());
                let fnum_u8 = match file_num {
                    Expr::IntLit(n) => Some(*n as u8),
                    Expr::FloatLit(f) => Some(*f as u8),
                    _ => None,
                };
                // Emit: read the record buffer, then slice it into field variables
                let tmp = format!("__file_buf{}", self.lift_counter);
                self.lift_counter += 1;
                let rec_expr = if record.is_some() {
                    format!("Some(({rec}) as i64 - 1)")
                } else { "None".into() };
                self.line(&format!("let {tmp} = __rt.read_record(({fnum}) as u8, {rec_expr});"));
                // TYPE-record path: deserialize the buffer into the record var's fields.
                if let Some((base, tn)) = record_var.as_ref()
                    .and_then(|rv| self.resolve_record_var(rv))
                {
                    let mut fields = Vec::new();
                    let mut off = 0usize;
                    self.record_layout(&base, &tn, &mut off, &mut fields);
                    for (acc, repr, foff) in &fields {
                        self.line(&record_get_line(acc, repr, foff, &tmp));
                    }
                } else if let Some(n) = fnum_u8 {
                    // FIELD-based path.
                    if let Some(fields) = self.file_fields.get(&n).cloned() {
                        for (vname, off, len) in &fields {
                            self.line(&format!(
                                "{vname} = qb_field_get(&{tmp}, {off}, {len});"
                            ));
                        }
                    }
                }
            }
            Stmt::FilePut { file_num, record, record_var } => {
                let fnum = self.emit_expr(file_num).unwrap_or_else(|_| "1.0".into());
                let fnum_u8 = match file_num {
                    Expr::IntLit(n) => Some(*n as u8),
                    Expr::FloatLit(f) => Some(*f as u8),
                    _ => None,
                };
                let rec  = record.as_ref()
                    .and_then(|e| self.emit_expr(e).ok())
                    .unwrap_or_else(|| "0.0".into());
                let rec_expr = if record.is_some() {
                    format!("Some(({rec}) as i64 - 1)")
                } else { "None".into() };
                let tmp = format!("__put_buf{}", self.lift_counter);
                self.lift_counter += 1;
                // TYPE-record path: serialize the record var's fields into the buffer.
                if let Some((base, tn)) = record_var.as_ref()
                    .and_then(|rv| self.resolve_record_var(rv))
                {
                    let mut fields = Vec::new();
                    let mut off = 0usize;
                    self.record_layout(&base, &tn, &mut off, &mut fields);
                    self.line(&format!("let mut {tmp} = vec![b' '; {off}];"));
                    for (acc, repr, foff) in &fields {
                        self.line(&record_put_line(acc, repr, foff, &tmp));
                    }
                } else {
                    // FIELD-based path: assemble field variables into the buffer.
                    let total_len = fnum_u8
                        .and_then(|n| self.file_fields.get(&n))
                        .map(|f| f.iter().map(|(_, _, l)| l).sum::<usize>())
                        .unwrap_or(0);
                    self.line(&format!("let mut {tmp} = vec![b' '; {total_len}];"));
                    if let Some(n) = fnum_u8 {
                        if let Some(fields) = self.file_fields.get(&n).cloned() {
                            for (vname, off, len) in &fields {
                                self.line(&format!(
                                    "qb_field_put(&mut {tmp}, {off}, &{vname}, {len});"
                                ));
                            }
                        }
                    }
                }
                self.line(&format!(
                    "__rt.write_record(({fnum}) as u8, {rec_expr}, &{tmp});"
                ));
            }
            Stmt::LSet { var, expr } => {
                let lhs = self.emit_lvalue(var);
                let rhs = self.emit_expr(expr).unwrap_or_else(|_| "String::new()".into());
                self.line(&format!("{lhs} = qb_lset(&{lhs}, &({rhs}).to_string());"));
            }
            Stmt::RSet { var, expr } => {
                let lhs = self.emit_lvalue(var);
                let rhs = self.emit_expr(expr).unwrap_or_else(|_| "String::new()".into());
                self.line(&format!("{lhs} = qb_rset(&{lhs}, &({rhs}).to_string());"));
            }
            Stmt::PrintFile { file_num, args, newline } => {
                let fnum = self.emit_expr(file_num).unwrap_or_else(|_| "1.0".into());
                // Collect all args into a single string, then write to file
                let mut parts: Vec<String> = Vec::new();
                for arg in args {
                    match arg {
                        PrintArg::Expr(e) => {
                            let s = self.emit_expr_inline(e);
                            if self.is_str_expr_ctx(e) {
                                parts.push(format!("({s}).to_string()"));
                            } else {
                                parts.push(format!("qb_print_num({s})"));
                            }
                        }
                        PrintArg::Comma => parts.push("\" \".to_string()".into()),
                        PrintArg::Tab(e) => {
                            let v = self.emit_expr_inline(e);
                            parts.push(format!("\" \".repeat(({v}) as usize)"));
                        }
                        PrintArg::Spc(e) => {
                            let v = self.emit_expr_inline(e);
                            parts.push(format!("\" \".repeat(({v}) as usize)"));
                        }
                    }
                }
                let joined = if parts.is_empty() {
                    "String::new()".into()
                } else {
                    format!("format!(\"{{}}\", [{}].join(\"\"))", parts.join(", "))
                };
                if *newline {
                    self.line(&format!("__rt.write_file(({fnum}) as u8, &({joined} + \"\\n\"));"));
                } else {
                    self.line(&format!("__rt.write_file(({fnum}) as u8, &{joined});"));
                }
            }
            Stmt::InputFile { file_num, vars } => {
                let fnum = self.emit_expr(file_num).unwrap_or_else(|_| "1.0".into());
                let tmp = format!("__file_line{}", self.lift_counter);
                self.lift_counter += 1;
                self.line(&format!("let {tmp} = __rt.read_file_line(({fnum}) as u8);"));
                // Split on comma for multiple vars (QB INPUT # is CSV-like)
                if vars.len() == 1 {
                    let lhs = self.emit_lvalue(&vars[0]);
                    match &vars[0] {
                        LValue::Scalar { ty: QbType::String, .. } |
                        LValue::Scalar { ty: QbType::UserType(_), .. } => {
                            self.line(&format!("{lhs} = {tmp};"));
                        }
                        _ => {
                            // QB INPUT # trims whitespace before parsing a numeric field.
                            // qb_print_num() emits " N " (leading space for positives,
                            // trailing space always), so without .trim() the parse fails.
                            self.line(&format!("{lhs} = {tmp}.trim().parse().unwrap_or_default();"));
                        }
                    }
                } else {
                    let parts_var = format!("__file_parts{}", self.lift_counter);
                    self.lift_counter += 1;
                    self.line(&format!(
                        "let {parts_var}: Vec<&str> = {tmp}.splitn({}, ',').collect();",
                        vars.len()
                    ));
                    for (i, v) in vars.iter().enumerate() {
                        let lhs = self.emit_lvalue(v);
                        let src = format!("{parts_var}.get({i}).copied().unwrap_or(\"\").trim()");
                        match v {
                            LValue::Scalar { ty: QbType::String, .. } => {
                                self.line(&format!("{lhs} = {src}.to_string();"));
                            }
                            _ => {
                                self.line(&format!("{lhs} = {src}.parse().unwrap_or_default();"));
                            }
                        }
                    }
                }
            }
            Stmt::LineInputFile { file_num, var } => {
                let fnum = self.emit_expr(file_num).unwrap_or_else(|_| "1.0".into());
                let lhs = self.emit_lvalue(var);
                self.line(&format!("{lhs} = __rt.read_file_line(({fnum}) as u8);"));
            }
            Stmt::WriteFile { file_num, args } => {
                // WRITE #n — CSV output with values quoted if strings
                let fnum = self.emit_expr(file_num).unwrap_or_else(|_| "1.0".into());
                let mut parts: Vec<String> = Vec::new();
                for a in args {
                    let s = self.emit_expr_inline(a);
                    if self.is_str_expr_ctx(a) {
                        parts.push(format!("format!(\"\\\"{{}}\\\"\", {s})"));
                    } else {
                        parts.push(format!("qb_str_fn({s})"));
                    }
                }
                let line_s = format!("[{}].join(\",\")", parts.join(", "));
                self.line(&format!(
                    "__rt.write_file(({fnum}) as u8, &({line_s} + \"\\n\"));"
                ));
            }

            Stmt::Read(vars) => {
                for v in vars {
                    let lhs = self.emit_lvalue(v);
                    self.line(&format!(
                        "{lhs} = qb_read_data(&__DATA, &__DATA_PTR).parse().unwrap_or_default();"
                    ));
                }
            }
            Stmt::Restore(label) => {
                let pos = if let Some(lbl) = label {
                    // Look up the DATA index for this label
                    self.data_labels.get(&lbl.to_uppercase()).copied().unwrap_or(0)
                } else {
                    0
                };
                self.line(&format!("__DATA_PTR.store({pos}, std::sync::atomic::Ordering::SeqCst);"));
            }
        }
        Ok(())
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn emit_dim(&mut self, decl: &VarDecl) {
        // For arrays, use typed name so string arrays get _s suffix consistently.
        // For scalars we keep the bare rust_ident (scalars aren't looked up in local_arrays).
        let name  = if decl.dims.is_empty() {
            rust_ident(&decl.name)
        } else {
            rust_ident_typed(&decl.name, &decl.ty)
        };
        let is_shared = self.shared_names.contains(&decl.name.to_lowercase());

        if decl.dims.is_empty() {
            // Scalar
            if let QbType::UserType(type_name) = &decl.ty {
                // Scalar TYPE variable — recursively expand to individual field scalars
                let tn = type_name.to_lowercase();
                let flat = flatten_type_fields(&tn, &self.type_defs.clone());
                if !flat.is_empty() {
                    for (fname, fty) in &flat {
                        let frust = qb_type_to_rust(fty);
                        // Check if this is an array-typed field inside the TYPE body
                        let field_upper = self.type_field_dims.get(&tn)
                            .and_then(|fd| fd.get(fname.as_str()))
                            .copied();
                        if is_shared {
                            // Shared: emit Vec init into GameState (default gives empty Vec)
                            if let Some(upper) = field_upper {
                                let default_val = if frust == "String" { "String::new()" } else { "0.0" };
                                self.line(&format!(
                                    "__gs.{name}__{fname} = vec![{default_val}; {}];",
                                    upper + 1
                                ));
                            }
                            // Scalar shared fields are already default-initialized in GameState
                        } else if let Some(upper) = field_upper {
                            let default_val = if frust == "String" { "String::new()" } else { "0.0" };
                            self.line(&format!(
                                "let mut {name}__{fname}: Vec<{frust}> = vec![{default_val}; {}];",
                                upper + 1
                            ));
                        } else {
                            self.line(&format!("let mut {name}__{fname}: {frust} = Default::default();"));
                        }
                    }
                    return;
                }
            }
            if is_shared { return; } // plain shared scalar is default-initialized in GameState
            let ty  = qb_type_to_rust(&decl.ty);
            self.line(&format!("let mut {name}: {ty} = Default::default();"));
        } else {
            // Array — "wasted-slots" strategy: always allocate (upper + 1) elements
            // so that raw QB indices lo..=upper are always valid Vec indices.
            // This is safe to pass to SUBs (callee uses the same raw index as the
            // caller) at the cost of a few unused low-index slots when lo > 0.
            // LBOUND/UBOUND are computed from the stored `array_lower` map, not
            // from Vec::len(), so they return the correct declared bounds.
            let ndims = decl.dims.len();

            // Per-dimension allocation sizes (wasted-slots: upper + 1), outermost first.
            let allocs: Vec<String> = decl.dims.iter().map(|d| {
                let upper = self.emit_expr(d).unwrap_or_else(|_| "0".into());
                format!("({upper}+1.0) as usize")
            }).collect();
            let alloc0 = allocs[0].clone();
            // Typed-array paths below still use a 2-D-max (alloc1) shape.
            let alloc1 = allocs.get(1).cloned();

            if is_shared && matches!(&decl.ty, QbType::UserType(_)) {
                // DIM SHARED typed array → initialize per-field Vecs inside GameState
                let lower = rust_ident(&decl.name);
                self.typed_array_dims.insert(decl.name.to_lowercase(), ndims);
                let type_name = if let QbType::UserType(tn) = &decl.ty { tn.to_lowercase() } else { String::new() };
                // Use recursively-flattened fields so nested TYPEs expand correctly
                let flat_fields = flatten_type_fields(&type_name, &self.type_defs.clone());
                let flat_map: HashMap<String, QbType> = flat_fields.into_iter().collect();
                if let Some(fields) = self.typed_fields.get(&lower).cloned() {
                    for field in &fields {
                        let elem_ty = flat_map.get(field.as_str())
                            .map(|t| qb_type_to_rust(t))
                            .unwrap_or("f64");
                        let default_val = if elem_ty == "String" { "String::new()" } else { "0.0" };
                        // Check if the TYPE field is itself an array; if so add inner alloc.
                        let field_upper = self.type_field_dims.get(&type_name)
                            .and_then(|fd| fd.get(field.as_str()))
                            .copied();
                        if let Some(ref a1) = alloc1 {
                            // outer array is 2-D: always Vec<Vec<…>>
                            self.line(&format!(
                                "__gs.{lower}__{field} = \
                                 vec![vec![{default_val}; {a1}]; {alloc0}];"
                            ));
                        } else if let Some(fu) = field_upper {
                            // 1-D outer array + array field → Vec<Vec<…>>
                            self.line(&format!(
                                "__gs.{lower}__{field} = \
                                 vec![vec![{default_val}; {}]; {alloc0}];",
                                fu + 1
                            ));
                        } else {
                            self.line(&format!(
                                "__gs.{lower}__{field} = vec![{default_val}; {alloc0}];"
                            ));
                        }
                    }
                }
            } else if matches!(&decl.ty, QbType::UserType(_)) {
                // Local user-defined type array
                let lower = rust_ident(&decl.name);
                self.local_arrays.insert(lower.clone());
                self.typed_array_dims.insert(decl.name.to_lowercase(), ndims);
                let type_name = if let QbType::UserType(tn) = &decl.ty { tn.to_lowercase() } else { String::new() };
                // Use recursively-flattened fields so nested TYPEs expand correctly
                let flat_fields = flatten_type_fields(&type_name, &self.type_defs.clone());
                let flat_map: HashMap<String, QbType> = flat_fields.into_iter().collect();
                if let Some(fields) = self.typed_fields.get(&lower).cloned() {
                    for field in &fields {
                        let elem_ty = flat_map.get(field.as_str())
                            .map(|t| qb_type_to_rust(t))
                            .unwrap_or("f64");
                        let default_val = if elem_ty == "String" { "String::new()" } else { "0.0" };
                        // Check if the TYPE field is itself an array; if so add inner alloc.
                        let field_upper = self.type_field_dims.get(&type_name)
                            .and_then(|fd| fd.get(field.as_str()))
                            .copied();
                        if let Some(ref a1) = alloc1 {
                            // outer array is 2-D: always Vec<Vec<…>>
                            self.line(&format!(
                                "let mut {lower}__{field}: Vec<Vec<{elem_ty}>> = \
                                 vec![vec![{default_val}; {a1}]; {alloc0}];"
                            ));
                        } else if let Some(fu) = field_upper {
                            // 1-D outer array + array field → Vec<Vec<…>>
                            self.line(&format!(
                                "let mut {lower}__{field}: Vec<Vec<{elem_ty}>> = \
                                 vec![vec![{default_val}; {}]; {alloc0}];",
                                fu + 1
                            ));
                        } else {
                            self.line(&format!(
                                "let mut {lower}__{field}: Vec<{elem_ty}> = \
                                 vec![{default_val}; {alloc0}];"
                            ));
                        }
                    }
                }
            } else if is_shared {
                // Plain N-D shared array (1/2/3-D) → nested Vec inside GameState.
                let init = nested_vec_init("Default::default()", &allocs);
                self.line(&format!("__gs.{name} = {init};"));
            } else {
                let ty = qb_type_to_rust(&decl.ty);
                self.local_arrays.insert(name.clone());
                let init = nested_vec_init("Default::default()", &allocs);
                if self.sm_mode {
                    // In state-machine mode, `let mut` was hoisted before the loop;
                    // just emit the allocation assignment so the arm re-initializes it.
                    self.line(&format!("{name} = {init};"));
                } else {
                    let vec_ty = nested_vec_type(ty, ndims);
                    self.line(&format!("let mut {name}: {vec_ty} = {init};"));
                }
            }
        }
    }

    /// ERASE name[, name...] — reset each array's elements to their default
    /// (0.0 / empty string) in place. The Vec keeps its allocated shape (QB
    /// ERASE on a static array just zeros it). Nesting follows the array's
    /// dimensionality, looked up from `array_dims`.
    fn emit_erase(&mut self, names: &[String]) {
        for name in names {
            let lower   = rust_ident(name);
            let name_lc = name.to_lowercase();
            let is_shared = self.shared_names.contains(&name_lc);
            let ndims = self.array_dims.get(&name_lc).copied().unwrap_or(1).max(1);

            // Typed array → zero each per-field Vec.
            if let Some(fields) = self.typed_fields.get(&lower).cloned() {
                let field_types: Option<Vec<(String, QbType)>> = self.var_type_name
                    .get(&lower).cloned()
                    .and_then(|tn| self.type_defs.get(tn.as_str()).cloned());
                for field in &fields {
                    let elem_ty = field_types.as_ref()
                        .and_then(|fts| fts.iter().find(|(f, _)| f == field))
                        .map(|(_, t)| t.clone())
                        .unwrap_or(QbType::Single);
                    let dv = if elem_ty == QbType::String { "String::new()" } else { "Default::default()" };
                    let base = if is_shared {
                        format!("__gs.{lower}__{field}")
                    } else {
                        format!("{lower}__{field}")
                    };
                    self.emit_zero_nested(&base, ndims, dv);
                }
                continue;
            }

            // Plain array.
            let (base, elem_ty) = if is_shared {
                let ty = self.shared_types.get(&name_lc).cloned().unwrap_or(QbType::Single);
                (format!("__gs.{}", rust_ident_typed(name, &ty)), ty)
            } else {
                (lower.clone(), QbType::Single)
            };
            let dv = if elem_ty == QbType::String { "String::new()" } else { "Default::default()" };
            self.emit_zero_nested(&base, ndims, dv);
        }
    }

    /// Emit `ndims` nested `iter_mut` loops that reset every leaf of `base`
    /// (a Vec / Vec<Vec> / Vec<Vec<Vec>>) to `default_val`.
    fn emit_zero_nested(&mut self, base: &str, ndims: usize, default_val: &str) {
        let mut cur = base.to_string();
        for d in 0..ndims {
            let v = format!("__er{d}");
            self.line(&format!("for {v} in {cur}.iter_mut() {{"));
            self.indent();
            cur = v;
        }
        self.line(&format!("*{cur} = {default_val};"));
        for _ in 0..ndims {
            self.dedent();
            self.line("}");
        }
    }

    fn emit_redim(&mut self, decl: &VarDecl) {
        if decl.dims.is_empty() { return; }
        let name      = rust_ident_typed(&decl.name, &decl.ty);
        let is_shared = self.shared_names.contains(&decl.name.to_lowercase());
        // Wasted-slots: allocate upper+1 so raw QB indices are always valid.
        let upper0 = self.emit_expr(&decl.dims[0]).unwrap_or_else(|_| "0".into());
        let alloc0 = format!("({upper0}+1.0) as usize");

        let allocs: Vec<String> = decl.dims.iter().map(|d| {
            let upper = self.emit_expr(d).unwrap_or_else(|_| "0".into());
            format!("({upper}+1.0) as usize")
        }).collect();
        let ndims  = allocs.len();
        let alloc1 = allocs.get(1).cloned(); // typed path is still 2-D-max

        let elem_ty = qb_type_to_rust(&decl.ty);
        let default_val = if decl.ty == QbType::String { "String::new()" } else { "Default::default()" };
        // Fill value for resizing the outer Vec: the inner (N-1)-D structure.
        let inner_fill = if ndims <= 1 {
            default_val.to_string()
        } else {
            nested_vec_init(default_val, &allocs[1..])
        };

        if is_shared {
            // Shared (GameState) — resize.  For typed (UserType) arrays, resize each
            // per-field Vec separately since the struct has one Vec per field.
            let name_bare = rust_ident(&decl.name);
            if let Some(fields) = self.typed_fields.get(&name_bare).cloned() {
                let type_name_opt = if let QbType::UserType(tn) = &decl.ty {
                    Some(tn.to_lowercase())
                } else { None };
                let field_types: Option<Vec<(String, QbType)>> = type_name_opt
                    .and_then(|tn| self.type_defs.get(tn.as_str()))
                    .cloned();
                for field in &fields {
                    let elem_ty = field_types.as_ref()
                        .and_then(|fts| fts.iter().find(|(f, _)| f == field))
                        .map(|(_, t)| t.clone())
                        .unwrap_or(QbType::Single);
                    let dv = if elem_ty == QbType::String { "String::new()" } else { "Default::default()" };
                    if let Some(ref a1) = alloc1 {
                        self.line(&format!(
                            "__gs.{name_bare}__{field}.resize({alloc0}, vec![{dv}; {a1}]);"));
                    } else {
                        self.line(&format!("__gs.{name_bare}__{field}.resize({alloc0}, {dv});"));
                    }
                }
            } else {
                // Plain N-D shared array.
                self.line(&format!("__gs.{name}.resize({alloc0}, {inner_fill});"));
            }
        } else {
            // Local — may need to declare first
            if !self.redim_declared.contains(&name) {
                self.redim_declared.insert(name.clone());
                let vec_ty = nested_vec_type(elem_ty, ndims);
                self.line(&format!("let mut {name}: {vec_ty} = Vec::new();"));
            }
            self.line(&format!("{name}.resize({alloc0}, {inner_fill});"));
        }
    }

    fn emit_do(&mut self, kind: &DoKind, body: &[Stmt]) -> Result<()> {
        // Detect named GOTO targets at the tail of the body.  These are "skip
        // to end of loop iteration" patterns (QB equivalent of `continue`).
        // If any are found, label the loop so the GOTO can emit `continue 'lbl`.
        let bottom_labels = find_bottom_goto_labels(body);
        let rust_loop_label: Option<String> = if bottom_labels.is_empty() {
            None
        } else {
            let n = self.loop_label_counter;
            self.loop_label_counter += 1;
            let lbl = format!("'_loop_{n}");
            for name in &bottom_labels {
                self.named_loop_labels.insert(name.to_lowercase(), lbl.clone());
            }
            Some(lbl)
        };
        let loop_prefix = rust_loop_label.as_deref().map(|l| format!("{l}: ")).unwrap_or_default();

        match kind {
            DoKind::WhilePre(c) => {
                let c = self.emit_cond_expr(c)?;
                self.line(&format!("{loop_prefix}while {c} {{"));
                self.indent(); self.emit_stmts(body)?; self.dedent();
                self.line("}");
            }
            DoKind::UntilPre(c) => {
                let c = self.emit_cond_expr(c)?;
                self.line(&format!("{loop_prefix}while !({c}) {{"));
                self.indent(); self.emit_stmts(body)?; self.dedent();
                self.line("}");
            }
            DoKind::WhilePost(c) => {
                self.line(&format!("{loop_prefix}loop {{"));
                self.indent(); self.emit_stmts(body)?;
                let c = self.emit_cond_expr(c)?;
                self.line(&format!("if !({c}) {{ break; }}"));
                self.dedent(); self.line("}");
            }
            DoKind::UntilPost(c) => {
                // Detect `DO: LOOP UNTIL INKEY$ = ""` drain loop.  When ON KEY(n)/ON TIMER
                // GOSUB handlers are registered, inject event dispatch so arrow keys
                // and timers fire inside the drain loop instead of being silently consumed.
                let has_key = !self.on_key_gosubs.is_empty();
                let has_timer = self.on_timer_gosub.is_some();
                if body.is_empty() && is_inkey_eq_empty(c) && (has_key || has_timer) {
                    let gs_arg = if self.gamestate_emitted { ", __gs" } else { "" };
                    self.line(&format!("{loop_prefix}loop {{"));
                    self.indent();
                    self.line("let __k = __rt.inkey();");
                    if has_key {
                        self.line(&format!("if !__k.is_empty() {{ __handle_key_event(&__k, __rt{gs_arg}); }}"));
                    }
                    if has_timer {
                        self.line(&format!("__handle_timer_event(__rt{gs_arg});"));
                    }
                    self.line("if __k.is_empty() { break; }");
                    self.dedent(); self.line("}");
                } else {
                    self.line(&format!("{loop_prefix}loop {{"));
                    self.indent(); self.emit_stmts(body)?;
                    let c = self.emit_cond_expr(c)?;
                    self.line(&format!("if {c} {{ break; }}"));
                    self.dedent(); self.line("}");
                }
            }
            DoKind::Infinite => {
                self.line(&format!("{loop_prefix}loop {{"));
                self.indent(); self.emit_stmts(body)?; self.dedent();
                self.line("}");
            }
        }

        // Remove the labels we registered for this loop scope
        for name in &bottom_labels {
            self.named_loop_labels.remove(&name.to_lowercase());
        }
        Ok(())
    }

    fn emit_print(&mut self, args: &[PrintArg], newline: bool) -> Result<()> {
        // The runtime's print/println accept &[PrintItem] where each item is
        // either a string value or a "comma zone" placeholder.  To keep the ABI
        // simple, we emit a series of __rt.print() calls separated by
        // __rt.print_zone() for commas, then a final println/print for the newline.
        //
        // Strategy:
        //   - collect consecutive non-Comma args into a print() call
        //   - on Comma, emit print() for accumulated args, then __rt.print_zone()
        //   - after all args, emit println() / print() for the newline flag

        // Flush helper — emits an __rt.print call for accumulated parts
        let flush = |this: &mut Emitter, parts: &mut Vec<String>, is_final: bool, newline: bool| {
            if is_final {
                if newline {
                    this.line(&format!("__rt.println(&[{}]);", parts.join(", ")));
                } else {
                    this.line(&format!("__rt.print(&[{}]);", parts.join(", ")));
                }
            } else if !parts.is_empty() {
                this.line(&format!("__rt.print(&[{}]);", parts.join(", ")));
            }
            parts.clear();
        };

        // Split at Comma boundaries
        let mut parts: Vec<String> = Vec::new();
        let mut has_comma = false;
        for arg in args {
            if matches!(arg, PrintArg::Comma) {
                has_comma = true;
                break;
            }
        }

        if !has_comma {
            // Fast path — no commas: single call
            for arg in args {
                match arg {
                    PrintArg::Expr(e) => {
                        let v = self.lift_expr(e);
                        if is_str_expr(e) || self.is_str_expr_ctx(e) {
                            parts.push(format!("qb_str(&({v}))"));
                        } else {
                            // Numeric: QB PRINT adds leading sign-space and trailing space
                            parts.push(format!("qb_print_num({v})"));
                        }
                    }
                    PrintArg::Tab(e) => {
                        let v = self.lift_expr(e);
                        let tmp = format!("__tmp{}", self.lift_counter);
                        self.lift_counter += 1;
                        self.line(&format!("let {tmp} = __rt.tab({v});"));
                        parts.push(tmp);
                    }
                    PrintArg::Spc(e) => {
                        let v = self.lift_expr(e);
                        parts.push(format!("qb_space({v})"));
                    }
                    PrintArg::Comma => unreachable!(),
                }
            }
            flush(self, &mut parts, true, newline);
        } else {
            // Slow path — commas present: flush + print_zone between zones
            for (i, arg) in args.iter().enumerate() {
                let is_last = i == args.len() - 1;
                match arg {
                    PrintArg::Expr(e) => {
                        let v = self.lift_expr(e);
                        if is_str_expr(e) || self.is_str_expr_ctx(e) {
                            parts.push(format!("qb_str(&({v}))"));
                        } else {
                            parts.push(format!("qb_print_num({v})"));
                        }
                    }
                    PrintArg::Tab(e) => {
                        let v = self.lift_expr(e);
                        let tmp = format!("__tmp{}", self.lift_counter);
                        self.lift_counter += 1;
                        self.line(&format!("let {tmp} = __rt.tab({v});"));
                        parts.push(tmp);
                    }
                    PrintArg::Spc(e) => {
                        let v = self.lift_expr(e);
                        parts.push(format!("qb_space({v})"));
                    }
                    PrintArg::Comma => {
                        // Flush accumulated args, then advance to next print zone
                        flush(self, &mut parts, false, false);
                        self.line("__rt.print_zone();");
                    }
                }
                if is_last {
                    flush(self, &mut parts, true, newline);
                }
            }
            // If the last arg was Comma (trailing comma → no newline), parts is empty
            if !parts.is_empty() {
                flush(self, &mut parts, true, newline);
            }
        }
        Ok(())
    }

    fn emit_input(&mut self, prompt: &Option<String>, vars: &[LValue]) -> Result<()> {
        if let Some(p) = prompt {
            let escaped: String = p.chars().map(|c| match c {
                '"'  => "\\\"".into(),
                '\\' => "\\\\".into(),
                c if (c as u32) > 127 => format!("\\u{{{:04X}}}", c as u32),
                c    => c.to_string(),
            }).collect();
            self.line(&format!("__rt.print_str(\"{escaped}? \");"));
        }
        for v in vars {
            let lhs = self.emit_lvalue(v);
            match v {
                // String param (&mut String) — dereference-assign
                LValue::Scalar { name, ty: QbType::String }
                    if self.str_params.contains(&rust_ident_typed(name, &QbType::String)) =>
                {
                    self.line(&format!("*{lhs} = __rt.input_line();"));
                }
                // String local (String) — direct assign
                LValue::Scalar { ty: QbType::String, .. } => {
                    self.line(&format!("{lhs} = __rt.input_line();"));
                }
                // Numeric — parse (trim whitespace first; QB ignores leading/trailing spaces)
                _ => {
                    self.line(&format!(
                        "{lhs} = __rt.input_line().trim().parse().unwrap_or_default();"
                    ));
                }
            }
        }
        Ok(())
    }

    fn emit_case_cond(&self, case: &CaseBranch) -> Result<String> {
        // If case values are string expressions, wrap with .to_string() so that
        // Rust's String == String comparison works (String != &str directly).
        let wrap = |e: &Expr| -> String {
            let v = self.emit_expr_inline(e);
            if is_str_expr(e) { format!("{v}.to_string()") } else { v }
        };
        let parts: Vec<String> = case.conditions.iter().map(|c| match c {
            CaseCond::Value(e)    => format!("__sel == {}", wrap(e)),
            CaseCond::Range(a, b) => format!("(__sel >= {} && __sel <= {})",
                wrap(a), wrap(b)),
            CaseCond::Is(op, e)  => {
                let o = match op {
                    CmpOp::Eq => "==", CmpOp::Ne => "!=",
                    CmpOp::Lt => "<",  CmpOp::Le => "<=",
                    CmpOp::Gt => ">",  CmpOp::Ge => ">=",
                };
                format!("__sel {o} {}", wrap(e))
            }
        }).collect();
        Ok(parts.join(" || "))
    }

    fn emit_expr_inline(&self, expr: &Expr) -> String {
        self.emit_expr_inner(expr).unwrap_or_else(|_| "/*err*/".into())
    }

    /// Rust binding name for a local scalar. When the scalar's name collides with
    /// a local array of the same name (QB lets `A$` and `A$()` coexist as distinct
    /// variables), suffix the scalar so the two don't share one Rust binding.
    fn local_scalar_name(&self, rn: &str) -> String {
        if self.local_arrays.contains(rn) {
            format!("{rn}__sc")
        } else {
            rn.to_string()
        }
    }

    // ── LValue emission ───────────────────────────────────────────────────────

    fn emit_lvalue(&self, lval: &LValue) -> String {
        match lval {
            LValue::Scalar { name, ty } => {
                let lower = name.to_lowercase();
                // Guard: if the shared variable is numeric but THIS access is a
                // string (ty == String), they are DISTINCT variables that share
                // the same base name (e.g. numeric `X` and string `X$` both
                // normalise to key "x").  Reject only that direction.
                //
                // We do NOT reject when shared=String but access=Single:
                // a `DIM SHARED Available AS STRING` referenced without a `$`
                // sigil gets LValue type Single from the parser, so the shared
                // type may be String while ty is Single — they are the same var.
                let type_matches = self.shared_types.get(&lower)
                    .map(|sty| {
                        let shared_is_numeric = sty != &QbType::String;
                        let access_is_string  = ty  == &QbType::String;
                        // Reject: shared numeric slot ← string access
                        if shared_is_numeric && access_is_string { return false; }
                        // Reject: shared string slot ← numeric access, but ONLY when
                        // there is an explicit local `DIM name` in the current sub/fn
                        // that declares this name as a numeric type.  Without a local
                        // DIM, the sigil-less access `B` may just be `B$` referenced
                        // without its $ (QB allows `Available$` to be used as `Available`).
                        if !shared_is_numeric && !access_is_string
                            && self.local_dim_names.contains(&lower)
                        {
                            return false; // local integer shadows the shared string
                        }
                        true
                    })
                    .unwrap_or(true); // no entry in shared_types → assume OK
                let rn = rust_ident_typed(name, ty);
                // If this is a string param declared with `AS STRING` (no sigil), the Rust
                // param was renamed to name_s (&mut String).  Return the bare name so
                // Stmt::Let can prepend `*` (→ `*name_s = …`); for reads, emit_expr_inner
                // handles dereferencing separately.
                let rn_s = rust_ident_typed(name, &QbType::String);
                if self.str_params.contains(&rn_s) {
                    return rn_s;
                }
                if self.numeric_params.contains(&rn) {
                    // Byref numeric param — parameters shadow any shared var with the same
                    // base name (e.g. SUB DrawPlayer(Player%) shadows DIM SHARED Player(1 TO 2)).
                    format!("(*{rn})")
                } else if self.shared_names.contains(&lower) && type_matches {
                    // For shared scalars, use the bare rust_ident (no sigil suffix).
                    // The GameState field was generated from the DIM declaration name,
                    // which may differ from the $ sigil form used at access sites.
                    // e.g. `DIM Available AS STRING` → `available: String` in GameState,
                    // but `Available$` access would produce `available_s` via rust_ident_typed.
                    let gs_name = rust_ident(name);
                    format!("__gs.{gs_name}")
                } else if self.current_fn_name_lc.as_deref() == Some(rn.as_str()) {
                    // Assignment to the function name inside a FUNCTION body →
                    // redirect to the "__fn_ret" local so recursive calls aren't shadowed.
                    "__fn_ret".to_string()
                } else {
                    // QB allows a scalar `A$` and an array `A$()` to coexist — they
                    // are distinct variables. Disambiguate the scalar binding.
                    self.local_scalar_name(&rn)
                }
            }
            LValue::Index { name, ty, indices } => {
                let lower = name.to_lowercase();
                // Wasted-slots: raw QB index is the Vec index directly.
                let subscript: String = indices.iter()
                    .map(|e| format!("[({}) as usize]", self.emit_expr_inline(e)))
                    .collect();
                // For shared arrays, use the authoritative type from shared_types;
                // for local string arrays (DIM name(...) AS STRING without $ sigil),
                // use QbType::String so the correct `_s`-suffixed name is produced.
                // (The AST LValue type may be stale, e.g. Single instead of String.)
                let effective_ty = if self.shared_names.contains(&lower) {
                    self.shared_types.get(&lower).cloned().unwrap_or_else(|| ty.clone())
                } else if self.local_string_arrays.contains(&lower) {
                    QbType::String
                } else {
                    ty.clone()
                };
                // Use rust_ident_typed so string arrays (help$) become help_s not help
                let rname = rust_ident_typed(name, &effective_ty);
                if self.shared_names.contains(&lower) {
                    format!("__gs.{rname}{subscript}")
                } else {
                    format!("{rname}{subscript}")
                }
            }
            LValue::Field { base, field } => {
                // Walk the full chain of nested Field nodes to build the flat field suffix,
                // e.g. Field(Field(Index(pts,i),"col"),"r") → pts__col__r[(i) as usize].
                let mut field_parts: Vec<String> = vec![rust_ident(field)];
                let mut cur: &LValue = base;
                while let LValue::Field { base: next, field: f } = cur {
                    field_parts.push(rust_ident(f));
                    cur = &**next;
                }
                field_parts.reverse(); // innermost first
                let field_suffix = field_parts.join("__");

                match cur {
                    LValue::Index { name, indices, .. } => {
                        let name_lc = name.to_lowercase();
                        let flat    = format!("{}__{field_suffix}", rust_ident(name));
                        let arr_field = if self.shared_names.contains(&name_lc) {
                            format!("__gs.{flat}")
                        } else {
                            flat
                        };
                        let subscript: String = indices.iter()
                            .map(|e| format!("[({}) as usize]", self.emit_expr_inline(e)))
                            .collect();
                        format!("{arr_field}{subscript}")
                    }
                    LValue::Scalar { name, .. } => {
                        let name_lc = name.to_lowercase();
                        let flat    = format!("{}__{field_suffix}", rust_ident(name));
                        if self.shared_names.contains(&name_lc) {
                            format!("__gs.{flat}")
                        } else if self.numeric_params.contains(&flat) {
                            // Scalar TYPE param — individual field is a &mut f64 param
                            format!("(*{flat})")
                        } else {
                            flat
                        }
                    }
                    other => {
                        // Unexpected deeper nesting — recursive fallback
                        format!("{}__{field_suffix}", self.emit_lvalue(other))
                    }
                }
            }
            LValue::FieldIndex { base, field, indices } => {
                // scalar.arrayField(idx) — array field inside a TYPE variable.
                // Two cases:
                //   scalar:  g.Cell(j)       → g__cell[j]
                //   indexed: boards(i).Cell(j) → boards__cell[i][j]
                let field_id = rust_ident(field);
                let inner_sub: String = indices.iter()
                    .map(|e| format!("[({}) as usize]", self.emit_expr_inline(e)))
                    .collect();
                match base.as_ref() {
                    LValue::Scalar { name, .. } => {
                        let name_lc = name.to_lowercase();
                        let flat = format!("{}__{field_id}", rust_ident(name));
                        let prefix = if self.shared_names.contains(&name_lc) {
                            format!("__gs.{flat}")
                        } else {
                            flat
                        };
                        format!("{prefix}{inner_sub}")
                    }
                    LValue::Index { name, indices: outer_indices, .. } => {
                        // arr(i).Field(j) → arr__field[i][j]
                        let name_lc = name.to_lowercase();
                        let flat = format!("{}__{field_id}", rust_ident(name));
                        let prefix = if self.shared_names.contains(&name_lc) {
                            format!("__gs.{flat}")
                        } else {
                            flat
                        };
                        let outer_sub: String = outer_indices.iter()
                            .map(|e| format!("[({}) as usize]", self.emit_expr_inline(e)))
                            .collect();
                        format!("{prefix}{outer_sub}{inner_sub}")
                    }
                    other => {
                        let base_str = self.emit_lvalue(other);
                        format!("{base_str}__{field_id}{inner_sub}")
                    }
                }
            }
        }
    }

    // ── Random-access TYPE-record (GET/PUT #n, rec, var) serialization ─────────

    /// If `lval` is (or indexes into) a TYPE variable with a known record
    /// layout, return the base LValue to (de)serialize and the lowercase TYPE
    /// name. A *bare array name* (no subscript) is normalized to its element at
    /// the lower bound — QB writes only the first element per record. Returns
    /// `None` when the variable isn't a TYPE record (caller falls back to the
    /// FIELD-based path).
    fn resolve_record_var(&self, lval: &LValue) -> Option<(LValue, String)> {
        match lval {
            LValue::Scalar { name, .. } => {
                let key = rust_ident(name);
                let tn  = self.var_type_name.get(&key)?.clone();
                if !self.type_layouts.contains_key(&tn) { return None; }
                let name_lc = name.to_lowercase();
                let is_array = self.array_names.contains(&name_lc)
                    || self.local_arrays.contains(&key);
                if is_array {
                    // Bare array → element at lower bound (QB element-1 semantics).
                    let lo = self.arr_lo(&name_lc, 0) as i32;
                    let base = LValue::Index {
                        name: name.clone(),
                        ty: QbType::UserType(tn.clone()),
                        indices: vec![Expr::IntLit(lo)],
                    };
                    Some((base, tn))
                } else {
                    Some((lval.clone(), tn))
                }
            }
            LValue::Index { name, .. } => {
                let key = rust_ident(name);
                let tn  = self.var_type_name.get(&key)?.clone();
                if !self.type_layouts.contains_key(&tn) { return None; }
                Some((lval.clone(), tn))
            }
            _ => None,
        }
    }

    /// Recursively flatten a TYPE's on-disk layout, producing one entry per leaf
    /// scalar field: (rust_accessor, repr, byte_offset). `base` is the record
    /// variable's LValue; each leaf builds `base.field…` and emits it via
    /// `emit_lvalue`, so accessor naming matches field access everywhere else.
    fn record_layout(
        &self,
        base: &LValue,
        type_name: &str,
        off: &mut usize,
        out: &mut Vec<(String, FieldRepr, usize)>,
    ) {
        let Some(layout) = self.type_layouts.get(type_name).cloned() else { return; };
        for (fname, repr) in &layout {
            let field_lv = LValue::Field {
                base: Box::new(base.clone()),
                field: fname.clone(),
            };
            match repr {
                FieldRepr::Nested(tn) => self.record_layout(&field_lv, tn, off, out),
                FieldRepr::Str(n) => {
                    out.push((self.emit_lvalue(&field_lv), repr.clone(), *off));
                    *off += *n;
                }
                FieldRepr::I16 => {
                    out.push((self.emit_lvalue(&field_lv), repr.clone(), *off));
                    *off += 2;
                }
                FieldRepr::I32 | FieldRepr::F32 => {
                    out.push((self.emit_lvalue(&field_lv), repr.clone(), *off));
                    *off += 4;
                }
                FieldRepr::F64 => {
                    out.push((self.emit_lvalue(&field_lv), repr.clone(), *off));
                    *off += 8;
                }
            }
        }
    }

    /// If `lval` is an index into a known typed array, return (base_name, index_expr, fields).
    /// Returns `(arr_rust_name, subscript, fields)` where `subscript` is the full
    /// bracketed index string for all dimensions, e.g. `"[(x) as usize][(y) as usize]"`.
    fn typed_array_index<'a>(&'a self, lval: &'a LValue)
        -> Option<(String, String, Vec<String>)>
    {
        if let LValue::Index { name, ty, indices } = lval {
            let lower = rust_ident_typed(name, ty);
            if let Some(fields) = self.typed_fields.get(lower.as_str()) {
                let subscript: String = indices.iter()
                    .map(|idx| format!("[({}) as usize]", self.emit_expr_inline(idx)))
                    .collect();
                return Some((lower, subscript, fields.clone()));
            }
        }
        None
    }

    /// Same as typed_array_index but handles `Expr::Call { name, args }` — the parser
    /// produces Call nodes for `arr(i)` on the RHS of assignments (not LValue::Index).
    fn typed_array_call(&self, expr: &Expr) -> Option<(String, String, Vec<String>)> {
        if let Expr::Call { name, args } = expr {
            if args.is_empty() { return None; }
            let lower = rust_ident(name);
            let name_lc = name.to_lowercase();
            let is_typed_arr = self.typed_fields.contains_key(lower.as_str())
                && (self.shared_names.contains(&name_lc)
                    || self.local_arrays.contains(&lower)
                    || self.array_names.contains(&name_lc));
            if is_typed_arr {
                if let Some(fields) = self.typed_fields.get(lower.as_str()) {
                    let subscript: String = args.iter()
                        .map(|a| format!("[({}) as usize]", self.emit_expr_inline(a)))
                        .collect();
                    return Some((lower, subscript, fields.clone()));
                }
            }
        }
        None
    }

    /// If `name` (rust_ident form) is a scalar TYPE variable, return its fields.
    fn scalar_type_fields(&self, name: &str) -> Option<Vec<String>> {
        let type_name = self.var_type_name.get(name)?;
        let fields = self.type_defs.get(type_name.as_str())?;
        Some(fields.iter().map(|(f, _)| f.clone()).collect())
    }

    /// Whole-record copy between two scalar TYPE variables (`OldBlock = CurBlock`):
    /// emit one per-field assignment. `lhs`/`rhs` are rust_ident names; `fields`
    /// are the flattened field paths (matching the GameState field names).
    fn emit_scalar_type_copy(&mut self, lhs: &str, rhs: &str, fields: &[String]) {
        for f in fields {
            let lf = format!("{lhs}__{f}");
            let l = if self.numeric_params.contains(&lf) {
                format!("*{lf}")
            } else if self.shared_names.contains(lhs) {
                format!("__gs.{lf}")
            } else { lf };
            let rf = format!("{rhs}__{f}");
            let r = if self.numeric_params.contains(&rf) {
                format!("(*{rf})")
            } else if self.shared_names.contains(rhs) {
                format!("__gs.{rf}")
            } else { rf };
            self.line(&format!("{l} = {r}.clone();"));
        }
    }

    /// Emit field-by-field copy from a typed array element to another typed array element.
    /// `lhs_arr`, `lhs_idx`: destination; `rhs_arr`, `rhs_idx`: source.
    /// All names are rust_ident-lowercase.
    /// `lhs_sub` and `rhs_sub` are full bracket subscript strings, e.g.
    /// `"[(x) as usize]"` or `"[(x) as usize][(y) as usize]"` for multi-dim arrays.
    fn emit_typed_array_copy(&mut self, lhs_arr: &str, lhs_sub: &str,
                                        rhs_arr: &str, rhs_sub: &str,
                                        fields: &[String])
    {
        for field in fields {
            let lhs_prefix = if self.shared_names.contains(lhs_arr) {
                format!("__gs.{lhs_arr}__{field}")
            } else { format!("{lhs_arr}__{field}") };
            let rhs_prefix = if self.shared_names.contains(rhs_arr) {
                format!("__gs.{rhs_arr}__{field}")
            } else { format!("{rhs_arr}__{field}") };
            // Use clone() for the rhs to avoid borrow issues with String fields
            self.line(&format!(
                "{lhs_prefix}{lhs_sub} = {rhs_prefix}{rhs_sub}.clone();"
            ));
        }
    }

    /// Emit field-by-field copy from a typed array element to a scalar TYPE variable.
    /// `sub` is the full bracket subscript string, e.g. `"[(x) as usize][(y) as usize]"`.
    fn emit_typed_arr_to_scalar(&mut self, scalar: &str, arr: &str, sub: &str, fields: &[String]) {
        for field in fields {
            let arr_prefix = if self.shared_names.contains(arr) {
                format!("__gs.{arr}__{field}")
            } else { format!("{arr}__{field}") };
            let sf = format!("{scalar}__{field}");
            let lhs = if self.numeric_params.contains(&sf) {
                format!("*{sf}")
            } else if self.shared_names.contains(scalar) {
                format!("__gs.{sf}")
            } else { sf };
            self.line(&format!(
                "{lhs} = {arr_prefix}{sub}.clone();"
            ));
        }
    }

    /// Emit field-by-field copy from a scalar TYPE variable to a typed array element.
    /// `sub` is the full bracket subscript string.
    fn emit_scalar_to_typed_arr(&mut self, arr: &str, sub: &str, scalar: &str, fields: &[String]) {
        for field in fields {
            let arr_prefix = if self.shared_names.contains(arr) {
                format!("__gs.{arr}__{field}")
            } else { format!("{arr}__{field}") };
            let sf = format!("{scalar}__{field}");
            let rhs = if self.numeric_params.contains(&sf) {
                format!("(*{sf})")
            } else if self.shared_names.contains(scalar) {
                format!("__gs.{sf}")
            } else { sf };
            self.line(&format!(
                "{arr_prefix}{sub} = {rhs}.clone();"
            ));
        }
    }

    /// Emit field-by-field SWAP between two typed array elements.
    /// `sub_a` and `sub_b` are full bracket subscript strings.
    fn emit_typed_array_swap(&mut self, arr_a: &str, sub_a: &str,
                                       arr_b: &str, sub_b: &str,
                                       fields: &[String])
    {
        for field in fields {
            let prefix_a = if self.shared_names.contains(arr_a) {
                format!("__gs.{arr_a}__{field}")
            } else { format!("{arr_a}__{field}") };
            let prefix_b = if self.shared_names.contains(arr_b) {
                format!("__gs.{arr_b}__{field}")
            } else { format!("{arr_b}__{field}") };
            if arr_a == arr_b {
                // Same Vec — use a temp to avoid the double-mutable-borrow error.
                // f64 fields are Copy, String fields need clone/reassign.
                let tc = self.lift_counter; self.lift_counter += 1;
                self.line(&format!("{{ let __swap_tmp{tc} = {prefix_a}{sub_a}.clone();"));
                self.line(&format!("  {prefix_a}{sub_a} = {prefix_b}{sub_b}.clone();"));
                self.line(&format!("  {prefix_b}{sub_b} = __swap_tmp{tc}; }}"));
            } else {
                self.line(&format!(
                    "std::mem::swap(&mut {prefix_a}{sub_a}, &mut {prefix_b}{sub_b});"
                ));
            }
        }
    }

    /// Return the bare array variable name for GET/PUT sprite ops (always shared for gorilla.bas).
    fn sprite_arr_name(&self, lval: &LValue) -> String {
        match lval {
            LValue::Scalar { name, ty } => {
                let rn = rust_ident_typed(name, ty);
                if self.shared_names.contains(&name.to_lowercase()) {
                    format!("__gs.{rn}")
                } else {
                    rn
                }
            }
            // For GET/PUT the array is always the whole vec, not an indexed element
            LValue::Index { name, .. } => {
                let lower = name.to_lowercase();
                if self.shared_names.contains(&lower) {
                    format!("__gs.{}", rust_ident(name))
                } else {
                    rust_ident(name)
                }
            }
            other => self.emit_lvalue(other),
        }
    }

    // ── Emit CALL arguments — expands arrays, creates string temps ────────────

    /// Emit arguments for a user-SUB call.  Returns one Rust expression string
    /// per QB argument, except for typed-array arguments which expand to one
    /// element per TYPE field.  May emit `let mut __tmp_str_N` temporaries as
    /// a side-effect for string-expression arguments.
    ///
    /// `sub_name` is the lowercased rust_ident of the callee; it is used to look
    /// up parameter names so that typed arrays passed under a different local name
    /// (e.g. `sammy()` → param `snake`) use the *param*'s field list.
    /// Returns (arg_strings, writebacks) where writebacks are (gs_field_path, temp_var) pairs
    /// to emit after the call: `gs_field = temp_var;`
    fn emit_call_args_with_wb(&mut self, sub_name: &str, args: &[Expr])
        -> (Vec<String>, Vec<(String, String)>)
    {
        let (args_out, wb) = {
            let mut wb_out: Vec<(String, String)> = Vec::new();
            let args_out = self.emit_call_args_inner(sub_name, args, &mut wb_out);
            (args_out, wb_out)
        };
        (args_out, wb)
    }

    fn emit_call_args_inner(&mut self, sub_name: &str, args: &[Expr],
                            writebacks: &mut Vec<(String, String)>) -> Vec<String> {
        // Build positional list of typed-array param names for the target sub
        let target_params: Vec<Option<String>> = {
            let ps = self.sub_params.get(sub_name).cloned().unwrap_or_default();
            // Only typed-array params matter; track which arg position maps to which param name
            let mut out = Vec::new();
            for p in &ps {
                if !p.dims.is_empty() {
                    out.push(Some(rust_ident(&p.name)));
                } else {
                    out.push(None);
                }
            }
            out
        };
        // ── Pre-scan: detect aliased array args (same array in multiple positions) ──
        // For each local/param array name, find which arg positions use it.
        // All but the last position get a cloned temporary; the last gets the real ref.
        let mut arr_positions: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, expr) in args.iter().enumerate() {
            if let Expr::Call { name, args: iargs } = expr {
                if iargs.is_empty() {
                    let lower = rust_ident(name);
                    if self.local_arrays.contains(&lower) || self.array_params.contains(&lower) {
                        arr_positions.entry(lower).or_default().push(i);
                    }
                }
            }
        }
        // Emit clone bindings for non-last occurrences of aliased arrays.
        // alias_for[i] = Some(clone_var_name) means arg i should use that clone.
        let mut alias_for: HashMap<usize, String> = HashMap::new();
        for (lower, positions) in &arr_positions {
            if positions.len() > 1 {
                // All but the last position get clones
                for &pos in &positions[..positions.len() - 1] {
                    let clone_var = format!("__arr_alias_{}", self.lift_counter);
                    self.lift_counter += 1;
                    if self.array_params.contains(lower) {
                        self.line(&format!("let mut {clone_var} = {lower}.clone();"));
                    } else {
                        self.line(&format!("let mut {clone_var} = {lower}.clone();"));
                    }
                    alias_for.insert(pos, clone_var);
                }
            }
        }

        let mut result = Vec::new();
        let mut borrowed: HashSet<String> = HashSet::new(); // tracks &mut f64 scalar borrows
        for (arg_i, expr) in args.iter().enumerate() {
            // ── Array-pass-whole: BCoor(), TotalWins(), Record() ──────────────
            if let Expr::Call { name, args: iargs } = expr {
                if iargs.is_empty() {
                    let lower = rust_ident(name);
                    // Use target param's field list when arg name differs from param name
                    // (e.g. sammy() passed as snake() in EraseSnake)
                    let field_key: String = target_params.get(arg_i)
                        .and_then(|o| o.as_deref())
                        .unwrap_or(lower.as_str())
                        .to_owned();
                    // Typed array → expand to per-field &mut
                    if let Some(fields) = self.typed_fields.get(field_key.as_str()).cloned() {
                        let in_gs = self.shared_names.contains(&lower);
                        for field in &fields {
                            if in_gs {
                                // Use std::mem::take to extract the Vec from __gs so we can
                                // pass &mut to the field while also passing __gs mutably.
                                // The writeback restores it after the call.
                                let tc = self.lift_counter; self.lift_counter += 1;
                                let tmp = format!("__taf{tc}");
                                self.line(&format!(
                                    "let mut {tmp} = std::mem::take(&mut __gs.{lower}__{field});"
                                ));
                                writebacks.push((
                                    format!("__gs.{lower}__{field}"),
                                    tmp.clone(),
                                ));
                                result.push(format!("&mut {tmp}"));
                            } else {
                                result.push(format!("&mut {lower}__{field}"));
                            }
                        }
                        continue;
                    }
                    // Shared array — use std::mem::take to avoid double-borrow of __gs
                    if self.shared_names.contains(&lower)
                        && self.array_names.contains(&lower)
                    {
                        let tc = self.lift_counter; self.lift_counter += 1;
                        let tmp = format!("__saf{tc}");
                        self.line(&format!(
                            "let mut {tmp} = std::mem::take(&mut __gs.{lower});"));
                        writebacks.push((format!("__gs.{lower}"), tmp.clone()));
                        result.push(format!("&mut {tmp}"));
                        continue;
                    }
                    // Local or param array
                    if self.local_arrays.contains(&lower)
                        || self.array_params.contains(&lower)
                    {
                        if let Some(clone_var) = alias_for.get(&arg_i) {
                            // Aliased non-last occurrence — use the pre-emitted clone
                            result.push(format!("&mut {clone_var}"));
                        } else if self.array_params.contains(&lower) {
                            // &mut Vec<f64> param — explicit reborrow so multiple uses compile
                            result.push(format!("&mut *{lower}"));
                        } else {
                            result.push(format!("&mut {lower}"));
                        }
                        continue;
                    }
                }
            }

            // ── Typed array element T(idx) → expand to per-field &mut refs ──────
            // Handles e.g. `TileDraw T(Index(Til))` where T() AS Tile expands to
            // `&mut gs.t__x1[idx], &mut gs.t__x2[idx], ...`
            if let Expr::Call { name, args: iargs } = expr {
                if !iargs.is_empty() {
                    let lower    = rust_ident(name);
                    let name_lc  = name.to_lowercase();
                    let in_shared = self.shared_names.contains(&name_lc);
                    let is_typed_arr = self.typed_fields.contains_key(lower.as_str())
                        && (in_shared
                            || self.local_arrays.contains(&lower)
                            || self.array_names.contains(&name_lc));
                    if is_typed_arr {
                        if let Some(fields) = self.typed_fields.get(lower.as_str()).cloned() {
                            // Hoist all indices to temps (one per dimension) evaluated
                            // once and shared across all field accesses.
                            let mut idx_temps: Vec<String> = Vec::new();
                            for idx in iargs {
                                let idx_val = self.lift_expr(idx);
                                let tc = self.lift_counter; self.lift_counter += 1;
                                self.line(&format!("let __taidx{tc} = ({idx_val}) as usize;"));
                                idx_temps.push(format!("__taidx{tc}"));
                            }
                            let subscript: String = idx_temps.iter()
                                .map(|t| format!("[{t}]"))
                                .collect();
                            if in_shared {
                                // Shared TYPE array: borrow __gs whole for the call AND
                                // its fields → conflict. Hoist each field to a temp and
                                // write back after the call.
                                for field in &fields {
                                    let gs_path = format!("__gs.{lower}__{field}{subscript}");
                                    let tc = self.lift_counter; self.lift_counter += 1;
                                    let tmp = format!("__tmp_gs{tc}");
                                    self.line(&format!("let mut {tmp} = {gs_path};"));
                                    writebacks.push((gs_path, tmp.clone()));
                                    result.push(format!("&mut {tmp}"));
                                }
                            } else {
                                for field in &fields {
                                    result.push(format!("&mut {lower}__{field}{subscript}"));
                                }
                            }
                            continue;
                        }
                    }
                }
            }

            // ── Plain array element arr(i) → hoist + write back (QB byref) ──────
            // QB passes array elements by reference: `CALL Swap(a(i), a(j))` must
            // mutate the array. Hoist the element to a temp (also avoids E0499
            // when two elements of the same array are passed) and write back.
            if let Expr::Call { name, args: iargs } = expr {
                if !iargs.is_empty() {
                    let name_bare = name.trim_end_matches(['$', '%', '!', '#', '&']).to_lowercase();
                    let is_string = name.ends_with('$')
                        || self.local_string_arrays.contains(&name.to_lowercase())
                        || matches!(self.shared_types.get(&name_bare), Some(QbType::String));
                    let typed_name = if is_string {
                        rust_ident_typed(name, &QbType::String)
                    } else {
                        rust_ident(name)
                    };
                    let in_shared = self.shared_names.contains(&name_bare)
                        && self.array_names.contains(&name_bare);
                    let is_plain_arr = !self.typed_fields.contains_key(rust_ident(name).as_str())
                        && !self.user_fns.contains(&rust_ident(name))
                        && (in_shared
                            || self.local_arrays.contains(&typed_name)
                            || self.array_params.contains(&rust_ident(name)));
                    if is_plain_arr {
                        // Evaluate each index exactly once
                        let mut subscript = String::new();
                        for idx in iargs {
                            let idx_val = self.lift_expr(idx);
                            let tc = self.lift_counter; self.lift_counter += 1;
                            self.line(&format!("let __baidx{tc} = ({idx_val}) as usize;"));
                            subscript.push_str(&format!("[__baidx{tc}]"));
                        }
                        let path = if in_shared {
                            format!("__gs.{typed_name}{subscript}")
                        } else {
                            format!("{typed_name}{subscript}")
                        };
                        let tc = self.lift_counter; self.lift_counter += 1;
                        if is_string {
                            let tmp = format!("__tmp_arrs{tc}");
                            self.line(&format!("let mut {tmp}: String = {path}.clone();"));
                            writebacks.push((path, format!("{tmp}.clone()")));
                            result.push(format!("&mut {tmp}"));
                        } else {
                            let tmp = format!("__tmp_arr{tc}");
                            self.line(&format!("let mut {tmp}: f64 = {path};"));
                            writebacks.push((path, tmp.clone()));
                            result.push(format!("&mut {tmp}"));
                        }
                        continue;
                    }
                }
            }

            // ── String scalar lvalue → pass &mut ─────────────────────────────
            if let Expr::Var(LValue::Scalar { name, ty: QbType::String }) = expr {
                let lower = rust_ident_typed(name, &QbType::String);
                let lval = if self.shared_names.contains(&name.to_lowercase()) {
                    format!("&mut __gs.{lower}")
                } else if self.str_params.contains(&lower) {
                    // Already a &mut String param — Rust auto-reborrows
                    lower
                } else {
                    format!("&mut {lower}")
                };
                result.push(lval);
                continue;
            }

            // ── String param declared AS STRING (no sigil) → pass as &mut String ─
            // e.g. `nm AS STRING` → Rust param `nm_s: &mut String`; in the body it
            // has ty=Single from the parser but str_params contains "nm_s".
            if let Expr::Var(LValue::Scalar { name, .. }) = expr {
                let rn_s = rust_ident_typed(name, &QbType::String);
                if self.str_params.contains(&rn_s) {
                    // Already a &mut String param — pass it onward (Rust auto-reborrows)
                    result.push(rn_s);
                    continue;
                }
                // Shared scalar declared AS STRING but accessed without $ sigil:
                // look up shared_types to detect, then hoist to temp (borrow-safe).
                let lc = name.to_lowercase();
                if !self.local_dim_names.contains(&lc) {
                    if let Some(QbType::String) = self.shared_types.get(&lc) {
                        let gs_name = rust_ident(name);
                        let gs_field = format!("__gs.{gs_name}");
                        let tmp = format!("__tmp_gs{}", self.lift_counter);
                        self.lift_counter += 1;
                        self.line(&format!("let mut {tmp}: String = {gs_field}.clone();"));
                        writebacks.push((gs_field, format!("{tmp}.clone()")));
                        result.push(format!("&mut {tmp}"));
                        continue;
                    }
                }
            }

            // ── String expression (temporary) → materialize as mut local ─────
            if is_str_expr(expr) || self.is_str_expr_ctx(expr) {
                let tmp = format!("__tmp_str{}", self.lift_counter);
                self.lift_counter += 1;
                let val = self.emit_expr_inner(expr)
                    .unwrap_or_else(|_| "String::new()".into());
                self.line(&format!("let mut {tmp} = ({val}).to_string();"));
                result.push(format!("&mut {tmp}"));
                continue;
            }

            // ── Scalar UserType lvalue → expand to per-field &mut args ──────────
            if let Expr::Var(LValue::Scalar { name, .. }) = expr {
                let base_name = rust_ident(name);
                let name_lc   = name.to_lowercase();
                // Check var_type_name to see if this scalar has a UserType
                let type_name_opt = self.var_type_name.get(&base_name).cloned();
                if let Some(tn_lc) = type_name_opt {
                    let flat = flatten_type_fields(&tn_lc, &self.type_defs.clone());
                    if !flat.is_empty() {
                        for (fname, _) in &flat {
                            let field_var = format!("{base_name}__{fname}");
                            // Field might itself be a byref param (when passing a SUB param onward)
                            let arg = if self.shared_names.contains(&name_lc) {
                                // Shared TYPE field lives in __gs; passing both __gs
                                // and &mut __gs.field would double-borrow (E0499).
                                // Hoist to a temp and write back after the call so
                                // byref mutations still propagate.
                                let gs_field = format!("__gs.{field_var}");
                                let tmp = format!("__tmp_gst{}", self.lift_counter);
                                self.lift_counter += 1;
                                self.line(&format!("let mut {tmp} = {gs_field}.clone();"));
                                writebacks.push((gs_field, tmp.clone()));
                                format!("&mut {tmp}")
                            } else if self.numeric_params.contains(&field_var) {
                                // Already &mut — reborrow
                                format!("&mut *{field_var}")
                            } else {
                                format!("&mut {field_var}")
                            };
                            result.push(arg);
                        }
                        continue;
                    }
                }
            }

            // ── Numeric scalar lvalue → pass &mut (QB byref semantics) ──────────
            if let Expr::Var(LValue::Scalar { name, ty }) = expr {
                if *ty != QbType::String {
                    let rn = rust_ident_typed(name, ty);
                    let lower = name.to_lowercase();
                    // If this same variable is already borrowed &mut in this call,
                    // hoist to a temp to avoid aliasing (Rust E0499)
                    let borrow_key = if self.shared_names.contains(&lower) {
                        format!("__gs.{rn}")
                    } else {
                        rn.clone()
                    };
                    if borrowed.contains(&borrow_key) {
                        // Alias: copy into a temp; writeback would require more bookkeeping,
                        // but aliased byref is undefined behavior in QB anyway
                        let val = self.emit_expr_inner(expr).unwrap_or_else(|_| "0.0".into());
                        let tmp = format!("__tmp_num{}", self.lift_counter);
                        self.lift_counter += 1;
                        self.line(&format!("let mut {tmp}: f64 = {val};"));
                        result.push(format!("&mut {tmp}"));
                        continue;
                    }
                    borrowed.insert(borrow_key);
                    let arg = if self.shared_names.contains(&lower) {
                        // Shared field of __gs: hoist to a temp so we can pass both
                        // __gs and &mut field without a double-borrow conflict.
                        let gs_field = format!("__gs.{rn}");
                        let tmp = format!("__tmp_gs{}", self.lift_counter);
                        self.lift_counter += 1;
                        self.line(&format!("let mut {tmp}: f64 = {gs_field};"));
                        writebacks.push((gs_field, tmp.clone()));
                        format!("&mut {tmp}")
                    } else if self.numeric_params.contains(&rn) {
                        // Already a &mut f64 in caller — reborrow
                        rn
                    } else {
                        format!("&mut {rn}")
                    };
                    result.push(arg);
                    continue;
                }
            }

            // ── Default: numeric expression — hoist to temp so we can &mut it ──
            {
                let val = self.emit_expr_inner(expr)
                    .unwrap_or_else(|_| "0.0".into());
                let tmp = format!("__tmp_num{}", self.lift_counter);
                self.lift_counter += 1;
                self.line(&format!("let mut {tmp}: f64 = {val};"));
                result.push(format!("&mut {tmp}"));
            }
        }
        result
    }

    fn emit_expr(&self, expr: &Expr) -> Result<String> { self.emit_expr_inner(expr) }

    /// Emit a condition expression for use directly in `if`/`while`.
    /// When the top-level expression is a comparison BinOp, emits the Rust comparison
    /// directly (e.g. `x == 9.0`) instead of the double-wrap `qb_bool(qb_from_bool(x == 9.0))`.
    /// AND/OR and all other QB expressions still go through `qb_bool(...)`.
    fn emit_cond_expr(&self, expr: &Expr) -> Result<String> {
        if let Expr::BinOp { op, lhs, rhs } = expr {
            let rust_op = match op {
                BinOp::Eq => Some("=="),
                BinOp::Ne => Some("!="),
                BinOp::Lt => Some("<"),
                BinOp::Le => Some("<="),
                BinOp::Gt => Some(">"),
                BinOp::Ge => Some(">="),
                _ => None,
            };
            if let Some(op_str) = rust_op {
                let l = self.emit_expr_inner(lhs)?;
                let r = self.emit_expr_inner(rhs)?;
                // String comparison: normalize both sides to &str
                let (l_cmp, r_cmp) = if is_str_expr(lhs) || is_str_expr(rhs) {
                    let lc = if is_str_expr(lhs) && !matches!(lhs.as_ref(), Expr::StrLit(_)) {
                        format!("({l}).as_str()")
                    } else { l.clone() };
                    let rc = if is_str_expr(rhs) && !matches!(rhs.as_ref(), Expr::StrLit(_)) {
                        format!("({r}).as_str()")
                    } else { r.clone() };
                    (lc, rc)
                } else {
                    (l.clone(), r.clone())
                };
                return Ok(format!("{l_cmp} {op_str} {r_cmp}"));
            }
        }
        let c = self.emit_expr(expr)?;
        Ok(format!("qb_bool({c})"))
    }

    /// Like emit_expr but lifts user-fn call sub-expressions to `let __tmp_N`
    /// temporaries, emitting those bindings inline. Use this when the expression
    /// appears inside an `__rt.method(...)` argument list to avoid double-borrow.
    fn lift_expr(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Call { name, args } => {
                let lower = rust_ident(name); // sigil-stripped lowercase
                let name_lc = name.to_lowercase(); // full lowercase WITH sigil, for built-in checks
                // Sigil-stripped bare name for array-vs-function disambiguation:
                // Expr::Call stores the full name-with-sigil ("Names$") while
                // shared_names/array_names keys are sigil-free ("names").
                let name_bare = name.trim_end_matches(['$', '%', '!', '#', '&']).to_lowercase();
                // Array access — not a fn call, emit directly
                if (self.shared_names.contains(&name_bare) && self.array_names.contains(&name_bare))
                    || self.local_arrays.contains(&lower)
                    || self.array_params.contains(&lower)
                    || self.local_string_arrays.contains(&name_bare)
                {
                    return self.emit_expr_inner(expr).unwrap_or_else(|_| "0.0".into());
                }
                // User-defined FUNCTION — lift to temp, handling &mut String args
                if self.user_fns.contains(&lower) {
                    let mut call_args: Vec<String> = Vec::new();
                    for e in args {
                        // String scalar → clone to mut temp, pass &mut
                        if let Expr::Var(LValue::Scalar { name: vn, ty: QbType::String }) = e {
                            let rn = rust_ident_typed(vn, &QbType::String);
                            let vn_lc = vn.to_lowercase();
                            // Check if this is an array passed whole (not a scalar)
                            let is_arr = self.local_arrays.contains(&rn)
                                || self.array_params.contains(&rn)
                                || (self.shared_names.contains(&vn_lc) && self.array_names.contains(&vn_lc));
                            if is_arr {
                                // Array arg — pass as &mut directly
                                let src = if self.shared_names.contains(&vn_lc) {
                                    format!("&mut __gs.{rn}")
                                } else {
                                    format!("&mut {rn}")
                                };
                                call_args.push(src);
                            } else {
                                let src = if self.shared_names.contains(&vn_lc) {
                                    format!("__gs.{rn}")
                                } else if self.str_params.contains(&rn) {
                                    format!("(*{rn})")
                                } else {
                                    rn.clone()
                                };
                                let tmp_s = format!("__tmp_s{}", self.lift_counter);
                                self.lift_counter += 1;
                                self.line(&format!("let mut {tmp_s}: String = ({src}).clone();"));
                                call_args.push(format!("&mut {tmp_s}"));
                            }
                        } else if is_str_expr(e) || self.is_str_expr_ctx(e) {
                            // Check if this is a whole-array pass (e.g. choice$() with empty args)
                            let is_arr_pass = if let Expr::Call { name: n, args: ea } = e {
                                if ea.is_empty() {
                                    let ln = rust_ident(n);
                                    self.local_arrays.contains(&ln)
                                        || self.array_params.contains(&ln)
                                        || (self.shared_names.contains(&n.to_lowercase())
                                            && self.array_names.contains(&n.to_lowercase()))
                                } else { false }
                            } else if let Expr::Var(LValue::Scalar { name: n, .. }) = e {
                                let rn = rust_ident(n);
                                self.local_arrays.contains(&rn) || self.array_params.contains(&rn)
                            } else { false };

                            if is_arr_pass {
                                // Whole array — pass as &mut directly
                                let v = self.emit_expr_inner(e).unwrap_or_default();
                                call_args.push(format!("&mut {v}"));
                            } else {
                                // String expr → materialize to mut temp, pass &mut
                                let v = self.emit_expr_inner(e).unwrap_or_default();
                                let tmp_s = format!("__tmp_s{}", self.lift_counter);
                                self.lift_counter += 1;
                                self.line(&format!("let mut {tmp_s}: String = ({v}).to_string();"));
                                call_args.push(format!("&mut {tmp_s}"));
                            }
                        } else {
                            // Check if it's a numeric array passed whole — needs &mut
                            let arr_whole = match e {
                                Expr::Var(LValue::Scalar { name: n, .. }) => {
                                    let rn = rust_ident(n);
                                    (self.local_arrays.contains(&rn) || self.array_params.contains(&rn))
                                        && !is_str_expr(e) && !self.is_str_expr_ctx(e)
                                }
                                Expr::Call { name: n, args: ea } if ea.is_empty() => {
                                    let rn = rust_ident(n);
                                    (self.local_arrays.contains(&rn) || self.array_params.contains(&rn)
                                        || (self.shared_names.contains(&n.to_lowercase()) && self.array_names.contains(&n.to_lowercase())))
                                        && !is_str_expr(e)
                                }
                                _ => false,
                            };
                            if arr_whole {
                                let v = self.emit_expr_inner(e).unwrap_or_default();
                                call_args.push(format!("&mut {v}"));
                            } else {
                                let v = self.lift_expr(e);
                                if v.contains("__gs") {
                                    // Reading a shared field while we also pass
                                    // `&mut __gs` to the call conflicts (E0503) —
                                    // hoist the value to a temp first.
                                    let tmp = format!("__fa{}", self.lift_counter);
                                    self.lift_counter += 1;
                                    self.line(&format!("let {tmp} = {v};"));
                                    call_args.push(tmp);
                                } else {
                                    call_args.push(v);
                                }
                            }
                        }
                    }
                    let sep = if call_args.is_empty() { "" } else { ", " };
                    let rt = if self.in_main { "&mut __rt, &mut __gs" } else { "__rt, __gs" };
                    let call = format!("{lower}({rt}{sep}{})", call_args.join(", "));
                    let tmp = format!("__tmp{}", self.lift_counter);
                    self.lift_counter += 1;
                    self.line(&format!("let {tmp} = {call};"));
                    return tmp;
                }
                // Built-in — recurse into args but don't lift
                let a: Vec<String> = args.iter().map(|a| self.lift_expr(a)).collect();
                // Special cases that need __rt — hoist to a temp (like Expr::Point
                // below) so the call doesn't double-borrow __rt when it ends up as
                // an argument to another `__rt.method(...)` (e.g. PRINT INT(RND*100)).
                let mut hoist = |this: &mut Self, call: String| -> String {
                    let tmp = format!("__tmp{}", this.lift_counter);
                    this.lift_counter += 1;
                    this.line(&format!("let {tmp} = {call};"));
                    tmp
                };
                if name_lc == "rnd"    {
                    if let Some(arg0) = args.first() {
                        let av = self.lift_expr(arg0);
                        return hoist(self, format!("__rt.rnd_arg({av})"));
                    }
                    return hoist(self, "__rt.rnd()".into());
                }
                if name_lc == "inkey$" { return hoist(self, "__rt.inkey()".into()); }
                if name_lc == "peek" && a.len() == 1 {
                    return hoist(self, format!("__rt.qb_peek({})", a[0]));
                }
                // PLAY(n) function form — returns notes remaining in background queue.
                if name_lc == "play" {
                    return hoist(self, "__rt.play_count()".into());
                }
                if name_lc == "pmap" && a.len() == 2 {
                    return hoist(self, format!("__rt.pmap({}, {})", a[0], a[1]));
                }
                if name_lc == "input$" {
                    let n = a.first().cloned().unwrap_or_else(|| "1.0".into());
                    return hoist(self, format!("__rt.input_str({n})"));
                }
                // UBOUND / LBOUND
                if name_lc == "ubound" || name_lc == "lbound" {
                    // Resolve array QB name for lower-bound lookup.
                    let arr_qb_lc: String = match args.first() {
                        // array_lower keys are sigil-free — strip before lookup
                        Some(Expr::Var(LValue::Scalar { name: n, .. }))
                        | Some(Expr::Call { name: n, args: _ }) => n
                            .trim_end_matches(['$', '%', '!', '#', '&'])
                            .to_lowercase(),
                        _ => String::new(),
                    };
                    // Optional second arg: dimension number (1-based QB).
                    let dim_idx: usize = match args.get(1) {
                        Some(Expr::IntLit(d))   => (*d as usize).saturating_sub(1),
                        Some(Expr::FloatLit(d)) => (*d as usize).saturating_sub(1),
                        _ => 0,
                    };
                    let lo = if arr_qb_lc.is_empty() { 0_i64 }
                             else { self.arr_lo(&arr_qb_lc, dim_idx) };
                    if name_lc == "lbound" {
                        return format!("{lo}.0");
                    }
                    // UBOUND: wasted-slots means Vec has (upper+1) elements, so
                    // arr.len() - 1 == upper exactly — no lo adjustment needed.
                    if let Some(_first) = a.first() {
                        let arr_name_raw = match args.first() {
                            Some(Expr::Var(LValue::Scalar { name: n, ty })) => rust_ident_typed(n, ty),
                            Some(Expr::Call { name: n, args: ea }) if ea.is_empty() => {
                                // String arrays carry an _s suffix locally
                                if n.ends_with('$')
                                   || self.local_string_arrays.contains(&n.to_lowercase()) {
                                    rust_ident_typed(n, &QbType::String)
                                } else {
                                    rust_ident(n)
                                }
                            }
                            _ => return format!("(({}.len() as f64) - 1.0)", a[0]),
                        };
                        let arr_lc = arr_name_raw.trim_end_matches("_s").to_string();
                        if self.shared_names.contains(&arr_lc) {
                            // For TYPE arrays flattened to per-field Vecs, use the first
                            // field Vec (all fields have the same length).
                            let rname = if let Some(fields) = self.typed_fields.get(arr_lc.as_str()) {
                                if let Some(f0) = fields.first() {
                                    format!("{arr_lc}__{f0}")
                                } else {
                                    rust_ident_typed(&arr_lc, &self.shared_types.get(&arr_lc).cloned().unwrap_or(QbType::Single))
                                }
                            } else if let Some(ty) = self.shared_types.get(&arr_lc) {
                                rust_ident_typed(&arr_lc, ty)
                            } else {
                                arr_name_raw.clone()
                            };
                            return format!("((__gs.{rname}.len() as f64) - 1.0)");
                        }
                        return format!("(({arr_name_raw}.len() as f64) - 1.0)");
                    }
                    return "0.0".to_string();
                }
                let fn_name = rust_fn_name(name);
                // INSTR 2-arg form
                if fn_name == "qb_instr" && args.len() == 2 {
                    let a0 = if matches!(&args[0], Expr::StrLit(_)) { a[0].clone() } else { format!("&({})", a[0]) };
                    let a1 = if matches!(&args[1], Expr::StrLit(_)) { a[1].clone() } else { format!("&({})", a[1]) };
                    return format!("qb_instr(1.0, {a0}, {a1})");
                }
                // MID$ optional len
                if fn_name == "qb_mid" {
                    let a0 = if matches!(&args[0], Expr::StrLit(_)) { a[0].clone() } else { format!("&({})", a[0]) };
                    let a2 = if a.len() >= 3 { format!("Some({})", a[2]) } else { "None".into() };
                    return format!("qb_mid({a0}, {}, {a2})", a[1]);
                }
                // STRING$(n, x): x may be char-code or string
                if fn_name == "qb_string" && args.len() == 2 {
                    if is_str_expr(&args[1]) {
                        let a1 = if matches!(&args[1], Expr::StrLit(_)) { a[1].clone() } else { format!("&({})", a[1]) };
                        return format!("qb_string_s({}, {a1})", a[0]);
                    } else {
                        return format!("qb_string({}, {})", a[0], a[1]);
                    }
                }
                let str_pos = str_arg_positions(&fn_name);
                let a2: Vec<String> = args.iter().enumerate()
                    .zip(a.iter())
                    .map(|((i, e), s)| {
                        if str_pos.contains(&i) && !matches!(e, Expr::StrLit(_)) {
                            format!("&({s})")
                        } else {
                            s.clone()
                        }
                    })
                    .collect();
                format!("{fn_name}({})", a2.join(", "))
            }
            Expr::BinOp { op, lhs, rhs } => {
                let l = self.lift_expr(lhs);
                let r = self.lift_expr(rhs);
                // String concatenation
                if *op == BinOp::Add && (is_str_expr(lhs) || is_str_expr(rhs)) {
                    return format!("format!(\"{{}}{{}}\" ,{l},{r})");
                }
                // String comparison: normalize both sides to &str
                let (l_cmp, r_cmp) = if matches!(op,
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge)
                    && (is_str_expr(lhs) || is_str_expr(rhs))
                {
                    let lc = if is_str_expr(lhs) && !matches!(lhs.as_ref(), Expr::StrLit(_)) {
                        format!("({l}).as_str()")
                    } else { l.clone() };
                    let rc = if is_str_expr(rhs) && !matches!(rhs.as_ref(), Expr::StrLit(_)) {
                        format!("({r}).as_str()")
                    } else { r.clone() };
                    (lc, rc)
                } else {
                    (l.clone(), r.clone())
                };
                match op {
                    BinOp::Add    => format!("({l} + {r})"),
                    BinOp::Sub    => format!("({l} - {r})"),
                    BinOp::Mul    => format!("({l} * {r})"),
                    BinOp::Div    => format!("({l} / {r})"),
                    BinOp::IntDiv => format!("qb_idiv({l}, {r})"),
                    BinOp::Pow    => format!("{l}.powf({r})"),
                    BinOp::Mod    => format!("qb_mod({l}, {r})"),
                    BinOp::Eq     => format!("qb_from_bool({l_cmp} == {r_cmp})"),
                    BinOp::Ne     => format!("qb_from_bool({l_cmp} != {r_cmp})"),
                    BinOp::Lt     => format!("qb_from_bool({l_cmp} < {r_cmp})"),
                    BinOp::Le     => format!("qb_from_bool({l_cmp} <= {r_cmp})"),
                    BinOp::Gt     => format!("qb_from_bool({l_cmp} > {r_cmp})"),
                    BinOp::Ge     => format!("qb_from_bool({l_cmp} >= {r_cmp})"),
                    BinOp::And    => format!("qb_and({l}, {r})"),
                    BinOp::Or     => format!("qb_or({l}, {r})"),
                    BinOp::Xor    => format!("qb_xor({l}, {r})"),
                    BinOp::Eqv    => format!("qb_eqv({l}, {r})"),
                    BinOp::Imp    => format!("qb_imp({l}, {r})"),
                }
            }
            Expr::UnOp { op, operand } => {
                let o = self.lift_expr(operand);
                match op {
                    UnOp::Neg => format!("(-{o})"),
                    UnOp::Not => format!("qb_not({o})"),
                }
            }
            Expr::Point { x, y } => {
                // POINT is __rt.point() — but it's inside another __rt call,
                // so lift it too
                let x = self.lift_expr(x);
                let y = self.lift_expr(y);
                let call = format!("__rt.point({x}, {y})");
                let tmp = format!("__tmp{}", self.lift_counter);
                self.lift_counter += 1;
                self.line(&format!("let {tmp} = {call};"));
                tmp
            }
            // Zero-arg user FUNCTION referenced without parens (e.g. `ComputeMem` in
            // `HEX$(ComputeMem)`) — emit_expr_inner would inline the call as
            // `computemem(__rt, __gs)`, which double-borrows __rt/__gs when nested
            // inside another __rt method call.  Hoist to a temp here, same as the
            // Expr::Call branch above handles explicit `ComputeMem()`.
            Expr::Var(LValue::Scalar { name, ty }) => {
                let lower = rust_ident(name);
                let lower_typed = rust_ident_typed(name, ty);
                // String-returning functions declared as `FUNCTION Foo$()` have their
                // `$` stripped by the parser when stored in the AST, so `name` = "Foo"
                // and `lower` = "foo", but `user_fns` contains "foo_s" (from the `$`
                // sigil on the definition).  Check the typed variant first.
                let fn_name = if self.user_fns.contains(&lower_typed) && !self.user_subs.contains(&lower_typed) {
                    Some(lower_typed)
                } else if self.user_fns.contains(&lower) && !self.user_subs.contains(&lower) {
                    Some(lower)
                } else {
                    None
                };
                if let Some(fn_name) = fn_name {
                    let rt = if self.in_main { "&mut __rt, &mut __gs" } else { "__rt, __gs" };
                    let call = format!("{fn_name}({rt})");
                    let tmp = format!("__tmp{}", self.lift_counter);
                    self.lift_counter += 1;
                    self.line(&format!("let {tmp} = {call};"));
                    tmp
                } else {
                    self.emit_expr_inner(expr).unwrap_or_else(|_| "0.0".into())
                }
            }
            // Literals and other simple refs — safe to emit inline
            _ => self.emit_expr_inner(expr).unwrap_or_else(|_| "0.0".into()),
        }
    }

    // ── Param emission — numerics by value, strings by &mut, arrays by &mut Vec

    fn emit_params(&self, params: &[VarDecl], _body: &[Stmt]) -> String {
        let mut parts = Vec::new();
        for p in params {
            let name = rust_ident_typed(&p.name, &p.ty);
            if !p.dims.is_empty() {
                // Array parameter — use base rust_ident (no _s, arrays aren't strings)
                let arr_name = rust_ident(&p.name);
                if let Some(fields) = self.typed_fields.get(&arr_name) {
                    // Typed array → one &mut Vec per TYPE field; 2-D arrays use Vec<Vec<f64>>
                    let ndims = self.typed_array_dims.get(&arr_name)
                        .copied().unwrap_or(1);
                    for field in fields {
                        if ndims >= 2 {
                            parts.push(format!("{arr_name}__{field}: &mut Vec<Vec<f64>>"));
                        } else {
                            parts.push(format!("{arr_name}__{field}: &mut Vec<f64>"));
                        }
                    }
                } else if p.ty == QbType::String {
                    // String array parameter (e.g. choice$(), help$())
                    let sname = rust_ident_typed(&p.name, &p.ty);
                    parts.push(format!("{sname}: &mut Vec<String>"));
                } else {
                    // Plain numeric array — check body for actual usage depth
                    // (an `spr()` param might be accessed as `spr(c, r)` = 2D)
                    let ndims = array_param_used_dims(arr_name.as_str(), _body);
                    let ty_str = nested_vec_type("f64", ndims);
                    parts.push(format!("{arr_name}: &mut {ty_str}"));
                }
            } else if p.ty == QbType::String {
                parts.push(format!("{name}: &mut String"));
            } else if let QbType::UserType(tn) = &p.ty {
                // Scalar TYPE parameter — expand to per-field &mut params
                let tn_lc = tn.to_lowercase();
                let flat = flatten_type_fields(&tn_lc, &self.type_defs.clone());
                let arr_name = rust_ident(&p.name);
                for (fname, fty) in &flat {
                    let field_rust_ty = qb_type_to_rust(fty);
                    let param_name = format!("{arr_name}__{fname}");
                    if self.numeric_params.contains(&param_name) {
                        parts.push(format!("{param_name}: &mut {field_rust_ty}"));
                    } else {
                        // FUNCTION context — pass by value
                        parts.push(format!("mut {param_name}: {field_rust_ty}"));
                    }
                }
            } else if self.numeric_params.contains(&name) {
                // SUB numeric scalar — passed by reference (QB default)
                parts.push(format!("{name}: &mut f64"));
            } else {
                // FUNCTION numeric scalar — passed by value
                parts.push(format!("mut {name}: f64"));
            }
        }
        parts.join(", ")
    }

    // ── Expression emission ───────────────────────────────────────────────────

    fn emit_expr_inner(&self, expr: &Expr) -> Result<String> {
        Ok(match expr {
            Expr::IntLit(n)   => format!("{n}.0f64"),
            Expr::FloatLit(f) => emit_f64_lit(*f),
            Expr::StrLit(s)   => {
                let escaped: String = s.chars().map(|c| {
                    if c == '"' { "\\\"".into() }
                    else if c == '\\' { "\\\\".into() }
                    else if (c as u32) > 127 { format!("\\u{{{:02X}}}", c as u32) }
                    else { c.to_string() }
                }).collect();
                format!("\"{}\"", escaped)
            }
            // A bare reference to a ZERO-ARG user FUNCTION is a call in QB:
            // `IF CheckFit = FALSE` calls CheckFit() and compares. (Read path only —
            // assignment to the function's own name is handled in emit_lvalue via
            // current_fn_name_lc → __fn_ret, so this never turns a write into a call.)
            // String-returning functions like `GetKey$` have their `$` stripped by the
            // parser, so `name` = "GetKey" and `rust_ident` yields "getkey", but
            // user_fns contains "getkey_s".  Check `rust_ident_typed` as well.
            Expr::Var(LValue::Scalar { name, ty })
                if {
                    let lower = rust_ident(name);
                    let lower_t = rust_ident_typed(name, ty);
                    (self.user_fns.contains(&lower) || self.user_fns.contains(&lower_t))
                    && (self.sub_params.get(&lower).or_else(|| self.sub_params.get(&lower_t))
                           .map_or(false, |p| p.is_empty()))
                    && self.current_fn_name_lc.as_deref() != Some(lower.as_str())
                    && self.current_fn_name_lc.as_deref() != Some(lower_t.as_str())
                    && !self.shared_names.contains(&name.to_lowercase())
                } =>
            {
                let lower_t = rust_ident_typed(name, ty);
                let fn_name = if self.user_fns.contains(&lower_t) { lower_t } else { rust_ident(name) };
                format!("{}({})", fn_name, self.rt_args())
            }
            Expr::Var(lv) => self.emit_lvalue(lv),

            Expr::BinOp { op, lhs, rhs } => {
                // String concatenation: A$ + B$ → format!("{}{}", a, b)
                if *op == BinOp::Add && (is_str_expr(lhs) || is_str_expr(rhs)) {
                    let l = self.emit_expr_inner(lhs)?;
                    let r = self.emit_expr_inner(rhs)?;
                    return Ok(format!("format!(\"{{}}{{}}\" ,{l},{r})"));
                }
                let l = self.emit_expr_inner(lhs)?;
                let r = self.emit_expr_inner(rhs)?;
                // String comparison: normalize both sides to &str to avoid
                // String vs &str ambiguity (Rust can't pick PartialOrd impl)
                let (l_cmp, r_cmp) = if matches!(op,
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge)
                    && (is_str_expr(lhs) || is_str_expr(rhs))
                {
                    let lc = if is_str_expr(lhs) && !matches!(lhs.as_ref(), Expr::StrLit(_)) {
                        format!("({l}).as_str()")
                    } else { l.clone() };
                    let rc = if is_str_expr(rhs) && !matches!(rhs.as_ref(), Expr::StrLit(_)) {
                        format!("({r}).as_str()")
                    } else { r.clone() };
                    (lc, rc)
                } else {
                    (l.clone(), r.clone())
                };
                match op {
                    BinOp::Add    => format!("({l} + {r})"),
                    BinOp::Sub    => format!("({l} - {r})"),
                    BinOp::Mul    => format!("({l} * {r})"),
                    BinOp::Div    => format!("({l} / {r})"),
                    BinOp::IntDiv => format!("qb_idiv({l}, {r})"),
                    BinOp::Pow    => format!("{l}.powf({r})"),
                    BinOp::Mod    => format!("qb_mod({l}, {r})"),
                    BinOp::Eq     => format!("qb_from_bool({l_cmp} == {r_cmp})"),
                    BinOp::Ne     => format!("qb_from_bool({l_cmp} != {r_cmp})"),
                    BinOp::Lt     => format!("qb_from_bool({l_cmp} < {r_cmp})"),
                    BinOp::Le     => format!("qb_from_bool({l_cmp} <= {r_cmp})"),
                    BinOp::Gt     => format!("qb_from_bool({l_cmp} > {r_cmp})"),
                    BinOp::Ge     => format!("qb_from_bool({l_cmp} >= {r_cmp})"),
                    BinOp::And    => format!("qb_and({l}, {r})"),
                    BinOp::Or     => format!("qb_or({l}, {r})"),
                    BinOp::Xor    => format!("qb_xor({l}, {r})"),
                    BinOp::Eqv    => format!("qb_eqv({l}, {r})"),
                    BinOp::Imp    => format!("qb_imp({l}, {r})"),
                }
            }

            Expr::UnOp { op, operand } => {
                let o = self.emit_expr_inner(operand)?;
                match op {
                    UnOp::Neg => format!("(-{o})"),
                    UnOp::Not => format!("qb_not({o})"),
                }
            }

            Expr::Call { name, args } => {
                let upper = name.to_uppercase();
                let lower = rust_ident(name); // sigil-stripped lowercase

                // RND / INKEY$ / INPUT$ / ERR / PMAP / PLAY(n) need __rt
                if upper == "PLAY" { return Ok("__rt.play_count()".into()); }
                if upper == "RND" {
                    if let Some(arg0) = args.first() {
                        let av = self.emit_expr_inner(arg0)?;
                        return Ok(format!("__rt.rnd_arg({av})"));
                    }
                    return Ok("__rt.rnd()".into());
                }
                if upper == "INKEY$" { return Ok("__rt.inkey()".into()); }
                if upper == "ERR"    { return Ok("__rt.err_code".into()); }
                if upper == "PMAP" && args.len() == 2 {
                    let a0 = self.emit_expr_inner(&args[0])?;
                    let a1 = self.emit_expr_inner(&args[1])?;
                    return Ok(format!("__rt.pmap({a0}, {a1})"));
                }
                if upper == "INPUT$" {
                    let n = args.first()
                        .map(|e| self.emit_expr_inner(e).unwrap_or_else(|_| "1.0".into()))
                        .unwrap_or_else(|| "1.0".into());
                    return Ok(format!("__rt.input_str({n})"));
                }

                // UBOUND(arr[, dim]) / LBOUND(arr[, dim])
                if upper == "UBOUND" || upper == "LBOUND" {
                    // Resolve QB array name for lower-bound lookup.
                    let arr_qb_lc: String = match args.first() {
                        // array_lower keys are sigil-free — strip before lookup
                        Some(Expr::Var(LValue::Scalar { name: n, .. }))
                        | Some(Expr::Call { name: n, args: _ }) => n
                            .trim_end_matches(['$', '%', '!', '#', '&'])
                            .to_lowercase(),
                        _ => String::new(),
                    };
                    // Optional second arg: dimension number (1-based QB).
                    let dim_idx: usize = match args.get(1) {
                        Some(Expr::IntLit(d))   => (*d as usize).saturating_sub(1),
                        Some(Expr::FloatLit(d)) => (*d as usize).saturating_sub(1),
                        _ => 0,
                    };
                    let lo = if arr_qb_lc.is_empty() { 0_i64 }
                             else { self.arr_lo(&arr_qb_lc, dim_idx) };
                    if upper == "LBOUND" {
                        return Ok(format!("{lo}.0"));
                    }
                    // UBOUND: wasted-slots → Vec has (upper+1) slots, so len-1 == upper.
                    if let Some(arr_expr) = args.first() {
                        let arr_name = match arr_expr {
                            Expr::Var(LValue::Scalar { name, ty }) => rust_ident_typed(name, ty),
                            Expr::Call { name, args: ea } if ea.is_empty() => {
                                // String arrays carry an _s suffix locally
                                if name.ends_with('$')
                                   || self.local_string_arrays.contains(&name.to_lowercase()) {
                                    rust_ident_typed(name, &QbType::String)
                                } else {
                                    rust_ident(name)
                                }
                            }
                            _ => {
                                let v = self.emit_expr_inner(arr_expr)?;
                                return Ok(format!("(({v}.len() as f64) - 1.0)"));
                            }
                        };
                        let arr_name_lc = arr_name.to_lowercase();
                        if self.shared_names.contains(&arr_name_lc) {
                            // For TYPE arrays flattened to per-field Vecs, use the first
                            // field Vec (all fields have the same length).
                            let rname = if let Some(fields) = self.typed_fields.get(arr_name.as_str()) {
                                if let Some(f0) = fields.first() {
                                    format!("{arr_name}__{f0}")
                                } else {
                                    rust_ident_typed(&arr_name, &self.shared_types.get(&arr_name_lc).cloned().unwrap_or(QbType::Single))
                                }
                            } else if let Some(ty) = self.shared_types.get(&arr_name_lc) {
                                rust_ident_typed(&arr_name, ty)
                            } else {
                                arr_name.clone()
                            };
                            return Ok(format!("((__gs.{rname}.len() as f64) - 1.0)"));
                        }
                        return Ok(format!("(({arr_name}.len() as f64) - 1.0)"));
                    }
                    return Ok("0.0".to_string()); // fallback
                }

                // Array disambiguation: shared array or local/param array.
                // Wasted-slots: raw QB index is used directly as the Vec index.
                // Use a sigil-stripped bare name for set lookups: Expr::Call stores the
                // full name-with-sigil ("Names$") while shared_names/array_names keys are
                // sigil-free ("names") — from VarDecl.name which is stripped at parse time.
                let name_bare = name.trim_end_matches(['$', '%', '!', '#', '&']).to_lowercase();
                if self.shared_names.contains(&name_bare) && self.array_names.contains(&name_bare) {
                    let idx: Vec<_> = args.iter()
                        .map(|e| self.emit_expr_inner(e).unwrap())
                        .collect();
                    let sub: String = idx.iter().map(|i| format!("[({i}) as usize]")).collect();
                    // Use typed name so string arrays get _s suffix
                    let rname = if let Some(ty) = self.shared_types.get(&name_bare) {
                        rust_ident_typed(name, ty)
                    } else {
                        lower.clone()
                    };
                    return Ok(format!("__gs.{rname}{sub}"));
                }
                if self.local_arrays.contains(&lower) || self.array_params.contains(&lower) {
                    let idx: Vec<_> = args.iter()
                        .map(|e| self.emit_expr_inner(e).unwrap())
                        .collect();
                    let sub: String = idx.iter().map(|i| format!("[({i}) as usize]")).collect();
                    return Ok(format!("{lower}{sub}"));
                }
                // Local string array without $ sigil (e.g. DIM rankStr(1 TO 10) AS STRING)
                if self.local_string_arrays.contains(&name_bare) {
                    let idx: Vec<_> = args.iter()
                        .map(|e| self.emit_expr_inner(e).unwrap())
                        .collect();
                    let sub: String = idx.iter().map(|i| format!("[({i}) as usize]")).collect();
                    let rname = rust_ident_typed(name, &QbType::String);
                    return Ok(format!("{rname}{sub}"));
                }

                // User-defined FUNCTION — prepend rt/gs args.
                // String scalar/element args are passed as &mut String.
                // When a string arg would borrow from __gs (e.g. __gs.field[i]), wrapping
                // the call in a block expression avoids the double-mutable-borrow of __gs.
                if self.user_fns.contains(&lower) {
                    let rt = self.rt_args();
                    // Expand scalar UserType args before building arg_info.
                    // e.g. `Inside(T)` where `T AS Tile` → expand to its fields, each
                    // passed BY REFERENCE (`&mut`) since QB passes TYPE params by ref
                    // and the FUNCTION may mutate them (torus Inside sets T.xc/T.yc).
                    // `expanded` items carry an optional precomputed `&mut` accessor.
                    let expanded: Vec<(Expr, Option<String>)> = args.iter().flat_map(|e| {
                        if let Expr::Var(LValue::Scalar { name: n, .. }) = e {
                            let base = rust_ident(n);
                            if let Some(type_name) = self.var_type_name.get(&base).cloned() {
                                let flat = flatten_type_fields(&type_name,
                                                               &self.type_defs.clone());
                                if !flat.is_empty() {
                                    return flat.into_iter().map(|(fname, fty)| {
                                        let field = format!("{base}__{fname}");
                                        // Compute the by-ref accessor for this field.
                                        let acc = if self.numeric_params.contains(&field) {
                                            // caller holds &mut f64 — reborrow
                                            format!("&mut *{field}")
                                        } else if self.shared_names.contains(&base) {
                                            format!("&mut __gs.{field}")
                                        } else {
                                            format!("&mut {field}")
                                        };
                                        (Expr::Var(LValue::Scalar { name: field, ty: fty }),
                                         Some(acc))
                                    }).collect::<Vec<_>>();
                                }
                            }
                        }
                        vec![(e.clone(), None)]
                    }).collect();

                    // Collect arg info: (value_str, is_str_scalar, is_whole_arr, byref_acc)
                    let arg_info: Vec<(String, bool, bool, Option<String>)> = expanded.iter().map(|(e, byref)| {
                        if byref.is_some() {
                            return (String::new(), false, false, byref.clone());
                        }
                        let v = self.emit_expr_inner(e).unwrap_or_default();
                        let is_str = is_str_expr(e) || self.is_str_expr_ctx(e);
                        let is_whole_arr = match e {
                            Expr::Var(LValue::Scalar { name: n, .. }) => {
                                let rn = rust_ident(n);
                                self.local_arrays.contains(&rn) || self.array_params.contains(&rn)
                            }
                            Expr::Call { name: n, args: ea } if ea.is_empty() => {
                                let rn = rust_ident(n);
                                self.local_arrays.contains(&rn) || self.array_params.contains(&rn)
                                    || (self.shared_names.contains(&n.to_lowercase()) && self.array_names.contains(&n.to_lowercase()))
                            }
                            _ => false,
                        };
                        (v, is_str && !is_whole_arr, is_whole_arr, None)
                    }).collect();

                    let has_str_scalar = arg_info.iter().any(|(_, is_str, _, _)| *is_str);
                    // A plain numeric arg that reads a shared field (`__gs.x`) conflicts
                    // with passing `&mut __gs` to the same call (E0503) — hoist it to a
                    // temp inside a block expression first.
                    let needs_hoist = arg_info.iter().any(|(v, is_str, whole, byref)| {
                        byref.is_none() && !*is_str && !*whole && v.contains("__gs")
                    });
                    let sep = if arg_info.is_empty() { "" } else { ", " };

                    if has_str_scalar || needs_hoist {
                        // Wrap in a Rust block expression so we can materialize temps
                        // (string and shared-field reads) before passing __gs mutably —
                        // avoids E0499/E0503 borrow conflicts.
                        let mut block_lets: Vec<String> = Vec::new();
                        let mut call_args: Vec<String> = Vec::new();
                        let mut tmp_idx = 0usize;
                        for (v, is_str_scalar, is_whole_arr, byref) in &arg_info {
                            if let Some(acc) = byref {
                                call_args.push(acc.clone());
                            } else if *is_whole_arr {
                                call_args.push(format!("&mut {v}"));
                            } else if *is_str_scalar {
                                let tmp = format!("__fn_s{tmp_idx}");
                                tmp_idx += 1;
                                block_lets.push(format!("let mut {tmp}: String = ({v}).to_string()"));
                                call_args.push(format!("&mut {tmp}"));
                            } else if v.contains("__gs") {
                                // Hoist shared-field read to a temp f64 (copy) before the call.
                                let tmp = format!("__fa{tmp_idx}");
                                tmp_idx += 1;
                                block_lets.push(format!("let {tmp} = {v}"));
                                call_args.push(tmp);
                            } else {
                                call_args.push(v.clone());
                            }
                        }
                        let lets_str = block_lets.join("; ");
                        let call_sep = if call_args.is_empty() { "" } else { ", " };
                        return Ok(format!(
                            "{{ {lets_str}; {lower}({rt}{call_sep}{}) }}",
                            call_args.join(", ")
                        ));
                    } else {
                        let a: Vec<_> = arg_info.iter().map(|(v, _, is_whole_arr, byref)| {
                            if let Some(acc) = byref { acc.clone() }
                            else if *is_whole_arr { format!("&mut {v}") }
                            else { v.clone() }
                        }).collect();
                        return Ok(format!("{lower}({rt}{sep}{})", a.join(", ")));
                    }
                }

                // Built-in function — wrap &str arguments where the fn expects &str
                let fn_name = rust_fn_name(name);

                // ── INSTR: 2-arg form → prepend default start=1.0 ────────────
                if fn_name == "qb_instr" && args.len() == 2 {
                    let s0 = self.emit_expr_inner(&args[0]).unwrap_or_default();
                    let s1 = self.emit_expr_inner(&args[1]).unwrap_or_default();
                    let a0 = if matches!(&args[0], Expr::StrLit(_)) { s0 } else { format!("&({s0})") };
                    let a1 = if matches!(&args[1], Expr::StrLit(_)) { s1 } else { format!("&({s1})") };
                    return Ok(format!("qb_instr(1.0, {a0}, {a1})"));
                }

                // ── MID$: 2-arg → None, 3-arg → Some(len) ───────────────────
                if fn_name == "qb_mid" {
                    let s0 = self.emit_expr_inner(&args[0]).unwrap_or_default();
                    let a0 = if matches!(&args[0], Expr::StrLit(_)) { s0 } else { format!("&({s0})") };
                    let a1 = self.emit_expr_inner(&args[1]).unwrap_or_default();
                    let a2 = if args.len() >= 3 {
                        let v = self.emit_expr_inner(&args[2]).unwrap_or_default();
                        format!("Some({v})")
                    } else {
                        "None".into()
                    };
                    return Ok(format!("qb_mid({a0}, {a1}, {a2})"));
                }

                // ── STRING$(n, x): x may be a char-code (f64) or a string ────
                if fn_name == "qb_string" && args.len() == 2 {
                    let a0 = self.emit_expr_inner(&args[0]).unwrap_or_default();
                    let a1 = self.emit_expr_inner(&args[1]).unwrap_or_default();
                    if is_str_expr(&args[1]) {
                        // STRING$(n, s$) → qb_string_s(n, &s)
                        let a1_ref = if matches!(&args[1], Expr::StrLit(_)) { a1 } else { format!("&({a1})") };
                        return Ok(format!("qb_string_s({a0}, {a1_ref})"));
                    } else {
                        return Ok(format!("qb_string({a0}, {a1})"));
                    }
                }

                let str_pos = str_arg_positions(&fn_name);
                let a: Vec<_> = args.iter().enumerate()
                    .map(|(i, e)| {
                        let s = self.emit_expr_inner(e).unwrap_or_else(|_| "/*err*/".into());
                        if str_pos.contains(&i) && !matches!(e, Expr::StrLit(_)) {
                            format!("&({s})")
                        } else {
                            s
                        }
                    })
                    .collect();
                format!("{fn_name}({})", a.join(", "))
            }

            Expr::Point { x, y } => {
                let x = self.emit_expr_inner(x)?;
                let y = self.emit_expr_inner(y)?;
                format!("__rt.point({x}, {y})")
            }
        })
    }
}

// ── Local variable collection ─────────────────────────────────────────────────

fn collect_locals(stmts: &[Stmt], exclude: &HashSet<String>) -> Vec<(String, QbType)> {
    let mut result: Vec<(String, QbType)> = Vec::new();
    let mut added: HashSet<String> = HashSet::new();

    fn push(name: &str, ty: &QbType,
            result: &mut Vec<(String, QbType)>,
            added:  &mut HashSet<String>,
            exclude: &HashSet<String>) {
        // Use rust_ident_typed so parser-stripped $ sigils get _s appended.
        let rname = rust_ident_typed(name, ty);
        if !exclude.contains(&rname) && !added.contains(&rname) {
            added.insert(rname.clone());
            result.push((rname, ty.clone()));
        }
    }

    /// Recursively scan an expression for scalar variable references.
    fn scan_expr(expr: &Expr,
                 result: &mut Vec<(String, QbType)>,
                 added:  &mut HashSet<String>,
                 exclude: &HashSet<String>) {
        match expr {
            Expr::Var(LValue::Scalar { name, ty }) => {
                push(name, ty, result, added, exclude);
            }
            Expr::Var(LValue::Index { indices, .. }) => {
                for e in indices { scan_expr(e, result, added, exclude); }
            }
            Expr::Var(LValue::Field { base, .. }) |
            Expr::Var(LValue::FieldIndex { base, .. }) => {
                scan_expr(&Expr::Var(*base.clone()), result, added, exclude);
            }
            Expr::BinOp { lhs, rhs, .. } => {
                scan_expr(lhs, result, added, exclude);
                scan_expr(rhs, result, added, exclude);
            }
            Expr::UnOp { operand, .. } => scan_expr(operand, result, added, exclude),
            Expr::Call { args, .. } => {
                for a in args { scan_expr(a, result, added, exclude); }
            }
            Expr::Point { x, y } => {
                scan_expr(x, result, added, exclude);
                scan_expr(y, result, added, exclude);
            }
            Expr::IntLit(_) | Expr::FloatLit(_) | Expr::StrLit(_) => {}
        }
    }

    /// Pre-mark FOR-loop counter variables as already-added so collect_locals
    /// won't re-declare them (emit_stmt handles their declaration inline).

    fn visit(stmts: &[Stmt], result: &mut Vec<(String, QbType)>,
             added: &mut HashSet<String>, exclude: &HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::Let { var, expr } => {
                    match var {
                        LValue::Scalar { name, ty } => {
                            push(name, ty, result, added, exclude);
                        }
                        LValue::Field { base, field } => {
                            // Flattened TYPE field: arr(i).Field → arr__field: Vec<f64>
                            if let LValue::Index { name, .. } = base.as_ref() {
                                let flat = format!("{}__{}", rust_ident(name), field.to_lowercase());
                                if !exclude.contains(&flat) && !added.contains(&flat) {
                                    added.insert(flat.clone());
                                    result.push((flat, QbType::UserType("vec_f64".into())));
                                }
                            }
                        }
                        LValue::FieldIndex { .. } => {}
                        LValue::Index { .. } => {}
                    }
                    scan_expr(expr, result, added, exclude);
                }
                Stmt::Dim(decl) | Stmt::ReDim(decl) => {
                    // Scan dimension expressions (e.g. DIM SHARED LBan&(x) → need x declared)
                    for e in &decl.dims { scan_expr(e, result, added, exclude); }
                    // REDIM local array declarations are emitted inline by emit_redim()
                    // (it tracks its own `redim_declared` set). Don't duplicate here.
                    // But scalar DIM (e.g. from FIELD) must be hoisted as locals.
                    if decl.dims.is_empty() && !decl.shared {
                        push(&decl.name, &decl.ty, result, added, exclude);
                    }
                }
                Stmt::For { var, from, to, step, body } => {
                    // Declare FOR counter at function scope (not inline) so it
                    // remains accessible when GOSUB blocks are inlined.
                    push(var, &QbType::Single, result, added, exclude);
                    scan_expr(from, result, added, exclude);
                    scan_expr(to,   result, added, exclude);
                    if let Some(s) = step { scan_expr(s, result, added, exclude); }
                    visit(body, result, added, exclude);
                }
                Stmt::Input { vars, .. } => {
                    for lv in vars {
                        if let LValue::Scalar { name, ty } = lv {
                            push(name, ty, result, added, exclude);
                        }
                    }
                }
                Stmt::Read(vars) => {
                    for lv in vars {
                        if let LValue::Scalar { name, ty } = lv {
                            push(name, ty, result, added, exclude);
                        }
                    }
                }
                Stmt::If { cond, then_body, elseif_branches, else_body } => {
                    scan_expr(cond, result, added, exclude);
                    visit(then_body, result, added, exclude);
                    for (ec, b) in elseif_branches {
                        scan_expr(ec, result, added, exclude);
                        visit(b, result, added, exclude);
                    }
                    if let Some(b) = else_body { visit(b, result, added, exclude); }
                }
                Stmt::While { cond, body } => {
                    scan_expr(cond, result, added, exclude);
                    visit(body, result, added, exclude);
                }
                Stmt::Do { kind, body } => {
                    match kind {
                        DoKind::WhilePre(e) | DoKind::UntilPre(e) |
                        DoKind::WhilePost(e) | DoKind::UntilPost(e) =>
                            scan_expr(e, result, added, exclude),
                        DoKind::Infinite => {}
                    }
                    visit(body, result, added, exclude);
                }
                Stmt::Select { expr, cases, default } => {
                    scan_expr(expr, result, added, exclude);
                    for c in cases {
                        for cond in &c.conditions {
                            match cond {
                                CaseCond::Value(e) | CaseCond::Is(_, e) =>
                                    scan_expr(e, result, added, exclude),
                                CaseCond::Range(a, b) => {
                                    scan_expr(a, result, added, exclude);
                                    scan_expr(b, result, added, exclude);
                                }
                            }
                        }
                        visit(&c.body, result, added, exclude);
                    }
                    if let Some(b) = default { visit(b, result, added, exclude); }
                }
                Stmt::Call { args, .. } => {
                    for arg in args { scan_expr(arg, result, added, exclude); }
                }
                Stmt::Print { args, .. } => {
                    for a in args {
                        match a {
                            PrintArg::Expr(e) | PrintArg::Tab(e) | PrintArg::Spc(e) =>
                                scan_expr(e, result, added, exclude),
                            PrintArg::Comma => {}
                        }
                    }
                }
                Stmt::PrintUsing { fmt, args, .. } => {
                    scan_expr(fmt, result, added, exclude);
                    for a in args { scan_expr(a, result, added, exclude); }
                }
                // Graphics statements — scan all sub-expressions for vars
                Stmt::Line { x1, y1, x2, y2, color, .. } => {
                    if let Some(e) = x1 { scan_expr(e, result, added, exclude); }
                    if let Some(e) = y1 { scan_expr(e, result, added, exclude); }
                    scan_expr(x2, result, added, exclude);
                    scan_expr(y2, result, added, exclude);
                    if let Some(c) = color { scan_expr(c, result, added, exclude); }
                }
                Stmt::View { x1, y1, x2, y2, fill, border } => {
                    for e in [x1, y1, x2, y2] { scan_expr(e, result, added, exclude); }
                    if let Some(f) = fill   { scan_expr(f, result, added, exclude); }
                    if let Some(b) = border { scan_expr(b, result, added, exclude); }
                }
                Stmt::Window { x1, y1, x2, y2, .. } => {
                    for e in [x1, y1, x2, y2] { scan_expr(e, result, added, exclude); }
                }
                Stmt::PaletteUsing(e) => { scan_expr(e, result, added, exclude); }
                Stmt::Circle { x, y, r, color, .. } => {
                    for e in [x, y, r] { scan_expr(e, result, added, exclude); }
                    if let Some(c) = color { scan_expr(c, result, added, exclude); }
                }
                Stmt::Paint { x, y, fill, border, .. } => {
                    for e in [x, y, fill] { scan_expr(e, result, added, exclude); }
                    if let Some(b) = border { scan_expr(b, result, added, exclude); }
                }
                Stmt::Pset { x, y, color, .. } => {
                    scan_expr(x, result, added, exclude);
                    scan_expr(y, result, added, exclude);
                    if let Some(c) = color { scan_expr(c, result, added, exclude); }
                }
                Stmt::Locate { row, col, cursor } => {
                    if let Some(e) = row { scan_expr(e, result, added, exclude); }
                    if let Some(e) = col  { scan_expr(e, result, added, exclude); }
                    if let Some(e) = cursor { scan_expr(e, result, added, exclude); }
                }
                Stmt::Color { fg, bg } => {
                    if let Some(e) = fg { scan_expr(e, result, added, exclude); }
                    if let Some(e) = bg { scan_expr(e, result, added, exclude); }
                }
                Stmt::Screen(e) | Stmt::Play(e) | Stmt::Randomize(Some(e)) => {
                    scan_expr(e, result, added, exclude);
                }
                Stmt::Sound { freq, duration } => {
                    scan_expr(freq, result, added, exclude);
                    scan_expr(duration, result, added, exclude);
                }
                Stmt::Poke { addr, val } => {
                    scan_expr(addr, result, added, exclude);
                    scan_expr(val, result, added, exclude);
                }
                Stmt::Swap(a, b) => {
                    if let LValue::Scalar { name, ty } = a { push(name, ty, result, added, exclude); }
                    if let LValue::Scalar { name, ty } = b { push(name, ty, result, added, exclude); }
                }
                Stmt::Palette { attr, color64 } => {
                    scan_expr(attr, result, added, exclude);
                    scan_expr(color64, result, added, exclude);
                }
                Stmt::PutSprite { x, y, .. } => {
                    scan_expr(x, result, added, exclude);
                    scan_expr(y, result, added, exclude);
                }
                Stmt::GetSprite { x1, y1, x2, y2, .. } => {
                    scan_expr(x1, result, added, exclude);
                    scan_expr(y1, result, added, exclude);
                    scan_expr(x2, result, added, exclude);
                    scan_expr(y2, result, added, exclude);
                }
                Stmt::Block(inner) => visit(inner, result, added, exclude),
                // FIELD variables must be pre-declared as String locals
                Stmt::Field { fields, .. } => {
                    for (_len_expr, var) in fields {
                        if let LValue::Scalar { name, .. } = var {
                            push(name, &QbType::String, result, added, exclude);
                        }
                    }
                }
                // LSet/RSet assign to a variable — ensure it's declared
                Stmt::LSet { var, expr } | Stmt::RSet { var, expr } => {
                    if let LValue::Scalar { name, ty } = var {
                        push(name, ty, result, added, exclude);
                    }
                    scan_expr(expr, result, added, exclude);
                }
                Stmt::InputFile { vars, file_num } => {
                    scan_expr(file_num, result, added, exclude);
                    for lv in vars {
                        if let LValue::Scalar { name, ty } = lv {
                            push(name, ty, result, added, exclude);
                        }
                    }
                }
                Stmt::LineInputFile { var, file_num } => {
                    scan_expr(file_num, result, added, exclude);
                    if let LValue::Scalar { name, ty } = var {
                        push(name, ty, result, added, exclude);
                    }
                }
                Stmt::PrintFile { file_num, args, .. } => {
                    scan_expr(file_num, result, added, exclude);
                    for a in args {
                        match a {
                            PrintArg::Expr(e) | PrintArg::Tab(e) | PrintArg::Spc(e) =>
                                scan_expr(e, result, added, exclude),
                            PrintArg::Comma => {}
                        }
                    }
                }
                Stmt::WriteFile { file_num, args } => {
                    scan_expr(file_num, result, added, exclude);
                    for a in args { scan_expr(a, result, added, exclude); }
                }
                Stmt::FileGet { file_num, record, record_var } |
                Stmt::FilePut { file_num, record, record_var } => {
                    scan_expr(file_num, result, added, exclude);
                    if let Some(r) = record { scan_expr(r, result, added, exclude); }
                    // Scan any index expressions inside the record variable so a
                    // loop counter used only there (e.g. PUT #1, n, ARR(k)) is seen.
                    if let Some(rv) = record_var {
                        let mut cur = rv;
                        loop {
                            match cur {
                                LValue::Index { indices, .. } => {
                                    for e in indices { scan_expr(e, result, added, exclude); }
                                    break;
                                }
                                LValue::Field { base, .. } |
                                LValue::FieldIndex { base, .. } => cur = base,
                                LValue::Scalar { .. } => break,
                            }
                        }
                    }
                }
                Stmt::Open { path, file_num, rec_len, .. } => {
                    scan_expr(path, result, added, exclude);
                    scan_expr(file_num, result, added, exclude);
                    if let Some(r) = rec_len { scan_expr(r, result, added, exclude); }
                }
                Stmt::Close { file_nums } => {
                    for e in file_nums { scan_expr(e, result, added, exclude); }
                }
                _ => {}
            }
        }
    }

    /// Pre-mark CONST names so collect_locals won't try to declare them as locals.
    fn pre_mark_consts(stmts: &[Stmt], added: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::Const { name, .. } => {
                    added.insert(rust_ident(name));
                }
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    pre_mark_consts(then_body, added);
                    for (_, b) in elseif_branches { pre_mark_consts(b, added); }
                    if let Some(b) = else_body { pre_mark_consts(b, added); }
                }
                Stmt::While { body, .. } | Stmt::Do { body, .. } => pre_mark_consts(body, added),
                Stmt::For { body, .. } => pre_mark_consts(body, added),
                Stmt::Select { cases, default, .. } => {
                    for c in cases { pre_mark_consts(&c.body, added); }
                    if let Some(b) = default { pre_mark_consts(b, added); }
                }
                Stmt::Block(inner) => pre_mark_consts(inner, added),
                _ => {}
            }
        }
    }

    // FOR vars are now pushed in visit() itself (not pre-marked/excluded)
    // so they appear in result and get declared at function top scope.
    pre_mark_consts(stmts, &mut added);
    visit(stmts, &mut result, &mut added, exclude);
    result
}

/// Collect names of arrays DIM'd locally inside a function body (not shared).
fn collect_local_array_names(stmts: &[Stmt]) -> HashSet<String> {
    let mut names = HashSet::new();
    fn visit(stmts: &[Stmt], names: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::Dim(d) | Stmt::ReDim(d) if !d.dims.is_empty() => {
                    // Insert the rust_ident_typed name (the name used in emit_lvalue/lift_expr).
                    // For string arrays (e.g. choice$→choice_s), don't insert the bare lowercase
                    // name to avoid colliding with a same-named numeric scalar (e.g. choice: f64).
                    let typed_name = rust_ident_typed(&d.name, &d.ty);
                    names.insert(typed_name);
                    // Also insert bare lowercase for non-string arrays (safe: no _s suffix collision)
                    if d.ty != QbType::String {
                        names.insert(d.name.to_lowercase());
                    }
                }
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    visit(then_body, names);
                    for (_, b) in elseif_branches { visit(b, names); }
                    if let Some(b) = else_body { visit(b, names); }
                }
                Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } => {
                    visit(body, names);
                }
                Stmt::Select { cases, default, .. } => {
                    for c in cases { visit(&c.body, names); }
                    if let Some(b) = default { visit(b, names); }
                }
                Stmt::Block(inner) => visit(inner, names),
                _ => {}
            }
        }
    }
    visit(stmts, &mut names);
    names
}

/// Collect bare-lowercase names of all explicit scalar DIM declarations
/// (non-shared, non-array) in the given statement list.  Used to detect when
/// a local `DIM B AS INTEGER` shadows a DIM SHARED string `B$` that shares
/// the same base name after sigil-stripping.
fn collect_local_dim_names(stmts: &[Stmt]) -> HashSet<String> {
    let mut names = HashSet::new();
    fn visit(stmts: &[Stmt], names: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::Dim(d) if d.dims.is_empty() && !d.shared => {
                    names.insert(d.name.to_lowercase());
                }
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    visit(then_body, names);
                    for (_, b) in elseif_branches { visit(b, names); }
                    if let Some(b) = else_body { visit(b, names); }
                }
                Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } => {
                    visit(body, names);
                }
                Stmt::Select { cases, default, .. } => {
                    for c in cases { visit(&c.body, names); }
                    if let Some(b) = default { visit(b, names); }
                }
                Stmt::Block(inner) => visit(inner, names),
                _ => {}
            }
        }
    }
    visit(stmts, &mut names);
    names
}

/// Scan a SUB body to find the maximum index depth used for a given array param.
/// Array params use a 1D placeholder in VarDecl regardless of actual usage,
/// e.g. `spr()` might be accessed as `spr(c, r)` (2D) inside the body.
fn array_param_used_dims(name: &str, stmts: &[Stmt]) -> usize {
    let name_lc = name.to_lowercase();
    let mut max = 1usize;
    fn visit_stmts(n: &str, stmts: &[Stmt], m: &mut usize) {
        for s in stmts { visit_stmt(n, s, m); }
    }
    fn visit_stmt(n: &str, stmt: &Stmt, m: &mut usize) {
        match stmt {
            Stmt::Let { var, expr } => { visit_lval(n, var, m); visit_expr(n, expr, m); }
            Stmt::If { cond, then_body, elseif_branches, else_body } => {
                visit_expr(n, cond, m);
                visit_stmts(n, then_body, m);
                for (e, b) in elseif_branches { visit_expr(n, e, m); visit_stmts(n, b, m); }
                if let Some(b) = else_body { visit_stmts(n, b, m); }
            }
            Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } => {
                visit_stmts(n, body, m);
            }
            Stmt::Select { cases, default, .. } => {
                for c in cases { visit_stmts(n, &c.body, m); }
                if let Some(b) = default { visit_stmts(n, b, m); }
            }
            Stmt::Block(inner) => visit_stmts(n, inner, m),
            Stmt::Call { args, .. } => { for a in args { visit_expr(n, a, m); } }
            Stmt::Print { args, .. } => {
                for a in args {
                    if let crate::parser::PrintArg::Expr(e) = a { visit_expr(n, e, m); }
                }
            }
            _ => {}
        }
    }
    fn visit_lval(n: &str, lv: &LValue, m: &mut usize) {
        if let LValue::Index { name: ln, indices, .. } = lv {
            if ln.to_lowercase() == n { *m = (*m).max(indices.len()); }
        }
    }
    fn visit_expr(n: &str, e: &Expr, m: &mut usize) {
        match e {
            Expr::Var(lv) => visit_lval(n, lv, m),
            Expr::BinOp { lhs, rhs, .. } => { visit_expr(n, lhs, m); visit_expr(n, rhs, m); }
            Expr::UnOp { operand, .. } => visit_expr(n, operand, m),
            Expr::Call { name, args } => {
                // Array access like spr(c, r) is often parsed as Expr::Call
                if name.to_lowercase() == n && !args.is_empty() {
                    *m = (*m).max(args.len());
                }
                for a in args { visit_expr(n, a, m); }
            }
            _ => {}
        }
    }
    visit_stmts(&name_lc, stmts, &mut max);
    max
}

/// Collect bare-lowercase names of non-shared local DIM'd string arrays.
/// Used by emit_lvalue to emit `name_s[...]` for arrays declared `DIM name(...) AS STRING`
/// even when accessed without the `$` sigil (so the parser records type as Single).
fn collect_local_string_arrays(stmts: &[Stmt]) -> HashSet<String> {
    let mut names = HashSet::new();
    fn visit(stmts: &[Stmt], names: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::Dim(d) if !d.dims.is_empty() && !d.shared && d.ty == QbType::String => {
                    names.insert(d.name.to_lowercase());
                }
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    visit(then_body, names);
                    for (_, b) in elseif_branches { visit(b, names); }
                    if let Some(b) = else_body { visit(b, names); }
                }
                Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } => {
                    visit(body, names);
                }
                Stmt::Select { cases, default, .. } => {
                    for c in cases { visit(&c.body, names); }
                    if let Some(b) = default { visit(b, names); }
                }
                Stmt::Block(inner) => visit(inner, names),
                _ => {}
            }
        }
    }
    visit(stmts, &mut names);
    names
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Format an f64 value as an unambiguous Rust f64 literal (e.g. `42.0f64`, `3.14f64`).
/// Using the `f64` type suffix avoids ambiguity when the literal is the receiver of a
/// method call (e.g. `2.0f64.powf(10.0f64)`) — bare `2.0` would make rustc error with
/// "can't call method `powf` on ambiguous numeric type `{float}`".
fn emit_f64_lit(f: f64) -> String {
    let s = format!("{f}");
    if s.contains('.') || s.contains('e') || s.contains('E') {
        format!("{s}f64")   // e.g. "3.14f64", "1.0f64"
    } else {
        format!("{s}.0f64") // e.g. "2.0f64" (float Display dropped the .0)
    }
}

fn rust_ident(name: &str) -> String {
    // String variables get an `_s` suffix so that QB variables `A` and `A$`
    // map to distinct Rust names (`a` and `a_s`) instead of both becoming `a`.
    // This covers names that still carry their `$` sigil (e.g. from CONST lists).
    let has_dollar = name.ends_with('$');
    let stripped = name.trim_end_matches(['$', '#', '!', '%', '&']);
    // Numeric labels (e.g. GOSUB 1780) must be prefixed so they're valid Rust identifiers.
    if stripped.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        let base = format!("lbl_{stripped}");
        return if has_dollar { format!("{base}_s") } else { base };
    }
    let base = match stripped.to_lowercase().as_str() {
        "loop" | "move" | "type" | "use" | "fn" | "let" | "return" |
        "match" | "mod"  | "ref"  | "in"  | "as" | "box" |
        "true"  | "false" | "self" | "super" | "crate" | "where" |
        "impl"  | "trait" | "enum" | "struct" | "pub"  | "mut" |
        "if"    | "else"  | "while" | "for"  | "break" | "continue" |
        // Not Rust keywords but conflict with std items used in emitted code
        "format" | "write" | "print" | "panic" | "assert" | "vec" | "string" => {
            format!("qb_{}", stripped.to_lowercase())
        }
        _ => stripped.to_lowercase(),
    };
    if has_dollar { format!("{base}_s") } else { base }
}

/// Like `rust_ident` but also applies the `_s` suffix when the parser has
/// already stripped the `$` sigil from a String variable.  Always use this
/// when you have both the QB name *and* the QbType (i.e. from an LValue or
/// VarDecl).  Use plain `rust_ident` only when operating on raw QB source
/// tokens that may still carry their sigil.
fn rust_ident_typed(name: &str, ty: &QbType) -> String {
    let base = rust_ident(name);
    // If the type is String but the name no longer ends with `$` (parser stripped
    // it), add the `_s` suffix so it doesn't collide with a numeric `name`.
    if *ty == QbType::String && !name.ends_with('$') {
        format!("{base}_s")
    } else {
        base // rust_ident already added _s when name had $
    }
}

fn rust_fn_name(name: &str) -> String {
    match name.to_uppercase().as_str() {
        "INT"     => "qb_int".into(),
        "FIX"     => "qb_fix".into(),
        "ABS"     => "qb_abs".into(),
        "SQR"     => "qb_sqr".into(),
        "SGN"     => "qb_sgn".into(),
        "SIN"     => "qb_sin".into(),
        "COS"     => "qb_cos".into(),
        "TAN"     => "qb_tan".into(),
        "ATN"     => "qb_atn".into(),
        "EXP"     => "qb_exp".into(),
        "LOG"     => "qb_log".into(),
        "CINT"    => "qb_cint".into(),
        "CLNG"    => "qb_cint".into(),
        "CSNG"    => "qb_csng".into(),
        "CDBL"    => "qb_cdbl".into(),
        "LEN"     => "qb_len".into(),
        "LEFT$"   => "qb_left".into(),
        "RIGHT$"  => "qb_right".into(),
        "MID$"    => "qb_mid".into(),
        "UCASE$"  => "qb_ucase".into(),
        "LCASE$"  => "qb_lcase".into(),
        "LTRIM$"  => "qb_ltrim".into(),
        "RTRIM$"  => "qb_rtrim".into(),
        "STR$"    => "qb_str_fn".into(),
        "VAL"     => "qb_val".into(),
        "CHR$"    => "qb_chr".into(),
        "ASC"     => "qb_asc".into(),
        "INSTR"   => "qb_instr".into(),
        "SPACE$"  => "qb_space".into(),
        "STRING$" => "qb_string".into(),
        "HEX$"    => "qb_hex".into(),
        "OCT$"    => "qb_oct".into(),
        "TIMER"   => "qb_timer".into(),
        "PEEK"    => "__rt.qb_peek".into(),
        "ENVIRON$"=> "qb_environ".into(),
        // Binary type-conversion functions
        "MKD"     => "MKD".into(),
        "MKS"     => "MKS".into(),
        "MKI"     => "MKI".into(),
        "MKL"     => "MKL".into(),
        "CVD"     => "CVD".into(),
        "CVS"     => "CVS".into(),
        "CVI"     => "CVI".into(),
        "CVL"     => "CVL".into(),
        // File I/O built-ins
        "EOF"     => "qb_eof_fn".into(),
        "LOF"     => "qb_lof_fn".into(),
        // Error handling
        "ERR"     => "__rt.err_code".into(),  // emitted as a field access, not a fn call
        other     => rust_ident(other),
    }
}

fn qb_type_to_rust(ty: &QbType) -> &'static str {
    match ty {
        QbType::Integer     => "f64",
        QbType::Single      => "f64",
        QbType::Double      => "f64",
        QbType::String      => "String",
        QbType::UserType(_) => "f64",
    }
}

/// Rust type for an N-dimensional array of `elem`: `Vec<Vec<...<elem>...>>`.
/// `ndims` 1 → `Vec<elem>`, 2 → `Vec<Vec<elem>>`, 3 → `Vec<Vec<Vec<elem>>>`.
fn nested_vec_type(elem: &str, ndims: usize) -> String {
    let n = ndims.max(1);
    format!("{}{}{}", "Vec<".repeat(n), elem, ">".repeat(n))
}

/// Rust initializer for an N-dimensional array filled with `default_val`.
/// `allocs` holds the per-dimension lengths, outermost first:
/// `[a0, a1, a2]` → `vec![vec![vec![D; a2]; a1]; a0]`.
fn nested_vec_init(default_val: &str, allocs: &[String]) -> String {
    if allocs.is_empty() {
        return format!("vec![{default_val}]");
    }
    // Build inside-out: innermost element is default_val, wrap with each dim.
    let mut expr = default_val.to_string();
    for a in allocs.iter().rev() {
        expr = format!("vec![{expr}; {a}]");
    }
    expr
}

// ── String-expression detector ────────────────────────────────────────────────

/// Returns true if `expr` statically evaluates to a QB String value.
fn is_str_expr(expr: &Expr) -> bool {
    match expr {
        Expr::StrLit(_) => true,
        Expr::Var(LValue::Scalar { ty: QbType::String, .. }) => true,
        // String array element access
        Expr::Var(LValue::Index { ty: QbType::String, .. }) => true,
        // TYPE field access — can't know type without type_defs; check name for $ sigil
        Expr::Var(LValue::Field { field, .. }) => field.ends_with('$'),
        Expr::BinOp { op: BinOp::Add, lhs, rhs } => is_str_expr(lhs) || is_str_expr(rhs),
        Expr::Call { name, .. } => {
            // String-returning built-ins or any user name ending with $
            // (covers string array element access like help$(i) and string functions)
            name.ends_with('$') || matches!(
                name.to_uppercase().as_str(),
                "LEFT$" | "RIGHT$" | "MID$" | "UCASE$" | "LCASE$" |
                "LTRIM$" | "RTRIM$" | "STR$" | "CHR$" | "HEX$" |
                "OCT$" | "STRING$" | "SPACE$" | "ENVIRON$" | "INKEY$" |
                "MKD" | "MKS" | "MKI" | "MKL"
            )
        }
        _ => false,
    }
}


// ── &str argument positions for built-in functions ────────────────────────────

/// Returns which zero-based argument positions of `fn_name` expect `&str`.
fn str_arg_positions(fn_name: &str) -> &'static [usize] {
    match fn_name {
        "qb_len" | "qb_left" | "qb_right" | "qb_mid" |
        "qb_ucase" | "qb_lcase" | "qb_ltrim" | "qb_rtrim" |
        "qb_val" | "qb_asc" | "qb_environ" |
        "CVD" | "CVS" | "CVI" | "CVL" => &[0],
        "qb_instr" => &[1, 2],
        _ => &[],
    }
}

// ── REDIM name collector ──────────────────────────────────────────────────────

/// Collect the rust_ident_typed names of all locally REDIM'd arrays in a body.
/// These are declared inline by emit_redim(), so emit_locals must exclude them.
fn collect_redim_names(stmts: &[Stmt]) -> HashSet<String> {
    let mut out = HashSet::new();
    fn visit(stmts: &[Stmt], out: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::ReDim(d) if !d.dims.is_empty() && !d.shared => {
                    out.insert(rust_ident_typed(&d.name, &d.ty));
                }
                Stmt::Block(inner) => visit(inner, out),
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    visit(then_body, out);
                    for (_, b) in elseif_branches { visit(b, out); }
                    if let Some(b) = else_body { visit(b, out); }
                }
                Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } => {
                    visit(body, out);
                }
                Stmt::Select { cases, default, .. } => {
                    for c in cases { visit(&c.body, out); }
                    if let Some(b) = default { visit(b, out); }
                }
                _ => {}
            }
        }
    }
    visit(stmts, &mut out);
    out
}

// ── TYPE variable name collector ─────────────────────────────────────────────

/// Walk all DIM/REDIM statements and record var_lower → type_name_lower
/// for any `DIM x [(...)] AS UserTypeName` declarations.
fn collect_var_type_names(prog: &AnalyzedProgram, out: &mut HashMap<String, String>) {
    fn visit_stmt(stmt: &Stmt, out: &mut HashMap<String, String>) {
        match stmt {
            Stmt::Dim(d) | Stmt::ReDim(d) => {
                if let QbType::UserType(ty) = &d.ty {
                    out.insert(rust_ident(&d.name), ty.to_lowercase());
                }
            }
            Stmt::Block(inner) => { for s in inner { visit_stmt(s, out); } }
            Stmt::If { then_body, elseif_branches, else_body, .. } => {
                for s in then_body { visit_stmt(s, out); }
                for (_, b) in elseif_branches { for s in b { visit_stmt(s, out); } }
                if let Some(b) = else_body { for s in b { visit_stmt(s, out); } }
            }
            Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } => {
                for s in body { visit_stmt(s, out); }
            }
            Stmt::Select { cases, default, .. } => {
                for c in cases { for s in &c.body { visit_stmt(s, out); } }
                if let Some(b) = default { for s in b { visit_stmt(s, out); } }
            }
            _ => {}
        }
    }
    for s in &prog.main_body { visit_stmt(s, out); }
    for sub in &prog.subs {
        for s in &sub.body { visit_stmt(s, out); }
        // Also record scalar UserType params so call-site expansion can detect them
        for p in &sub.params {
            if p.dims.is_empty() {
                if let QbType::UserType(ty) = &p.ty {
                    out.insert(rust_ident(&p.name), ty.to_lowercase());
                }
            }
        }
    }
    for f in &prog.functions {
        for s in &f.body { visit_stmt(s, out); }
        for p in &f.params {
            if p.dims.is_empty() {
                if let QbType::UserType(ty) = &p.ty {
                    out.insert(rust_ident(&p.name), ty.to_lowercase());
                }
            }
        }
    }
}

// ── Typed array field collector ───────────────────────────────────────────────

/// Scan every SUB, FUNCTION, and main-body for `arr(i).Field` accesses and
/// build a map from lowercased base-array name → ordered list of field names.
/// Also records the max number of indices seen for each typed array into `dims`.
/// This drives both `emit_params` (typed array expansion) and `emit_dim`.
fn collect_typed_array_fields(prog: &AnalyzedProgram)
    -> (HashMap<String, Vec<String>>, HashMap<String, usize>)
{
    let mut map:  HashMap<String, Vec<String>> = HashMap::new();
    let mut dims: HashMap<String, usize>       = HashMap::new();

    fn record(base: &str, field: &str, nidx: usize,
              map: &mut HashMap<String, Vec<String>>,
              dims: &mut HashMap<String, usize>)
    {
        let key = base.to_lowercase();
        let entry = map.entry(key.clone()).or_default();
        let fl = field.to_lowercase();
        if !entry.contains(&fl) { entry.push(fl); }
        let d = dims.entry(key).or_insert(0);
        if nidx > *d { *d = nidx; }
    }

    fn visit_lv(lv: &LValue,
                map:  &mut HashMap<String, Vec<String>>,
                dims: &mut HashMap<String, usize>)
    {
        match lv {
            LValue::Field { base, field } => {
                if let LValue::Index { name, indices, .. } = base.as_ref() {
                    record(name, field, indices.len(), map, dims);
                    for e in indices { visit_expr(e, map, dims); }
                }
                visit_lv(base, map, dims);
            }
            LValue::FieldIndex { base, indices, .. } => {
                for e in indices { visit_expr(e, map, dims); }
                visit_lv(base, map, dims);
            }
            LValue::Index { indices, .. } => {
                for e in indices { visit_expr(e, map, dims); }
            }
            LValue::Scalar { .. } => {}
        }
    }

    fn visit_expr(expr: &Expr,
                  map:  &mut HashMap<String, Vec<String>>,
                  dims: &mut HashMap<String, usize>)
    {
        match expr {
            Expr::Var(lv) => visit_lv(lv, map, dims),
            Expr::BinOp { lhs, rhs, .. } => {
                visit_expr(lhs, map, dims); visit_expr(rhs, map, dims);
            }
            Expr::UnOp { operand, .. } => visit_expr(operand, map, dims),
            Expr::Call { args, .. } => { for a in args { visit_expr(a, map, dims); } }
            Expr::Point { x, y } => {
                visit_expr(x, map, dims); visit_expr(y, map, dims);
            }
            Expr::IntLit(_) | Expr::FloatLit(_) | Expr::StrLit(_) => {}
        }
    }

    fn visit_stmts(stmts: &[Stmt],
                   map:   &mut HashMap<String, Vec<String>>,
                   dims:  &mut HashMap<String, usize>)
    {
        for s in stmts { visit_stmt(s, map, dims); }
    }

    fn visit_stmt(stmt: &Stmt,
                  map:  &mut HashMap<String, Vec<String>>,
                  dims: &mut HashMap<String, usize>)
    {
        match stmt {
            Stmt::Let { var, expr } => {
                visit_lv(var, map, dims); visit_expr(expr, map, dims);
            }
            Stmt::If { cond, then_body, elseif_branches, else_body } => {
                visit_expr(cond, map, dims);
                visit_stmts(then_body, map, dims);
                for (e, b) in elseif_branches {
                    visit_expr(e, map, dims); visit_stmts(b, map, dims);
                }
                if let Some(b) = else_body { visit_stmts(b, map, dims); }
            }
            Stmt::For { from, to, step, body, .. } => {
                visit_expr(from, map, dims); visit_expr(to, map, dims);
                if let Some(s) = step { visit_expr(s, map, dims); }
                visit_stmts(body, map, dims);
            }
            Stmt::While { cond, body } => {
                visit_expr(cond, map, dims); visit_stmts(body, map, dims);
            }
            Stmt::Do { kind, body } => {
                match kind {
                    DoKind::WhilePre(e) | DoKind::UntilPre(e) |
                    DoKind::WhilePost(e) | DoKind::UntilPost(e) => visit_expr(e, map, dims),
                    DoKind::Infinite => {}
                }
                visit_stmts(body, map, dims);
            }
            Stmt::Select { expr, cases, default } => {
                visit_expr(expr, map, dims);
                for c in cases { visit_stmts(&c.body, map, dims); }
                if let Some(b) = default { visit_stmts(b, map, dims); }
            }
            Stmt::Call { args, .. } | Stmt::Data(args) => {
                for a in args { visit_expr(a, map, dims); }
            }
            Stmt::Print { args, .. } => {
                for a in args {
                    match a {
                        PrintArg::Expr(e) | PrintArg::Tab(e) | PrintArg::Spc(e) =>
                            visit_expr(e, map, dims),
                        PrintArg::Comma => {}
                    }
                }
            }
            Stmt::PrintUsing { fmt, args, .. } => {
                visit_expr(fmt, map, dims);
                for a in args { visit_expr(a, map, dims); }
            }
            Stmt::Circle { x, y, r, color, .. } => {
                visit_expr(x, map, dims); visit_expr(y, map, dims);
                visit_expr(r, map, dims);
                if let Some(c) = color { visit_expr(c, map, dims); }
            }
            Stmt::Line { x1, y1, x2, y2, color, .. } => {
                if let Some(e) = x1 { visit_expr(e, map, dims); }
                if let Some(e) = y1 { visit_expr(e, map, dims); }
                visit_expr(x2, map, dims); visit_expr(y2, map, dims);
                if let Some(c) = color { visit_expr(c, map, dims); }
            }
            Stmt::View { x1, y1, x2, y2, fill, border } => {
                visit_expr(x1, map, dims); visit_expr(y1, map, dims);
                visit_expr(x2, map, dims); visit_expr(y2, map, dims);
                if let Some(f) = fill   { visit_expr(f, map, dims); }
                if let Some(b) = border { visit_expr(b, map, dims); }
            }
            Stmt::Window { x1, y1, x2, y2, .. } => {
                visit_expr(x1, map, dims); visit_expr(y1, map, dims);
                visit_expr(x2, map, dims); visit_expr(y2, map, dims);
            }
            Stmt::PaletteUsing(e) => { visit_expr(e, map, dims); }
            Stmt::Pset { x, y, color, .. } => {
                visit_expr(x, map, dims); visit_expr(y, map, dims);
                if let Some(c) = color { visit_expr(c, map, dims); }
            }
            Stmt::Paint { x, y, fill, border, .. } => {
                visit_expr(x, map, dims); visit_expr(y, map, dims);
                visit_expr(fill, map, dims);
                if let Some(b) = border { visit_expr(b, map, dims); }
            }
            Stmt::Block(inner) => visit_stmts(inner, map, dims),
            _ => {}
        }
    }

    for sub in &prog.subs      { visit_stmts(&sub.body, &mut map, &mut dims); }
    for f   in &prog.functions { visit_stmts(&f.body,   &mut map, &mut dims); }
    visit_stmts(&prog.main_body, &mut map, &mut dims);
    (map, dims)
}

// ── TYPE field flattening ─────────────────────────────────────────────────────

/// Recursively flatten a user-defined TYPE's fields, expanding any nested
/// UserType fields into their constituent scalar fields.
/// Returns `Vec<(flat_field_path, QbType)>` where `flat_field_path` uses `__`
/// separators, e.g. `"col__r"` for a `Col AS Color` field where `Color` has `R`.
/// Emit a line that unpacks one record field from buffer `buf` into `acc`
/// (a string or f64 accessor) for a random-access GET.
fn record_get_line(acc: &str, repr: &FieldRepr, off: &usize, buf: &str) -> String {
    match repr {
        FieldRepr::Str(n) => format!("{acc} = qb_rec_get_str(&{buf}, {off}, {n});"),
        FieldRepr::I16    => format!("{acc} = qb_rec_get_i16(&{buf}, {off});"),
        FieldRepr::I32    => format!("{acc} = qb_rec_get_i32(&{buf}, {off});"),
        FieldRepr::F32    => format!("{acc} = qb_rec_get_f32(&{buf}, {off});"),
        FieldRepr::F64    => format!("{acc} = qb_rec_get_f64(&{buf}, {off});"),
        FieldRepr::Nested(_) => String::new(), // never a leaf
    }
}

/// Emit a line that packs accessor `acc` into buffer `buf` for a random-access PUT.
fn record_put_line(acc: &str, repr: &FieldRepr, off: &usize, buf: &str) -> String {
    match repr {
        FieldRepr::Str(n) => format!("qb_rec_put_str(&mut {buf}, {off}, &{acc}, {n});"),
        FieldRepr::I16    => format!("qb_rec_put_i16(&mut {buf}, {off}, {acc});"),
        FieldRepr::I32    => format!("qb_rec_put_i32(&mut {buf}, {off}, {acc});"),
        FieldRepr::F32    => format!("qb_rec_put_f32(&mut {buf}, {off}, {acc});"),
        FieldRepr::F64    => format!("qb_rec_put_f64(&mut {buf}, {off}, {acc});"),
        FieldRepr::Nested(_) => String::new(),
    }
}

fn flatten_type_fields(
    type_name: &str,
    type_defs: &HashMap<String, Vec<(String, QbType)>>,
) -> Vec<(String, QbType)> {
    let Some(fields) = type_defs.get(type_name) else { return Vec::new(); };
    let mut result = Vec::new();
    for (fname, fty) in fields {
        if let QbType::UserType(nested_tn) = fty {
            let nested_lc = nested_tn.to_lowercase();
            let nested = flatten_type_fields(&nested_lc, type_defs);
            for (nested_path, nested_ty) in nested {
                result.push((format!("{fname}__{nested_path}"), nested_ty));
            }
        } else {
            result.push((fname.clone(), fty.clone()));
        }
    }
    result
}

// ── DIM lower-bound helpers ───────────────────────────────────────────────────

/// Extract a constant integer lower bound from a parsed Expr.
/// Returns 0 for any non-constant expression (safe fallback — means no index offset).
fn lower_bound_i64(expr: &Expr) -> i64 {
    match expr {
        Expr::IntLit(n)   => *n as i64,
        Expr::FloatLit(f) => *f as i64,
        Expr::UnOp { op: UnOp::Neg, operand } => -lower_bound_i64(operand),
        _ => 0,
    }
}

/// Walk `stmts` recursively and record the lower bound for every DIM/REDIM
/// array declaration into `map`.  Called as a pre-pass before emitting so
/// that subs emitted before `fn main` already know the lower bounds of shared
/// arrays declared in the main body.
fn collect_array_lower_bounds(stmts: &[Stmt], map: &mut HashMap<String, Vec<i64>>) {
    for stmt in stmts {
        match stmt {
            Stmt::Dim(d) | Stmt::ReDim(d) if !d.dims.is_empty() => {
                let lows: Vec<i64> = d.dim_lower.iter().map(lower_bound_i64).collect();
                map.entry(d.name.to_lowercase()).or_insert(lows);
            }
            Stmt::If { then_body, elseif_branches, else_body, .. } => {
                collect_array_lower_bounds(then_body, map);
                for (_, b) in elseif_branches { collect_array_lower_bounds(b, map); }
                if let Some(b) = else_body { collect_array_lower_bounds(b, map); }
            }
            Stmt::For   { body, .. } => collect_array_lower_bounds(body, map),
            Stmt::While { body, .. } => collect_array_lower_bounds(body, map),
            Stmt::Do    { body, .. } => collect_array_lower_bounds(body, map),
            Stmt::Select { cases, default, .. } => {
                for c in cases { collect_array_lower_bounds(&c.body, map); }
                if let Some(b) = default { collect_array_lower_bounds(b, map); }
            }
            Stmt::Block(inner) => collect_array_lower_bounds(inner, map),
            _ => {}
        }
    }
}

// ── GOSUB-block extractor ─────────────────────────────────────────────────────

/// Split `main_body` into:
///   - statements that precede the first `Label` (go into `fn main`)
///   - one `(label_name, body_stmts)` per labeled block (emitted as `fn label(...)`)
///
/// Labeled blocks are delimited by successive `Stmt::Label` nodes; `Stmt::Return`
/// inside a block is kept (it becomes `return;` in the emitted fn).
/// Collect every label name that appears as a GOSUB target (recursively).
fn collect_gosub_targets(stmts: &[Stmt]) -> HashSet<String> {
    let mut targets = HashSet::new();
    for stmt in stmts {
        match stmt {
            Stmt::Gosub(label) => { targets.insert(label.clone()); }
            // ON KEY/TIMER GOSUB targets are extracted as functions so the runtime
            // key-event dispatch helper can call them by name.
            Stmt::OnKeyGosub { target, .. } | Stmt::OnTimerGosub { target, .. } => {
                targets.insert(target.clone());
            }
            // ON ERROR GOTO labels are treated as gosub-style targets only when
            // the label is a named (non-numeric) identifier.  Numeric labels live
            // in the state-machine match arms and must NOT be extracted as fns.
            Stmt::OnError { label } if label != "0" && label.parse::<i64>().is_err() => {
                targets.insert(label.clone());
            }
            // ON expr GOSUB L1,L2,… — every label is a GOSUB target (extracted as
            // a fn so the call + RETURN works), including numeric line labels.
            Stmt::OnGoto { labels, is_gosub: true, .. } => {
                for l in labels { targets.insert(l.clone()); }
            }
            // Named ON GOTO targets are also extracted as GOSUB fns so the emitter
            // can convert `ON x GOTO Label` into a direct fn call (Fix 3 in
            // emit_stmt).  Numeric labels must stay in the __pc state machine.
            Stmt::OnGoto { labels, is_gosub: false, .. } => {
                for l in labels {
                    if l.parse::<u32>().is_err() {
                        targets.insert(l.clone());
                    }
                }
            }
            Stmt::If { then_body, elseif_branches, else_body, .. } => {
                targets.extend(collect_gosub_targets(then_body));
                for (_, eb) in elseif_branches { targets.extend(collect_gosub_targets(eb)); }
                if let Some(eb) = else_body { targets.extend(collect_gosub_targets(eb)); }
            }
            Stmt::For    { body, .. } => { targets.extend(collect_gosub_targets(body)); }
            Stmt::While  { body, .. } => { targets.extend(collect_gosub_targets(body)); }
            Stmt::Do     { body, .. } => { targets.extend(collect_gosub_targets(body)); }
            Stmt::Select { cases, default, .. } => {
                for c in cases { targets.extend(collect_gosub_targets(&c.body)); }
                if let Some(b) = default { targets.extend(collect_gosub_targets(b)); }
            }
            Stmt::Block(inner) => { targets.extend(collect_gosub_targets(inner)); }
            _ => {}
        }
    }
    targets
}

/// Scan SUB/FUNCTION bodies for ON KEY(n) GOSUB and ON TIMER(n) GOSUB targets.
/// These labels live in the main body and must be extracted as gosub functions
/// even though no explicit GOSUB statement in the main body references them.
fn collect_event_gosub_targets_from_stmts(stmts: &[Stmt], targets: &mut HashSet<String>) {
    for stmt in stmts {
        match stmt {
            Stmt::OnKeyGosub   { target, .. } |
            Stmt::OnTimerGosub { target, .. } => { targets.insert(target.clone()); }
            Stmt::If { then_body, elseif_branches, else_body, .. } => {
                collect_event_gosub_targets_from_stmts(then_body, targets);
                for (_, b) in elseif_branches { collect_event_gosub_targets_from_stmts(b, targets); }
                if let Some(b) = else_body { collect_event_gosub_targets_from_stmts(b, targets); }
            }
            Stmt::For    { body, .. } |
            Stmt::While  { body, .. } |
            Stmt::Do     { body, .. } => collect_event_gosub_targets_from_stmts(body, targets),
            Stmt::Select { cases, default, .. } => {
                for c in cases { collect_event_gosub_targets_from_stmts(&c.body, targets); }
                if let Some(b) = default { collect_event_gosub_targets_from_stmts(b, targets); }
            }
            Stmt::Block(inner) => collect_event_gosub_targets_from_stmts(inner, targets),
            _ => {}
        }
    }
}

/// Collect all GOTO target labels reachable from `stmts`.
fn collect_goto_targets(stmts: &[Stmt]) -> HashSet<String> {
    let mut targets = HashSet::new();
    for stmt in stmts {
        match stmt {
            Stmt::Goto(label) => { targets.insert(label.clone()); }
            // ON expr GOTO L1,L2,… — every label is a GOTO target (stays an SM arm).
            Stmt::OnGoto { labels, is_gosub: false, .. } => {
                for l in labels { targets.insert(l.clone()); }
            }
            Stmt::If { then_body, elseif_branches, else_body, .. } => {
                targets.extend(collect_goto_targets(then_body));
                for (_, eb) in elseif_branches { targets.extend(collect_goto_targets(eb)); }
                if let Some(eb) = else_body { targets.extend(collect_goto_targets(eb)); }
            }
            Stmt::For    { body, .. } => { targets.extend(collect_goto_targets(body)); }
            Stmt::While  { body, .. } => { targets.extend(collect_goto_targets(body)); }
            Stmt::Do     { body, .. } => { targets.extend(collect_goto_targets(body)); }
            Stmt::Select { cases, default, .. } => {
                for c in cases { targets.extend(collect_goto_targets(&c.body)); }
                if let Some(b) = default { targets.extend(collect_goto_targets(b)); }
            }
            Stmt::Block(inner) => { targets.extend(collect_goto_targets(inner)); }
            _ => {}
        }
    }
    targets
}

/// Split main body into (inline_stmts, gosub_fn_blocks).
///
/// Labels that are GOSUB targets start a new extracted function block.
/// While inside an active GOSUB block, intermediate labels that are neither
/// GOSUB targets nor GOTO targets are absorbed into that block — this correctly
/// handles old-style BASIC where GOSUB routines span multiple numbered lines
/// and the code falls through from label to label until a RETURN.
fn extract_gosub_blocks(stmts: &[Stmt], extra_gosub_targets: &HashSet<String>) -> (Vec<Stmt>, Vec<(String, Vec<Stmt>)>) {
    let mut gosub_targets = collect_gosub_targets(stmts);
    gosub_targets.extend(extra_gosub_targets.iter().cloned());
    let goto_targets  = collect_goto_targets(stmts);

    let mut main_stmts: Vec<Stmt> = Vec::new();
    let mut blocks: Vec<(String, Vec<Stmt>)> = Vec::new();
    let mut cur_label: Option<String> = None;
    let mut cur_body:  Vec<Stmt> = Vec::new();
    let mut cur_is_gosub = false;

    for stmt in stmts {
        if let Stmt::Label(name) = stmt {
            let is_gosub_target = gosub_targets.contains(name.as_str());
            let is_goto_target  = goto_targets.contains(name.as_str());

            if cur_is_gosub && !is_gosub_target && !is_goto_target {
                // Intermediate line-number label inside a GOSUB body — absorb it.
                // GOSUB subroutines in old-style BASIC fall through multiple
                // numbered lines until RETURN; keep them together in one fn.
                cur_body.push(stmt.clone());
            } else {
                // Flush previous block
                if let Some(label) = cur_label.take() {
                    if cur_is_gosub {
                        blocks.push((label, cur_body.clone()));
                    } else {
                        main_stmts.push(Stmt::Label(label));
                        main_stmts.append(&mut cur_body);
                    }
                    cur_body.clear();
                }
                cur_label    = Some(name.clone());
                cur_is_gosub = is_gosub_target;
            }
        } else if cur_label.is_some() {
            cur_body.push(stmt.clone());
        } else {
            main_stmts.push(stmt.clone());
        }
    }
    // Flush last block
    if let Some(label) = cur_label {
        if cur_is_gosub {
            blocks.push((label, cur_body));
        } else {
            main_stmts.push(Stmt::Label(label));
            main_stmts.extend(cur_body);
        }
    }

    (main_stmts, blocks)
}

// ── Cross-boundary array detection ───────────────────────────────────────────
//
// In QB, GOSUB does not create a new scope — all variables live in the same
// flat namespace.  Our emitter turns GOSUB blocks into separate Rust functions,
// which breaks that assumption.  Any array that is declared in one scope (main
// or a GOSUB block) but *used* in another must be promoted to GameState so
// both scopes can reach it via `__gs`.

fn collect_array_names_stmts(stmts: &[Stmt]) -> HashSet<String> {
    let mut out = HashSet::new();
    for stmt in stmts {
        collect_array_names_stmt(stmt, &mut out);
    }
    out
}

fn collect_array_names_stmt(stmt: &Stmt, out: &mut HashSet<String>) {
    match stmt {
        // DIM / REDIM of an array → record the name
        Stmt::Dim(d) | Stmt::ReDim(d) if !d.dims.is_empty() => {
            out.insert(rust_ident(&d.name));
        }
        // GET / PUT sprite ops reference an array directly
        Stmt::GetSprite { arr, .. } | Stmt::PutSprite { arr, .. } => {
            let name = match arr {
                LValue::Scalar { name, .. } | LValue::Index { name, .. } => name,
                LValue::Field  { base, .. } | LValue::FieldIndex { base, .. } => match base.as_ref() {
                    LValue::Scalar { name, .. } | LValue::Index { name, .. } => name,
                    _ => return,
                },
            };
            out.insert(rust_ident(name));
        }
        // CALL args may contain bare array references (array passed without ())
        Stmt::Call { args, .. } => {
            for e in args { collect_array_names_expr(e, out); }
        }
        // Recurse into compound statements
        Stmt::If { then_body, elseif_branches, else_body, .. } => {
            out.extend(collect_array_names_stmts(then_body));
            for (_, b) in elseif_branches { out.extend(collect_array_names_stmts(b)); }
            if let Some(eb) = else_body { out.extend(collect_array_names_stmts(eb)); }
        }
        Stmt::For  { body, .. }  => out.extend(collect_array_names_stmts(body)),
        Stmt::While { body, .. } => out.extend(collect_array_names_stmts(body)),
        Stmt::Do    { body, .. } => out.extend(collect_array_names_stmts(body)),
        Stmt::Select { cases, default, .. } => {
            for c in cases { out.extend(collect_array_names_stmts(&c.body)); }
            if let Some(d) = default { out.extend(collect_array_names_stmts(d)); }
        }
        Stmt::Block(inner) => out.extend(collect_array_names_stmts(inner)),
        _ => {}
    }
}

fn collect_array_names_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        // A call with no args is how the parser represents a bare array reference
        // (e.g. `arr()` in CALL MySub(arr())). We cannot tell here whether it is
        // truly an array, but over-inclusion is safe.
        Expr::Call { name, args } if args.is_empty() => { out.insert(rust_ident(name)); }
        Expr::Call { args, .. } => { for a in args { collect_array_names_expr(a, out); } }
        Expr::BinOp { lhs, rhs, .. } => {
            collect_array_names_expr(lhs, out);
            collect_array_names_expr(rhs, out);
        }
        Expr::UnOp { operand, .. } => collect_array_names_expr(operand, out),
        _ => {}
    }
}

// ── Named-GOTO labeled-loop helpers ──────────────────────────────────────────

/// Returns the set of label names that are (a) at the tail of `body` (the very
/// last statements, possibly several consecutive Label stmts) and (b) targeted
/// by a named GOTO somewhere within `body`.  These represent the QB idiom of
/// jumping to the end of a DO loop iteration, equivalent to `continue`.
/// Returns true for `INKEY$ = ""` (the DO: LOOP UNTIL INKEY$ = "" drain pattern).
fn is_inkey_eq_empty(expr: &Expr) -> bool {
    match expr {
        Expr::BinOp { op: BinOp::Eq, lhs, rhs } => {
            let lhs_is_inkey = matches!(lhs.as_ref(),
                Expr::Call { name, args } if name.to_lowercase() == "inkey$" && args.is_empty());
            let rhs_is_empty = matches!(rhs.as_ref(), Expr::StrLit(s) if s.is_empty());
            lhs_is_inkey && rhs_is_empty
        }
        _ => false,
    }
}

fn find_bottom_goto_labels(body: &[Stmt]) -> HashSet<String> {
    // Collect all named GOTO targets within the body
    let mut goto_targets: HashSet<String> = HashSet::new();
    collect_named_goto_targets_stmts(body, &mut goto_targets);
    if goto_targets.is_empty() { return HashSet::new(); }

    // Walk from the end of the body, collecting consecutive Label stmts
    let mut bottom_labels: HashSet<String> = HashSet::new();
    for stmt in body.iter().rev() {
        match stmt {
            Stmt::Label(name) if goto_targets.contains(name) => {
                bottom_labels.insert(name.clone());
            }
            Stmt::Label(_) => {
                // Label not targeted by any GOTO — stop scanning
                break;
            }
            _ => break,
        }
    }
    bottom_labels
}

fn collect_named_goto_targets_stmts(stmts: &[Stmt], out: &mut HashSet<String>) {
    for stmt in stmts {
        collect_named_goto_targets_stmt(stmt, out);
    }
}

fn collect_named_goto_targets_stmt(stmt: &Stmt, out: &mut HashSet<String>) {
    match stmt {
        Stmt::Goto(label) if label.parse::<u32>().is_err() => {
            out.insert(label.clone());
        }
        Stmt::If { then_body, elseif_branches, else_body, .. } => {
            collect_named_goto_targets_stmts(then_body, out);
            for (_, b) in elseif_branches { collect_named_goto_targets_stmts(b, out); }
            if let Some(b) = else_body { collect_named_goto_targets_stmts(b, out); }
        }
        Stmt::For    { body, .. } => collect_named_goto_targets_stmts(body, out),
        Stmt::While  { body, .. } => collect_named_goto_targets_stmts(body, out),
        Stmt::Do     { body, .. } => collect_named_goto_targets_stmts(body, out),
        Stmt::Select { cases, default, .. } => {
            for c in cases { collect_named_goto_targets_stmts(&c.body, out); }
            if let Some(b) = default { collect_named_goto_targets_stmts(b, out); }
        }
        Stmt::Block(inner) => collect_named_goto_targets_stmts(inner, out),
        _ => {}
    }
}

// ── Cross-boundary scalar detection ──────────────────────────────────────────

/// Collect all scalar variable (lowercase_name → QbType) references from a
/// statement tree.  Array-indexed accesses are excluded (we only want scalars).
fn collect_scalar_names_stmts(stmts: &[Stmt]) -> HashMap<String, QbType> {
    let mut out: HashMap<String, QbType> = HashMap::new();
    collect_scalar_names_inner(stmts, &mut out);
    out
}

fn collect_scalar_names_inner(stmts: &[Stmt], out: &mut HashMap<String, QbType>) {
    for stmt in stmts {
        collect_scalar_names_stmt(stmt, out);
    }
}

fn collect_scalar_names_stmt(stmt: &Stmt, out: &mut HashMap<String, QbType>) {
    match stmt {
        Stmt::Let { var, expr } => {
            if let LValue::Scalar { name, ty } = var {
                out.entry(name.to_lowercase()).or_insert_with(|| ty.clone());
            }
            collect_scalar_names_expr(expr, out);
        }
        Stmt::Dim(decl) if decl.dims.is_empty() && !decl.shared => {
            out.entry(decl.name.to_lowercase()).or_insert_with(|| decl.ty.clone());
        }
        Stmt::For { var, from, to, step, body } => {
            out.entry(var.to_lowercase()).or_insert(QbType::Single);
            collect_scalar_names_expr(from, out);
            collect_scalar_names_expr(to, out);
            if let Some(s) = step { collect_scalar_names_expr(s, out); }
            collect_scalar_names_inner(body, out);
        }
        Stmt::If { cond, then_body, elseif_branches, else_body } => {
            collect_scalar_names_expr(cond, out);
            collect_scalar_names_inner(then_body, out);
            for (ec, b) in elseif_branches {
                collect_scalar_names_expr(ec, out);
                collect_scalar_names_inner(b, out);
            }
            if let Some(b) = else_body { collect_scalar_names_inner(b, out); }
        }
        Stmt::While { cond, body } => {
            collect_scalar_names_expr(cond, out);
            collect_scalar_names_inner(body, out);
        }
        Stmt::Do { kind, body } => {
            match kind {
                DoKind::WhilePre(e)  | DoKind::UntilPre(e)  |
                DoKind::WhilePost(e) | DoKind::UntilPost(e) =>
                    collect_scalar_names_expr(e, out),
                DoKind::Infinite => {}
            }
            collect_scalar_names_inner(body, out);
        }
        Stmt::Select { expr, cases, default } => {
            collect_scalar_names_expr(expr, out);
            for c in cases { collect_scalar_names_inner(&c.body, out); }
            if let Some(b) = default { collect_scalar_names_inner(b, out); }
        }
        Stmt::Print { args, .. } => {
            for a in args {
                if let PrintArg::Expr(e) = a { collect_scalar_names_expr(e, out); }
            }
        }
        Stmt::Input { vars, .. } => {
            for lv in vars {
                if let LValue::Scalar { name, ty } = lv {
                    out.entry(name.to_lowercase()).or_insert_with(|| ty.clone());
                }
            }
        }
        Stmt::Call { args, .. } => {
            for a in args { collect_scalar_names_expr(a, out); }
        }
        Stmt::Block(inner) => collect_scalar_names_inner(inner, out),
        _ => {}
    }
}

fn collect_scalar_names_expr(expr: &Expr, out: &mut HashMap<String, QbType>) {
    match expr {
        Expr::Var(LValue::Scalar { name, ty }) => {
            out.entry(name.to_lowercase()).or_insert_with(|| ty.clone());
        }
        Expr::BinOp { lhs, rhs, .. } => {
            collect_scalar_names_expr(lhs, out);
            collect_scalar_names_expr(rhs, out);
        }
        Expr::UnOp { operand, .. } => collect_scalar_names_expr(operand, out),
        Expr::Call { args, .. } => {
            for a in args { collect_scalar_names_expr(a, out); }
        }
        Expr::Point { x, y } => {
            collect_scalar_names_expr(x, out);
            collect_scalar_names_expr(y, out);
        }
        Expr::IntLit(_) | Expr::FloatLit(_) | Expr::StrLit(_) => {}
        Expr::Var(_) => {} // Index or Field — not a plain scalar
    }
}

/// Returns scalars that appear in both the main body and at least one GOSUB
/// body but are not already DIM SHARED.  These must be promoted to GameState
/// so the extracted GOSUB function can read the caller's runtime values.
///
/// `exclude` contains lowercase variable names that must NOT be promoted
/// (e.g. parameters of named SUBs, which are already shared via &mut).
fn detect_cross_boundary_scalars(
    main_stmts: &[Stmt],
    gosub_fns:  &[(String, Vec<Stmt>)],
    exclude:    &HashSet<String>,
) -> Vec<(String, QbType)> {
    let main_scalars = collect_scalar_names_stmts(main_stmts);
    let mut result: HashMap<String, QbType> = HashMap::new();

    for (_, body) in gosub_fns {
        let gosub_scalars = collect_scalar_names_stmts(body);
        for (name, ty) in &gosub_scalars {
            if main_scalars.contains_key(name)
               && !result.contains_key(name)
               && !exclude.contains(name)
            {
                // Use main's authoritative type, not the gosub's.  This guards
                // against name collisions between e.g. `X` (f64, in main) and
                // `X$` (String, in a gosub body): both normalise to key "x" in
                // collect_scalar_names, so a gosub-body String X$ can falsely
                // appear to be the same cross-boundary variable as numeric X.
                // The main body is the canonical declaration site.
                let effective_ty = main_scalars.get(name).cloned().unwrap_or_else(|| ty.clone());
                result.insert(name.clone(), effective_ty);
            }
        }
    }

    let mut out: Vec<(String, QbType)> = result.into_iter().collect();
    out.sort_by_key(|(n, _)| n.clone());
    out
}

fn detect_cross_boundary_arrays(
    main_stmts: &[Stmt],
    gosub_fns:  &[(String, Vec<Stmt>)],
) -> HashSet<String> {
    let main_arrays = collect_array_names_stmts(main_stmts);
    let mut result  = HashSet::new();

    for (_, body) in gosub_fns {
        let gosub_arrays = collect_array_names_stmts(body);
        // Any name that appears in BOTH main and this GOSUB body is cross-boundary
        // (catches arrays DIM'd in both scopes, e.g. old-style GOSUB subroutines
        //  that also DIM their own array).
        for name in gosub_arrays.intersection(&main_arrays) {
            result.insert(name.clone());
        }
        // Also find array USES in the GOSUB body (subscript reads/writes like
        // Numbers(I)) by scanning for Expr::Call with non-empty args where the
        // name matches a known main-body array.  This handles ON GOTO→fn targets
        // that use main-body arrays without re-declaring them.
        let mut uses = HashSet::new();
        collect_array_use_refs_stmts(body, &main_arrays, &mut uses);
        result.extend(uses);
    }

    // Cross-references between two different GOSUB blocks
    let gosub_array_sets: Vec<HashSet<String>> =
        gosub_fns.iter().map(|(_, b)| {
            let mut s = collect_array_names_stmts(b);
            collect_array_use_refs_stmts(b, &main_arrays, &mut s);
            s
        }).collect();
    for i in 0..gosub_array_sets.len() {
        for j in (i + 1)..gosub_array_sets.len() {
            for name in gosub_array_sets[i].intersection(&gosub_array_sets[j]) {
                result.insert(name.clone());
            }
        }
    }

    result
}

/// Collect names of subscript-accessed arrays in `stmts`, filtered to only
/// names that are already known to be arrays (present in `known_arrays`).
/// This avoids promoting plain function calls (e.g. `factorial(n)`) — only
/// calls whose name is a confirmed array declaration are collected.
fn collect_array_use_refs_stmts(stmts: &[Stmt], known_arrays: &HashSet<String>,
                                 out: &mut HashSet<String>) {
    for stmt in stmts {
        collect_array_use_refs_stmt(stmt, known_arrays, out);
    }
}

fn collect_array_use_refs_stmt(stmt: &Stmt, known_arrays: &HashSet<String>,
                                out: &mut HashSet<String>) {
    // Helper to recurse into an expression
    fn scan_expr(e: &Expr, ka: &HashSet<String>, out: &mut HashSet<String>) {
        match e {
            Expr::Call { name, args } if !args.is_empty() => {
                // Strip sigil chars ($, %, !, #, &) before lowercasing: Expr::Call
                // stores the full name-with-sigil (e.g. "Names$") while known_arrays
                // keys are sigil-free (DIM VarDecl.name strips the sigil at parse time).
                let bare = name.trim_end_matches(['$', '%', '!', '#', '&']).to_lowercase();
                if ka.contains(&bare) { out.insert(bare); }
                for a in args { scan_expr(a, ka, out); }
            }
            Expr::Call { args, .. } => {
                for a in args { scan_expr(a, ka, out); }
            }
            Expr::BinOp { lhs, rhs, .. } => { scan_expr(lhs, ka, out); scan_expr(rhs, ka, out); }
            Expr::UnOp  { operand, .. }   => scan_expr(operand, ka, out),
            _ => {}
        }
    }
    // Helper to recurse into an LValue
    fn scan_lv(lv: &LValue, ka: &HashSet<String>, out: &mut HashSet<String>) {
        match lv {
            LValue::Index { name, indices, .. } => {
                let n = rust_ident(name);
                if ka.contains(&n) { out.insert(n); }
                for e in indices { scan_expr(e, ka, out); }
            }
            LValue::Field { base, .. } => scan_lv(base, ka, out),
            _ => {}
        }
    }

    match stmt {
        Stmt::Let { var, expr }     => { scan_lv(var, known_arrays, out); scan_expr(expr, known_arrays, out); }
        Stmt::LSet { var, expr } | Stmt::RSet { var, expr } => {
            scan_lv(var, known_arrays, out); scan_expr(expr, known_arrays, out);
        }
        Stmt::Print { args, .. } | Stmt::PrintFile { args, .. } => {
            for a in args {
                if let PrintArg::Expr(e) | PrintArg::Tab(e) | PrintArg::Spc(e) = a {
                    scan_expr(e, known_arrays, out);
                }
            }
        }
        Stmt::PrintUsing { fmt, args, .. } => {
            scan_expr(fmt, known_arrays, out);
            for e in args { scan_expr(e, known_arrays, out); }
        }
        Stmt::Input  { vars, .. } => { for lv in vars { scan_lv(lv, known_arrays, out); } }
        Stmt::Read   (vars)       => { for lv in vars { scan_lv(lv, known_arrays, out); } }
        Stmt::Call   { args, .. } => { for e in args { scan_expr(e, known_arrays, out); } }
        Stmt::If { cond, then_body, elseif_branches, else_body } => {
            scan_expr(cond, known_arrays, out);
            collect_array_use_refs_stmts(then_body, known_arrays, out);
            for (ec, b) in elseif_branches {
                scan_expr(ec, known_arrays, out);
                collect_array_use_refs_stmts(b, known_arrays, out);
            }
            if let Some(b) = else_body { collect_array_use_refs_stmts(b, known_arrays, out); }
        }
        Stmt::For { from, to, step, body, .. } => {
            scan_expr(from, known_arrays, out);
            scan_expr(to,   known_arrays, out);
            if let Some(s) = step { scan_expr(s, known_arrays, out); }
            collect_array_use_refs_stmts(body, known_arrays, out);
        }
        Stmt::While { cond, body } => {
            scan_expr(cond, known_arrays, out);
            collect_array_use_refs_stmts(body, known_arrays, out);
        }
        Stmt::Do { kind, body } => {
            match kind {
                DoKind::WhilePre(e) | DoKind::UntilPre(e) |
                DoKind::WhilePost(e) | DoKind::UntilPost(e) => scan_expr(e, known_arrays, out),
                DoKind::Infinite => {}
            }
            collect_array_use_refs_stmts(body, known_arrays, out);
        }
        Stmt::Select { expr, cases, default } => {
            scan_expr(expr, known_arrays, out);
            for c in cases { collect_array_use_refs_stmts(&c.body, known_arrays, out); }
            if let Some(b) = default { collect_array_use_refs_stmts(b, known_arrays, out); }
        }
        Stmt::Block(inner)  => collect_array_use_refs_stmts(inner, known_arrays, out),
        Stmt::Dim(d) | Stmt::ReDim(d) if !d.dims.is_empty() => {
            for e in &d.dims      { scan_expr(e, known_arrays, out); }
            for e in &d.dim_lower { scan_expr(e, known_arrays, out); }
        }
        _ => {}
    }
}

// ── State-machine helpers ─────────────────────────────────────────────────────

/// Collect all non-shared DIM'd local array declarations from `stmts` (recursive).
/// Returns `(rust_name, elem_ty_str, ndims)` for each unique array.
fn collect_sm_local_arrays(
    stmts: &[Stmt],
    shared_names: &HashSet<String>,
) -> Vec<(String, &'static str, usize)> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut result = Vec::new();
    collect_sm_local_arrays_inner(stmts, shared_names, &mut seen, &mut result);
    result
}

fn collect_sm_local_arrays_inner(
    stmts: &[Stmt],
    shared_names: &HashSet<String>,
    seen: &mut HashSet<String>,
    result: &mut Vec<(String, &'static str, usize)>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Dim(decl) if !decl.dims.is_empty() && !decl.shared => {
                let lc = decl.name.to_lowercase();
                if !shared_names.contains(&lc) {
                    let name = rust_ident_typed(&decl.name, &decl.ty);
                    if seen.insert(name.clone()) {
                        let elem = qb_type_to_rust(&decl.ty);
                        result.push((name, elem, decl.dims.len()));
                    }
                }
            }
            Stmt::If { then_body, elseif_branches, else_body, .. } => {
                collect_sm_local_arrays_inner(then_body, shared_names, seen, result);
                for (_, b) in elseif_branches {
                    collect_sm_local_arrays_inner(b, shared_names, seen, result);
                }
                if let Some(b) = else_body {
                    collect_sm_local_arrays_inner(b, shared_names, seen, result);
                }
            }
            Stmt::For  { body, .. } => collect_sm_local_arrays_inner(body, shared_names, seen, result),
            Stmt::While { body, .. } => collect_sm_local_arrays_inner(body, shared_names, seen, result),
            Stmt::Do    { body, .. } => collect_sm_local_arrays_inner(body, shared_names, seen, result),
            Stmt::Select { cases, default, .. } => {
                for c in cases { collect_sm_local_arrays_inner(&c.body, shared_names, seen, result); }
                if let Some(b) = default { collect_sm_local_arrays_inner(b, shared_names, seen, result); }
            }
            Stmt::Block(inner) => collect_sm_local_arrays_inner(inner, shared_names, seen, result),
            _ => {}
        }
    }
}

/// Returns true if `stmt` or any nested stmt contains a `Stmt::Goto`.

/// Like `stmt_has_goto` but only matches GOTO targets that are numeric line
/// numbers (i.e. GW-BASIC / line-numbered programs).  Named-label GOTOs in
/// modern QBasic programs use a different handling path and must NOT trigger
/// the state-machine emitter.
fn stmt_has_numeric_goto(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Goto(label) => label.parse::<u32>().is_ok(),
        // ON expr GOTO <numeric> implies a line-numbered program → state machine.
        Stmt::OnGoto { labels, is_gosub: false, .. } =>
            labels.iter().any(|l| l.parse::<u32>().is_ok()),
        Stmt::If { then_body, elseif_branches, else_body, .. } => {
            then_body.iter().any(stmt_has_numeric_goto)
            || elseif_branches.iter().any(|(_, b)| b.iter().any(stmt_has_numeric_goto))
            || else_body.as_ref().map_or(false, |b| b.iter().any(stmt_has_numeric_goto))
        }
        Stmt::For    { body, .. } => body.iter().any(stmt_has_numeric_goto),
        Stmt::While  { body, .. } => body.iter().any(stmt_has_numeric_goto),
        Stmt::Do     { body, .. } => body.iter().any(stmt_has_numeric_goto),
        Stmt::Select { cases, default, .. } => {
            cases.iter().any(|c| c.body.iter().any(stmt_has_numeric_goto))
            || default.as_ref().map_or(false, |b| b.iter().any(stmt_has_numeric_goto))
        }
        Stmt::Block(inner) => inner.iter().any(stmt_has_numeric_goto),
        _ => false,
    }
}

/// Partition a flat statement list into blocks separated by numeric line-number labels.
/// Returns `Vec<(pc, body_stmts)>` where `pc` is the line number (or 0 for stmts
/// appearing before the first numeric label).
fn flatten_to_blocks(stmts: &[Stmt]) -> Vec<(u32, Vec<Stmt>)> {
    let mut blocks: Vec<(u32, Vec<Stmt>)> = Vec::new();
    let mut current_pc: Option<u32> = None;
    let mut current_body: Vec<Stmt> = Vec::new();

    for stmt in stmts {
        if let Stmt::Label(s) = stmt {
            if let Ok(n) = s.parse::<u32>() {
                // Flush the previous block
                if let Some(pc) = current_pc.take() {
                    blocks.push((pc, std::mem::take(&mut current_body)));
                } else if !current_body.is_empty() {
                    blocks.push((0, std::mem::take(&mut current_body)));
                }
                current_pc = Some(n);
                // Don't add the Label itself to the body — it becomes the block key
            } else {
                // Non-numeric label (e.g. named GOSUB target) — keep as comment in body
                current_body.push(stmt.clone());
            }
        } else {
            current_body.push(stmt.clone());
        }
    }

    // Flush the last block
    if let Some(pc) = current_pc {
        blocks.push((pc, current_body));
    } else if !current_body.is_empty() {
        blocks.push((0, current_body));
    }

    // Sort by pc so arms are in line-number order (source should already be ordered,
    // but guard against edge cases)
    blocks.sort_by_key(|(pc, _)| *pc);
    blocks
}

// ── Per-sub shared-name helpers ───────────────────────────────────────────────

/// Collect names from all `SHARED var` declarations in a sub body (including
/// nested control structures).  These are the module-level variables that THIS
/// specific sub explicitly opts into via a SHARED statement.
fn collect_sub_explicit_shared(stmts: &[Stmt]) -> HashSet<String> {
    let mut out = HashSet::new();
    fn visit(stmts: &[Stmt], out: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::SharedDecl(ns) => { for n in ns { out.insert(n.clone()); } }
                Stmt::Block(inner) => visit(inner, out),
                Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } =>
                    visit(body, out),
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    visit(then_body, out);
                    for (_, b) in elseif_branches { visit(b, out); }
                    if let Some(b) = else_body { visit(b, out); }
                }
                Stmt::Select { cases, default, .. } => {
                    for c in cases { visit(&c.body, out); }
                    if let Some(b) = default { visit(b, out); }
                }
                _ => {}
            }
        }
    }
    visit(stmts, &mut out);
    out
}

/// Collect names of variables declared with `DIM SHARED` (the `SHARED` keyword
/// directly on the DIM statement) anywhere in the main body.  These are always
/// visible in every sub without a per-sub SHARED declaration.
fn collect_dim_shared_names(stmts: &[Stmt]) -> HashSet<String> {
    let mut out = HashSet::new();
    fn visit(stmts: &[Stmt], out: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::Dim(d) if d.shared => { out.insert(rust_ident(&d.name)); }
                Stmt::ReDim(d) if d.shared => { out.insert(rust_ident(&d.name)); }
                Stmt::Block(inner) => visit(inner, out),
                Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } =>
                    visit(body, out),
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    visit(then_body, out);
                    for (_, b) in elseif_branches { visit(b, out); }
                    if let Some(b) = else_body { visit(b, out); }
                }
                Stmt::Select { cases, default, .. } => {
                    for c in cases { visit(&c.body, out); }
                    if let Some(b) = default { visit(b, out); }
                }
                _ => {}
            }
        }
    }
    visit(stmts, &mut out);
    out
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn emit(prog: &AnalyzedProgram) -> Result<String> {
    Emitter::new().emit(prog)
}

// ── Post-processing: collapse single-use __tmpN temporaries ──────────────────
//
// lift_expr() hoists every user-fn call and __rt.* built-in call into a
// `let __tmpN = expr;` temporary so that args to `__rt.method(...)` calls don't
// double-borrow `__rt` or `__gs`.  That's correct when the result ends up as
// an argument to another `__rt.*` call, but when the result is immediately
// assigned to a plain variable (`y = __tmp1;`) the temp is unnecessary.
//
// This pass detects the pattern:
//   let __tmpN = expr;          (immutable, plain __tmp prefix, no __tmp_*)
//   ...
//   lhs = __tmpN;               (__tmpN appears exactly once in the rest)
// and collapses it to:
//   lhs = expr;
//
// Safety:
// - Only immutable `let` (not `let mut`) are collapsed.
// - Exactly-two-occurrence check: if __tmpN is used anywhere else (as an arg
//   to an __rt call, in a complex expression, etc.) count > 2 → left alone.
// - The use must be a standalone assignment (`lhs = __tmpN;`), not embedded in
//   a larger expression — checked by matching the entire trimmed RHS.

fn count_word_occurrences(s: &str, word: &str) -> usize {
    let sb = s.as_bytes();
    let wb = word.as_bytes();
    let wlen = wb.len();
    let mut count = 0;
    let mut i = 0;
    while i + wlen <= sb.len() {
        if &sb[i..i + wlen] == wb {
            let before_ok = i == 0 || !sb[i - 1].is_ascii_alphanumeric() && sb[i - 1] != b'_';
            let after_ok = i + wlen >= sb.len()
                || !sb[i + wlen].is_ascii_alphanumeric() && sb[i + wlen] != b'_';
            if before_ok && after_ok {
                count += 1;
            }
        }
        i += 1;
    }
    count
}

fn inline_single_use_tmps(out: &str) -> String {
    let lines: Vec<&str> = out.lines().collect();
    let mut deletions: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut replacements: std::collections::HashMap<usize, String> =
        std::collections::HashMap::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        // Only immutable, only plain __tmp (not __tmp_num, __tmp_gs, __tmp_str, etc.)
        if !trimmed.starts_with("let __tmp") { continue; }
        if trimmed.starts_with("let mut __tmp") { continue; }

        // Parse: "let __tmpN = expr;"
        let after_let = &trimmed["let ".len()..];
        let eq_pos = match after_let.find(" = ") { Some(p) => p, None => continue };
        let tmp_name = &after_let[..eq_pos];
        // tmp_name must be __tmp followed by ONLY digits (e.g. __tmp42, not __tmp_num3)
        let digits_part = match tmp_name.strip_prefix("__tmp") { Some(d) => d, None => continue };
        if digits_part.is_empty() || !digits_part.chars().all(|c| c.is_ascii_digit()) { continue; }
        let expr = after_let[eq_pos + 3..].trim_end_matches(';').trim();

        // Count total word-boundary occurrences — must be exactly 2 (def + one use)
        if count_word_occurrences(out, tmp_name) != 2 { continue; }

        // Find the other line and verify it's a standalone `lhs = __tmpN;`
        let use_suffix = format!(" = {};", tmp_name);
        for (j, other) in lines.iter().enumerate() {
            if j == i { continue; }
            let ot = other.trim_start();
            if !ot.ends_with(use_suffix.as_str()) { continue; }
            // lhs is everything before " = __tmpN;"
            let lhs = &ot[..ot.len() - use_suffix.len()];
            // lhs must not itself contain __tmpN (edge-case guard)
            if lhs.contains(tmp_name) { continue; }
            let indent: &str = &other[..other.len() - other.trim_start().len()];
            replacements.insert(j, format!("{indent}{lhs} = {expr};"));
            deletions.insert(i);
            break;
        }
    }

    if deletions.is_empty() {
        return out.to_string();
    }

    let mut result = String::with_capacity(out.len());
    for (i, line) in lines.iter().enumerate() {
        if deletions.contains(&i) { continue; }
        result.push_str(replacements.get(&i).map(String::as_str).unwrap_or(line));
        result.push('\n');
    }
    result
}
