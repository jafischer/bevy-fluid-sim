use bevy::math::Vec2;

pub const OFFSETS_2D: [(i32, i32); 9] = [
    (-1, 1),
    (0, 1),
    (1, 1),
    (-1, 0),
    (0, 0),
    (1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

const HASH_K1: u32 = 15823;
const HASH_K2: u32 = 9737333;

pub fn get_cell_2d(position: &Vec2, smoothing_radius: f32) -> (i32, i32) {
    ((position.x / smoothing_radius).floor() as i32, (position.y / smoothing_radius).floor() as i32)
}

pub fn hash_cell_2d(cell: &(i32, i32)) -> u32 {
    let a = cell.0 as u32 * HASH_K1;
    let b = cell.1 as u32 * HASH_K2;
    a + b
}

pub fn key_from_hash(hash: u32, table_size: u32) -> u32 {
    hash % table_size
}
