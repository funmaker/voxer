
@group(0) @binding(2) var<storage, read_write> stats: Stats;

struct Stats {
	entries: atomic<u32>,
	collisions: atomic<u32>,
}
