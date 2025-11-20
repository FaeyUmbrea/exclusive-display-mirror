use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct BootstrapperArgs {
    /// Force update even if version matches
    #[arg(long)]
    pub force: bool,
    /// Only clean up manifest and files, then exit
    #[arg(long)]
    pub cleanup: bool,
}
