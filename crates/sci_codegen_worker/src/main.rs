use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use sci_protocol::{
    AbiPassModePlan, BasicBlockPlan, CallingConventionPlan, CompileRequest, CompileResponse,
    DiagnosticLocation, Endian, ExternFunctionPlan, FieldLayoutRecipe, FnAbiPlan, FunctionPlan,
    NicheRecipe, Operation, PLAN_VERSION, ScalarLayoutRecipe, ScalarType, SciModulePlan,
    SwitchCasePlan, TagEncodingRecipe, TargetPlan, TerminatorPlan, TypeLayoutRecipe, ValueRef,
    VariantRecipe, read_frame, write_frame,
};

const SUPPORTED_RUSTC_COMMIT: &str = "fcbe7917ba18120d9eda136f1c7c5a60c78e554e";
const SUPPORTED_TARGET: &str = "x86_64-unknown-linux-gnu";
const SUPPORTED_OBJECT_FORMAT: &str = "elf";
const SUPPORTED_DATA_LAYOUT: &str =
    "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128";
const SUPPORTED_CPU: &str = "x86-64";
const SUPPORTED_FEATURES: &str = "";
const SUPPORTED_RELOCATION_MODEL: &str = "pic";

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
            diagnostic_code: String::new(),
            diagnostic_location: None,
        },
        Err(diagnostic) => classified_response(request.request_id, diagnostic),
    };
    write_frame(io::stdout().lock(), &response)
        .map_err(|err| format!("response encode failed: {err}"))
}

fn classified_response(request_id: u64, diagnostic: String) -> CompileResponse {
    CompileResponse {
        request_id,
        success: false,
        diagnostic_code: classify_diagnostic_code(&diagnostic).into(),
        diagnostic_location: diagnostic_location(&diagnostic),
        diagnostic,
    }
}

fn classify_diagnostic_code(diagnostic: &str) -> &'static str {
    if diagnostic.contains("unsupported Pair pass mode")
        || diagnostic.contains("unsupported Cast pass mode")
        || diagnostic.contains("unsupported Indirect pass mode")
    {
        "SCI_ABI_UNSUPPORTED_PASS_MODE"
    } else if diagnostic.contains(" ABI")
        || diagnostic.contains("calling convention")
        || diagnostic.contains("variadic")
        || diagnostic.contains("unwinding")
    {
        "SCI_ABI_INVALID"
    } else if diagnostic.contains("unsupported target")
        || diagnostic.contains("target descriptor")
        || diagnostic.contains("data layout")
        || diagnostic.contains("relocation model")
        || diagnostic.contains("code model")
    {
        "SCI_TARGET_UNSUPPORTED"
    } else if diagnostic.contains(" load ") || diagnostic.contains(" store ") {
        "SCI_MEMORY_INVALID"
    } else if diagnostic.contains("type layout")
        || diagnostic.contains("field")
        || diagnostic.contains("variant")
        || diagnostic.contains("niche")
        || diagnostic.contains("alignment")
    {
        "SCI_LAYOUT_INVALID"
    } else if diagnostic.contains("block")
        || diagnostic.contains("callee")
        || diagnostic.contains("branch")
        || diagnostic.contains("terminator")
    {
        "SCI_CFG_INVALID"
    } else if diagnostic.contains("SA builder failed") {
        "SCI_OBJECT_EMIT_FAILED"
    } else if diagnostic.contains("failed to create")
        || diagnostic.contains("failed to write")
        || diagnostic.contains("failed to start")
    {
        "SCI_IO_FAILED"
    } else {
        "SCI_WORKER_REJECTED"
    }
}

fn diagnostic_location(diagnostic: &str) -> Option<DiagnosticLocation> {
    let function = diagnostic_function(diagnostic);
    let block = diagnostic_number_after(diagnostic, "block ");
    let local = diagnostic_number_after(diagnostic, "local ");
    if function.is_none() && block.is_none() && local.is_none() {
        None
    } else {
        Some(DiagnosticLocation {
            function,
            block,
            local,
        })
    }
}

fn diagnostic_function(diagnostic: &str) -> Option<String> {
    if let Some(rest) = diagnostic.strip_prefix("function ") {
        return rest.split_whitespace().next().map(str::to_string);
    }
    if let Some(rest) = diagnostic.strip_prefix("extern function ") {
        return rest.split_whitespace().next().map(str::to_string);
    }
    diagnostic
        .split_once(':')
        .and_then(|(name, _)| (!name.contains(char::is_whitespace)).then(|| name.to_string()))
}

fn diagnostic_number_after(diagnostic: &str, marker: &str) -> Option<u32> {
    let rest = diagnostic.split(marker).nth(1)?;
    let number = rest
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    (!number.is_empty()).then(|| number.parse().ok()).flatten()
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
    validate_target(&module.target)?;
    if module.functions.is_empty() {
        return Err("module contains no functions".into());
    }
    let type_layouts: BTreeMap<&str, &TypeLayoutRecipe> = module
        .type_layouts
        .iter()
        .map(|layout| (layout.ty.as_str(), layout))
        .collect();
    if type_layouts.len() != module.type_layouts.len() {
        return Err("module contains duplicate type layout recipes".into());
    }
    for layout in &module.type_layouts {
        validate_type_layout(layout)?;
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

fn validate_type_layout(layout: &TypeLayoutRecipe) -> Result<(), String> {
    if layout.ty.is_empty() {
        return Err("type layout recipe has empty type name".into());
    }
    validate_size_align("type layout", layout.size, layout.align)?;
    validate_field_layout("type layout", layout.size, &layout.fields)?;
    validate_variant_layout(layout)?;
    if let Some(niche) = &layout.largest_niche {
        validate_niche("type layout largest niche", layout.size, niche)?;
    }
    for scalar in &layout.scalar_valid_ranges {
        validate_scalar_layout("type layout scalar", scalar)?;
    }
    Ok(())
}

fn validate_size_align(context: &str, size: u64, align: u64) -> Result<(), String> {
    if align == 0 || !align.is_power_of_two() {
        return Err(format!("{context} has invalid alignment {align}"));
    }
    if size > 0 && size % align != 0 {
        return Err(format!(
            "{context} size {size} is not a multiple of alignment {align}"
        ));
    }
    Ok(())
}

fn validate_field_layout(
    context: &str,
    size: u64,
    fields: &FieldLayoutRecipe,
) -> Result<(), String> {
    match fields {
        FieldLayoutRecipe::Primitive => Ok(()),
        FieldLayoutRecipe::Union { count } => {
            if *count == 0 {
                return Err(format!("{context} union field count is zero"));
            }
            Ok(())
        }
        FieldLayoutRecipe::Array { stride, count } => {
            let bytes = stride.checked_mul(*count).ok_or_else(|| {
                format!("{context} array field layout overflows: {stride} * {count}")
            })?;
            if bytes > size {
                return Err(format!(
                    "{context} array field bytes {bytes} exceed layout size {size}"
                ));
            }
            Ok(())
        }
        FieldLayoutRecipe::Arbitrary {
            offsets,
            memory_order,
        } => {
            if offsets.len() != memory_order.len() {
                return Err(format!(
                    "{context} field offsets and memory order lengths differ"
                ));
            }
            let mut seen = BTreeSet::new();
            for field in memory_order {
                let index = usize::try_from(*field)
                    .map_err(|_| format!("{context} memory-order field index overflows"))?;
                if index >= offsets.len() {
                    return Err(format!(
                        "{context} memory-order field {field} is out of range"
                    ));
                }
                if !seen.insert(*field) {
                    return Err(format!(
                        "{context} memory-order field {field} appears more than once"
                    ));
                }
            }
            for offset in offsets {
                if *offset > size {
                    return Err(format!(
                        "{context} field offset {offset} exceeds layout size {size}"
                    ));
                }
            }
            Ok(())
        }
    }
}

fn validate_variant_layout(layout: &TypeLayoutRecipe) -> Result<(), String> {
    match &layout.variants {
        VariantRecipe::Empty => {
            if !layout.uninhabited {
                return Err(format!(
                    "type layout `{}` has empty variants but is inhabited",
                    layout.ty
                ));
            }
            Ok(())
        }
        VariantRecipe::Single { .. } => Ok(()),
        VariantRecipe::Multiple {
            tag,
            tag_field,
            tag_encoding,
            variants,
        } => {
            validate_scalar_layout("type layout variant tag", tag)?;
            if let Some(field_count) = field_count(&layout.fields)
                && usize::try_from(*tag_field).map_or(true, |field| field >= field_count)
            {
                return Err(format!(
                    "type layout `{}` variant tag field {} is out of range",
                    layout.ty, tag_field
                ));
            }
            validate_tag_encoding(tag_encoding)?;
            if variants.is_empty() {
                return Err(format!(
                    "type layout `{}` has multiple variants but no variant layouts",
                    layout.ty
                ));
            }
            let mut seen = BTreeSet::new();
            for variant in variants {
                if !seen.insert(variant.index) {
                    return Err(format!(
                        "type layout `{}` repeats variant {}",
                        layout.ty, variant.index
                    ));
                }
                validate_size_align("variant layout", variant.size, variant.align)?;
                validate_field_layout("variant layout", variant.size, &variant.fields)?;
            }
            Ok(())
        }
    }
}

fn validate_scalar_layout(context: &str, scalar: &ScalarLayoutRecipe) -> Result<(), String> {
    if scalar.primitive.is_empty() {
        return Err(format!("{context} has empty primitive"));
    }
    Ok(())
}

fn validate_niche(context: &str, size: u64, niche: &NicheRecipe) -> Result<(), String> {
    if niche.primitive.is_empty() {
        return Err(format!("{context} has empty primitive"));
    }
    if niche.offset >= size && size != 0 {
        return Err(format!(
            "{context} offset {} is outside layout size {size}",
            niche.offset
        ));
    }
    Ok(())
}

fn validate_tag_encoding(encoding: &TagEncodingRecipe) -> Result<(), String> {
    match encoding {
        TagEncodingRecipe::Direct => Ok(()),
        TagEncodingRecipe::Niche {
            niche_variants_start,
            niche_variants_end,
            ..
        } => {
            if niche_variants_start > niche_variants_end {
                return Err(format!(
                    "niche tag variant range {}..={} is inverted",
                    niche_variants_start, niche_variants_end
                ));
            }
            Ok(())
        }
    }
}

fn field_count(fields: &FieldLayoutRecipe) -> Option<usize> {
    match fields {
        FieldLayoutRecipe::Primitive => Some(0),
        FieldLayoutRecipe::Union { count } => usize::try_from(*count).ok(),
        FieldLayoutRecipe::Array { count, .. } => usize::try_from(*count).ok(),
        FieldLayoutRecipe::Arbitrary { offsets, .. } => Some(offsets.len()),
    }
}

fn validate_target(target: &TargetPlan) -> Result<(), String> {
    if target.triple != SUPPORTED_TARGET {
        return Err(format!(
            "unsupported target triple `{}`; expected `{SUPPORTED_TARGET}`",
            target.triple
        ));
    }
    if target.object_format != SUPPORTED_OBJECT_FORMAT {
        return Err(format!(
            "unsupported target object format `{}`; expected `{SUPPORTED_OBJECT_FORMAT}`",
            target.object_format
        ));
    }
    if target.data_layout != SUPPORTED_DATA_LAYOUT {
        return Err(format!(
            "unsupported target data layout `{}`; expected `{SUPPORTED_DATA_LAYOUT}`",
            target.data_layout
        ));
    }
    if target.pointer_width != 64 || target.endian != Endian::Little {
        return Err(format!(
            "unsupported target scalar contract: {}-bit / {:?}",
            target.pointer_width, target.endian
        ));
    }
    if target.cpu != SUPPORTED_CPU {
        return Err(format!(
            "unsupported target CPU `{}`; expected `{SUPPORTED_CPU}`",
            target.cpu
        ));
    }
    if target.features != SUPPORTED_FEATURES {
        return Err(format!(
            "unsupported target features `{}`; expected `{SUPPORTED_FEATURES}`",
            target.features
        ));
    }
    if target.relocation_model != SUPPORTED_RELOCATION_MODEL {
        return Err(format!(
            "unsupported relocation model `{}`; expected `{SUPPORTED_RELOCATION_MODEL}`",
            target.relocation_model
        ));
    }
    if target.code_model.is_some() {
        return Err(format!(
            "unsupported code model `{:?}`; expected target default",
            target.code_model
        ));
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
    validate_fn_abi(
        &format!("extern function {}", function.symbol),
        &function.abi,
        function.argument_types.len(),
        function.return_type.is_some(),
    )?;
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

fn validate_fn_abi(
    context: &str,
    abi: &FnAbiPlan,
    lowered_argument_count: usize,
    has_return_value: bool,
) -> Result<(), String> {
    match &abi.convention {
        CallingConventionPlan::C | CallingConventionPlan::Rust => {}
        other => {
            return Err(format!(
                "{context} uses unsupported calling convention {other:?}"
            ));
        }
    }
    if abi.variadic {
        return Err(format!("{context} uses unsupported variadic ABI"));
    }
    if abi.can_unwind {
        return Err(format!("{context} uses unsupported unwinding ABI"));
    }
    if abi.arguments.len() != lowered_argument_count {
        return Err(format!(
            "{context} ABI has {} arguments but plan lowered {}",
            abi.arguments.len(),
            lowered_argument_count
        ));
    }
    if abi.fixed_count
        != u32::try_from(abi.arguments.len()).map_err(|_| format!("{context} has too many args"))?
    {
        return Err(format!(
            "{context} ABI fixed_count does not match arguments"
        ));
    }
    for (index, argument) in abi.arguments.iter().enumerate() {
        validate_abi_value(context, &format!("argument {index}"), argument, true)?;
    }
    validate_abi_value(context, "return", &abi.return_value, has_return_value)?;
    Ok(())
}

fn validate_abi_value(
    context: &str,
    label: &str,
    value: &sci_protocol::AbiValuePlan,
    is_lowered: bool,
) -> Result<(), String> {
    validate_size_align(
        &format!("{context} ABI {label} layout"),
        value.size,
        value.align,
    )?;
    match value.mode {
        AbiPassModePlan::Ignore if !is_lowered => Ok(()),
        AbiPassModePlan::Direct if is_lowered => Ok(()),
        AbiPassModePlan::Ignore | AbiPassModePlan::Direct => Err(format!(
            "{context} ABI {label} mode does not match lowered value presence"
        )),
        AbiPassModePlan::Pair => Err(format!(
            "{context} ABI {label} uses unsupported Pair pass mode"
        )),
        AbiPassModePlan::Cast { .. } => Err(format!(
            "{context} ABI {label} uses unsupported Cast pass mode"
        )),
        AbiPassModePlan::Indirect { .. } => Err(format!(
            "{context} ABI {label} uses unsupported Indirect pass mode"
        )),
    }
}

fn validate_function(
    function: &FunctionPlan,
    functions: &BTreeMap<&str, &FunctionPlan>,
    extern_functions: &BTreeMap<&str, &ExternFunctionPlan>,
) -> Result<(), String> {
    validate_symbol("function", &function.symbol)?;
    validate_fn_abi(
        &format!("function {}", function.symbol),
        &function.abi,
        function.argument_locals.len(),
        function.return_local.is_some(),
    )?;

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
                if let Some(dst) = operation_destination(operation) {
                    if !locals.contains_key(&dst) {
                        return Err(format!("operation writes missing local {dst}"));
                    }
                    exit.insert(dst);
                }
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
            if dst_ty == ScalarType::Ptr {
                return Err(format!(
                    "{} binary operation on ptr is not supported",
                    function.symbol
                ));
            }
            if value_type(locals, lhs)? != dst_ty || value_type(locals, rhs)? != dst_ty {
                return Err(format!(
                    "{} binary operation has inconsistent scalar types",
                    function.symbol
                ));
            }
            defined.insert(*dst);
        }
        Operation::Compare { dst, op, lhs, rhs } => {
            validate_value(locals, defined, lhs)?;
            validate_value(locals, defined, rhs)?;
            validate_destination(locals, *dst)?;
            if locals[dst] != ScalarType::I1 {
                return Err(format!(
                    "{} compare destination must be i1",
                    function.symbol
                ));
            }
            let lhs_ty = value_type(locals, lhs)?;
            let rhs_ty = value_type(locals, rhs)?;
            if (lhs_ty == ScalarType::Ptr || rhs_ty == ScalarType::Ptr)
                && !matches!(
                    op,
                    sci_protocol::CompareOp::Eq | sci_protocol::CompareOp::Ne
                )
            {
                return Err(format!(
                    "{} ordered compare operation on ptr is not supported",
                    function.symbol
                ));
            }
            if lhs_ty != rhs_ty {
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
            if value_type(locals, src)? == ScalarType::Ptr || *ty == ScalarType::Ptr {
                return Err(format!(
                    "{} cast operation on ptr is not supported",
                    function.symbol
                ));
            }
            if locals[dst] != *ty {
                return Err(format!(
                    "{} cast destination type mismatch",
                    function.symbol
                ));
            }
            defined.insert(*dst);
        }
        Operation::Load {
            dst,
            ptr,
            offset: _,
            ty,
            align,
        } => {
            validate_value(locals, defined, ptr)?;
            validate_destination(locals, *dst)?;
            validate_memory_align(function, "load", *align)?;
            if value_type(locals, ptr)? != ScalarType::Ptr {
                return Err(format!("{} load source must be ptr", function.symbol));
            }
            if locals[dst] != *ty {
                return Err(format!(
                    "{} load destination type mismatch",
                    function.symbol
                ));
            }
            defined.insert(*dst);
        }
        Operation::Store {
            ptr,
            offset: _,
            value,
            ty,
            align,
        } => {
            validate_value(locals, defined, ptr)?;
            validate_value(locals, defined, value)?;
            validate_memory_align(function, "store", *align)?;
            if value_type(locals, ptr)? != ScalarType::Ptr {
                return Err(format!("{} store destination must be ptr", function.symbol));
            }
            if value_type(locals, value)? != *ty {
                return Err(format!("{} store value type mismatch", function.symbol));
            }
        }
    }
    Ok(())
}

fn validate_memory_align(function: &FunctionPlan, op: &str, align: u64) -> Result<(), String> {
    if align == 0 || !align.is_power_of_two() {
        return Err(format!(
            "{} {op} has invalid alignment {align}",
            function.symbol
        ));
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
            if discr_ty == ScalarType::Ptr {
                return Err(format!(
                    "{} ptr SwitchInt is not supported",
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

fn operation_destination(operation: &Operation) -> Option<u32> {
    match operation {
        Operation::Copy { dst, .. }
        | Operation::Binary { dst, .. }
        | Operation::Compare { dst, .. }
        | Operation::Cast { dst, .. }
        | Operation::Load { dst, .. } => Some(*dst),
        Operation::Store { .. } => None,
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
            emit_operation(out, &locals, operation);
            if let Some(dst) = operation_destination(operation) {
                defined.insert(dst);
            }
        }
        emit_terminator(out, function, block.id, &defined, &block.terminator);
    }
    Ok(())
}

fn emit_operation(out: &mut String, locals: &BTreeMap<u32, ScalarType>, operation: &Operation) {
    match operation {
        Operation::Copy { dst, src } => {
            out.push_str("    ");
            out.push_str(&local_name(*dst));
            if matches!(
                src,
                ValueRef::Integer {
                    ty: ScalarType::Ptr,
                    ..
                }
            ) {
                out.push_str(" = ");
                emit_value(out, src);
                out.push_str(" as ptr\n");
            } else if locals[dst] == ScalarType::Ptr {
                out.push_str(" = ptr_add ");
                emit_value(out, src);
                out.push_str(", 0\n");
            } else {
                out.push_str(" = add ");
                emit_value(out, src);
                out.push_str(", 0\n");
            }
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
        Operation::Load {
            dst,
            ptr,
            offset,
            ty,
            align: _,
        } => {
            out.push_str("    ");
            out.push_str(&local_name(*dst));
            out.push_str(" = load ");
            emit_address(out, ptr, *offset);
            out.push_str(" as ");
            out.push_str(ty.sa_name());
            out.push('\n');
        }
        Operation::Store {
            ptr,
            offset,
            value,
            ty,
            align: _,
        } => {
            out.push_str("    store ");
            emit_address(out, ptr, *offset);
            out.push_str(", ");
            emit_value(out, value);
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
            ScalarType::Ptr => out.push_str(&bits.to_string()),
        },
    }
}

fn emit_address(out: &mut String, ptr: &ValueRef, offset: u64) {
    emit_value(out, ptr);
    out.push('+');
    out.push_str(&offset.to_string());
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

#[cfg(test)]
mod tests {
    use super::*;
    use sci_protocol::{
        AbiRegisterKind, AbiRegisterPlan, AbiUniformPlan, AbiValuePlan, LocalPlan,
        ValidRangeRecipe, VariantLayoutRecipe,
    };

    fn abi_value(mode: AbiPassModePlan) -> AbiValuePlan {
        abi_value_with(8, 8, mode)
    }

    fn abi_value_with(size: u64, align: u64, mode: AbiPassModePlan) -> AbiValuePlan {
        AbiValuePlan { size, align, mode }
    }

    fn direct_abi_value(size: u64, align: u64) -> AbiValuePlan {
        abi_value_with(size, align, AbiPassModePlan::Direct)
    }

    fn ignored_abi_value() -> AbiValuePlan {
        abi_value_with(0, 1, AbiPassModePlan::Ignore)
    }

    fn cast_abi_value() -> AbiValuePlan {
        abi_value(AbiPassModePlan::Cast {
            pad_i32: false,
            prefix: vec![integer_register(64)],
            rest_offset: None,
            rest: AbiUniformPlan {
                unit: integer_register(64),
                total_bytes: 8,
                consecutive: true,
            },
        })
    }

    fn indirect_abi_value() -> AbiValuePlan {
        abi_value(AbiPassModePlan::Indirect {
            has_metadata: false,
            on_stack: true,
        })
    }

    fn fn_abi(arguments: Vec<AbiValuePlan>, return_value: AbiValuePlan) -> FnAbiPlan {
        FnAbiPlan {
            convention: CallingConventionPlan::C,
            variadic: false,
            fixed_count: arguments.len() as u32,
            can_unwind: false,
            arguments,
            return_value,
        }
    }

    fn integer_register(bits: u64) -> AbiRegisterPlan {
        AbiRegisterPlan {
            kind: AbiRegisterKind::Integer,
            bits,
        }
    }

    fn supported_target() -> TargetPlan {
        TargetPlan {
            triple: SUPPORTED_TARGET.into(),
            object_format: SUPPORTED_OBJECT_FORMAT.into(),
            data_layout: SUPPORTED_DATA_LAYOUT.into(),
            pointer_width: 64,
            endian: Endian::Little,
            cpu: SUPPORTED_CPU.into(),
            features: SUPPORTED_FEATURES.into(),
            relocation_model: SUPPORTED_RELOCATION_MODEL.into(),
            code_model: None,
        }
    }

    fn scalar_layout() -> ScalarLayoutRecipe {
        ScalarLayoutRecipe {
            primitive: "Int(I32, true)".into(),
            valid_range: Some(ValidRangeRecipe {
                start: 0,
                end: u32::MAX.into(),
            }),
        }
    }

    fn struct_layout() -> TypeLayoutRecipe {
        TypeLayoutRecipe {
            ty: "(i32, i32)".into(),
            size: 8,
            align: 4,
            uninhabited: false,
            fields: FieldLayoutRecipe::Arbitrary {
                offsets: vec![0, 4],
                memory_order: vec![0, 1],
            },
            variants: VariantRecipe::Single { index: 0 },
            largest_niche: None,
            scalar_valid_ranges: vec![scalar_layout(), scalar_layout()],
        }
    }

    fn union_layout() -> TypeLayoutRecipe {
        TypeLayoutRecipe {
            ty: "union U".into(),
            size: 8,
            align: 8,
            uninhabited: false,
            fields: FieldLayoutRecipe::Union { count: 2 },
            variants: VariantRecipe::Single { index: 0 },
            largest_niche: None,
            scalar_valid_ranges: Vec::new(),
        }
    }

    fn array_layout() -> TypeLayoutRecipe {
        TypeLayoutRecipe {
            ty: "[i32; 4]".into(),
            size: 16,
            align: 4,
            uninhabited: false,
            fields: FieldLayoutRecipe::Array {
                stride: 4,
                count: 4,
            },
            variants: VariantRecipe::Single { index: 0 },
            largest_niche: None,
            scalar_valid_ranges: Vec::new(),
        }
    }

    fn empty_layout() -> TypeLayoutRecipe {
        TypeLayoutRecipe {
            ty: "!".into(),
            size: 0,
            align: 1,
            uninhabited: true,
            fields: FieldLayoutRecipe::Primitive,
            variants: VariantRecipe::Empty,
            largest_niche: None,
            scalar_valid_ranges: Vec::new(),
        }
    }

    fn enum_niche_layout() -> TypeLayoutRecipe {
        TypeLayoutRecipe {
            ty: "Option<&i32>".into(),
            size: 8,
            align: 8,
            uninhabited: false,
            fields: FieldLayoutRecipe::Arbitrary {
                offsets: vec![0],
                memory_order: vec![0],
            },
            variants: VariantRecipe::Multiple {
                tag: ScalarLayoutRecipe {
                    primitive: "Pointer(AddressSpace(0))".into(),
                    valid_range: Some(ValidRangeRecipe {
                        start: 1,
                        end: u64::MAX.into(),
                    }),
                },
                tag_field: 0,
                tag_encoding: TagEncodingRecipe::Niche {
                    untagged_variant: 1,
                    niche_start: 0,
                    niche_variants_start: 0,
                    niche_variants_end: 0,
                },
                variants: vec![
                    VariantLayoutRecipe {
                        index: 0,
                        size: 8,
                        align: 8,
                        fields: FieldLayoutRecipe::Arbitrary {
                            offsets: Vec::new(),
                            memory_order: Vec::new(),
                        },
                    },
                    VariantLayoutRecipe {
                        index: 1,
                        size: 8,
                        align: 8,
                        fields: FieldLayoutRecipe::Arbitrary {
                            offsets: vec![0],
                            memory_order: vec![0],
                        },
                    },
                ],
            },
            largest_niche: Some(NicheRecipe {
                offset: 0,
                primitive: "Pointer(AddressSpace(0))".into(),
                valid_range: ValidRangeRecipe {
                    start: 1,
                    end: u64::MAX.into(),
                },
            }),
            scalar_valid_ranges: Vec::new(),
        }
    }

    fn assert_abi_error_contains(abi: FnAbiPlan, expected: &str) {
        let err = validate_fn_abi("test_fn", &abi, abi.arguments.len(), false)
            .expect_err("ABI should be rejected");
        assert!(
            err.contains(expected),
            "expected diagnostic containing `{expected}`, got `{err}`"
        );
    }

    fn memory_function(operations: Vec<Operation>) -> FunctionPlan {
        FunctionPlan {
            symbol: "memory_fn".into(),
            abi: fn_abi(vec![direct_abi_value(8, 8)], direct_abi_value(4, 4)),
            argument_locals: vec![1],
            return_local: Some(0),
            locals: vec![
                LocalPlan {
                    id: 0,
                    ty: ScalarType::I32,
                },
                LocalPlan {
                    id: 1,
                    ty: ScalarType::Ptr,
                },
                LocalPlan {
                    id: 2,
                    ty: ScalarType::I32,
                },
            ],
            blocks: vec![BasicBlockPlan {
                id: 0,
                operations,
                terminator: TerminatorPlan::Return,
            }],
        }
    }

    #[test]
    fn load_store_memory_operations_are_accepted() {
        let function = memory_function(vec![
            Operation::Load {
                dst: 2,
                ptr: ValueRef::Local(1),
                offset: 0,
                ty: ScalarType::I32,
                align: 4,
            },
            Operation::Store {
                ptr: ValueRef::Local(1),
                offset: 0,
                value: ValueRef::Local(2),
                ty: ScalarType::I32,
                align: 4,
            },
            Operation::Copy {
                dst: 0,
                src: ValueRef::Local(2),
            },
        ]);

        validate_function(&function, &BTreeMap::new(), &BTreeMap::new())
            .expect("load/store memory function should validate");
    }

    #[test]
    fn malformed_memory_operation_is_rejected() {
        let function = memory_function(vec![Operation::Load {
            dst: 2,
            ptr: ValueRef::Local(1),
            offset: 0,
            ty: ScalarType::I32,
            align: 3,
        }]);
        let err = validate_function(&function, &BTreeMap::new(), &BTreeMap::new())
            .expect_err("malformed load should be rejected");
        assert!(
            err.contains("invalid alignment"),
            "expected alignment diagnostic, got `{err}`"
        );
    }

    #[test]
    fn worker_diagnostic_response_carries_code_and_location() {
        let response = classified_response(
            7,
            "function add ABI argument 0 uses unsupported Pair pass mode".into(),
        );

        assert!(!response.success);
        assert_eq!(response.request_id, 7);
        assert_eq!(response.diagnostic_code, "SCI_ABI_UNSUPPORTED_PASS_MODE");
        assert_eq!(
            response.diagnostic_location,
            Some(DiagnosticLocation {
                function: Some("add".into()),
                block: None,
                local: None,
            })
        );
    }

    #[test]
    fn worker_diagnostic_location_extracts_local_numbers() {
        assert_eq!(
            diagnostic_location("function add argument local 3 is missing"),
            Some(DiagnosticLocation {
                function: Some("add".into()),
                block: None,
                local: Some(3),
            })
        );
    }

    #[test]
    fn direct_arguments_and_ignored_return_are_accepted() {
        let abi = fn_abi(
            vec![abi_value(AbiPassModePlan::Direct)],
            abi_value(AbiPassModePlan::Ignore),
        );

        validate_fn_abi("test_fn", &abi, 1, false).expect("direct ABI should validate");
    }

    #[test]
    fn abi_fixture_matrix_validates_current_boundary() {
        let mut rust_direct = fn_abi(vec![direct_abi_value(4, 4)], direct_abi_value(4, 4));
        rust_direct.convention = CallingConventionPlan::Rust;

        let mut fixed_count_mismatch = fn_abi(vec![direct_abi_value(4, 4)], ignored_abi_value());
        fixed_count_mismatch.fixed_count = 2;

        let mut variadic = fn_abi(vec![direct_abi_value(4, 4)], ignored_abi_value());
        variadic.variadic = true;

        let mut can_unwind = fn_abi(vec![direct_abi_value(4, 4)], ignored_abi_value());
        can_unwind.can_unwind = true;

        let mut other_convention = fn_abi(vec![direct_abi_value(4, 4)], ignored_abi_value());
        other_convention.convention = CallingConventionPlan::Other("fastcall".into());

        let fixtures = vec![
            (
                "c_direct_void",
                fn_abi(vec![direct_abi_value(4, 4)], ignored_abi_value()),
                1,
                false,
                None,
            ),
            (
                "c_direct_return",
                fn_abi(
                    vec![direct_abi_value(4, 4), direct_abi_value(8, 8)],
                    direct_abi_value(8, 8),
                ),
                2,
                true,
                None,
            ),
            ("rust_direct_return", rust_direct, 1, true, None),
            (
                "lowered_arg_count_mismatch",
                fn_abi(vec![direct_abi_value(4, 4)], ignored_abi_value()),
                2,
                false,
                Some("lowered 2"),
            ),
            (
                "fixed_count_mismatch",
                fixed_count_mismatch,
                1,
                false,
                Some("fixed_count"),
            ),
            (
                "direct_return_missing_destination",
                fn_abi(Vec::new(), direct_abi_value(4, 4)),
                0,
                false,
                Some("mode does not match"),
            ),
            (
                "ignore_return_with_destination",
                fn_abi(Vec::new(), ignored_abi_value()),
                0,
                true,
                Some("mode does not match"),
            ),
            (
                "pair_argument",
                fn_abi(vec![abi_value(AbiPassModePlan::Pair)], ignored_abi_value()),
                1,
                false,
                Some("unsupported Pair"),
            ),
            (
                "pair_return",
                fn_abi(Vec::new(), abi_value(AbiPassModePlan::Pair)),
                0,
                true,
                Some("unsupported Pair"),
            ),
            (
                "cast_argument",
                fn_abi(vec![cast_abi_value()], ignored_abi_value()),
                1,
                false,
                Some("unsupported Cast"),
            ),
            (
                "cast_return",
                fn_abi(Vec::new(), cast_abi_value()),
                0,
                true,
                Some("unsupported Cast"),
            ),
            (
                "indirect_argument",
                fn_abi(vec![indirect_abi_value()], ignored_abi_value()),
                1,
                false,
                Some("unsupported Indirect"),
            ),
            (
                "indirect_return",
                fn_abi(Vec::new(), indirect_abi_value()),
                0,
                true,
                Some("unsupported Indirect"),
            ),
            (
                "invalid_argument_alignment",
                fn_abi(vec![direct_abi_value(8, 3)], ignored_abi_value()),
                1,
                false,
                Some("invalid alignment"),
            ),
            (
                "invalid_return_size_alignment",
                fn_abi(Vec::new(), direct_abi_value(6, 4)),
                0,
                true,
                Some("not a multiple"),
            ),
            ("variadic", variadic, 1, false, Some("variadic")),
            ("can_unwind", can_unwind, 1, false, Some("unwinding")),
            (
                "other_convention",
                other_convention,
                1,
                false,
                Some("unsupported calling convention"),
            ),
        ];

        for (name, abi, lowered_argument_count, has_return_value, expected) in fixtures {
            let result = validate_fn_abi(name, &abi, lowered_argument_count, has_return_value);
            match expected {
                Some(expected) => {
                    let err = result.expect_err("ABI fixture should be rejected");
                    assert!(
                        err.contains(expected),
                        "fixture `{name}` expected diagnostic containing `{expected}`, got `{err}`"
                    );
                }
                None => result
                    .unwrap_or_else(|err| panic!("fixture `{name}` should validate, got `{err}`")),
            }
        }
    }

    #[test]
    fn supported_target_descriptor_is_accepted() {
        validate_target(&supported_target()).expect("target descriptor should validate");
    }

    #[test]
    fn target_data_layout_mismatch_is_rejected_before_emission() {
        let mut target = supported_target();
        target.data_layout = "e-p:64:64".into();

        let err = validate_target(&target).expect_err("target descriptor should be rejected");
        assert!(
            err.contains("unsupported target data layout"),
            "expected data-layout diagnostic, got `{err}`"
        );
    }

    #[test]
    fn struct_type_layout_recipe_is_accepted() {
        validate_type_layout(&struct_layout()).expect("type layout should validate");
    }

    #[test]
    fn type_layout_fixture_matrix_validates_current_boundary() {
        let primitive = TypeLayoutRecipe {
            ty: "i32".into(),
            size: 4,
            align: 4,
            uninhabited: false,
            fields: FieldLayoutRecipe::Primitive,
            variants: VariantRecipe::Single { index: 0 },
            largest_niche: None,
            scalar_valid_ranges: vec![scalar_layout()],
        };

        let mut inhabited_empty = empty_layout();
        inhabited_empty.uninhabited = false;

        let mut bad_alignment = struct_layout();
        bad_alignment.align = 0;

        let mut bad_size_alignment = struct_layout();
        bad_size_alignment.size = 6;

        let mut zero_union = union_layout();
        zero_union.fields = FieldLayoutRecipe::Union { count: 0 };

        let mut oversized_array = array_layout();
        oversized_array.fields = FieldLayoutRecipe::Array {
            stride: 8,
            count: 4,
        };

        let mut memory_order_out_of_range = struct_layout();
        memory_order_out_of_range.fields = FieldLayoutRecipe::Arbitrary {
            offsets: vec![0, 4],
            memory_order: vec![0, 2],
        };

        let mut inverted_niche_range = enum_niche_layout();
        if let VariantRecipe::Multiple { tag_encoding, .. } = &mut inverted_niche_range.variants {
            *tag_encoding = TagEncodingRecipe::Niche {
                untagged_variant: 1,
                niche_start: 0,
                niche_variants_start: 2,
                niche_variants_end: 1,
            };
        }

        let mut repeated_variant = enum_niche_layout();
        if let VariantRecipe::Multiple { variants, .. } = &mut repeated_variant.variants {
            variants[1].index = variants[0].index;
        }

        let mut tag_field_out_of_range = enum_niche_layout();
        if let VariantRecipe::Multiple { tag_field, .. } = &mut tag_field_out_of_range.variants {
            *tag_field = 2;
        }

        let mut invalid_variant_alignment = enum_niche_layout();
        if let VariantRecipe::Multiple { variants, .. } = &mut invalid_variant_alignment.variants {
            variants[0].align = 3;
        }

        let mut bad_niche = enum_niche_layout();
        if let Some(niche) = &mut bad_niche.largest_niche {
            niche.primitive.clear();
        }

        let mut bad_scalar = primitive.clone();
        bad_scalar.scalar_valid_ranges[0].primitive.clear();

        let fixtures = vec![
            ("primitive", primitive, None),
            ("struct", struct_layout(), None),
            ("union", union_layout(), None),
            ("array", array_layout(), None),
            ("empty", empty_layout(), None),
            ("enum_niche", enum_niche_layout(), None),
            ("inhabited_empty", inhabited_empty, Some("empty variants")),
            ("bad_alignment", bad_alignment, Some("invalid alignment")),
            (
                "bad_size_alignment",
                bad_size_alignment,
                Some("not a multiple"),
            ),
            ("zero_union", zero_union, Some("union field count is zero")),
            (
                "oversized_array",
                oversized_array,
                Some("exceed layout size"),
            ),
            (
                "memory_order_out_of_range",
                memory_order_out_of_range,
                Some("out of range"),
            ),
            (
                "inverted_niche_range",
                inverted_niche_range,
                Some("is inverted"),
            ),
            (
                "repeated_variant",
                repeated_variant,
                Some("repeats variant"),
            ),
            (
                "tag_field_out_of_range",
                tag_field_out_of_range,
                Some("tag field"),
            ),
            (
                "invalid_variant_alignment",
                invalid_variant_alignment,
                Some("invalid alignment"),
            ),
            ("bad_niche", bad_niche, Some("empty primitive")),
            ("bad_scalar", bad_scalar, Some("empty primitive")),
        ];

        for (name, layout, expected) in fixtures {
            let result = validate_type_layout(&layout);
            match expected {
                Some(expected) => {
                    let err = result.expect_err("type layout fixture should be rejected");
                    assert!(
                        err.contains(expected),
                        "fixture `{name}` expected diagnostic containing `{expected}`, got `{err}`"
                    );
                }
                None => result
                    .unwrap_or_else(|err| panic!("fixture `{name}` should validate, got `{err}`")),
            }
        }
    }

    #[test]
    fn malformed_type_layout_memory_order_is_rejected() {
        let mut layout = struct_layout();
        layout.fields = FieldLayoutRecipe::Arbitrary {
            offsets: vec![0, 4],
            memory_order: vec![0, 0],
        };

        let err = validate_type_layout(&layout).expect_err("type layout should be rejected");
        assert!(
            err.contains("appears more than once"),
            "expected memory-order diagnostic, got `{err}`"
        );
    }

    #[test]
    fn pair_argument_is_rejected_before_emission() {
        let abi = fn_abi(
            vec![abi_value(AbiPassModePlan::Pair)],
            abi_value(AbiPassModePlan::Ignore),
        );

        assert_abi_error_contains(abi, "unsupported Pair pass mode");
    }

    #[test]
    fn cast_argument_is_rejected_before_emission() {
        let abi = fn_abi(
            vec![abi_value(AbiPassModePlan::Cast {
                pad_i32: false,
                prefix: vec![integer_register(64)],
                rest_offset: None,
                rest: AbiUniformPlan {
                    unit: integer_register(64),
                    total_bytes: 8,
                    consecutive: true,
                },
            })],
            abi_value(AbiPassModePlan::Ignore),
        );

        assert_abi_error_contains(abi, "unsupported Cast pass mode");
    }

    #[test]
    fn indirect_argument_is_rejected_before_emission() {
        let abi = fn_abi(
            vec![abi_value(AbiPassModePlan::Indirect {
                has_metadata: false,
                on_stack: true,
            })],
            abi_value(AbiPassModePlan::Ignore),
        );

        assert_abi_error_contains(abi, "unsupported Indirect pass mode");
    }
}
