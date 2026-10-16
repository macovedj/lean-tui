use std::{path::Path, time::Duration};

use crate::{error::Result, scene::Rehearsal, stage::Focus};

pub fn play(cast_file: &Path) -> Result<()> {
    let mut take = Rehearsal::open("Samples/prime_tactic.lean", 3)?.start_take(cast_file)?;
    take.inspect_line(8)?;
    take.inspect_line(12)?;
    take.focus(Focus::View)?;
    take.press(&["]"])?;
    take.hold(Duration::from_secs(2))?;
    take.press(&["]"])?;
    take.hold(Duration::from_secs(2))?;
    take.press(&["?"])?;
    take.hold(Duration::from_secs(3))?;
    take.escape()?;
    take.focus(Focus::Editor)?;
    take.inspect_line(29)?;
    take.hold(Duration::from_secs(3))?;
    take.finish()
}
