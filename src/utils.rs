pub fn get_continual_coord(n: i64, max: u32) -> u32 {
    (n).rem_euclid(max as i64) as u32
}
