use super::bindings::{BindingsStack, GlobalBindings};
use super::code::*;
use super::defs::*;
use super::errors::*;
use super::instructions::*;
use super::resolver::SymbolResolver;
use super::symbols::*;
use super::CompilationParams;
use alloc::{format, vec, vec::Vec};
use werbolg_core as ir;
use werbolg_core::{
    ConstrId, FunId, GlobalId, Ident, LitId, Namespace, NifId, Path, PathType, Span,
};

pub(crate) struct CompilationSharedState {}
pub(crate) struct CompilationLocalState {
    namespace: Namespace,
    bindings: LocalBindings,
}

pub(crate) struct CodeBuilder<'a, L: Clone + Eq + core::hash::Hash> {
    #[allow(unused)]
    pub(crate) shared: &'a CompilationSharedState,
    pub(crate) params: CompilationParams<L>,
    pub(crate) funs_tbl: SymbolsTable<FunId>,
    pub(crate) funs_vec: IdVec<FunId, FunDef>,
    pub(crate) lambdas_vec: IdVecAfter<FunId, FunDef>,
    pub(crate) constrs: SymbolsTableData<ConstrId, ConstrDef>,
    pub(crate) lits: UniqueTableBuilder<LitId, L>,
    pub(crate) main_code: Code,
    pub(crate) lambdas: Vec<(CodeRef, ir::FunImpl)>,
    pub(crate) globals: GlobalBindings<BindingType>,
    pub(crate) resolver: Option<SymbolResolver>,
}

pub struct LocalBindings {
    bindings: BindingsStack<BindingType>,
    local: Vec<u16>,
    max_local: u16,
}

impl LocalBindings {
    pub fn new() -> Self {
        Self {
            bindings: BindingsStack::new(),
            local: vec![0],
            max_local: 0,
        }
    }

    pub fn add_param(&mut self, ident: Ident, n: u8) {
        self.bindings
            .add(ident, BindingType::Param(ParamBindIndex(n)))
    }

    pub fn add_local(&mut self, ident: Ident) -> LocalBindIndex {
        match self.local.last_mut() {
            None => panic!("internal error: cannot add local without an empty binding stack"),
            Some(x) => {
                let local = *x;
                *x += 1;

                let local = LocalBindIndex(local);
                self.bindings.add(ident, BindingType::Local(local));
                local
            }
        }
    }

    pub fn scope_enter(&mut self) {
        let top = self.local.last().unwrap();
        self.local.push(*top);
        self.bindings.scope_enter();
    }

    pub fn scope_leave(&mut self) {
        let _x = self.bindings.scope_pop();
        let local = self.local.pop().unwrap();
        self.max_local = core::cmp::max(self.max_local, local);
    }

    pub fn scope_terminate(mut self) -> LocalStackSize {
        self.scope_leave();
        assert_eq!(self.local.len(), 1, "internal compilation error");
        LocalStackSize(self.max_local as u16)
    }
}

#[derive(Clone, Copy)]
pub enum BindingType {
    Global(GlobalId),
    Nif(NifId),
    Fun(FunId),
    Param(ParamBindIndex),
    Local(LocalBindIndex),
}

impl<'a, L: Clone + Eq + core::hash::Hash> CodeBuilder<'a, L> {
    pub fn new(
        shared: &'a CompilationSharedState,
        params: CompilationParams<L>,
        funs_tbl: SymbolsTable<FunId>,
        lambdas_vec: IdVecAfter<FunId, FunDef>,
        globals: GlobalBindings<BindingType>,
    ) -> Self {
        Self {
            shared,
            params,
            funs_tbl,
            funs_vec: IdVec::new(),
            lambdas_vec,
            main_code: Code::new(),
            lambdas: Vec::new(),
            constrs: SymbolsTableData::new(),
            lits: UniqueTableBuilder::new(),
            globals,
            resolver: None,
        }
    }

    fn lambda_setaside(&mut self, code_ref: CodeRef, fun_impl: ir::FunImpl) {
        self.lambdas.push((code_ref, fun_impl));
    }

    fn get_instruction_address(&self) -> InstructionAddress {
        self.main_code.position()
    }

    pub fn set_module_resolver(&mut self, uses: &SymbolResolver) {
        self.resolver = Some(uses.clone());
    }

    fn write_code(&mut self) -> &mut Code {
        &mut self.main_code
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FunPos {
    Root,
    NotRoot,
}

pub(crate) fn generate_func_code<'a, L: Clone + Eq + core::hash::Hash>(
    state: &mut CodeBuilder<'a, L>,
    namespace: &Namespace,
    fundef: Option<ir::FunDef>,
    funimpl: ir::FunImpl,
) -> Result<FunDef, CompilationError> {
    let name = fundef.map(|x| x.name.clone());
    let ir::FunImpl { vars, body } = funimpl;

    let mut local = CompilationLocalState {
        bindings: LocalBindings::new(),
        namespace: namespace.clone(),
    };
    local.bindings.scope_enter();

    for (var_i, var) in vars.iter().enumerate() {
        let var_i = var_i.try_into().map_err(|_| {
            CompilationError::FunctionParamsMoreThanLimit(var.0.span.clone(), vars.len())
        })?;
        local.bindings.add_param(var.0.clone().unspan(), var_i);
    }

    let arity = vars.len().try_into().map(|n| CallArity(n)).unwrap();

    let code_pos = state.get_instruction_address();
    let tc = generate_expression_code(state, &mut local, FunPos::Root, body.clone())?;
    if !tc {
        state.write_code().push(Instruction::Ret);
    }
    let stack_size = local.bindings.scope_terminate();

    // now compute the code for the lambdas. This is in a loop
    // since it can generate further lambdas
    while !state.lambdas.is_empty() {
        let mut lambdas = Vec::new();
        core::mem::swap(&mut state.lambdas, &mut lambdas);

        for (code_ref, fun_impl) in lambdas {
            let lirdef = generate_func_code(state, namespace, None, fun_impl).map_err(
                |e: CompilationError| e.context(format!("function lambda code {:?}", name)),
            )?;
            let lambda_funid = state.lambdas_vec.push(lirdef);

            state
                .write_code()
                .resolve_temp(code_ref, Instruction::FetchFun(lambda_funid));
        }
    }

    Ok(FunDef {
        name,
        arity,
        code_pos,
        stack_size,
    })
}

fn generate_expression_code<'a, L: Clone + Eq + core::hash::Hash>(
    state: &mut CodeBuilder<'a, L>,
    local: &mut CompilationLocalState,
    funpos: FunPos,
    expr: ir::Expr,
) -> Result<bool, CompilationError> {
    match expr {
        ir::Expr::Literal(span, lit) => {
            let lit_id = state.lits.add((state.params.literal_mapper)(span, lit)?);
            state.write_code().push(Instruction::PushLiteral(lit_id));
            Ok(false)
        }
        ir::Expr::Path(span, path) => {
            let x = fetch_ident(state, local, span, path.clone())?;
            match x {
                BindingType::Global(idx) => {
                    state.write_code().push(Instruction::FetchGlobal(idx));
                }
                BindingType::Nif(idx) => {
                    state.write_code().push(Instruction::FetchNif(idx));
                }
                BindingType::Fun(idx) => {
                    state.write_code().push(Instruction::FetchFun(idx));
                }
                BindingType::Local(idx) => {
                    state.write_code().push(Instruction::FetchStackLocal(idx));
                }
                BindingType::Param(idx) => {
                    state.write_code().push(Instruction::FetchStackParam(idx));
                }
            }
            Ok(false)
        }
        ir::Expr::Sequence(span, l) => {
            let call_arity = l
                .len()
                .try_into()
                .map_err(|sz| CompilationError::CallTooManyArguments(span.clone(), sz))?;
            for e in l {
                let _: bool = generate_expression_code(state, local, FunPos::NotRoot, e)?;
            }
            match state.params.sequence_constructor {
                None => return Err(CompilationError::SequenceNotSupported(span)),
                Some(nifid) => {
                    state
                        .write_code()
                        .push(Instruction::CallNif(nifid, call_arity));
                    Ok(true)
                }
            }
        }
        ir::Expr::Let(binder, body, in_expr) => {
            let x = body.clone();
            let _: bool = generate_expression_code(state, local, FunPos::NotRoot, *body)
                .map_err(|e| e.context(alloc::format!("{:?}", *x)))?;
            match binder {
                ir::Binder::Ident(ident) => {
                    let bind = local.bindings.add_local(ident.clone());
                    state.write_code().push(Instruction::LocalBind(bind));
                }
                ir::Binder::Ignore => {
                    state.write_code().push(Instruction::IgnoreOne);
                }
                ir::Binder::Unit => {
                    // TODO, not sure ignore one is the best to do here
                    state.write_code().push(Instruction::IgnoreOne);
                }
                ir::Binder::Deconstruct(_name, _) => {
                    todo!()
                }
            }
            let tc = generate_expression_code(state, local, funpos, *in_expr)?;
            Ok(tc)
        }
        ir::Expr::Field(expr, struct_ident, field_ident) => {
            let resolved = resolve_symbol(&state, &local, &struct_ident.inner);
            let result = resolved
                .into_iter()
                .filter_map(|res| match res {
                    Resolution::Constructor(c, _) => Some(c),
                    _ => None,
                })
                .collect::<Vec<_>>();

            let constr_id = if result.is_empty() {
                return Err(CompilationError::MissingSymbol(
                    struct_ident.span,
                    struct_ident.inner,
                ));
            } else if result.len() > 1 {
                /*
                return Err(CompilationError::DuplicateSymbol(
                    struct_ident.span,
                    struct_ident.inner,
                ));
                */
                todo!()
            } else {
                &result[0]
            };

            let constr_def =
                state
                    .constrs
                    .get_by_id(*constr_id)
                    .ok_or(CompilationError::MissingConstructor(
                        struct_ident.span.clone(),
                        struct_ident.inner.clone(),
                    ))?;

            let ConstrDef::Struct(struct_def) = constr_def else {
                return Err(CompilationError::ConstructorNotStructure(
                    struct_ident.span,
                    struct_ident.inner,
                ));
            };

            let Some(index) = struct_def.find_field_index(&field_ident.inner) else {
                return Err(CompilationError::StructureFieldNotExistant(
                    field_ident.span,
                    struct_ident.inner,
                    field_ident.inner,
                ));
            };

            let _: bool = generate_expression_code(state, local, funpos, *expr)?;
            state
                .write_code()
                .push(Instruction::AccessField(*constr_id, index));
            Ok(false)
        }
        ir::Expr::Lambda(_span, funimpl) => {
            let lambda_fetch = state.write_code().push_temp();

            state.lambda_setaside(lambda_fetch, *funimpl);

            Ok(false)
        }
        ir::Expr::Call(span, args) => {
            assert!(args.len() > 0);
            let len = args.len() - 1;
            for arg in args {
                let _: bool = generate_expression_code(state, local, FunPos::NotRoot, arg)?;
                ()
            }
            let call_arity = len
                .try_into()
                .map_err(|sz| CompilationError::CallTooManyArguments(span, sz))?;
            if funpos == FunPos::Root {
                state
                    .write_code()
                    .push(Instruction::Call(TailCall::Yes, call_arity));
                Ok(true)
            } else {
                state
                    .write_code()
                    .push(Instruction::Call(TailCall::No, call_arity));
                Ok(false)
            }
        }
        ir::Expr::If {
            span: _,
            cond,
            then_expr,
            else_expr,
        } => {
            let _: bool =
                generate_expression_code(state, local, FunPos::NotRoot, (*cond).unspan())?;

            let cond_jump_ref = state.write_code().push_temp();
            let cond_pos = state.get_instruction_address();

            local.bindings.scope_enter();
            let tc_then = generate_expression_code(state, local, funpos, (*then_expr).unspan())?;
            local.bindings.scope_leave();

            // if we are at the root, check if we need to ret or not, otherwise
            // push a temporary for jumping to the end of the block
            let jump_else_ref = if funpos == FunPos::Root {
                if !tc_then {
                    state.write_code().push(Instruction::Ret);
                };
                None
            } else {
                Some(state.write_code().push_temp())
            };

            let else_pos = state.get_instruction_address();

            local.bindings.scope_enter();
            let tc_else = generate_expression_code(state, local, funpos, (*else_expr).unspan())?;
            local.bindings.scope_leave();

            let end_pos = state.get_instruction_address();

            if funpos == FunPos::Root {
                if !tc_else {
                    state.write_code().push(Instruction::Ret);
                }
            }

            // write the cond jump displacement to jump to the else block if condition fails
            state
                .write_code()
                .resolve_temp(cond_jump_ref, Instruction::CondJump(else_pos - cond_pos));

            if let Some(jump_else_ref) = jump_else_ref {
                state
                    .write_code()
                    .resolve_temp(jump_else_ref, Instruction::Jump(end_pos - else_pos));
            }

            Ok(true)
        }
    }
}

fn fetch_ident<'a, L: Clone + Eq + core::hash::Hash>(
    state: &CodeBuilder<'a, L>,
    local: &CompilationLocalState,
    span: Span,
    path: Path,
) -> Result<BindingType, CompilationError> {
    let resolved = resolve_symbol(state, local, &path);

    if resolved.is_empty() {
        std::println!("resolution empty");
        Err(CompilationError::MissingSymbol(span, path))
    } else if resolved.len() > 1 {
        Err(CompilationError::MultipleSymbol(span, path))
    } else {
        let r = &resolved[0];
        match r {
            Resolution::Constructor(_, _) => {
                // some error
                todo!()
            }
            Resolution::Binding(r) => Ok(*r),
        }
    }
}

pub enum Resolution {
    Constructor(ConstrId, Vec<Ident>),
    Binding(BindingType),
}

pub fn resolve_symbol_at<'a, L: Clone + Eq + core::hash::Hash>(
    state: &CodeBuilder<'a, L>,
    namespace: Namespace,
    path: &Path,
) -> Vec<Resolution> {
    let mut out = Vec::new();

    let mut constr_table = Some(&state.constrs.table.0);
    let mut bind_table = Some(&state.globals.0);

    // if namespace is not empty, we need to tweak those symbol table
    if !namespace.is_root() {
        loop {
            let (ident, ns) = namespace.clone().drop_first();
            if let Some(tbl) = constr_table {
                constr_table = tbl.get_sub(&ident).ok();
            } else {
                panic!("missing module {:?}", namespace)
            }
            if let Some(tbl) = bind_table {
                bind_table = tbl.get_sub(&ident).ok();
            } else {
                panic!("missing module {:?}", namespace)
            }

            if ns.is_root() {
                break;
            }
        }
    }

    let mut idents = path.components();
    let mut current_ns = namespace;

    while let Some((ident, remaining)) = idents.next() {
        let mut namespace_entered = false;
        // check in the constructor symbols (struct / enum)
        if let Some(tbl) = constr_table {
            if let Some(constr) = tbl.current().get(ident) {
                out.push(Resolution::Constructor(constr, remaining.to_vec()))
            }
            constr_table = tbl.get_sub(ident).ok();
            if constr_table.is_some() {
                namespace_entered = true;
            }
        }
        // check in the function symbols
        if let Some(tbl) = bind_table {
            if let Some(bty) = tbl.current().get(ident) {
                if remaining.is_empty() {
                    out.push(Resolution::Binding(*bty))
                }
            }
            bind_table = tbl.get_sub(ident).ok();
            if bind_table.is_some() {
                namespace_entered = true;
            }
        }

        if !namespace_entered {
            // error now ?
            break;
        }

        current_ns = current_ns.append(ident.clone())
    }
    out
}

pub fn resolve_symbol<'a, L: Clone + Eq + core::hash::Hash>(
    state: &CodeBuilder<'a, L>,
    local: &CompilationLocalState,
    path: &Path,
) -> Vec<Resolution> {
    match path.path_type() {
        PathType::Absolute => {
            // if the path is absolute, we only lookup through the defined symbols, so we never
            // look in the local bindings
            resolve_symbol_at(state, Namespace::root(), path)
        }
        PathType::Relative => {
            if let Some(local_path) = path.get_local() {
                if let Some(bound) = local.bindings.bindings.get(local_path) {
                    return vec![Resolution::Binding(*bound)];
                }
            }
            let mut result = resolve_symbol_at(state, local.namespace.clone(), path);
            let mut root_result = resolve_symbol_at(state, Namespace::root(), path);
            result.append(&mut root_result);
            result
        }
    }
}
