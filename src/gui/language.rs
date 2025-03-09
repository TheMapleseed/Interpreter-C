use lazy_static::lazy_static;
use std::collections::HashMap;
use monaco_editor::languages::{
    LanguageConfiguration, TokensProvider, IMonarchLanguage,
    MonarchLanguageConfiguration, TokenType,
};

lazy_static! {
    // C Language Keywords
    pub static ref C_KEYWORDS: Vec<&'static str> = vec![
        // Storage class specifiers
        "auto", "extern", "register", "static", "typedef",
        
        // Type qualifiers
        "const", "volatile", "restrict",
        
        // Type specifiers
        "void", "char", "short", "int", "long", "float", "double",
        "signed", "unsigned", "_Bool", "_Complex", "_Imaginary",
        
        // Struct/Union
        "struct", "union", "enum",
        
        // Control flow
        "if", "else", "switch", "case", "default",
        "while", "do", "for", "break", "continue",
        "return", "goto",
        
        // Other
        "sizeof", "_Alignof", "_Atomic", "inline",
    ];

    // C Operators
    pub static ref C_OPERATORS: Vec<&'static str> = vec![
        "+", "-", "*", "/", "%",
        "++", "--",
        "==", "!=", "<", ">", "<=", ">=",
        "&&", "||", "!",
        "&", "|", "^", "~", "<<", ">>",
        "=", "+=", "-=", "*=", "/=", "%=",
        "&=", "|=", "^=", "<<=", ">>=",
        "->", ".",
        "?", ":",
    ];

    // C Preprocessor Directives
    pub static ref C_PREPROCESSOR: Vec<&'static str> = vec![
        "#include", "#define", "#undef",
        "#if", "#ifdef", "#ifndef", "#else", "#elif", "#endif",
        "#line", "#error", "#pragma",
    ];
}

/// Monaco editor language configuration for C
pub struct CLanguageSupport {
    config: LanguageConfiguration,
    tokens: CTokensProvider,
}

impl CLanguageSupport {
    pub fn new() -> Self {
        CLanguageSupport {
            config: create_language_config(),
            tokens: CTokensProvider::new(),
        }
    }

    pub fn register(&self, editor: &monaco_editor::Editor) -> Result<(), GuiError> {
        editor.register_language("c", self.config.clone())?;
        editor.register_tokens_provider("c", self.tokens.clone())?;
        Ok(())
    }
}

/// Token definitions for C syntax highlighting
#[derive(Clone)]
struct CTokensProvider {
    monarch: MonarchLanguageConfiguration,
}

impl CTokensProvider {
    fn new() -> Self {
        CTokensProvider {
            monarch: MonarchLanguageConfiguration {
                tokenizer: create_tokenizer_rules(),
                ..Default::default()
            },
        }
    }
}

fn create_tokenizer_rules() -> HashMap<String, Vec<TokenRule>> {
    let mut rules = HashMap::new();

    // Comments
    rules.insert("comments".to_string(), vec![
        TokenRule::new(r"//.*$", "comment.line"),
        TokenRule::new(r"/\*", "comment.block", "comment"),
    ]);

    // Strings
    rules.insert("strings".to_string(), vec![
        TokenRule::new(r#""([^"\\]|\\.)*""#, "string"),
        TokenRule::new(r"'([^'\\]|\\.)*'", "string"),
    ]);

    // Numbers
    rules.insert("numbers".to_string(), vec![
        TokenRule::new(r"\b0[xX][0-9a-fA-F]+\b", "number.hex"),
        TokenRule::new(r"\b0[0-7]+\b", "number.octal"),
        TokenRule::new(r"\b\d*\.\d+([eE][-+]?\d+)?\b", "number.float"),
        TokenRule::new(r"\b\d+\b", "number"),
    ]);

    // Preprocessor
    rules.insert("preprocessor".to_string(), vec![
        TokenRule::new(r"^[ \t]*#\w+", "preprocessor"),
    ]);

    // Keywords
    rules.insert("keywords".to_string(), vec![
        TokenRule::new(
            &C_KEYWORDS.iter()
                .map(|k| regex::escape(k))
                .collect::<Vec<_>>()
                .join("|"),
            "keyword"
        ),
    ]);

    rules
}

/// Theme configuration for C syntax highlighting
pub struct CSyntaxTheme {
    rules: Vec<ThemeRule>,
}

impl CSyntaxTheme {
    pub fn cursor_dark() -> Self {
        CSyntaxTheme {
            rules: vec![
                ThemeRule {
                    token: "keyword".to_string(),
                    foreground: "#569CD6".to_string(),
                    font_style: "bold".to_string(),
                },
                ThemeRule {
                    token: "comment".to_string(),
                    foreground: "#6A9955".to_string(),
                    font_style: "italic".to_string(),
                },
                ThemeRule {
                    token: "string".to_string(),
                    foreground: "#CE9178".to_string(),
                    font_style: "normal".to_string(),
                },
                ThemeRule {
                    token: "number".to_string(),
                    foreground: "#B5CEA8".to_string(),
                    font_style: "normal".to_string(),
                },
                ThemeRule {
                    token: "type".to_string(),
                    foreground: "#4EC9B0".to_string(),
                    font_style: "normal".to_string(),
                },
                ThemeRule {
                    token: "preprocessor".to_string(),
                    foreground: "#C586C0".to_string(),
                    font_style: "normal".to_string(),
                },
                // Additional syntax highlighting rules
                ThemeRule {
                    token: "function".to_string(),
                    foreground: "#DCDCAA".to_string(),
                    font_style: "normal".to_string(),
                },
                ThemeRule {
                    token: "variable".to_string(),
                    foreground: "#9CDCFE".to_string(),
                    font_style: "normal".to_string(),
                },
            ],
        }
    }
}

/// Special handling for GCC/Clang diagnostic highlighting
pub struct DiagnosticHighlighter {
    error_decorations: Vec<ErrorDecoration>,
    warning_decorations: Vec<WarningDecoration>,
}

impl DiagnosticHighlighter {
    pub fn highlight_error(
        &mut self,
        editor: &monaco_editor::Editor,
        error: &CompilerError
    ) -> Result<(), GuiError> {
        let decoration = ErrorDecoration {
            range: error.location.to_range(),
            message: error.message.clone(),
            severity: error.severity,
        };

        editor.add_decoration(&decoration)?;
        self.error_decorations.push(decoration);
        Ok(())
    }

    pub fn highlight_warning(
        &mut self,
        editor: &monaco_editor::Editor,
        warning: &CompilerWarning
    ) -> Result<(), GuiError> {
        let decoration = WarningDecoration {
            range: warning.location.to_range(),
            message: warning.message.clone(),
            code: warning.code.clone(),
        };

        editor.add_decoration(&decoration)?;
        self.warning_decorations.push(decoration);
        Ok(())
    }
}

// Example usage:
/*
fn setup_editor(editor: &monaco_editor::Editor) -> Result<(), GuiError> {
    let c_language = CLanguageSupport::new();
    c_language.register(editor)?;

    let theme = CSyntaxTheme::cursor_dark();
    editor.set_theme(&theme)?;

    let highlighter = DiagnosticHighlighter::new();
    
    // Example error highlighting
    highlighter.highlight_error(editor, &CompilerError {
        message: "undefined reference to 'foo'".to_string(),
        location: SourceLocation { line: 10, column: 5, ..Default::default() },
        severity: ErrorSeverity::Error,
    })?;

    Ok(())
}
*/ 
