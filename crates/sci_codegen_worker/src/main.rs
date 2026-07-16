use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use sci_protocol::{
    BasicBlockPlan, CompileRequest, CompileResponse, ExternFunctionPlan, FunctionPlan, Operation,
    PLAN_VERSION, ScalarType, SciModulePlan, SwitchCasePlan, TerminatorPlan, ValueRef, read_frame,
    write_frame,
};

const SUPPORTED_RUSTC_COMMIT: &str = "fcbe7917ba18120d9eda136f1c7c5a60c78e554e";
const SUPPORTED_TARGET: &str = "x86_64-unknown-linux-gnu";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("sci-codegen-worker: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args_os();
    let _program = args.next();
    match args.next().as_deref() {
        Some(value) if value == "--stdio-once" => run_stdio_once(),
        _ => Err("usage: sci-codegen-worker --stdio-once".into()),
    }
}

fn run_stdio_once() -> Result<(), String> {
    let request: CompileRequest =
        read_frame(io::stdin().lock()).map_err(|err| format!("request decode failed: {err}"))?;
    let response = match compile_request(&request) {
        Ok(()) => CompileResponse {
            request_id: request.request_id,
            success: true,
            diagnostic: String::new(),
        },
        Err(diagnostic) => CompileResponse {
            request_id: request.request_id,
            success: false,
            diagnostic,
        },
    };
    write_frame(io::stdout().lock(), &response)
        .map_err(|err| format!("response encode failed: {err}"))
}

fn compile_request(request: &CompileRequest) -> Result<(), String> {
    validate_module(&request.module)?;

    let sa = emit_sa(&request.module)?;
    let output_path = Path::new(&request.output_path);
    let parent = output_path
        .parent()
        .ok_or_else(|| "object output path has no parent".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("failed to create object output directory: {err}"))?;

    let sa_path = match &request.emit_sa_path {
        Some(path) => PathBuf::from(path),
        None => output_path.with_extension("sci.sa"),
    };
    if let Some(parent) = sa_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create SA output directory: {err}"))?;
    }
    fs::write(&sa_path, sa.as_bytes())
        .map_err(|err| format!("failed to write {}: {err}", sa_path.display()))?;

    let sci = std::env::var_os("SCI_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/root/projects/sci/zig-out/bin/sa"));
    let output = Command::new(&sci)
        .arg("build-obj")
        .arg(&sa_path)
        .arg("-o")
        .arg(output_path)
        .arg("--no-debug")
        .output()
        .map_err(|err| format!("failed to execute {}: {err}", sci.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "SCI object emission failed with {}\nstdout:\n{}\nstderr:\n{}",
            output.status, stdout, stderr
        ));
    }
    if !output_path.is_file() {
        return Err(format!(
            "SCI reported success without publishing {}",
            output_path.display()
        ));
    }
    Ok(())
}

fn validate_module(module: &SciModulePlan) -> Result<(), String> {
    if module.plan_version != PLAN_VERSION {
        return Err(format!(
            "unsupported plan version {}, expected {}",
            module.plan_version, PLAN_VERSION
        ));
    }
    if module.rustc_commit != SUPPORTED_RUSTC_COMMIT {
        return Err(format!(
            "rustc commit mismatch: got {}, expected {}",
            module.rustc_commit, SUPPORTED_RUSTC_COMMIT
        ));
    }
    if module.target.triple != SUPPORTED_TARGET
        || module.target.pointer_width != 64
        || module.target.endian != sci_protocol::Endian::Little
    {
        return Err(format!(
            "unsupported target contract: {} / {}-bit / {:?}",
            module.target.triple, module.target.pointer_width, module.target.endian
        ));
    }
    if module.functions.is_empty() {
        return Err("module contains no functions".into());
    }
    let functions: BTreeMap<&str, &FunctionPlan> = module
        .functions
        .iter()
        .map(|function| (function.symbol.as_str(), function))
        .collect();
    if functions.len() != module.functions.len() {
        return Err("module contains duplicate function symbols".into());
    }
    let extern_functions: BTreeMap<&str, &ExternFunctionPlan> = module
        .extern_functions
        .iter()
        .map(|function| (function.symbol.as_str(), function))
        .collect();
    if extern_functions.len() != module.extern_functions.len() {
        return Err("module contains duplicate extern function symbols".into());
    }
    for extern_function in &module.extern_functions {
        validate_extern_function(extern_function)?;
        if functions.contains_key(extern_function.symbol.as_str()) {
            return Err(format!(
                "extern function `{}` duplicates a defined function",
                extern_function.symbol
            ));
        }
    }
    for function in &module.functions {
        validate_function(function, &functions, &extern_functions)?;
    }
    Ok(())
}

fn validate_symbol(kind: &str, symbol: &str) -> Result<(), String> {
    if symbol.is_empty() {
        return Err(format!("{kind} symbol is empty"));
    }
    if !symbol
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'$'))
    {
        return Err(format!(
            "{kind} symbol contains unsupported SA characters: {symbol}"
        ));
    }
    Ok(())
}

fn validate_extern_function(function: &ExternFunctionPlan) -> Result<(), String> {
    validate_symbol("extern function", &function.symbol)?;
    if function
        .argument_types
        .iter()
        .any(|ty| *ty == ScalarType::I1)
    {
        return Err(format!(
            "extern function {} uses unsupported i1 ABI argument",
            function.symbol
        ));
    }
    if function.return_type == Some(ScalarType::I1) {
        return Err(format!(
            "extern function {} uses unsupported i1 ABI return",
            function.symbol
        ));
    }
    Ok(())
}

fn validate_function(
    function: &FunctionPlan,
    functions: &BTreeMap<&str, &FunctionPlan>,
    extern_functions: &BTreeMap<&str, &ExternFunctionPlan>,
) -> Result<(), String> {
    validate_symbol("function", &function.symbol)?;

    let locals: BTreeMap<u32, ScalarType> = function
        .locals
        .iter()
        .map(|local| (local.id, local.ty))
        .collect();
    if locals.len() != function.locals.len() {
        return Err(format!("{} has duplicate local ids", function.symbol));
    }
    if let Some(return_local) = function.return_local
        && !locals.contains_key(&return_local)
    {
        return Err(format!("{} return local is missing", function.symbol));
    }
    for argument in &function.argument_locals {
        if !locals.contains_key(argument) {
            return Err(format!(
                "{} argument local {} is missing",
                function.symbol, argument
            ));
        }
    }

    let blocks: BTreeMap<u32, &BasicBlockPlan> = function
        .blocks
        .iter()
        .map(|block| (block.id, block))
        .collect();
    if blocks.len() != function.blocks.len() {
        return Err(format!("{} has duplicate block ids", function.symbol));
    }
    if !blocks.contains_key(&0) {
        return Err(format!("{} is missing entry block 0", function.symbol));
    }
    for block in &function.blocks {
        for target in terminator_successors(&block.terminator) {
            if !blocks.contains_key(&target) {
                return Err(format!(
                    "{} block {} targets missing block {}",
                    function.symbol, block.id, target
                ));
            }
        }
    }

    let block_entries = compute_block_entries(function, &locals)?;
    for block in &function.blocks {
        let mut defined = block_entries
            .get(&block.id)
            .cloned()
            .ok_or_else(|| format!("{} block {} is unreachable", function.symbol, block.id))?;
        for operation in &block.operations {
            validate_operation(function, &locals, &mut defined, operation)?;
        }
        validate_terminator(
            function,
            functions,
            extern_functions,
            &locals,
            &defined,
            &block.terminator,
        )?;
    }
    Ok(())
}

fn compute_block_entries(
    function: &FunctionPlan,
    locals: &BTreeMap<u32, ScalarType>,
) -> Result<BTreeMap<u32, BTreeSet<u32>>, String> {
    let mut entries: BTreeMap<u32, Option<BTreeSet<u32>>> = function
        .blocks
        .iter()
        .map(|block| (block.id, None))
        .collect();
    entries.insert(0, Some(function.argument_locals.iter().copied().collect()));

    let mut changed = true;
    while changed {
        changed = false;
        for block in &function.blocks {
            let Some(mut exit) = entries.get(&block.id).and_then(Clone::clone) else {
                continue;
            };
            for operation in &block.operations {
                let dst = operation_destination(operation);
                if !locals.contains_key(&dst) {
                    return Err(format!("operation writes missing local {dst}"));
                }
                exit.insert(dst);
            }
            if let Some(dst) = terminator_destination(&block.terminator) {
                if !locals.contains_key(&dst) {
                    return Err(format!("terminator writes missing local {dst}"));
                }
                exit.insert(dst);
            }
            for successor in terminator_successors(&block.terminator) {
                let Some(slot) = entries.get_mut(&successor) else {
                    return Err(format!(
                        "{} block {} targets missing block {}",
                        function.symbol, block.id, successor
                    ));
                };
                let next = match slot {
                    Some(existing) => existing.intersection(&exit).copied().collect(),
                    None => exit.clone(),
                };
                if slot.as_ref() != Some(&next) {
                    *slot = Some(next);
                    changed = true;
                }
            }
        }
    }

    let mut resolved = BTreeMap::new();
    for (block, entry) in entries {
        let Some(entry) = entry else {
            return Err(format!(
                "{} block {} is unreachable",
                function.symbol, block
            ));
        };
        resolved.insert(block, entry);
    }
    Ok(resolved)
}

fn validate_operation(
    function: &FunctionPlan,
    locals: &BTreeMap<u32, ScalarType>,
    defined: &mut BTreeSet<u32>,
    operation: &Operation,
) -> Result<(), String> {
    match operation {
        Operation::Copy { dst, src } => {
            validate_value(locals, defined, src)?;
            validate_destination(locals, *dst)?;
            if value_type(locals, src)? != locals[dst] {
                return Err(format!(
                    "{} copy has inconsistent scalar types",
                    function.symbol
                ));
            }
            defined.insert(*dst);
        }
        Operation::Binary { dst, lhs, rhs, .. } => {
            validate_value(locals, defined, lhs)?;
            validate_value(locals, defined, rhs)?;
            validate_destination(locals, *dst)?;
            let dst_ty = locals[dst];
            if value_type(locals, lhs)? != dst_ty || value_type(locals, rhs)? != dst_ty {
                return Err(format!(
                    "{} binary operation has inconsistent scalar types",
                    function.symbol
                ));
            }
            defined.insert(*dst);
        }
        Operation::Compare { dst, lhs, rhs, .. } => {
            validate_value(locals, defined, lhs)?;
            validate_value(locals, defined, rhs)?;
            validate_destination(locals, *dst)?;
            if locals[dst] != ScalarType::I1 {
                return Err(format!(
                    "{} compare destination must be i1",
                    function.symbol
                ));
            }
            if value_type(locals, lhs)? != value_type(locals, rhs)? {
                return Err(format!(
                    "{} compare operation has inconsistent scalar types",
                    function.symbol
                ));
            }
            defined.insert(*dst);
        }
        Operation::Cast { dst, src, ty, .. } => {
            validate_value(locals, defined, src)?;
            validate_destination(locals, *dst)?;
            if locals[dst] != *ty {
                return Err(format!(
                    "{} cast destination type mismatch",
                    function.symbol
                ));
            }
            defined.insert(*dst);
        }
    }
    Ok(())
}

fn validate_terminator(
    function: &FunctionPlan,
    functions: &BTreeMap<&str, &FunctionPlan>,
    extern_functions: &BTreeMap<&str, &ExternFunctionPlan>,
    locals: &BTreeMap<u32, ScalarType>,
    defined: &BTreeSet<u32>,
    terminator: &TerminatorPlan,
) -> Result<(), String> {
    match terminator {
        TerminatorPlan::Return => {
            if let Some(return_local) = function.return_local
                && !defined.contains(&return_local)
            {
                return Err(format!(
                    "{} return local is not initialized",
                    function.symbol
                ));
            }
        }
        TerminatorPlan::Goto { .. } => {}
        TerminatorPlan::Branch { condition, .. } => {
            validate_value(locals, defined, condition)?;
            if value_type(locals, condition)? != ScalarType::I1 {
                return Err(format!("{} branch condition must be i1", function.symbol));
            }
        }
        TerminatorPlan::Assert { condition, .. } => {
            validate_value(locals, defined, condition)?;
            if value_type(locals, condition)? != ScalarType::I1 {
                return Err(format!("{} assert condition must be i1", function.symbol));
            }
        }
        TerminatorPlan::SwitchInt {
            discr,
            cases,
            otherwise: _,
        } => {
            validate_value(locals, defined, discr)?;
            let discr_ty = value_type(locals, discr)?;
            if discr_ty == ScalarType::I1 {
                return Err(format!(
                    "{} bool SwitchInt must be represented as Branch",
                    function.symbol
                ));
            }
            let mut seen_values = BTreeSet::new();
            for case in cases {
                validate_value(locals, defined, &case.value)?;
                if value_type(locals, &case.value)? != discr_ty {
                    return Err(format!(
                        "{} SwitchInt case type does not match discriminator",
                        function.symbol
                    ));
                }
                if !seen_values.insert(value_key(&case.value)) {
                    return Err(format!(
                        "{} SwitchInt contains duplicate case value",
                        function.symbol
                    ));
                }
            }
        }
        TerminatorPlan::Call {
            callee,
            args,
            destination,
            ..
        } => {
            if let Some(callee_function) = functions.get(callee.as_str()) {
                if args.len() != callee_function.argument_locals.len() {
                    return Err(format!(
                        "{} call to {} has {} args, expected {}",
                        function.symbol,
                        callee_function.symbol,
                        args.len(),
                        callee_function.argument_locals.len()
                    ));
                }
                let callee_locals: BTreeMap<u32, ScalarType> = callee_function
                    .locals
                    .iter()
                    .map(|local| (local.id, local.ty))
                    .collect();
                for (arg, callee_local) in args.iter().zip(&callee_function.argument_locals) {
                    validate_value(locals, defined, arg)?;
                    if value_type(locals, arg)? != callee_locals[callee_local] {
                        return Err(format!(
                            "{} call to {} has argument type mismatch",
                            function.symbol, callee_function.symbol
                        ));
                    }
                }
                match (destination, callee_function.return_local) {
                    (Some(destination), Some(return_local)) => {
                        validate_destination(locals, *destination)?;
                        if locals[destination] != callee_locals[&return_local] {
                            return Err(format!(
                                "{} call to {} has return type mismatch",
                                function.symbol, callee_function.symbol
                            ));
                        }
                    }
                    (None, None) => {}
                    (Some(_), None) => {
                        return Err(format!(
                            "{} call to void function {} has a destination",
                            function.symbol, callee_function.symbol
                        ));
                    }
                    (None, Some(_)) => {
                        return Err(format!(
                            "{} call to {} is missing a destination",
                            function.symbol, callee_function.symbol
                        ));
                    }
                }
                return Ok(());
            }
            let extern_function = extern_functions
                .get(callee.as_str())
                .ok_or_else(|| format!("{} calls missing callee `{callee}`", function.symbol))?;
            if args.len() != extern_function.argument_types.len() {
                return Err(format!(
                    "{} call to {} has {} args, expected {}",
                    function.symbol,
                    extern_function.symbol,
                    args.len(),
                    extern_function.argument_types.len()
                ));
            }
            for (arg, expected_ty) in args.iter().zip(&extern_function.argument_types) {
                validate_value(locals, defined, arg)?;
                if value_type(locals, arg)? != *expected_ty {
                    return Err(format!(
                        "{} call to {} has argument type mismatch",
                        function.symbol, extern_function.symbol
                    ));
                }
            }
            match (destination, extern_function.return_type) {
                (Some(destination), Some(return_type)) => {
                    validate_destination(locals, *destination)?;
                    if locals[destination] != return_type {
                        return Err(format!(
                            "{} call to {} has return type mismatch",
                            function.symbol, extern_function.symbol
                        ));
                    }
                }
                (None, None) => {}
                (Some(_), None) => {
                    return Err(format!(
                        "{} call to void extern {} has a destination",
                        function.symbol, extern_function.symbol
                    ));
                }
                (None, Some(_)) => {
                    return Err(format!(
                        "{} call to {} is missing a destination",
                        function.symbol, extern_function.symbol
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_value(
    locals: &BTreeMap<u32, ScalarType>,
    defined: &BTreeSet<u32>,
    value: &ValueRef,
) -> Result<(), String> {
    if let ValueRef::Local(local) = value {
        if !locals.contains_key(local) {
            return Err(format!("value references missing local {local}"));
        }
        if !defined.contains(local) {
            return Err(format!("value references uninitialized local {local}"));
        }
    }
    Ok(())
}

fn validate_destination(locals: &BTreeMap<u32, ScalarType>, dst: u32) -> Result<(), String> {
    if !locals.contains_key(&dst) {
        return Err(format!("operation writes missing local {dst}"));
    }
    Ok(())
}

fn operation_destination(operation: &Operation) -> u32 {
    match operation {
        Operation::Copy { dst, .. }
        | Operation::Binary { dst, .. }
        | Operation::Compare { dst, .. }
        | Operation::Cast { dst, .. } => *dst,
    }
}

fn terminator_successors(terminator: &TerminatorPlan) -> Vec<u32> {
    match terminator {
        TerminatorPlan::Return => Vec::new(),
        TerminatorPlan::Goto { target } => vec![*target],
        TerminatorPlan::Branch {
            true_target,
            false_target,
            ..
        } => vec![*true_target, *false_target],
        TerminatorPlan::Assert { target, .. } => vec![*target],
        TerminatorPlan::SwitchInt {
            cases, otherwise, ..
        } => {
            let mut targets = cases.iter().map(|case| case.target).collect::<Vec<_>>();
            targets.push(*otherwise);
            targets
        }
        TerminatorPlan::Call { target, .. } => vec![*target],
    }
}

fn terminator_destination(terminator: &TerminatorPlan) -> Option<u32> {
    match terminator {
        TerminatorPlan::Call { destination, .. } => *destination,
        TerminatorPlan::Return
        | TerminatorPlan::Goto { .. }
        | TerminatorPlan::Branch { .. }
        | TerminatorPlan::Assert { .. }
        | TerminatorPlan::SwitchInt { .. } => None,
    }
}

fn value_key(value: &ValueRef) -> (u8, u64) {
    match value {
        ValueRef::Local(local) => (0, u64::from(*local)),
        ValueRef::Integer { ty, bits } => (*ty as u8, *bits),
    }
}

fn value_type(locals: &BTreeMap<u32, ScalarType>, value: &ValueRef) -> Result<ScalarType, String> {
    match value {
        ValueRef::Local(local) => locals
            .get(local)
            .copied()
            .ok_or_else(|| format!("missing local {local}")),
        ValueRef::Integer { ty, .. } => Ok(*ty),
    }
}

fn emit_sa(module: &SciModulePlan) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("// Generated by rustc_codegen_sci; do not edit.\n");
    out.push_str(&format!(
        "// rustc={} target={} cgu={}\n\n",
        module.rustc_commit, module.target.triple, module.cgu_name
    ));
    for function in &module.extern_functions {
        emit_extern_function(&mut out, function);
    }
    if !module.extern_functions.is_empty() {
        out.push('\n');
    }
    for function in &module.functions {
        emit_function(&mut out, function)?;
        out.push('\n');
    }
    Ok(out)
}

fn emit_extern_function(out: &mut String, function: &ExternFunctionPlan) {
    out.push_str("@extern ");
    out.push_str(&function.symbol);
    out.push('(');
    for (index, ty) in function.argument_types.iter().enumerate() {
        if index != 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("arg{index}: {}", ty.sa_name()));
    }
    out.push_str(") -> ");
    match function.return_type {
        Some(return_type) => out.push_str(return_type.sa_name()),
        None => out.push_str("void"),
    }
    out.push('\n');
}

fn emit_function(out: &mut String, function: &FunctionPlan) -> Result<(), String> {
    let locals: BTreeMap<u32, ScalarType> = function
        .locals
        .iter()
        .map(|local| (local.id, local.ty))
        .collect();
    out.push_str("@export ");
    out.push_str(&function.symbol);
    out.push('(');
    for (index, local) in function.argument_locals.iter().enumerate() {
        if index != 0 {
            out.push_str(", ");
        }
        out.push_str(&local_name(*local));
        out.push_str(": ");
        out.push_str(locals[local].sa_name());
    }
    out.push_str(") -> ");
    match function.return_local {
        Some(return_local) => {
            let return_ty = locals
                .get(&return_local)
                .ok_or_else(|| "missing return local type".to_string())?;
            out.push_str(return_ty.sa_name());
        }
        None => out.push_str("void"),
    }
    out.push_str(":\n");

    let block_entries = compute_block_entries(function, &locals)?;
    for block in &function.blocks {
        out.push_str(&block_label(block.id));
        out.push_str(":\n");
        let mut defined = block_entries
            .get(&block.id)
            .cloned()
            .ok_or_else(|| format!("{} block {} is unreachable", function.symbol, block.id))?;
        for operation in &block.operations {
            emit_operation(out, operation);
            defined.insert(operation_destination(operation));
        }
        emit_terminator(out, function, block.id, &defined, &block.terminator);
    }
    Ok(())
}

fn emit_operation(out: &mut String, operation: &Operation) {
    match operation {
        Operation::Copy { dst, src } => {
            out.push_str("    ");
            out.push_str(&local_name(*dst));
            out.push_str(" = add ");
            emit_value(out, src);
            out.push_str(", 0\n");
        }
        Operation::Binary { dst, op, lhs, rhs } => {
            out.push_str("    ");
            out.push_str(&local_name(*dst));
            out.push_str(" = ");
            out.push_str(op.sa_name());
            out.push(' ');
            emit_value(out, lhs);
            out.push_str(", ");
            emit_value(out, rhs);
            out.push('\n');
        }
        Operation::Compare { dst, op, lhs, rhs } => {
            out.push_str("    ");
            out.push_str(&local_name(*dst));
            out.push_str(" = ");
            out.push_str(op.sa_name());
            out.push(' ');
            emit_value(out, lhs);
            out.push_str(", ");
            emit_value(out, rhs);
            out.push('\n');
        }
        Operation::Cast { dst, op, src, ty } => {
            out.push_str("    ");
            out.push_str(&local_name(*dst));
            out.push_str(" = ");
            out.push_str(op.sa_name());
            out.push(' ');
            emit_value(out, src);
            out.push_str(" as ");
            out.push_str(ty.sa_name());
            out.push('\n');
        }
    }
}

fn emit_terminator(
    out: &mut String,
    function: &FunctionPlan,
    block_id: u32,
    defined: &BTreeSet<u32>,
    terminator: &TerminatorPlan,
) {
    match terminator {
        TerminatorPlan::Return => {
            let mut releasable = defined.clone();
            if let Some(return_local) = function.return_local {
                releasable.remove(&return_local);
            }
            for local in releasable.into_iter().rev() {
                out.push_str("    !");
                out.push_str(&local_name(local));
                out.push('\n');
            }
            match function.return_local {
                Some(return_local) => {
                    out.push_str("    return ");
                    out.push_str(&local_name(return_local));
                    out.push('\n');
                }
                None => out.push_str("    return\n"),
            }
        }
        TerminatorPlan::Goto { target } => {
            out.push_str("    jmp ");
            out.push_str(&block_label(*target));
            out.push('\n');
        }
        TerminatorPlan::Branch {
            condition,
            true_target,
            false_target,
        } => {
            out.push_str("    br ");
            emit_value(out, condition);
            out.push_str(" -> ");
            out.push_str(&block_label(*true_target));
            out.push_str(", ");
            out.push_str(&block_label(*false_target));
            out.push('\n');
        }
        TerminatorPlan::Assert {
            condition,
            expected,
            target,
            panic_code,
        } => {
            let panic_label = assert_panic_label(function, block_id, *panic_code);
            out.push_str("    br ");
            emit_value(out, condition);
            out.push_str(" -> ");
            if *expected {
                out.push_str(&block_label(*target));
                out.push_str(", ");
                out.push_str(&panic_label);
            } else {
                out.push_str(&panic_label);
                out.push_str(", ");
                out.push_str(&block_label(*target));
            }
            out.push('\n');
            out.push_str(&panic_label);
            out.push_str(":\n");
            for local in defined.iter().copied().rev() {
                out.push_str("    !");
                out.push_str(&local_name(local));
                out.push('\n');
            }
            out.push_str("    panic(");
            out.push_str(&panic_code.to_string());
            out.push_str(")\n");
        }
        TerminatorPlan::SwitchInt {
            discr,
            cases,
            otherwise,
        } => emit_switch_int(out, function, block_id, defined, discr, cases, *otherwise),
        TerminatorPlan::Call {
            callee,
            args,
            destination,
            target,
        } => {
            out.push_str("    ");
            if let Some(destination) = destination {
                out.push_str(&local_name(*destination));
                out.push_str(" = ");
            }
            out.push_str("call @");
            out.push_str(callee);
            out.push('(');
            for (index, arg) in args.iter().enumerate() {
                if index != 0 {
                    out.push_str(", ");
                }
                emit_value(out, arg);
            }
            out.push_str(")\n    jmp ");
            out.push_str(&block_label(*target));
            out.push('\n');
        }
    }
}

fn emit_value(out: &mut String, value: &ValueRef) {
    match value {
        ValueRef::Local(local) => out.push_str(&local_name(*local)),
        ValueRef::Integer { ty, bits } => match ty {
            ScalarType::I1 => out.push_str(&(u8::from(*bits != 0)).to_string()),
            ScalarType::I8 => out.push_str(&(*bits as i8).to_string()),
            ScalarType::I16 => out.push_str(&(*bits as i16).to_string()),
            ScalarType::I32 => out.push_str(&(*bits as i32).to_string()),
            ScalarType::I64 => out.push_str(&(*bits as i64).to_string()),
            ScalarType::U8 => out.push_str(&(*bits as u8).to_string()),
            ScalarType::U16 => out.push_str(&(*bits as u16).to_string()),
            ScalarType::U32 => out.push_str(&(*bits as u32).to_string()),
            ScalarType::U64 => out.push_str(&bits.to_string()),
        },
    }
}

fn local_name(local: u32) -> String {
    format!("v{local}")
}

fn block_label(block: u32) -> String {
    format!("L_bb{block}")
}

fn emit_switch_int(
    out: &mut String,
    function: &FunctionPlan,
    block_id: u32,
    defined: &BTreeSet<u32>,
    discr: &ValueRef,
    cases: &[SwitchCasePlan],
    otherwise: u32,
) {
    if cases.is_empty() {
        out.push_str("    jmp ");
        out.push_str(&block_label(otherwise));
        out.push('\n');
        return;
    }

    for (index, case) in cases.iter().enumerate() {
        if index != 0 {
            out.push_str(&switch_next_label(function, block_id, index));
            out.push_str(":\n");
            out.push_str("    !");
            out.push_str(&switch_cmp_name(block_id, index - 1));
            out.push('\n');
        }
        let cmp = switch_cmp_name(block_id, index);
        out.push_str("    ");
        out.push_str(&cmp);
        out.push_str(" = eq ");
        emit_value(out, discr);
        out.push_str(", ");
        emit_value(out, &case.value);
        out.push('\n');
        out.push_str("    br ");
        out.push_str(&cmp);
        out.push_str(" -> ");
        out.push_str(&switch_hit_label(function, block_id, index));
        out.push_str(", ");
        if index + 1 == cases.len() {
            out.push_str(&switch_otherwise_label(function, block_id));
        } else {
            out.push_str(&switch_next_label(function, block_id, index + 1));
        }
        out.push('\n');

        out.push_str(&switch_hit_label(function, block_id, index));
        out.push_str(":\n");
        out.push_str("    !");
        out.push_str(&cmp);
        out.push('\n');
        out.push_str("    jmp ");
        out.push_str(&block_label(case.target));
        out.push('\n');
    }

    out.push_str(&switch_otherwise_label(function, block_id));
    out.push_str(":\n");
    out.push_str("    !");
    out.push_str(&switch_cmp_name(block_id, cases.len() - 1));
    out.push('\n');
    let _ = defined;
    out.push_str("    jmp ");
    out.push_str(&block_label(otherwise));
    out.push('\n');
}

fn assert_panic_label(function: &FunctionPlan, block_id: u32, panic_code: u32) -> String {
    format!(
        "L_assert_panic_{}_{}_{}",
        function.symbol.replace(['.', '$'], "_"),
        block_id,
        panic_code
    )
}

fn switch_cmp_name(block_id: u32, index: usize) -> String {
    format!("v_switch_{block_id}_{index}")
}

fn switch_hit_label(function: &FunctionPlan, block_id: u32, index: usize) -> String {
    format!(
        "L_switch_hit_{}_{}_{}",
        function.symbol.replace(['.', '$'], "_"),
        block_id,
        index
    )
}

fn switch_next_label(function: &FunctionPlan, block_id: u32, index: usize) -> String {
    format!(
        "L_switch_next_{}_{}_{}",
        function.symbol.replace(['.', '$'], "_"),
        block_id,
        index
    )
}

fn switch_otherwise_label(function: &FunctionPlan, block_id: u32) -> String {
    format!(
        "L_switch_otherwise_{}_{}",
        function.symbol.replace(['.', '$'], "_"),
        block_id
    )
}
