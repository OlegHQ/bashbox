//! Builtin Command Dispatch
//!
//! Handles dispatch of built-in shell commands like export, unset, cd, etc.
//! Separated from interpreter.rs for modularity.

use crate::interpreter::builtins::hash_cmd::handle_hash as handle_hash_tuple;
use crate::interpreter::builtins::{
    handle_break, handle_cd, handle_compgen, handle_complete, handle_compopt, handle_continue,
    handle_declare, handle_dirs, handle_exit, handle_export, handle_getopts, handle_help,
    handle_let, handle_local, handle_mapfile, handle_popd, handle_pushd, handle_read,
    handle_readonly, handle_return, handle_set, handle_shift, handle_shopt, handle_unset,
};
use crate::interpreter::errors::InterpreterError;
use crate::interpreter::helpers::result::{failure, from_builtin, from_tuple, test_result, OK};
use crate::interpreter::helpers::shell_constants::SHELL_BUILTINS;
use crate::interpreter::interpreter::FileSystem as SyncFileSystem;
use crate::interpreter::types::{ExecResult, InterpreterState};
use std::collections::HashMap;

/// Type for the function that runs a command recursively
pub type RunCommandFn<'a> = &'a dyn Fn(
    &mut InterpreterState,
    &str,      // command_name
    &[String], // args
    &[bool],   // quoted_args
    &str,      // stdin
    bool,      // skip_functions
    bool,      // use_default_path
    i32,       // stdin_source_fd
) -> Result<ExecResult, InterpreterError>;

/// Type for the function that builds exported environment
pub type BuildExportedEnvFn<'a> = &'a dyn Fn() -> HashMap<String, String>;

/// Type for the function that executes user scripts
pub type ExecuteUserScriptFn<'a> = &'a dyn Fn(&str, &[String], Option<&str>) -> ExecResult;

/// Dispatch context containing dependencies needed for builtin dispatch
pub struct BuiltinDispatchContext<'a> {
    pub state: &'a mut InterpreterState,
    pub run_command: RunCommandFn<'a>,
    pub build_exported_env: BuildExportedEnvFn<'a>,
    pub execute_user_script: ExecuteUserScriptFn<'a>,
    /// Sync VFS (virtual or host-backed); used by `cd` for existence checks.
    pub sync_fs: &'a dyn SyncFileSystem,
}

/// Dispatch a command to the appropriate builtin handler or external command.
/// Returns `Ok(None)` if the command should be handled by external command resolution.
pub fn dispatch_builtin(
    dispatch_ctx: &mut BuiltinDispatchContext,
    command_name: &str,
    args: &[String],
    _quoted_args: &[bool],
    stdin: &str,
    skip_functions: bool,
    _use_default_path: bool,
    stdin_source_fd: i32,
) -> Result<Option<ExecResult>, InterpreterError> {
    // Built-in commands (special builtins that cannot be overridden by functions)
    match command_name {
        "export" => {
            return Ok(Some(from_builtin(handle_export(dispatch_ctx.state, args))));
        }
        "exit" => {
            if let Err(e) = handle_exit(dispatch_ctx.state, args) {
                return Err(e);
            }
            unreachable!()
        }
        "set" => {
            return Ok(Some(from_builtin(handle_set(dispatch_ctx.state, args)?)));
        }
        "break" => {
            return Ok(Some(from_builtin(handle_break(dispatch_ctx.state, args)?)));
        }
        "continue" => {
            return Ok(Some(from_builtin(handle_continue(
                dispatch_ctx.state,
                args,
            )?)));
        }
        "return" => {
            return match handle_return(dispatch_ctx.state, args) {
                Ok(br) => Ok(Some(from_builtin(br))),
                Err(e) => Err(e),
            };
        }
        "shift" => {
            return Ok(Some(from_builtin(handle_shift(dispatch_ctx.state, args)?)));
        }
        "shopt" => {
            return Ok(Some(from_builtin(handle_shopt(dispatch_ctx.state, args))));
        }
        "help" => {
            return Ok(Some(from_builtin(handle_help(args))));
        }
        "unset" => {
            return Ok(Some(handle_unset(dispatch_ctx.state, args)));
        }
        "local" => {
            return Ok(Some(handle_local(dispatch_ctx.state, args)));
        }
        "getopts" => {
            return Ok(Some(from_builtin(handle_getopts(dispatch_ctx.state, args))));
        }
        "compgen" => {
            return Ok(Some(from_tuple(handle_compgen(dispatch_ctx.state, args))));
        }
        "complete" => {
            return Ok(Some(from_builtin(handle_complete(
                dispatch_ctx.state,
                args,
            ))));
        }
        "compopt" => {
            return Ok(Some(from_builtin(handle_compopt(dispatch_ctx.state, args))));
        }
        "pushd" => {
            return Ok(Some(from_tuple(handle_pushd(dispatch_ctx.state, args))));
        }
        "popd" => {
            return Ok(Some(from_tuple(handle_popd(dispatch_ctx.state, args))));
        }
        "dirs" => {
            return Ok(Some(from_tuple(handle_dirs(dispatch_ctx.state, args))));
        }
        "source" | "." => {
            return Ok(Some(handle_source_stub(dispatch_ctx.state, args)));
        }
        "read" => {
            return Ok(Some(from_builtin(handle_read(
                dispatch_ctx.state,
                args,
                stdin,
                stdin_source_fd,
            ))));
        }
        "mapfile" | "readarray" => {
            return Ok(Some(from_builtin(handle_mapfile(
                dispatch_ctx.state,
                args,
                stdin,
            ))));
        }
        "declare" | "typeset" => {
            return Ok(Some(from_builtin(handle_declare(dispatch_ctx.state, args))));
        }
        "readonly" => {
            return Ok(Some(from_builtin(handle_readonly(
                dispatch_ctx.state,
                args,
            ))));
        }
        _ => {}
    }

    // In POSIX mode, eval is a special builtin that cannot be overridden by functions
    if command_name == "eval" && dispatch_ctx.state.options.posix {
        return Ok(Some(handle_eval_stub(dispatch_ctx.state, args, stdin)));
    }

    // User-defined functions override most builtins (except special ones above)
    if !skip_functions {
        if dispatch_ctx.state.functions.contains_key(command_name) {
            return Ok(Some(OK));
        }
    }

    // Simple builtins (can be overridden by functions)
    match command_name {
        ":" | "true" => {
            return Ok(Some(OK));
        }
        "false" => {
            return Ok(Some(test_result(false)));
        }
        "command" => {
            return Ok(Some(handle_command_builtin(dispatch_ctx, args, stdin)?));
        }
        "builtin" => {
            return Ok(Some(handle_builtin_builtin(dispatch_ctx, args, stdin)?));
        }
        "exec" => {
            if args.is_empty() {
                return Ok(Some(OK));
            }
            let cmd = &args[0];
            let rest: Vec<String> = args[1..].to_vec();
            return Ok(Some((dispatch_ctx.run_command)(
                dispatch_ctx.state,
                cmd,
                &rest,
                &[],
                stdin,
                false,
                false,
                -1,
            )?));
        }
        "wait" => {
            return Ok(Some(OK));
        }
        "eval" => {
            return Ok(Some(handle_eval_stub(dispatch_ctx.state, args, stdin)));
        }
        "cd" => {
            return Ok(Some(handle_cd(
                dispatch_ctx.state,
                args,
                Some(dispatch_ctx.sync_fs),
            )));
        }
        "let" => {
            return Ok(Some(from_tuple(handle_let(dispatch_ctx.state, args))));
        }
        "type" => {
            return Ok(Some(handle_type_stub(dispatch_ctx.state, args)));
        }
        "hash" => {
            return Ok(Some(from_tuple(handle_hash_tuple(
                dispatch_ctx.state,
                args,
            ))));
        }
        _ => {}
    }

    Ok(None)
}

/// Handle the 'command' builtin
fn handle_command_builtin(
    dispatch_ctx: &mut BuiltinDispatchContext,
    args: &[String],
    stdin: &str,
) -> Result<ExecResult, InterpreterError> {
    if args.is_empty() {
        return Ok(OK);
    }

    let mut use_default_path = false;
    let mut verbose_describe = false;
    let mut show_path = false;
    let mut cmd_args = args.to_vec();

    while !cmd_args.is_empty() && cmd_args[0].starts_with('-') {
        let opt = &cmd_args[0];
        if opt == "--" {
            cmd_args.remove(0);
            break;
        }
        for ch in opt[1..].chars() {
            match ch {
                'p' => use_default_path = true,
                'V' => verbose_describe = true,
                'v' => show_path = true,
                _ => {}
            }
        }
        cmd_args.remove(0);
    }

    if cmd_args.is_empty() {
        return Ok(OK);
    }

    if show_path || verbose_describe {
        return Ok(handle_command_v_stub(
            dispatch_ctx.state,
            &cmd_args,
            show_path,
            verbose_describe,
        ));
    }

    let cmd = &cmd_args[0];
    let rest: Vec<String> = cmd_args[1..].to_vec();
    (dispatch_ctx.run_command)(
        dispatch_ctx.state,
        cmd,
        &rest,
        &[],
        stdin,
        true,
        use_default_path,
        -1,
    )
}

/// Handle the 'builtin' builtin
fn handle_builtin_builtin(
    dispatch_ctx: &mut BuiltinDispatchContext,
    args: &[String],
    stdin: &str,
) -> Result<ExecResult, InterpreterError> {
    if args.is_empty() {
        return Ok(OK);
    }

    let mut cmd_args = args.to_vec();
    if cmd_args[0] == "--" {
        cmd_args.remove(0);
        if cmd_args.is_empty() {
            return Ok(OK);
        }
    }

    let cmd = &cmd_args[0];

    if !SHELL_BUILTINS.contains(cmd.as_str()) {
        return Ok(failure(format!(
            "bash: builtin: {}: not a shell builtin\n",
            cmd
        )));
    }

    let rest: Vec<String> = cmd_args[1..].to_vec();
    (dispatch_ctx.run_command)(dispatch_ctx.state, cmd, &rest, &[], stdin, true, false, -1)
}

// ============================================================================
// Stubs and helpers for builtins not fully wired
// ============================================================================

fn handle_command_v_stub(
    state: &InterpreterState,
    names: &[String],
    show_path: bool,
    verbose_describe: bool,
) -> ExecResult {
    let mut stdout = String::new();
    let mut exit_code = 0;

    for name in names {
        if SHELL_BUILTINS.contains(name.as_str()) {
            if verbose_describe {
                stdout.push_str(&format!("{} is a shell builtin\n", name));
            } else {
                stdout.push_str(&format!("{}\n", name));
            }
        } else if state.functions.contains_key(name) {
            if verbose_describe {
                stdout.push_str(&format!("{} is a function\n", name));
            } else {
                stdout.push_str(&format!("{}\n", name));
            }
        } else {
            exit_code = 1;
        }
    }

    let _ = show_path;
    ExecResult::new(stdout, String::new(), exit_code)
}

fn handle_source_stub(_state: &mut InterpreterState, args: &[String]) -> ExecResult {
    if args.is_empty() {
        return failure("bash: source: filename argument required\n");
    }
    OK
}

fn handle_eval_stub(_state: &mut InterpreterState, _args: &[String], _stdin: &str) -> ExecResult {
    OK
}

fn handle_type_stub(state: &InterpreterState, args: &[String]) -> ExecResult {
    let mut stdout = String::new();
    let mut exit_code = 0;

    for name in args {
        if name.starts_with('-') {
            continue;
        }

        if SHELL_BUILTINS.contains(name.as_str()) {
            stdout.push_str(&format!("{} is a shell builtin\n", name));
        } else if state.functions.contains_key(name) {
            stdout.push_str(&format!("{} is a function\n", name));
        } else if let Some(ref aliases) = state.aliases {
            if let Some(alias_val) = aliases.get(name) {
                stdout.push_str(&format!("{} is aliased to `{}'\n", name, alias_val));
            } else {
                stdout.push_str(&format!("bash: type: {}: not found\n", name));
                exit_code = 1;
            }
        } else {
            stdout.push_str(&format!("bash: type: {}: not found\n", name));
            exit_code = 1;
        }
    }

    ExecResult::new(stdout, String::new(), exit_code)
}
