use std::{path::Path, time::Duration};

use crate::{error::Result, scene::Rehearsal, stage::Focus};

pub fn play(cast_file: &Path) -> Result<()> {
    let mut take = Rehearsal::open("Samples/monadic.lean", 12)?
        .switch_view_to("Effect Flow")?
        .start_take(cast_file)?;
    take.inspect_line(26)?;
    take.inspect_line(28)?;
    take.focus(Focus::View)?;
    take.press(&["e", "j", "j"])?;
    take.hold(Duration::from_secs(3))?;
    take.finish()
}
