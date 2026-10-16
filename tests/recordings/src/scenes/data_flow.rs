use std::{path::Path, time::Duration};

use crate::{error::Result, scene::Rehearsal, stage::Focus};

pub fn play(cast_file: &Path) -> Result<()> {
    let mut take = Rehearsal::open("Samples/sorting.lean", 5)?
        .switch_view_to("Data Flow")?
        .start_take(cast_file)?;
    take.inspect_line(6)?;
    take.inspect_line(11)?;
    take.inspect_line(18)?;
    take.focus(Focus::View)?;
    take.press(&["j", "j", "l"])?;
    take.hold(Duration::from_secs(3))?;
    take.finish()
}
