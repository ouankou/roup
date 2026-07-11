use crate::host::ast::*;
use std::fmt;

pub struct CanonicalDisplay<'a> {
    expression: &'a Expr,
    language: HostLanguage,
}

impl Expr {
    pub fn canonical(&self, language: HostLanguage) -> CanonicalDisplay<'_> {
        CanonicalDisplay {
            expression: self,
            language,
        }
    }
}

impl fmt::Display for CanonicalDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Renderer {
            language: self.language,
        }
        .expression(f, self.expression, 0)
    }
}

struct Renderer {
    language: HostLanguage,
}

impl Renderer {
    fn expression(&self, f: &mut fmt::Formatter<'_>, expression: &Expr, parent: u8) -> fmt::Result {
        let own = self.precedence(expression);
        let needs_parentheses = own < parent;
        if needs_parentheses {
            f.write_str("(")?;
        }
        match &expression.kind {
            ExprKind::Literal(literal) => self.literal(f, literal)?,
            ExprKind::Name(name) => self.qualified_name(f, name)?,
            ExprKind::CppTemplateId {
                template,
                arguments,
            } => {
                self.expression(f, template, 100)?;
                f.write_str("<")?;
                for (index, argument) in arguments.iter().enumerate() {
                    if index != 0 {
                        f.write_str(", ")?;
                    }
                    match argument {
                        CppTemplateArgument::Type(type_name) => write!(f, "{type_name}")?,
                        CppTemplateArgument::Expression(expression)
                        | CppTemplateArgument::Ambiguous { expression, .. } => {
                            self.expression(f, expression, 0)?;
                        }
                    }
                }
                f.write_str(">")?;
            }
            ExprKind::LegacyQualifiedInteger { qualifier, value } => {
                write!(f, "{qualifier}::")?;
                self.integer(f, value)?;
            }
            ExprKind::Parenthesized(inner) => {
                f.write_str("(")?;
                self.expression(f, inner, 0)?;
                f.write_str(")")?;
            }
            ExprKind::Unary { op, operand } => {
                f.write_str(self.unary_operator(*op))?;
                self.expression(f, operand, self.unary_precedence(*op))?;
            }
            ExprKind::FortranDefinedUnary { operator, operand } => {
                write!(f, ".{operator}. ")?;
                self.expression(f, operand, 17)?;
            }
            ExprKind::Binary { op, left, right } => {
                let precedence = self.binary_precedence(*op);
                if *op == BinaryOp::Power {
                    self.expression(f, left, precedence + 1)?;
                    write!(f, " {} ", self.binary_operator(*op))?;
                    self.expression(f, right, precedence)?;
                } else {
                    self.expression(f, left, precedence)?;
                    write!(f, " {} ", self.binary_operator(*op))?;
                    self.expression(f, right, precedence + 1)?;
                }
            }
            ExprKind::FortranDefinedBinary {
                operator,
                left,
                right,
            } => {
                self.expression(f, left, 1)?;
                write!(f, " .{operator}. ")?;
                self.expression(f, right, 2)?;
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                self.expression(f, condition, 4)?;
                f.write_str(" ? ")?;
                self.expression(f, then_expr, 0)?;
                f.write_str(" : ")?;
                self.expression(f, else_expr, 2)?;
            }
            ExprKind::Assignment { op, target, value } => {
                self.expression(f, target, 3)?;
                write!(f, " {} ", assignment_operator(*op))?;
                self.expression(f, value, 2)?;
            }
            ExprKind::Call { callee, arguments } => {
                self.expression(f, callee, 100)?;
                f.write_str("(")?;
                for (index, argument) in arguments.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    self.expression(f, argument, 2)?;
                }
                f.write_str(")")?;
            }
            ExprKind::Subscript { base, subscript } => {
                self.expression(f, base, 100)?;
                f.write_str("[")?;
                self.subscript(f, subscript)?;
                f.write_str("]")?;
            }
            ExprKind::Member {
                base,
                access,
                member,
            } => {
                self.expression(f, base, 100)?;
                f.write_str(match access {
                    MemberAccess::Dot => ".",
                    MemberAccess::Arrow => "->",
                    MemberAccess::Scope => "::",
                    MemberAccess::FortranComponent => "%",
                })?;
                write!(f, "{member}")?;
            }
            ExprKind::Postfix { op, operand } => {
                self.expression(f, operand, 100)?;
                f.write_str(match op {
                    PostfixOp::Increment => "++",
                    PostfixOp::Decrement => "--",
                })?;
            }
            ExprKind::FortranApply {
                designator,
                arguments,
            } => {
                self.expression(f, designator, 100)?;
                f.write_str("(")?;
                for (index, argument) in arguments.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    match argument {
                        FortranArgument::Positional(value) => self.expression(f, value, 0)?,
                        FortranArgument::Keyword { name, value } => {
                            write!(f, "{name}=")?;
                            self.expression(f, value, 0)?;
                        }
                        FortranArgument::Section(section) => self.section(f, section)?,
                    }
                }
                f.write_str(")")?;
            }
        }
        if needs_parentheses {
            f.write_str(")")?;
        }
        Ok(())
    }

    fn literal(&self, f: &mut fmt::Formatter<'_>, literal: &Literal) -> fmt::Result {
        match literal {
            Literal::Boolean(value) => match self.language {
                HostLanguage::Fortran => f.write_str(if *value { ".true." } else { ".false." }),
                HostLanguage::C | HostLanguage::Cpp => {
                    f.write_str(if *value { "true" } else { "false" })
                }
            },
            Literal::NullPointer => f.write_str("nullptr"),
            Literal::Integer(value) => self.integer(f, value),
            Literal::Real(value) => self.real(f, value),
            Literal::Character(value) => {
                f.write_str(character_prefix(value.encoding))?;
                f.write_str("'")?;
                write_escaped_c_char(f, value.value, '\'')?;
                f.write_str("'")
            }
            Literal::String(value) => {
                if value.encoding == CharacterEncoding::Fortran {
                    let delimiter = match value.delimiter {
                        crate::host::StringDelimiter::SingleQuote => '\'',
                        crate::host::StringDelimiter::DoubleQuote => '"',
                    };
                    write!(f, "{delimiter}")?;
                    for ch in value.value.chars() {
                        if ch == delimiter {
                            write!(f, "{delimiter}{delimiter}")?;
                        } else {
                            write!(f, "{ch}")?;
                        }
                    }
                    write!(f, "{delimiter}")
                } else {
                    f.write_str(character_prefix(value.encoding))?;
                    f.write_str("\"")?;
                    for ch in value.value.chars() {
                        write_escaped_c_char(f, ch, '"')?;
                    }
                    f.write_str("\"")
                }
            }
        }
    }

    fn integer(&self, f: &mut fmt::Formatter<'_>, literal: &IntegerLiteral) -> fmt::Result {
        match literal.base {
            IntegerBase::Binary => write!(f, "0b{:b}", literal.value)?,
            IntegerBase::Octal => write!(f, "0{:o}", literal.value)?,
            IntegerBase::Decimal => write!(f, "{}", literal.value)?,
            IntegerBase::Hexadecimal => write!(f, "0x{:x}", literal.value)?,
        }
        match &literal.suffix {
            IntegerSuffix::None => {}
            IntegerSuffix::C(suffix) => {
                if suffix.unsigned {
                    f.write_str("u")?;
                }
                f.write_str(match suffix.width {
                    CIntegerWidth::Default => "",
                    CIntegerWidth::Long => "l",
                    CIntegerWidth::LongLong => "ll",
                })?;
                if suffix.size_t {
                    f.write_str("z")?;
                }
            }
            IntegerSuffix::Fortran(kind) => {
                f.write_str("_")?;
                self.fortran_kind(f, kind)?;
            }
        }
        Ok(())
    }

    fn real(&self, f: &mut fmt::Formatter<'_>, literal: &RealLiteral) -> fmt::Result {
        let digits = literal.coefficient.to_string();
        let fractional_digits = literal.fractional_digits as usize;
        if fractional_digits == 0 {
            f.write_str(&digits)?;
            if literal.exponent.is_none() {
                f.write_str(".0")?;
            }
        } else if digits.len() <= fractional_digits {
            f.write_str("0.")?;
            for _ in 0..(fractional_digits - digits.len()) {
                f.write_str("0")?;
            }
            f.write_str(&digits)?;
        } else {
            let split = digits.len() - fractional_digits;
            f.write_str(&digits[..split])?;
            f.write_str(".")?;
            f.write_str(&digits[split..])?;
        }
        if let Some(exponent) = &literal.exponent {
            f.write_str(match exponent.kind {
                RealExponentKind::E => "e",
                RealExponentKind::D => "d",
            })?;
            if exponent.negative {
                f.write_str("-")?;
            }
            write!(f, "{}", exponent.magnitude)?;
        }
        match &literal.suffix {
            RealSuffix::C(CRealSuffix::Float) => f.write_str("f")?,
            RealSuffix::C(CRealSuffix::Double) => {}
            RealSuffix::C(CRealSuffix::LongDouble) => f.write_str("l")?,
            RealSuffix::Fortran(Some(kind)) => {
                f.write_str("_")?;
                self.fortran_kind(f, kind)?;
            }
            RealSuffix::Fortran(None) => {}
        }
        Ok(())
    }

    fn fortran_kind(&self, f: &mut fmt::Formatter<'_>, kind: &FortranKind) -> fmt::Result {
        match kind {
            FortranKind::Numeric(value) => write!(f, "{value}"),
            FortranKind::Named(name) => write!(f, "{name}"),
        }
    }

    fn qualified_name(&self, f: &mut fmt::Formatter<'_>, name: &QualifiedName) -> fmt::Result {
        if name.global {
            f.write_str("::")?;
        }
        for (index, segment) in name.segments.iter().enumerate() {
            if index > 0 {
                f.write_str("::")?;
            }
            write!(f, "{segment}")?;
        }
        Ok(())
    }

    fn subscript(&self, f: &mut fmt::Formatter<'_>, subscript: &Subscript) -> fmt::Result {
        match subscript {
            Subscript::Index(index) => self.expression(f, index, 0),
            Subscript::Section(section) => self.section(f, section),
        }
    }

    fn section(&self, f: &mut fmt::Formatter<'_>, section: &ArraySection) -> fmt::Result {
        if let Some(lower) = &section.lower {
            self.expression(f, lower, 0)?;
        }
        f.write_str(":")?;
        if let Some(upper) = &section.upper_or_length {
            self.expression(f, upper, 0)?;
        }
        if section.stride.is_some() {
            f.write_str(":")?;
            if let Some(stride) = &section.stride {
                self.expression(f, stride, 0)?;
            }
        }
        Ok(())
    }

    fn precedence(&self, expression: &Expr) -> u8 {
        match &expression.kind {
            ExprKind::Literal(_)
            | ExprKind::Name(_)
            | ExprKind::LegacyQualifiedInteger { .. }
            | ExprKind::Parenthesized(_) => 110,
            ExprKind::Call { .. }
            | ExprKind::CppTemplateId { .. }
            | ExprKind::Subscript { .. }
            | ExprKind::Member { .. }
            | ExprKind::Postfix { .. }
            | ExprKind::FortranApply { .. } => 100,
            ExprKind::Unary { op, .. } => self.unary_precedence(*op),
            ExprKind::FortranDefinedUnary { .. } => 17,
            ExprKind::Binary { op, .. } => self.binary_precedence(*op),
            ExprKind::FortranDefinedBinary { .. } => 1,
            ExprKind::Conditional { .. } => 3,
            ExprKind::Assignment { .. } => 2,
        }
    }

    fn unary_precedence(&self, op: UnaryOp) -> u8 {
        match (self.language, op) {
            (HostLanguage::Fortran, UnaryOp::LogicalNot) => 7,
            (HostLanguage::Fortran, _) => 15,
            _ => 90,
        }
    }

    fn binary_precedence(&self, op: BinaryOp) -> u8 {
        match self.language {
            HostLanguage::C | HostLanguage::Cpp => match op {
                BinaryOp::Comma => 1,
                BinaryOp::LogicalOr => 4,
                BinaryOp::LogicalAnd => 6,
                BinaryOp::BitwiseOr => 8,
                BinaryOp::BitwiseXor => 10,
                BinaryOp::BitwiseAnd => 12,
                BinaryOp::Equal | BinaryOp::NotEqual => 14,
                BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual => 16,
                BinaryOp::ShiftLeft | BinaryOp::ShiftRight => 18,
                BinaryOp::Add | BinaryOp::Subtract => 20,
                BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder => 22,
                BinaryOp::Power
                | BinaryOp::Concatenate
                | BinaryOp::LogicalEqv
                | BinaryOp::LogicalNeqv => 0,
            },
            HostLanguage::Fortran => match op {
                BinaryOp::LogicalEqv | BinaryOp::LogicalNeqv => 2,
                BinaryOp::LogicalOr => 4,
                BinaryOp::LogicalAnd => 6,
                BinaryOp::Equal
                | BinaryOp::NotEqual
                | BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual => 8,
                BinaryOp::Concatenate => 10,
                BinaryOp::Add | BinaryOp::Subtract => 12,
                BinaryOp::Multiply | BinaryOp::Divide => 14,
                BinaryOp::Power => 16,
                BinaryOp::Remainder
                | BinaryOp::ShiftLeft
                | BinaryOp::ShiftRight
                | BinaryOp::BitwiseAnd
                | BinaryOp::BitwiseXor
                | BinaryOp::BitwiseOr
                | BinaryOp::Comma => 0,
            },
        }
    }

    fn unary_operator(&self, op: UnaryOp) -> &'static str {
        match op {
            UnaryOp::Plus => "+",
            UnaryOp::Minus => "-",
            UnaryOp::LogicalNot if self.language == HostLanguage::Fortran => ".not. ",
            UnaryOp::LogicalNot => "!",
            UnaryOp::BitwiseNot => "~",
            UnaryOp::Dereference => "*",
            UnaryOp::AddressOf => "&",
            UnaryOp::PreIncrement => "++",
            UnaryOp::PreDecrement => "--",
        }
    }

    fn binary_operator(&self, op: BinaryOp) -> &'static str {
        match op {
            BinaryOp::Power => "**",
            BinaryOp::Multiply => "*",
            BinaryOp::Divide => "/",
            BinaryOp::Remainder => "%",
            BinaryOp::Add => "+",
            BinaryOp::Subtract => "-",
            BinaryOp::Concatenate => "//",
            BinaryOp::ShiftLeft => "<<",
            BinaryOp::ShiftRight => ">>",
            BinaryOp::Less => "<",
            BinaryOp::LessEqual => "<=",
            BinaryOp::Greater => ">",
            BinaryOp::GreaterEqual => ">=",
            BinaryOp::Equal => "==",
            BinaryOp::NotEqual if self.language == HostLanguage::Fortran => "/=",
            BinaryOp::NotEqual => "!=",
            BinaryOp::BitwiseAnd => "&",
            BinaryOp::BitwiseXor => "^",
            BinaryOp::BitwiseOr => "|",
            BinaryOp::LogicalAnd if self.language == HostLanguage::Fortran => ".and.",
            BinaryOp::LogicalAnd => "&&",
            BinaryOp::LogicalOr if self.language == HostLanguage::Fortran => ".or.",
            BinaryOp::LogicalOr => "||",
            BinaryOp::LogicalEqv => ".eqv.",
            BinaryOp::LogicalNeqv => ".neqv.",
            BinaryOp::Comma => ",",
        }
    }
}

fn assignment_operator(op: AssignmentOp) -> &'static str {
    match op {
        AssignmentOp::Assign => "=",
        AssignmentOp::AddAssign => "+=",
        AssignmentOp::SubtractAssign => "-=",
        AssignmentOp::MultiplyAssign => "*=",
        AssignmentOp::DivideAssign => "/=",
        AssignmentOp::RemainderAssign => "%=",
        AssignmentOp::ShiftLeftAssign => "<<=",
        AssignmentOp::ShiftRightAssign => ">>=",
        AssignmentOp::BitwiseAndAssign => "&=",
        AssignmentOp::BitwiseXorAssign => "^=",
        AssignmentOp::BitwiseOrAssign => "|=",
    }
}

fn character_prefix(encoding: CharacterEncoding) -> &'static str {
    match encoding {
        CharacterEncoding::Ordinary | CharacterEncoding::Fortran => "",
        CharacterEncoding::Utf8 => "u8",
        CharacterEncoding::Utf16 => "u",
        CharacterEncoding::Utf32 => "U",
        CharacterEncoding::Wide => "L",
    }
}

fn write_escaped_c_char(f: &mut fmt::Formatter<'_>, value: char, delimiter: char) -> fmt::Result {
    match value {
        '\\' => f.write_str("\\\\"),
        '\n' => f.write_str("\\n"),
        '\r' => f.write_str("\\r"),
        '\t' => f.write_str("\\t"),
        // Use all three octal digits so a following decimal digit cannot be
        // absorbed into this escape when the canonical text is reparsed.
        '\0' => f.write_str("\\000"),
        '\u{7}' => f.write_str("\\a"),
        '\u{8}' => f.write_str("\\b"),
        '\u{b}' => f.write_str("\\v"),
        '\u{c}' => f.write_str("\\f"),
        ch if ch == delimiter => write!(f, "\\{ch}"),
        ch if ch.is_control() => write!(f, "\\u{:04x}", ch as u32),
        ch => write!(f, "{ch}"),
    }
}

#[cfg(test)]
mod tests {
    use crate::host::{HostLanguage, parse_expression};

    #[test]
    fn canonical_c_render_preserves_tree_semantics() {
        let parsed = parse_expression("a+(b*c)", HostLanguage::C).unwrap();
        assert_eq!(parsed.canonical(HostLanguage::C).to_string(), "a + (b * c)");
    }

    #[test]
    fn canonical_cpp_render_handles_full_postfix_chain() {
        let parsed = parse_expression("::ns::f(x)->member[1:4]", HostLanguage::Cpp).unwrap();
        assert_eq!(
            parsed.canonical(HostLanguage::Cpp).to_string(),
            "::ns::f(x)->member[1:4]"
        );
    }

    #[test]
    fn canonical_fortran_render_uses_language_operators() {
        let parsed =
            parse_expression(".not. a(1:n:2)%ready .or. .false.", HostLanguage::Fortran).unwrap();
        assert_eq!(
            parsed.canonical(HostLanguage::Fortran).to_string(),
            ".not. a(1:n:2)%ready .or. .false."
        );
    }

    #[test]
    fn rendering_round_trips_to_same_typed_tree_shape() {
        let first = parse_expression("x = y ? f(a, b[1:3]) : z", HostLanguage::Cpp).unwrap();
        let rendered = first.canonical(HostLanguage::Cpp).to_string();
        let second = parse_expression(&rendered, HostLanguage::Cpp).unwrap();
        assert_eq!(first.kind, second.kind);
    }

    #[test]
    fn canonical_rendering_is_idempotent_for_every_language() {
        let cases = [
            (HostLanguage::C, "value=flag?data[index+1]:call(-x,y)"),
            (
                HostLanguage::Cpp,
                "::ns::factory(obj.member)->values[lower:length]",
            ),
            (
                HostLanguage::Fortran,
                ".not.array(1:n:2,:)%ready.or..false.",
            ),
        ];

        for (language, source) in cases {
            let first = parse_expression(source, language).unwrap();
            let canonical = first.canonical(language).to_string();
            let second = parse_expression(&canonical, language).unwrap();
            assert_eq!(
                second.canonical(language).to_string(),
                canonical,
                "canonical rendering was unstable for {language:?}"
            );
        }
    }

    #[test]
    fn canonical_string_escaping_does_not_merge_adjacent_digits() {
        let first = parse_expression("\"\\0001\"", HostLanguage::C).unwrap();
        let canonical = first.canonical(HostLanguage::C).to_string();
        assert_eq!(canonical, "\"\\0001\"");
        let second = parse_expression(&canonical, HostLanguage::C).unwrap();
        assert_eq!(first.kind, second.kind);
    }
}
