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
    AbsPath, ConstrId, FunId, GlobalId, Ident, LitId, Namespace, NifId, Path, PathType, Span,
};

pub(crate) struct CodeBuilder<'a, L: Clone + Eq + core::hash::Hash> {
    pub(crate) params: &'a CompilationParams<L>,
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
        params: &'a CompilationParams<L>,
        funs_tbl: SymbolsTable<FunId>,
        lambdas_vec: IdVecAfter<FunId, FunDef>,
        globals: GlobalBindings<BindingType>,
    ) -> Self {
        Self {
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
    fundef: Option<ir::FunDef>,
    funimpl: ir::FunImpl,
) -> Result<FunDef, CompilationError> {
    let name = fundef.map(|x| x.name.clone());
    let ir::FunImpl { vars, body } = funimpl;

    let mut local = LocalBindings::new();
    local.scope_enter();

    for (var_i, var) in vars.iter().enumerate() {
        let var_i = var_i.try_into().map_err(|_| {
            CompilationError::FunctionParamsMoreThanLimit(var.0.span.clone(), vars.len())
        })?;
        local.add_param(var.0.clone().unspan(), var_i);
    }

    let arity = vars.len().try_into().map(|n| CallArity(n)).unwrap();

    let code_pos = state.get_instruction_address();
    let tc = generate_expression_code(state, &mut local, FunPos::Root, body.clone())?;
    if !tc {
        state.write_code().push(Instruction::Ret);
    }
    let stack_size = local.scope_terminate();

    // now compute the code for the lambdas. This is in a loop
    // since it can generate further lambdas
    while !state.lambdas.is_empty() {
        let mut lambdas = Vec::new();
        core::mem::swap(&mut state.lambdas, &mut lambdas);

        for (code_ref, fun_impl) in lambdas {
            let lirdef =
                generate_func_code(state, None, fun_impl).map_err(|e: CompilationError| {
                    e.context(format!("function lambda code {:?}", name))
                })?;
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
    local: &mut LocalBindings,
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
        ir::Expr::List(_span, _l) => {
            todo!("list ?")
        }
        ir::Expr::Let(binder, body, in_expr) => {
            let x = body.clone();
            let _: bool = generate_expression_code(state, local, FunPos::NotRoot, *body)
                .map_err(|e| e.context(alloc::format!("{:?}", *x)))?;
            match binder {
                ir::Binder::Ident(ident) => {
                    let bind = local.add_local(ident.clone());
                    state.write_code().push(Instruction::LocalBind(bind));
                }
                ir::Binder::Ignore => {
                    state.write_code().push(Instruction::IgnoreOne);
                }
                ir::Binder::Unit => {
                    // TODO, not sure ignore one is the best to do here
                    state.write_code().push(Instruction::IgnoreOne);
                }
            }
            let tc = generate_expression_code(state, local, funpos, *in_expr)?;
            Ok(tc)
        }
        ir::Expr::Field(expr, struct_ident, field_ident) => {
            let (struct_path, _) = resolve_path(&state.resolver, &struct_ident.inner);
            let (constr_id, constr_def) =
                state
                    .constrs
                    .get(&struct_path)
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
                .push(Instruction::AccessField(constr_id, index));
            Ok(false)
        }
        ir::Expr::Lambda(_span, funimpl) => {
            let lambda_fetch = state.write_code().push_temp();

            state.lambda_setaside(lambda_fetch, *funimpl);

            Ok(false)
        }
        ir::Expr::Call(_span, args) => {
            assert!(args.len() > 0);
            let len = args.len() - 1;
            for arg in args {
                let _: bool = generate_expression_code(state, local, FunPos::NotRoot, arg)?;
                ()
            }
            if funpos == FunPos::Root {
                state
                    .write_code()
                    .push(Instruction::Call(TailCall::Yes, CallArity(len as u8)));
                Ok(true)
            } else {
                state
                    .write_code()
                    .push(Instruction::Call(TailCall::No, CallArity(len as u8)));
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

            local.scope_enter();
            let tc_then = generate_expression_code(state, local, funpos, (*then_expr).unspan())?;
            local.scope_leave();

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

            local.scope_enter();
            let tc_else = generate_expression_code(state, local, funpos, (*else_expr).unspan())?;
            local.scope_leave();

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
    local: &LocalBindings,
    span: Span,
    path: Path,
) -> Result<BindingType, CompilationError> {
    if let Some(local_path) = path.get_local() {
        if let Some(bound) = local.bindings.get(local_path) {
            return Ok(*bound);
        }
    }

    let resolved = resolve_path(&state.resolver, &path);

    if let Some(bound) = state.globals.get(&resolved.0) {
        Ok(*bound)
    } else {
        if let Some(resolved) = &resolved.1 {
            if let Some(bound) = state.globals.get(resolved) {
                Ok(*bound)
            } else {
                Err(CompilationError::MissingSymbol(span, path))
            }
        } else {
            Err(CompilationError::MissingSymbol(span, path))
        }
    }
}

fn resolve_path(resolver: &Option<SymbolResolver>, path: &Path) -> (AbsPath, Option<AbsPath>) {
    match path.path_type() {
        PathType::Absolute => {
            let (namespace, ident) = path.split();
            (AbsPath::new(&namespace, &ident), None)
        }
        PathType::Relative => {
            if let Some(resolver) = resolver {
                let (namespace, ident) = path.split();
                let full_namespace = resolver.current.append_namespace(&namespace);
                (
                    AbsPath::new(&full_namespace, &ident),
                    Some(AbsPath::new(&Namespace::root(), &ident)),
                )
            } else {
                panic!("no resolver")
            }
        }
    }
}
