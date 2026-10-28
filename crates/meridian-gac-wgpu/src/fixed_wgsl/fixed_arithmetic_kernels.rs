//! Compute pipelines for the fixed-point operations.

use meridian_compute_app::ComputeContext;
use meridian_compute_wgpu::GpuComputeDevice;
use meridian_gpu_wgpu::{BufferUsage, ComputePipeline};
use meridian_numeric::Fixed;

use super::{FixedBinaryOp, dispatch_entry_points_wgsl};

/// Compiled compute pipelines for every [`Fixed`] binary operation,
/// built once against a [`ComputeContext`]'s GPU backend
/// ([`ComputeContext::with_gpu`]) and reused across dispatches.
#[derive(Debug)]
pub struct FixedArithmeticKernels {
    add: ComputePipeline,
    sub: ComputePipeline,
    mul: ComputePipeline,
    div: ComputePipeline,
    sqrt: ComputePipeline,
    atan2: ComputePipeline,
    sin_cos: ComputePipeline,
}

impl FixedArithmeticKernels {
    /// Compiles the WGSL module and builds every pipeline (the four
    /// binary ops plus `sqrt`). `context` must already have a GPU
    /// backend ([`ComputeContext::with_gpu`]) — panics otherwise, the
    /// same "caller opted into GPU dispatch explicitly, so a missing
    /// backend is a logic error to surface immediately" policy
    /// `meridian-rendering-wgpu`'s own headless-device tests use.
    pub fn new(context: &ComputeContext) -> Self {
        let gpu = context
            .gpu::<GpuComputeDevice>()
            .expect("FixedArithmeticKernels::new requires a ComputeContext with a GPU backend (see ComputeContext::with_gpu)");
        let source = dispatch_entry_points_wgsl();
        let shader = gpu.create_shader("fixed_arithmetic", &source);
        let add = gpu.create_compute_pipeline(&shader, FixedBinaryOp::Add.entry_point());
        let sub = gpu.create_compute_pipeline(&shader, FixedBinaryOp::Sub.entry_point());
        let mul = gpu.create_compute_pipeline(&shader, FixedBinaryOp::Mul.entry_point());
        let div = gpu.create_compute_pipeline(&shader, FixedBinaryOp::Div.entry_point());
        let sqrt = gpu.create_compute_pipeline(&shader, "dispatch_sqrt");
        let atan2 = gpu.create_compute_pipeline(&shader, FixedBinaryOp::Atan2.entry_point());
        let sin_cos = gpu.create_compute_pipeline(&shader, "dispatch_sin_cos");
        Self {
            add,
            sub,
            mul,
            div,
            sqrt,
            atan2,
            sin_cos,
        }
    }

    fn pipeline(&self, op: FixedBinaryOp) -> &ComputePipeline {
        match op {
            FixedBinaryOp::Add => &self.add,
            FixedBinaryOp::Sub => &self.sub,
            FixedBinaryOp::Mul => &self.mul,
            FixedBinaryOp::Div => &self.div,
            FixedBinaryOp::Atan2 => &self.atan2,
        }
    }

    /// Runs `op` element-wise over `pairs` on the GPU, returning one
    /// [`Fixed`] result per pair, in order. A real `async fn` — reading
    /// the result buffer back waits on in-flight GPU work (see
    /// `meridian_gpu_wgpu`'s module doc for this workspace's "async on
    /// genuine I/O" policy).
    pub async fn dispatch(
        &self,
        context: &ComputeContext,
        op: FixedBinaryOp,
        pairs: &[(Fixed, Fixed)],
    ) -> Vec<Fixed> {
        let gpu = context.gpu::<GpuComputeDevice>().expect(
            "FixedArithmeticKernels::dispatch requires a ComputeContext with a GPU backend",
        );

        let mut operand_bytes = Vec::with_capacity(pairs.len() * 8);
        for (a, b) in pairs {
            operand_bytes.extend_from_slice(&a.to_bits().to_le_bytes());
            operand_bytes.extend_from_slice(&b.to_bits().to_le_bytes());
        }
        let operands = gpu.allocate_buffer(operand_bytes.len(), BufferUsage::Storage);
        gpu.write_buffer(&operands, &operand_bytes);

        let results = gpu.allocate_buffer(pairs.len() * 4, BufferUsage::Storage);

        // Two-buffer dispatch: `GpuComputeDevice::dispatch`'s single-
        // buffer convenience doesn't cover "read from one buffer, write
        // to another," so this builds its own bind group and records its
        // own compute pass via the general (multi-buffer) forms
        // `meridian_gpu_wgpu::Device`/`CommandBuffer` expose for
        // exactly this shape.
        let pipeline = self.pipeline(op);
        let device = gpu.gpu_driver_device();
        let bind_group =
            device.create_bind_group(&pipeline.bind_group_layout(), &[&operands, &results]);

        let mut commands = device.create_command_buffer();
        commands.dispatch_compute_with_bind_group(
            pipeline,
            &bind_group,
            (pairs.len() as u32).div_ceil(64).max(1),
        );
        commands.submit();

        let result_bytes = gpu.read_buffer(&results).await;
        result_bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| Fixed::from_bits(i32::from_le_bytes(le_word(c, 0))))
            .collect()
    }

    /// Runs `sqrt` element-wise over `values` on the GPU, returning one
    /// [`Fixed`] result per value, in order. Every `value` must be
    /// non-negative — the same precondition `Fixed::sqrt` asserts on the
    /// CPU side; WGSL has no panic, so passing a negative value here
    /// produces a nonsensical (not merely wrong-sign) result rather than
    /// erroring, and callers are responsible for upholding it themselves.
    pub async fn dispatch_sqrt(&self, context: &ComputeContext, values: &[Fixed]) -> Vec<Fixed> {
        let gpu = context.gpu::<GpuComputeDevice>().expect(
            "FixedArithmeticKernels::dispatch_sqrt requires a ComputeContext with a GPU backend",
        );

        let mut operand_bytes = Vec::with_capacity(values.len() * 4);
        for value in values {
            operand_bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        let operands = gpu.allocate_buffer(operand_bytes.len(), BufferUsage::Storage);
        gpu.write_buffer(&operands, &operand_bytes);

        let results = gpu.allocate_buffer(values.len() * 4, BufferUsage::Storage);

        let device = gpu.gpu_driver_device();
        let bind_group =
            device.create_bind_group(&self.sqrt.bind_group_layout(), &[&operands, &results]);

        let mut commands = device.create_command_buffer();
        commands.dispatch_compute_with_bind_group(
            &self.sqrt,
            &bind_group,
            (values.len() as u32).div_ceil(64).max(1),
        );
        commands.submit();

        let result_bytes = gpu.read_buffer(&results).await;
        result_bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| Fixed::from_bits(i32::from_le_bytes(le_word(c, 0))))
            .collect()
    }

    /// Runs `sin_cos` element-wise over `thetas` on the GPU, returning one
    /// `(sin, cos)` pair per angle, in order. Unlike every other op here,
    /// the `results` buffer holds two `i32`s per invocation
    /// (`fixed_sin_cos`'s `vec2<i32>`), so this builds its own
    /// double-width buffer rather than reusing [`dispatch`](Self::dispatch).
    pub async fn dispatch_sin_cos(
        &self,
        context: &ComputeContext,
        thetas: &[Fixed],
    ) -> Vec<(Fixed, Fixed)> {
        let gpu = context.gpu::<GpuComputeDevice>().expect(
            "FixedArithmeticKernels::dispatch_sin_cos requires a ComputeContext with a GPU backend",
        );

        let mut operand_bytes = Vec::with_capacity(thetas.len() * 4);
        for theta in thetas {
            operand_bytes.extend_from_slice(&theta.to_bits().to_le_bytes());
        }
        let operands = gpu.allocate_buffer(operand_bytes.len(), BufferUsage::Storage);
        gpu.write_buffer(&operands, &operand_bytes);

        let results = gpu.allocate_buffer(thetas.len() * 8, BufferUsage::Storage);

        let device = gpu.gpu_driver_device();
        let bind_group =
            device.create_bind_group(&self.sin_cos.bind_group_layout(), &[&operands, &results]);

        let mut commands = device.create_command_buffer();
        commands.dispatch_compute_with_bind_group(
            &self.sin_cos,
            &bind_group,
            (thetas.len() as u32).div_ceil(64).max(1),
        );
        commands.submit();

        let result_bytes = gpu.read_buffer(&results).await;
        result_bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|pair| {
                let sin = Fixed::from_bits(i32::from_le_bytes(le_word(pair, 0)));
                let cos = Fixed::from_bits(i32::from_le_bytes(le_word(pair, 4)));
                (sin, cos)
            })
            .collect()
    }
}

/// The four bytes of the little-endian word at `at` in a GPU readback.
/// Readback buffers are sized by the dispatch that wrote them, so `at + 4`
/// is in range for every index the caller computes from that size.
fn le_word(bytes: &[u8], at: usize) -> [u8; 4] {
    [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]
}
