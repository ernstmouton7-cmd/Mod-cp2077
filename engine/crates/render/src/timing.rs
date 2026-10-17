//! GPU pass timing with timestamp queries (non-blocking readback).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const MAX_PASSES: u32 = 48;

pub(crate) struct GpuTimer {
    query_set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    names: Vec<&'static str>,
    pending_names: Vec<&'static str>,
    in_flight: bool,
    ready: Arc<AtomicBool>,
    period_ns: f32,
    pub results: Vec<(&'static str, f32)>,
    pub enabled: bool,
}

impl GpuTimer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, supported: bool) -> Option<Self> {
        if !supported {
            return None;
        }
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: MAX_PASSES * 2,
        });
        let size = (MAX_PASSES * 2 * 8) as u64;
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ts-resolve"),
            size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ts-readback"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Some(Self {
            query_set,
            resolve,
            readback,
            names: Vec::new(),
            pending_names: Vec::new(),
            in_flight: false,
            ready: Arc::new(AtomicBool::new(false)),
            period_ns: queue.get_timestamp_period(),
            results: Vec::new(),
            enabled: true,
        })
    }

    pub fn begin_frame(&mut self) {
        self.names.clear();
    }

    pub fn query_set(&self) -> wgpu::QuerySet {
        self.query_set.clone()
    }

    pub fn slot(&mut self, name: &'static str) -> Option<u32> {
        if !self.enabled || self.in_flight || self.names.len() as u32 >= MAX_PASSES {
            return None;
        }
        self.names.push(name);
        Some(self.names.len() as u32 - 1)
    }

    pub fn render(&mut self, name: &'static str) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let i = self.slot(name)?;
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.query_set,
            beginning_of_pass_write_index: Some(2 * i),
            end_of_pass_write_index: Some(2 * i + 1),
        })
    }

    pub fn compute(&mut self, name: &'static str) -> Option<wgpu::ComputePassTimestampWrites<'_>> {
        let i = self.slot(name)?;
        Some(wgpu::ComputePassTimestampWrites {
            query_set: &self.query_set,
            beginning_of_pass_write_index: Some(2 * i),
            end_of_pass_write_index: Some(2 * i + 1),
        })
    }

    pub fn resolve(&mut self, enc: &mut wgpu::CommandEncoder) {
        if self.in_flight || self.names.is_empty() {
            return;
        }
        let n = self.names.len() as u32 * 2;
        enc.resolve_query_set(&self.query_set, 0..n, &self.resolve, 0);
        enc.copy_buffer_to_buffer(&self.resolve, 0, &self.readback, 0, n as u64 * 8);
    }

    pub fn after_submit(&mut self) {
        if self.in_flight || self.names.is_empty() {
            return;
        }
        self.in_flight = true;
        self.pending_names = std::mem::take(&mut self.names);
        let ready = self.ready.clone();
        self.readback.map_async(wgpu::MapMode::Read, .., move |r| {
            if r.is_ok() {
                ready.store(true, Ordering::Release);
            }
        });
    }

    /// Collects results if the readback finished. Returns total ms.
    pub fn collect(&mut self) -> Option<f32> {
        if !self.in_flight || !self.ready.load(Ordering::Acquire) {
            return None;
        }
        self.ready.store(false, Ordering::Release);
        let n = self.pending_names.len();
        let mut res = Vec::with_capacity(n);
        let mut total = 0.0;
        if let Ok(view) = self.readback.get_mapped_range(..(n as u64 * 16)) {
            let ts: &[u64] = bytemuck::cast_slice(&view);
            for (i, name) in self.pending_names.iter().enumerate() {
                let (a, b) = (ts[2 * i], ts[2 * i + 1]);
                let ms = b.saturating_sub(a) as f32 * self.period_ns / 1.0e6;
                total += ms;
                res.push((*name, ms));
            }
        }
        self.readback.unmap();
        self.in_flight = false;
        self.results = res;
        Some(total)
    }
}
