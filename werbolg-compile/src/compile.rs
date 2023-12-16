use super::bindings::BindingsStack;
use super::code::*;
use super::defs::*;
use super::instructions::*;
use super::symbols::*;
use super::CompilationError;
use werbolg_core as ir;
use werbolg_core::{ConstrId, FunId, GlobalId, Ident, LitId, Literal, NifId, Span};

pub(crate) struct RewriteState {
    pub(crate) funs_tbl: SymbolsTable<FunId>,
    pub(crate) funs_vec: IdVec<FunId, FunDef>,
    pub(crate) constrs: SymbolsTableData<ConstrId, ConstrDef>,
    pub(crate) lits: UniqueTableBuilder<LitId, Literal>,
    pub(crate) main_code: Code,
    pub(crate) lambdas: IdVecAfter<FunId, FunDef>,
    pub(crate) lambdas_code: Code,
    pub(crate) in_lambda: CodeState,
    pub(crate) bindings: BindingsStack<BindingType>,
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
            None => panic!("cannot happen"),
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
}

#[derive(Clone, Copy)]
pub enum BindingType {
    #[allow(unused)]
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

impl RewriteState {
    pub fn new(
        funs_tbl: SymbolsTable<FunId>,
        lambdas: IdVecAfter<FunId, FunDef>,
        bindings: BindingsStack<BindingType>,
    ) -> Self {
        Self {
            funs_tbl,
            funs_vec: IdVec::new(),
            main_code: Code::new(),
            lambdas,
            lambdas_code: Code::new(),
            constrs: SymbolsTableData::new(),
            lits: UniqueTableBuilder::new(),
            in_lambda: CodeState::default(),
            bindings,
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

pub(crate) fn alloc_fun(
    state: &mut SymbolsTableData<FunId, ir::FunDef>,
    fundef: ir::FunDef,
) -> Result<FunId, CompilationError> {
    let ident = fundef.name.clone();
    if let Some(ident) = ident {
        state
            .add(ident.clone(), fundef)
            .ok_or_else(|| CompilationError::DuplicateSymbol(ident))
    } else {
        Ok(state.add_anon(fundef))
    }
}

pub(crate) fn alloc_struct(
    state: &mut SymbolsTableData<ConstrId, ConstrDef>,
    ir::StructDef { name, fields }: ir::StructDef,
) -> Result<ConstrId, CompilationError> {
    let stru = StructDef {
        name: name.unspan(),
        fields: fields.into_iter().map(|v| v.unspan()).collect(),
    };
    let name = stru.name.clone();
    state
        .add(name.clone(), ConstrDef::Struct(stru))
        .ok_or_else(|| CompilationError::DuplicateSymbol(name))
}

pub(crate) fn rewrite_fun(
    state: &mut RewriteState,
    fundef: ir::FunDef,
) -> Result<FunDef, CompilationError> {
    let ir::FunDef { name, vars, body } = fundef;

    let mut local = LocalBindings::new();

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
    rewrite_expr2(state, &mut local, body.clone())?;

    local.scope_leave();

    state.write_code().push(Instruction::Ret);
    Ok(FunDef {
        name,
        arity,
        code_pos,
        stack_size: LocalStackSize(local.max_local as u32),
    })
}

fn rewrite_expr2(
    state: &mut RewriteState,
    local: &mut LocalBindings,
    expr: ir::Expr,
) -> Result<(), CompilationError> {
    match expr {
        ir::Expr::Literal(_span, lit) => {
            let lit_id = state.lits.add(lit);
            state.write_code().push(Instruction::PushLiteral(lit_id));
            Ok(())
        }
        ir::Expr::Ident(span, ident) => {
            let x = fetch_ident(state, local, span, ident.clone())?;
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
            rewrite_expr2(state, local, *body)?;
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
            rewrite_expr2(state, local, *in_expr)?;
            Ok(())
        }
        ir::Expr::Field(expr, ident) => {
            rewrite_expr2(state, local, *expr)?;
            //state.write_code().push(Instruction::AccessField(ident));
            todo!()
            //Ok(())
        }
        ir::Expr::Lambda(_span, fundef) => {
            let prev = state.set_in_lambda();
            rewrite_fun(state, *fundef)?;

            state.restore_codestate(prev);
            todo!()
        }
        ir::Expr::Call(_span, args) => {
            assert!(args.len() > 0);
            let len = args.len() - 1;
            for arg in args {
                rewrite_expr2(state, local, arg)?;
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
            rewrite_expr2(state, local, (*cond).unspan())?;

            let cond_jump_ref = state.write_code().push_temp();
            let cond_pos = state.get_instruction_address();

            local.scope_enter();
            rewrite_expr2(state, local, (*then_expr).unspan())?;
            local.scope_leave();

            let jump_else_ref = state.write_code().push_temp();
            let else_pos = state.get_instruction_address();

            local.scope_enter();
            rewrite_expr2(state, local, (*else_expr).unspan())?;
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

fn fetch_ident(
    state: &RewriteState,
    local: &LocalBindings,
    span: Span,
    ident: Ident,
) -> Result<BindingType, CompilationError> {
    local
        .bindings
        .get(&ident)
        .or_else(|| state.bindings.get(&ident))
        .map(|x| *x)
        .ok_or(CompilationError::MissingSymbol(span, ident))
}

fn append_ident(local: &mut LocalBindings, ident: &Ident) -> LocalBindIndex {
    local.add_local(ident.clone())
}
