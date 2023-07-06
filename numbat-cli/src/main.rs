mod ansi_formatter;
mod completion;

use ansi_formatter::ANSIFormatter;
use completion::NumbatCompleter;

use numbat::markup;
use numbat::pretty_print::PrettyPrint;
use numbat::{markup::Formatter, Context, ExitStatus, InterpreterResult, NumbatError, ParseError};

use anyhow::{bail, Context as AnyhowContext, Result};
use clap::Parser;
use rustyline::config::Configurer;
use rustyline::{
    self, error::ReadlineError, history::DefaultHistory, Completer, Editor, Helper, Hinter,
    Validator,
};
use rustyline::{EventHandler, Highlighter, KeyCode, KeyEvent, Modifiers};

use std::fs;
use std::path::PathBuf;

type ControlFlow = std::ops::ControlFlow<numbat::ExitStatus>;

const PROMPT: &str = ">>> ";

#[derive(Parser, Debug)]
#[command(version, about, name("numbat"))]
struct Args {
    /// Path to source file with Numbat code. If none is given, an interactive
    /// session is started.
    file: Option<PathBuf>,

    /// Evaluate a single expression
    #[arg(short, long, value_name = "CODE", conflicts_with = "file")]
    expression: Option<String>,

    /// Do not load the prelude with predefined physical dimensions and units.
    #[arg(long)]
    no_prelude: bool,

    /// Whether or not to pretty-print every input expression.
    #[arg(long)]
    pretty_print: bool,

    /// Turn on debug mode (e.g. disassembler output).
    #[arg(long, short)]
    debug: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ExecutionMode {
    Normal,
    Interactive,
}

impl ExecutionMode {
    fn exit_status_in_case_of_error(&self) -> ControlFlow {
        if matches!(self, ExecutionMode::Normal) {
            ControlFlow::Break(ExitStatus::Error)
        } else {
            ControlFlow::Continue(())
        }
    }
}

#[derive(Completer, Helper, Hinter, Validator, Highlighter)]
struct NumbatHelper {
    #[rustyline(Completer)]
    completer: NumbatCompleter,
}

struct Cli {
    args: Args,
    context: Context,
    current_filename: Option<PathBuf>,
}

impl Cli {
    fn new() -> Self {
        let args = Args::parse();
        let mut context = Context::new_without_prelude(args.debug);
        context.add_module_path("/home/ped1st/software/numbat/modules"); // TODO
        Self {
            context,
            args,
            current_filename: None,
        }
    }

    fn run(&mut self) -> Result<()> {
        if !self.args.no_prelude {
            let prelude_path = self.get_prelude_path();

            self.current_filename = Some(prelude_path.clone());
            let prelude_code = fs::read_to_string(&prelude_path).context(format!(
                "Error while reading prelude from {}",
                prelude_path.to_string_lossy()
            ))?;
            let result = self.parse_and_evaluate(&prelude_code, ExecutionMode::Normal, false);
            if result.is_break() {
                bail!("Interpreter error in Prelude code")
            }
        }

        let code: Option<String> = if let Some(ref path) = self.args.file {
            self.current_filename = Some(path.clone());
            Some(fs::read_to_string(path).context(format!(
                "Could not load source file '{}'",
                path.to_string_lossy()
            ))?)
        } else {
            self.current_filename = None;
            self.args.expression.clone()
        };

        if let Some(code) = code {
            let result =
                self.parse_and_evaluate(&code, ExecutionMode::Normal, self.args.pretty_print);

            match result {
                std::ops::ControlFlow::Continue(()) => Ok(()),
                std::ops::ControlFlow::Break(ExitStatus::Success) => Ok(()),
                std::ops::ControlFlow::Break(ExitStatus::Error) => {
                    bail!("Interpreter stopped due to error")
                }
            }
        } else {
            self.repl()
        }
    }

    fn repl(&mut self) -> Result<()> {
        println!();
        println!(" █▄░█ █░█ █▀▄▀█ █▄▄ ▄▀█ ▀█▀");
        println!(" █░▀█ █▄█ █░▀░█ █▄█ █▀█ ░█░");
        println!();

        let history_path = self.get_history_path()?;

        let mut rl = Editor::<NumbatHelper, DefaultHistory>::new()?;
        rl.set_max_history_size(1000)
            .context("Error while configuring history size")?;
        rl.set_completion_type(rustyline::CompletionType::List);
        rl.set_helper(Some(NumbatHelper {
            completer: NumbatCompleter {},
        }));
        rl.bind_sequence(
            KeyEvent(KeyCode::Enter, Modifiers::ALT),
            EventHandler::Simple(rustyline::Cmd::Newline),
        );
        rl.load_history(&history_path).ok();

        let result = self.repl_loop(&mut rl);

        rl.save_history(&history_path).context(format!(
            "Error while saving history to '{}'",
            history_path.to_string_lossy()
        ))?;

        result
    }

    fn repl_loop(&mut self, rl: &mut Editor<NumbatHelper, DefaultHistory>) -> Result<()> {
        loop {
            let readline = rl.readline(PROMPT);
            match readline {
                Ok(line) => {
                    if !line.trim().is_empty() {
                        rl.add_history_entry(&line)?;
                        let result = self.parse_and_evaluate(
                            &line,
                            ExecutionMode::Interactive,
                            self.args.pretty_print,
                        );

                        match result {
                            std::ops::ControlFlow::Continue(()) => {}
                            std::ops::ControlFlow::Break(ExitStatus::Success) => {
                                return Ok(());
                            }
                            std::ops::ControlFlow::Break(ExitStatus::Error) => {
                                bail!("Interpreter stopped due to error")
                            }
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {}
                Err(ReadlineError::Eof) => {
                    return Ok(());
                }
                Err(err) => {
                    bail!(err);
                }
            }
        }
    }

    #[must_use]
    fn parse_and_evaluate(
        &mut self,
        input: &str,
        execution_mode: ExecutionMode,
        pretty_print: bool,
    ) -> ControlFlow {
        let result = self.context.interpret(input);

        match result {
            Ok((statements, interpreter_result)) => {
                if pretty_print {
                    println!();
                    for statement in &statements {
                        let repr = ANSIFormatter {}.format(&statement.pretty_print(), true);
                        println!("{}", repr);
                        println!();
                    }
                } else if execution_mode == ExecutionMode::Interactive {
                    println!();
                }

                match interpreter_result {
                    InterpreterResult::Quantity(quantity) => {
                        let q_markup = markup::whitespace("    ")
                            + markup::operator("=")
                            + markup::space()
                            + quantity.pretty_print();
                        println!("{}", ANSIFormatter {}.format(&q_markup, false));
                        println!();

                        ControlFlow::Continue(())
                    }
                    InterpreterResult::Continue => ControlFlow::Continue(()),
                    InterpreterResult::Exit(exit_status) => ControlFlow::Break(exit_status),
                }
            }
            Err(NumbatError::ParseError(ref e @ ParseError { ref span, .. })) => {
                let line = input.lines().nth(span.line - 1).unwrap();

                let filename = self
                    .current_filename
                    .as_deref()
                    .map(|p| p.to_string_lossy())
                    .unwrap_or_else(|| "<input>".into());
                eprintln!(
                    "File {filename}:{line_number}:{position}",
                    filename = filename,
                    line_number = span.line,
                    position = span.position
                );
                eprintln!("    {line}");
                eprintln!("    {offset}^", offset = " ".repeat(span.position - 1));
                eprintln!("{}", e);

                execution_mode.exit_status_in_case_of_error()
            }
            Err(NumbatError::ResolverError(e)) => {
                eprintln!("Module resolver error: {:#}", e);
                execution_mode.exit_status_in_case_of_error()
            }
            Err(NumbatError::NameResolutionError(e)) => {
                eprintln!("Name resolution error: {:#}", e);
                execution_mode.exit_status_in_case_of_error()
            }
            Err(NumbatError::TypeCheckError(e)) => {
                eprintln!("Type check error: {:#}", e);
                execution_mode.exit_status_in_case_of_error()
            }
            Err(NumbatError::RuntimeError(e)) => {
                eprintln!("Runtime error: {:#}", e);
                execution_mode.exit_status_in_case_of_error()
            }
        }
    }

    fn get_prelude_path(&self) -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config_dir.join("numbat").join("prelude.nbt") // TODO: allow for preludes in system paths, user paths, …
    }

    fn get_history_path(&self) -> Result<PathBuf> {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("numbat");
        fs::create_dir(&data_dir).ok();
        Ok(data_dir.join("history"))
    }
}

fn main() {
    let mut cli = Cli::new();

    if let Err(e) = cli.run() {
        eprintln!("{:#}", e);
        std::process::exit(1);
    }
}
