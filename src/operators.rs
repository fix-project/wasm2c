use anyhow::{Result, bail};
use itertools::Itertools;
use smallvec::SmallVec;
use std::{
    fmt::{Display, Formatter},
    io::Write,
};
use wasmparser::{
    BlockType, FrameKind, FrameStack, FuncValidator, ValType, VisitOperator, WasmModuleResources,
    for_each_visit_operator,
};

#[derive(Clone)]
struct Operand(usize);

impl Display for Operand {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(out, "s{}", self.0)
    }
}

pub(crate) struct ExpressionPrinter<'w, W> {
    counter: usize,
    stack: Vec<Operand>,
    out: &'w mut W,
}

// Implementations of a few operators.
// The "implement_some_operators" macro (below) controls the ones we need to implement.

// TODO: use a better writer abstraction so we are not interpolating strings so closely
impl<'w, W: Write> ExpressionPrinter<'w, W> {
    fn visit_i32_const<T: WasmModuleResources>(
        &mut self,
        value: i32,
        _params: &[Operand],
        _validator: &FuncValidator<T>,
    ) -> Result<()> {
        self.push(&ValType::I32, value)
    }

    fn visit_i64_const<T: WasmModuleResources>(
        &mut self,
        value: i64,
        _params: &[Operand],
        _validator: &FuncValidator<T>,
    ) -> Result<()> {
        self.push(&ValType::I64, value)
    }

    fn visit_i32_add<T: WasmModuleResources>(
        &mut self,
        params: &[Operand],
        _validator: &FuncValidator<T>,
    ) -> Result<()> {
        self.push(&ValType::I32, format!("{} + {}", params[0], params[1]))
    }

    fn visit_drop<T: WasmModuleResources>(
        &mut self,
        _params: &[Operand],
        _validator: &FuncValidator<T>,
    ) -> Result<()> {
        Ok(())
    }

    fn visit_end<T: WasmModuleResources>(
        &mut self,
        params: &[Operand],
        validator: &FuncValidator<T>,
    ) -> Result<()> {
        let control_frame = validator.get_control_frame(0);

        if let Some(control_frame) = control_frame {
            self.enter_control_frame(params);

            match control_frame.kind {
                FrameKind::Block => {
                    // Write label for future `br`
                    writeln!(self.out, "block{}:", control_frame.height)?;

                    Ok(())
                }
                kind => bail!("exit on {kind:?} is not implemented"),
            }
        } else {
            // Function end
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
    }

    fn visit_block<T: WasmModuleResources>(
        &mut self,
        _block_type: BlockType,
        params: &[Operand],
        _validator: &FuncValidator<T>,
    ) -> Result<()> {
        // Re-push `params` again
        self.enter_control_frame(params);

        Ok(())
    }

    pub fn new(out: &'w mut W) -> Self {
        Self {
            counter: 0,
            stack: vec![],
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

    fn enter_control_frame(&mut self, params: &[Operand]) {
        // Restore params that were automatically popped
        self.stack.extend(params.iter().cloned());
        self.counter += params.len();
    }
}

macro_rules! implement_some_operators {
    // Implemented operators can be added here incrementally.
    ($self:tt $arity:tt I32Const $($t:tt)*) => { implement_some_operators!(@forward $self $arity $($t)*) };
    ($self:tt $arity:tt I64Const $($t:tt)*) => { implement_some_operators!(@forward $self $arity $($t)*) };
    ($self:tt $arity:tt I32Add   $($t:tt)*) => { implement_some_operators!(@forward $self $arity $($t)*) };
    ($self:tt $arity:tt Drop     $($t:tt)*) => { implement_some_operators!(@forward $self $arity $($t)*) };
    ($self:tt $arity:tt End      $($t:tt)*) => { implement_some_operators!(@forward $self $arity $($t)*) };
    ($self:tt $arity:tt Block    $($t:tt)*) => { implement_some_operators!(@forward $self $arity $($t)*) };

    (@forward $self:tt $arity:tt $({ $($arg:ident),* })? => $visit:ident) => {{
        let (num_params, num_results) = $arity.expect("arity for valid operator");
        let params: SmallVec<[Operand;2]> = (0..num_params).map(|_| ($self.stack).pop()).collect();
        let push_count_pre = ($self.stack).counter;
        ($self.stack).$visit($($($arg,)*)? &params, &*($self.validator))?;
        debug_assert_eq!(($self.stack).counter - push_count_pre, num_results as usize,
                         "push count for {} (left) did not match arity (right)", stringify!($visit));
    	Ok(())
    }};

    // Bail out gracefully on any other operator.
    ($self:tt $arity:tt $op:ident $($t:tt)*) => {{ let _ = $arity;
						   bail!("operator {} unimplemented", stringify!($op)); }};
}

pub struct OperatorVisitor<'a, 'w, T, W> {
    validator: &'a mut FuncValidator<T>,
    offset: u64,
    stack: &'a mut ExpressionPrinter<'w, W>,
}

impl<'w, T: WasmModuleResources, W> FrameStack for OperatorVisitor<'_, 'w, T, W> {
    fn current_frame(&self) -> Option<FrameKind> {
        self.validator.get_control_frame(0).map(|frame| frame.kind)
    }
}

// Pair the ExpressionPrinter with a FuncValidator visitor to validate each operator first.
macro_rules! define_forward {
    ($( @$proposal:ident $op:ident $({ $($arg:ident: $argty:ty),* })? => $visit:ident ($($ann:tt)*) )*) => {
        $(
            fn $visit(&mut self $($(, $arg: $argty)*)?) -> Result<()> {
                let operator = wasmparser::Operator::$op $({ $($arg: $arg.clone()),* })?;
                // Compute the arity *before* validation changes the validator's state.
                let arity = operator.operator_arity(&*self.validator);
                self.validator.op(self.offset, &operator)?;
                implement_some_operators!(self arity $op $({ $($arg),* })? => $visit)
            }
        )*
    };
}

impl<'a, 'w, T, W: Write> VisitOperator<'a> for OperatorVisitor<'_, 'w, T, W>
where
    T: WasmModuleResources,
{
    type Output = Result<()>;
    for_each_visit_operator!(define_forward);
}
