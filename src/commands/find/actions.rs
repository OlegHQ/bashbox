//! Post-traverse `-delete` and `-exec` handling for `find`.

use super::types::Expression;
use crate::commands::types::CommandContext;
use crate::fs::RmOptions;

pub(super) fn expression_has_action(expr: &Expression) -> bool {
    match expr {
        Expression::Print
        | Expression::Print0
        | Expression::Printf { .. }
        | Expression::Delete
        | Expression::Exec { .. } => true,
        Expression::Not(inner) => expression_has_action(inner),
        Expression::And(left, right) | Expression::Or(left, right) => {
            expression_has_action(left) || expression_has_action(right)
        }
        _ => false,
    }
}

pub(super) fn expression_has_delete(expr: &Expression) -> bool {
    match expr {
        Expression::Delete => true,
        Expression::Not(inner) => expression_has_delete(inner),
        Expression::And(left, right) | Expression::Or(left, right) => {
            expression_has_delete(left) || expression_has_delete(right)
        }
        _ => false,
    }
}

pub(super) fn expression_get_exec(expr: &Expression) -> Option<(Vec<String>, bool)> {
    match expr {
        Expression::Exec { command, batch } => Some((command.clone(), *batch)),
        Expression::Not(inner) => expression_get_exec(inner),
        Expression::And(left, right) | Expression::Or(left, right) => {
            expression_get_exec(left).or_else(|| expression_get_exec(right))
        }
        _ => None,
    }
}

pub(super) async fn apply_delete_and_exec(
    ctx: &CommandContext,
    expression: &Expression,
    matched_paths: &[String],
    output: &mut String,
    all_stderr: &mut String,
    exit_code: &mut i32,
) {
    if expression_has_delete(expression) {
        let mut sorted = matched_paths.to_vec();
        sorted.sort_by(|a, b| b.len().cmp(&a.len()));
        for file in &sorted {
            let full_path = ctx.fs.resolve_path(&ctx.cwd, file);
            match ctx
                .fs
                .rm(
                    &full_path,
                    &RmOptions {
                        recursive: false,
                        force: false,
                    },
                )
                .await
            {
                Ok(()) => {}
                Err(e) => {
                    all_stderr.push_str(&format!("find: cannot delete '{}': {}\n", file, e));
                    *exit_code = 1;
                }
            }
        }
    }

    if let Some((command_parts, batch)) = expression_get_exec(expression) {
        if let Some(ref exec_fn) = ctx.exec_fn {
            if batch {
                let mut cmd_parts: Vec<String> = Vec::new();
                for part in &command_parts {
                    if part == "{}" {
                        cmd_parts.extend(matched_paths.iter().cloned());
                    } else {
                        cmd_parts.push(part.clone());
                    }
                }
                let cmd = cmd_parts
                    .iter()
                    .map(|p| format!("\"{}\"", p))
                    .collect::<Vec<_>>()
                    .join(" ");
                let result = exec_fn(
                    cmd,
                    String::new(),
                    ctx.cwd.clone(),
                    ctx.env.clone(),
                    ctx.fs.clone(),
                )
                .await;
                output.push_str(&result.stdout);
                all_stderr.push_str(&result.stderr);
                if result.exit_code != 0 {
                    *exit_code = result.exit_code;
                }
            } else {
                for file in matched_paths {
                    let cmd_with_file: Vec<String> = command_parts
                        .iter()
                        .map(|part| {
                            if part == "{}" {
                                file.clone()
                            } else {
                                part.clone()
                            }
                        })
                        .collect();
                    let cmd = cmd_with_file
                        .iter()
                        .map(|p| format!("\"{}\"", p))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let result = exec_fn(
                        cmd,
                        String::new(),
                        ctx.cwd.clone(),
                        ctx.env.clone(),
                        ctx.fs.clone(),
                    )
                    .await;
                    output.push_str(&result.stdout);
                    all_stderr.push_str(&result.stderr);
                    if result.exit_code != 0 {
                        *exit_code = result.exit_code;
                    }
                }
            }
        }
    }
}
