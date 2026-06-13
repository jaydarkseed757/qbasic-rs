use crate::error::QbError;
use anyhow::Result;

// ── Token ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    IntLit(i32),
    FloatLit(f64),
    StrLit(String),

    // Identifiers — sigil encodes the declared type
    Ident(String),       // bare / single (no sigil or !)
    IdentStr(String),    // name$
    IdentInt(String),    // name%
    IdentSng(String),    // name!
    IdentDbl(String),    // name#  or  name& (Long → f64)

    // ── Keywords ──────────────────────────────────────────────────────────────

    // Declarations / structure
    Declare, Sub, Function, End,
    Dim, ReDim, Shared, As, Preserve, Common, Static,
    Type, // user-defined type (stub — not used in Gorillas)

    // Control flow
    If, Then, Else, ElseIf,
    #[allow(dead_code)] EndIf,
    For, To, Step, Next,
    While, Wend,
    Do, Loop, Until,
    Select, Case,
    Goto, Gosub, Return,
    Exit,

    // I/O
    Print, Input, Locate, Color, Cls,

    // Graphics
    Screen, Circle, Line, Pset, Preset, Paint, View, Window,

    // Sound
    Play, Sound, Beep,

    // Misc statements
    Let, Call, Swap, Erase,
    Stop,
    #[allow(dead_code)] End_,   // END as a statement (distinct from END SUB / END IF)
    Data, Read, Restore,
    Randomize,
    Option_, // OPTION BASE

    // gorilla.bas extras
    Const,
    Def,           // DEF FN / DEF SEG
    On,            // ON ERROR GOTO
    ErrorKw,       // ERROR (keyword, avoids collision with std Error)
    Resume,        // RESUME / RESUME NEXT
    Palette,       // PALETTE color, value
    Poke,          // POKE addr, val
    Out,           // OUT port, val
    Inp,           // INP(port) — read hardware port
    Put,           // PUT (x,y), array, mode  (sprite blit)
    Get,           // GET (x1,y1)-(x2,y2), array  (screen capture)
    Width,         // WIDTH col, row
    DefInt,        // DEFINT A-Z
    DefSng,        // DEFSNG
    DefDbl,        // DEFDBL
    DefStr,        // DEFSTR

    // Boolean / logical keywords
    And, Or, Not, Xor, Eqv, Imp,
    Mod,

    // SELECT CASE helpers
    Is,

    // ── Operators ─────────────────────────────────────────────────────────────
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Backslash,  // \ (integer division)
    Caret,      // ^
    Eq,         // =
    Ne,         // <>
    Lt,         // <
    Le,         // <=
    Gt,         // >
    Ge,         // >=
    Amp,        // & (string concat — rare in QB but exists)

    // ── Punctuation ───────────────────────────────────────────────────────────
    LParen,    // (
    RParen,    // )
    Comma,     // ,
    Semicolon, // ;
    Colon,     // :
    Dot,       // .
    Hash,      // # (also used as file-number prefix: OPEN … #1)

    // ── Transpiler directives (from REM QBC …) ───────────────────────────────
    /// `REM QBC FULLSPEED` etc. — directive text uppercased, whitespace-trimmed.
    QbcDirective(String),

    // ── Structure ─────────────────────────────────────────────────────────────
    Newline,
    Eof,
}

impl Token {
    /// Reconstruct the source text of a token for use in unquoted DATA elements.
    pub fn to_data_string(&self) -> String {
        match self {
            Token::Ident(s) | Token::IdentStr(s) | Token::IdentInt(s) |
            Token::IdentSng(s) | Token::IdentDbl(s) => s.clone(),
            Token::IntLit(n)   => n.to_string(),
            Token::FloatLit(f) => f.to_string(),
            Token::StrLit(s)   => format!("\"{s}\""),
            Token::Plus  => "+".into(),  Token::Minus  => "-".into(),
            Token::Star  => "*".into(),  Token::Slash  => "/".into(),
            Token::Eq    => "=".into(),  Token::Ne     => "<>".into(),
            Token::Lt    => "<".into(),  Token::Le     => "<=".into(),
            Token::Gt    => ">".into(),  Token::Ge     => ">=".into(),
            Token::LParen => "(".into(), Token::RParen => ")".into(),
            Token::Comma  => ",".into(), Token::Semicolon => ";".into(),
            Token::Colon  => ":".into(), Token::Dot    => ".".into(),
            Token::Hash   => "#".into(),
            // Keywords that might appear in unquoted DATA: preserve as-is
            _ => format!("{self:?}"),
        }
    }
}

// ── Spanned ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Spanned {
    pub token: Token,
    pub line:  u32,
}

impl Spanned {
    fn new(token: Token, line: u32) -> Self {
        Self { token, line }
    }
}

// ── Keyword map ───────────────────────────────────────────────────────────────

fn keyword(word: &str) -> Option<Token> {
    // QB is case-insensitive; caller passes the uppercased word.
    Some(match word {
        "COMMON"    => Token::Common,
        "CONST"     => Token::Const,
        "DEF"       => Token::Def,
        "DEFINT"    => Token::DefInt,
        "DEFSNG"    => Token::DefSng,
        "DEFDBL"    => Token::DefDbl,
        "DEFSTR"    => Token::DefStr,
        "ERROR"     => Token::ErrorKw,
        "GET"       => Token::Get,
        "ON"        => Token::On,
        "PALETTE"   => Token::Palette,
        "POKE"      => Token::Poke,
        "OUT"       => Token::Out,
        "INP"       => Token::Inp,
        "PUT"       => Token::Put,
        "RESUME"    => Token::Resume,
        "WIDTH"     => Token::Width,
        "AND"       => Token::And,
        "AS"        => Token::As,
        "BEEP"      => Token::Beep,
        "CALL"      => Token::Call,
        "CASE"      => Token::Case,
        "CIRCLE"    => Token::Circle,
        "CLS"       => Token::Cls,
        "COLOR"     => Token::Color,
        "DATA"      => Token::Data,
        "DECLARE"   => Token::Declare,
        "DIM"       => Token::Dim,
        "DO"        => Token::Do,
        "ELSE"      => Token::Else,
        "ELSEIF"    => Token::ElseIf,
        "END"       => Token::End,
        "EXIT"      => Token::Exit,
        "FOR"       => Token::For,
        "FUNCTION"  => Token::Function,
        "GOSUB"     => Token::Gosub,
        "GOTO"      => Token::Goto,
        "IF"        => Token::If,
        "INPUT"     => Token::Input,
        "IS"        => Token::Is,
        "LET"       => Token::Let,
        "LINE"      => Token::Line,
        "LOCATE"    => Token::Locate,
        "LOOP"      => Token::Loop,
        "MOD"       => Token::Mod,
        "NEXT"      => Token::Next,
        "NOT"       => Token::Not,
        "OPTION"    => Token::Option_,
        "OR"        => Token::Or,
        "PAINT"     => Token::Paint,
        "PLAY"      => Token::Play,
        "PRESET"    => Token::Preset,
        "PRESERVE"  => Token::Preserve,
        "PRINT"     => Token::Print,
        "LPRINT"    => Token::Print,   // line-printer output → treat as PRINT
        "PSET"      => Token::Pset,
        "RANDOMIZE" => Token::Randomize,
        "READ"      => Token::Read,
        "REDIM"     => Token::ReDim,
        "REM"       => Token::Newline, // comment — rest of line discarded by caller
        "RESTORE"   => Token::Restore,
        "RETURN"    => Token::Return,
        "SCREEN"    => Token::Screen,
        "SELECT"    => Token::Select,
        "SHARED"    => Token::Shared,
        "SOUND"     => Token::Sound,
        "STATIC"    => Token::Static,
        "STEP"      => Token::Step,
        "STOP"      => Token::Stop,
        "SUB"       => Token::Sub,
        "SWAP"      => Token::Swap,
        "ERASE"     => Token::Erase,
        "THEN"      => Token::Then,
        "TO"        => Token::To,
        "TYPE"      => Token::Type,
        "UNTIL"     => Token::Until,
        "VIEW"      => Token::View,
        "WEND"      => Token::Wend,
        "WHILE"     => Token::While,
        "WINDOW"    => Token::Window,
        "XOR"       => Token::Xor,
        "EQV"       => Token::Eqv,
        "IMP"       => Token::Imp,
        _ => return None,
    })
}

// ── Tokenizer ─────────────────────────────────────────────────────────────────

pub fn tokenize(source: &str) -> Result<Vec<Spanned>> {
    let mut out   = Vec::new();
    let mut chars = source.chars().peekable();
    let mut line  = 1u32;
    // True once we see an integer literal in statement position — i.e. the
    // program is GW-BASIC / line-numbered style.  In that mode a physical line
    // that does NOT begin with a line number is a continuation of the previous
    // logical line and must NOT introduce a Newline token.
    let mut in_line_numbered_mode = false;

    macro_rules! push {
        ($tok:expr) => {{ out.push(Spanned::new($tok, line)); }};
    }

    while let Some(&ch) = chars.peek() {
        match ch {
            // ── Whitespace (non-newline) ──────────────────────────────────────
            ' ' | '\t' | '\r' => { chars.next(); }

            // ── Newline ───────────────────────────────────────────────────────
            '\n' => {
                // In line-numbered (GW-BASIC) mode a physical line that does
                // not start with a line number continues the previous logical
                // line.  Detect this by cloning the iterator and peeking past
                // any leading whitespace.  If the first non-whitespace char is
                // a digit (new line number), blank, or EOF → new logical line
                // (emit Newline as normal).  Otherwise → continuation (no token).
                let last_is_newline = out.last()
                    .map(|s: &Spanned| s.token == Token::Newline)
                    .unwrap_or(true);
                chars.next();
                line += 1;

                if in_line_numbered_mode {
                    let mut probe = chars.clone();
                    while matches!(probe.peek(), Some(&' ') | Some(&'\t') | Some(&'\r')) {
                        probe.next();
                    }
                    let is_new_logical_line = matches!(
                        probe.peek(),
                        None | Some(&'\n') | Some(&('0'..='9'))
                    );
                    if is_new_logical_line && !last_is_newline {
                        push!(Token::Newline);
                    }
                    // Continuation: emit nothing; leading whitespace is consumed
                    // by the outer loop's whitespace arm on the next iteration.
                } else {
                    // Non-line-numbered program: every \n is a statement separator.
                    if !last_is_newline {
                        push!(Token::Newline);
                    }
                }
            }

            // ── Line comment ──────────────────────────────────────────────────
            '\'' => {
                // Consume to end of line; emit Newline so the parser sees the
                // logical line break.
                while chars.peek().map(|&c| c != '\n').unwrap_or(false) {
                    chars.next();
                }
            }

            // ── String literal ────────────────────────────────────────────────
            '"' => {
                chars.next(); // consume opening quote
                let mut s = String::new();
                loop {
                    match chars.next() {
                        // Old IBM BASIC silently closes strings at end-of-line;
                        // several lines in DONKEY.BAS rely on this.
                        None | Some('\n') => { break; }
                        Some('"') => {
                            // QB uses "" to embed a literal quote character
                            if chars.peek() == Some(&'"') {
                                chars.next();
                                s.push('"');
                            } else {
                                break;
                            }
                        }
                        Some(c) => s.push(c),
                    }
                }
                push!(Token::StrLit(s));
            }

            // ── Numeric literal ───────────────────────────────────────────────
            '0'..='9' => {
                let tok = lex_number(&mut chars, line)?;
                // Entering line-numbered mode: first IntLit seen in statement
                // position (start of stream or immediately after a Newline) is
                // a GW-BASIC line number.
                if !in_line_numbered_mode {
                    if matches!(tok, Token::IntLit(_)) {
                        let in_stmt_pos = out.last()
                            .map(|s| s.token == Token::Newline)
                            .unwrap_or(true); // empty = very start of file
                        if in_stmt_pos {
                            in_line_numbered_mode = true;
                        }
                    }
                }
                push!(tok);
            }

            // '.' is a decimal point only when followed by a digit (.5, .25);
            // otherwise it's the member-access Dot punctuation.
            '.' => {
                let mut probe = chars.clone();
                probe.next(); // skip the '.'
                if probe.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    let tok = lex_number(&mut chars, line)?;
                    push!(tok);
                } else {
                    chars.next();
                    push!(Token::Dot);
                }
            }

            // ── Hex / octal literal  &H1F  &O77 ──────────────────────────────
            '&' => {
                chars.next();
                match chars.peek() {
                    Some(&'H') | Some(&'h') => {
                        chars.next();
                        let hex: String = std::iter::from_fn(|| {
                            if chars.peek().map(|c| c.is_ascii_hexdigit()).unwrap_or(false) {
                                chars.next()
                            } else { None }
                        }).collect();
                        let n = i32::from_str_radix(&hex, 16).map_err(|_| QbError::Lex {
                            line,
                            msg: format!("bad hex literal &H{hex}"),
                        })?;
                        push!(Token::IntLit(n));
                    }
                    Some(&'O') | Some(&'o') => {
                        chars.next();
                        let oct: String = std::iter::from_fn(|| {
                            if chars.peek().map(|c| matches!(c, '0'..='7')).unwrap_or(false) {
                                chars.next()
                            } else { None }
                        }).collect();
                        let n = i32::from_str_radix(&oct, 8).map_err(|_| QbError::Lex {
                            line,
                            msg: format!("bad octal literal &O{oct}"),
                        })?;
                        push!(Token::IntLit(n));
                    }
                    _ => {
                        // bare & — string concat operator (rare)
                        push!(Token::Amp);
                    }
                }
            }

            // ── Identifier or keyword ─────────────────────────────────────────
            'a'..='z' | 'A'..='Z' | '_' => {
                let word = lex_ident(&mut chars);
                let upper = word.to_ascii_uppercase();

                // REM is a line comment only in statement position (after newline,
                // colon, or at the very start of input) — not when used as a
                // variable name mid-expression (e.g. `rem = BASE MOD x`).
                if upper == "REM" {
                    // REM is a comment only in statement position:
                    //   - start of input
                    //   - after a Newline or Colon
                    //   - after a line number (IntLit or FloatLit directly following Newline/start)
                    let in_stmt_pos = match out.last().map(|t| &t.token) {
                        None => true,
                        Some(Token::Newline) | Some(Token::Colon) => true,
                        Some(Token::IntLit(_)) | Some(Token::FloatLit(_)) => {
                            // It's a line number if the token before it is Newline or start
                            let prev2 = out.len().checked_sub(2)
                                .and_then(|i| out.get(i))
                                .map(|t| &t.token);
                            matches!(prev2, None | Some(Token::Newline))
                        }
                        _ => false,
                    };
                    if in_stmt_pos {
                        // Collect the rest of the line so we can inspect it for QBC directives.
                        let mut comment = String::new();
                        while chars.peek().map(|&c| c != '\n').unwrap_or(false) {
                            comment.push(chars.next().unwrap());
                        }
                        let trimmed = comment.trim();
                        if trimmed.to_ascii_uppercase().starts_with("QBC") {
                            // After "QBC", split into keyword + value.
                            // Keyword is uppercased (for matching); value preserves original case
                            // so that REM QBC TITLE My Game keeps "My Game" as-is.
                            let after_qbc = trimmed["QBC".len()..].trim();
                            let directive = if let Some(sp) = after_qbc.find(' ') {
                                // e.g. "FPS 30" or "TITLE My Game"
                                let kw  = after_qbc[..sp].to_ascii_uppercase();
                                let val = after_qbc[sp+1..].trim();
                                format!("{kw} {val}")
                            } else {
                                // e.g. "FULLSPEED"
                                after_qbc.to_ascii_uppercase()
                            };
                            if !directive.is_empty() {
                                push!(Token::QbcDirective(directive));
                            }
                        }
                        continue;
                    }
                    // Not in statement position — treat as a plain identifier.
                    push!(Token::Ident(word));
                    continue;
                }

                // QB4.5 line continuation: a bare `_` at end of a logical line
                // (followed only by optional spaces/tabs then `\n`) joins the
                // next physical line without emitting a Newline token.
                if word == "_" {
                    // Peek past any trailing spaces/tabs.
                    let mut probe = chars.clone();
                    while matches!(probe.peek(), Some(&' ') | Some(&'\t') | Some(&'\r')) {
                        probe.next();
                    }
                    if matches!(probe.peek(), Some(&'\n') | None) {
                        // It is a continuation — consume the trailing whitespace
                        // and the newline, bump the line counter, emit nothing.
                        while matches!(chars.peek(), Some(&' ') | Some(&'\t') | Some(&'\r')) {
                            chars.next();
                        }
                        if chars.peek() == Some(&'\n') {
                            chars.next();
                            line += 1;
                        }
                        continue;
                    }
                }

                // Check for sigil immediately following the identifier
                let sigil = match chars.peek() {
                    Some(&'$') => { chars.next(); Some('$') }
                    Some(&'%') => { chars.next(); Some('%') }
                    Some(&'!') => { chars.next(); Some('!') }
                    Some(&'#') => { chars.next(); Some('#') }
                    Some(&'&') => { chars.next(); Some('&') } // Long → treat as Double/f64
                    _          => None,
                };

                let tok = if sigil.is_none() {
                    // Keywords only match bare identifiers (no sigil)
                    keyword(&upper).unwrap_or(Token::Ident(word))
                } else {
                    match sigil.unwrap() {
                        '$' => Token::IdentStr(word),
                        '%' => Token::IdentInt(word),
                        '!' => Token::IdentSng(word),
                        '#' | '&' => Token::IdentDbl(word), // Long treated same as Double
                        _   => unreachable!(),
                    }
                };
                push!(tok);
            }

            // ── Operators ─────────────────────────────────────────────────────
            '+' => { chars.next(); push!(Token::Plus); }
            '-' => { chars.next(); push!(Token::Minus); }
            '*' => { chars.next(); push!(Token::Star); }
            '/' => { chars.next(); push!(Token::Slash); }
            '\\' => { chars.next(); push!(Token::Backslash); }
            '^' => { chars.next(); push!(Token::Caret); }
            '=' => {
                chars.next();
                // Old IBM BASIC accepted => as an alias for >= (reversed operator)
                if chars.peek() == Some(&'>') {
                    chars.next();
                    push!(Token::Ge);
                } else {
                    push!(Token::Eq);
                }
            }

            '<' => {
                chars.next();
                match chars.peek() {
                    Some(&'>') => { chars.next(); push!(Token::Ne); }
                    Some(&'=') => { chars.next(); push!(Token::Le); }
                    _          => push!(Token::Lt),
                }
            }
            '>' => {
                chars.next();
                match chars.peek() {
                    Some(&'=') => { chars.next(); push!(Token::Ge); }
                    _          => push!(Token::Gt),
                }
            }

            // ── Punctuation ───────────────────────────────────────────────────
            '(' => { chars.next(); push!(Token::LParen); }
            ')' => { chars.next(); push!(Token::RParen); }
            ',' => { chars.next(); push!(Token::Comma); }
            ';' => { chars.next(); push!(Token::Semicolon); }
            ':' => { chars.next(); push!(Token::Colon); }
            '#' => { chars.next(); push!(Token::Hash); }

            other => {
                return Err(QbError::Lex {
                    line,
                    msg: format!("unexpected character: {other:?}"),
                }.into());
            }
        }
    }

    // Ensure the stream ends with Newline + Eof so the parser's peek()
    // always has something to match.
    if out.last().map(|s: &Spanned| s.token != Token::Newline).unwrap_or(false) {
        push!(Token::Newline);
    }
    push!(Token::Eof);

    Ok(out)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Consume an identifier (letters, digits, underscore).
fn lex_ident(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut s = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_alphanumeric() || c == '_' {
            s.push(c);
            chars.next();
        } else {
            break;
        }
    }
    s
}

/// Consume an integer or float literal.
/// Handles: 42  3.14  1E-5  .5  42!  42#  42%
fn lex_number(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    line: u32,
) -> Result<Token> {
    let mut s      = String::new();
    let mut is_float = false;

    // Integer part
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() { s.push(c); chars.next(); }
        else { break; }
    }

    // Fractional part
    if chars.peek() == Some(&'.') {
        // Look ahead: if next char after '.' is also a digit (or end), it's a float.
        // A bare '.' with no digit before or after is the Dot punctuation token,
        // but we only enter lex_number when the first char was a digit, so this
        // is always a decimal point here.
        is_float = true;
        s.push('.');
        chars.next();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() { s.push(c); chars.next(); }
            else { break; }
        }
    }

    // Exponent part
    if matches!(chars.peek(), Some(&'E') | Some(&'e') | Some(&'D') | Some(&'d')) {
        is_float = true;
        s.push('E'); // normalize to E
        chars.next();
        if matches!(chars.peek(), Some(&'+') | Some(&'-')) {
            s.push(chars.next().unwrap());
        }
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() { s.push(c); chars.next(); }
            else { break; }
        }
    }

    // Optional type suffix — consume and use to decide token type
    match chars.peek() {
        Some(&'!') => { chars.next(); /* single — treat as float */ is_float = true; }
        Some(&'#') => { chars.next(); /* double — treat as float */ is_float = true; }
        Some(&'%') => {
            chars.next(); // integer suffix
            let n = s.parse::<i32>().map_err(|_| QbError::Lex {
                line,
                msg: format!("integer literal out of range: {s}"),
            })?;
            return Ok(Token::IntLit(n));
        }
        _ => {}
    }

    if is_float {
        let v = s.parse::<f64>().map_err(|_| QbError::Lex {
            line,
            msg: format!("bad float literal: {s}"),
        })?;
        Ok(Token::FloatLit(v))
    } else {
        // Try i32 first; if it overflows, store as float (QB LONG range)
        match s.parse::<i32>() {
            Ok(n)  => Ok(Token::IntLit(n)),
            Err(_) => {
                let v = s.parse::<f64>().map_err(|_| QbError::Lex {
                    line,
                    msg: format!("numeric literal out of range: {s}"),
                })?;
                Ok(Token::FloatLit(v))
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Token> {
        tokenize(src).unwrap()
            .into_iter()
            .map(|s| s.token)
            .filter(|t| !matches!(t, Token::Newline | Token::Eof))
            .collect()
    }

    #[test]
    fn keywords_are_case_insensitive() {
        let t = toks("print PRINT Print");
        assert!(t.iter().all(|t| *t == Token::Print));
    }

    #[test]
    fn sigil_identifiers() {
        let t = toks(r#"A$ B% C! D#"#);
        assert_eq!(t[0], Token::IdentStr("A".into()));
        assert_eq!(t[1], Token::IdentInt("B".into()));
        assert_eq!(t[2], Token::IdentSng("C".into()));
        assert_eq!(t[3], Token::IdentDbl("D".into()));
    }

    #[test]
    fn string_literal_double_quote_escape() {
        let t = toks(r#""hello ""world""#);
        assert_eq!(t[0], Token::StrLit(r#"hello "world"#.into()));
    }

    #[test]
    fn numeric_literals() {
        let t = toks("42 3.14 1E2 &H1F &O17");
        assert_eq!(t[0], Token::IntLit(42));
        assert_eq!(t[1], Token::FloatLit(3.14));
        assert_eq!(t[2], Token::FloatLit(100.0));
        assert_eq!(t[3], Token::IntLit(31));   // &H1F = 31
        assert_eq!(t[4], Token::IntLit(15));   // &O17 = 15
    }

    #[test]
    fn comment_rem_skipped() {
        // REM after a colon is in statement position — treated as a comment.
        let t = toks("PRINT 1: REM this is a comment\nPRINT 2");
        assert_eq!(t, vec![Token::Print, Token::IntLit(1), Token::Colon, Token::Print, Token::IntLit(2)]);
        // REM at the start of a line is also a comment.
        let t2 = toks("REM full line comment\nPRINT 2");
        assert_eq!(t2, vec![Token::Print, Token::IntLit(2)]);
    }

    #[test]
    fn comment_apostrophe_skipped() {
        let t = toks("PRINT 1 ' this too\nPRINT 2");
        assert_eq!(t, vec![Token::Print, Token::IntLit(1), Token::Print, Token::IntLit(2)]);
    }

    #[test]
    fn comparison_operators() {
        let t = toks("<> <= >=");
        assert_eq!(t, vec![Token::Ne, Token::Le, Token::Ge]);
    }

    #[test]
    fn multi_stmt_colon() {
        let t = toks("A = 1 : B = 2");
        assert!(t.contains(&Token::Colon));
    }
}
