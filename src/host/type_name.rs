//! Typed lexical syntax for host-language type names.
//!
//! OpenMP delegates type-name grammar to the base language. ROUP does not
//! pretend to perform compiler name lookup, but it does lex every component,
//! reject malformed delimiters and unsupported characters, and retain only
//! typed tokens. There is no raw-string type-name escape hatch.

use std::fmt;

use crate::source::Span;

use crate::version::{CStandard, CppStandard, FortranStandard, HostLanguageProfile};

use super::{Expr, ExprKind, HostLanguage, LexError, Lexer, Literal, TokenKind};

/// A non-empty, lexed, delimiter-balanced host-language type-name.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeName {
    profile: HostLanguageProfile,
    tokens: Vec<TokenKind>,
}

impl TypeName {
    /// Parse host type syntax into typed lexical tokens.
    pub fn parse(source: &str, language: HostLanguage) -> Result<Self, TypeNameError> {
        Self::parse_with_profile(source, HostLanguageProfile::latest(language))
    }

    /// Parse under an exact base-language standard.
    pub fn parse_with_profile(
        source: &str,
        profile: HostLanguageProfile,
    ) -> Result<Self, TypeNameError> {
        let language = profile.language();
        if source.trim().is_empty() {
            return Err(TypeNameError::Empty);
        }

        let mut tokens = Lexer::for_type_name_with_profile(source, profile).tokenize()?;
        let Some(end) = tokens.pop() else {
            return Err(TypeNameError::Empty);
        };
        if !matches!(end.kind, TokenKind::End) {
            return Err(TypeNameError::MissingEndToken);
        }
        if tokens.is_empty() {
            return Err(TypeNameError::Empty);
        }

        let mut delimiters: Vec<(Delimiter, Span)> = Vec::new();
        let mut angle_depth = 0usize;
        let mut first_angle = None;
        let mut has_word = false;
        let mut kinds = Vec::with_capacity(tokens.len());
        for token in &tokens {
            match token.kind {
                TokenKind::Identifier(_) => has_word = true,
                TokenKind::LeftParen => delimiters.push((Delimiter::Parenthesis, token.span)),
                TokenKind::LeftBracket => delimiters.push((Delimiter::Bracket, token.span)),
                TokenKind::RightParen => {
                    close_delimiter(&mut delimiters, Delimiter::Parenthesis, token.span)?;
                }
                TokenKind::RightBracket => {
                    close_delimiter(&mut delimiters, Delimiter::Bracket, token.span)?;
                }
                TokenKind::Less if delimiters.is_empty() => {
                    let template_prefix = matches!(
                        kinds.last(),
                        Some(
                            TokenKind::Identifier(_)
                                | TokenKind::Greater
                                | TokenKind::ShiftRight
                                | TokenKind::RightParen
                                | TokenKind::RightBracket
                        )
                    );
                    if language != HostLanguage::Cpp || !template_prefix {
                        return Err(TypeNameError::UnexpectedAngle { span: token.span });
                    }
                    first_angle.get_or_insert(token.span);
                    angle_depth += 1;
                }
                TokenKind::Greater if delimiters.is_empty() => {
                    if angle_depth == 0 {
                        return Err(TypeNameError::UnexpectedAngle { span: token.span });
                    }
                    angle_depth -= 1;
                }
                TokenKind::ShiftRight if delimiters.is_empty() && angle_depth > 0 => {
                    if angle_depth < 2 {
                        return Err(TypeNameError::UnexpectedAngle { span: token.span });
                    }
                    angle_depth -= 2;
                }
                TokenKind::Comma if delimiters.is_empty() && angle_depth == 0 => {
                    return Err(TypeNameError::TopLevelComma { span: token.span });
                }
                TokenKind::End => return Err(TypeNameError::UnexpectedEndToken),
                _ => {}
            }
            kinds.push(token.kind.clone());
        }

        if let Some((delimiter, span)) = delimiters.pop() {
            return Err(TypeNameError::UnclosedDelimiter { delimiter, span });
        }
        if angle_depth != 0 {
            return Err(TypeNameError::UnclosedAngle {
                span: first_angle.ok_or(TypeNameError::MissingEndToken)?,
            });
        }
        if !has_word {
            return Err(TypeNameError::MissingTypeWord);
        }
        let valid_start = matches!(
            kinds.first(),
            Some(TokenKind::Identifier(_) | TokenKind::Scope)
        );
        let mut valid_end = matches!(
            kinds.last(),
            Some(
                TokenKind::Identifier(_)
                    | TokenKind::RightParen
                    | TokenKind::RightBracket
                    | TokenKind::Greater
                    | TokenKind::ShiftRight
                    | TokenKind::Star
                    | TokenKind::Ampersand
                    | TokenKind::LogicalAnd
            )
        );
        if language == HostLanguage::C
            && matches!(
                kinds.last(),
                Some(TokenKind::Ampersand | TokenKind::LogicalAnd)
            )
        {
            valid_end = false;
        }
        if language == HostLanguage::Fortran
            && matches!(kinds.last(), Some(TokenKind::Integer(_)))
            && matches!(
                kinds.get(kinds.len().saturating_sub(2)),
                Some(TokenKind::Star)
            )
        {
            valid_end = true;
        }
        if !valid_start || !valid_end {
            return Err(TypeNameError::InvalidBoundaryToken);
        }

        validate_structured_syntax(&kinds, language)?;
        validate_profile_features(&kinds, profile)?;

        Ok(Self {
            profile,
            tokens: kinds,
        })
    }

    #[must_use]
    pub const fn language(&self) -> HostLanguage {
        self.profile.language()
    }

    #[must_use]
    pub const fn profile(&self) -> HostLanguageProfile {
        self.profile
    }

    /// Typed lexical components in source order. Delimiters are retained as
    /// tokens after their balance has been validated.
    #[must_use]
    pub fn tokens(&self) -> &[TokenKind] {
        &self.tokens
    }

    /// Render a name-like token sequence without inserting whitespace around
    /// C++ qualification and template punctuation. Adjacent word tokens stay
    /// separated so canonicalization cannot merge distinct identifiers or
    /// literals into a different token.
    pub(crate) fn fmt_compact(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, token) in self.tokens.iter().enumerate() {
            if index > 0 && compact_tokens_need_space(&self.tokens[index - 1], token) {
                formatter.write_str(" ")?;
            }
            write_token(formatter, token, self.language())?;
        }
        Ok(())
    }
}

impl fmt::Display for TypeName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, token) in self.tokens.iter().enumerate() {
            if index > 0 {
                formatter.write_str(" ")?;
            }
            write_token(formatter, token, self.language())?;
        }
        Ok(())
    }
}

fn compact_tokens_need_space(previous: &TokenKind, current: &TokenKind) -> bool {
    matches!(previous, TokenKind::Comma) || (is_word_token(previous) && is_word_token(current))
}

fn is_word_token(token: &TokenKind) -> bool {
    matches!(
        token,
        TokenKind::Identifier(_)
            | TokenKind::Integer(_)
            | TokenKind::Real(_)
            | TokenKind::Character(_)
            | TokenKind::String(_)
            | TokenKind::Boolean(_)
            | TokenKind::NullPointer
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delimiter {
    Parenthesis,
    Bracket,
}

impl Delimiter {
    const fn opening(self) -> char {
        match self {
            Self::Parenthesis => '(',
            Self::Bracket => '[',
        }
    }

    const fn closing(self) -> char {
        match self {
            Self::Parenthesis => ')',
            Self::Bracket => ']',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeNameError {
    Empty,
    Lexical(LexError),
    MissingTypeWord,
    InvalidBoundaryToken,
    UnexpectedEndToken,
    MissingEndToken,
    UnexpectedAngle {
        span: Span,
    },
    UnclosedAngle {
        span: Span,
    },
    TopLevelComma {
        span: Span,
    },
    UnexpectedClosingDelimiter {
        expected: Option<Delimiter>,
        actual: Delimiter,
        span: Span,
    },
    UnclosedDelimiter {
        delimiter: Delimiter,
        span: Span,
    },
    NotAvailableInProfile(&'static str),
    InvalidSyntax(&'static str),
}

impl fmt::Display for TypeNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("type name must not be empty"),
            Self::Lexical(error) => write!(formatter, "invalid type-name token: {error}"),
            Self::MissingTypeWord => {
                formatter.write_str("type name must contain an identifier or type keyword")
            }
            Self::InvalidBoundaryToken => {
                formatter.write_str("type name starts or ends with an invalid token")
            }
            Self::UnexpectedEndToken | Self::MissingEndToken => {
                formatter.write_str("type-name lexer violated its end-token invariant")
            }
            Self::UnexpectedAngle { span } => {
                write!(formatter, "unexpected template angle token at {span}")
            }
            Self::UnclosedAngle { span } => {
                write!(formatter, "unclosed template argument list from {span}")
            }
            Self::TopLevelComma { span } => {
                write!(
                    formatter,
                    "one type name cannot contain a top-level comma at {span}"
                )
            }
            Self::UnexpectedClosingDelimiter {
                expected,
                actual,
                span,
            } => {
                if let Some(expected) = expected {
                    write!(
                        formatter,
                        "unexpected `{}` at {span}; expected `{}`",
                        actual.closing(),
                        expected.closing()
                    )
                } else {
                    write!(
                        formatter,
                        "unexpected closing `{}` at {span}",
                        actual.closing()
                    )
                }
            }
            Self::UnclosedDelimiter { delimiter, span } => write!(
                formatter,
                "unclosed `{}` from {span}; expected `{}`",
                delimiter.opening(),
                delimiter.closing()
            ),
            Self::NotAvailableInProfile(feature) => write!(
                formatter,
                "{feature} is not available in the configured host language standard"
            ),
            Self::InvalidSyntax(reason) => write!(formatter, "invalid type-name syntax: {reason}"),
        }
    }
}

impl std::error::Error for TypeNameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Lexical(error) => Some(error),
            _ => None,
        }
    }
}

impl From<LexError> for TypeNameError {
    fn from(error: LexError) -> Self {
        Self::Lexical(error)
    }
}

/// A private syntax tree built directly from lexer tokens. Parentheses,
/// brackets, and template argument lists are represented explicitly so that
/// validation never has to render and reparse a type spelling.
#[derive(Debug)]
enum TypeSyntax<'a> {
    Token(&'a TokenKind),
    Parenthesized(Vec<TypeSyntax<'a>>),
    Bracketed(Vec<TypeSyntax<'a>>),
    TemplateArguments(Vec<Vec<TypeSyntax<'a>>>),
}

#[derive(Clone, Copy)]
enum GroupEnd {
    Parenthesis,
    Bracket,
}

#[derive(Clone, Copy)]
struct TypeSyntaxParser<'a> {
    tokens: &'a [TokenKind],
    cursor: usize,
    pending_greater: usize,
}

impl<'a> TypeSyntaxParser<'a> {
    const MAX_NESTING_DEPTH: usize = 64;

    const fn new(tokens: &'a [TokenKind]) -> Self {
        Self {
            tokens,
            cursor: 0,
            pending_greater: 0,
        }
    }

    fn parse(mut self, language: HostLanguage) -> Result<Vec<TypeSyntax<'a>>, TypeNameError> {
        let syntax = self.parse_group(None, 0, 0, language)?;
        if self.cursor != self.tokens.len() || self.pending_greater != 0 {
            return Err(TypeNameError::InvalidSyntax(
                "unexpected token after the complete type name",
            ));
        }
        Ok(syntax)
    }

    fn parse_group(
        &mut self,
        end: Option<GroupEnd>,
        template_depth: usize,
        nesting_depth: usize,
        language: HostLanguage,
    ) -> Result<Vec<TypeSyntax<'a>>, TypeNameError> {
        if nesting_depth > Self::MAX_NESTING_DEPTH {
            return Err(TypeNameError::InvalidSyntax(
                "type-name nesting limit exceeded",
            ));
        }
        let mut nodes = Vec::new();
        loop {
            let Some(token) = self.tokens.get(self.cursor) else {
                return if end.is_none() {
                    Ok(nodes)
                } else {
                    Err(TypeNameError::InvalidSyntax(
                        "unterminated parenthesized or bracketed type component",
                    ))
                };
            };

            if matches!(
                (end, token),
                (Some(GroupEnd::Parenthesis), TokenKind::RightParen)
            ) || matches!(
                (end, token),
                (Some(GroupEnd::Bracket), TokenKind::RightBracket)
            ) {
                self.cursor += 1;
                return Ok(nodes);
            }
            if matches!(token, TokenKind::RightParen | TokenKind::RightBracket) {
                return Err(TypeNameError::InvalidSyntax(
                    "mismatched closing delimiter in type name",
                ));
            }

            nodes.push(self.parse_node(&nodes, template_depth, nesting_depth, language)?);
        }
    }

    fn parse_node(
        &mut self,
        previous: &[TypeSyntax<'a>],
        template_depth: usize,
        nesting_depth: usize,
        language: HostLanguage,
    ) -> Result<TypeSyntax<'a>, TypeNameError> {
        let token = self
            .tokens
            .get(self.cursor)
            .ok_or(TypeNameError::InvalidSyntax(
                "type-name parser reached the end of its token stream",
            ))?;
        match token {
            TokenKind::LeftParen => {
                self.cursor += 1;
                Ok(TypeSyntax::Parenthesized(self.parse_group(
                    Some(GroupEnd::Parenthesis),
                    template_depth,
                    nesting_depth + 1,
                    language,
                )?))
            }
            TokenKind::LeftBracket => {
                self.cursor += 1;
                Ok(TypeSyntax::Bracketed(self.parse_group(
                    Some(GroupEnd::Bracket),
                    template_depth,
                    nesting_depth + 1,
                    language,
                )?))
            }
            TokenKind::Less
                if language == HostLanguage::Cpp
                    && matches!(
                        previous.last(),
                        Some(TypeSyntax::Token(TokenKind::Identifier(_)))
                    ) =>
            {
                // `<` can also be a comparison inside a non-type template
                // argument. Try the template grammar transactionally and keep
                // it as an operator if no complete template list follows.
                let checkpoint = *self;
                match self.parse_template_arguments(template_depth + 1, nesting_depth + 1, language)
                {
                    Ok(arguments) => Ok(TypeSyntax::TemplateArguments(arguments)),
                    Err(_) => {
                        *self = checkpoint;
                        self.cursor += 1;
                        Ok(TypeSyntax::Token(token))
                    }
                }
            }
            _ => {
                self.cursor += 1;
                Ok(TypeSyntax::Token(token))
            }
        }
    }

    fn parse_template_arguments(
        &mut self,
        template_depth: usize,
        nesting_depth: usize,
        language: HostLanguage,
    ) -> Result<Vec<Vec<TypeSyntax<'a>>>, TypeNameError> {
        if nesting_depth > Self::MAX_NESTING_DEPTH {
            return Err(TypeNameError::InvalidSyntax(
                "type-name nesting limit exceeded",
            ));
        }
        if !matches!(self.tokens.get(self.cursor), Some(TokenKind::Less)) {
            return Err(TypeNameError::InvalidSyntax(
                "template argument list does not start with `<`",
            ));
        }
        self.cursor += 1;
        let mut arguments = Vec::new();

        loop {
            if self.is_angle_close(template_depth) {
                return Err(TypeNameError::InvalidSyntax(
                    "template argument list contains an empty argument",
                ));
            }

            let mut argument = Vec::new();
            while !self.is_angle_close(template_depth)
                && !matches!(self.tokens.get(self.cursor), Some(TokenKind::Comma))
            {
                if self.cursor == self.tokens.len() {
                    return Err(TypeNameError::InvalidSyntax(
                        "unterminated template argument list",
                    ));
                }
                argument.push(self.parse_node(
                    &argument,
                    template_depth,
                    nesting_depth,
                    language,
                )?);
            }
            if argument.is_empty() {
                return Err(TypeNameError::InvalidSyntax(
                    "template argument list contains an empty argument",
                ));
            }
            arguments.push(argument);

            if matches!(self.tokens.get(self.cursor), Some(TokenKind::Comma)) {
                self.cursor += 1;
                if self.is_angle_close(template_depth) {
                    return Err(TypeNameError::InvalidSyntax(
                        "template argument list ends with a comma",
                    ));
                }
                continue;
            }

            self.take_angle_close(template_depth)?;
            return Ok(arguments);
        }
    }

    fn is_angle_close(&self, template_depth: usize) -> bool {
        self.pending_greater != 0
            || matches!(self.tokens.get(self.cursor), Some(TokenKind::Greater))
            || (template_depth >= 2
                && matches!(self.tokens.get(self.cursor), Some(TokenKind::ShiftRight)))
    }

    fn take_angle_close(&mut self, template_depth: usize) -> Result<(), TypeNameError> {
        if self.pending_greater != 0 {
            self.pending_greater -= 1;
            return Ok(());
        }
        match self.tokens.get(self.cursor) {
            Some(TokenKind::Greater) => {
                self.cursor += 1;
                Ok(())
            }
            Some(TokenKind::ShiftRight) if template_depth >= 2 => {
                self.cursor += 1;
                self.pending_greater = 1;
                Ok(())
            }
            _ => Err(TypeNameError::InvalidSyntax(
                "template argument list is missing its closing `>`",
            )),
        }
    }
}

fn validate_structured_syntax(
    tokens: &[TokenKind],
    language: HostLanguage,
) -> Result<(), TypeNameError> {
    let syntax = TypeSyntaxParser::new(tokens).parse(language)?;
    let valid = match language {
        HostLanguage::C | HostLanguage::Cpp => validate_c_family_type(&syntax, language),
        HostLanguage::Fortran => validate_fortran_type(&syntax),
    };
    if valid {
        Ok(())
    } else {
        Err(TypeNameError::InvalidSyntax(
            "tokens do not form a host-language type name",
        ))
    }
}

fn validate_c_family_type(nodes: &[TypeSyntax<'_>], language: HostLanguage) -> bool {
    if nodes.is_empty() {
        return false;
    }

    let mut saw_type_word = false;
    let mut declarator_started = false;
    let mut saw_reference = false;

    for (index, node) in nodes.iter().enumerate() {
        match node {
            TypeSyntax::Token(TokenKind::Identifier(identifier)) => {
                if declarator_started && !is_c_family_declarator_qualifier(identifier.as_str()) {
                    return false;
                }
                saw_type_word = true;
            }
            TypeSyntax::Token(TokenKind::Scope) => {
                if language != HostLanguage::Cpp || !scope_has_valid_neighbors(nodes, index) {
                    return false;
                }
            }
            TypeSyntax::TemplateArguments(arguments) => {
                if language != HostLanguage::Cpp
                    || !matches!(
                        nodes.get(index.wrapping_sub(1)),
                        Some(TypeSyntax::Token(TokenKind::Identifier(_)))
                    )
                    || !arguments
                        .iter()
                        .all(|argument| validate_cpp_template_argument(argument))
                {
                    return false;
                }
                saw_type_word = true;
            }
            TypeSyntax::Token(TokenKind::Star) => {
                if !saw_type_word || saw_reference {
                    return false;
                }
                declarator_started = true;
            }
            TypeSyntax::Token(TokenKind::Ampersand | TokenKind::LogicalAnd) => {
                if language != HostLanguage::Cpp || !saw_type_word || saw_reference {
                    return false;
                }
                declarator_started = true;
                saw_reference = true;
            }
            TypeSyntax::Parenthesized(inner) => {
                let preceding_identifier =
                    nodes.get(index.wrapping_sub(1)).and_then(node_identifier);
                let valid = match preceding_identifier {
                    Some("decltype") => {
                        language == HostLanguage::Cpp && validate_expression(inner, language)
                    }
                    Some("typeof" | "typeof_unqual" | "__typeof" | "__typeof__") => {
                        validate_c_family_type(inner, language)
                            || validate_expression(inner, language)
                    }
                    Some("_Atomic") => {
                        language == HostLanguage::C && validate_c_family_type(inner, language)
                    }
                    Some("_BitInt") => {
                        language == HostLanguage::C && validate_expression(inner, language)
                    }
                    Some(name) if name == "noexcept" && declarator_started => {
                        language == HostLanguage::Cpp
                            && (inner.is_empty() || validate_expression(inner, language))
                    }
                    _ if looks_like_abstract_declarator(inner) => {
                        validate_abstract_declarator(inner, language)
                    }
                    _ => saw_type_word && validate_parameter_type_list(inner, language),
                };
                if !valid {
                    return false;
                }
                declarator_started = true;
            }
            TypeSyntax::Bracketed(inner) => {
                if !saw_type_word
                    || (!inner.is_empty()
                        && !is_single_token(inner, |token| matches!(token, TokenKind::Star))
                        && !validate_expression(inner, language))
                {
                    return false;
                }
                declarator_started = true;
            }
            _ => return false,
        }
    }

    saw_type_word
}

fn validate_cpp_template_argument(nodes: &[TypeSyntax<'_>]) -> bool {
    !nodes.is_empty()
        && (validate_c_family_type(nodes, HostLanguage::Cpp)
            || validate_expression(nodes, HostLanguage::Cpp))
}

fn validate_parameter_type_list(nodes: &[TypeSyntax<'_>], language: HostLanguage) -> bool {
    if nodes.is_empty() {
        return true;
    }
    split_on_commas(nodes).is_some_and(|parameters| {
        parameters
            .into_iter()
            .all(|parameter| is_ellipsis(parameter) || validate_c_family_type(parameter, language))
    })
}

fn looks_like_abstract_declarator(nodes: &[TypeSyntax<'_>]) -> bool {
    matches!(
        nodes.first(),
        Some(
            TypeSyntax::Token(
                TokenKind::Star | TokenKind::Ampersand | TokenKind::LogicalAnd | TokenKind::Scope
            ) | TypeSyntax::Parenthesized(_)
                | TypeSyntax::Bracketed(_)
        )
    ) || (nodes
        .iter()
        .any(|node| matches!(node, TypeSyntax::Token(TokenKind::Scope)))
        && nodes
            .iter()
            .any(|node| matches!(node, TypeSyntax::Token(TokenKind::Star))))
}

fn validate_abstract_declarator(nodes: &[TypeSyntax<'_>], language: HostLanguage) -> bool {
    if nodes.is_empty() {
        return false;
    }
    let mut saw_operator = false;
    let mut saw_reference = false;

    for (index, node) in nodes.iter().enumerate() {
        match node {
            TypeSyntax::Token(TokenKind::Star) => {
                if saw_reference {
                    return false;
                }
                saw_operator = true;
            }
            TypeSyntax::Token(TokenKind::Ampersand | TokenKind::LogicalAnd) => {
                if language != HostLanguage::Cpp || saw_reference {
                    return false;
                }
                saw_reference = true;
                saw_operator = true;
            }
            TypeSyntax::Token(TokenKind::Identifier(identifier)) => {
                let next_is_scope = matches!(
                    nodes.get(index + 1),
                    Some(TypeSyntax::Token(TokenKind::Scope))
                );
                let next_is_template =
                    matches!(nodes.get(index + 1), Some(TypeSyntax::TemplateArguments(_)));
                let previous_is_scope = matches!(
                    index
                        .checked_sub(1)
                        .and_then(|previous| nodes.get(previous)),
                    Some(TypeSyntax::Token(TokenKind::Scope))
                );
                if !is_c_family_declarator_qualifier(identifier.as_str())
                    && !next_is_scope
                    && !next_is_template
                    && !previous_is_scope
                {
                    return false;
                }
            }
            TypeSyntax::Token(TokenKind::Scope) => {
                if language != HostLanguage::Cpp || !scope_has_valid_neighbors(nodes, index) {
                    return false;
                }
            }
            TypeSyntax::TemplateArguments(arguments) => {
                if language != HostLanguage::Cpp
                    || !matches!(
                        index
                            .checked_sub(1)
                            .and_then(|previous| nodes.get(previous)),
                        Some(TypeSyntax::Token(TokenKind::Identifier(_)))
                    )
                    || !arguments
                        .iter()
                        .all(|argument| validate_cpp_template_argument(argument))
                {
                    return false;
                }
            }
            TypeSyntax::Parenthesized(inner) => {
                if !validate_abstract_declarator(inner, language) {
                    return false;
                }
                saw_operator = true;
            }
            TypeSyntax::Bracketed(inner) => {
                if !inner.is_empty()
                    && !is_single_token(inner, |token| matches!(token, TokenKind::Star))
                    && !validate_expression(inner, language)
                {
                    return false;
                }
                saw_operator = true;
            }
            _ => return false,
        }
    }
    saw_operator
}

fn validate_fortran_type(nodes: &[TypeSyntax<'_>]) -> bool {
    if nodes.is_empty() {
        return false;
    }
    let mut saw_word = false;
    let mut saw_selector = false;
    let mut index = 0usize;
    while index < nodes.len() {
        match &nodes[index] {
            TypeSyntax::Token(TokenKind::Identifier(_)) => saw_word = true,
            TypeSyntax::Parenthesized(inner) if saw_word && !saw_selector => {
                if !validate_fortran_selector_list(inner) {
                    return false;
                }
                saw_selector = true;
            }
            TypeSyntax::Token(TokenKind::Star)
                if saw_word
                    && index + 1 == nodes.len() - 1
                    && matches!(
                        nodes.get(index + 1),
                        Some(TypeSyntax::Token(TokenKind::Integer(_)))
                    ) =>
            {
                return true;
            }
            _ => return false,
        }
        index += 1;
    }
    saw_word
}

fn validate_fortran_selector_list(nodes: &[TypeSyntax<'_>]) -> bool {
    if nodes.is_empty() {
        return false;
    }
    split_on_commas(nodes).is_some_and(|selectors| {
        selectors.into_iter().all(|selector| {
            if let [
                TypeSyntax::Token(TokenKind::Identifier(_)),
                TypeSyntax::Token(TokenKind::Equal),
                rest @ ..,
            ] = selector
            {
                !rest.is_empty() && validate_fortran_selector_value(rest)
            } else {
                validate_fortran_selector_value(selector)
            }
        })
    })
}

fn validate_fortran_selector_value(nodes: &[TypeSyntax<'_>]) -> bool {
    is_single_token(nodes, |token| {
        matches!(token, TokenKind::Star | TokenKind::Colon)
    }) || validate_expression(nodes, HostLanguage::Fortran)
}

fn validate_expression(nodes: &[TypeSyntax<'_>], language: HostLanguage) -> bool {
    if nodes.is_empty() {
        return false;
    }
    let mut expects_operand = true;
    let mut expects_name_after_access = false;
    let mut conditional_depth = 0usize;

    for node in nodes {
        match node {
            TypeSyntax::Token(TokenKind::Identifier(_)) => {
                if !expects_operand {
                    return false;
                }
                expects_operand = false;
                expects_name_after_access = false;
            }
            TypeSyntax::Token(
                TokenKind::Integer(_)
                | TokenKind::Real(_)
                | TokenKind::Character(_)
                | TokenKind::String(_)
                | TokenKind::Boolean(_)
                | TokenKind::NullPointer,
            ) => {
                if !expects_operand || expects_name_after_access {
                    return false;
                }
                expects_operand = false;
            }
            TypeSyntax::Token(TokenKind::Scope) => {
                if language != HostLanguage::Cpp || expects_name_after_access {
                    return false;
                }
                if expects_operand {
                    expects_name_after_access = true;
                } else {
                    expects_operand = true;
                    expects_name_after_access = true;
                }
            }
            TypeSyntax::Token(TokenKind::Dot | TokenKind::Arrow | TokenKind::Percent) => {
                let valid_access = match language {
                    HostLanguage::C | HostLanguage::Cpp => {
                        matches!(node, TypeSyntax::Token(TokenKind::Dot | TokenKind::Arrow))
                    }
                    HostLanguage::Fortran => matches!(node, TypeSyntax::Token(TokenKind::Percent)),
                };
                if expects_operand || !valid_access {
                    return false;
                }
                expects_operand = true;
                expects_name_after_access = true;
            }
            TypeSyntax::Token(
                TokenKind::LogicalNot
                | TokenKind::BitwiseNot
                | TokenKind::PlusPlus
                | TokenKind::MinusMinus,
            ) if expects_operand && !expects_name_after_access => {}
            TypeSyntax::Token(
                TokenKind::Plus | TokenKind::Minus | TokenKind::Star | TokenKind::Ampersand,
            ) if expects_operand && !expects_name_after_access => {}
            TypeSyntax::Token(TokenKind::PlusPlus | TokenKind::MinusMinus) if !expects_operand => {}
            TypeSyntax::Token(TokenKind::Question) => {
                if language == HostLanguage::Fortran || expects_operand {
                    return false;
                }
                conditional_depth += 1;
                expects_operand = true;
            }
            TypeSyntax::Token(TokenKind::Colon) => {
                if language == HostLanguage::Fortran || expects_operand || conditional_depth == 0 {
                    return false;
                }
                conditional_depth -= 1;
                expects_operand = true;
            }
            TypeSyntax::Token(token) if is_binary_or_assignment_operator(token, language) => {
                if expects_operand {
                    return false;
                }
                expects_operand = true;
                expects_name_after_access = false;
            }
            TypeSyntax::Parenthesized(inner) => {
                if expects_name_after_access {
                    return false;
                }
                if inner.is_empty() {
                    if expects_operand {
                        return false;
                    }
                } else if !validate_expression(inner, language) {
                    return false;
                }
                expects_operand = false;
            }
            TypeSyntax::Bracketed(inner) => {
                if expects_operand || inner.is_empty() || !validate_expression(inner, language) {
                    return false;
                }
                expects_operand = false;
            }
            TypeSyntax::TemplateArguments(arguments) => {
                if language != HostLanguage::Cpp
                    || expects_operand
                    || !arguments
                        .iter()
                        .all(|argument| validate_cpp_template_argument(argument))
                {
                    return false;
                }
            }
            _ => return false,
        }
    }

    !expects_operand && !expects_name_after_access && conditional_depth == 0
}

fn is_binary_or_assignment_operator(token: &TokenKind, language: HostLanguage) -> bool {
    match language {
        HostLanguage::C | HostLanguage::Cpp => matches!(
            token,
            TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Remainder
                | TokenKind::Ampersand
                | TokenKind::Pipe
                | TokenKind::Caret
                | TokenKind::LogicalAnd
                | TokenKind::LogicalOr
                | TokenKind::Equal
                | TokenKind::EqualEqual
                | TokenKind::NotEqual
                | TokenKind::Less
                | TokenKind::LessEqual
                | TokenKind::Greater
                | TokenKind::GreaterEqual
                | TokenKind::ShiftLeft
                | TokenKind::ShiftRight
                | TokenKind::PlusEqual
                | TokenKind::MinusEqual
                | TokenKind::StarEqual
                | TokenKind::SlashEqual
                | TokenKind::RemainderEqual
                | TokenKind::AmpersandEqual
                | TokenKind::PipeEqual
                | TokenKind::CaretEqual
                | TokenKind::ShiftLeftEqual
                | TokenKind::ShiftRightEqual
                | TokenKind::Comma
        ),
        HostLanguage::Fortran => matches!(
            token,
            TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Power
                | TokenKind::Concat
                | TokenKind::LogicalAnd
                | TokenKind::LogicalOr
                | TokenKind::LogicalEqv
                | TokenKind::LogicalNeqv
                | TokenKind::Equal
                | TokenKind::EqualEqual
                | TokenKind::NotEqual
                | TokenKind::Less
                | TokenKind::LessEqual
                | TokenKind::Greater
                | TokenKind::GreaterEqual
                | TokenKind::Comma
        ),
    }
}

fn split_on_commas<'a>(nodes: &'a [TypeSyntax<'a>]) -> Option<Vec<&'a [TypeSyntax<'a>]>> {
    let mut result = Vec::new();
    let mut start = 0usize;
    for (index, node) in nodes.iter().enumerate() {
        if matches!(node, TypeSyntax::Token(TokenKind::Comma)) {
            if start == index {
                return None;
            }
            result.push(&nodes[start..index]);
            start = index + 1;
        }
    }
    if start == nodes.len() {
        return None;
    }
    result.push(&nodes[start..]);
    Some(result)
}

fn scope_has_valid_neighbors(nodes: &[TypeSyntax<'_>], index: usize) -> bool {
    let left_is_valid = index == 0
        || matches!(
            nodes.get(index - 1),
            Some(TypeSyntax::Token(TokenKind::Identifier(_)) | TypeSyntax::TemplateArguments(_))
        );
    let right_is_valid = matches!(
        nodes.get(index + 1),
        Some(TypeSyntax::Token(TokenKind::Identifier(_)))
    ) || matches!(
        nodes.get(index + 1),
        Some(TypeSyntax::Token(TokenKind::Star))
    );
    left_is_valid && right_is_valid
}

fn node_identifier<'a>(node: &'a TypeSyntax<'a>) -> Option<&'a str> {
    let TypeSyntax::Token(TokenKind::Identifier(identifier)) = node else {
        return None;
    };
    Some(identifier.as_str())
}

fn is_c_family_declarator_qualifier(name: &str) -> bool {
    matches!(
        name,
        "const"
            | "volatile"
            | "restrict"
            | "_Atomic"
            | "noexcept"
            | "transaction_safe"
            | "__restrict"
            | "__restrict__"
    )
}

fn is_ellipsis(nodes: &[TypeSyntax<'_>]) -> bool {
    matches!(
        nodes,
        [
            TypeSyntax::Token(TokenKind::Dot),
            TypeSyntax::Token(TokenKind::Dot),
            TypeSyntax::Token(TokenKind::Dot)
        ]
    )
}

fn is_single_token(nodes: &[TypeSyntax<'_>], predicate: impl FnOnce(&TokenKind) -> bool) -> bool {
    let [TypeSyntax::Token(token)] = nodes else {
        return false;
    };
    predicate(token)
}

fn validate_profile_features(
    tokens: &[TokenKind],
    profile: HostLanguageProfile,
) -> Result<(), TypeNameError> {
    match profile {
        HostLanguageProfile::C(standard) => {
            if standard < CStandard::C99 && has_adjacent_identifiers(tokens, "long", "long") {
                return Err(TypeNameError::NotAvailableInProfile("long long type"));
            }
            if standard < CStandard::C11 && has_identifier_call(tokens, "_Atomic") {
                return Err(TypeNameError::NotAvailableInProfile(
                    "_Atomic type specifier",
                ));
            }
            if standard < CStandard::C23 {
                for (name, feature) in [
                    ("_BitInt", "_BitInt type specifier"),
                    ("typeof", "typeof type specifier"),
                    ("typeof_unqual", "typeof_unqual type specifier"),
                ] {
                    if has_identifier_call(tokens, name) {
                        return Err(TypeNameError::NotAvailableInProfile(feature));
                    }
                }
            }
        }
        HostLanguageProfile::Cpp(standard) => {
            if standard < CppStandard::Cpp11 {
                if has_adjacent_identifiers(tokens, "long", "long") {
                    return Err(TypeNameError::NotAvailableInProfile("long long type"));
                }
                if has_identifier_call(tokens, "decltype") {
                    return Err(TypeNameError::NotAvailableInProfile(
                        "decltype type specifier",
                    ));
                }
                if tokens
                    .iter()
                    .any(|token| matches!(token, TokenKind::LogicalAnd))
                {
                    return Err(TypeNameError::NotAvailableInProfile(
                        "rvalue reference type",
                    ));
                }
            }
        }
        HostLanguageProfile::Fortran(standard) => {
            if standard < FortranStandard::Fortran90 {
                if tokens.iter().any(|token| matches!(token, TokenKind::Equal)) {
                    return Err(TypeNameError::NotAvailableInProfile(
                        "Fortran kind or length selector",
                    ));
                }
                if has_identifier_call(tokens, "type") {
                    return Err(TypeNameError::NotAvailableInProfile("Fortran derived type"));
                }
            }
            if standard < FortranStandard::Fortran2003 && has_identifier_call(tokens, "class") {
                return Err(TypeNameError::NotAvailableInProfile(
                    "Fortran polymorphic class type",
                ));
            }
            if standard < FortranStandard::Fortran2018
                && (has_identifier_followed_by_star(tokens, "type") || has_assumed_rank(tokens))
            {
                return Err(TypeNameError::NotAvailableInProfile(
                    "Fortran assumed type or assumed rank",
                ));
            }
        }
    }
    Ok(())
}

fn identifier_is(token: &TokenKind, expected: &str) -> bool {
    matches!(token, TokenKind::Identifier(identifier) if identifier.as_str().eq_ignore_ascii_case(expected))
}

fn has_adjacent_identifiers(tokens: &[TokenKind], first: &str, second: &str) -> bool {
    tokens
        .windows(2)
        .any(|pair| identifier_is(&pair[0], first) && identifier_is(&pair[1], second))
}

fn has_identifier_call(tokens: &[TokenKind], name: &str) -> bool {
    tokens
        .windows(2)
        .any(|pair| identifier_is(&pair[0], name) && matches!(pair[1], TokenKind::LeftParen))
}

fn has_identifier_followed_by_star(tokens: &[TokenKind], name: &str) -> bool {
    tokens.windows(3).any(|triple| {
        identifier_is(&triple[0], name)
            && matches!(triple[1], TokenKind::LeftParen)
            && matches!(triple[2], TokenKind::Star)
    })
}

fn has_assumed_rank(tokens: &[TokenKind]) -> bool {
    tokens.windows(4).any(|window| {
        matches!(window[0], TokenKind::LeftParen)
            && matches!(window[1], TokenKind::Dot)
            && matches!(window[2], TokenKind::Dot)
            && matches!(window[3], TokenKind::RightParen)
    })
}

fn close_delimiter(
    delimiters: &mut Vec<(Delimiter, Span)>,
    actual: Delimiter,
    span: Span,
) -> Result<(), TypeNameError> {
    let expected = delimiters.last().map(|(delimiter, _)| *delimiter);
    if expected != Some(actual) {
        return Err(TypeNameError::UnexpectedClosingDelimiter {
            expected,
            actual,
            span,
        });
    }
    delimiters.pop();
    Ok(())
}

fn write_token(
    formatter: &mut fmt::Formatter<'_>,
    token: &TokenKind,
    language: HostLanguage,
) -> fmt::Result {
    match token {
        TokenKind::Identifier(identifier) => write!(formatter, "{identifier}"),
        TokenKind::Integer(literal) => {
            write_literal(formatter, Literal::Integer(literal.clone()), language)
        }
        TokenKind::Real(literal) => {
            write_literal(formatter, Literal::Real(literal.clone()), language)
        }
        TokenKind::Character(literal) => {
            write_literal(formatter, Literal::Character(literal.clone()), language)
        }
        TokenKind::String(literal) => {
            write_literal(formatter, Literal::String(literal.clone()), language)
        }
        TokenKind::Boolean(value) => write_literal(formatter, Literal::Boolean(*value), language),
        TokenKind::NullPointer => write_literal(formatter, Literal::NullPointer, language),
        TokenKind::LeftParen => formatter.write_str("("),
        TokenKind::RightParen => formatter.write_str(")"),
        TokenKind::LeftBracket => formatter.write_str("["),
        TokenKind::RightBracket => formatter.write_str("]"),
        TokenKind::Comma => formatter.write_str(","),
        TokenKind::Question => formatter.write_str("?"),
        TokenKind::Colon => formatter.write_str(":"),
        TokenKind::Scope => formatter.write_str("::"),
        TokenKind::Dot => formatter.write_str("."),
        TokenKind::Percent | TokenKind::Remainder => formatter.write_str("%"),
        TokenKind::Arrow => formatter.write_str("->"),
        TokenKind::Plus => formatter.write_str("+"),
        TokenKind::Minus => formatter.write_str("-"),
        TokenKind::Star => formatter.write_str("*"),
        TokenKind::Slash => formatter.write_str("/"),
        TokenKind::Power => formatter.write_str("**"),
        TokenKind::Concat => formatter.write_str("//"),
        TokenKind::LogicalNot => formatter.write_str(match language {
            HostLanguage::Fortran => ".not.",
            HostLanguage::C | HostLanguage::Cpp => "!",
        }),
        TokenKind::BitwiseNot => formatter.write_str("~"),
        TokenKind::Ampersand => formatter.write_str("&"),
        TokenKind::Pipe => formatter.write_str("|"),
        TokenKind::Caret => formatter.write_str("^"),
        TokenKind::LogicalAnd => formatter.write_str(match language {
            HostLanguage::Fortran => ".and.",
            HostLanguage::C | HostLanguage::Cpp => "&&",
        }),
        TokenKind::LogicalOr => formatter.write_str(match language {
            HostLanguage::Fortran => ".or.",
            HostLanguage::C | HostLanguage::Cpp => "||",
        }),
        TokenKind::LogicalEqv => formatter.write_str(".eqv."),
        TokenKind::LogicalNeqv => formatter.write_str(".neqv."),
        TokenKind::Equal => formatter.write_str("="),
        TokenKind::EqualEqual => formatter.write_str("=="),
        TokenKind::NotEqual => formatter.write_str(match language {
            HostLanguage::Fortran => "/=",
            HostLanguage::C | HostLanguage::Cpp => "!=",
        }),
        TokenKind::Less => formatter.write_str("<"),
        TokenKind::LessEqual => formatter.write_str("<="),
        TokenKind::Greater => formatter.write_str(">"),
        TokenKind::GreaterEqual => formatter.write_str(">="),
        TokenKind::ShiftLeft => formatter.write_str("<<"),
        TokenKind::ShiftRight => formatter.write_str(">>"),
        TokenKind::PlusPlus => formatter.write_str("++"),
        TokenKind::MinusMinus => formatter.write_str("--"),
        TokenKind::PlusEqual => formatter.write_str("+="),
        TokenKind::MinusEqual => formatter.write_str("-="),
        TokenKind::StarEqual => formatter.write_str("*="),
        TokenKind::SlashEqual => formatter.write_str("/="),
        TokenKind::RemainderEqual => formatter.write_str("%="),
        TokenKind::AmpersandEqual => formatter.write_str("&="),
        TokenKind::PipeEqual => formatter.write_str("|="),
        TokenKind::CaretEqual => formatter.write_str("^="),
        TokenKind::ShiftLeftEqual => formatter.write_str("<<="),
        TokenKind::ShiftRightEqual => formatter.write_str(">>="),
        TokenKind::End => Err(fmt::Error),
    }
}

fn write_literal(
    formatter: &mut fmt::Formatter<'_>,
    literal: Literal,
    language: HostLanguage,
) -> fmt::Result {
    fmt::Display::fmt(
        &Expr::new(Span::start_of(""), ExprKind::Literal(literal)).canonical(language),
        formatter,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_and_cpp_keywords_are_lexed_as_typed_words() {
        let c = TypeName::parse("const unsigned long *", HostLanguage::C).unwrap();
        assert_eq!(c.to_string(), "const unsigned long *");

        let cpp =
            TypeName::parse("std::vector<std::pair<int, 4>> const &", HostLanguage::Cpp).unwrap();
        assert!(
            cpp.tokens()
                .iter()
                .all(|token| !matches!(token, TokenKind::End))
        );
        assert_eq!(
            cpp.to_string(),
            "std :: vector < std :: pair < int , 4 >> const &"
        );
    }

    #[test]
    fn fortran_type_selector_is_balanced_and_typed() {
        let name = TypeName::parse("integer(kind=8)", HostLanguage::Fortran).unwrap();
        assert_eq!(name.to_string(), "integer ( kind = 8 )");

        let historical = TypeName::parse("integer*4", HostLanguage::Fortran).unwrap();
        assert_eq!(historical.to_string(), "integer * 4");
    }

    #[test]
    fn malformed_type_syntax_is_a_hard_error() {
        for source in [
            "",
            "int)",
            "type(name",
            "int @",
            "vector<int",
            "int, long",
            "int +",
        ] {
            assert!(
                TypeName::parse(source, HostLanguage::Cpp).is_err(),
                "`{source}` must fail"
            );
        }
    }

    #[test]
    fn expression_token_bags_are_not_type_names() {
        for source in [
            "int + garbage",
            "foo = bar",
            "int ? x : y",
            "int && &&",
            "int (value = other)",
            "int [N +]",
        ] {
            assert!(
                TypeName::parse(source, HostLanguage::Cpp).is_err(),
                "expression-like token bag `{source}` must fail"
            );
        }
    }

    #[test]
    fn c_and_cpp_abstract_declarators_remain_valid() {
        for source in [
            "int (*)(double, const char *)",
            "unsigned long (*)[N + 1]",
            "void (*)(int (*)(double))",
            "_Atomic(int) *",
            "typeof(value + 1) *",
        ] {
            assert!(
                TypeName::parse(source, HostLanguage::C).is_ok(),
                "valid C type `{source}` was rejected"
            );
        }

        for source in [
            "std::array<std::pair<int, long>, N + 1> const &",
            "void (*)(std::vector<int> const &, int (*)[4])",
            "int (Widget::*)(double) const",
            "decltype((value)) &&",
            "std::integral_constant<int, (N > 0 ? N : 1)>",
        ] {
            assert!(
                TypeName::parse(source, HostLanguage::Cpp).is_ok(),
                "valid C++ type `{source}` was rejected"
            );
        }
    }

    #[test]
    fn malformed_cpp_template_arguments_are_hard_errors() {
        for source in [
            "f<int +>",
            "f<+>",
            "f<int && &&>",
            "outer<inner<int +>>",
            "outer<int, foo =>",
        ] {
            assert!(
                TypeName::parse(source, HostLanguage::Cpp).is_err(),
                "malformed template-id `{source}` must fail"
            );
        }
    }

    #[test]
    fn fortran_type_forms_remain_structured() {
        for source in [
            "double precision",
            "type(my_type)",
            "class(*)",
            "character(len=*, kind=kind('a'))",
            "integer(kind=selected_int_kind(9))",
        ] {
            assert!(
                TypeName::parse(source, HostLanguage::Fortran).is_ok(),
                "valid Fortran type `{source}` was rejected"
            );
        }

        for source in ["integer + garbage", "type(foo) = bar", "real ? x : y"] {
            assert!(
                TypeName::parse(source, HostLanguage::Fortran).is_err(),
                "malformed Fortran type `{source}` must fail"
            );
        }
    }

    #[test]
    fn excessive_type_syntax_nesting_is_a_hard_error() {
        let deeply_parenthesized = format!(
            "decltype({}value{})",
            "(".repeat(TypeSyntaxParser::MAX_NESTING_DEPTH + 1),
            ")".repeat(TypeSyntaxParser::MAX_NESTING_DEPTH + 1)
        );
        assert!(TypeName::parse(&deeply_parenthesized, HostLanguage::Cpp).is_err());

        let mut deeply_templated = "int".to_string();
        for _ in 0..=TypeSyntaxParser::MAX_NESTING_DEPTH {
            deeply_templated = format!("wrapper<{deeply_templated}>");
        }
        assert!(TypeName::parse(&deeply_templated, HostLanguage::Cpp).is_err());
    }
}
