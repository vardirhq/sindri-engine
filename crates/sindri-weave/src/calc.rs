//! `calc()`, `min()`, `max()` and `clamp()`: arithmetic on lengths, as CSS
//! does it.
//!
//! Lengths can be added and subtracted, and multiplied or divided by plain
//! numbers, so the linear part of whatever is written reduces to so many
//! overlay units, so many pixels, so much of the viewport's width and height,
//! and so much of the containing box: a [`Mix`]. `min()`, `max()` and
//! `clamp()` are not linear, so they are kept as nodes over their arguments and
//! chosen between when the viewport and the box are known, as a single length
//! is resolved then. `clamp(MIN, VAL, MAX)` is `max(MIN, min(VAL, MAX))`.
//!
//! That is what a responsive size needs without a media query:
//! `width: clamp(240px, 40vw, 360px)` is 40% of the screen, never narrower
//! than a thumb's reach or wider than a card should be.

/// A length made of several units at once: what a `calc()` comes to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Mix {
    pub overlay: f32,
    pub pixels: f32,
    pub view_width: f32,
    pub view_height: f32,
    pub percent: f32,
    /// Whether any part is a percentage, which then needs a box to resolve.
    pub relative: bool,
}

impl Mix {
    fn scaled(self, by: f32) -> Self {
        Self {
            overlay: self.overlay * by,
            pixels: self.pixels * by,
            view_width: self.view_width * by,
            view_height: self.view_height * by,
            percent: self.percent * by,
            relative: self.relative,
        }
    }

    fn plus(self, other: Self) -> Self {
        Self {
            overlay: self.overlay + other.overlay,
            pixels: self.pixels + other.pixels,
            view_width: self.view_width + other.view_width,
            view_height: self.view_height + other.view_height,
            percent: self.percent + other.percent,
            relative: self.relative || other.relative,
        }
    }
}

/// One side of an operation: a plain number, or a length.
#[derive(Clone, Debug, PartialEq)]
enum Term {
    Number(f32),
    Length(Calc),
}

/// What a `calc()` and its kin come to: a linear mix of units where it can,
/// and a choice between such mixes where `min()`, `max()` or `clamp()` asks
/// for one.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Calc {
    Linear(Mix),
    /// Several lengths added: needed only once one of them is a choice.
    Sum(Vec<Calc>),
    /// A choice, scaled by a number.
    Scaled(Box<Calc>, f32),
    Min(Vec<Calc>),
    Max(Vec<Calc>),
}

impl Calc {
    /// The length in overlay units, given how each linear mix resolves.
    /// `None` when a mix cannot be resolved, such as a percentage with no box.
    pub(crate) fn resolve(&self, mix: &impl Fn(Mix) -> Option<f32>) -> Option<f32> {
        match self {
            Self::Linear(linear) => mix(*linear),
            Self::Sum(parts) => parts.iter().map(|part| part.resolve(mix)).sum(),
            Self::Scaled(inner, by) => Some(inner.resolve(mix)? * by),
            Self::Min(parts) => parts
                .iter()
                .map(|part| part.resolve(mix))
                .try_fold(f32::INFINITY, |least, value| Some(least.min(value?))),
            Self::Max(parts) => parts
                .iter()
                .map(|part| part.resolve(mix))
                .try_fold(f32::NEG_INFINITY, |most, value| Some(most.max(value?))),
        }
    }

    fn plus(self, other: Self) -> Self {
        match (self, other) {
            (Self::Linear(a), Self::Linear(b)) => Self::Linear(a.plus(b)),
            (Self::Sum(mut parts), other) => {
                parts.push(other);
                Self::Sum(parts)
            }
            (one, other) => Self::Sum(vec![one, other]),
        }
    }

    fn scaled(self, by: f32) -> Self {
        match self {
            Self::Linear(mix) => Self::Linear(mix.scaled(by)),
            Self::Sum(parts) => Self::Sum(parts.into_iter().map(|part| part.scaled(by)).collect()),
            Self::Scaled(inner, was) => Self::Scaled(inner, was * by),
            choice => Self::Scaled(Box::new(choice), by),
        }
    }
}

/// `calc(...)`, `min(...)`, `max(...)` or `clamp(...)`, read. `None` for
/// anything CSS would not accept: an unknown unit, two lengths multiplied,
/// division by a length or by zero, a bare number where a length belongs, a
/// number and a length compared, or `clamp()` without its three arguments.
pub(crate) fn parse(text: &str) -> Option<Calc> {
    let tokens = tokenize(text.trim())?;
    let mut position = 0;
    let term = operand(&tokens, &mut position)?;
    if position != tokens.len() || !matches!(tokens.first(), Some(Token::Function(_))) {
        return None;
    }
    match term {
        Term::Length(calc) => Some(calc),
        // `calc(0)` is a length of nothing, as a bare `0` is.
        Term::Number(0.0) => Some(Calc::Linear(Mix::default())),
        Term::Number(_) => None,
    }
}

/// Whether `text` is written as one of the functions this reads.
pub(crate) fn is_function(text: &str) -> bool {
    let text = text.trim_start();
    FUNCTIONS.iter().any(|function| {
        text.strip_prefix(function.name())
            .is_some_and(|rest| rest.starts_with('('))
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Function {
    Calc,
    Min,
    Max,
    Clamp,
}

const FUNCTIONS: [Function; 4] = [
    Function::Calc,
    Function::Min,
    Function::Max,
    Function::Clamp,
];

impl Function {
    const fn name(self) -> &'static str {
        match self {
            Self::Calc => "calc",
            Self::Min => "min",
            Self::Max => "max",
            Self::Clamp => "clamp",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Value(Term),
    Plus,
    Minus,
    Times,
    Divide,
    Open,
    /// A function's name and its `(`.
    Function(Function),
    Comma,
    Close,
}

fn tokenize(text: &str) -> Option<Vec<Token>> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        match c {
            _ if c.is_whitespace() => at += 1,
            '+' | '-' if starts_number(&chars, at, &tokens) => {
                let (term, next) = value(&chars, at)?;
                tokens.push(Token::Value(term));
                at = next;
            }
            '+' => {
                tokens.push(Token::Plus);
                at += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                at += 1;
            }
            '*' => {
                tokens.push(Token::Times);
                at += 1;
            }
            '/' => {
                tokens.push(Token::Divide);
                at += 1;
            }
            '(' => {
                tokens.push(Token::Open);
                at += 1;
            }
            ')' => {
                tokens.push(Token::Close);
                at += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                at += 1;
            }
            _ if c.is_ascii_digit() || c == '.' => {
                let (term, next) = value(&chars, at)?;
                tokens.push(Token::Value(term));
                at = next;
            }
            _ if c.is_ascii_alphabetic() => {
                let rest: String = chars[at..].iter().take(6).collect();
                let function = FUNCTIONS.into_iter().find(|function| {
                    rest.strip_prefix(function.name())
                        .is_some_and(|after| after.starts_with('('))
                })?;
                tokens.push(Token::Function(function));
                at += function.name().len() + 1;
            }
            _ => return None,
        }
    }
    Some(tokens)
}

/// Whether a sign at `at` belongs to a number (`-2px`) rather than being an
/// operator: it does when nothing, an operator or `(` is before it and a
/// digit follows. CSS needs spaces around `+` and `-` operators, so `a -b`
/// cannot be read any other way.
fn starts_number(chars: &[char], at: usize, tokens: &[Token]) -> bool {
    let operand_expected = !matches!(tokens.last(), Some(Token::Value(_) | Token::Close));
    operand_expected
        && chars
            .get(at + 1)
            .is_some_and(|next| next.is_ascii_digit() || *next == '.')
}

/// A number and its unit, starting at `at`.
fn value(chars: &[char], at: usize) -> Option<(Term, usize)> {
    let mut end = at;
    if matches!(chars.get(end), Some('+' | '-')) {
        end += 1;
    }
    while chars
        .get(end)
        .is_some_and(|c| c.is_ascii_digit() || *c == '.')
    {
        end += 1;
    }
    let number: f32 = chars[at..end].iter().collect::<String>().parse().ok()?;
    if !number.is_finite() {
        return None;
    }
    let unit_start = end;
    while chars
        .get(end)
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == '%')
    {
        end += 1;
    }
    let unit: String = chars[unit_start..end].iter().collect();
    let length = |mix: Mix| Term::Length(Calc::Linear(mix));
    let term = match unit.as_str() {
        "" => Term::Number(number),
        "px" => length(Mix {
            pixels: number,
            ..Mix::default()
        }),
        "vw" => length(Mix {
            view_width: number,
            ..Mix::default()
        }),
        "vh" => length(Mix {
            view_height: number,
            ..Mix::default()
        }),
        "%" => length(Mix {
            percent: number,
            relative: true,
            ..Mix::default()
        }),
        // The overlay's own unit, written out: `calc(0.5u + 10px)`.
        "u" => length(Mix {
            overlay: number,
            ..Mix::default()
        }),
        _ => return None,
    };
    Some((term, end))
}

fn sum(tokens: &[Token], at: &mut usize) -> Option<Term> {
    let mut total = product(tokens, at)?;
    while let Some(sign @ (Token::Plus | Token::Minus)) = tokens.get(*at) {
        let negative = *sign == Token::Minus;
        *at += 1;
        let next = product(tokens, at)?;
        let next = if negative { scale(next, -1.0) } else { next };
        total = match (total, next) {
            (Term::Number(a), Term::Number(b)) => Term::Number(a + b),
            (Term::Length(a), Term::Length(b)) => Term::Length(a.plus(b)),
            // CSS will not add a number to a length.
            _ => return None,
        };
    }
    Some(total)
}

fn product(tokens: &[Token], at: &mut usize) -> Option<Term> {
    let mut total = operand(tokens, at)?;
    while let Some(op @ (Token::Times | Token::Divide)) = tokens.get(*at) {
        let dividing = *op == Token::Divide;
        *at += 1;
        let next = operand(tokens, at)?;
        total = match (total, next, dividing) {
            (Term::Number(a), Term::Number(b), false) => Term::Number(a * b),
            (Term::Number(a), Term::Number(b), true) if b != 0.0 => Term::Number(a / b),
            (Term::Length(a), Term::Number(b), false)
            | (Term::Number(b), Term::Length(a), false) => Term::Length(a.scaled(b)),
            (Term::Length(a), Term::Number(b), true) if b != 0.0 => Term::Length(a.scaled(1.0 / b)),
            _ => return None,
        };
    }
    Some(total)
}

fn operand(tokens: &[Token], at: &mut usize) -> Option<Term> {
    match tokens.get(*at)? {
        Token::Value(term) => {
            *at += 1;
            Some(term.clone())
        }
        Token::Open | Token::Function(Function::Calc) => {
            *at += 1;
            let inner = sum(tokens, at)?;
            (tokens.get(*at) == Some(&Token::Close)).then(|| {
                *at += 1;
                inner
            })
        }
        Token::Function(function) => {
            let function = *function;
            *at += 1;
            let mut arguments = vec![sum(tokens, at)?];
            while tokens.get(*at) == Some(&Token::Comma) {
                *at += 1;
                arguments.push(sum(tokens, at)?);
            }
            if tokens.get(*at) != Some(&Token::Close) {
                return None;
            }
            *at += 1;
            choose(function, arguments)
        }
        _ => None,
    }
}

/// `min()`, `max()` or `clamp()` over its arguments, which must all be
/// numbers or all lengths. Numbers are chosen between at once.
fn choose(function: Function, arguments: Vec<Term>) -> Option<Term> {
    if function == Function::Clamp && arguments.len() != 3 {
        return None;
    }
    if arguments.iter().all(|term| matches!(term, Term::Number(_))) {
        let numbers: Vec<f32> = arguments
            .iter()
            .map(|term| match term {
                Term::Number(number) => *number,
                Term::Length(_) => unreachable!("all numbers"),
            })
            .collect();
        let least = |values: &[f32]| values.iter().copied().fold(f32::INFINITY, f32::min);
        let most = |values: &[f32]| values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        return Some(Term::Number(match function {
            Function::Min => least(&numbers),
            Function::Max => most(&numbers),
            Function::Clamp => numbers[0].max(numbers[1].min(numbers[2])),
            Function::Calc => return None,
        }));
    }
    let lengths = arguments
        .into_iter()
        .map(|term| match term {
            Term::Length(calc) => Some(calc),
            Term::Number(_) => None,
        })
        .collect::<Option<Vec<Calc>>>()?;
    Some(Term::Length(match function {
        Function::Min => Calc::Min(lengths),
        Function::Max => Calc::Max(lengths),
        Function::Clamp => {
            let mut lengths = lengths.into_iter();
            let (low, value, high) = (lengths.next()?, lengths.next()?, lengths.next()?);
            Calc::Max(vec![low, Calc::Min(vec![value, high])])
        }
        Function::Calc => return None,
    }))
}

fn scale(term: Term, by: f32) -> Term {
    match term {
        Term::Number(number) => Term::Number(number * by),
        Term::Length(mix) => Term::Length(mix.scaled(by)),
    }
}

#[cfg(test)]
mod tests;
