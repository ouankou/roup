use crate::host::ast::{
    CIntegerSuffix, CIntegerWidth, CRealSuffix, CharacterEncoding, CharacterLiteral, FortranKind,
    HostLanguage, Identifier, IntegerBase, IntegerLiteral, IntegerSuffix, RealExponent,
    RealExponentKind, RealLiteral, RealSuffix, StringLiteral,
};
use crate::source::{SourceError, Span};
use crate::version::{CStandard, CppStandard, FortranStandard, HostLanguageProfile};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(Identifier),
    ReservedKeyword(Identifier),
    Integer(IntegerLiteral),
    Real(RealLiteral),
    Character(CharacterLiteral),
    String(StringLiteral),
    Boolean(bool),
    NullPointer,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Comma,
    Question,
    Colon,
    Scope,
    Dot,
    Percent,
    Arrow,
    Plus,
    Minus,
    Star,
    Slash,
    Power,
    Concat,
    Remainder,
    LogicalNot,
    BitwiseNot,
    Ampersand,
    Pipe,
    Caret,
    LogicalAnd,
    LogicalOr,
    LogicalEqv,
    LogicalNeqv,
    FortranDefinedOperator(Identifier),
    Equal,
    EqualEqual,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    ShiftLeft,
    ShiftRight,
    PlusPlus,
    MinusMinus,
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    RemainderEqual,
    AmpersandEqual,
    PipeEqual,
    CaretEqual,
    ShiftLeftEqual,
    ShiftRightEqual,
    End,
}

impl TokenKind {
    pub fn description(&self) -> &'static str {
        match self {
            TokenKind::Identifier(_) => "identifier",
            TokenKind::ReservedKeyword(_) => "reserved keyword",
            TokenKind::Integer(_) => "integer literal",
            TokenKind::Real(_) => "real literal",
            TokenKind::Character(_) => "character literal",
            TokenKind::String(_) => "string literal",
            TokenKind::Boolean(_) => "boolean literal",
            TokenKind::NullPointer => "null pointer literal",
            TokenKind::LeftParen => "`(`",
            TokenKind::RightParen => "`)`",
            TokenKind::LeftBracket => "`[`",
            TokenKind::RightBracket => "`]`",
            TokenKind::Comma => "`,`",
            TokenKind::Question => "`?`",
            TokenKind::Colon => "`:`",
            TokenKind::Scope => "`::`",
            TokenKind::Dot => "`.`",
            TokenKind::Percent => "`%`",
            TokenKind::Arrow => "`->`",
            TokenKind::Plus => "`+`",
            TokenKind::Minus => "`-`",
            TokenKind::Star => "`*`",
            TokenKind::Slash => "`/`",
            TokenKind::Power => "`**`",
            TokenKind::Concat => "`//`",
            TokenKind::Remainder => "`%`",
            TokenKind::LogicalNot => "logical not",
            TokenKind::BitwiseNot => "bitwise not",
            TokenKind::Ampersand => "`&`",
            TokenKind::Pipe => "`|`",
            TokenKind::Caret => "`^`",
            TokenKind::LogicalAnd => "logical and",
            TokenKind::LogicalOr => "logical or",
            TokenKind::LogicalEqv => "logical equivalence",
            TokenKind::LogicalNeqv => "logical inequivalence",
            TokenKind::FortranDefinedOperator(_) => "Fortran defined operator",
            TokenKind::Equal => "`=`",
            TokenKind::EqualEqual => "`==`",
            TokenKind::NotEqual => "not equal",
            TokenKind::Less => "`<`",
            TokenKind::LessEqual => "`<=`",
            TokenKind::Greater => "`>`",
            TokenKind::GreaterEqual => "`>=`",
            TokenKind::ShiftLeft => "`<<`",
            TokenKind::ShiftRight => "`>>`",
            TokenKind::PlusPlus => "`++`",
            TokenKind::MinusMinus => "`--`",
            TokenKind::PlusEqual => "`+=`",
            TokenKind::MinusEqual => "`-=`",
            TokenKind::StarEqual => "`*=`",
            TokenKind::SlashEqual => "`/=`",
            TokenKind::RemainderEqual => "`%=`",
            TokenKind::AmpersandEqual => "`&=`",
            TokenKind::PipeEqual => "`|=`",
            TokenKind::CaretEqual => "`^=`",
            TokenKind::ShiftLeftEqual => "`<<=`",
            TokenKind::ShiftRightEqual => "`>>=`",
            TokenKind::End => "end of input",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub span: Span,
    pub kind: LexErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexErrorKind {
    UnexpectedCharacter(char),
    UnterminatedBlockComment,
    UnterminatedLiteral,
    InvalidEscape(String),
    InvalidCharacterLiteral,
    InvalidNumber(String),
    IntegerOverflow,
    UnsupportedLiteral(&'static str),
    UnsupportedOperator(&'static str),
    NotAvailableInProfile(&'static str),
    UnsupportedKeyword,
    InvalidSpan(SourceError),
    InternalInvariant(&'static str),
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl fmt::Display for LexErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexErrorKind::UnexpectedCharacter(ch) => write!(f, "unexpected character `{ch}`"),
            LexErrorKind::UnterminatedBlockComment => f.write_str("unterminated block comment"),
            LexErrorKind::UnterminatedLiteral => f.write_str("unterminated literal"),
            LexErrorKind::InvalidEscape(value) => write!(f, "invalid escape sequence `{value}`"),
            LexErrorKind::InvalidCharacterLiteral => {
                f.write_str("character literal must contain exactly one character")
            }
            LexErrorKind::InvalidNumber(value) => write!(f, "invalid number `{value}`"),
            LexErrorKind::IntegerOverflow => f.write_str("integer literal exceeds 128 bits"),
            LexErrorKind::UnsupportedLiteral(feature) => {
                write!(f, "unsupported literal form: {feature}")
            }
            LexErrorKind::UnsupportedOperator(feature) => {
                write!(f, "unsupported operator: {feature}")
            }
            LexErrorKind::NotAvailableInProfile(feature) => {
                write!(
                    f,
                    "{feature} is not available in the configured host language standard"
                )
            }
            LexErrorKind::UnsupportedKeyword => {
                f.write_str("keyword is outside the supported expression subset")
            }
            LexErrorKind::InvalidSpan(error) => write!(f, "invalid lexer source span: {error}"),
            LexErrorKind::InternalInvariant(message) => {
                write!(f, "lexer invariant failed: {message}")
            }
        }
    }
}

impl std::error::Error for LexError {}

/// Byte-offset lexer. Every cursor update advances by `char::len_utf8`, and
/// every emitted span is validated by `crate::source::Span`.
pub struct Lexer<'a> {
    source: &'a str,
    profile: HostLanguageProfile,
    language: HostLanguage,
    offset: usize,
    allow_reserved_words: bool,
    tokenize_reserved_words: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, language: HostLanguage) -> Self {
        Self::with_profile(source, HostLanguageProfile::latest(language))
    }

    pub fn with_profile(source: &'a str, profile: HostLanguageProfile) -> Self {
        Self {
            source,
            profile,
            language: profile.language(),
            offset: 0,
            allow_reserved_words: false,
            tokenize_reserved_words: false,
        }
    }

    /// Create a lexer for host-language syntax whose grammar, rather than the
    /// expression lexer, classifies reserved words. This is used by the typed
    /// type-name parser: words such as `int`, `struct`, and `typename` are
    /// meaningful tokens there instead of unsupported expression keywords.
    pub(crate) fn for_type_name_with_profile(
        source: &'a str,
        profile: HostLanguageProfile,
    ) -> Self {
        Self {
            source,
            profile,
            language: profile.language(),
            offset: 0,
            allow_reserved_words: true,
            tokenize_reserved_words: false,
        }
    }

    /// Create a compatibility lexer in which host keywords may occupy name
    /// positions after the strict expression grammar has declined the input.
    pub(crate) fn for_extension_expression_with_profile(
        source: &'a str,
        profile: HostLanguageProfile,
    ) -> Self {
        Self {
            source,
            profile,
            language: profile.language(),
            offset: 0,
            allow_reserved_words: true,
            tokenize_reserved_words: false,
        }
    }

    /// Create a lexer for the strict expression parser. Reserved words are
    /// emitted as distinct typed tokens so a C++ template-argument grammar can
    /// consume type keywords while ordinary expression positions still reject
    /// them.
    pub(crate) fn for_expression_with_profile(
        source: &'a str,
        profile: HostLanguageProfile,
    ) -> Self {
        Self {
            source,
            profile,
            language: profile.language(),
            offset: 0,
            allow_reserved_words: false,
            tokenize_reserved_words: true,
        }
    }

    pub fn source(&self) -> &'a str {
        self.source
    }

    pub fn language(&self) -> HostLanguage {
        self.language
    }

    pub const fn profile(&self) -> HostLanguageProfile {
        self.profile
    }

    fn supports_binary_integer(&self) -> bool {
        matches!(
            self.profile,
            HostLanguageProfile::C(CStandard::C23)
                | HostLanguageProfile::Cpp(
                    CppStandard::Cpp14
                        | CppStandard::Cpp17
                        | CppStandard::Cpp20
                        | CppStandard::Cpp23
                )
        )
    }

    fn supports_digit_separator(&self) -> bool {
        self.supports_binary_integer()
    }

    fn supports_null_pointer_literal(&self) -> bool {
        matches!(
            self.profile,
            HostLanguageProfile::C(CStandard::C23)
                | HostLanguageProfile::Cpp(
                    CppStandard::Cpp11
                        | CppStandard::Cpp14
                        | CppStandard::Cpp17
                        | CppStandard::Cpp20
                        | CppStandard::Cpp23
                )
        )
    }

    fn supports_long_long_suffix(&self) -> bool {
        matches!(
            self.profile,
            HostLanguageProfile::C(
                CStandard::C99 | CStandard::C11 | CStandard::C18 | CStandard::C23
            ) | HostLanguageProfile::Cpp(
                CppStandard::Cpp11
                    | CppStandard::Cpp14
                    | CppStandard::Cpp17
                    | CppStandard::Cpp20
                    | CppStandard::Cpp23
            )
        )
    }

    fn supports_size_suffix(&self) -> bool {
        matches!(
            self.profile,
            HostLanguageProfile::C(CStandard::C23) | HostLanguageProfile::Cpp(CppStandard::Cpp23)
        )
    }

    fn supports_fortran_kind_suffix(&self) -> bool {
        !matches!(
            self.profile,
            HostLanguageProfile::Fortran(FortranStandard::Fortran77)
        )
    }

    fn supports_line_comment(&self) -> bool {
        !matches!(self.profile, HostLanguageProfile::C(CStandard::C89))
    }

    fn literal_encoding_available(&self, encoding: CharacterEncoding, quote: char) -> bool {
        matches!(
            (self.profile, encoding, quote),
            (_, CharacterEncoding::Ordinary | CharacterEncoding::Wide, _)
                | (
                    HostLanguageProfile::Fortran(_),
                    CharacterEncoding::Fortran,
                    _
                )
                | (
                    HostLanguageProfile::C(CStandard::C11 | CStandard::C18 | CStandard::C23),
                    CharacterEncoding::Utf16 | CharacterEncoding::Utf32,
                    _,
                )
                | (
                    HostLanguageProfile::C(CStandard::C11 | CStandard::C18 | CStandard::C23),
                    CharacterEncoding::Utf8,
                    '"',
                )
                | (
                    HostLanguageProfile::C(CStandard::C23),
                    CharacterEncoding::Utf8,
                    '\''
                )
                | (
                    HostLanguageProfile::Cpp(
                        CppStandard::Cpp11
                            | CppStandard::Cpp14
                            | CppStandard::Cpp17
                            | CppStandard::Cpp20
                            | CppStandard::Cpp23,
                    ),
                    CharacterEncoding::Utf16 | CharacterEncoding::Utf32,
                    _,
                )
                | (
                    HostLanguageProfile::Cpp(
                        CppStandard::Cpp11
                            | CppStandard::Cpp14
                            | CppStandard::Cpp17
                            | CppStandard::Cpp20
                            | CppStandard::Cpp23,
                    ),
                    CharacterEncoding::Utf8,
                    '"',
                )
                | (
                    HostLanguageProfile::Cpp(
                        CppStandard::Cpp17 | CppStandard::Cpp20 | CppStandard::Cpp23,
                    ),
                    CharacterEncoding::Utf8,
                    '\'',
                )
        )
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let done = matches!(token.kind, TokenKind::End);
            tokens.push(token);
            if done {
                return Ok(tokens);
            }
        }
    }

    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_trivia()?;
        let start = self.offset;
        let Some(ch) = self.peek_char() else {
            return Ok(Token {
                kind: TokenKind::End,
                span: self.span(start, start)?,
            });
        };

        if let Some((encoding, quote, prefix_len)) = self.literal_prefix() {
            if !self.literal_encoding_available(encoding, quote) {
                return Err(self.error(
                    start,
                    start + prefix_len.max(1),
                    LexErrorKind::NotAvailableInProfile("encoded character or string literal"),
                ));
            }
            self.offset += prefix_len;
            return self.lex_quoted(start, encoding, quote);
        }

        if ch.is_ascii_digit()
            || (ch == '.'
                && self
                    .peek_nth_char(1)
                    .is_some_and(|next| next.is_ascii_digit()))
        {
            return self.lex_number();
        }

        if is_identifier_start(ch) {
            return self.lex_identifier_or_keyword();
        }

        if ch == '.'
            && self.language == HostLanguage::Fortran
            && !self.starts_with("..")
            && let Some(token) = self.lex_fortran_dotted()?
        {
            return Ok(token);
        }

        self.lex_symbol()
    }

    fn skip_trivia(&mut self) -> Result<(), LexError> {
        loop {
            while self.peek_char().is_some_and(char::is_whitespace) {
                self.advance_char();
            }

            if self.language != HostLanguage::Fortran && self.starts_with("/*") {
                let start = self.offset;
                self.offset += 2;
                let Some(relative_end) = self.source[self.offset..].find("*/") else {
                    self.offset = self.source.len();
                    return Err(self.error(
                        start,
                        self.offset,
                        LexErrorKind::UnterminatedBlockComment,
                    ));
                };
                self.offset += relative_end + 2;
                continue;
            }

            if self.language != HostLanguage::Fortran && self.starts_with("//") {
                if !self.supports_line_comment() {
                    return Err(self.error(
                        self.offset,
                        self.offset + 2,
                        LexErrorKind::NotAvailableInProfile("C++-style line comment"),
                    ));
                }
                self.offset += 2;
                while self.peek_char().is_some_and(|ch| ch != '\n' && ch != '\r') {
                    self.advance_char();
                }
                continue;
            }

            if self.language == HostLanguage::Fortran && self.starts_with("!") {
                self.offset += 1;
                while self.peek_char().is_some_and(|ch| ch != '\n' && ch != '\r') {
                    self.advance_char();
                }
                continue;
            }
            return Ok(());
        }
    }

    fn lex_identifier_or_keyword(&mut self) -> Result<Token, LexError> {
        let start = self.offset;
        self.advance_char();
        while self.peek_char().is_some_and(is_identifier_continue) {
            self.advance_char();
        }
        let text = &self.source[start..self.offset];
        // C and C++ keywords are case-sensitive.  Fortran identifiers remain
        // identifiers here; only its dotted operators are case-insensitive.
        let kind = match (self.language, text) {
            (HostLanguage::C, "true")
                if matches!(self.profile, HostLanguageProfile::C(CStandard::C23)) =>
            {
                TokenKind::Boolean(true)
            }
            (HostLanguage::C, "false")
                if matches!(self.profile, HostLanguageProfile::C(CStandard::C23)) =>
            {
                TokenKind::Boolean(false)
            }
            (HostLanguage::Cpp, "true") => TokenKind::Boolean(true),
            (HostLanguage::Cpp, "false") => TokenKind::Boolean(false),
            (HostLanguage::C | HostLanguage::Cpp, "nullptr")
                if self.supports_null_pointer_literal() =>
            {
                TokenKind::NullPointer
            }
            (HostLanguage::Cpp, "and") => TokenKind::LogicalAnd,
            (HostLanguage::Cpp, "or") => TokenKind::LogicalOr,
            (HostLanguage::Cpp, "not") => TokenKind::LogicalNot,
            (HostLanguage::Cpp, "bitand") => TokenKind::Ampersand,
            (HostLanguage::Cpp, "bitor") => TokenKind::Pipe,
            (HostLanguage::Cpp, "xor") => TokenKind::Caret,
            (HostLanguage::Cpp, "compl") => TokenKind::BitwiseNot,
            (HostLanguage::Cpp, "not_eq") => TokenKind::NotEqual,
            (HostLanguage::Cpp, "and_eq") => TokenKind::AmpersandEqual,
            (HostLanguage::Cpp, "or_eq") => TokenKind::PipeEqual,
            (HostLanguage::Cpp, "xor_eq") => TokenKind::CaretEqual,
            _ if !self.allow_reserved_words && is_reserved_keyword(self.profile, text) => {
                if self.tokenize_reserved_words {
                    TokenKind::ReservedKeyword(Identifier::from_lexed(text))
                } else {
                    return Err(self.error(start, self.offset, LexErrorKind::UnsupportedKeyword));
                }
            }
            (HostLanguage::Fortran, _) => {
                TokenKind::Identifier(Identifier::from_lexed(&text.to_lowercase()))
            }
            _ => TokenKind::Identifier(Identifier::from_lexed(text)),
        };
        Ok(Token {
            kind,
            span: self.span(start, self.offset)?,
        })
    }

    fn lex_fortran_dotted(&mut self) -> Result<Option<Token>, LexError> {
        let start = self.offset;
        let Some(relative_end) = self.source[start + 1..].find('.') else {
            return Ok(None);
        };
        let end = start + 1 + relative_end + 1;
        let spelling = &self.source[start..end];
        let lower = spelling.to_ascii_lowercase();
        let kind = match lower.as_str() {
            ".true." => TokenKind::Boolean(true),
            ".false." => TokenKind::Boolean(false),
            ".not." => TokenKind::LogicalNot,
            ".and." => TokenKind::LogicalAnd,
            ".or." => TokenKind::LogicalOr,
            ".eqv." => TokenKind::LogicalEqv,
            ".neqv." => TokenKind::LogicalNeqv,
            ".eq." => TokenKind::EqualEqual,
            ".ne." => TokenKind::NotEqual,
            ".lt." => TokenKind::Less,
            ".le." => TokenKind::LessEqual,
            ".gt." => TokenKind::Greater,
            ".ge." => TokenKind::GreaterEqual,
            _ => TokenKind::FortranDefinedOperator(
                Identifier::new(&lower[1..lower.len() - 1]).map_err(|_| {
                    self.error(
                        start,
                        end,
                        LexErrorKind::UnsupportedOperator("invalid Fortran defined operator"),
                    )
                })?,
            ),
        };
        self.offset = end;
        Ok(Some(Token {
            kind,
            span: self.span(start, end)?,
        }))
    }

    fn lex_number(&mut self) -> Result<Token, LexError> {
        let start = self.offset;
        if self.starts_with("0x") || self.starts_with("0X") {
            if self.language == HostLanguage::Fortran {
                return Err(self.error(
                    start,
                    start + 2,
                    LexErrorKind::UnsupportedLiteral("C-style hexadecimal integer in Fortran"),
                ));
            }
            return self.lex_based_integer(start, IntegerBase::Hexadecimal, 2, 16);
        }
        if self.starts_with("0b") || self.starts_with("0B") {
            if self.language == HostLanguage::Fortran {
                return Err(self.error(
                    start,
                    start + 2,
                    LexErrorKind::UnsupportedLiteral("C-style binary integer in Fortran"),
                ));
            }
            if !self.supports_binary_integer() {
                return Err(self.error(
                    start,
                    start + 2,
                    LexErrorKind::NotAvailableInProfile("binary integer literal"),
                ));
            }
            return self.lex_based_integer(start, IntegerBase::Binary, 2, 2);
        }

        let leading_dot = self.peek_char() == Some('.');
        let whole = if leading_dot {
            String::new()
        } else {
            self.consume_decimal_digits(true)?
        };

        let mut fraction = None;
        if self.peek_char() == Some('.') && !self.starts_with("..") {
            self.advance_char();
            fraction = Some(self.consume_decimal_digits(false)?);
        }

        let mut exponent = None;
        if let Some(ch) = self.peek_char() {
            let exponent_kind = match ch {
                'e' | 'E' => Some(RealExponentKind::E),
                'd' | 'D' if self.language == HostLanguage::Fortran => Some(RealExponentKind::D),
                _ => None,
            };
            if let Some(kind) = exponent_kind {
                self.advance_char();
                let negative = match self.peek_char() {
                    Some('+') => {
                        self.advance_char();
                        false
                    }
                    Some('-') => {
                        self.advance_char();
                        true
                    }
                    _ => false,
                };
                let digits = self.consume_decimal_digits(true)?;
                let magnitude = digits.parse::<u32>().map_err(|_| {
                    self.error(
                        start,
                        self.offset,
                        LexErrorKind::InvalidNumber(digits.clone()),
                    )
                })?;
                exponent = Some(RealExponent {
                    kind,
                    negative,
                    magnitude,
                });
            }
        }

        let is_real = leading_dot || fraction.is_some() || exponent.is_some();
        if is_real {
            let suffix = self.lex_real_suffix(start)?;
            self.reject_numeric_tail(start)?;
            let fraction = fraction.as_deref().unwrap_or("");
            let whole = if whole.is_empty() { "0" } else { &whole };
            let coefficient_text = format!("{whole}{fraction}");
            let coefficient = coefficient_text.parse::<u128>().map_err(|error| {
                let kind = if error.kind() == &std::num::IntErrorKind::PosOverflow {
                    LexErrorKind::IntegerOverflow
                } else {
                    LexErrorKind::InvalidNumber(self.source[start..self.offset].to_owned())
                };
                self.error(start, self.offset, kind)
            })?;
            let fractional_digits = u32::try_from(fraction.len())
                .map_err(|_| self.error(start, self.offset, LexErrorKind::IntegerOverflow))?;
            return Ok(Token {
                kind: TokenKind::Real(RealLiteral {
                    coefficient,
                    fractional_digits,
                    exponent,
                    suffix,
                }),
                span: self.span(start, self.offset)?,
            });
        }

        let (suffix, digits_end) = self.lex_integer_suffix(start)?;
        self.reject_numeric_tail(start)?;
        let digits = strip_digit_separators(&self.source[start..digits_end]);
        let base = if self.language != HostLanguage::Fortran
            && digits.len() > 1
            && digits.starts_with('0')
        {
            IntegerBase::Octal
        } else {
            IntegerBase::Decimal
        };
        let radix = if base == IntegerBase::Octal { 8 } else { 10 };
        let value = u128::from_str_radix(&digits, radix).map_err(|err| {
            let kind = if err.kind() == &std::num::IntErrorKind::PosOverflow {
                LexErrorKind::IntegerOverflow
            } else {
                LexErrorKind::InvalidNumber(self.source[start..self.offset].to_owned())
            };
            self.error(start, self.offset, kind)
        })?;
        Ok(Token {
            kind: TokenKind::Integer(IntegerLiteral {
                base,
                value,
                suffix,
            }),
            span: self.span(start, self.offset)?,
        })
    }

    fn lex_based_integer(
        &mut self,
        start: usize,
        base: IntegerBase,
        prefix_len: usize,
        radix: u32,
    ) -> Result<Token, LexError> {
        self.offset += prefix_len;
        let digits_start = self.offset;
        let mut separator_allowed = false;
        while let Some(ch) = self.peek_char() {
            if ch.is_digit(radix) {
                separator_allowed = true;
                self.advance_char();
            } else if self.language != HostLanguage::Fortran && ch == '\'' {
                if !self.supports_digit_separator() {
                    self.advance_char();
                    return Err(self.error(
                        start,
                        self.offset,
                        LexErrorKind::NotAvailableInProfile("digit separator"),
                    ));
                }
                let next_is_digit = self
                    .peek_nth_char(1)
                    .is_some_and(|next| next.is_digit(radix));
                if !separator_allowed || !next_is_digit {
                    self.advance_char();
                    return Err(self.error(
                        start,
                        self.offset,
                        LexErrorKind::InvalidNumber(self.source[start..self.offset].to_owned()),
                    ));
                }
                separator_allowed = false;
                self.advance_char();
            } else {
                break;
            }
        }
        if self.offset == digits_start {
            return Err(self.error(
                start,
                self.offset,
                LexErrorKind::InvalidNumber(self.source[start..self.offset].to_owned()),
            ));
        }
        if self.peek_char() == Some('.')
            || self.peek_char().is_some_and(|ch| matches!(ch, 'p' | 'P'))
        {
            return Err(self.error(
                start,
                self.offset,
                LexErrorKind::UnsupportedLiteral("hexadecimal floating literal"),
            ));
        }
        let digits_end = self.offset;
        let (suffix, _) = self.lex_integer_suffix(start)?;
        self.reject_numeric_tail(start)?;
        let digits = strip_digit_separators(&self.source[digits_start..digits_end]);
        let value = u128::from_str_radix(&digits, radix).map_err(|err| {
            let kind = if err.kind() == &std::num::IntErrorKind::PosOverflow {
                LexErrorKind::IntegerOverflow
            } else {
                LexErrorKind::InvalidNumber(self.source[start..self.offset].to_owned())
            };
            self.error(start, self.offset, kind)
        })?;
        Ok(Token {
            kind: TokenKind::Integer(IntegerLiteral {
                base,
                value,
                suffix,
            }),
            span: self.span(start, self.offset)?,
        })
    }

    fn consume_decimal_digits(&mut self, require_one: bool) -> Result<String, LexError> {
        let start = self.offset;
        let mut digits = String::new();
        let mut separator_allowed = false;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() {
                digits.push(ch);
                separator_allowed = true;
                self.advance_char();
            } else if self.language != HostLanguage::Fortran && ch == '\'' {
                if !self.supports_digit_separator() {
                    self.advance_char();
                    return Err(self.error(
                        start,
                        self.offset,
                        LexErrorKind::NotAvailableInProfile("digit separator"),
                    ));
                }
                let next_is_digit = self
                    .peek_nth_char(1)
                    .is_some_and(|next| next.is_ascii_digit());
                if !separator_allowed || !next_is_digit {
                    self.advance_char();
                    return Err(self.error(
                        start,
                        self.offset,
                        LexErrorKind::InvalidNumber(self.source[start..self.offset].to_owned()),
                    ));
                }
                separator_allowed = false;
                self.advance_char();
            } else {
                break;
            }
        }
        if require_one && digits.is_empty() {
            return Err(self.error(
                start,
                self.offset,
                LexErrorKind::InvalidNumber(self.source[start..self.offset].to_owned()),
            ));
        }
        Ok(digits)
    }

    fn lex_integer_suffix(&mut self, start: usize) -> Result<(IntegerSuffix, usize), LexError> {
        let digits_end = self.offset;
        if self.language == HostLanguage::Fortran && self.peek_char() == Some('_') {
            if !self.supports_fortran_kind_suffix() {
                return Err(self.error(
                    start,
                    self.offset + 1,
                    LexErrorKind::NotAvailableInProfile("Fortran kind suffix"),
                ));
            }
            self.advance_char();
            let kind_start = self.offset;
            while self.peek_char().is_some_and(is_identifier_continue) {
                self.advance_char();
            }
            let kind_text = &self.source[kind_start..self.offset];
            if kind_text.is_empty() {
                return Err(self.error(
                    start,
                    self.offset,
                    LexErrorKind::InvalidNumber(self.source[start..self.offset].to_owned()),
                ));
            }
            let kind = if kind_text.chars().all(|ch| ch.is_ascii_digit()) {
                FortranKind::Numeric(kind_text.parse::<u32>().map_err(|_| {
                    self.error(
                        start,
                        self.offset,
                        LexErrorKind::InvalidNumber(kind_text.into()),
                    )
                })?)
            } else {
                FortranKind::Named(Identifier::new(kind_text).map_err(|_| {
                    self.error(
                        start,
                        self.offset,
                        LexErrorKind::InvalidNumber(kind_text.into()),
                    )
                })?)
            };
            return Ok((IntegerSuffix::Fortran(kind), digits_end));
        }

        if self.language == HostLanguage::Fortran {
            return Ok((IntegerSuffix::None, digits_end));
        }

        let suffix_start = self.offset;
        while self.peek_char().is_some_and(|ch| ch.is_ascii_alphabetic()) {
            self.advance_char();
        }
        let spelling = self.source[suffix_start..self.offset].to_ascii_lowercase();
        if spelling.is_empty() {
            return Ok((IntegerSuffix::None, digits_end));
        }
        let suffix = match spelling.as_str() {
            "u" => CIntegerSuffix {
                unsigned: true,
                ..CIntegerSuffix::default()
            },
            "l" => CIntegerSuffix {
                width: CIntegerWidth::Long,
                ..CIntegerSuffix::default()
            },
            "ul" | "lu" => CIntegerSuffix {
                unsigned: true,
                width: CIntegerWidth::Long,
                ..CIntegerSuffix::default()
            },
            "ll" => CIntegerSuffix {
                width: CIntegerWidth::LongLong,
                ..CIntegerSuffix::default()
            },
            "ull" | "llu" => CIntegerSuffix {
                unsigned: true,
                width: CIntegerWidth::LongLong,
                ..CIntegerSuffix::default()
            },
            "z" => CIntegerSuffix {
                size_t: true,
                ..CIntegerSuffix::default()
            },
            "uz" | "zu" => CIntegerSuffix {
                unsigned: true,
                size_t: true,
                ..CIntegerSuffix::default()
            },
            _ => {
                return Err(self.error(
                    suffix_start,
                    self.offset,
                    LexErrorKind::InvalidNumber(self.source[start..self.offset].to_owned()),
                ));
            }
        };
        if matches!(suffix.width, CIntegerWidth::LongLong) && !self.supports_long_long_suffix() {
            return Err(self.error(
                suffix_start,
                self.offset,
                LexErrorKind::NotAvailableInProfile("long long integer suffix"),
            ));
        }
        if suffix.size_t && !self.supports_size_suffix() {
            return Err(self.error(
                suffix_start,
                self.offset,
                LexErrorKind::NotAvailableInProfile("size integer suffix"),
            ));
        }
        Ok((IntegerSuffix::C(suffix), digits_end))
    }

    fn lex_real_suffix(&mut self, start: usize) -> Result<RealSuffix, LexError> {
        if self.language == HostLanguage::Fortran {
            let kind = if self.peek_char() == Some('_') {
                if !self.supports_fortran_kind_suffix() {
                    return Err(self.error(
                        start,
                        self.offset + 1,
                        LexErrorKind::NotAvailableInProfile("Fortran kind suffix"),
                    ));
                }
                self.advance_char();
                let kind_start = self.offset;
                while self.peek_char().is_some_and(is_identifier_continue) {
                    self.advance_char();
                }
                let text = &self.source[kind_start..self.offset];
                if text.is_empty() {
                    return Err(self.error(
                        start,
                        self.offset,
                        LexErrorKind::InvalidNumber(self.source[start..self.offset].to_owned()),
                    ));
                }
                Some(if text.chars().all(|ch| ch.is_ascii_digit()) {
                    FortranKind::Numeric(text.parse::<u32>().map_err(|_| {
                        self.error(start, self.offset, LexErrorKind::InvalidNumber(text.into()))
                    })?)
                } else {
                    FortranKind::Named(Identifier::new(text).map_err(|_| {
                        self.error(start, self.offset, LexErrorKind::InvalidNumber(text.into()))
                    })?)
                })
            } else {
                None
            };
            return Ok(RealSuffix::Fortran(kind));
        }

        let suffix = match self.peek_char() {
            Some('f' | 'F') => {
                self.advance_char();
                CRealSuffix::Float
            }
            Some('l' | 'L') => {
                self.advance_char();
                CRealSuffix::LongDouble
            }
            _ => CRealSuffix::Double,
        };
        Ok(RealSuffix::C(suffix))
    }

    fn reject_numeric_tail(&self, start: usize) -> Result<(), LexError> {
        if self.peek_char().is_some_and(is_identifier_continue) {
            return Err(self.error(
                start,
                self.offset + self.peek_char().map_or(0, char::len_utf8),
                LexErrorKind::InvalidNumber(self.source[start..self.offset].to_owned()),
            ));
        }
        Ok(())
    }

    fn literal_prefix(&self) -> Option<(CharacterEncoding, char, usize)> {
        if self.language == HostLanguage::Fortran {
            let quote = self.peek_char()?;
            return matches!(quote, '\'' | '"').then_some((CharacterEncoding::Fortran, quote, 0));
        }
        for (prefix, encoding) in [
            ("u8", CharacterEncoding::Utf8),
            ("u", CharacterEncoding::Utf16),
            ("U", CharacterEncoding::Utf32),
            ("L", CharacterEncoding::Wide),
        ] {
            if self.starts_with(prefix) {
                let after = self.offset + prefix.len();
                if let Some(quote) = self.source[after..].chars().next()
                    && matches!(quote, '\'' | '"')
                {
                    return Some((encoding, quote, prefix.len()));
                }
            }
        }
        let quote = self.peek_char()?;
        matches!(quote, '\'' | '"').then_some((CharacterEncoding::Ordinary, quote, 0))
    }

    fn lex_quoted(
        &mut self,
        start: usize,
        encoding: CharacterEncoding,
        quote: char,
    ) -> Result<Token, LexError> {
        if self.peek_char() != Some(quote) {
            return Err(self.error(
                start,
                self.offset,
                LexErrorKind::InternalInvariant("quoted literal did not start at its delimiter"),
            ));
        }
        self.advance_char();
        let mut value = String::new();
        let mut code_units = Vec::new();
        loop {
            let Some(ch) = self.peek_char() else {
                return Err(self.error(start, self.offset, LexErrorKind::UnterminatedLiteral));
            };
            if ch == quote {
                self.advance_char();
                if self.language == HostLanguage::Fortran && self.peek_char() == Some(quote) {
                    value.push(quote);
                    code_units.push(crate::host::LiteralCodeUnit::Scalar(quote));
                    self.advance_char();
                    continue;
                }
                break;
            }
            if matches!(ch, '\n' | '\r') {
                return Err(self.error(start, self.offset, LexErrorKind::UnterminatedLiteral));
            }
            if ch == '\\' && self.language != HostLanguage::Fortran {
                self.advance_char();
                let (character, code_unit) = self.lex_escape(start)?;
                value.push(character);
                code_units.push(code_unit);
            } else {
                value.push(ch);
                code_units.push(crate::host::LiteralCodeUnit::Scalar(ch));
                self.advance_char();
            }
        }

        let kind = if quote == '\'' && self.language != HostLanguage::Fortran {
            let Some(code_unit) = code_units.pop() else {
                return Err(self.error(start, self.offset, LexErrorKind::InvalidCharacterLiteral));
            };
            if !code_units.is_empty() {
                return Err(self.error(start, self.offset, LexErrorKind::InvalidCharacterLiteral));
            }
            let character = value.chars().next().ok_or_else(|| {
                self.error(start, self.offset, LexErrorKind::InvalidCharacterLiteral)
            })?;
            TokenKind::Character(CharacterLiteral {
                encoding,
                code_unit,
                value: character,
            })
        } else {
            TokenKind::String(StringLiteral {
                encoding,
                delimiter: if quote == '\'' {
                    crate::host::StringDelimiter::SingleQuote
                } else {
                    crate::host::StringDelimiter::DoubleQuote
                },
                code_units,
                value,
            })
        };
        Ok(Token {
            kind,
            span: self.span(start, self.offset)?,
        })
    }

    fn lex_escape(
        &mut self,
        literal_start: usize,
    ) -> Result<(char, crate::host::LiteralCodeUnit), LexError> {
        let escape_start = self.offset.saturating_sub(1);
        let Some(ch) = self.peek_char() else {
            return Err(self.error(
                literal_start,
                self.offset,
                LexErrorKind::UnterminatedLiteral,
            ));
        };
        self.advance_char();
        let simple = match ch {
            '\\' => Some('\\'),
            '\'' => Some('\''),
            '"' => Some('"'),
            'n' => Some('\n'),
            'r' => Some('\r'),
            't' => Some('\t'),
            'a' => Some('\u{7}'),
            'b' => Some('\u{8}'),
            'f' => Some('\u{c}'),
            'v' => Some('\u{b}'),
            _ => None,
        };
        if let Some(value) = simple {
            return Ok((value, crate::host::LiteralCodeUnit::Scalar(value)));
        }

        let (radix, digits) = match ch {
            'x' => (16, None),
            'u' => (16, Some(4)),
            'U' => (16, Some(8)),
            '0'..='7' => {
                let Some(mut value) = ch.to_digit(8) else {
                    return Err(self.error(
                        escape_start,
                        self.offset,
                        LexErrorKind::InternalInvariant("octal escape token was not octal"),
                    ));
                };
                for _ in 0..2 {
                    let Some(next) = self.peek_char() else { break };
                    let Some(digit) = next.to_digit(8) else { break };
                    value = value * 8 + digit;
                    self.advance_char();
                }
                return Ok((
                    char::from_u32(value).unwrap_or(char::REPLACEMENT_CHARACTER),
                    crate::host::LiteralCodeUnit::NumericEscape {
                        radix: crate::host::LiteralEscapeRadix::Octal,
                        value,
                    },
                ));
            }
            _ => {
                return Err(self.error(
                    escape_start,
                    self.offset,
                    LexErrorKind::InvalidEscape(self.source[escape_start..self.offset].into()),
                ));
            }
        };
        let digits_start = self.offset;
        let mut count = 0usize;
        let mut value = 0u32;
        while let Some(next) = self.peek_char() {
            let Some(digit) = next.to_digit(radix) else {
                break;
            };
            if digits.is_some_and(|limit| count == limit) {
                break;
            }
            value = value
                .checked_mul(radix)
                .and_then(|value| value.checked_add(digit))
                .ok_or_else(|| {
                    self.error(
                        escape_start,
                        self.offset,
                        LexErrorKind::InvalidEscape(self.source[escape_start..self.offset].into()),
                    )
                })?;
            count += 1;
            self.advance_char();
        }
        if count == 0 || digits.is_some_and(|required| count != required) {
            return Err(self.error(
                escape_start,
                self.offset.max(digits_start),
                LexErrorKind::InvalidEscape(self.source[escape_start..self.offset].into()),
            ));
        }
        if digits.is_some() {
            let character = char::from_u32(value).ok_or_else(|| {
                self.error(
                    escape_start,
                    self.offset,
                    LexErrorKind::InvalidEscape(self.source[escape_start..self.offset].into()),
                )
            })?;
            return Ok((character, crate::host::LiteralCodeUnit::Scalar(character)));
        }
        Ok((
            char::from_u32(value).unwrap_or(char::REPLACEMENT_CHARACTER),
            crate::host::LiteralCodeUnit::NumericEscape {
                radix: crate::host::LiteralEscapeRadix::Hexadecimal,
                value,
            },
        ))
    }

    fn lex_symbol(&mut self) -> Result<Token, LexError> {
        let start = self.offset;
        macro_rules! symbol {
            ($text:literal, $kind:expr) => {
                if self.starts_with($text) {
                    self.offset += $text.len();
                    return Ok(Token {
                        kind: $kind,
                        span: self.span(start, self.offset)?,
                    });
                }
            };
        }

        symbol!("<<=", TokenKind::ShiftLeftEqual);
        symbol!(">>=", TokenKind::ShiftRightEqual);
        symbol!("->", TokenKind::Arrow);
        symbol!("++", TokenKind::PlusPlus);
        symbol!("--", TokenKind::MinusMinus);
        if self.language == HostLanguage::Fortran {
            symbol!("**", TokenKind::Power);
            symbol!("//", TokenKind::Concat);
        }
        symbol!("<=", TokenKind::LessEqual);
        symbol!(">=", TokenKind::GreaterEqual);
        symbol!("==", TokenKind::EqualEqual);
        symbol!("!=", TokenKind::NotEqual);
        if self.language == HostLanguage::Fortran {
            symbol!("/=", TokenKind::NotEqual);
        }
        if self.language != HostLanguage::Fortran {
            symbol!("&&", TokenKind::LogicalAnd);
            symbol!("||", TokenKind::LogicalOr);
            symbol!("+=", TokenKind::PlusEqual);
            symbol!("-=", TokenKind::MinusEqual);
            symbol!("*=", TokenKind::StarEqual);
            symbol!("/=", TokenKind::SlashEqual);
            symbol!("%=", TokenKind::RemainderEqual);
            symbol!("&=", TokenKind::AmpersandEqual);
            symbol!("|=", TokenKind::PipeEqual);
            symbol!("^=", TokenKind::CaretEqual);
            symbol!("<<", TokenKind::ShiftLeft);
            symbol!(">>", TokenKind::ShiftRight);
            symbol!("::", TokenKind::Scope);
        }

        let Some(ch) = self.advance_char() else {
            return Err(self.error(
                start,
                self.offset,
                LexErrorKind::InternalInvariant("symbol lexer called at end of input"),
            ));
        };
        if ch == '%'
            && matches!(
                self.profile,
                HostLanguageProfile::Fortran(FortranStandard::Fortran77)
            )
        {
            return Err(self.error(
                start,
                self.offset,
                LexErrorKind::NotAvailableInProfile("Fortran derived-type component selector"),
            ));
        }
        let kind = match ch {
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            ',' => TokenKind::Comma,
            '?' if self.language != HostLanguage::Fortran => TokenKind::Question,
            ':' => TokenKind::Colon,
            '.' => TokenKind::Dot,
            '%' if self.language == HostLanguage::Fortran => TokenKind::Percent,
            '%' => TokenKind::Remainder,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '!' if self.language != HostLanguage::Fortran => TokenKind::LogicalNot,
            '~' if self.language != HostLanguage::Fortran => TokenKind::BitwiseNot,
            '&' if self.language != HostLanguage::Fortran => TokenKind::Ampersand,
            '|' if self.language != HostLanguage::Fortran => TokenKind::Pipe,
            '^' if self.language != HostLanguage::Fortran => TokenKind::Caret,
            '=' => TokenKind::Equal,
            '<' => TokenKind::Less,
            '>' => TokenKind::Greater,
            _ => return Err(self.error(start, self.offset, LexErrorKind::UnexpectedCharacter(ch))),
        };
        Ok(Token {
            kind,
            span: self.span(start, self.offset)?,
        })
    }

    fn starts_with(&self, text: &str) -> bool {
        self.source[self.offset..].starts_with(text)
    }

    fn peek_char(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn peek_nth_char(&self, n: usize) -> Option<char> {
        self.source[self.offset..].chars().nth(n)
    }

    fn advance_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.offset += ch.len_utf8();
        Some(ch)
    }

    fn span(&self, start: usize, end: usize) -> Result<Span, LexError> {
        Span::new(self.source, start, end).map_err(|error| LexError {
            span: Span::entire(self.source),
            kind: LexErrorKind::InvalidSpan(error),
        })
    }

    fn error(&self, start: usize, end: usize, kind: LexErrorKind) -> LexError {
        match Span::new(self.source, start, end.min(self.source.len())) {
            Ok(span) => LexError { span, kind },
            Err(error) => LexError {
                span: Span::entire(self.source),
                kind: LexErrorKind::InvalidSpan(error),
            },
        }
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

pub(crate) fn is_reserved_keyword(profile: HostLanguageProfile, text: &str) -> bool {
    match profile {
        HostLanguageProfile::Fortran(_) => false,
        HostLanguageProfile::C(standard) => {
            let c89 = matches!(
                text,
                "auto"
                    | "break"
                    | "case"
                    | "char"
                    | "const"
                    | "continue"
                    | "default"
                    | "do"
                    | "double"
                    | "else"
                    | "enum"
                    | "extern"
                    | "float"
                    | "for"
                    | "goto"
                    | "if"
                    | "int"
                    | "long"
                    | "register"
                    | "return"
                    | "short"
                    | "signed"
                    | "sizeof"
                    | "static"
                    | "struct"
                    | "switch"
                    | "typedef"
                    | "union"
                    | "unsigned"
                    | "void"
                    | "volatile"
                    | "while"
            );
            let c99 = standard >= CStandard::C99
                && matches!(
                    text,
                    "inline" | "restrict" | "_Bool" | "_Complex" | "_Imaginary"
                );
            let c11 = standard >= CStandard::C11
                && matches!(
                    text,
                    "_Alignas"
                        | "_Alignof"
                        | "_Atomic"
                        | "_Generic"
                        | "_Noreturn"
                        | "_Static_assert"
                        | "_Thread_local"
                );
            let c23 = standard >= CStandard::C23
                && matches!(
                    text,
                    "alignas"
                        | "alignof"
                        | "bool"
                        | "constexpr"
                        | "static_assert"
                        | "thread_local"
                        | "typeof"
                        | "typeof_unqual"
                        | "_BitInt"
                        | "_Decimal128"
                        | "_Decimal32"
                        | "_Decimal64"
                );
            c89 || c99 || c11 || c23
        }
        HostLanguageProfile::Cpp(standard) => {
            let cpp98 = matches!(
                text,
                "asm"
                    | "auto"
                    | "bool"
                    | "break"
                    | "case"
                    | "catch"
                    | "char"
                    | "class"
                    | "const"
                    | "const_cast"
                    | "continue"
                    | "default"
                    | "delete"
                    | "do"
                    | "double"
                    | "dynamic_cast"
                    | "else"
                    | "enum"
                    | "explicit"
                    | "export"
                    | "extern"
                    | "float"
                    | "for"
                    | "friend"
                    | "goto"
                    | "if"
                    | "inline"
                    | "int"
                    | "long"
                    | "mutable"
                    | "namespace"
                    | "new"
                    | "operator"
                    | "private"
                    | "protected"
                    | "public"
                    | "register"
                    | "reinterpret_cast"
                    | "return"
                    | "short"
                    | "signed"
                    | "sizeof"
                    | "static"
                    | "static_cast"
                    | "struct"
                    | "switch"
                    | "template"
                    | "this"
                    | "throw"
                    | "try"
                    | "typedef"
                    | "typeid"
                    | "typename"
                    | "union"
                    | "unsigned"
                    | "using"
                    | "virtual"
                    | "void"
                    | "volatile"
                    | "wchar_t"
                    | "while"
            );
            let cpp11 = standard >= CppStandard::Cpp11
                && matches!(
                    text,
                    "alignas"
                        | "alignof"
                        | "char16_t"
                        | "char32_t"
                        | "constexpr"
                        | "decltype"
                        | "noexcept"
                        | "static_assert"
                        | "thread_local"
                );
            let cpp20 = standard >= CppStandard::Cpp20
                && matches!(
                    text,
                    "char8_t"
                        | "co_await"
                        | "co_return"
                        | "co_yield"
                        | "concept"
                        | "consteval"
                        | "constinit"
                        | "requires"
                );
            cpp98 || cpp11 || cpp20
        }
    }
}

fn strip_digit_separators(text: &str) -> String {
    text.chars().filter(|ch| *ch != '\'').collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_identifier_spans_are_byte_safe() {
        let tokens = Lexer::new("α + β", HostLanguage::Cpp).tokenize().unwrap();
        assert_eq!(tokens[0].span.byte_range(), 0..2);
        assert_eq!(tokens[2].span.byte_range(), 5..7);
    }

    #[test]
    fn classifies_literals_without_opaque_lexemes() {
        let tokens = Lexer::new("0xffULL 1.25e-2f 'x' \"ok\"", HostLanguage::Cpp)
            .tokenize()
            .unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Integer(_)));
        assert!(matches!(tokens[1].kind, TokenKind::Real(_)));
        assert!(matches!(tokens[2].kind, TokenKind::Character(_)));
        assert!(matches!(tokens[3].kind, TokenKind::String(_)));
    }

    #[test]
    fn recognizes_fortran_operators_case_insensitively() {
        let tokens = Lexer::new(".NOT. flag .OR. .TRUE.", HostLanguage::Fortran)
            .tokenize()
            .unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::LogicalNot));
        assert!(matches!(tokens[2].kind, TokenKind::LogicalOr));
        assert!(matches!(tokens[3].kind, TokenKind::Boolean(true)));
    }

    #[test]
    fn rejects_unterminated_comment() {
        let error = Lexer::new("a /*", HostLanguage::C).tokenize().unwrap_err();
        assert_eq!(error.kind, LexErrorKind::UnterminatedBlockComment);
    }

    #[test]
    fn numeric_literal_escapes_remain_typed_code_units() {
        let tokens = Lexer::new("'\\x110000' \"a\\377f\"", HostLanguage::C)
            .tokenize()
            .unwrap();
        let TokenKind::Character(character) = &tokens[0].kind else {
            panic!("expected character literal");
        };
        assert_eq!(character.value, char::REPLACEMENT_CHARACTER);
        assert_eq!(
            character.code_unit,
            crate::host::LiteralCodeUnit::NumericEscape {
                radix: crate::host::LiteralEscapeRadix::Hexadecimal,
                value: 0x11_0000,
            }
        );

        let TokenKind::String(string) = &tokens[1].kind else {
            panic!("expected string literal");
        };
        assert_eq!(
            string.code_units,
            [
                crate::host::LiteralCodeUnit::Scalar('a'),
                crate::host::LiteralCodeUnit::NumericEscape {
                    radix: crate::host::LiteralEscapeRadix::Octal,
                    value: 0o377,
                },
                crate::host::LiteralCodeUnit::Scalar('f'),
            ]
        );
    }

    #[test]
    fn c_and_cpp_keywords_are_case_sensitive_and_never_names() {
        let upper = Lexer::new("TRUE AND", HostLanguage::Cpp)
            .tokenize()
            .unwrap();
        assert!(matches!(upper[0].kind, TokenKind::Identifier(_)));
        assert!(matches!(upper[1].kind, TokenKind::Identifier(_)));

        for (language, source) in [
            (HostLanguage::C, "sizeof"),
            (HostLanguage::C, "_Generic"),
            (HostLanguage::Cpp, "new"),
            (HostLanguage::Cpp, "static_cast"),
        ] {
            let error = Lexer::new(source, language).tokenize().unwrap_err();
            assert_eq!(error.kind, LexErrorKind::UnsupportedKeyword);
        }
    }

    #[test]
    fn numeric_rules_are_language_specific_and_strict() {
        let fortran = Lexer::new("012", HostLanguage::Fortran).tokenize().unwrap();
        let TokenKind::Integer(integer) = &fortran[0].kind else {
            panic!("expected integer")
        };
        assert_eq!(integer.base, IntegerBase::Decimal);
        assert_eq!(integer.value, 12);

        assert!(
            Lexer::new("0x10", HostLanguage::Fortran)
                .tokenize()
                .is_err()
        );
        for source in ["1''2", "0x'1", "12uu", "12lz"] {
            assert!(
                Lexer::new(source, HostLanguage::Cpp).tokenize().is_err(),
                "invalid numeric spelling was accepted: {source}"
            );
        }
    }

    #[test]
    fn octal_escape_consumes_up_to_three_digits() {
        let tokens = Lexer::new("'\\012'", HostLanguage::C).tokenize().unwrap();
        let TokenKind::Character(character) = &tokens[0].kind else {
            panic!("expected character literal")
        };
        assert_eq!(character.value, '\n');
    }
}
