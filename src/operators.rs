use anyhow::{Result, bail, ensure};
use itertools::Itertools;
use smallvec::SmallVec;
use std::{
    fmt::{Display, Formatter},
    io::Write,
};
use wasmparser::{
    FrameKind, FrameStack, ModuleArity, ValType, VisitOperator, for_each_visit_operator,
};

struct Operand(usize);

impl Display for Operand {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(out, "s{}", self.0)
    }
}

pub(crate) struct ExpressionPrinter<'w, W> {
    counter: usize,
    stack: Vec<Operand>,
    frame_depth: usize, // TODO: increment on block/etc.
    out: &'w mut W,
}

// Implementations of a few operators.
// The "implement_some_operators" macro (below) controls the ones we need to implement.

// TODO: use a better writer abstraction so we are not interpolating strings so closely
impl<'w, W: Write> ExpressionPrinter<'w, W> {
    fn visit_i32_const(&mut self, value: i32, _params: &[Operand]) -> Result<()> {
        self.push(&ValType::I32, value)
    }

    fn visit_i64_const(&mut self, value: i64, _params: &[Operand]) -> Result<()> {
        self.push(&ValType::I64, value)
    }

    fn visit_i32_add(&mut self, params: &[Operand]) -> Result<()> {
        self.push(&ValType::I32, format!("{} + {}", params[0], params[1]))
    }

    fn visit_drop(&mut self, _params: &[Operand]) -> Result<()> {
        Ok(())
    }

    fn visit_end(&mut self, params: &[Operand]) -> Result<()> {
        ensure!(self.frame_depth == 0, "non-function end unimplemented");
        debug_assert!(self.stack.is_empty());
        if params.is_empty() {
            // no need to do anything at the end of the function
        } else if params.len() == 1 {
            writeln!(self.out, "return {};", params[0])?;
        } else {
            writeln!(
                self.out,
                "return {{{}}};",
                params.iter().rev().map(ToString::to_string).format(", ")
            )?;
        }
        self.counter += params.len(); // "end" operand formally pushes params to outer frame
        Ok(())
    }

    pub fn new(out: &'w mut W) -> Self {
        Self {
            counter: 0,
            stack: vec![],
            frame_depth: 0,
            out,
        }
    }

    pub(crate) fn visitor<V: ModuleArity>(
        &mut self,
        validator: V,
    ) -> OperatorVisitor<'_, 'w, V, W> {
        OperatorVisitor {
            validator,
            stack: self,
        }
    }

    fn push(&mut self, ty: &ValType, expr: impl Display) -> Result<()> {
        let top_of_stack = self.stack.push_mut(Operand(self.counter));
        writeln!(self.out, "{ty} {top_of_stack} = {expr};")?;
        self.counter += 1;
        Ok(())
    }

    fn pop(&mut self) -> Operand {
        self.stack.pop().unwrap()
    }
}

macro_rules! implement_some_operators {
    // Implemented operators can be added here incrementally.
    ($self:tt $arity:tt I32Const $($t:tt)*) => { implement_some_operators!(@forward $self $arity $($t)*) };
    ($self:tt $arity:tt I64Const $($t:tt)*) => { implement_some_operators!(@forward $self $arity $($t)*) };
    ($self:tt $arity:tt I32Add   $($t:tt)*) => { implement_some_operators!(@forward $self $arity $($t)*) };
    ($self:tt $arity:tt Drop     $($t:tt)*) => { implement_some_operators!(@forward $self $arity $($t)*) };
    ($self:tt $arity:tt End      $($t:tt)*) => { implement_some_operators!(@forward $self $arity $($t)*) };

    (@forward $self:tt $arity:tt $({ $($arg:ident),* })? => $visit:ident) => {{
        let (num_params, num_results) = $arity.expect("arity for valid operator");
        let params: SmallVec<[Operand;2]> = (0..num_params).map(|_| $self.pop()).collect();
        let push_count_pre = $self.counter;
        $self.$visit($($($arg,)*)? &params)?;
        debug_assert_eq!($self.counter - push_count_pre, num_results as usize,
                         "push count for {} (left) did not match arity (right)", stringify!($visit));
	Ok(())
    }};

    // Bail out gracefully on any other operator.
    ($self:tt $arity:tt $op:ident $($t:tt)*) => {{ let _ = $arity;
						   bail!("operator {} unimplemented", stringify!($op)); }};
}

pub struct OperatorVisitor<'a, 'w, V, W> {
    validator: V,
    stack: &'a mut ExpressionPrinter<'w, W>,
}

impl<'w, V: FrameStack, W> FrameStack for OperatorVisitor<'_, 'w, V, W> {
    fn current_frame(&self) -> Option<FrameKind> {
        self.validator.current_frame()
    }
}

// Pair the ExpressionPrinter with a FuncValidator visitor to validate each operator first.
macro_rules! define_forward {
    ($( @$proposal:ident $op:ident $({ $($arg:ident: $argty:ty),* })? => $visit:ident ($($ann:tt)*) )*) => {
        $(
            fn $visit(&mut self $($(, $arg: $argty)*)?) -> Result<()> {
                // Compute the arity *before* validation changes the validator's state.
                let arity = wasmparser::Operator::$op $({ $($arg: $arg.clone()),* })?
                    .operator_arity(&self.validator);
                self.validator.$visit($($($arg.clone()),*)?)?;
                implement_some_operators!((self.stack) arity $op $({ $($arg),* })? => $visit)
            }
        )*
    };
}

impl<'a, 'w, V, W: Write> VisitOperator<'a> for OperatorVisitor<'_, 'w, V, W>
where
    V: VisitOperator<'a, Output = Result<(), wasmparser::Error>> + ModuleArity,
{
    type Output = Result<()>;
    for_each_visit_operator!(define_forward);
}
