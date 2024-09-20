use crate::{cells::WorldCell, grid::Area};

pub fn update_air(area: &mut Area<WorldCell>) {
    let mut total: u16 = 0;

    total += area.up_left.air.pollution as u16;
    total += area.up.air.pollution as u16;
    total += area.up_right.air.pollution as u16;
    total += area.left.air.pollution as u16;
    total += area.center.air.pollution as u16;
    total += area.right.air.pollution as u16;
    total += area.down_left.air.pollution as u16;
    total += area.down.air.pollution as u16;
    total += area.down_right.air.pollution as u16;

    let foreach: u8 = (total / 9) as u8;
    let left = (total - (foreach as u16 * 9)) as u8;

    if left == 0 && foreach != 0 && foreach != 255 {
        area.up_left.air.pollution = foreach - 1;
        area.up.air.pollution = foreach - 1;
        area.up_right.air.pollution = foreach;
        area.left.air.pollution = foreach - 1;
        area.center.air.pollution = foreach;
        area.right.air.pollution = foreach + 1;
        area.down_left.air.pollution = foreach;
        area.down.air.pollution = foreach + 1;
        area.down_right.air.pollution = foreach + 1;
    }
}
