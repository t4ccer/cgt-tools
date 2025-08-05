pub use crate::evaluate::Args;
use cgt::short::partizan::games::domineering::Domineering;

pub fn run(args: Args) -> anyhow::Result<()> {
    crate::evaluate::run::<Domineering>(args)
}
