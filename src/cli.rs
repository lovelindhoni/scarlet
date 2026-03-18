use argh::FromArgs;
use std::path::PathBuf;

#[derive(FromArgs)]
/// Scarlet: An objected oriented, dynamically interpreted, garbage collected programming language
pub struct ScarletCli {
    /// path to your scarlet script file for execution
    #[argh(option)]
    pub run: Option<PathBuf>,

    /// run scarlet in REPL mode
    #[argh(switch)]
    pub repl: bool,

    /// version of scarlet
    #[argh(switch, short = 'v')]
    pub version: bool,

    /// run scarlet in debug mode
    #[argh(switch, short = 'd')]
    pub debug: bool,
}
