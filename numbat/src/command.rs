use crate::{
    parser::ParseErrorKind,
    resolver::{CodeSource, Resolver},
    span::{SourceCodePositition, Span},
    ParseError,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ListItems {
    Functions,
    Dimensions,
    Variables,
    Units,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command<'a> {
    Help,
    List { items: Option<ListItems> },
    Clear,
    Save { dst: &'a str },
    Quit,
}

/// For tracking spans. Contains `(start, start+len)` for each (whitespace-separated)
/// word in the input
fn get_word_boundaries(input: &str) -> Vec<(u32, u32)> {
    let mut word_boundaries = Vec::new();
    let mut prev_char_was_whitespace = true;
    let mut start_idx = 0;
    for (i, c) in input.char_indices() {
        if prev_char_was_whitespace && !c.is_whitespace() {
            start_idx = u32::try_from(i).unwrap();
        } else if !prev_char_was_whitespace && c.is_whitespace() {
            word_boundaries.push((start_idx, u32::try_from(i).unwrap()));
        }
        prev_char_was_whitespace = c.is_whitespace();
    }

    // if no whitespace after last word, need to add last word
    if !prev_char_was_whitespace {
        word_boundaries.push((start_idx, u32::try_from(input.len()).unwrap()));
    }

    word_boundaries
}

fn span_from_boundary((start, end): (u32, u32), code_source_id: usize) -> Span {
    Span {
        start: SourceCodePositition {
            byte: start,
            line: 1,
            position: start,
        },
        end: SourceCodePositition {
            byte: end,
            line: 1,
            position: end,
        },
        code_source_id,
    }
}

macro_rules! handle_arg_count {
    (count = 0, $words:expr, $word_boundaries:expr, $code_source_id:expr, "help", $if_ok:expr $(,)?) => {{
        if $words.next().is_some() {
            let start = $word_boundaries[1].0;
            let end = $word_boundaries.last().unwrap().1;
            return Some(Err(ParseError {
                kind: ParseErrorKind::InvalidCommand(
                    "`help` takes 0 arguments; use `info <item>` for information about an item",
                ),
                span: span_from_boundary((start, end), $code_source_id),
            }));
        }

        $if_ok
    }};
    (count = 0, $words:expr, $word_boundaries:expr, $code_source_id:expr, $command:literal, $if_ok:expr $(,)?) => {{
        if $words.next().is_some() {
            let start = $word_boundaries[1].0;
            let end = $word_boundaries.last().unwrap().1;
            return Some(Err(ParseError {
                kind: ParseErrorKind::InvalidCommand(concat!("`", $command, "` takes 0 arguments")),
                span: span_from_boundary((start, end), $code_source_id),
            }));
        }

        $if_ok
    }};
}

/// Attempt to parse the input as a command, such as "help", "list <args>", "quit", etc
///
/// Returns:
/// - `None`, if the input does not begin with a command keyword
/// - `Some(Ok(Command))`, if the input is a valid command
/// - `Some(Err(_))`, if the input starts with a valid command but has the wrong number
///   or kind of arguments, e.g. `list foobar`
pub fn parse_command<'a>(
    input: &'a str,
    resolver: &mut Resolver,
) -> Option<Result<Command<'a>, ParseError>> {
    let word_boundaries = get_word_boundaries(input);
    let code_source_id = resolver.add_code_source(CodeSource::Text, input);

    let mut words = input.split_whitespace();
    let Some(command_str) = words.next() else {
        // should never hit this branch in practice because all-whitespace inputs are
        // skipped over

        return Some(Err(ParseError {
            kind: ParseErrorKind::InvalidCommand("invalid empty command"),
            span: span_from_boundary((0, u32::try_from(input.len()).unwrap()), code_source_id),
        }));
    };

    let command = match command_str {
        "help" => handle_arg_count!(
            count = 0,
            words,
            word_boundaries,
            code_source_id,
            "help",
            Command::Help,
        ),
        "clear" => handle_arg_count!(
            count = 0,
            words,
            word_boundaries,
            code_source_id,
            "clear",
            Command::Clear,
        ),
        "quit" => handle_arg_count!(
            count = 0,
            words,
            word_boundaries,
            code_source_id,
            "quit",
            Command::Quit,
        ),
        "exit" => handle_arg_count!(
            count = 0,
            words,
            word_boundaries,
            code_source_id,
            "exit",
            Command::Quit,
        ),
        "list" | "ls" => {
            let items = match words.next() {
                None => None,
                Some("functions") => Some(ListItems::Functions),
                Some("dimensions") => Some(ListItems::Dimensions),
                Some("variables") => Some(ListItems::Variables),
                Some("units") => Some(ListItems::Units),
                _ => {
                    return Some(Err(ParseError {
                        kind: ParseErrorKind::InvalidCommand(
                            "if provided, the argument to `list` or `ls` must be \
                             one of: functions, dimensions, variables, units",
                        ),
                        span: span_from_boundary(word_boundaries[1], code_source_id),
                    }));
                }
            };
            if words.next().is_some() {
                let start = word_boundaries[2].0;
                let end = word_boundaries.last().unwrap().1;
                return Some(Err(ParseError {
                    kind: ParseErrorKind::InvalidCommand("`list` takes at most one argument"),
                    span: span_from_boundary((start, end), code_source_id),
                }));
            }

            Command::List { items }
        }
        "save" => {
            let err_msg = "`save` requires exactly one argument, the destination";
            let Some(dst) = words.next() else {
                return Some(Err(ParseError {
                    kind: ParseErrorKind::InvalidCommand(err_msg),
                    span: span_from_boundary(word_boundaries[0], code_source_id),
                }));
            };

            if words.next().is_some() {
                let start = word_boundaries[2].0;
                let end = word_boundaries.last().unwrap().1;
                return Some(Err(ParseError {
                    kind: ParseErrorKind::InvalidCommand(err_msg),
                    span: span_from_boundary((start, end), code_source_id),
                }));
            }

            Command::Save { dst }
        }

        _ => return None,
    };

    Some(Ok(command))
}
