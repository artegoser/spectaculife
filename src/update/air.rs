use crate::{all_foreach_left, cells::WorldCell, grid::Area};

pub fn update_air(area: &mut Area<WorldCell>) {
    let (foreach, left) = all_foreach_left!(area, air, pollution);

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
