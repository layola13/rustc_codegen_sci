#![feature(rustc_private)]
#![warn(rust_2018_idioms)]

extern crate rustc_abi;
extern crate rustc_codegen_ssa;
extern crate rustc_driver as _;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;

use std::any::Any;
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use rustc_abi::{
    BackendRepr, CanonAbi, Endian as RustcEndian, ExternAbi, FieldIdx as LayoutFieldIdx,
    FieldsShape, Niche, Primitive, Reg, RegKind, Scalar as RustcScalar, TagEncoding,
    VariantIdx as LayoutVariantIdx, Variants,
};
use rustc_codegen_ssa::back::write::produce_final_output_artifacts;
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_codegen_ssa::{CompiledModule, CompiledModules, CrateInfo, ModuleKind, TargetConfig};
use rustc_middle::dep_graph::WorkProductMap;
use rustc_middle::mir::{
    AggregateKind, BasicBlock, BinOp, Body, ConstOperand, Local, Operand, Place, ProjectionElem,
    Rvalue, StatementKind, TerminatorKind, UnOp, UnwindAction,
};
use rustc_middle::mono::MonoItem;
use rustc_middle::ty::{self, Instance, Ty, TyCtxt};
use rustc_session::Session;
use rustc_session::config::{CrateType, OutFileName, OutputFilenames, OutputType};
use rustc_span::{Span, Symbol};
use rustc_target::callconv::{FnAbi, PassMode};
use rustc_target::spec::PanicStrategy;
use sci_protocol::{
    AbiPassModePlan, AbiRegisterKind, AbiRegisterPlan, AbiUniformPlan, AbiValuePlan,
    BasicBlockPlan, BinaryOp, CallSignaturePlan, CallingConventionPlan, CastOp, CompareOp,
    CompileRequest, CompileResponse, DiagnosticLocation, DiagnosticPayload, Endian,
    ExternFunctionPlan, FieldLayoutRecipe, FnAbiPlan, FunctionPlan, LocalPlan, NicheRecipe,
    Operation, PLAN_VERSION, ScalarLayoutRecipe, ScalarType, SciModulePlan, SwitchCasePlan,
    TagEncodingRecipe, TargetPlan, TerminatorPlan, TypeLayoutRecipe, ValidRangeRecipe, ValueRef,
    VariantLayoutRecipe, VariantRecipe, read_frame, write_frame,
};

const BACKEND_NAME: &str = "sci";
const RUSTC_COMMIT: &str = "fcbe7917ba18120d9eda136f1c7c5a60c78e554e";
const SUPPORTED_TARGET: &str = "x86_64-unknown-linux-gnu";

struct SciCodegenBackend;

impl CodegenBackend for SciCodegenBackend {
    fn name(&self) -> &'static str {
        BACKEND_NAME
    }

    fn init(&self, sess: &Session) {
        use rustc_session::config::{InstrumentCoverage, Lto};

        if sess.target.llvm_target != SUPPORTED_TARGET {
            sess.dcx().fatal(format!(
                "rustc_codegen_sci does not yet support target `{}`",
                sess.target.llvm_target
            ));
        }
        if sess.panic_strategy() != PanicStrategy::Abort {
            sess.dcx()
                .fatal("rustc_codegen_sci bring-up requires `-Cpanic=abort`");
        }
        match sess.lto() {
            Lto::No | Lto::ThinLocal => {}
            Lto::Thin | Lto::Fat => {
                sess.dcx()
                    .fatal("rustc_codegen_sci does not yet support LTO");
            }
        }
        if sess.opts.cg.instrument_coverage() != InstrumentCoverage::No {
            sess.dcx()
                .fatal("rustc_codegen_sci does not yet support coverage instrumentation");
        }
        if let Some(cpu) = &sess.opts.cg.target_cpu
            && cpu != sess.target.cpu.as_ref()
        {
            sess.dcx().fatal(format!(
                "rustc_codegen_sci does not yet support target CPU `{cpu}`"
            ));
        }
        if !sess.opts.cg.target_feature.is_empty() {
            sess.dcx()
                .fatal("rustc_codegen_sci does not yet support custom target features");
        }
    }

    fn target_config(&self, _sess: &Session) -> TargetConfig {
        TargetConfig {
            target_features: vec![Symbol::intern("x87"), Symbol::intern("sse2")],
            unstable_target_features: vec![Symbol::intern("x87"), Symbol::intern("sse2")],
            has_reliable_f16: false,
            has_reliable_f16_math: false,
            has_reliable_f128: false,
            has_reliable_f128_math: false,
        }
    }

    fn supported_crate_types(&self, _sess: &Session) -> Vec<CrateType> {
        vec![CrateType::Executable, CrateType::Rlib]
    }

    fn thin_lto_supported(&self) -> bool {
        false
    }

    fn target_cpu(&self, sess: &Session) -> String {
        sess.opts
            .cg
            .target_cpu
            .clone()
            .unwrap_or_else(|| sess.target.cpu.to_string())
    }

    fn codegen_crate<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Box<dyn Any> {
        let modules = match codegen_crate(tcx) {
            Ok(modules) => modules,
            Err(err) => emit_backend_diagnostic(tcx, err),
        };
        Box::new(SciOngoingCodegen { modules })
    }

    fn join_codegen(
        &self,
        ongoing_codegen: Box<dyn Any>,
        sess: &Session,
        outputs: &OutputFilenames,
        _crate_info: &CrateInfo,
    ) -> (CompiledModules, WorkProductMap) {
        let ongoing = ongoing_codegen
            .downcast::<SciOngoingCodegen>()
            .expect("rustc_codegen_sci received foreign ongoing-codegen state");
        produce_final_output_artifacts(sess, &ongoing.modules, outputs);
        (ongoing.modules, WorkProductMap::default())
    }
}

struct SciOngoingCodegen {
    modules: CompiledModules,
}

struct BackendDiagnostic {
    payload: DiagnosticPayload,
    span: Option<Span>,
}

impl BackendDiagnostic {
    fn new(message: impl Into<String>) -> Self {
        Self::with_location(message, None, None)
    }

    fn with_location(
        message: impl Into<String>,
        location: Option<DiagnosticLocation>,
        span: Option<Span>,
    ) -> Self {
        let message = message.into();
        Self {
            payload: backend_diagnostic_payload(message, location),
            span,
        }
    }
}

impl From<String> for BackendDiagnostic {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

fn emit_backend_diagnostic(tcx: TyCtxt<'_>, diagnostic: BackendDiagnostic) -> ! {
    let message = format_backend_rejection(&diagnostic.payload);
    if let Some(span) = diagnostic.span {
        tcx.dcx().span_fatal(span, message)
    } else {
        tcx.dcx().fatal(message)
    }
}

fn codegen_crate<'tcx>(tcx: TyCtxt<'tcx>) -> Result<CompiledModules, BackendDiagnostic> {
    let partitions = tcx.collect_and_partition_mono_items(());
    let outputs = tcx.output_filenames(());
    let sa_output_dir = match outputs.path(OutputType::Object) {
        OutFileName::Real(path) => path.parent().map(Path::to_path_buf),
        OutFileName::Stdout => None,
    };
    let mut modules = Vec::with_capacity(partitions.codegen_units.len());

    for cgu in partitions.codegen_units {
        let cgu_name = cgu.name().as_str().to_owned();
        let object_path = outputs.temp_path_for_cgu(OutputType::Object, &cgu_name);
        let mut functions = Vec::new();
        let mut module_state = ModuleLoweringState::default();

        for (mono_item, _item_data) in cgu.items_in_deterministic_order(tcx) {
            match mono_item {
                MonoItem::Fn(instance) => {
                    functions.push(lower_function(tcx, instance, &mut module_state)?);
                }
                MonoItem::Static(def_id) => {
                    return Err(BackendDiagnostic::new(format!(
                        "rustc_codegen_sci does not yet support static mono item `{}`",
                        tcx.def_path_str(def_id)
                    )));
                }
                MonoItem::GlobalAsm(item_id) => {
                    return Err(BackendDiagnostic::new(format!(
                        "rustc_codegen_sci does not yet support global_asm item `{:?}`",
                        item_id
                    )));
                }
            }
        }

        if functions.is_empty() {
            return Err(BackendDiagnostic::new(format!(
                "rustc_codegen_sci produced no supported functions for CGU `{cgu_name}`"
            )));
        }

        let module = SciModulePlan {
            plan_version: PLAN_VERSION,
            rustc_commit: RUSTC_COMMIT.to_owned(),
            target: TargetPlan {
                triple: tcx.sess.target.llvm_target.to_string(),
                object_format: tcx.sess.target.binary_format.to_string(),
                data_layout: tcx.sess.target.data_layout.to_string(),
                pointer_width: u8::try_from(tcx.sess.target.pointer_width)
                    .map_err(|_| "target pointer width does not fit in SCI protocol".to_string())?,
                endian: match tcx.data_layout.endian {
                    RustcEndian::Little => Endian::Little,
                    RustcEndian::Big => Endian::Big,
                },
                cpu: tcx
                    .sess
                    .opts
                    .cg
                    .target_cpu
                    .clone()
                    .unwrap_or_else(|| tcx.sess.target.cpu.to_string()),
                features: tcx.sess.target.features.to_string(),
                relocation_model: tcx.sess.relocation_model().to_string(),
                code_model: tcx.sess.code_model().map(|model| model.to_string()),
            },
            cgu_name: cgu_name.clone(),
            type_layouts: module_state.type_layouts.into_values().collect(),
            extern_functions: module_state.extern_functions.into_values().collect(),
            functions,
        };

        let sa_path = sa_output_dir
            .as_ref()
            .map(|dir| dir.join(format!("{cgu_name}.sci.sa")));
        run_worker(
            &module,
            &object_path,
            sa_path.as_deref(),
            1 + modules.len() as u64,
        )?;
        modules.push(CompiledModule {
            name: cgu_name,
            kind: ModuleKind::Regular,
            object: Some(object_path),
            global_asm_object: None,
            dwarf_object: None,
            bytecode: None,
            assembly: None,
            llvm_ir: None,
            links_from_incr_cache: Vec::new(),
        });
    }

    Ok(CompiledModules {
        modules,
        allocator_module: None,
    })
}

#[derive(Default)]
struct ModuleLoweringState {
    extern_functions: BTreeMap<String, ExternFunctionPlan>,
    type_layouts: BTreeMap<String, TypeLayoutRecipe>,
}

impl ModuleLoweringState {
    fn register_extern_function(
        &mut self,
        extern_function: ExternFunctionPlan,
    ) -> Result<(), String> {
        if let Some(existing) = self.extern_functions.get(&extern_function.symbol) {
            if existing != &extern_function {
                return Err(format!(
                    "extern function `{}` is referenced with incompatible signatures",
                    extern_function.symbol
                ));
            }
            return Ok(());
        }
        self.extern_functions
            .insert(extern_function.symbol.clone(), extern_function);
        Ok(())
    }

    fn register_type_layout<'tcx>(
        &mut self,
        tcx: TyCtxt<'tcx>,
        ty: Ty<'tcx>,
    ) -> Result<(), String> {
        let key = layout_type_name(ty);
        if self.type_layouts.contains_key(&key) {
            return Ok(());
        }
        let recipe = lower_type_layout_recipe(tcx, ty)?;
        self.type_layouts.insert(key, recipe);
        Ok(())
    }
}

fn lower_function<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    module_state: &mut ModuleLoweringState,
) -> Result<FunctionPlan, BackendDiagnostic> {
    let mir = tcx.instance_mir(instance.def);
    let fn_abi = lower_fn_abi_plan(tcx, instance)?;
    validate_backend_fn_abi_boundary(tcx, instance, &fn_abi)?;

    let stack_slot_recipes = stack_slot_recipes(tcx, instance, mir)?;
    let mut state = LoweringState::new(mir.local_decls.len());
    let mut locals = Vec::with_capacity(mir.local_decls.len());
    for (local, decl) in mir.local_decls.iter_enumerated() {
        let ty = monomorphize_ty(tcx, instance, decl.ty);
        module_state.register_type_layout(tcx, ty)?;
        if let Some(ty) = scalar_type_for_ty(ty) {
            locals.push(LocalPlan {
                id: local_id(local),
                ty,
            });
            if let Some(recipe) = stack_slot_recipes.get(&local_id(local)) {
                state.allocate_stack_slot(local, recipe.size, recipe.align, recipe.ty);
            }
        } else if is_unit_ty(ty) {
            if local.index() != 0 && local.index() <= mir.arg_count {
                return Err(BackendDiagnostic::new(format!(
                    "{}: unit function arguments are not supported by the current ABI plan",
                    tcx.symbol_name(instance).name
                )));
            }
        } else if is_empty_struct_ty(ty) {
            if local == rustc_middle::mir::RETURN_PLACE
                || local.index() != 0 && local.index() <= mir.arg_count
            {
                return Err(BackendDiagnostic::new(format!(
                    "{}: zero-sized struct function ABI is not yet supported",
                    tcx.symbol_name(instance).name
                )));
            }
        } else if let Some(field_types) = scalar_aggregate_field_types(tcx, ty) {
            if local == rustc_middle::mir::RETURN_PLACE {
                if !is_supported_aggregate_return_abi(&field_types, &fn_abi.return_value) {
                    return Err(BackendDiagnostic::with_location(
                        format!(
                            "{}: aggregate return ABI is not yet supported",
                            tcx.symbol_name(instance).name
                        ),
                        Some(DiagnosticLocation {
                            function: Some(tcx.symbol_name(instance).name.to_string()),
                            block: None,
                            local: Some(0),
                        }),
                        None,
                    ));
                }
            }
            if local.index() != 0 && local.index() <= mir.arg_count {
                let argument = &fn_abi.arguments[local.index() - 1];
                if !is_supported_aggregate_argument_abi(&field_types, argument) {
                    return Err(BackendDiagnostic::with_location(
                        format!(
                            "{}: aggregate argument ABI is not yet supported",
                            tcx.symbol_name(instance).name
                        ),
                        Some(DiagnosticLocation {
                            function: Some(tcx.symbol_name(instance).name.to_string()),
                            block: None,
                            local: Some(local.index() as u32),
                        }),
                        None,
                    ));
                }
            }
            for (field, ty) in field_types.into_iter().enumerate() {
                let id = state.synthetic_tuple_field(local, field);
                locals.push(LocalPlan { id, ty });
            }
        } else {
            return Err(BackendDiagnostic::with_location(
                format!(
                    "{}: local {:?} has unsupported type `{}`",
                    tcx.symbol_name(instance).name,
                    local,
                    ty
                ),
                Some(DiagnosticLocation {
                    function: Some(tcx.symbol_name(instance).name.to_string()),
                    block: None,
                    local: Some(local.index() as u32),
                }),
                None,
            ));
        }
    }

    let mut argument_locals = Vec::with_capacity(mir.arg_count);
    for index in 0..mir.arg_count {
        let local = rustc_middle::mir::Local::arg(index);
        let ty = monomorphize_ty(tcx, instance, mir.local_decls[local].ty);
        if is_unit_ty(ty) {
            continue;
        }
        if let Some(field_types) = scalar_aggregate_field_types(tcx, ty) {
            for field in 0..field_types.len() {
                argument_locals.push(state.tuple_field(local, field).ok_or_else(|| {
                    BackendDiagnostic::with_location(
                        format!(
                            "{}: aggregate argument field {field} is missing a synthetic local",
                            tcx.symbol_name(instance).name
                        ),
                        Some(DiagnosticLocation {
                            function: Some(tcx.symbol_name(instance).name.to_string()),
                            block: None,
                            local: Some(local.index() as u32),
                        }),
                        None,
                    )
                })?);
            }
        } else {
            argument_locals.push(local_id(local));
        }
    }
    let return_ty = monomorphize_ty(
        tcx,
        instance,
        mir.local_decls[rustc_middle::mir::RETURN_PLACE].ty,
    );
    let return_locals = if is_unit_ty(return_ty) {
        Vec::new()
    } else if let Some(field_types) = scalar_aggregate_field_types(tcx, return_ty) {
        field_types
            .iter()
            .enumerate()
            .map(|(field, _)| {
                state
                    .tuple_field(rustc_middle::mir::RETURN_PLACE, field)
                    .ok_or_else(|| {
                        BackendDiagnostic::with_location(
                            format!(
                                "{}: aggregate return field {field} is missing a synthetic local",
                                tcx.symbol_name(instance).name
                            ),
                            Some(DiagnosticLocation {
                                function: Some(tcx.symbol_name(instance).name.to_string()),
                                block: None,
                                local: Some(0),
                            }),
                            None,
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        vec![local_id(rustc_middle::mir::RETURN_PLACE)]
    };

    let mut blocks = Vec::with_capacity(mir.basic_blocks.len());
    for (block_id, block) in mir.basic_blocks.iter_enumerated() {
        let mut operations = Vec::new();
        if block_id.index() == 0 {
            operations.extend(
                state
                    .stack_slots
                    .values()
                    .map(|slot| Operation::StackAlloc {
                        dst: slot.ptr,
                        size: slot.size,
                        align: slot.align,
                    }),
            );
        }
        for (statement_index, statement) in block.statements.iter().enumerate() {
            match &statement.kind {
                StatementKind::Assign(assign) => {
                    let (place, rvalue) = &**assign;
                    operations.extend(
                        lower_assignment(tcx, instance, mir, &mut state, *place, rvalue).map_err(
                            |err| {
                                annotate_mir_statement_error(
                                    tcx,
                                    instance,
                                    block_id,
                                    statement_index,
                                    statement.source_info.span,
                                    err,
                                )
                            },
                        )?,
                    );
                }
                StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::Nop => {}
                other => {
                    return Err(annotate_mir_statement_error(
                        tcx,
                        instance,
                        block_id,
                        statement_index,
                        statement.source_info.span,
                        format!("unsupported MIR statement `{other:?}`"),
                    ));
                }
            }
        }
        blocks.push(BasicBlockPlan {
            id: block_id_id(block_id),
            operations,
            terminator: lower_terminator(
                tcx,
                instance,
                mir,
                &state,
                module_state,
                &block.terminator().kind,
            )
            .map_err(|err| {
                annotate_mir_terminator_error(
                    tcx,
                    instance,
                    block_id,
                    block.terminator().source_info.span,
                    err,
                )
            })?,
        });
    }

    locals.extend(state.synthetic_locals);

    Ok(FunctionPlan {
        symbol: tcx.symbol_name(instance).name.to_string(),
        abi: fn_abi,
        argument_locals,
        return_locals,
        locals,
        blocks,
    })
}

fn validate_backend_fn_abi_boundary<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    abi: &FnAbiPlan,
) -> Result<(), BackendDiagnostic> {
    let symbol = tcx.symbol_name(instance).name;
    let span = tcx.def_span(instance.def_id());
    for (index, argument) in abi.arguments.iter().enumerate() {
        if is_supported_cast_abi_value(argument)
            || is_supported_wide_cast_abi_argument_value(argument)
            || matches!(argument.mode, AbiPassModePlan::Pair)
        {
            continue;
        }
        if let Some(mode) = unsupported_backend_pass_mode(&argument.mode) {
            return Err(BackendDiagnostic::with_location(
                format!("{symbol}: ABI argument {index} uses unsupported {mode} pass mode"),
                Some(DiagnosticLocation {
                    function: Some(symbol.to_string()),
                    block: None,
                    local: None,
                }),
                Some(span),
            ));
        }
    }
    if is_supported_cast_abi_value(&abi.return_value)
        || is_supported_wide_cast_abi_argument_value(&abi.return_value)
        || matches!(abi.return_value.mode, AbiPassModePlan::Pair)
    {
        return Ok(());
    }
    if let Some(mode) = unsupported_backend_pass_mode(&abi.return_value.mode) {
        return Err(BackendDiagnostic::with_location(
            format!("{symbol}: ABI return uses unsupported {mode} pass mode"),
            Some(DiagnosticLocation {
                function: Some(symbol.to_string()),
                block: None,
                local: None,
            }),
            Some(span),
        ));
    }
    Ok(())
}

fn is_supported_cast_abi_value(value: &AbiValuePlan) -> bool {
    let AbiPassModePlan::Cast {
        pad_i32,
        prefix,
        rest_offset,
        rest,
    } = &value.mode
    else {
        return false;
    };
    !*pad_i32
        && prefix.is_empty()
        && rest_offset.is_none()
        && rest.unit.kind == AbiRegisterKind::Integer
        && rest.total_bytes == value.size
        && matches!(value.size, 1 | 2 | 4 | 8)
        && value.align <= 8
}

fn is_supported_pair_abi_argument(field_types: &[ScalarType], value: &AbiValuePlan) -> bool {
    matches!(value.mode, AbiPassModePlan::Pair)
        && field_types.len() == 2
        && value.align <= 8
        && field_types
            .iter()
            .copied()
            .map(scalar_type_size_bytes)
            .try_fold(0_u64, |sum, size| Some(sum + size?))
            == Some(value.size)
}

fn is_supported_wide_cast_abi_argument_value(value: &AbiValuePlan) -> bool {
    let AbiPassModePlan::Cast {
        pad_i32,
        prefix,
        rest_offset,
        rest,
    } = &value.mode
    else {
        return false;
    };
    let prefix_bytes = prefix
        .iter()
        .try_fold(0_u64, |sum, register| {
            (register.kind == AbiRegisterKind::Integer)
                .then_some(sum + register.bits.checked_div(8)?)
        })
        .unwrap_or(u64::MAX);
    !*pad_i32
        && prefix.len() == 1
        && rest_offset.is_none()
        && rest.unit.kind == AbiRegisterKind::Integer
        && prefix_bytes + rest.total_bytes == value.size
        && value.size == 16
        && value.align <= 8
}

fn is_supported_wide_cast_abi_argument(field_types: &[ScalarType], value: &AbiValuePlan) -> bool {
    is_supported_wide_cast_abi_argument_value(value)
        && field_types.len() == 2
        && field_types
            .iter()
            .copied()
            .map(scalar_type_size_bytes)
            .try_fold(0_u64, |sum, size| Some(sum + size?))
            == Some(value.size)
}

fn is_supported_aggregate_argument_abi(field_types: &[ScalarType], value: &AbiValuePlan) -> bool {
    if is_supported_cast_abi_value(value) {
        return field_types.len() == 1
            && scalar_type_size_bytes(field_types[0]) == Some(value.size);
    }
    is_supported_pair_abi_argument(field_types, value)
        || is_supported_wide_cast_abi_argument(field_types, value)
}

fn is_supported_aggregate_return_abi(field_types: &[ScalarType], value: &AbiValuePlan) -> bool {
    is_supported_aggregate_argument_abi(field_types, value)
}

fn unsupported_backend_pass_mode(mode: &AbiPassModePlan) -> Option<&'static str> {
    match mode {
        AbiPassModePlan::Ignore | AbiPassModePlan::Direct => None,
        AbiPassModePlan::Pair => Some("Pair"),
        AbiPassModePlan::Cast { .. } => Some("Cast"),
        AbiPassModePlan::Indirect { .. } => Some("Indirect"),
    }
}

fn stack_slot_recipes<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
) -> Result<BTreeMap<u32, StackSlotRecipe>, String> {
    let mut slots = BTreeMap::new();
    for block in mir.basic_blocks.iter() {
        for statement in &block.statements {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let (_, rvalue) = &**assign;
            let place = match rvalue {
                Rvalue::Ref(_, _, place) | Rvalue::RawPtr(_, place) => *place,
                _ => continue,
            };
            if !is_supported_stack_slot_place(mir, place) {
                continue;
            }
            let ty = monomorphize_ty(tcx, instance, mir.local_decls[place.local].ty);
            let Ok((scalar_ty, size, align)) = scalar_memory_layout(tcx, instance, ty) else {
                continue;
            };
            slots.insert(
                local_id(place.local),
                StackSlotRecipe {
                    size,
                    align,
                    ty: scalar_ty,
                },
            );
        }
    }
    Ok(slots)
}

fn is_supported_stack_slot_place(mir: &Body<'_>, place: Place<'_>) -> bool {
    place.projection.is_empty()
        && place.local != rustc_middle::mir::RETURN_PLACE
        && place.local.index() > mir.arg_count
}

fn annotate_mir_statement_error<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    block: BasicBlock,
    statement_index: usize,
    span: Span,
    err: String,
) -> BackendDiagnostic {
    annotate_mir_error(
        tcx,
        instance,
        block,
        &format!("statement {statement_index}"),
        span,
        err,
    )
}

fn annotate_mir_terminator_error<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    block: BasicBlock,
    span: Span,
    err: String,
) -> BackendDiagnostic {
    annotate_mir_error(tcx, instance, block, "terminator", span, err)
}

fn annotate_mir_error<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    block: BasicBlock,
    site: &str,
    span: Span,
    err: String,
) -> BackendDiagnostic {
    let symbol = tcx.symbol_name(instance).name;
    let prefix = format!("{symbol}: ");
    let detail = err.strip_prefix(&prefix).unwrap_or(&err);
    BackendDiagnostic::with_location(
        format!("{symbol}: block {} {site}: {detail}", block.index()),
        Some(DiagnosticLocation {
            function: Some(symbol.to_string()),
            block: Some(block.index() as u32),
            local: None,
        }),
        Some(span),
    )
}

struct LoweringState {
    next_synthetic_local: u32,
    tuple_fields: BTreeMap<(u32, usize), u32>,
    stack_slots: BTreeMap<u32, StackSlot>,
    synthetic_locals: Vec<LocalPlan>,
}

#[derive(Clone, Copy)]
struct StackSlot {
    ptr: u32,
    size: u64,
    align: u64,
    ty: ScalarType,
}

#[derive(Clone, Copy)]
struct StackSlotRecipe {
    size: u64,
    align: u64,
    ty: ScalarType,
}

impl LoweringState {
    fn new(mir_local_count: usize) -> Self {
        Self {
            next_synthetic_local: u32::try_from(mir_local_count)
                .expect("MIR local count exceeds u32"),
            tuple_fields: BTreeMap::new(),
            stack_slots: BTreeMap::new(),
            synthetic_locals: Vec::new(),
        }
    }

    fn synthetic_tuple_field(&mut self, local: Local, field: usize) -> u32 {
        let key = (local_id(local), field);
        if let Some(id) = self.tuple_fields.get(&key) {
            return *id;
        }
        let id = self.allocate_synthetic();
        self.tuple_fields.insert(key, id);
        id
    }

    fn tuple_field(&self, local: Local, field: usize) -> Option<u32> {
        self.tuple_fields.get(&(local_id(local), field)).copied()
    }

    fn allocate_stack_slot(&mut self, local: Local, size: u64, align: u64, ty: ScalarType) -> u32 {
        let key = local_id(local);
        if let Some(slot) = self.stack_slots.get(&key) {
            return slot.ptr;
        }
        let ptr = self.allocate_synthetic();
        self.stack_slots.insert(
            key,
            StackSlot {
                ptr,
                size,
                align,
                ty,
            },
        );
        self.synthetic_locals.push(LocalPlan {
            id: ptr,
            ty: ScalarType::Ptr,
        });
        ptr
    }

    fn stack_slot(&self, local: Local) -> Option<StackSlot> {
        self.stack_slots.get(&local_id(local)).copied()
    }

    fn allocate_temp(&mut self, ty: ScalarType) -> u32 {
        let id = self.allocate_synthetic();
        self.synthetic_locals.push(LocalPlan { id, ty });
        id
    }

    fn allocate_synthetic(&mut self) -> u32 {
        let id = self.next_synthetic_local;
        self.next_synthetic_local = self
            .next_synthetic_local
            .checked_add(1)
            .expect("synthetic local id overflow");
        id
    }
}

fn lower_terminator<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
    state: &LoweringState,
    module_state: &mut ModuleLoweringState,
    terminator: &TerminatorKind<'tcx>,
) -> Result<TerminatorPlan, String> {
    match terminator {
        TerminatorKind::Return => Ok(TerminatorPlan::Return),
        TerminatorKind::Goto { target } => Ok(TerminatorPlan::Goto {
            target: block_id_id(*target),
        }),
        TerminatorKind::SwitchInt { discr, targets } => {
            let discr_ty = monomorphize_ty(tcx, instance, discr.ty(&mir.local_decls, tcx));
            let discr_scalar = scalar_type_for_ty(discr_ty).ok_or_else(|| {
                format!(
                    "{}: SwitchInt discriminator has unsupported type `{}`",
                    tcx.symbol_name(instance).name,
                    discr_ty
                )
            })?;
            if discr_scalar == ScalarType::I1 {
                let false_target = targets.target_for_value(0);
                let true_target = targets.otherwise();
                if targets.target_for_value(1) != true_target {
                    return Err(format!(
                        "{}: bool SwitchInt must use otherwise for the true edge",
                        tcx.symbol_name(instance).name
                    ));
                }
                return Ok(TerminatorPlan::Branch {
                    condition: lower_operand(tcx, instance, mir, state, discr)?,
                    true_target: block_id_id(true_target),
                    false_target: block_id_id(false_target),
                });
            }
            let cases = targets
                .iter()
                .map(|(value, target)| {
                    Ok(SwitchCasePlan {
                        value: ValueRef::Integer {
                            ty: discr_scalar,
                            bits: u64::try_from(value).map_err(|_| {
                                format!(
                                    "{}: SwitchInt case value exceeds 64 bits",
                                    tcx.symbol_name(instance).name
                                )
                            })?,
                        },
                        target: block_id_id(target),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(TerminatorPlan::SwitchInt {
                discr: lower_operand(tcx, instance, mir, state, discr)?,
                cases,
                otherwise: block_id_id(targets.otherwise()),
            })
        }
        TerminatorKind::Call {
            func,
            args,
            destination,
            target,
            unwind,
            ..
        } => {
            if *unwind != UnwindAction::Unreachable {
                return Err(format!(
                    "{}: only calls with unreachable unwind are currently supported",
                    tcx.symbol_name(instance).name
                ));
            }
            let target = target.ok_or_else(|| {
                format!(
                    "{}: divergent calls are not currently supported",
                    tcx.symbol_name(instance).name
                )
            })?;
            let args = args
                .iter()
                .map(|arg| lower_call_arguments(tcx, instance, mir, state, &arg.node))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            let lowered_arg_count = args.len();
            if func.const_fn_def().is_some() {
                let callee = lower_direct_callee(tcx, instance, func)?;
                let callee_symbol = tcx.symbol_name(callee).name.to_string();
                if tcx.is_foreign_item(callee.def_id()) {
                    let extern_function = lower_extern_function(
                        tcx,
                        instance,
                        callee,
                        module_state,
                        mir,
                        func,
                        lowered_arg_count,
                        destination,
                    )?;
                    module_state.register_extern_function(extern_function)?;
                }
                return Ok(TerminatorPlan::Call {
                    callee: callee_symbol,
                    args,
                    destinations: lower_call_destinations(tcx, instance, mir, state, *destination)?,
                    target: block_id_id(target),
                });
            }

            Ok(TerminatorPlan::CallIndirect {
                callee: lower_operand(tcx, instance, mir, state, func)?,
                args,
                signature: lower_indirect_call_signature(
                    tcx,
                    instance,
                    mir,
                    module_state,
                    func,
                    lowered_arg_count,
                    destination,
                )?,
                destinations: lower_call_destinations(tcx, instance, mir, state, *destination)?,
                target: block_id_id(target),
            })
        }
        TerminatorKind::Assert {
            cond,
            expected,
            target,
            unwind,
            ..
        } => {
            if *unwind != UnwindAction::Unreachable {
                return Err(format!(
                    "{}: only asserts with unreachable unwind are currently supported",
                    tcx.symbol_name(instance).name
                ));
            }
            Ok(TerminatorPlan::Assert {
                condition: lower_operand(tcx, instance, mir, state, cond)?,
                expected: *expected,
                target: block_id_id(*target),
                panic_code: 1001,
            })
        }
        other => Err(format!(
            "{}: unsupported MIR terminator `{other:?}`",
            tcx.symbol_name(instance).name
        )),
    }
}

fn lower_direct_callee<'tcx>(
    tcx: TyCtxt<'tcx>,
    caller: Instance<'tcx>,
    func: &Operand<'tcx>,
) -> Result<Instance<'tcx>, String> {
    let (def_id, args) = func.const_fn_def().ok_or_else(|| {
        format!(
            "{}: only direct function calls are currently supported",
            tcx.symbol_name(caller).name
        )
    })?;
    let callee =
        Instance::resolve_for_fn_ptr(tcx, ty::TypingEnv::fully_monomorphized(), def_id, args)
            .ok_or_else(|| {
                format!(
                    "{}: failed to resolve direct callee `{}`",
                    tcx.symbol_name(caller).name,
                    tcx.def_path_str(def_id)
                )
            })?;
    Ok(callee)
}

fn lower_extern_function<'tcx>(
    tcx: TyCtxt<'tcx>,
    caller: Instance<'tcx>,
    callee: Instance<'tcx>,
    module_state: &mut ModuleLoweringState,
    mir: &Body<'tcx>,
    func: &Operand<'tcx>,
    lowered_arg_count: usize,
    destination: &Place<'tcx>,
) -> Result<ExternFunctionPlan, String> {
    let (def_id, args) = func.const_fn_def().ok_or_else(|| {
        format!(
            "{}: only direct extern function calls are currently supported",
            tcx.symbol_name(caller).name
        )
    })?;
    let sig = tcx.fn_sig(def_id).instantiate(tcx, args).skip_norm_wip();
    if !matches!(sig.abi(), ExternAbi::C { unwind: false }) {
        return Err(format!(
            "{}: extern callee `{}` uses unsupported ABI {}",
            tcx.symbol_name(caller).name,
            tcx.def_path_str(callee.def_id()),
            sig.abi()
        ));
    }
    if sig.c_variadic() {
        return Err(format!(
            "{}: variadic extern callee `{}` is not supported",
            tcx.symbol_name(caller).name,
            tcx.def_path_str(callee.def_id())
        ));
    }
    let signature_inputs = sig.inputs().skip_binder();
    let signature_input_count = signature_inputs.len();

    let fn_abi = lower_fn_abi_plan(tcx, callee)?;
    let mut argument_types = Vec::with_capacity(signature_input_count);
    for (index, input) in signature_inputs.iter().enumerate() {
        let ty = monomorphize_ty(tcx, caller, *input);
        module_state.register_type_layout(tcx, ty)?;
        if let Some(scalar) = scalar_type_for_ty(ty) {
            argument_types.push(scalar);
        } else {
            let field_types = scalar_aggregate_field_types(tcx, ty).ok_or_else(|| {
                format!(
                    "{}: extern callee `{}` argument has unsupported type `{}`",
                    tcx.symbol_name(caller).name,
                    tcx.def_path_str(callee.def_id()),
                    ty
                )
            })?;
            let abi_argument = &fn_abi.arguments[index];
            if !is_supported_aggregate_argument_abi(&field_types, abi_argument) {
                return Err(format!(
                    "{}: extern callee `{}` aggregate argument ABI is not yet supported: mode {:?}, size {}, align {}, fields {:?}",
                    tcx.symbol_name(caller).name,
                    tcx.def_path_str(callee.def_id()),
                    abi_argument.mode,
                    abi_argument.size,
                    abi_argument.align,
                    field_types
                ));
            }
            argument_types.extend(field_types);
        }
    }
    if lowered_arg_count != argument_types.len() {
        return Err(format!(
            "{}: extern callee `{}` lowered with {lowered_arg_count} args, expected {}",
            tcx.symbol_name(caller).name,
            tcx.def_path_str(callee.def_id()),
            argument_types.len()
        ));
    }

    let sig_return_ty = monomorphize_ty(tcx, caller, sig.output().skip_binder());
    module_state.register_type_layout(tcx, sig_return_ty)?;
    let destination_ty = monomorphize_ty(tcx, caller, destination.ty(&mir.local_decls, tcx).ty);
    module_state.register_type_layout(tcx, destination_ty)?;
    let return_type = if is_unit_ty(sig_return_ty) {
        if !is_unit_ty(destination_ty) {
            return Err(format!(
                "{}: void extern callee `{}` returns into non-unit destination `{}`",
                tcx.symbol_name(caller).name,
                tcx.def_path_str(callee.def_id()),
                destination_ty
            ));
        }
        Vec::new()
    } else if let Some(return_type) = scalar_type_for_ty(destination_ty) {
        if scalar_type_for_ty(sig_return_ty) != Some(return_type) {
            return Err(format!(
                "{}: extern callee `{}` return type does not match destination",
                tcx.symbol_name(caller).name,
                tcx.def_path_str(callee.def_id())
            ));
        }
        vec![return_type]
    } else {
        let destination_field_types = scalar_aggregate_field_types(tcx, destination_ty)
            .ok_or_else(|| {
                format!(
                    "{}: extern callee `{}` return destination has unsupported type `{}`",
                    tcx.symbol_name(caller).name,
                    tcx.def_path_str(callee.def_id()),
                    destination_ty
                )
            })?;
        let signature_field_types =
            scalar_aggregate_field_types(tcx, sig_return_ty).ok_or_else(|| {
                format!(
                    "{}: extern callee `{}` return type has unsupported type `{}`",
                    tcx.symbol_name(caller).name,
                    tcx.def_path_str(callee.def_id()),
                    sig_return_ty
                )
            })?;
        if destination_field_types != signature_field_types {
            return Err(format!(
                "{}: extern callee `{}` return type does not match destination",
                tcx.symbol_name(caller).name,
                tcx.def_path_str(callee.def_id())
            ));
        }
        if !is_supported_aggregate_return_abi(&destination_field_types, &fn_abi.return_value) {
            return Err(format!(
                "{}: extern callee `{}` aggregate return ABI is not yet supported",
                tcx.symbol_name(caller).name,
                tcx.def_path_str(callee.def_id())
            ));
        }
        destination_field_types
    };

    Ok(ExternFunctionPlan {
        symbol: tcx.symbol_name(callee).name.to_string(),
        abi: fn_abi,
        argument_types,
        return_types: return_type,
    })
}

fn lower_indirect_call_signature<'tcx>(
    tcx: TyCtxt<'tcx>,
    caller: Instance<'tcx>,
    mir: &Body<'tcx>,
    module_state: &mut ModuleLoweringState,
    func: &Operand<'tcx>,
    lowered_arg_count: usize,
    destination: &Place<'tcx>,
) -> Result<CallSignaturePlan, String> {
    let fn_ty = monomorphize_ty(tcx, caller, func.ty(&mir.local_decls, tcx));
    if !fn_ty.is_fn_ptr() {
        return Err(format!(
            "{}: only direct function calls or function pointer calls are currently supported",
            tcx.symbol_name(caller).name
        ));
    }
    let fn_sig = fn_ty.fn_sig(tcx);
    if !matches!(fn_sig.abi(), ExternAbi::C { unwind: false }) {
        return Err(format!(
            "{}: function pointer call uses unsupported ABI {}",
            tcx.symbol_name(caller).name,
            fn_sig.abi()
        ));
    }
    if fn_sig.c_variadic() {
        return Err(format!(
            "{}: variadic function pointer calls are not supported",
            tcx.symbol_name(caller).name
        ));
    }
    let fn_abi = tcx
        .fn_abi_of_fn_ptr(
            ty::TypingEnv::fully_monomorphized().as_query_input((fn_sig, ty::List::empty())),
        )
        .map_err(|err| {
            format!(
                "{}: failed to compute function pointer FnAbi: {err:?}",
                tcx.symbol_name(caller).name
            )
        })?;
    validate_backend_indirect_call_abi(tcx, caller, lower_rustc_fn_abi(fn_abi))?;

    let signature_inputs = fn_sig.inputs().skip_binder();
    let signature_input_count = signature_inputs.len();
    if lowered_arg_count != signature_input_count {
        return Err(format!(
            "{}: function pointer call lowered with {lowered_arg_count} args, expected {signature_input_count}",
            tcx.symbol_name(caller).name
        ));
    }
    let mut argument_types = Vec::with_capacity(signature_input_count);
    for input in signature_inputs.iter() {
        let ty = monomorphize_ty(tcx, caller, *input);
        module_state.register_type_layout(tcx, ty)?;
        argument_types.push(scalar_type_for_ty(ty).ok_or_else(|| {
            format!(
                "{}: function pointer argument has unsupported type `{}`",
                tcx.symbol_name(caller).name,
                ty
            )
        })?);
    }

    let sig_return_ty = monomorphize_ty(tcx, caller, fn_sig.output().skip_binder());
    module_state.register_type_layout(tcx, sig_return_ty)?;
    let destination_ty = monomorphize_ty(tcx, caller, destination.ty(&mir.local_decls, tcx).ty);
    module_state.register_type_layout(tcx, destination_ty)?;
    let return_types = if is_unit_ty(sig_return_ty) {
        if !is_unit_ty(destination_ty) {
            return Err(format!(
                "{}: void function pointer call returns into non-unit destination `{}`",
                tcx.symbol_name(caller).name,
                destination_ty
            ));
        }
        Vec::new()
    } else {
        let return_type = scalar_type_for_ty(destination_ty).ok_or_else(|| {
            format!(
                "{}: function pointer return destination has unsupported type `{}`",
                tcx.symbol_name(caller).name,
                destination_ty
            )
        })?;
        if scalar_type_for_ty(sig_return_ty) != Some(return_type) {
            return Err(format!(
                "{}: function pointer return type does not match destination",
                tcx.symbol_name(caller).name
            ));
        }
        vec![return_type]
    };

    Ok(CallSignaturePlan {
        argument_types,
        return_types,
    })
}

fn validate_backend_indirect_call_abi<'tcx>(
    tcx: TyCtxt<'tcx>,
    caller: Instance<'tcx>,
    abi: FnAbiPlan,
) -> Result<(), String> {
    for (index, argument) in abi.arguments.iter().enumerate() {
        if let Some(mode) = unsupported_backend_pass_mode(&argument.mode) {
            return Err(format!(
                "{}: function pointer ABI argument {index} uses unsupported {mode} pass mode",
                tcx.symbol_name(caller).name
            ));
        }
    }
    if let Some(mode) = unsupported_backend_pass_mode(&abi.return_value.mode) {
        return Err(format!(
            "{}: function pointer ABI return uses unsupported {mode} pass mode",
            tcx.symbol_name(caller).name
        ));
    }
    Ok(())
}

fn lower_fn_abi_plan<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
) -> Result<FnAbiPlan, String> {
    let fn_abi = tcx
        .fn_abi_of_instance(
            ty::TypingEnv::fully_monomorphized().as_query_input((instance, ty::List::empty())),
        )
        .map_err(|err| {
            format!(
                "{}: failed to compute rustc FnAbi: {err:?}",
                tcx.symbol_name(instance).name
            )
        })?;
    Ok(lower_rustc_fn_abi(fn_abi))
}

fn lower_rustc_fn_abi<'tcx>(fn_abi: &FnAbi<'tcx, Ty<'tcx>>) -> FnAbiPlan {
    FnAbiPlan {
        convention: lower_calling_convention(fn_abi.conv),
        variadic: fn_abi.c_variadic,
        fixed_count: fn_abi.fixed_count,
        can_unwind: fn_abi.can_unwind,
        arguments: fn_abi.args.iter().map(lower_abi_value).collect(),
        return_value: lower_abi_value(&fn_abi.ret),
    }
}

fn lower_calling_convention(conv: CanonAbi) -> CallingConventionPlan {
    match conv {
        CanonAbi::C => CallingConventionPlan::C,
        CanonAbi::Rust => CallingConventionPlan::Rust,
        CanonAbi::RustCold => CallingConventionPlan::RustCold,
        CanonAbi::RustPreserveNone => CallingConventionPlan::RustPreserveNone,
        CanonAbi::RustTail => CallingConventionPlan::RustTail,
        other => CallingConventionPlan::Other(format!("{other:?}")),
    }
}

fn lower_abi_value<'tcx>(value: &rustc_target::callconv::ArgAbi<'tcx, Ty<'tcx>>) -> AbiValuePlan {
    AbiValuePlan {
        size: value.layout.size.bytes(),
        align: value.layout.align.abi.bytes(),
        mode: lower_pass_mode(&value.mode),
    }
}

fn lower_pass_mode(mode: &PassMode) -> AbiPassModePlan {
    match mode {
        PassMode::Ignore => AbiPassModePlan::Ignore,
        PassMode::Direct(_) => AbiPassModePlan::Direct,
        PassMode::Pair(_, _) => AbiPassModePlan::Pair,
        PassMode::Cast { pad_i32, cast } => AbiPassModePlan::Cast {
            pad_i32: *pad_i32,
            prefix: cast
                .prefix
                .iter()
                .copied()
                .map(lower_abi_register)
                .collect(),
            rest_offset: cast.rest_offset.map(|offset| offset.bytes()),
            rest: lower_abi_uniform(cast.rest),
        },
        PassMode::Indirect {
            meta_attrs,
            on_stack,
            ..
        } => AbiPassModePlan::Indirect {
            has_metadata: meta_attrs.is_some(),
            on_stack: *on_stack,
        },
    }
}

fn lower_abi_uniform(uniform: rustc_target::callconv::Uniform) -> AbiUniformPlan {
    AbiUniformPlan {
        unit: lower_abi_register(uniform.unit),
        total_bytes: uniform.total.bytes(),
        consecutive: uniform.is_consecutive,
    }
}

fn lower_abi_register(reg: Reg) -> AbiRegisterPlan {
    AbiRegisterPlan {
        kind: match reg.kind {
            RegKind::Integer => AbiRegisterKind::Integer,
            RegKind::Float => AbiRegisterKind::Float,
            RegKind::Vector { .. } => AbiRegisterKind::Vector,
        },
        bits: reg.size.bits(),
    }
}

fn lower_type_layout_recipe<'tcx>(
    tcx: TyCtxt<'tcx>,
    ty: Ty<'tcx>,
) -> Result<TypeLayoutRecipe, String> {
    let layout = tcx
        .layout_of(ty::TypingEnv::fully_monomorphized().as_query_input(ty))
        .map_err(|err| format!("failed to compute layout for `{:?}`: {err:?}", ty.kind()))?;
    Ok(TypeLayoutRecipe {
        ty: layout_type_name(ty),
        size: layout.size.bytes(),
        align: layout.align.abi.bytes(),
        uninhabited: layout.uninhabited,
        fields: lower_field_layout_recipe(&layout.fields)?,
        variants: lower_variant_recipe(&layout.variants, layout.align.abi.bytes())?,
        largest_niche: layout.largest_niche.map(lower_niche_recipe),
        scalar_valid_ranges: lower_backend_repr_scalars(&layout.backend_repr),
    })
}

fn lower_field_layout_recipe(
    fields: &FieldsShape<LayoutFieldIdx>,
) -> Result<FieldLayoutRecipe, String> {
    Ok(match fields {
        FieldsShape::Primitive => FieldLayoutRecipe::Primitive,
        FieldsShape::Union(count) => FieldLayoutRecipe::Union {
            count: u32::try_from(count.get())
                .map_err(|_| "union field count exceeds u32".to_string())?,
        },
        FieldsShape::Array { stride, count } => FieldLayoutRecipe::Array {
            stride: stride.bytes(),
            count: *count,
        },
        FieldsShape::Arbitrary {
            offsets,
            in_memory_order,
        } => FieldLayoutRecipe::Arbitrary {
            offsets: offsets.iter().map(|offset| offset.bytes()).collect(),
            memory_order: in_memory_order
                .iter()
                .map(|field| {
                    u32::try_from(field.index()).map_err(|_| "field index exceeds u32".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?,
        },
    })
}

fn lower_variant_recipe(
    variants: &Variants<LayoutFieldIdx, LayoutVariantIdx>,
    parent_align: u64,
) -> Result<VariantRecipe, String> {
    Ok(match variants {
        Variants::Empty => VariantRecipe::Empty,
        Variants::Single { index } => VariantRecipe::Single {
            index: u32::try_from(index.index())
                .map_err(|_| "single variant index exceeds u32".to_string())?,
        },
        Variants::Multiple {
            tag,
            tag_encoding,
            tag_field,
            variants,
        } => VariantRecipe::Multiple {
            tag: lower_scalar_layout_recipe_for_protocol(*tag),
            tag_field: u32::try_from(tag_field.index())
                .map_err(|_| "tag field index exceeds u32".to_string())?,
            tag_encoding: lower_tag_encoding_recipe(tag_encoding)?,
            variants: variants
                .iter()
                .enumerate()
                .map(|(index, variant)| {
                    let offsets = variant
                        .field_offsets
                        .iter()
                        .map(|offset| offset.bytes())
                        .collect::<Vec<_>>();
                    let memory_order = (0..offsets.len())
                        .map(|field| {
                            u32::try_from(field)
                                .map_err(|_| "variant field index exceeds u32".to_string())
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(VariantLayoutRecipe {
                        index: u32::try_from(index)
                            .map_err(|_| "variant index exceeds u32".to_string())?,
                        size: variant.size.bytes(),
                        align: parent_align,
                        fields: FieldLayoutRecipe::Arbitrary {
                            offsets,
                            memory_order,
                        },
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
        },
    })
}

fn lower_tag_encoding_recipe(
    encoding: &TagEncoding<LayoutVariantIdx>,
) -> Result<TagEncodingRecipe, String> {
    Ok(match encoding {
        TagEncoding::Direct => TagEncodingRecipe::Direct,
        TagEncoding::Niche {
            untagged_variant,
            niche_variants,
            niche_start,
        } => TagEncodingRecipe::Niche {
            untagged_variant: u32::try_from(untagged_variant.index())
                .map_err(|_| "untagged variant index exceeds u32".to_string())?,
            niche_start: *niche_start,
            niche_variants_start: u32::try_from(niche_variants.start.index())
                .map_err(|_| "niche start variant index exceeds u32".to_string())?,
            niche_variants_end: u32::try_from(niche_variants.last.index())
                .map_err(|_| "niche end variant index exceeds u32".to_string())?,
        },
    })
}

fn lower_backend_repr_scalars(repr: &BackendRepr) -> Vec<ScalarLayoutRecipe> {
    match repr {
        BackendRepr::Scalar(scalar) => vec![lower_scalar_layout_recipe_for_protocol(*scalar)],
        BackendRepr::ScalarPair { a, b, .. } => vec![
            lower_scalar_layout_recipe_for_protocol(*a),
            lower_scalar_layout_recipe_for_protocol(*b),
        ],
        BackendRepr::SimdVector { element, .. }
        | BackendRepr::SimdScalableVector { element, .. } => {
            vec![lower_scalar_layout_recipe_for_protocol(*element)]
        }
        BackendRepr::Memory { .. } => Vec::new(),
    }
}

fn lower_scalar_layout_recipe_for_protocol(scalar: RustcScalar) -> ScalarLayoutRecipe {
    match scalar {
        RustcScalar::Initialized { value, valid_range } => ScalarLayoutRecipe {
            primitive: lower_primitive_name(value),
            valid_range: Some(lower_valid_range_recipe(valid_range)),
        },
        RustcScalar::Union { value } => ScalarLayoutRecipe {
            primitive: lower_primitive_name(value),
            valid_range: None,
        },
    }
}

fn lower_niche_recipe(niche: Niche) -> NicheRecipe {
    NicheRecipe {
        offset: niche.offset.bytes(),
        primitive: lower_primitive_name(niche.value),
        valid_range: lower_valid_range_recipe(niche.valid_range),
    }
}

fn lower_valid_range_recipe(range: rustc_abi::WrappingRange) -> ValidRangeRecipe {
    ValidRangeRecipe {
        start: range.start,
        end: range.end,
    }
}

fn lower_primitive_name(primitive: Primitive) -> String {
    format!("{primitive:?}")
}

fn layout_type_name(ty: Ty<'_>) -> String {
    format!("{:?}", ty.kind())
}

fn lower_assignment<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
    state: &mut LoweringState,
    place: Place<'tcx>,
    rvalue: &Rvalue<'tcx>,
) -> Result<Vec<Operation>, String> {
    let place_ty = monomorphize_ty(tcx, instance, place.ty(&mir.local_decls, tcx).ty);
    if is_unit_ty(place_ty) || is_empty_struct_ty(place_ty) {
        return Ok(Vec::new());
    }

    if let Rvalue::BinaryOp(
        op @ (BinOp::AddWithOverflow | BinOp::SubWithOverflow | BinOp::MulWithOverflow),
        operands,
    ) = rvalue
    {
        return lower_checked_binary_op(
            tcx,
            instance,
            mir,
            state,
            place,
            *op,
            &operands.0,
            &operands.1,
        );
    }

    if let Some(slot) = stack_slot_for_place(state, place) {
        let dst = lower_destination(state, place)?;
        let mut operations = vec![lower_rvalue(tcx, instance, mir, state, dst, rvalue)?];
        operations.push(Operation::Store {
            ptr: ValueRef::Local(slot.ptr),
            offset: 0,
            value: ValueRef::Local(dst),
            ty: slot.ty,
            align: slot.align,
        });
        return Ok(operations);
    }

    if let Some(memory) = lower_memory_place(tcx, instance, mir, place)? {
        let (ty, _size, align) = scalar_memory_layout(tcx, instance, place_ty)?;
        let temp = state.allocate_temp(ty);
        let mut operations = vec![lower_rvalue(tcx, instance, mir, state, temp, rvalue)?];
        operations.push(Operation::Store {
            ptr: memory.ptr,
            offset: memory.offset,
            value: ValueRef::Local(temp),
            ty,
            align,
        });
        return Ok(operations);
    }

    if scalar_aggregate_field_types(tcx, place_ty).is_some() {
        return lower_aggregate_assignment(tcx, instance, mir, state, place, rvalue);
    }

    let dst = lower_destination(state, place)?;
    Ok(vec![lower_rvalue(tcx, instance, mir, state, dst, rvalue)?])
}

fn lower_aggregate_assignment<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
    state: &LoweringState,
    place: Place<'tcx>,
    rvalue: &Rvalue<'tcx>,
) -> Result<Vec<Operation>, String> {
    if !place.projection.is_empty() {
        return Err(format!(
            "{}: aggregate destination must be an unprojected local",
            tcx.symbol_name(instance).name
        ));
    }
    let place_ty = monomorphize_ty(tcx, instance, place.ty(&mir.local_decls, tcx).ty);
    let field_types = scalar_aggregate_field_types(tcx, place_ty).ok_or_else(|| {
        format!(
            "{}: aggregate destination has unsupported type `{}`",
            tcx.symbol_name(instance).name,
            place_ty
        )
    })?;

    match rvalue {
        Rvalue::Aggregate(kind, operands) => {
            match **kind {
                AggregateKind::Tuple => {}
                AggregateKind::Adt(def_id, ..) if tcx.adt_def(def_id).is_struct() => {}
                _ => {
                    return Err(format!(
                        "{}: only tuple and struct aggregate rvalues are currently supported",
                        tcx.symbol_name(instance).name
                    ));
                }
            }
            lower_aggregate_operands(
                tcx,
                instance,
                mir,
                state,
                place,
                &field_types,
                operands.iter(),
            )
        }
        Rvalue::Use(Operand::Copy(src_place) | Operand::Move(src_place), _) => {
            lower_aggregate_copy(tcx, instance, mir, state, place, &field_types, *src_place)
        }
        other => Err(format!(
            "{}: aggregate assignment only supports aggregate construction and local copy/move, got `{other:?}`",
            tcx.symbol_name(instance).name
        )),
    }
}

fn lower_aggregate_operands<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
    state: &LoweringState,
    place: Place<'tcx>,
    field_types: &[ScalarType],
    operands: impl ExactSizeIterator<Item = &'a Operand<'tcx>>,
) -> Result<Vec<Operation>, String>
where
    'tcx: 'a,
{
    let operand_count = operands.len();
    if operand_count != field_types.len() {
        return Err(format!(
            "{}: aggregate has {} operands for {} fields",
            tcx.symbol_name(instance).name,
            operand_count,
            field_types.len()
        ));
    }
    operands
        .enumerate()
        .map(|(field, operand)| {
            let dst = state.tuple_field(place.local, field).ok_or_else(|| {
                format!(
                    "{}: aggregate field {field} is missing a synthetic local",
                    tcx.symbol_name(instance).name
                )
            })?;
            let operand_ty = monomorphize_ty(tcx, instance, operand.ty(&mir.local_decls, tcx));
            let operand_scalar = scalar_type_for_ty(operand_ty).ok_or_else(|| {
                format!(
                    "{}: aggregate field {field} has unsupported operand type `{}`",
                    tcx.symbol_name(instance).name,
                    operand_ty
                )
            })?;
            if operand_scalar != field_types[field] {
                return Err(format!(
                    "{}: aggregate field {field} type mismatch",
                    tcx.symbol_name(instance).name
                ));
            }
            Ok(Operation::Copy {
                dst,
                src: lower_operand(tcx, instance, mir, state, operand)?,
            })
        })
        .collect()
}

fn lower_aggregate_copy<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
    state: &LoweringState,
    dst_place: Place<'tcx>,
    dst_field_types: &[ScalarType],
    src_place: Place<'tcx>,
) -> Result<Vec<Operation>, String> {
    if !src_place.projection.is_empty() {
        return Err(format!(
            "{}: aggregate copy source must be an unprojected local",
            tcx.symbol_name(instance).name
        ));
    }
    let src_ty = monomorphize_ty(tcx, instance, src_place.ty(&mir.local_decls, tcx).ty);
    let src_field_types = scalar_aggregate_field_types(tcx, src_ty).ok_or_else(|| {
        format!(
            "{}: aggregate copy source has unsupported type `{}`",
            tcx.symbol_name(instance).name,
            src_ty
        )
    })?;
    if src_field_types != dst_field_types {
        return Err(format!(
            "{}: aggregate copy field type mismatch",
            tcx.symbol_name(instance).name
        ));
    }

    (0..dst_field_types.len())
        .map(|field| {
            let dst = state.tuple_field(dst_place.local, field).ok_or_else(|| {
                format!(
                    "{}: aggregate destination field {field} is missing a synthetic local",
                    tcx.symbol_name(instance).name
                )
            })?;
            let src = state.tuple_field(src_place.local, field).ok_or_else(|| {
                format!(
                    "{}: aggregate source field {field} is missing a synthetic local",
                    tcx.symbol_name(instance).name
                )
            })?;
            Ok(Operation::Copy {
                dst,
                src: ValueRef::Local(src),
            })
        })
        .collect()
}

fn lower_rvalue<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
    state: &LoweringState,
    dst: u32,
    rvalue: &Rvalue<'tcx>,
) -> Result<Operation, String> {
    match rvalue {
        Rvalue::Use(Operand::Copy(place) | Operand::Move(place), _) => {
            if let Some(slot) = stack_slot_for_place(state, *place) {
                return Ok(Operation::Load {
                    dst,
                    ptr: ValueRef::Local(slot.ptr),
                    offset: 0,
                    ty: slot.ty,
                    align: slot.align,
                });
            }
            if let Some(memory) = lower_memory_place(tcx, instance, mir, *place)? {
                let place_ty = monomorphize_ty(tcx, instance, place.ty(&mir.local_decls, tcx).ty);
                let (ty, _size, align) = scalar_memory_layout(tcx, instance, place_ty)?;
                return Ok(Operation::Load {
                    dst,
                    ptr: memory.ptr,
                    offset: memory.offset,
                    ty,
                    align,
                });
            }
            Ok(Operation::Copy {
                dst,
                src: lower_operand(tcx, instance, mir, state, &Operand::Copy(*place))?,
            })
        }
        Rvalue::Use(operand, _) => Ok(Operation::Copy {
            dst,
            src: lower_operand(tcx, instance, mir, state, operand)?,
        }),
        Rvalue::BinaryOp(op, operands) => {
            if let Some(op) = lower_binary_op(tcx, instance, mir, *op, &operands.0)? {
                Ok(Operation::Binary {
                    dst,
                    op,
                    lhs: lower_operand(tcx, instance, mir, state, &operands.0)?,
                    rhs: lower_operand(tcx, instance, mir, state, &operands.1)?,
                })
            } else if let Some(op) = lower_compare_op(tcx, instance, mir, *op, &operands.0)? {
                Ok(Operation::Compare {
                    dst,
                    op,
                    lhs: lower_operand(tcx, instance, mir, state, &operands.0)?,
                    rhs: lower_operand(tcx, instance, mir, state, &operands.1)?,
                })
            } else {
                Err(format!(
                    "{}: unsupported binary operation `{op:?}`",
                    tcx.symbol_name(instance).name
                ))
            }
        }
        Rvalue::UnaryOp(op, operand) => {
            lower_unary_op(tcx, instance, mir, state, dst, *op, operand)
        }
        Rvalue::Cast(kind, operand, ty) => {
            let src_ty = monomorphize_ty(tcx, instance, operand.ty(&mir.local_decls, tcx));
            let dst_ty = monomorphize_ty(tcx, instance, *ty);
            if *kind == rustc_middle::mir::CastKind::PtrToPtr
                && scalar_type_for_ty(src_ty) == Some(ScalarType::Ptr)
                && scalar_type_for_ty(dst_ty) == Some(ScalarType::Ptr)
            {
                return Ok(Operation::Copy {
                    dst,
                    src: lower_operand(tcx, instance, mir, state, operand)?,
                });
            }
            if *kind != rustc_middle::mir::CastKind::IntToInt {
                return Err(format!(
                    "{}: unsupported cast kind `{kind:?}`",
                    tcx.symbol_name(instance).name
                ));
            }
            let dst_scalar = scalar_type_for_ty(dst_ty).ok_or_else(|| {
                format!(
                    "{}: cast destination has unsupported type `{}`",
                    tcx.symbol_name(instance).name,
                    dst_ty
                )
            })?;
            Ok(Operation::Cast {
                dst,
                op: lower_int_cast(tcx, instance, src_ty, dst_ty)?,
                src: lower_operand(tcx, instance, mir, state, operand)?,
                ty: dst_scalar,
            })
        }
        Rvalue::Ref(_, _, place) | Rvalue::RawPtr(_, place) => {
            let slot = stack_slot_for_place(state, *place).ok_or_else(|| {
                format!(
                    "{}: taking the address of `{place:?}` is not yet supported",
                    tcx.symbol_name(instance).name
                )
            })?;
            Ok(Operation::Copy {
                dst,
                src: ValueRef::Local(slot.ptr),
            })
        }
        other => Err(format!(
            "{}: unsupported rvalue `{other:?}`",
            tcx.symbol_name(instance).name
        )),
    }
}

fn lower_checked_binary_op<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
    state: &mut LoweringState,
    place: Place<'tcx>,
    op: BinOp,
    lhs: &Operand<'tcx>,
    rhs: &Operand<'tcx>,
) -> Result<Vec<Operation>, String> {
    if !place.projection.is_empty() {
        return Err(format!(
            "{}: checked arithmetic destination must be an unprojected tuple local",
            tcx.symbol_name(instance).name
        ));
    }
    let result_dst = state.tuple_field(place.local, 0).ok_or_else(|| {
        format!(
            "{}: checked arithmetic destination is not a supported tuple local",
            tcx.symbol_name(instance).name
        )
    })?;
    let overflow_dst = state.tuple_field(place.local, 1).ok_or_else(|| {
        format!(
            "{}: checked arithmetic overflow field is missing",
            tcx.symbol_name(instance).name
        )
    })?;

    let lhs_ty = monomorphize_ty(tcx, instance, lhs.ty(&mir.local_decls, tcx));
    let lhs_scalar = scalar_type_for_ty(lhs_ty).ok_or_else(|| {
        format!(
            "{}: checked arithmetic source has unsupported type `{}`",
            tcx.symbol_name(instance).name,
            lhs_ty
        )
    })?;
    let signed = is_signed_integer(lhs_ty).ok_or_else(|| {
        format!(
            "{}: checked arithmetic requires integer operands, got `{}`",
            tcx.symbol_name(instance).name,
            lhs_ty
        )
    })?;
    let lhs_value = lower_operand(tcx, instance, mir, state, lhs)?;
    let rhs_value = lower_operand(tcx, instance, mir, state, rhs)?;
    let mut operations = vec![Operation::Binary {
        dst: result_dst,
        op: match op {
            BinOp::AddWithOverflow => BinaryOp::Add,
            BinOp::SubWithOverflow => BinaryOp::Sub,
            BinOp::MulWithOverflow => BinaryOp::Mul,
            _ => unreachable!("not a checked arithmetic operation"),
        },
        lhs: lhs_value.clone(),
        rhs: rhs_value.clone(),
    }];

    if op == BinOp::MulWithOverflow {
        append_mul_overflow_check(
            tcx,
            instance,
            state,
            &mut operations,
            lhs_scalar,
            signed,
            lhs_value,
            rhs_value,
            result_dst,
            overflow_dst,
        )?;
    } else if signed {
        append_signed_overflow_check(
            state,
            &mut operations,
            op,
            lhs_scalar,
            lhs_value,
            rhs_value,
            result_dst,
            overflow_dst,
        );
    } else {
        operations.push(match op {
            BinOp::AddWithOverflow => Operation::Compare {
                dst: overflow_dst,
                op: CompareOp::Ult,
                lhs: ValueRef::Local(result_dst),
                rhs: lhs_value,
            },
            BinOp::SubWithOverflow => Operation::Compare {
                dst: overflow_dst,
                op: CompareOp::Ult,
                lhs: lhs_value,
                rhs: rhs_value,
            },
            _ => unreachable!("checked multiplication handled above"),
        });
    }
    Ok(operations)
}

fn append_mul_overflow_check<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    state: &mut LoweringState,
    operations: &mut Vec<Operation>,
    ty: ScalarType,
    signed: bool,
    lhs: ValueRef,
    rhs: ValueRef,
    result_dst: u32,
    overflow_dst: u32,
) -> Result<(), String> {
    let wide_ty = match (ty, signed) {
        (ScalarType::I8 | ScalarType::I16 | ScalarType::I32, true) => ScalarType::I64,
        (ScalarType::U8 | ScalarType::U16 | ScalarType::U32, false) => ScalarType::U64,
        (ScalarType::I64, true) => {
            append_signed_i64_mul_overflow_check(state, operations, lhs, rhs, overflow_dst);
            return Ok(());
        }
        (ScalarType::U64, false) => {
            append_unsigned_u64_mul_overflow_check(state, operations, lhs, rhs, overflow_dst);
            return Ok(());
        }
        _ => {
            return Err(format!(
                "{}: checked multiplication for {:?} is not yet supported",
                tcx.symbol_name(instance).name,
                ty
            ));
        }
    };
    let widen_op = if signed { CastOp::Sext } else { CastOp::Zext };
    let lhs_wide = state.allocate_temp(wide_ty);
    let rhs_wide = state.allocate_temp(wide_ty);
    let product_wide = state.allocate_temp(wide_ty);
    let result_wide = state.allocate_temp(wide_ty);
    operations.push(Operation::Cast {
        dst: lhs_wide,
        op: widen_op,
        src: lhs,
        ty: wide_ty,
    });
    operations.push(Operation::Cast {
        dst: rhs_wide,
        op: widen_op,
        src: rhs,
        ty: wide_ty,
    });
    operations.push(Operation::Binary {
        dst: product_wide,
        op: BinaryOp::Mul,
        lhs: ValueRef::Local(lhs_wide),
        rhs: ValueRef::Local(rhs_wide),
    });
    operations.push(Operation::Cast {
        dst: result_wide,
        op: widen_op,
        src: ValueRef::Local(result_dst),
        ty: wide_ty,
    });
    operations.push(Operation::Compare {
        dst: overflow_dst,
        op: CompareOp::Ne,
        lhs: ValueRef::Local(product_wide),
        rhs: ValueRef::Local(result_wide),
    });
    Ok(())
}

fn append_unsigned_u64_mul_overflow_check(
    state: &mut LoweringState,
    operations: &mut Vec<Operation>,
    lhs: ValueRef,
    rhs: ValueRef,
    overflow_dst: u32,
) {
    let mask32 = ValueRef::Integer {
        ty: ScalarType::U64,
        bits: u32::MAX.into(),
    };
    let shift32 = ValueRef::Integer {
        ty: ScalarType::U64,
        bits: 32,
    };
    let zero = ValueRef::Integer {
        ty: ScalarType::U64,
        bits: 0,
    };

    let lhs_low = state.allocate_temp(ScalarType::U64);
    let rhs_low = state.allocate_temp(ScalarType::U64);
    let lhs_high = state.allocate_temp(ScalarType::U64);
    let rhs_high = state.allocate_temp(ScalarType::U64);
    let low_product = state.allocate_temp(ScalarType::U64);
    let cross_left = state.allocate_temp(ScalarType::U64);
    let cross_right = state.allocate_temp(ScalarType::U64);
    let high_product = state.allocate_temp(ScalarType::U64);
    let low_carry = state.allocate_temp(ScalarType::U64);
    let cross_sum1 = state.allocate_temp(ScalarType::U64);
    let cross_carry1 = state.allocate_temp(ScalarType::I1);
    let cross_sum2 = state.allocate_temp(ScalarType::U64);
    let cross_carry2 = state.allocate_temp(ScalarType::I1);
    let cross_high = state.allocate_temp(ScalarType::U64);
    let high_nonzero = state.allocate_temp(ScalarType::I1);
    let cross_nonzero = state.allocate_temp(ScalarType::I1);
    let high_or_carry = state.allocate_temp(ScalarType::I1);
    let cross_or_carry = state.allocate_temp(ScalarType::I1);

    operations.push(Operation::Binary {
        dst: lhs_low,
        op: BinaryOp::BitAnd,
        lhs: lhs.clone(),
        rhs: mask32.clone(),
    });
    operations.push(Operation::Binary {
        dst: rhs_low,
        op: BinaryOp::BitAnd,
        lhs: rhs.clone(),
        rhs: mask32,
    });
    operations.push(Operation::Binary {
        dst: lhs_high,
        op: BinaryOp::LShr,
        lhs: lhs,
        rhs: shift32.clone(),
    });
    operations.push(Operation::Binary {
        dst: rhs_high,
        op: BinaryOp::LShr,
        lhs: rhs,
        rhs: shift32.clone(),
    });
    operations.push(Operation::Binary {
        dst: low_product,
        op: BinaryOp::Mul,
        lhs: ValueRef::Local(lhs_low),
        rhs: ValueRef::Local(rhs_low),
    });
    operations.push(Operation::Binary {
        dst: cross_left,
        op: BinaryOp::Mul,
        lhs: ValueRef::Local(lhs_high),
        rhs: ValueRef::Local(rhs_low),
    });
    operations.push(Operation::Binary {
        dst: cross_right,
        op: BinaryOp::Mul,
        lhs: ValueRef::Local(lhs_low),
        rhs: ValueRef::Local(rhs_high),
    });
    operations.push(Operation::Binary {
        dst: high_product,
        op: BinaryOp::Mul,
        lhs: ValueRef::Local(lhs_high),
        rhs: ValueRef::Local(rhs_high),
    });
    operations.push(Operation::Binary {
        dst: low_carry,
        op: BinaryOp::LShr,
        lhs: ValueRef::Local(low_product),
        rhs: shift32.clone(),
    });
    operations.push(Operation::Binary {
        dst: cross_sum1,
        op: BinaryOp::Add,
        lhs: ValueRef::Local(cross_left),
        rhs: ValueRef::Local(low_carry),
    });
    operations.push(Operation::Compare {
        dst: cross_carry1,
        op: CompareOp::Ult,
        lhs: ValueRef::Local(cross_sum1),
        rhs: ValueRef::Local(cross_left),
    });
    operations.push(Operation::Binary {
        dst: cross_sum2,
        op: BinaryOp::Add,
        lhs: ValueRef::Local(cross_sum1),
        rhs: ValueRef::Local(cross_right),
    });
    operations.push(Operation::Compare {
        dst: cross_carry2,
        op: CompareOp::Ult,
        lhs: ValueRef::Local(cross_sum2),
        rhs: ValueRef::Local(cross_sum1),
    });
    operations.push(Operation::Binary {
        dst: cross_high,
        op: BinaryOp::LShr,
        lhs: ValueRef::Local(cross_sum2),
        rhs: shift32,
    });
    operations.push(Operation::Compare {
        dst: high_nonzero,
        op: CompareOp::Ne,
        lhs: ValueRef::Local(high_product),
        rhs: zero.clone(),
    });
    operations.push(Operation::Compare {
        dst: cross_nonzero,
        op: CompareOp::Ne,
        lhs: ValueRef::Local(cross_high),
        rhs: zero,
    });
    operations.push(Operation::Binary {
        dst: high_or_carry,
        op: BinaryOp::BitOr,
        lhs: ValueRef::Local(high_nonzero),
        rhs: ValueRef::Local(cross_carry1),
    });
    operations.push(Operation::Binary {
        dst: cross_or_carry,
        op: BinaryOp::BitOr,
        lhs: ValueRef::Local(cross_nonzero),
        rhs: ValueRef::Local(cross_carry2),
    });
    operations.push(Operation::Binary {
        dst: overflow_dst,
        op: BinaryOp::BitOr,
        lhs: ValueRef::Local(high_or_carry),
        rhs: ValueRef::Local(cross_or_carry),
    });
}

fn append_signed_i64_mul_overflow_check(
    state: &mut LoweringState,
    operations: &mut Vec<Operation>,
    lhs: ValueRef,
    rhs: ValueRef,
    overflow_dst: u32,
) {
    let zero_i64 = ValueRef::Integer {
        ty: ScalarType::I64,
        bits: 0,
    };
    let sign_shift = ValueRef::Integer {
        ty: ScalarType::I64,
        bits: 63,
    };
    let one_i1 = ValueRef::Integer {
        ty: ScalarType::I1,
        bits: 1,
    };

    let lhs_negative = state.allocate_temp(ScalarType::I1);
    let rhs_negative = state.allocate_temp(ScalarType::I1);
    let result_negative = state.allocate_temp(ScalarType::I1);
    let result_positive = state.allocate_temp(ScalarType::I1);
    let lhs_mask_i64 = state.allocate_temp(ScalarType::I64);
    let rhs_mask_i64 = state.allocate_temp(ScalarType::I64);
    let lhs_bits = state.allocate_temp(ScalarType::U64);
    let rhs_bits = state.allocate_temp(ScalarType::U64);
    let lhs_mask = state.allocate_temp(ScalarType::U64);
    let rhs_mask = state.allocate_temp(ScalarType::U64);
    let lhs_xored = state.allocate_temp(ScalarType::U64);
    let rhs_xored = state.allocate_temp(ScalarType::U64);
    let lhs_abs = state.allocate_temp(ScalarType::U64);
    let rhs_abs = state.allocate_temp(ScalarType::U64);
    let magnitude_product = state.allocate_temp(ScalarType::U64);
    let unsigned_overflow = state.allocate_temp(ScalarType::I1);
    let positive_limit_overflow = state.allocate_temp(ScalarType::I1);
    let negative_limit_overflow = state.allocate_temp(ScalarType::I1);
    let positive_overflow = state.allocate_temp(ScalarType::I1);
    let negative_overflow = state.allocate_temp(ScalarType::I1);
    let signed_limit_overflow = state.allocate_temp(ScalarType::I1);

    operations.push(Operation::Compare {
        dst: lhs_negative,
        op: CompareOp::Slt,
        lhs: lhs.clone(),
        rhs: zero_i64.clone(),
    });
    operations.push(Operation::Compare {
        dst: rhs_negative,
        op: CompareOp::Slt,
        lhs: rhs.clone(),
        rhs: zero_i64.clone(),
    });
    operations.push(Operation::Binary {
        dst: result_negative,
        op: BinaryOp::BitXor,
        lhs: ValueRef::Local(lhs_negative),
        rhs: ValueRef::Local(rhs_negative),
    });
    operations.push(Operation::Binary {
        dst: result_positive,
        op: BinaryOp::BitXor,
        lhs: ValueRef::Local(result_negative),
        rhs: one_i1,
    });
    operations.push(Operation::Binary {
        dst: lhs_mask_i64,
        op: BinaryOp::AShr,
        lhs: lhs.clone(),
        rhs: sign_shift.clone(),
    });
    operations.push(Operation::Binary {
        dst: rhs_mask_i64,
        op: BinaryOp::AShr,
        lhs: rhs.clone(),
        rhs: sign_shift,
    });
    operations.push(Operation::Cast {
        dst: lhs_bits,
        op: CastOp::Bitcast,
        src: lhs,
        ty: ScalarType::U64,
    });
    operations.push(Operation::Cast {
        dst: rhs_bits,
        op: CastOp::Bitcast,
        src: rhs,
        ty: ScalarType::U64,
    });
    operations.push(Operation::Cast {
        dst: lhs_mask,
        op: CastOp::Bitcast,
        src: ValueRef::Local(lhs_mask_i64),
        ty: ScalarType::U64,
    });
    operations.push(Operation::Cast {
        dst: rhs_mask,
        op: CastOp::Bitcast,
        src: ValueRef::Local(rhs_mask_i64),
        ty: ScalarType::U64,
    });
    operations.push(Operation::Binary {
        dst: lhs_xored,
        op: BinaryOp::BitXor,
        lhs: ValueRef::Local(lhs_bits),
        rhs: ValueRef::Local(lhs_mask),
    });
    operations.push(Operation::Binary {
        dst: rhs_xored,
        op: BinaryOp::BitXor,
        lhs: ValueRef::Local(rhs_bits),
        rhs: ValueRef::Local(rhs_mask),
    });
    operations.push(Operation::Binary {
        dst: lhs_abs,
        op: BinaryOp::Sub,
        lhs: ValueRef::Local(lhs_xored),
        rhs: ValueRef::Local(lhs_mask),
    });
    operations.push(Operation::Binary {
        dst: rhs_abs,
        op: BinaryOp::Sub,
        lhs: ValueRef::Local(rhs_xored),
        rhs: ValueRef::Local(rhs_mask),
    });
    operations.push(Operation::Binary {
        dst: magnitude_product,
        op: BinaryOp::Mul,
        lhs: ValueRef::Local(lhs_abs),
        rhs: ValueRef::Local(rhs_abs),
    });
    append_unsigned_u64_mul_overflow_check(
        state,
        operations,
        ValueRef::Local(lhs_abs),
        ValueRef::Local(rhs_abs),
        unsigned_overflow,
    );
    operations.push(Operation::Compare {
        dst: positive_limit_overflow,
        op: CompareOp::Ugt,
        lhs: ValueRef::Local(magnitude_product),
        rhs: ValueRef::Integer {
            ty: ScalarType::U64,
            bits: i64::MAX as u64,
        },
    });
    operations.push(Operation::Compare {
        dst: negative_limit_overflow,
        op: CompareOp::Ugt,
        lhs: ValueRef::Local(magnitude_product),
        rhs: ValueRef::Integer {
            ty: ScalarType::U64,
            bits: 1_u64 << 63,
        },
    });
    operations.push(Operation::Binary {
        dst: positive_overflow,
        op: BinaryOp::BitAnd,
        lhs: ValueRef::Local(result_positive),
        rhs: ValueRef::Local(positive_limit_overflow),
    });
    operations.push(Operation::Binary {
        dst: negative_overflow,
        op: BinaryOp::BitAnd,
        lhs: ValueRef::Local(result_negative),
        rhs: ValueRef::Local(negative_limit_overflow),
    });
    operations.push(Operation::Binary {
        dst: signed_limit_overflow,
        op: BinaryOp::BitOr,
        lhs: ValueRef::Local(positive_overflow),
        rhs: ValueRef::Local(negative_overflow),
    });
    operations.push(Operation::Binary {
        dst: overflow_dst,
        op: BinaryOp::BitOr,
        lhs: ValueRef::Local(unsigned_overflow),
        rhs: ValueRef::Local(signed_limit_overflow),
    });
}

fn append_signed_overflow_check(
    state: &mut LoweringState,
    operations: &mut Vec<Operation>,
    op: BinOp,
    ty: ScalarType,
    lhs: ValueRef,
    rhs: ValueRef,
    result_dst: u32,
    overflow_dst: u32,
) {
    let lhs_negative = state.allocate_temp(ScalarType::I1);
    let rhs_negative = state.allocate_temp(ScalarType::I1);
    let result_negative = state.allocate_temp(ScalarType::I1);
    let sign_relation = state.allocate_temp(ScalarType::I1);
    let result_changed = state.allocate_temp(ScalarType::I1);
    let zero = ValueRef::Integer { ty, bits: 0 };
    operations.push(Operation::Compare {
        dst: lhs_negative,
        op: CompareOp::Slt,
        lhs: lhs.clone(),
        rhs: zero.clone(),
    });
    operations.push(Operation::Compare {
        dst: rhs_negative,
        op: CompareOp::Slt,
        lhs: rhs,
        rhs: zero,
    });
    operations.push(Operation::Compare {
        dst: result_negative,
        op: CompareOp::Slt,
        lhs: ValueRef::Local(result_dst),
        rhs: ValueRef::Integer { ty, bits: 0 },
    });
    operations.push(Operation::Compare {
        dst: sign_relation,
        op: match op {
            BinOp::AddWithOverflow => CompareOp::Eq,
            BinOp::SubWithOverflow => CompareOp::Ne,
            _ => unreachable!("checked multiplication not supported"),
        },
        lhs: ValueRef::Local(lhs_negative),
        rhs: ValueRef::Local(rhs_negative),
    });
    operations.push(Operation::Compare {
        dst: result_changed,
        op: CompareOp::Ne,
        lhs: ValueRef::Local(lhs_negative),
        rhs: ValueRef::Local(result_negative),
    });
    operations.push(Operation::Binary {
        dst: overflow_dst,
        op: BinaryOp::BitAnd,
        lhs: ValueRef::Local(sign_relation),
        rhs: ValueRef::Local(result_changed),
    });
}

fn lower_binary_op<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
    op: BinOp,
    lhs: &Operand<'tcx>,
) -> Result<Option<BinaryOp>, String> {
    let signed = || -> Result<bool, String> {
        let lhs_ty = monomorphize_ty(tcx, instance, lhs.ty(&mir.local_decls, tcx));
        is_signed_integer(lhs_ty).ok_or_else(|| {
            format!(
                "{}: `{op:?}` requires integer operands, got `{}`",
                tcx.symbol_name(instance).name,
                lhs_ty
            )
        })
    };
    Ok(Some(match op {
        BinOp::Add | BinOp::AddUnchecked => BinaryOp::Add,
        BinOp::Sub | BinOp::SubUnchecked => BinaryOp::Sub,
        BinOp::Mul | BinOp::MulUnchecked => BinaryOp::Mul,
        BinOp::BitXor => BinaryOp::BitXor,
        BinOp::BitAnd => BinaryOp::BitAnd,
        BinOp::BitOr => BinaryOp::BitOr,
        BinOp::Div => {
            if signed()? {
                BinaryOp::SDiv
            } else {
                BinaryOp::UDiv
            }
        }
        BinOp::Rem => {
            if signed()? {
                BinaryOp::SRem
            } else {
                BinaryOp::URem
            }
        }
        BinOp::Shl | BinOp::ShlUnchecked => BinaryOp::Shl,
        BinOp::Shr | BinOp::ShrUnchecked => {
            if signed()? {
                BinaryOp::AShr
            } else {
                BinaryOp::LShr
            }
        }
        _ => return Ok(None),
    }))
}

fn lower_unary_op<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
    state: &LoweringState,
    dst: u32,
    op: UnOp,
    operand: &Operand<'tcx>,
) -> Result<Operation, String> {
    let operand_ty = monomorphize_ty(tcx, instance, operand.ty(&mir.local_decls, tcx));
    let scalar_ty = scalar_type_for_ty(operand_ty).ok_or_else(|| {
        format!(
            "{}: unary operation source has unsupported type `{}`",
            tcx.symbol_name(instance).name,
            operand_ty
        )
    })?;
    match op {
        UnOp::Neg => Ok(Operation::Binary {
            dst,
            op: BinaryOp::Sub,
            lhs: ValueRef::Integer {
                ty: scalar_ty,
                bits: 0,
            },
            rhs: lower_operand(tcx, instance, mir, state, operand)?,
        }),
        UnOp::Not => Ok(Operation::Binary {
            dst,
            op: BinaryOp::BitXor,
            lhs: lower_operand(tcx, instance, mir, state, operand)?,
            rhs: ValueRef::Integer {
                ty: scalar_ty,
                bits: all_ones_for_scalar(scalar_ty),
            },
        }),
        other => Err(format!(
            "{}: unsupported unary operation `{other:?}`",
            tcx.symbol_name(instance).name
        )),
    }
}

fn lower_compare_op<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
    op: BinOp,
    lhs: &Operand<'tcx>,
) -> Result<Option<CompareOp>, String> {
    let signed = match op {
        BinOp::Eq => return Ok(Some(CompareOp::Eq)),
        BinOp::Ne => return Ok(Some(CompareOp::Ne)),
        BinOp::Lt | BinOp::Le | BinOp::Ge | BinOp::Gt => {
            let lhs_ty = monomorphize_ty(tcx, instance, lhs.ty(&mir.local_decls, tcx));
            is_signed_integer(lhs_ty).ok_or_else(|| {
                format!(
                    "{}: ordered compare requires integer operands, got `{}`",
                    tcx.symbol_name(instance).name,
                    lhs_ty
                )
            })?
        }
        _ => return Ok(None),
    };

    Ok(Some(match (op, signed) {
        (BinOp::Lt, true) => CompareOp::Slt,
        (BinOp::Le, true) => CompareOp::Sle,
        (BinOp::Ge, true) => CompareOp::Sge,
        (BinOp::Gt, true) => CompareOp::Sgt,
        (BinOp::Lt, false) => CompareOp::Ult,
        (BinOp::Le, false) => CompareOp::Ule,
        (BinOp::Ge, false) => CompareOp::Uge,
        (BinOp::Gt, false) => CompareOp::Ugt,
        _ => unreachable!("handled compare operation"),
    }))
}

fn lower_int_cast<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    src_ty: Ty<'tcx>,
    dst_ty: Ty<'tcx>,
) -> Result<CastOp, String> {
    let src_width = scalar_bit_width(src_ty).ok_or_else(|| {
        format!(
            "{}: cast source has unsupported type `{}`",
            tcx.symbol_name(instance).name,
            src_ty
        )
    })?;
    let dst_width = scalar_bit_width(dst_ty).ok_or_else(|| {
        format!(
            "{}: cast destination has unsupported type `{}`",
            tcx.symbol_name(instance).name,
            dst_ty
        )
    })?;
    if dst_width < src_width {
        Ok(CastOp::Trunc)
    } else if dst_width > src_width {
        if is_signed_integer(src_ty).unwrap_or(false) {
            Ok(CastOp::Sext)
        } else {
            Ok(CastOp::Zext)
        }
    } else {
        Ok(CastOp::Bitcast)
    }
}

fn lower_operand<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
    state: &LoweringState,
    operand: &Operand<'tcx>,
) -> Result<ValueRef, String> {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => lower_place_as_value(state, *place),
        Operand::Constant(constant) => lower_constant(tcx, instance, constant),
        other => Err(format!(
            "{}: unsupported operand `{other:?}`",
            tcx.symbol_name(instance).name
        )),
    }
    .and_then(|value| {
        if let ValueRef::Local(local) = value {
            let mir_local = rustc_middle::mir::Local::from_u32(local);
            let is_tuple_field = state
                .tuple_fields
                .values()
                .any(|synthetic| *synthetic == local);
            let is_temp = state
                .synthetic_locals
                .iter()
                .any(|synthetic| synthetic.id == local);
            if mir.local_decls.get(mir_local).is_none() && !is_tuple_field && !is_temp {
                return Err(format!(
                    "{}: local operand {} does not exist",
                    tcx.symbol_name(instance).name,
                    local
                ));
            }
            Ok(ValueRef::Local(local))
        } else {
            Ok(value)
        }
    })
}

fn lower_call_arguments<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
    state: &LoweringState,
    operand: &Operand<'tcx>,
) -> Result<Vec<ValueRef>, String> {
    let ty = monomorphize_ty(tcx, instance, operand.ty(&mir.local_decls, tcx));
    if let Some(field_types) = scalar_aggregate_field_types(tcx, ty) {
        let place = match operand {
            Operand::Copy(place) | Operand::Move(place) => *place,
            _ => {
                return Err(format!(
                    "{}: aggregate call argument must be a place operand",
                    tcx.symbol_name(instance).name
                ));
            }
        };
        if !place.projection.is_empty() {
            return Err(format!(
                "{}: aggregate call argument must be an unprojected local",
                tcx.symbol_name(instance).name
            ));
        }
        return field_types
            .iter()
            .enumerate()
            .map(|(field, _)| {
                let local = state.tuple_field(place.local, field).ok_or_else(|| {
                    format!(
                        "{}: aggregate call argument field {field} is missing a synthetic local",
                        tcx.symbol_name(instance).name
                    )
                })?;
                Ok(ValueRef::Local(local))
            })
            .collect();
    }
    lower_operand(tcx, instance, mir, state, operand).map(|value| vec![value])
}

fn lower_constant<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    constant: &ConstOperand<'tcx>,
) -> Result<ValueRef, String> {
    let const_ = instance.instantiate_mir_and_normalize_erasing_regions(
        tcx,
        ty::TypingEnv::fully_monomorphized(),
        ty::EarlyBinder::bind(tcx, constant.const_),
    );
    let ty = const_.ty();
    let scalar_ty = scalar_type_for_ty(ty).ok_or_else(|| {
        format!(
            "{}: constant has unsupported type `{}`",
            tcx.symbol_name(instance).name,
            ty
        )
    })?;
    let size = const_
        .try_to_scalar_int()
        .ok_or_else(|| {
            format!(
                "{}: only already-evaluated integer scalar constants are supported",
                tcx.symbol_name(instance).name
            )
        })?
        .size();
    let bits = const_.try_to_bits(size).ok_or_else(|| {
        format!(
            "{}: failed to read scalar constant bits",
            tcx.symbol_name(instance).name
        )
    })?;
    let bits = u64::try_from(bits).map_err(|_| {
        format!(
            "{}: constants wider than 64 bits are not supported",
            tcx.symbol_name(instance).name
        )
    })?;
    Ok(ValueRef::Integer {
        ty: scalar_ty,
        bits,
    })
}

fn lower_destination(state: &LoweringState, place: Place<'_>) -> Result<u32, String> {
    match lower_place_as_value(state, place)? {
        ValueRef::Local(local) => Ok(local),
        ValueRef::Integer { .. } => unreachable!("places cannot lower to integer constants"),
    }
}

fn stack_slot_for_place(state: &LoweringState, place: Place<'_>) -> Option<StackSlot> {
    place
        .projection
        .is_empty()
        .then(|| state.stack_slot(place.local))
        .flatten()
}

fn lower_call_destinations<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
    state: &LoweringState,
    place: Place<'tcx>,
) -> Result<Vec<u32>, String> {
    let ty = monomorphize_ty(tcx, instance, place.ty(&mir.local_decls, tcx).ty);
    if is_unit_ty(ty) {
        Ok(Vec::new())
    } else if let Some(field_types) = scalar_aggregate_field_types(tcx, ty) {
        field_types
            .iter()
            .enumerate()
            .map(|(field, _)| {
                state.tuple_field(place.local, field).ok_or_else(|| {
                    format!(
                        "{}: aggregate call destination field {field} is missing a synthetic local",
                        tcx.symbol_name(instance).name
                    )
                })
            })
            .collect()
    } else {
        lower_destination(state, place).map(|local| vec![local])
    }
}

fn lower_place_as_value(state: &LoweringState, place: Place<'_>) -> Result<ValueRef, String> {
    if place.projection.is_empty() {
        return Ok(ValueRef::Local(local_id(place.local)));
    }
    if place.projection.len() == 1
        && let ProjectionElem::Field(field, _) = place.projection[0]
        && let Some(local) = state.tuple_field(place.local, field.as_usize())
    {
        return Ok(ValueRef::Local(local));
    }
    Err(format!(
        "rustc_codegen_sci does not yet support projected place `{place:?}`"
    ))
}

struct MemoryPlace {
    ptr: ValueRef,
    offset: u64,
}

fn lower_memory_place<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
    place: Place<'tcx>,
) -> Result<Option<MemoryPlace>, String> {
    let Some((ProjectionElem::Deref, rest)) = place.projection.split_first() else {
        return Ok(None);
    };

    let mut current_ty = monomorphize_ty(tcx, instance, mir.local_decls[place.local].ty);
    current_ty = match current_ty.kind() {
        ty::RawPtr(pointee, _) | ty::Ref(_, pointee, _) => *pointee,
        _ => {
            return Err(format!(
                "{}: deref base has unsupported type `{}`",
                tcx.symbol_name(instance).name,
                current_ty
            ));
        }
    };

    let mut offset = 0_u64;
    for projection in rest {
        match projection {
            ProjectionElem::Field(field, field_ty) => {
                let layout = layout_for_memory_projection(tcx, instance, current_ty)?;
                offset = offset
                    .checked_add(layout.fields.offset(field.as_usize()).bytes())
                    .ok_or_else(|| {
                        format!(
                            "{}: memory field offset overflow for `{place:?}`",
                            tcx.symbol_name(instance).name
                        )
                    })?;
                current_ty = monomorphize_ty(tcx, instance, *field_ty);
            }
            ProjectionElem::ConstantIndex {
                offset: index,
                min_length,
                from_end: false,
            } => {
                let index = usize::try_from(*index).map_err(|_| {
                    format!(
                        "{}: memory constant index exceeds usize for `{place:?}`",
                        tcx.symbol_name(instance).name
                    )
                })?;
                if index as u64 >= *min_length {
                    return Err(format!(
                        "{}: memory constant index {index} exceeds minimum length {min_length} for `{place:?}`",
                        tcx.symbol_name(instance).name
                    ));
                }
                let (element_ty, element_offset) =
                    memory_array_element(tcx, instance, current_ty, index, place)?;
                offset = offset.checked_add(element_offset).ok_or_else(|| {
                    format!(
                        "{}: memory constant index offset overflow for `{place:?}`",
                        tcx.symbol_name(instance).name
                    )
                })?;
                current_ty = element_ty;
            }
            ProjectionElem::Index(index_local) => {
                let index = constant_memory_index(tcx, instance, mir, *index_local)?;
                let (element_ty, element_offset) =
                    memory_array_element(tcx, instance, current_ty, index, place)?;
                offset = offset.checked_add(element_offset).ok_or_else(|| {
                    format!(
                        "{}: memory index offset overflow for `{place:?}`",
                        tcx.symbol_name(instance).name
                    )
                })?;
                current_ty = element_ty;
            }
            other => {
                return Err(format!(
                    "{}: unsupported deref projection `{other:?}` in `{place:?}`",
                    tcx.symbol_name(instance).name
                ));
            }
        }
    }

    Ok(Some(MemoryPlace {
        ptr: ValueRef::Local(local_id(place.local)),
        offset,
    }))
}

fn memory_array_element<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    base_ty: Ty<'tcx>,
    index: usize,
    place: Place<'tcx>,
) -> Result<(Ty<'tcx>, u64), String> {
    let element_ty = match base_ty.kind() {
        ty::Array(element, _) => *element,
        _ => {
            return Err(format!(
                "{}: array index projection has unsupported base type `{}`",
                tcx.symbol_name(instance).name,
                base_ty
            ));
        }
    };
    let layout = layout_for_memory_projection(tcx, instance, base_ty)?;
    let FieldsShape::Array { count, .. } = &layout.fields else {
        return Err(format!(
            "{}: array index projection has non-array layout for `{}`",
            tcx.symbol_name(instance).name,
            base_ty
        ));
    };
    if index as u64 >= *count {
        return Err(format!(
            "{}: array index {index} exceeds array length {count} for `{place:?}`",
            tcx.symbol_name(instance).name
        ));
    }
    Ok((element_ty, layout.fields.offset(index).bytes()))
}

fn constant_memory_index<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    mir: &Body<'tcx>,
    index_local: Local,
) -> Result<usize, String> {
    if index_local == rustc_middle::mir::RETURN_PLACE
        || index_local.index() != 0 && index_local.index() <= mir.arg_count
    {
        return Err(format!(
            "{}: dynamic memory index local `{index_local:?}` is not supported",
            tcx.symbol_name(instance).name
        ));
    }
    let index_ty = mir
        .local_decls
        .get(index_local)
        .map(|decl| monomorphize_ty(tcx, instance, decl.ty))
        .ok_or_else(|| {
            format!(
                "{}: memory index local `{index_local:?}` is missing",
                tcx.symbol_name(instance).name
            )
        })?;
    if !matches!(index_ty.kind(), ty::Uint(ty::UintTy::Usize)) {
        return Err(format!(
            "{}: memory index local `{index_local:?}` has unsupported type `{index_ty}`",
            tcx.symbol_name(instance).name
        ));
    }

    let mut value = None;
    for block in mir.basic_blocks.iter() {
        for statement in &block.statements {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let (place, rvalue) = &**assign;
            if place.local != index_local || !place.projection.is_empty() {
                continue;
            }
            let Rvalue::Use(Operand::Constant(constant), _) = rvalue else {
                return Err(format!(
                    "{}: dynamic memory index local `{index_local:?}` is not supported",
                    tcx.symbol_name(instance).name
                ));
            };
            if value.is_some() {
                return Err(format!(
                    "{}: memory index local `{index_local:?}` has multiple assignments",
                    tcx.symbol_name(instance).name
                ));
            }
            let bits = constant_usize_bits(tcx, instance, constant)?;
            value = Some(usize::try_from(bits).map_err(|_| {
                format!(
                    "{}: memory index constant exceeds usize",
                    tcx.symbol_name(instance).name
                )
            })?);
        }
    }
    value.ok_or_else(|| {
        format!(
            "{}: memory index local `{index_local:?}` has no constant assignment",
            tcx.symbol_name(instance).name
        )
    })
}

fn constant_usize_bits<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    constant: &ConstOperand<'tcx>,
) -> Result<u64, String> {
    let value = lower_constant(tcx, instance, constant)?;
    match value {
        ValueRef::Integer { bits, .. } => Ok(bits),
        ValueRef::Local(_) => unreachable!("constants cannot lower to locals"),
    }
}

fn layout_for_memory_projection<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    ty: Ty<'tcx>,
) -> Result<rustc_middle::ty::layout::TyAndLayout<'tcx>, String> {
    tcx.layout_of(ty::TypingEnv::fully_monomorphized().as_query_input(ty))
        .map_err(|err| {
            format!(
                "{}: failed to compute memory projection layout for `{}`: {err:?}",
                tcx.symbol_name(instance).name,
                ty
            )
        })
}

fn scalar_memory_layout<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    ty: Ty<'tcx>,
) -> Result<(ScalarType, u64, u64), String> {
    let scalar = scalar_type_for_ty(ty).ok_or_else(|| {
        format!(
            "{}: memory operation has unsupported type `{}`",
            tcx.symbol_name(instance).name,
            ty
        )
    })?;
    let layout = tcx
        .layout_of(ty::TypingEnv::fully_monomorphized().as_query_input(ty))
        .map_err(|err| {
            format!(
                "{}: failed to compute memory layout for `{}`: {err:?}",
                tcx.symbol_name(instance).name,
                ty
            )
        })?;
    Ok((scalar, layout.size.bytes(), layout.align.abi.bytes()))
}

fn monomorphize_ty<'tcx>(tcx: TyCtxt<'tcx>, instance: Instance<'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
    instance.instantiate_mir_and_normalize_erasing_regions(
        tcx,
        ty::TypingEnv::fully_monomorphized(),
        ty::EarlyBinder::bind(tcx, ty),
    )
}

fn scalar_type_for_ty(ty: Ty<'_>) -> Option<ScalarType> {
    match ty.kind() {
        ty::Bool => Some(ScalarType::I1),
        ty::Int(ty::IntTy::I8) => Some(ScalarType::I8),
        ty::Int(ty::IntTy::I16) => Some(ScalarType::I16),
        ty::Int(ty::IntTy::I32) => Some(ScalarType::I32),
        ty::Int(ty::IntTy::I64 | ty::IntTy::Isize) => Some(ScalarType::I64),
        ty::Uint(ty::UintTy::U8) => Some(ScalarType::U8),
        ty::Uint(ty::UintTy::U16) => Some(ScalarType::U16),
        ty::Uint(ty::UintTy::U32) => Some(ScalarType::U32),
        ty::Uint(ty::UintTy::U64 | ty::UintTy::Usize) => Some(ScalarType::U64),
        ty::RawPtr(..) | ty::Ref(..) | ty::FnPtr(..) => Some(ScalarType::Ptr),
        _ => None,
    }
}

fn is_unit_ty(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Tuple(fields) if fields.is_empty())
}

fn is_empty_struct_ty(ty: Ty<'_>) -> bool {
    match ty.kind() {
        ty::Adt(adt_def, _) => adt_def.is_struct() && adt_def.non_enum_variant().fields.is_empty(),
        _ => false,
    }
}

fn scalar_aggregate_field_types<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Vec<ScalarType>> {
    match ty.kind() {
        ty::Tuple(fields) => {
            if fields.is_empty() {
                return None;
            }
            fields.iter().map(scalar_type_for_ty).collect()
        }
        ty::Adt(adt_def, args) if adt_def.is_struct() => {
            let fields = &adt_def.non_enum_variant().fields;
            if fields.is_empty() {
                return None;
            }
            fields
                .iter()
                .map(|field| scalar_type_for_ty(field.ty(tcx, args).skip_norm_wip()))
                .collect()
        }
        _ => None,
    }
}

fn scalar_bit_width(ty: Ty<'_>) -> Option<u32> {
    match ty.kind() {
        ty::Bool => Some(1),
        ty::Int(ty::IntTy::I8) | ty::Uint(ty::UintTy::U8) => Some(8),
        ty::Int(ty::IntTy::I16) | ty::Uint(ty::UintTy::U16) => Some(16),
        ty::Int(ty::IntTy::I32) | ty::Uint(ty::UintTy::U32) => Some(32),
        ty::Int(ty::IntTy::I64 | ty::IntTy::Isize)
        | ty::Uint(ty::UintTy::U64 | ty::UintTy::Usize) => Some(64),
        _ => None,
    }
}

fn scalar_type_size_bytes(ty: ScalarType) -> Option<u64> {
    match ty {
        ScalarType::I8 | ScalarType::U8 => Some(1),
        ScalarType::I16 | ScalarType::U16 => Some(2),
        ScalarType::I32 | ScalarType::U32 => Some(4),
        ScalarType::I64 | ScalarType::U64 => Some(8),
        ScalarType::I1 | ScalarType::Ptr => None,
    }
}

fn all_ones_for_scalar(ty: ScalarType) -> u64 {
    match ty {
        ScalarType::I1 => 1,
        ScalarType::I8 | ScalarType::U8 => u8::MAX.into(),
        ScalarType::I16 | ScalarType::U16 => u16::MAX.into(),
        ScalarType::I32 | ScalarType::U32 => u32::MAX.into(),
        ScalarType::I64 | ScalarType::U64 => u64::MAX,
        ScalarType::Ptr => unreachable!("pointer values do not have an integer all-ones mask"),
    }
}

fn is_signed_integer(ty: Ty<'_>) -> Option<bool> {
    match ty.kind() {
        ty::Int(_) => Some(true),
        ty::Uint(_) | ty::Bool => Some(false),
        _ => None,
    }
}

fn local_id(local: rustc_middle::mir::Local) -> u32 {
    u32::try_from(local.index()).expect("MIR local index exceeds u32")
}

fn block_id_id(block: BasicBlock) -> u32 {
    u32::try_from(block.index()).expect("MIR block index exceeds u32")
}

fn run_worker(
    module: &SciModulePlan,
    object_path: &Path,
    emit_sa_path: Option<&Path>,
    request_id: u64,
) -> Result<(), String> {
    let worker = env::var_os("SCI_CODEGEN_WORKER")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("sci-codegen-worker"));
    let mut child = Command::new(&worker)
        .arg("--stdio-once")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to start {}: {err}", worker.display()))?;

    let request = CompileRequest {
        request_id,
        output_path: object_path.display().to_string(),
        emit_sa_path: emit_sa_path.map(|path| path.display().to_string()),
        module: module.clone(),
    };
    {
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "failed to open SCI worker stdin".to_string())?;
        write_frame(stdin, &request)
            .map_err(|err| format!("failed to send SCI compile request: {err}"))?;
    }

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to open SCI worker stdout".to_string())?;
    let response: CompileResponse =
        read_frame(stdout).map_err(|err| format!("failed to read SCI worker response: {err}"))?;
    let output = child
        .wait_with_output()
        .map_err(|err| format!("failed to wait for SCI worker: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "SCI worker exited with {}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    if response.request_id != request_id {
        return Err(format!(
            "SCI worker response id mismatch: got {}, expected {}",
            response.request_id, request_id
        ));
    }
    if !response.success {
        return Err(format_worker_rejection(&response));
    }
    if !object_path.is_file() {
        return Err(format!(
            "SCI worker reported success without object {}",
            object_path.display()
        ));
    }
    Ok(())
}

fn backend_diagnostic_payload(
    message: String,
    location: Option<DiagnosticLocation>,
) -> DiagnosticPayload {
    let location = location.or_else(|| backend_diagnostic_location(&message));
    DiagnosticPayload {
        code: classify_backend_diagnostic_code(&message).into(),
        message,
        location,
    }
}

fn format_backend_rejection(diagnostic: &DiagnosticPayload) -> String {
    if diagnostic.code.is_empty() {
        return diagnostic.message.clone();
    }
    let mut message = String::from("rustc_codegen_sci backend rejected module [");
    message.push_str(&diagnostic.code);
    message.push(']');
    if let Some(location) = &diagnostic.location {
        let location = format_diagnostic_location(location);
        if !location.is_empty() {
            message.push_str(" at ");
            message.push_str(&location);
        }
    }
    message.push_str(": ");
    message.push_str(&diagnostic.message);
    message
}

fn classify_backend_diagnostic_code(diagnostic: &str) -> &'static str {
    if diagnostic.contains("unsupported MIR")
        || diagnostic.contains("unsupported rvalue")
        || diagnostic.contains("unsupported operand")
        || diagnostic.contains("unsupported binary operation")
        || diagnostic.contains("unsupported unary operation")
        || diagnostic.contains("unsupported cast kind")
        || diagnostic.contains("unsupported deref projection")
        || diagnostic.contains("projected place")
        || diagnostic.contains("taking the address")
        || diagnostic.contains("dynamic memory index")
    {
        "SCI_BACKEND_MIR_UNSUPPORTED"
    } else if diagnostic.contains("ABI")
        || diagnostic.contains("FnAbi")
        || diagnostic.contains("extern callee")
        || diagnostic.contains("aggregate argument")
        || diagnostic.contains("aggregate return")
        || diagnostic.contains("zero-sized struct function ABI")
        || diagnostic.contains("unit function arguments")
    {
        "SCI_BACKEND_ABI_UNSUPPORTED"
    } else if diagnostic.contains("layout")
        || diagnostic.contains("field")
        || diagnostic.contains("variant")
        || diagnostic.contains("niche")
        || diagnostic.contains("alignment")
    {
        "SCI_BACKEND_LAYOUT_ERROR"
    } else if diagnostic.contains("static mono item")
        || diagnostic.contains("allocation")
        || diagnostic.contains("relocation")
    {
        "SCI_BACKEND_STATIC_UNSUPPORTED"
    } else if diagnostic.contains("target")
        || diagnostic.contains("pointer width")
        || diagnostic.contains("relocation model")
        || diagnostic.contains("code model")
    {
        "SCI_BACKEND_TARGET_UNSUPPORTED"
    } else if diagnostic.contains("SCI worker")
        || diagnostic.contains("SCI compile request")
        || diagnostic.contains("object")
        || diagnostic.contains("failed to start")
        || diagnostic.contains("failed to open")
        || diagnostic.contains("failed to read")
        || diagnostic.contains("failed to wait")
    {
        "SCI_BACKEND_IO"
    } else {
        "SCI_BACKEND_REJECTED"
    }
}

fn format_worker_rejection(response: &CompileResponse) -> String {
    let mut message = String::from("SCI worker rejected module");
    if let Some(diagnostic) = &response.diagnostic {
        message.push_str(" [");
        message.push_str(&diagnostic.code);
        message.push(']');
    }
    if let Some(location) = response
        .diagnostic
        .as_ref()
        .and_then(|diagnostic| diagnostic.location.as_ref())
    {
        let location = format_diagnostic_location(location);
        if !location.is_empty() {
            message.push_str(" at ");
            message.push_str(&location);
        }
    }
    message.push_str(": ");
    if let Some(diagnostic) = &response.diagnostic {
        message.push_str(&diagnostic.message);
    }
    message
}

fn format_diagnostic_location(location: &sci_protocol::DiagnosticLocation) -> String {
    let mut parts = Vec::new();
    if let Some(function) = &location.function {
        parts.push(format!("function `{function}`"));
    }
    if let Some(block) = location.block {
        parts.push(format!("block {block}"));
    }
    if let Some(local) = location.local {
        parts.push(format!("local {local}"));
    }
    parts.join(", ")
}

fn backend_diagnostic_location(message: &str) -> Option<DiagnosticLocation> {
    let function = backend_diagnostic_function(message);
    let block = diagnostic_number_after(message, "block ");
    let local = diagnostic_number_after(message, "local ");
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

fn backend_diagnostic_function(message: &str) -> Option<String> {
    if let Some(rest) = message.strip_prefix("function ") {
        return rest.split_whitespace().next().map(str::to_string);
    }
    if let Some(rest) = message.strip_prefix("extern function ") {
        return rest.split_whitespace().next().map(str::to_string);
    }
    message
        .split_once(':')
        .and_then(|(name, _)| (!name.contains(char::is_whitespace)).then(|| name.to_string()))
}

fn diagnostic_number_after(message: &str, marker: &str) -> Option<u32> {
    let rest = message.split(marker).nth(1)?;
    let number = rest
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    (!number.is_empty()).then(|| number.parse().ok()).flatten()
}

#[unsafe(no_mangle)]
pub fn __rustc_codegen_backend() -> Box<dyn CodegenBackend> {
    Box::new(SciCodegenBackend)
}
