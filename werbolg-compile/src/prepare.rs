pub use crate::params::CompilationParams;

use crate::compile::{self, *};
pub use crate::defs::*;
use crate::resolver::SymbolResolver;
use crate::CompilationUnit;
use werbolg_core as ir;
use werbolg_core::{AbsPath, ConstrId, FunId, Namespace};

use crate::bindings::{BindingType, GlobalBindings};
pub use crate::environ::Environment;
pub use crate::errors::CompilationError;
use crate::symbols::{self, IdVecAfter, SymbolsTableData};

use alloc::{format, string::String, vec::Vec};
use hashbrown::HashMap;

/// State of compilation
pub struct CompilationState<L: Clone + Eq + core::hash::Hash> {
    params: CompilationParams<L>,
    funs: SymbolsTableData<FunId, (Namespace, ir::FunDef, ir::FunImpl)>,
    constrs: SymbolsTableData<ConstrId, ConstrDef>,
    namespaces: HashMap<Namespace, SymbolResolver>,
}

impl<L: Clone + Eq + core::hash::Hash> CompilationState<L> {
    /// Create a new compilation state
    pub fn new(params: CompilationParams<L>) -> Self {
        Self {
            params,
            funs: SymbolsTableData::new(),
            constrs: SymbolsTableData::new(),
            namespaces: HashMap::new(),
        }
    }

    /// Add a ir::module to the compilation state
    pub fn add_module(
        &mut self,
        namespace: &Namespace,
        module: ir::Module,
    ) -> Result<(), CompilationError> {
        let mut uses = Vec::new();
        self.funs.create_namespace(namespace.clone())?;
        self.constrs.create_namespace(namespace.clone())?;

        for stmt in module.statements.into_iter() {
            match stmt {
                ir::Statement::Use(u) => {
                    uses.push(u);
                }
                ir::Statement::Function(span, fundef, funimpl) => {
                    let ident = fundef.name.clone();
                    let path = AbsPath::new(namespace, &ident);
                    let _funid = self
                        .funs
                        .add(&path, (namespace.clone(), fundef, funimpl))
                        .ok_or_else(|| CompilationError::DuplicateSymbol(span, ident))?;
                    ()
                }
                ir::Statement::Struct(span, structdef) => {
                    let stru = StructDef {
                        name: structdef.name.unspan(),
                        fields: structdef.fields.into_iter().map(|v| v.unspan()).collect(),
                    };
                    let name = stru.name.clone();
                    let path = AbsPath::new(namespace, &name);
                    self.constrs
                        .add(&path, ConstrDef::Struct(stru))
                        .ok_or_else(|| CompilationError::DuplicateSymbol(span, name))?;
                }
                ir::Statement::Expr(_) => (),
            }
        }

        if self
            .namespaces
            .insert(
                namespace.clone(),
                SymbolResolver::new(namespace.clone(), uses),
            )
            .is_some()
        {
            return Err(CompilationError::NamespaceError(
                symbols::NamespaceError::Duplicate(namespace.clone()),
            ));
        }

        Ok(())
    }

    /// Finalize compilation and return a CompilationUnit containing all the modules compiled in the state
    pub fn finalize<N, G>(
        self,
        environ: &mut Environment<N, G>,
    ) -> Result<CompilationUnit<L>, CompilationError> {
        let SymbolsTableData { table, vecdata } = self.funs;

        /*
        for (p, _id) in table.to_vec(Namespace::root()) {
            std::println!("{:?}", p)
        }
        */

        let mut root_bindings = GlobalBindings::new();

        for (path, id) in environ.symbols.iter() {
            // unwrap is ok here, the environment should check for duplicate symbol and
            // missing namespace
            match id {
                crate::environ::EnvironmentId::Nif(nif_id) => root_bindings
                    .add(path.clone(), BindingType::Nif(nif_id))
                    .unwrap(),
                crate::environ::EnvironmentId::Global(global_id) => root_bindings
                    .add(path.clone(), BindingType::Global(global_id))
                    .unwrap(),
            }
        }

        for (path, fun_id) in table.iter() {
            root_bindings
                .add(path.clone(), BindingType::Fun(fun_id))
                .map_err(|()| {
                    CompilationError::DuplicateSymbolEnv(String::from("Fun"), path.clone())
                })?
        }

        // all modules share this compilation state
        let shared = CompilationSharedState {};

        let mut state = compile::CodeBuilder::new(
            &shared,
            self.params,
            table,
            IdVecAfter::new(vecdata.next_id()),
            root_bindings,
        );

        for (funid, (namespace, fundef, funimpl)) in vecdata.into_iter() {
            let Some(uses) = self.namespaces.get(&namespace) else {
                panic!("internal error: namespace not defined");
            };
            state.set_module_resolver(uses);

            let fun_name = fundef.name.clone();
            let lirdef = compile::generate_func_code(&mut state, &namespace, Some(fundef), funimpl)
                .map_err(|e| {
                    e.context(format!(
                        "namespace {:?} function code {:?}",
                        namespace, fun_name
                    ))
                })?;
            let lirid = state.funs_vec.push(lirdef);
            assert_eq!(funid, lirid)
        }

        // merge the lambdas vec with the main fun vec
        state.funs_vec.concat(&mut state.lambdas_vec);

        Ok(CompilationUnit {
            lits: state.lits.finalize(),
            constrs: state.constrs,
            funs: state.funs_vec,
            funs_tbl: state.funs_tbl,
            code: state.main_code.finalize(),
        })
    }
}
