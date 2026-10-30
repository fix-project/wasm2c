use anyhow::{Result, bail};
use itertools::Itertools;
use smallvec::SmallVec;
use std::{
    fmt::{Display, Formatter},
    io::Write,
};
use wasmparser::{
    BlockType, Frame, FrameKind, FrameStack, FuncValidator, Operator, ValType, VisitOperator,
    WasmModuleResources, for_each_visit_operator,
};

#[derive(Copy, Clone)]
struct Operand(usize);

impl Display for Operand {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(out, "s{}", self.0)
    }
}

pub(crate) struct ExpressionPrinter<'w, W> {
    stack: Vec<Operand>,
    pending: Vec<ValType>,
    push_counter: usize,
    out: &'w mut W,
}

struct OpInfo {
    params: SmallVec<[Operand; 2]>,
    frame: Frame,
}

// Implementations of a few operators.
// The "implement_some_operators" macro (below) controls the ones we need to implement.

// TODO: use a better writer abstraction so we are not interpolating strings so closely
impl<'w, W: Write> ExpressionPrinter<'w, W> {
    fn visit_i32_const(&mut self, value: i32, _info: &OpInfo) -> Result<()> {
        self.push(value)
    }

    fn visit_i64_const(&mut self, value: i64, _info: &OpInfo) -> Result<()> {
        self.push(value)
    }

    fn visit_i32_add(&mut self, info: &OpInfo) -> Result<()> {
        self.push(format!("{} + {}", info.params[0], info.params[1]))
    }

    fn visit_drop(&mut self, _info: &OpInfo) -> Result<()> {
        Ok(())
    }

    fn visit_end(&mut self, info: &OpInfo) -> Result<()> {
        self.cross_frame(&info.params);

        match info.frame.kind {
            FrameKind::Block => {
                // TODO: write label with unique name (or names-section-assigned name)
                writeln!(self.out, "block{}:", info.frame.height)?;

                Ok(())
            }
            kind => bail!("exit on {kind:?} is not implemented"),
        }
    }

    pub(crate) fn finish(&mut self) -> Result<()> {
        // Function end
        debug_assert!(self.pending.is_empty());
        match self.stack.as_slice() {
            [] => {} // no need to do anything at the end of the function
            [param] => writeln!(self.out, "return {};", param)?,
            params => writeln!(
                self.out,
                "return {{{}}};",
                params.iter().rev().map(ToString::to_string).format(", ")
            )?,
        }
        Ok(())
    }

    fn visit_block(&mut self, _ty: BlockType, info: &OpInfo) -> Result<()> {
        // Re-push `params` again
        self.cross_frame(&info.params);

        Ok(())
    }

    pub fn new(out: &'w mut W) -> Self {
        Self {
            stack: vec![],
            pending: vec![],
            push_counter: 0,
            out,
        }
    }

    pub(crate) fn visitor<'a, T: WasmModuleResources>(
        &'a mut self,
        validator: &'a mut FuncValidator<T>,
        offset: u64,
    ) -> OperatorVisitor<'a, 'w, T, W> {
        OperatorVisitor {
            validator,
            offset,
            printer: self,
        }
    }

    fn push(&mut self, expr: impl Display) -> Result<()> {
        let ty = self.pending.pop().unwrap();
        let top_of_stack = self.stack.push_mut(Operand(self.push_counter));
        writeln!(self.out, "{ty} {top_of_stack} = {expr};")?;
        self.push_counter += 1;

        Ok(())
    }

    fn cross_frame(&mut self, params: &[Operand]) {
        assert_eq!(params.len(), self.pending.len());

        // Restore params that were automatically popped
        self.stack.extend_from_slice(params);
        self.pending.clear();
    }
}

macro_rules! operator {
    // Implemented operators can be added here incrementally.
    ($self:tt $info:tt I32Const $($t:tt)*) => { operator!(@forward $self $info $($t)*) };
    ($self:tt $info:tt I64Const $($t:tt)*) => { operator!(@forward $self $info $($t)*) };
    ($self:tt $info:tt I32Add   $($t:tt)*) => { operator!(@forward $self $info $($t)*) };
    ($self:tt $info:tt Drop     $($t:tt)*) => { operator!(@forward $self $info $($t)*) };
    ($self:tt $info:tt End      $($t:tt)*) => { operator!(@forward $self $info $($t)*) };
    ($self:tt $info:tt Block    $($t:tt)*) => { operator!(@forward $self $info $($t)*) };

    (@forward $self:tt $info:tt $({ $($arg:ident),* })? => $visit:ident) =>
    {{ $self.printer.$visit($($($arg,)*)? &$info)?;
       if !$self.printer.pending.is_empty() {
           panic!("visitor {} did not push enough operands", stringify!($visit));
       }
       Ok(())
    }};

    // Bail out gracefully on any other operator.
    ($self:tt $info:ident $op:ident $($t:tt)*)
        => {{ let _ = $info; bail!("operator {} unimplemented", stringify!($op)); }};
}

pub struct OperatorVisitor<'a, 'w, T, W> {
    validator: &'a mut FuncValidator<T>,
    offset: u64,
    printer: &'a mut ExpressionPrinter<'w, W>,
}

impl<'w, T: WasmModuleResources, W> FrameStack for OperatorVisitor<'_, 'w, T, W> {
    fn current_frame(&self) -> Option<FrameKind> {
        self.validator.get_control_frame(0).map(|frame| frame.kind)
    }
}

macro_rules! define_forward {
    ($( @$proposal:ident $op:ident $({ $($arg:ident: $argty:ty),* })? => $visit:ident ($($ann:tt)*) )*) => {
        $(
            fn $visit(&mut self $($(, $arg: $argty)*)?) -> Result<()> {
                let operator = wasmparser::Operator::$op $({ $($arg: $arg.clone()),* })?;
		let info = self.op(&operator)?;
                operator!(self info $op $({ $($arg),* })? => $visit)
            }
        )*
    };
}

impl<'a, 'w, T: WasmModuleResources, W> OperatorVisitor<'a, 'w, T, W> {
    fn op(&mut self, op: &Operator) -> Result<OpInfo> {
        // Collect arity, top control frame, and params before validation changes the validator's state.
        let Some((num_params, num_results)) = op.operator_arity(self.validator) else {
            self.validator.op(self.offset, op)?; // report validation error
            panic!("operator valid but missing arity");
        };
        let num_params: usize = num_params.try_into().unwrap();
        let num_results: usize = num_results.try_into().unwrap();
        let frame = self.validator.get_control_frame(0).copied().expect("frame");

        // TODO: handle stack polymorphism in unreachable code
        let params = (0..num_params)
            .map(|_| self.printer.stack.pop().unwrap())
            .collect();

        // validate the operator
        self.validator.op(self.offset, op)?;

        // collect results
        for i in 0..num_results {
            self.printer
                .pending
                .push(self.validator.get_operand_type(i).unwrap().unwrap());
        }

        Ok(OpInfo { params, frame })
    }
}

impl<'a, 'w, T, W: Write> VisitOperator<'a> for OperatorVisitor<'_, 'w, T, W>
where
    T: WasmModuleResources,
{
    type Output = Result<()>;
    for_each_visit_operator!(define_forward);
}
