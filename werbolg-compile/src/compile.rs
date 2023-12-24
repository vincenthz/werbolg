use super::bindings::{BindingsStack, GlobalBindings};
use super::code::*;
use super::defs::*;
use super::errors::*;
use super::instructions::*;
use super::symbols::*;
use super::CompilationParams;
use alloc::{vec, vec::Vec};
use werbolg_core as ir;
use werbolg_core::{ConstrId, FunId, GlobalId, Ident, LitId, NifId, Path, Span};

pub(crate) struct RewriteState<'a, L: Clone + Eq + core::hash::Hash> {
    pub(crate) params: &'a CompilationParams<L>,
    pub(crate) funs_tbl: SymbolsTable<FunId>,
    pub(crate) funs_vec: IdVec<FunId, FunDef>,
    pub(crate) constrs: SymbolsTableData<ConstrId, ConstrDef>,
    pub(crate) lits: UniqueTableBuilder<LitId, L>,
    pub(crate) main_code: Code,
    pub(crate) lambdas: IdVecAfter<FunId, FunDef>,
    pub(crate) lambdas_code: Code,
    pub(crate) in_lambda: CodeState,
    pub(crate) globals: GlobalBindings<BindingType>,
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
            .add(Path::relative(ident), BindingType::Param(ParamBindIndex(n)))
    }

    pub fn add_local(&mut self, ident: Ident) -> LocalBindIndex {
        match self.local.last_mut() {
            None => panic!("internal error: cannot add local without an empty binding stack"),
            Some(x) => {
                let local = *x;
                *x += 1;

                let local = LocalBindIndex(local);
                self.bindings
                    .add(Path::relative(ident), BindingType::Local(local));
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

#[derive(Clone, Copy, Default)]
pub enum CodeState {
    #[default]
    InMain,
    InLambda,
}

impl<'a, L: Clone + Eq + core::hash::Hash> RewriteState<'a, L> {
    pub fn new(
        params: &'a CompilationParams<L>,
        funs_tbl: SymbolsTable<FunId>,
        lambdas: IdVecAfter<FunId, FunDef>,
        globals: GlobalBindings<BindingType>,
    ) -> Self {
        Self {
            params,
            funs_tbl,
            funs_vec: IdVec::new(),
            main_code: Code::new(),
            lambdas,
            lambdas_code: Code::new(),
            constrs: SymbolsTableData::new(),
            lits: UniqueTableBuilder::new(),
            in_lambda: CodeState::default(),
            globals,
        }
    }

    #[must_use = "code state need to be restore using restore_codestate"]
    fn set_in_lambda(&mut self) -> CodeState {
        let saved = self.in_lambda;
        self.in_lambda = CodeState::InLambda;
        saved
    }

    fn restore_codestate(&mut self, code_state: CodeState) {
        self.in_lambda = code_state;
    }

    fn get_instruction_address(&self) -> InstructionAddress {
        match self.in_lambda {
            CodeState::InMain => self.main_code.position(),
            CodeState::InLambda => self.lambdas_code.position(),
        }
    }

    fn write_code(&mut self) -> &mut Code {
        match self.in_lambda {
            CodeState::InMain => &mut self.main_code,
            CodeState::InLambda => &mut self.lambdas_code,
        }
    }
}

pub(crate) fn generate_func_code<'a, L: Clone + Eq + core::hash::Hash>(
    state: &mut RewriteState<'a, L>,
    fundef: ir::FunDef,
) -> Result<FunDef, CompilationError> {
    let ir::FunDef {
        privacy: _,
        name,
        vars,
        body,
    } = fundef;

    let mut local = LocalBindings::new();
    local.scope_enter();

    let arity = vars
        .len()
        .try_into()
        .map(|n| CallArity(n))
        .map_err(|_| CompilationError::FunctionParamsMoreThanLimit(vars.len()))?;

    for (var_i, var) in vars.iter().enumerate() {
        let var_i = var_i
            .try_into()
            .map_err(|_| CompilationError::FunctionParamsMoreThanLimit(vars.len()))?;
        local.add_param(var.0.clone().unspan(), var_i);
    }

    let code_pos = state.get_instruction_address();
    generate_expression_code(state, &mut local, body.clone())?;

    let stack_size = local.scope_terminate();

    state.write_code().push(Instruction::Ret);
    Ok(FunDef {
        name,
        arity,
        code_pos,
        stack_size,
    })
}

fn generate_expression_code<'a, L: Clone + Eq + core::hash::Hash>(
    state: &mut RewriteState<'a, L>,
    local: &mut LocalBindings,
    expr: ir::Expr,
) -> Result<(), CompilationError> {
    match expr {
        ir::Expr::Literal(_span, lit) => {
            let lit_id = state.lits.add((state.params.literal_mapper)(lit)?);
            state.write_code().push(Instruction::PushLiteral(lit_id));
            Ok(())
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
            Ok(())
        }
        ir::Expr::List(_span, _l) => {
            todo!()
        }
        ir::Expr::Let(binder, body, in_expr) => {
            generate_expression_code(state, local, *body)?;
            match binder {
                ir::Binder::Ident(ident) => {
                    let bind = append_ident(local, &ident);
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
            generate_expression_code(state, local, *in_expr)?;
            Ok(())
        }
        ir::Expr::Field(expr, struct_ident, field_ident) => {
            let (constr_id, constr_def) = state
                .constrs
                .get(&NamespaceResolver::none(), &struct_ident.inner)
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

            generate_expression_code(state, local, *expr)?;
            state
                .write_code()
                .push(Instruction::AccessField(constr_id, index));
            Ok(())
        }
        ir::Expr::Lambda(_span, fundef) => {
            let prev = state.set_in_lambda();
            generate_func_code(state, *fundef)?;

            state.restore_codestate(prev);
            todo!()
        }
        ir::Expr::Call(_span, args) => {
            assert!(args.len() > 0);
            let len = args.len() - 1;
            for arg in args {
                generate_expression_code(state, local, arg)?;
            }
            state
                .write_code()
                .push(Instruction::Call(CallArity(len as u8)));
            Ok(())
        }
        ir::Expr::If {
            span: _,
            cond,
            then_expr,
            else_expr,
        } => {
            generate_expression_code(state, local, (*cond).unspan())?;

            let cond_jump_ref = state.write_code().push_temp();
            let cond_pos = state.get_instruction_address();

            local.scope_enter();
            generate_expression_code(state, local, (*then_expr).unspan())?;
            local.scope_leave();

            let jump_else_ref = state.write_code().push_temp();
            let else_pos = state.get_instruction_address();

            local.scope_enter();
            generate_expression_code(state, local, (*else_expr).unspan())?;
            local.scope_leave();

            let end_pos = state.get_instruction_address();

            state
                .write_code()
                .resolve_temp(cond_jump_ref, Instruction::CondJump(else_pos - cond_pos));
            state
                .write_code()
                .resolve_temp(jump_else_ref, Instruction::Jump(end_pos - else_pos));

            Ok(())
        }
    }
}

fn fetch_ident<'a, L: Clone + Eq + core::hash::Hash>(
    state: &RewriteState<'a, L>,
    local: &LocalBindings,
    span: Span,
    path: Path,
) -> Result<BindingType, CompilationError> {
    local
        .bindings
        .get(&path)
        .or_else(|| state.globals.get(&path))
        .map(|x| *x)
        .ok_or(CompilationError::MissingSymbol(span, path))
}

fn append_ident(local: &mut LocalBindings, ident: &Ident) -> LocalBindIndex {
    local.add_local(ident.clone())
}
