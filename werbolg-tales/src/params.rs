pub struct TalesParams {
    pub dump_ir: bool,
    pub dump_instr: bool,
    pub exec_step_trace: bool,
    pub step_address: Vec<u64>,
    pub frontend: Option<Frontend>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frontend {
    Rusty,
    Lispy,
}
