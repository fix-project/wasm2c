use crate::module::Module;
use anyhow::{Result, bail};
use itertools::Itertools;
use smallvec::SmallVec;
use std::{
    fmt::{Display, Formatter},
    io::Write,
};
use wasmparser::{
    BlockType, Frame, FrameKind, FuncValidator, MemArg, Operator, ValType, WasmModuleResources,
};

#[derive(Copy, Clone)]
struct Operand(usize);

impl Display for Operand {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(out, "s{}", self.0)
    }
}

#[derive(Clone)]
struct BlockFrame {
    // 0 is reserved for the function frame.
    idx: usize,
    kind: FrameKind,
    result: Option<Operand>,
    branched: bool,
}

pub(crate) struct ExpressionPrinter<'w, W> {
    module: &'w Module<'w>,
    stack: Vec<Operand>,
    pending: Vec<ValType>,
    frames: Vec<BlockFrame>,
    push_counter: usize,
    label_counter: usize,
    reachable: bool,
    out: &'w mut W,
}

// Implementations of a few operators.
// TODO: use a better writer abstraction so we are not interpolating strings so closely
type Params = SmallVec<[Operand; 2]>;
impl<'w, W: Write> ExpressionPrinter<'w, W> {
    fn do_operator(&mut self, op: &Operator, params: Params, frame: Frame) -> Result<()> {
        use Operator::*;
        match op {
            I32Const { .. } => self.push(const_literal(op)?),
            I64Const { .. } => self.push(const_literal(op)?),
            I32Add => self.push(format!("{} + {}", params[0], params[1])),
            Drop => Ok(()),
            Block { blockty } => self.visit_block(*blockty, params),
            Br { relative_depth } => self.visit_br(*relative_depth, params),
            End => self.visit_end(params, frame),
            GlobalGet { global_index } => self.push(self.module.global_names.get(*global_index)),
            GlobalSet { global_index } => self.visit_global_set(*global_index, params),

            I32Load { memarg } => self.visit_load("i32", memarg, params),
            I64Load { memarg } => self.visit_load("i64", memarg, params),
            F32Load { memarg } => self.visit_load("f32", memarg, params),
            F64Load { memarg } => self.visit_load("f64", memarg, params),
            I32Load8S { memarg } | I64Load8S { memarg } => {
                self.visit_load("int8_t", memarg, params)
            }
            I32Load8U { memarg } | I64Load8U { memarg } => {
                self.visit_load("uint8_t", memarg, params)
            }
            I32Load16S { memarg } | I64Load16S { memarg } => {
                self.visit_load("int16_t", memarg, params)
            }
            I32Load16U { memarg } | I64Load16U { memarg } => {
                self.visit_load("uint16_t", memarg, params)
            }
            I64Load32S { memarg } => self.visit_load("int32_t", memarg, params),
            I64Load32U { memarg } => self.visit_load("uint32_t", memarg, params),
            I32Store { memarg } => self.visit_store("i32", memarg, params),
            I64Store { memarg } => self.visit_store("i64", memarg, params),
            F32Store { memarg } => self.visit_store("f32", memarg, params),
            F64Store { memarg } => self.visit_store("f64", memarg, params),
            I32Store8 { memarg } | I64Store8 { memarg } => {
                self.visit_store("uint8_t", memarg, params)
            }
            I32Store16 { memarg } | I64Store16 { memarg } => {
                self.visit_store("uint16_t", memarg, params)
            }
            I64Store32 { memarg } => self.visit_store("uint32_t", memarg, params),
            MemorySize { mem } => {
                let memory = self.module.memory_names.get(*mem);
                self.push(format!("{memory}.pages()"))
            }
            MemoryGrow { mem } => {
                let memory = self.module.memory_names.get(*mem);
                self.push(format!("{memory}.grow({})", params[0]))
            }

            op => bail!("operator {} unimplemented", op_name(op)),
        }
    }

    fn visit_load(&mut self, ty: &str, memarg: &MemArg, params: Params) -> Result<()> {
        let memory = self.module.memory_names.get(memarg.memory);
        let result = *self.pending.last().expect("load result type");
        let load = format!("{memory}.load<{ty}>({}, {})", params[0], memarg.offset);
        self.push(format!("static_cast<{result}>({load})"))
    }

    fn visit_store(&mut self, ty: &str, memarg: &MemArg, params: Params) -> Result<()> {
        let memory = self.module.memory_names.get(memarg.memory);
        let (address, value) = (params[0], params[1]);
        let offset = memarg.offset;
        writeln!(
            self.out,
            "{memory}.store({address}, {offset}, static_cast<{ty}>({value}));"
        )?;
        Ok(())
    }

    fn visit_block(&mut self, blockty: BlockType, params: Params) -> Result<()> {
        self.cross_frame(&params);

        // Declare result before entering the block
        let result = match blockty {
            BlockType::Empty => None,
            BlockType::Type(ty) => Some(self.push_default(ty)?),
            BlockType::FuncType(_) => bail!("unimplemented multi-value"),
        };

        let idx = self.label_counter;
        self.label_counter += 1;
        writeln!(self.out, "{{")?;

        self.frames.push(BlockFrame {
            idx,
            kind: FrameKind::Block,
            result,
            branched: false,
        });

        Ok(())
    }

    fn visit_br(&mut self, relative_depth: u32, params: Params) -> Result<()> {
        let target_index = self
            .frames
            .len()
            .checked_sub(1 + relative_depth as usize)
            .expect("valid branch depth");

        let target = self.frames[target_index].clone();

        // Frame 0 is the implicit function frame, so branching to it is
        // equivalent to an early return.
        if target_index == 0 {
            // wait until we implement `visit_return` to reuse the logic
            bail!("unimplemented early return from function frame");
        }

        match target.kind {
            FrameKind::Block => {}
            kind => bail!("unimplemented branch to {kind:?} frame"),
        }

        // Conclude block once we're unconditionally branching out
        self.conclude_block(&target, &params)?;
        writeln!(self.out, "goto block{};", target.idx)?;

        self.frames[target_index].branched = true;
        self.reachable = false;

        Ok(())
    }

    fn visit_end(&mut self, params: Params, frame: Frame) -> Result<()> {
        // function end
        if self.frames.len() == 1 {
            return self.end_function(params, frame);
        }

        match self.current_block().kind {
            FrameKind::Block => self.end_block(params, frame),
            _ => bail!("unimplemented block kind"),
        }
    }

    fn end_function(&mut self, params: Params, frame: Frame) -> Result<()> {
        self.stack.truncate(frame.height);
        self.cross_frame(&params);
        self.frames.pop();
        Ok(())
    }

    fn end_block(&mut self, params: Params, frame: Frame) -> Result<()> {
        // close block out, put goto label, and drain block results
        let block = self.frames.pop().expect("block frame");
        self.stack.truncate(frame.height);

        if self.reachable {
            self.conclude_block(&block, &params)?;
        }

        writeln!(self.out, "}}")?;

        if block.branched {
            writeln!(self.out, "block{}: ;", block.idx)?;
        }

        self.reachable = match block.kind {
            // branching to a loop causes a loop
            FrameKind::Loop => self.reachable,
            _ => self.reachable || block.branched,
        };

        if self.reachable {
            self.stack.extend(block.result);
        }

        self.pending.clear();
        Ok(())
    }

    fn visit_global_set(&mut self, global_index: u32, params: Params) -> Result<()> {
        let name = self.module.global_names.get(global_index);
        writeln!(self.out, "{name} = {};", params[0])?;
        Ok(())
    }

    pub fn new(module: &'w Module<'w>, out: &'w mut W) -> Self {
        Self {
            module,
            stack: vec![],
            pending: vec![],
            frames: vec![BlockFrame {
                idx: 0,
                // wasmparser represents the implicit function frame as a block
                kind: FrameKind::Block,
                result: None,
                branched: false,
            }],
            push_counter: 0,
            label_counter: 1,
            reachable: true,
            out,
        }
    }

    fn push_default(&mut self, ty: ValType) -> Result<Operand> {
        let operand = Operand(self.push_counter);
        self.push_counter += 1;
        writeln!(self.out, "{ty} {operand}{{}};")?;
        Ok(operand)
    }

    fn push(&mut self, expr: impl Display) -> Result<()> {
        let ty = self.pending.pop().unwrap();
        let operand = Operand(self.push_counter);
        self.stack.push(operand);
        writeln!(self.out, "{ty} {operand} = {expr};")?;
        self.push_counter += 1;

        Ok(())
    }

    fn cross_frame(&mut self, params: &[Operand]) {
        assert_eq!(params.len(), self.pending.len());

        // Restore params that were automatically popped
        self.stack.extend_from_slice(params);
        self.pending.clear();
    }

    fn current_block(&self) -> &BlockFrame {
        self.frames.last().expect("control frame")
    }

    fn conclude_block(&mut self, block: &BlockFrame, params: &Params) -> Result<()> {
        if let Some(slot) = block.result {
            writeln!(self.out, "{slot} = {};", params[0])?;
        }

        Ok(())
    }

    // public entry point -- called on each operator
    pub fn op<T: WasmModuleResources>(
        &mut self,
        validator: &mut FuncValidator<T>,
        op: &Operator,
        offset: u64,
    ) -> Result<()> {
        // Skip unreachable code (like wasm2c)
        if !self.reachable {
            // Once we reach unreachable code, we stop pushing blocks, but
            // the validator doesn't, so we just wait until our frames and the
            // validators frames line up again to indicate we've ended the
            // unreachable
            if matches!(op, Operator::End)
                && validator.control_stack_height() as usize == self.frames.len()
            {
                let frame = validator.get_control_frame(0).copied().expect("frame");
                validator.op(offset, op)?;
                self.visit_end(Params::new(), frame)?;
            } else {
                validator.op(offset, op)?;
            }

            return Ok(());
        }

        // Collect arity, top control frame, and params before validation changes the validator's state.
        let Some((num_params, num_results)) = op.operator_arity(validator) else {
            validator.op(offset, op)?; // report validation error
            panic!("operator valid but missing arity");
        };
        let num_params: usize = num_params.try_into().unwrap();
        let num_results: usize = num_results.try_into().unwrap();
        let frame = validator.get_control_frame(0).copied().expect("frame");

        let split = self.stack.len() - num_params;
        let params: Params = self.stack.drain(split..).collect();

        // validate the operator
        validator.op(offset, op)?;

        // collect results
        for i in 0..num_results {
            self.pending
                .push(validator.get_operand_type(i).unwrap().unwrap());
        }

        self.do_operator(op, params, frame)?;

        debug_assert!(
            self.pending.is_empty(),
            "operator {} did not push enough operands",
            op_name(op),
        );

        Ok(())
    }

    // public method to be called when function is over
    pub(crate) fn finish(&mut self) -> Result<()> {
        // Function end
        debug_assert!(self.pending.is_empty());
        match self.stack.as_slice() {
            [] => {} // no need to do anything at the end of the function
            [param] => writeln!(self.out, "return {};", param)?,
            params => writeln!(self.out, "return {{{}}};", params.iter().format(", "))?,
        }
        Ok(())
    }
}

fn op_name(op: &Operator) -> String {
    let s = format!("{op:?}");
    let end = s.find(|c: char| !c.is_alphanumeric()).unwrap_or(s.len());
    s[..end].to_owned()
}

pub(crate) fn const_literal(op: &Operator<'_>) -> Result<String> {
    use Operator::*;
    Ok(match op {
        I32Const { value } => format!("i32{{{}u}}", *value as u32),
        I64Const { value } => format!("i64{{{}u}}", *value as u64),
        F32Const { .. } => bail!("unimplemented floating-point literal"),
        F64Const { .. } => bail!("unimplemented floating-point literal"),
        _ => panic!("const_literal called on non-numeric Wasm operator"),
    })
}
