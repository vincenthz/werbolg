use super::location::Location;
use super::value::Value;
use alloc::{vec, vec::Vec};
use werbolg_core as ir;

pub struct ExecutionStack {
    pub values: Vec<Value>,
    pub work: Vec<Work>,
    pub constr: Vec<ExecutionAtom>,
}

pub struct Work(Vec<ir::Expr>);

impl ExecutionStack {
    pub fn new() -> Self {
        ExecutionStack {
            values: Vec::new(),
            work: Vec::new(),
            constr: Vec::new(),
        }
    }

    pub fn push_work1(&mut self, constr: ExecutionAtom, expr: &ir::Expr) {
        self.work.push(Work(vec![expr.clone()]));
        self.constr.push(constr);
    }

    pub fn push_work(&mut self, constr: ExecutionAtom, exprs: &Vec<ir::Expr>) {
        assert!(!exprs.is_empty());
        self.work.push(Work(exprs.clone()));
        self.constr.push(constr);
    }

    pub fn push_value(&mut self, value: Value) {
        self.values.push(value)
    }

    pub fn next_work(&mut self) -> ExecutionNext {
        fn pop_end_rev<T>(v: &mut Vec<T>, mut nb: usize) -> Vec<T> {
            if nb > v.len() {
                panic!(
                    "pop_end_rev: trying to get {} values, but {} found",
                    nb,
                    v.len()
                );
            }
            let mut ret = Vec::with_capacity(nb);
            while nb > 0 {
                ret.push(v.pop().unwrap());
                nb -= 1;
            }
            ret
        }

        match self.work.pop() {
            None => {
                let val = self.values.pop().expect("one value if no expression left");
                assert!(self.values.is_empty());
                assert!(self.constr.is_empty());
                ExecutionNext::Finish(val)
            }
            Some(mut exprs) => {
                if exprs.0.is_empty() {
                    let constr = self.constr.pop().unwrap();
                    let nb_args = constr.arity();
                    let args = pop_end_rev(&mut self.values, nb_args);
                    ExecutionNext::Reduce(constr, args)
                } else {
                    let x = exprs.0.pop().unwrap();
                    self.work.push(Work(exprs.0));
                    ExecutionNext::Shift(x)
                }
            }
        }
    }
}

pub enum ExecutionAtom {
    List(usize),
    ThenElse(ir::Expr, ir::Expr),
    Call(usize, Location),
    Let(ir::Binder, ir::Expr),
    PopScope,
}

impl ExecutionAtom {
    pub fn arity(&self) -> usize {
        match self {
            ExecutionAtom::List(u) => *u,
            ExecutionAtom::ThenElse(_, _) => 1,
            ExecutionAtom::Call(u, _) => *u,
            ExecutionAtom::Let(_, _) => 1,
            ExecutionAtom::PopScope => 1,
        }
    }
}

pub enum ExecutionNext {
    Shift(ir::Expr),
    Reduce(ExecutionAtom, Vec<Value>),
    Finish(Value),
}
