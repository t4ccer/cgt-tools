pub use crate::evaluate::Args;
use cgt::short::partizan::games::ski_jumps::SkiJumps;

pub fn run(args: Args) -> anyhow::Result<()> {
    crate::evaluate::run::<SkiJumps>(args)
}
