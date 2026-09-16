@publish import package::hashmap::commons::{ getKey, locationHint, ENTRY_SIZE, NOT_FOUND };
import package::stats::stats;

@group(1) @binding(0) var<storage, read_write> hashmap: array<atomic<u32>>;

fn insert(key: vec2u) -> u32 {
	let hashmap_entries = arrayLength(&hashmap) / ENTRY_SIZE;
	let location_hint = locationHint(key) % hashmap_entries;

	var entry = location_hint;
	loop {
		entry = (entry + 1) % hashmap_entries;

		if entry == location_hint {
			return NOT_FOUND;
		}

		var result = atomicCompareExchange(entry * ENTRY_SIZE, 0, key.x);
		if !result.exchanged && result.old_value != key.x {
			atomicAdd(&stats.collisions, 1);
			continue;
		}

		result = atomicCompareExchange(entry * ENTRY_SIZE + 1, 0, key.y);
		if !result.exchanged && result.old_value != key.y {
			atomicAdd(&stats.collisions, 1);
			continue;
		}

		break;
	}

	return entry * ENTRY_SIZE + 2;
}

struct AtomicCompareExchangeResult {
  old_value: u32,
  exchanged: bool,
}

fn atomicCompareExchange(idx: u32, cmp: u32, v: u32) -> AtomicCompareExchangeResult {
	loop {
		let result = atomicCompareExchangeWeak(&hashmap[idx], cmp, v);
		if result.exchanged || result.old_value != cmp {
			var ret = AtomicCompareExchangeResult();
			ret.old_value = result.old_value;
			ret.exchanged = result.exchanged;
			return ret;
		}
	}
	return AtomicCompareExchangeResult();
}
