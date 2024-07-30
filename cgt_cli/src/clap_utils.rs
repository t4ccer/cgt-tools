macro_rules! mk_subcommand {
    ($($variant:ident => $module:ident),* $(,)?) => {
        $(mod $module;)*

        #[derive(::clap::Subcommand, Debug)]
        pub enum Command {
            $($variant($module::Args),)*
        }

        #[derive(::clap::Parser, Debug)]
        pub struct Args {
            #[clap(subcommand)]
            command: Command,
        }

        pub fn run(args: Args) -> ::anyhow::Result<()> {
            match args.command {
                $(Command::$variant(args) => $module::run(args),)*
            }
        }
    };
}

pub(crate) use mk_subcommand;
