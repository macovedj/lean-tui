use std::{path::Path, time::Duration};

use crate::{error::Result, scene::Rehearsal, stage::Focus};

pub fn play(cast_file: &Path) -> Result<()> {
    let mut take = Rehearsal::open("Samples/prime_tactic.lean", 8)?
        .switch_view_to("Semantic tableau")?
        .start_take(cast_file)?;
    take.inspect_line(11)?;
    take.inspect_line(14)?;
    take.inspect_line(17)?;
    take.focus(Focus::View)?;
    take.press(&["j", "j", "l", "l", "k"])?;
    take.hold(Duration::from_secs(3))?;
    take.finish()
}
