use super::location::Location;
use super::value::Value;
use alloc::vec::Vec;
use werbolg_core::{lir, Ident};

pub struct ExecutionStack<'m> {
    pub values: Vec<Value>,
    pub work: Vec<Work<'m>>,
    pub constr: Vec<ExecutionAtom<'m>>,
}

pub enum Work<'m> {
    Empty,
    One(&'m lir::Expr),
    Many(&'m [lir::Expr]),
}

impl<'m> ExecutionStack<'m> {
    pub fn new() -> Self {
        ExecutionStack {
            values: Vec::new(),
            work: Vec::new(),
            constr: Vec::new(),
        }
    }

    pub fn push_work1(&mut self, constr: ExecutionAtom<'m>, expr: &'m lir::Expr) {
        self.work.push(Work::One(expr));
        self.constr.push(constr);
    }

    pub fn push_work(&mut self, constr: ExecutionAtom<'m>, exprs: &'m [lir::Expr]) {
        if exprs.len() == 0 {
            self.work.push(Work::Empty)
        } else if exprs.len() == 1 {
            self.work.push(Work::One(&exprs[0]))
        } else {
            self.work.push(Work::Many(exprs));
        }
        self.constr.push(constr);
    }

    pub fn push_value(&mut self, value: Value) {
        self.values.push(value)
    }

    pub fn next_work(&mut self) -> ExecutionNext<'m> {
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
            Some(Work::Empty) => {
                let constr = self.constr.pop().unwrap();
                let nb_args = constr.arity();
                let args = pop_end_rev(&mut self.values, nb_args);
                ExecutionNext::Reduce(constr, args)
            }
            Some(Work::One(e)) => {
                self.work.push(Work::Empty);
                ExecutionNext::Shift(e)
            }
            Some(Work::Many(es)) => {
                if let Some((last, previous)) = es.split_last() {
                    if previous.len() > 1 {
                        self.work.push(Work::Many(previous));
                    } else {
                        self.work.push(Work::One(&previous[0]));
                    }
                    ExecutionNext::Shift(last)
                } else {
                    panic!("internal error: empty work for many")
                }
            }
        }
    }
}

pub enum ExecutionAtom<'m> {
    List(usize),
    Field(&'m Ident),
    ThenElse(&'m lir::Expr, &'m lir::Expr),
    Call(usize, Location),
    Let(lir::Binder, &'m lir::Expr),
    PopScope,
}

impl<'m> ExecutionAtom<'m> {
    pub fn arity(&self) -> usize {
        match self {
            ExecutionAtom::List(u) => *u,
            ExecutionAtom::Field(_) => 1,
            ExecutionAtom::ThenElse(_, _) => 1,
            ExecutionAtom::Call(u, _) => *u,
            ExecutionAtom::Let(_, _) => 1,
            ExecutionAtom::PopScope => 1,
        }
    }
}

pub enum ExecutionNext<'m> {
    Shift(&'m lir::Expr),
    Reduce(ExecutionAtom<'m>, Vec<Value>),
    Finish(Value),
}
