//#![no_std]

extern crate alloc;

mod basic;
mod code;
mod environ;
mod id;
mod ir;
mod location;
mod symbols;

pub mod lir;

pub use basic::*;
pub use id::{ConstrId, FunId, Id, LitId};
pub use ir::*;
pub use location::*;

use alloc::boxed::Box;
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
}

/// Compile a IR Module into an optimised-for-execution LIR Module
pub fn compile(module: ir::Module) -> Result<lir::Module, CompilationError> {
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

    let mut state = RewriteState {
        funs_tbl: table,
        funs_vec: IdVec::new(),
        main_code: code::Code::new(),
        lambdas: IdVecAfter::new(vecdata.next_id()),
        lambdas_code: code::Code::new(),
        constrs: SymbolsTableData::new(),
        lits: UniqueTableBuilder::new(),
        in_lambda: CodeState::default(),
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
        code: state.main_code,
    })
}

fn rewrite_fun(state: &mut RewriteState, fundef: FunDef) -> Result<lir::FunDef, CompilationError> {
    let FunDef { name, vars, body } = fundef;

    let code_pos = state.get_instruction_address();
    let lir_vars = vars.into_iter().map(|v| lir::Variable(v.0)).collect();
    rewrite_expr2(state, body.clone())?;
    let lir_body = rewrite_expr(state, body)?;
    Ok(lir::FunDef {
        name,
        vars: lir_vars,
        body: lir_body,
        code_pos,
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

fn rewrite_expr2(state: &mut RewriteState, expr: Expr) -> Result<(), CompilationError> {
    match expr {
        Expr::Literal(_span, lit) => {
            let lit_id = state.lits.add(lit);
            state.write_code().push(lir::Statement::PushLiteral(lit_id));
            Ok(())
        }
        Expr::Ident(_span, ident) => {
            state.write_code().push(lir::Statement::FetchIdent(ident));
            Ok(())
        }
        Expr::List(span, l) => {
            todo!()
        }
        Expr::Let(binder, body, in_expr) => {
            rewrite_boxed_expr2(state, body)?;
            todo!()
        }
        Expr::Field(expr, ident) => {
            todo!()
        }
        Expr::Lambda(span, fundef) => {
            let prev = state.set_in_lambda();
            //rewrite_fun();

            state.restore_codestate(prev);
            todo!()
        }
        Expr::Call(span, args) => {
            assert!(args.len() > 0);
            let len = args.len() - 1;
            for arg in args {
                rewrite_expr2(state, arg)?;
            }
            state.write_code().push(lir::Statement::Call(len));
            Ok(())
        }
        Expr::If {
            span,
            cond,
            then_expr,
            else_expr,
        } => {
            todo!()
        }
    }
}

fn rewrite_boxed_expr2(state: &mut RewriteState, expr: Box<Expr>) -> Result<(), CompilationError> {
    Ok(rewrite_expr2(state, expr.as_ref().clone())?)
}
