const POINT_MAPPINGS: [(u8, u8); 25] = {
    [
        (1, 0),
        (3, 0),
        (2, 0),
        (6, 0),
        (4, 0),
        (12, 0),
        (8, 0),
        (24, 0),
        (16, 0),
        (48, 0),
        (32, 0),
        (96, 0),
        (64, 0),
        (64, 1),
        (0, 1),
        (0, 3),
        (0, 2),
        (0, 6),
        (0, 4),
        (0, 12),
        (0, 8),
        (0, 24),
        (0, 16),
        (0, 48),
        (0, 32),
    ]
};

const FILL_MAPPINGS: [(u8, u8); 13] = {
    [
        (1, 0),
        (3, 0),
        (7, 0),
        (15, 0),
        (31, 0),
        (63, 0),
        (127, 0),
        (127, 1),
        (127, 3),
        (127, 7),
        (127, 15),
        (127, 31),
        (127, 63),
    ]
};

pub fn range_point(val: f32) -> (u8, u8) {
    // TODO: check this math
    let index = val.clamp(0.0, 1.0) * (POINT_MAPPINGS.len() - 1) as f32;
    let lower_index = index.floor() as usize;
    POINT_MAPPINGS[lower_index]
}

pub fn range_fill(val: f32) -> (u8, u8) {
    let index = val.clamp(0.0, 1.0) * (FILL_MAPPINGS.len() - 1) as f32;
    let lower_index = index.floor() as usize;
    FILL_MAPPINGS[lower_index]
}
