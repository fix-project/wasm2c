use anyhow::{Result, bail};
use itertools::Itertools;
use smallvec::SmallVec;
use std::{
    fmt::{Display, Formatter},
    io::Write,
};
use wasmparser::{
    BlockType, CompositeInnerType, FrameKind, FrameStack, ModuleArity, ValType, VisitOperator,
    for_each_visit_operator,
};

#[derive(Clone, Copy, Debug)]
struct Operand(usize);

impl Display for Operand {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(out, "s{}", self.0)
    }
}

#[derive(Debug, Clone)]
struct IfFrame {
    had_else: bool,
    stack_height_on_enter: usize,
    consumes: Vec<Operand>,
    produces: Vec<Operand>,
}

impl IfFrame {
    fn enter_frame<'w, W: Write>(&self, printer: &mut ExpressionPrinter<'w, W>) -> Result<()> {
        // Pushes data consumed by if
        self.consumes
            .iter()
            .map(|c| printer.push_raw("auto", c))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }

    fn exit_frame<'w, W: Write>(
        &self,
        printer: &mut ExpressionPrinter<'w, W>,
        params: &[Operand],
    ) -> Result<()> {
        // Pushes data produced by if
        let mut param_it = params.iter().rev();
        self.produces
            .iter()
            .map(|p| -> Result<_, _> {
                let op = param_it.next().unwrap();
                writeln!(printer.out, "{} = {};", p, op)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }
}

pub(crate) struct ExpressionPrinter<'w, W> {
    value_count: usize,
    stack_pushes: usize,
    stack: Vec<Operand>,
    if_frames: Vec<IfFrame>,
    #[allow(dead_code)]
    frame_depth: usize, // TODO: increment on block/etc.
    out: &'w mut W,
}

// Implementations of a few operators.
// The "implement_some_operators" macro (below) controls the ones we need to implement.

// TODO: use a better writer abstraction so we are not interpolating strings so closely
impl<'w, W: Write> ExpressionPrinter<'w, W> {
    fn visit_i32_const(&mut self, value: i32, _params: &[Operand]) -> Result<()> {
        self.push(&ValType::I32, value)?;

        Ok(())
    }

    fn visit_i64_const(&mut self, value: i64, _params: &[Operand]) -> Result<()> {
        self.push(&ValType::I64, value)?;

        Ok(())
    }

    fn visit_i32_add(&mut self, params: &[Operand]) -> Result<()> {
        self.push(&ValType::I32, format!("{} + {}", params[0], params[1]))?;

        Ok(())
    }

    fn visit_drop(&mut self, _params: &[Operand]) -> Result<()> {
        Ok(())
    }

    fn visit_end(&mut self, params: &[Operand]) -> Result<()> {
        if let Some(frame) = self.if_frames.pop() {
            debug_assert!(self.stack.len() == frame.stack_height_on_enter);
            frame.exit_frame(self, params)?;

            // WebAssembly treats an else-less if statement as an empty else,
            // or passing through everything on else
            if !frame.had_else && !frame.produces.is_empty() {
                writeln!(self.out, "}} else {{")?;

                debug_assert_eq!(frame.consumes.len(), frame.produces.len());

                for (output, input) in frame.produces.iter().zip(&frame.consumes) {
                    writeln!(self.out, "{output} = {input};")?;
                }
            }

            writeln!(self.out, "}}")?;
            self.stack.extend_from_slice(&frame.produces);
        } else {
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
        }

        self.value_count += params.len(); // "end" operand formally pushes params to outer frame
        self.stack_pushes += params.len();

        Ok(())
    }

    fn visit_if<V: ModuleArity>(
        &mut self,
        validator: &V,
        block_ty: BlockType,
        params: &[Operand],
    ) -> Result<()> {
        let frame = match block_ty {
            BlockType::Empty => IfFrame {
                stack_height_on_enter: self.stack.len(),
                had_else: false,
                consumes: vec![],
                produces: vec![],
            },
            BlockType::Type(val_type) => IfFrame {
                stack_height_on_enter: self.stack.len(),
                had_else: false,
                consumes: vec![],
                produces: vec![self.push(&val_type, "0")?],
            },
            BlockType::FuncType(t) => {
                if let CompositeInnerType::Func(f) =
                    &validator.sub_type_at(t).unwrap().composite_type.inner
                {
                    IfFrame {
                        stack_height_on_enter: self.stack.len(),
                        had_else: false,
                        consumes: params[1..].iter().rev().copied().collect(),
                        produces: f
                            .results()
                            .iter()
                            .map(|p| self.push(p, "0"))
                            .collect::<Result<Vec<_>, _>>()?,
                    }
                } else {
                    unreachable!()
                }
            }
        };

        frame.produces.iter().for_each(|_| {
            self.pop();
        });
        self.stack_pushes -= frame.produces.len();

        // Push `produces` with default values
        writeln!(self.out, "if ({}) {{", params[0])?;
        frame.enter_frame(self)?;
        self.if_frames.push(frame);

        Ok(())
    }

    fn visit_else(&mut self, params: &[Operand]) -> Result<()> {
        let frame_mut = self.if_frames.last_mut().unwrap();
        frame_mut.had_else = true;
        let frame = frame_mut.clone();

        frame.exit_frame(self, params)?;
        writeln!(self.out, "}} else {{")?;
        frame.enter_frame(self)?;

        Ok(())
    }

    pub fn new(out: &'w mut W) -> Self {
        Self {
            value_count: 0,
            stack_pushes: 0,
            stack: vec![],
            if_frames: vec![],
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

    fn push_raw(&mut self, ty: impl Display, expr: impl Display) -> Result<Operand> {
        let op = Operand(self.value_count);
        let top_of_stack = self.stack.push_mut(op);
        writeln!(self.out, "{ty} {top_of_stack} = {expr};")?;
        self.value_count += 1;
        self.stack_pushes += 1;
        Ok(op)
    }

    fn push(&mut self, ty: &ValType, expr: impl Display) -> Result<Operand> {
        self.push_raw(ty, expr)
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
    ($self:tt $arity:tt If       $($t:tt)*) => { implement_some_operators!(@forward-validator $self $arity $($t)*) };
    ($self:tt $arity:tt Else     $($t:tt)*) => { implement_some_operators!(@forward $self $arity $($t)*) };

    (@forward $self:tt $arity:tt $({ $($arg:ident),* })? => $visit:ident) => {{
        implement_some_operators!(@base $self $visit $arity (|params: &[Operand]| -> Result<()> {
            ($self.stack).$visit($($($arg,)*)? params)
        }))
    }};

    (@forward-validator $self:tt $arity:tt $({ $($arg:ident),* })? => $visit:ident) => {{
        implement_some_operators!(@base $self $visit $arity (|params: &[Operand]| -> Result<()> {
            ($self.stack).$visit(&($self.validator), $($($arg,)*)? params)
        }))
    }};

    (@base $self:tt $visit:ident $arity:tt $call:tt) => {{
        let (num_params, num_results) = $arity.expect("arity for valid operator");
        let params: SmallVec<[Operand;2]> = (0..num_params).map(|_| ($self.stack).pop()).collect();
        let push_count_pre = ($self.stack).stack_pushes;
        ($call)(&params)?;
        debug_assert_eq!(($self.stack).stack_pushes - push_count_pre, num_results as usize,
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
                implement_some_operators!(self arity $op $({ $($arg),* })? => $visit)
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
