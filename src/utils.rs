pub fn get_continual_coord(n: i32, max: u32) -> u32 {
    (n).rem_euclid(max as i32) as u32
}
