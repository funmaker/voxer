use std::sync::{Arc, Mutex};
use log::error;
use wgpu::{Buffer, BufferUsages, CommandEncoder, MapMode, QuerySet, QueryType, RenderPass};

use crate::application::bench::Benchmark;
use crate::application::render_context::{RenderContext, TIMING_QUERY_COUNT};

pub struct GpuBenchmark {
	bench: Arc<Mutex<Benchmark>>,
	query_set: QuerySet,
	ticks: Vec<&'static str>,
	resolve_pool: Pool<Arc<Buffer>>,
	read_pool: Pool<Arc<Buffer>>,
}

impl GpuBenchmark {
	pub fn new(render: &RenderContext) -> Self {
		let query_set = render.device.create_query_set(&wgpu::QuerySetDescriptor {
			label: Some("Timing Query Set"),
			ty: QueryType::Timestamp,
			count: TIMING_QUERY_COUNT,
		});
		
		Self {
			bench: Arc::new(Mutex::new(Benchmark::new())),
			query_set,
			ticks: vec![],
			resolve_pool: Pool::new(),
			read_pool: Pool::new(),
		}
	}
	
	pub fn new_frame(&mut self, render: &RenderContext) {
		let resolve_buffer = self.resolve_pool.take()
		                                      .unwrap_or_else(|| {
			                                      Arc::new(render.device.create_buffer(&wgpu::BufferDescriptor {
				                                      label: Some("Timing Resolve Buffer"),
				                                      size: (TIMING_QUERY_COUNT * 8) as u64,
				                                      usage: BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
				                                      mapped_at_creation: false,
			                                      }))
		                                      });
		
		let read_buffer = self.read_pool.take()
		                                .unwrap_or_else(|| {
			                                Arc::new(render.device.create_buffer(&wgpu::BufferDescriptor {
				                                label: Some("Timing Read Buffer"),
				                                size: resolve_buffer.size(),
				                                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
				                                mapped_at_creation: false,
			                                }))
		                                });
		
		let mut encoder = render.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("GPU Benchmark Encoder") });
		
		self.tick("Frame End", &mut encoder);
		
		encoder.resolve_query_set(&self.query_set,
		                          0..self.ticks.len() as u32,
		                          &resolve_buffer,
		                          0);
		
		encoder.copy_buffer_to_buffer(&resolve_buffer, 0, &read_buffer, 0, resolve_buffer.size());
		
		let ticks = std::mem::replace(&mut self.ticks, vec![]);
		let output_len = ticks.len() as u64 * 8;
		
		self.tick("Frame Start", &mut encoder);
		
		render.queue.submit(Some(encoder.finish()));
		
		let bench = self.bench.clone();
		let resolve_pool = self.resolve_pool.clone();
		let read_pool = self.read_pool.clone();
		let resolve_buffer_capture = resolve_buffer.clone();
		let read_buffer_capture = read_buffer.clone();
		let timestamp_scale = render.queue.get_timestamp_period() as f64 / 1_000_000_000.0;
		
		read_buffer.slice(..output_len).map_async(MapMode::Read, move |result| {
			if let Err(err) = result {
				error!("GPU Bench Buffer slice error: {err}");
				return;
			}
			
			let mut gpu_bench = bench.lock().unwrap();
			let view = read_buffer_capture.slice(..output_len).get_mapped_range();
			let timestamps: &[u64] = bytemuck::cast_slice(&view);
			
			gpu_bench.new_frame();
			
			for (n, stage) in ticks.into_iter().enumerate().skip(1) {
				let time = (timestamps[n] - timestamps[0]) as f64 * timestamp_scale;
				gpu_bench.tick_exact(stage, time);
			}
			
			drop(view);
			drop(gpu_bench);
			read_buffer_capture.unmap();
			
			resolve_pool.give_back(resolve_buffer_capture);
			read_pool.give_back(read_buffer_capture);
		});
	}
	
	pub fn tick(&mut self, stage: &'static str, encoder: &mut impl WriteTimestamp) {
		let pos = self.ticks.len() as u32;
		assert!(pos < TIMING_QUERY_COUNT);
		encoder.write_timestamp(&self.query_set, pos);
		self.ticks.push(stage);
	}
	
	pub fn toggle_open(&mut self) {
		self.bench.lock().unwrap().toggle_open();
	}
	
	pub fn on_gui_window(&mut self, ctx: &egui::Context, title: &str, target_fps: Option<f32>) {
		self.bench.lock().unwrap().on_gui_window(ctx, title, target_fps);
	}
}

#[derive(Debug, Clone)]
struct Pool<T>(Arc<Mutex<Vec<T>>>);

impl<T> Pool<T> {
	fn new() -> Self {
		Pool(Arc::new(Mutex::new(vec![])))
	}
	
	fn take(&self) -> Option<T> {
		self.0.lock().unwrap().pop()
	}
	
	fn give_back(&self, value: T) {
		self.0.lock().unwrap().push(value)
	}
}

pub trait WriteTimestamp {
	fn write_timestamp(&mut self, query_set: &QuerySet, query_index: u32) -> ();
}

impl WriteTimestamp for CommandEncoder {
	fn write_timestamp(&mut self, query_set: &QuerySet, query_index: u32) -> () {
		self.write_timestamp(query_set, query_index);
	}
}

impl WriteTimestamp for RenderPass<'_> {
	fn write_timestamp(&mut self, query_set: &QuerySet, query_index: u32) -> () {
		self.write_timestamp(query_set, query_index);
	}
}
