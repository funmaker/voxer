@publish import package::hashmap::commons::{ getKey, locationHint, ENTRY_SIZE, NOT_FOUND };

@group(1) @binding(0) var<storage, read> hashmap: array<u32>;

fn lookup(key: vec2u) -> u32 {
	let hashmap_entries = arrayLength(&hashmap) / ENTRY_SIZE;
	let location_hint = locationHint(key) % hashmap_entries;

	var entry = location_hint;
	loop {
		entry = (entry + 1) % hashmap_entries;

		if entry == location_hint {
			return NOT_FOUND;
		} else if hashmap[entry * ENTRY_SIZE + 0] == 0 && hashmap[entry * ENTRY_SIZE + 1] == 0 {
			return NOT_FOUND;
		} else if hashmap[entry * ENTRY_SIZE + 0] == key.x && hashmap[entry * ENTRY_SIZE + 1] == key.y {
			break;
		}
	}

	return entry * ENTRY_SIZE + 2;
}
