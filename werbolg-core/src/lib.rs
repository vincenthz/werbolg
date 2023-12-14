//#![no_std]

extern crate alloc;

mod basic;
mod bindings;
pub mod code;
mod environ;
mod id;
mod ir;
mod location;
pub mod symbols;

pub mod lir;

pub use basic::*;
pub use code::{InstructionAddress, InstructionDiff};
pub use environ::{Environment, ValueFun};
pub use id::{ConstrId, FunId, GlobalId, Id, LitId, NifId};
pub use ir::*;
use lir::ParamBindIndex;
pub use location::*;

use alloc::boxed::Box;
use bindings::BindingsStack;
use symbols::{IdVec, IdVecAfter, SymbolsTable, SymbolsTableData, UniqueTableBuilder};

struct RewriteState {
    funs_tbl: SymbolsTable<FunId>,
    funs_vec: IdVec<FunId, lir::FunDef>,
    constrs: SymbolsTableData<ConstrId, lir::ConstrDef>,
    lits: UniqueTableBuilder<LitId, basic::Literal>,
    main_code: code::Code,
    lambdas: IdVecAfter<FunId, lir::FunDef>,
    lambdas_code: code::Code,
    in_lambda: CodeState,
    bindings: BindingsStack<BindingType>,
}

pub struct LocalBindings {
    bindings: BindingsStack<BindingType>,
    local: Vec<u32>,
    max_local: u32,
}

impl LocalBindings {
    pub fn new() -> Self {
        Self {
            bindings: BindingsStack::new(),
            local: vec![0],
            max_local: 0,
        }
    }

    pub fn add_param(&mut self, ident: Ident, n: u32) {
        self.bindings
            .add(ident, BindingType::Param(ParamBindIndex(n)))
    }

    pub fn add_local(&mut self, ident: Ident) -> lir::LocalBindIndex {
        match self.local.last_mut() {
            None => panic!("cannot happen"),
            Some(x) => {
                let local = *x;
                *x += 1;

                let local = lir::LocalBindIndex(local as u32);
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
    Global(id::GlobalId),
    Fun(id::FunId),
    Param(lir::ParamBindIndex),
    Local(lir::LocalBindIndex),
}

#[derive(Clone, Copy, Default)]
pub enum CodeState {
    #[default]
    InMain,
    InLambda,
}

impl RewriteState {
    #[must_use = "code state need to be restore using restore_codestate"]
    fn set_in_lambda(&mut self) -> CodeState {
        let saved = self.in_lambda;
        self.in_lambda = CodeState::InLambda;
        saved
    }

    fn restore_codestate(&mut self, code_state: CodeState) {
        self.in_lambda = code_state;
    }

    fn get_instruction_address(&self) -> code::InstructionAddress {
        match self.in_lambda {
            CodeState::InMain => self.main_code.position(),
            CodeState::InLambda => self.lambdas_code.position(),
        }
    }

    fn write_code(&mut self) -> &mut code::Code {
        match self.in_lambda {
            CodeState::InMain => &mut self.main_code,
            CodeState::InLambda => &mut self.lambdas_code,
        }
    }
}

#[derive(Debug)]
pub enum CompilationError {
    DuplicateSymbol(Ident),
    MissingSymbol(Span, Ident),
}

/// Compile a IR Module into an optimised-for-execution LIR Module
pub fn compile(
    module: ir::Module,
    environ: &mut Environment,
) -> Result<lir::Module, CompilationError> {
    let mut funs = SymbolsTableData::new();
    let mut constrs = SymbolsTableData::new();

    for stmt in module.statements.into_iter() {
        match stmt {
            ir::Statement::Function(_span, fundef) => {
                alloc_fun(&mut funs, fundef)?;
            }
            ir::Statement::Struct(_span, structdef) => {
                alloc_struct(&mut constrs, structdef)?;
            }
            ir::Statement::Expr(_) => (),
        }
    }

    let SymbolsTableData { table, vecdata } = funs;

    let mut bindings = BindingsStack::new();
    for (_id, (ident, idx)) in environ.symbols.vecdata.iter() {
        bindings.add(ident.clone(), BindingType::Global(*idx))
    }

    for (ident, fun_id) in table.iter() {
        bindings.add(ident.clone(), BindingType::Fun(fun_id))
    }

    let mut state = RewriteState {
        funs_tbl: table,
        funs_vec: IdVec::new(),
        main_code: code::Code::new(),
        lambdas: IdVecAfter::new(vecdata.next_id()),
        lambdas_code: code::Code::new(),
        constrs: SymbolsTableData::new(),
        lits: UniqueTableBuilder::new(),
        in_lambda: CodeState::default(),
        bindings,
    };

    for (funid, fundef) in vecdata.into_iter() {
        let lirdef = rewrite_fun(&mut state, fundef)?;
        let lirid = state.funs_vec.push(lirdef);
        assert_eq!(funid, lirid)
    }

    // merge the lambdas code with the main code
    // also remap the fundef of all lambdas to include this new offset
    let lambda_instruction_diff = state.main_code.merge(state.lambdas_code);
    state
        .lambdas
        .remap(|fundef| fundef.code_pos += lambda_instruction_diff);

    state.funs_vec.concat(&mut state.lambdas);
    let funs = state.funs_vec;

    Ok(lir::Module {
        lits: state.lits.finalize(),
        constrs: state.constrs,
        funs: funs,
        funs_tbl: state.funs_tbl,
        code: state.main_code.finalize(),
    })
}

fn rewrite_fun(state: &mut RewriteState, fundef: FunDef) -> Result<lir::FunDef, CompilationError> {
    let FunDef { name, vars, body } = fundef;

    let mut local = LocalBindings::new();

    for (var_i, var) in vars.iter().enumerate() {
        local.add_param(var.0.clone().unspan(), var_i as u32);
    }

    let code_pos = state.get_instruction_address();
    rewrite_expr2(state, &mut local, body.clone())?;

    let lir_vars = vars.into_iter().map(|v| lir::Variable(v.0)).collect();

    let lir_body = rewrite_expr(state, body)?;
    state.write_code().push(lir::Statement::Ret);
    Ok(lir::FunDef {
        name,
        vars: lir_vars,
        body: lir_body,
        code_pos,
        stack_size: lir::LocalStackSize(local.max_local as u32),
    })
}

fn alloc_fun(
    state: &mut SymbolsTableData<FunId, FunDef>,
    fundef: FunDef,
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

fn alloc_struct(
    state: &mut SymbolsTableData<ConstrId, lir::ConstrDef>,
    StructDef { name, fields }: StructDef,
) -> Result<ConstrId, CompilationError> {
    let stru = lir::StructDef {
        name: name.unspan(),
        fields: fields.into_iter().map(|v| v.unspan()).collect(),
    };
    let name = stru.name.clone();
    state
        .add(name.clone(), lir::ConstrDef::Struct(stru))
        .ok_or_else(|| CompilationError::DuplicateSymbol(name))
}

fn rewrite_expr(state: &mut RewriteState, expr: Expr) -> Result<lir::Expr, CompilationError> {
    match expr {
        Expr::Literal(span, lit) => {
            let lit_id = state.lits.add(lit);
            Ok(lir::Expr::Literal(span, lit_id))
        }
        Expr::List(span, l) => {
            let l = l
                .into_iter()
                .map(|e| rewrite_expr(state, e))
                .collect::<Result<_, _>>()?;
            Ok(lir::Expr::List(span, l))
        }
        Expr::Let(binder, body, in_expr) => Ok(lir::Expr::Let(
            rewrite_binder(binder),
            rewrite_boxed_expr(state, body)?,
            rewrite_boxed_expr(state, in_expr)?,
        )),
        Expr::Field(expr, ident) => {
            let expr = rewrite_boxed_expr(state, expr)?;
            Ok(lir::Expr::Field(expr, ident))
        }
        Expr::Ident(span, ident) => Ok(lir::Expr::Ident(span, ident)),
        Expr::Lambda(span, fundef) => {
            let lirdef = rewrite_fun(state, *fundef)?;
            let lambda_id = state.lambdas.push(lirdef);
            Ok(lir::Expr::Lambda(span, lambda_id))
        }
        Expr::Call(span, args) => {
            let args = args
                .into_iter()
                .map(|e| rewrite_expr(state, e))
                .collect::<Result<_, _>>()?;
            Ok(lir::Expr::Call(span, args))
        }
        Expr::If {
            span,
            cond,
            then_expr,
            else_expr,
        } => Ok(lir::Expr::If {
            span,
            cond: rewrite_boxed_sexpr(state, cond)?,
            then_expr: rewrite_boxed_sexpr(state, then_expr)?,
            else_expr: rewrite_boxed_sexpr(state, else_expr)?,
        }),
    }
}

fn rewrite_boxed_expr(
    state: &mut RewriteState,
    expr: Box<Expr>,
) -> Result<Box<lir::Expr>, CompilationError> {
    Ok(Box::new(rewrite_expr(state, expr.as_ref().clone())?))
}

fn rewrite_boxed_sexpr(
    state: &mut RewriteState,
    expr: Box<Spanned<Expr>>,
) -> Result<Box<Spanned<lir::Expr>>, CompilationError> {
    let span = expr.span.clone();
    Ok(Box::new(Spanned {
        span,
        inner: rewrite_expr(state, expr.as_ref().clone().unspan())?,
    }))
}

fn rewrite_binder(binder: Binder) -> lir::Binder {
    match binder {
        Binder::Unit => lir::Binder::Unit,
        Binder::Ignore => lir::Binder::Ignore,
        Binder::Ident(i) => lir::Binder::Ident(i),
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

fn append_ident(local: &mut LocalBindings, ident: &Ident) -> lir::LocalBindIndex {
    local.add_local(ident.clone())
}

fn rewrite_expr2(
    state: &mut RewriteState,
    local: &mut LocalBindings,
    expr: Expr,
) -> Result<(), CompilationError> {
    match expr {
        Expr::Literal(_span, lit) => {
            let lit_id = state.lits.add(lit);
            state.write_code().push(lir::Statement::PushLiteral(lit_id));
            Ok(())
        }
        Expr::Ident(span, ident) => {
            let x = fetch_ident(state, local, span, ident.clone())?;
            match x {
                BindingType::Global(idx) => {
                    state.write_code().push(lir::Statement::FetchGlobal(idx));
                }
                BindingType::Fun(idx) => {
                    state.write_code().push(lir::Statement::FetchFun(idx));
                }
                BindingType::Local(idx) => {
                    state
                        .write_code()
                        .push(lir::Statement::FetchStackLocal(idx));
                }
                BindingType::Param(idx) => {
                    state
                        .write_code()
                        .push(lir::Statement::FetchStackParam(idx));
                }
            }
            Ok(())
        }
        Expr::List(_span, l) => {
            todo!()
        }
        Expr::Let(binder, body, in_expr) => {
            rewrite_expr2(state, local, *body)?;
            match binder {
                Binder::Ident(ident) => {
                    let bind = append_ident(local, &ident);
                    state.write_code().push(lir::Statement::LocalBind(bind));
                }
                Binder::Ignore => {
                    state.write_code().push(lir::Statement::IgnoreOne);
                }
                Binder::Unit => {
                    // TODO, not sure ignore one is the best to do here
                    state.write_code().push(lir::Statement::IgnoreOne);
                }
            }
            rewrite_expr2(state, local, *in_expr)?;
            Ok(())
        }
        Expr::Field(expr, ident) => {
            rewrite_expr2(state, local, *expr)?;
            state.write_code().push(lir::Statement::AccessField(ident));
            Ok(())
        }
        Expr::Lambda(_span, fundef) => {
            let prev = state.set_in_lambda();
            rewrite_fun(state, *fundef)?;

            state.restore_codestate(prev);
            todo!()
        }
        Expr::Call(_span, args) => {
            assert!(args.len() > 0);
            let len = args.len() - 1;
            for arg in args {
                rewrite_expr2(state, local, arg)?;
            }
            state
                .write_code()
                .push(lir::Statement::Call(lir::CallArity(len as u32)));
            Ok(())
        }
        Expr::If {
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
                .resolve_temp(cond_jump_ref, lir::Statement::CondJump(else_pos - cond_pos));
            state
                .write_code()
                .resolve_temp(jump_else_ref, lir::Statement::Jump(end_pos - else_pos));

            Ok(())
        }
    }
}
